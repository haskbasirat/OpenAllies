//! Native dynamic cell collision (54F370/54F190), descriptor filter (552340),
//! and the point/circle/box versus circle branch of 46EF30. Shape data is native
//! gameplay geometry, never inferred from the rendered mesh. Required world
//! callbacks can mutate descriptors and actors; later reads preserve native order.
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
    #[error("invalid collision grid or cell coordinates")]
    Grid,
    #[error("unsupported native shape kind {0}")]
    Shape(u32),
    #[error("collision world boundary failed: {0}")]
    World(String),
}
pub type Result<T> = std::result::Result<T, Error>;
const SOLID: u32 = 0x2000_0400;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Descriptor {
    pub source_definition: Option<u32>,
    pub source_actor: Option<u32>,
    pub ignore_actor: Option<u32>,
    pub ignore_definition: Option<u32>,
    pub collision_owner: Option<u32>,
    pub hostility_context: Option<u32>,
    pub ignore_all: bool,
    pub ignore_moving: bool,
    pub custom_filter: bool,
    pub mask: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Grid {
    pub dimensions: [i32; 2],
    pub coordinate_shift: u32,
    pub cell_size_bits: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tile {
    pub coordinates: [i32; 2],
    pub flags20: u32,
    pub actor_reference: u32,
    pub first_node: Option<u32>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Node {
    pub actor: u32,
    pub next: Option<u32>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Actor {
    pub definition: u32,
    pub flags: u32,
    pub position: [u32; 2],
    pub facing: [u32; 2],
}
/// Six-word native shape: POINT=0, CIRCLE=1, SQUARE=2, RECT=3.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shape {
    pub kind: u32,
    pub radius_bits: u32,
    pub bounds: [u32; 4],
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Probe {
    pub position: [u32; 2],
    pub radius_bits: u32,
}

/// Snapshots and callbacks are required. There are no clear-tile, empty-shape,
/// or friendly-allegiance defaults. Snapshot getters must have no side effects.
/// Callback errors may follow earlier mutations; this API is nontransactional.
pub trait World {
    fn tile(&self, id: u32) -> Result<Tile>;
    fn node(&self, id: u32) -> Result<Node>;
    fn actor(&self, id: u32) -> Result<Actor>;
    fn definition_flags(&self, definition: u32) -> Result<u32>;
    fn definition_flag_1a8(&self, definition: u32) -> Result<bool>;
    /// Actor virtual+4, still in x87 precision (not an early binary32 store).
    fn blocking_radius(
        &mut self,
        actor: u32,
        descriptor: Option<&mut Descriptor>,
    ) -> Result<Binary>;
    fn custom_filter(&mut self, actor: u32, descriptor: &mut Descriptor) -> Result<bool>;
    fn source_excludes(
        &mut self,
        source: u32,
        actor: u32,
        descriptor: &mut Descriptor,
    ) -> Result<bool>;
    fn category_b4(&mut self, actor: u32, descriptor: &mut Descriptor) -> Result<bool>;
    fn allegiance(&mut self, actor: u32, descriptor: &mut Descriptor) -> Result<Option<u32>>;
    /// Cache the context's virtual dispatch identity before the second allegiance
    /// query; the context receiver itself is read after that query.
    fn hostility_dispatch(&self, context: u32) -> Result<u32>;
    fn hostile(
        &mut self,
        dispatch: u32,
        context: Option<u32>,
        other: Option<u32>,
        descriptor: &mut Descriptor,
    ) -> Result<bool>;
    /// Exact 4CEDE0 leaf, requiring the live organization/mind state.
    fn moving_exclusion(&mut self, actor: u32, descriptor: &mut Descriptor) -> Result<bool>;
    /// Generation-checked GTID lookup, not a direct actor identity cast.
    fn resolve_reference(&self, reference: u32) -> Result<Option<u32>>;
    fn outer_actor(&mut self, actor: u32, descriptor: &mut Descriptor) -> Result<Option<u32>>;
    fn blocking_shape(&mut self, actor: u32, descriptor: Option<&mut Descriptor>) -> Result<Shape>;
}
fn b(bits: u32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(bits)?)
}
fn stored(value: Binary) -> Result<Binary> {
    b(value.store_f32()?)
}

/// Native 552340: callback results and descriptor fields deliberately are not cached
/// across later virtual calls, except the native hostility dispatch identity.
pub fn excluded<W: World>(actor: u32, d: &mut Descriptor, world: &mut W) -> Result<bool> {
    if d.ignore_all || d.ignore_actor == Some(actor) {
        return Ok(true);
    }
    if d.custom_filter && world.custom_filter(actor, d)? {
        return Ok(true);
    }
    if d.ignore_definition == Some(world.actor(actor)?.definition) {
        return Ok(true);
    }
    if let Some(source) = d.source_actor
        && world.source_excludes(source, actor, d)?
    {
        return Ok(true);
    }
    if !world.category_b4(actor, d)? {
        return Ok(false);
    }
    if d.hostility_context.is_some() {
        let flags = world.actor(actor)?.flags;
        if flags & (1 << 29) == 0 && flags & (1 << 10) != 0 && world.allegiance(actor, d)?.is_some()
        {
            let context = d.hostility_context.ok_or_else(|| {
                Error::World(
                    "allegiance callback removed native context before dispatch read".into(),
                )
            })?;
            let dispatch = world.hostility_dispatch(context)?;
            let other = world.allegiance(actor, d)?;
            if !world.hostile(dispatch, d.hostility_context, other, d)? {
                return Ok(true);
            }
        }
    }
    if d.ignore_moving && world.moving_exclusion(actor, d)? {
        return Ok(true);
    }
    if world.definition_flag_1a8(world.actor(actor)?.definition)?
        && let Some(source) = d.source_definition
        && world.definition_flags(source)? & SOLID != SOLID
    {
        return Ok(true);
    }
    Ok(false)
}

pub fn probe(grid: &Grid, cell: [i32; 2], precision: Precision) -> Result<Probe> {
    if grid.dimensions.iter().any(|&n| n <= 0)
        || (0..2).any(|i| cell[i] < 0 || cell[i] >= grid.dimensions[i])
    {
        return Err(Error::Grid);
    }
    let size = b(grid.cell_size_bits)?;
    if size.is_zero() || size.is_negative_nonzero() {
        return Err(Error::Grid);
    }
    let radius_bits = Binary::from_i32(size.truncate_i32()?)
        .mul(&b(0x3e80_0000)?, precision)?
        .store_f32()?;
    let half = stored(size.mul(&b(0x3f00_0000)?, precision)?)?;
    let mut position = [0; 2];
    for (out, coordinate) in position.iter_mut().zip(cell) {
        *out = Binary::from_i32(coordinate.wrapping_shl(grid.coordinate_shift & 31))
            .add(&half, precision)?
            .store_f32()?;
    }
    Ok(Probe {
        position,
        radius_bits,
    })
}

/// Actual 46EF30/46ED10/46E400 branch used by the circular cell probe. Box-box
/// SAT is intentionally outside this API; the probe can only be a circle.
pub fn overlaps(shape: &Shape, actor: &Actor, probe: &Probe, p: Precision) -> Result<bool> {
    if shape.kind > 3 {
        return Err(Error::Shape(shape.kind));
    }
    let dx = stored(b(probe.position[0])?.sub(&b(actor.position[0])?, p)?)?;
    let dy = stored(b(probe.position[1])?.sub(&b(actor.position[1])?, p)?)?;
    let radius = b(probe.radius_bits)?;
    if shape.kind <= 1 {
        let radius = if shape.kind == 1 {
            radius.add(&b(shape.radius_bits)?, p)?
        } else {
            radius
        };
        let distance = dy
            .mul(&dy, p)?
            .add(&dx.mul(&dx, p)?, p)?
            .add(&b(0x38d1_b717)?, p)?;
        return Ok(radius.mul(&radius, p)?.compare(&distance) != Ordering::Less);
    }
    let fx = b(actor.facing[0])?;
    let fy = b(actor.facing[1])?;
    let x = stored(dx.mul(&fy, p)?.sub(&dy.mul(&fx, p)?, p)?)?;
    let y = stored(dy.mul(&fy, p)?.add(&dx.mul(&fx, p)?, p)?)?;
    let outside = |value: Binary, min: u32, max: u32| -> Result<Option<Binary>> {
        let lo = b(min)?;
        let hi = b(max)?;
        if value.compare(&lo) == Ordering::Less {
            Ok(Some(lo.sub(&value, p)?))
        } else if value.compare(&hi) == Ordering::Greater {
            Ok(Some(value.sub(&hi, p)?))
        } else {
            Ok(None)
        }
    };
    match (
        outside(x, shape.bounds[0], shape.bounds[2])?,
        outside(y, shape.bounds[1], shape.bounds[3])?,
    ) {
        (None, None) => Ok(true),
        (Some(v), None) | (None, Some(v)) => Ok(v.compare(&radius) == Ordering::Less),
        (Some(x), Some(y)) => Ok(radius
            .mul(&radius, p)?
            .compare(&y.mul(&y, p)?.add(&x.mul(&x, p)?, p)?)
            == Ordering::Greater),
    }
}

/// 54F190: first eligible, non-excluded actor whose native shape overlaps the
/// probe. Node.next is read after callbacks, including callbacks on rejected actors.
pub fn find_blocker<W: World>(
    tile_id: u32,
    grid: &Grid,
    mut descriptor: Option<&mut Descriptor>,
    p: Precision,
    world: &mut W,
) -> Result<Option<u32>> {
    let tile = world.tile(tile_id)?;
    let mut node = tile.first_node;
    let probe = probe(grid, tile.coordinates, p)?;
    while let Some(id) = node {
        let actor = world.node(id)?.actor;
        let flags = world.actor(actor)?.flags;
        if flags & SOLID == SOLID {
            let cell_size = Binary::from_i32(b(grid.cell_size_bits)?.truncate_i32()?);
            let radius = world.blocking_radius(actor, descriptor.as_deref_mut())?;
            if cell_size.compare(&radius.mul(&Binary::from_u32(4), p)?) != Ordering::Greater {
                let mut skip = false;
                if let Some(d) = descriptor.as_deref_mut() {
                    skip = excluded(actor, d, world)?;
                    if !skip
                        && d.collision_owner.is_some()
                        && let Some(reference_actor) =
                            world.resolve_reference(world.tile(tile_id)?.actor_reference)?
                        && world.outer_actor(reference_actor, d)?.is_some()
                    {
                        skip = world.outer_actor(reference_actor, d)? == d.collision_owner;
                    }
                }
                if !skip {
                    let placement = world.actor(actor)?;
                    let shape = world.blocking_shape(actor, descriptor.as_deref_mut())?;
                    if overlaps(&shape, &placement, &probe, p)? {
                        return Ok(Some(actor));
                    }
                }
            }
        }
        node = world.node(id)?.next;
    }
    Ok(None)
}

/// 54F370. Mask-only terrain collision does not need a dynamic object scan.
pub fn blocked<W: World>(
    tile_id: u32,
    grid: &Grid,
    d: &mut Descriptor,
    p: Precision,
    world: &mut W,
) -> Result<bool> {
    let flags = world.tile(tile_id)?.flags20;
    if flags & d.mask == 0 {
        return Ok(false);
    }
    let filtered = d.source_actor.is_some()
        || d.ignore_actor.is_some()
        || d.ignore_definition.is_some()
        || d.collision_owner.is_some();
    if filtered && flags & (1 << 10) != 0 {
        return Ok(find_blocker(tile_id, grid, Some(d), p, world)?.is_some());
    }
    Ok(true)
}
