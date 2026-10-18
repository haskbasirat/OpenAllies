//! Native552DD0/5531F0 prepared descriptor and553BA0 first exact spatial hit.
//! The query uses live index lists and actual full placed-shape intersections.
use crate::{
    Error, Result,
    collision::{self, Descriptor, Shape},
    geometry::{Placement, orientation, tile_rectangle, world_bounds},
    index::{ActorKey, SpatialIndex},
    index_world::{CollisionPolicy, CollisionView},
};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};

pub trait QueryPolicy: CollisionPolicy {
    /// Definition virtual+30 yields a nonnull ObjectData view. This is not
    /// inferred from a renderer or object name; ordinary native definitions
    /// can supply their already verified category result.
    fn definition_has_object_data(&self, definition: u32) -> collision::Result<bool>;
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PreparedQuery {
    pub placement: Placement,
    pub descriptor: Descriptor,
    /// Native+38 actor flags required in full, separate from+3C tile mask.
    pub required_actor_flags: u32,
}
fn world_error(e: collision::Error) -> Error {
    Error::World(e.to_string())
}
impl PreparedQuery {
    /// Ordinary552DD0→5531F0→5522D0. Every constructor field needed by this
    /// branch is overwritten with the source actor's actual immutable shapes.
    pub fn for_actor<P: QueryPolicy>(
        index: &SpatialIndex,
        actor: ActorKey,
        strict: bool,
        extra_flags: u32,
        policy: &P,
    ) -> Result<Self> {
        let member = index.member(actor)?;
        let mut query = Self {
            placement: member.placement(true),
            descriptor: Descriptor {
                source_definition: Some(member.actor.definition),
                source_actor: Some(actor),
                ..Descriptor::default()
            },
            required_actor_flags: 0,
        };
        query.set_position(index, member.actor.position, policy)?;
        let flags = member.actor.flags;
        if flags & ((1 << 18) | (1 << 12)) == 0 {
            query.required_actor_flags |= 0x400;
            if strict || flags & (1 << 10) == 0 {
                query.required_actor_flags |= 0x20000000;
            }
            query.descriptor.mask |= if flags & (1 << 21) != 0 { 0x600 } else { 0x500 };
        }
        query.required_actor_flags |= extra_flags;
        query.descriptor.mask |= extra_flags;
        Ok(query)
    }
    ///552250; definition snap flag bit16 is read only after a successful
    /// actual ObjectData-view dispatch. The ordinary shape getter is immutable.
    pub fn set_position<P: QueryPolicy>(
        &mut self,
        index: &SpatialIndex,
        mut xy: [u32; 2],
        policy: &P,
    ) -> Result<()> {
        if let Some(definition) = self.descriptor.source_definition
            && policy
                .definition_has_object_data(definition)
                .map_err(world_error)?
            && policy.definition_flags(definition).map_err(world_error)? & (1 << 16) != 0
        {
            crate::snap_half_cell(&mut xy, index.grid().cell_size_bits, index.precision())?;
        }
        for bits in xy {
            Binary::from_f32_bits(bits)?;
        }
        self.placement.position = xy;
        Ok(())
    }
}
/// Exact constructed GMap+C/+10 extents and+30 full-cell square. The actual
/// stored extents may exceed dimensions*cellsize by a remainder; do not infer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryGeometry {
    pub extent_bits: [u32; 2],
    pub cell_shape: Shape,
}
impl QueryGeometry {
    /// Original556689..696→46EB60 with explicit constructor FPU precision.
    pub fn from_native_extents(
        index: &SpatialIndex,
        extent_bits: [u32; 2],
        constructor_precision: Precision,
    ) -> Result<Self> {
        for (bits, cells) in extent_bits.into_iter().zip(index.grid().dimensions) {
            let value = Binary::from_f32_bits(bits)?;
            if value.is_negative_nonzero()
                || value.truncate_i32()? >> (index.grid().coordinate_shift & 31) != cells
            {
                return Err(Error::InvalidGrid);
            }
        }
        let half = Binary::from_f32_bits(index.grid().cell_size_bits)?
            .mul(&Binary::from_f32_bits(0x3f000000)?, constructor_precision)?;
        let negative = half.clone().negated();
        let radius = half
            .mul(&half, constructor_precision)?
            .add(
                &negative.mul(&negative, constructor_precision)?,
                constructor_precision,
            )?
            .sqrt(constructor_precision)?
            .store_f32()?;
        Ok(Self {
            extent_bits,
            cell_shape: Shape {
                kind: 2,
                radius_bits: radius,
                bounds: [
                    negative.store_f32()?,
                    negative.store_f32()?,
                    half.store_f32()?,
                    half.store_f32()?,
                ],
            },
        })
    }
    pub(crate) fn tile_placement(&self, index: &SpatialIndex, cell: [i32; 2]) -> Result<Placement> {
        let p = index.precision();
        let half = Binary::from_f32_bits(index.grid().cell_size_bits)?
            .mul(&Binary::from_f32_bits(0x3f000000)?, p)?
            .store_f32()?;
        let mut position = [0; 2];
        for a in 0..2 {
            position[a] =
                Binary::from_i32(cell[a].wrapping_shl(index.grid().coordinate_shift & 31))
                    .add(&Binary::from_f32_bits(half)?, p)?
                    .store_f32()?;
        }
        Ok(Placement {
            shape: self.cell_shape.clone(),
            facing: [0, 0xbf800000],
            position,
        })
    }
}
/// Native result words:0 none,1 static tile,2 actor GTID,3 outside world.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hit {
    pub kind: u32,
    pub reference: u32,
    pub cell: [i32; 2],
}
impl Hit {
    pub fn blocked(&self) -> bool {
        self.kind != 0
    }
}

/// Original553370: lower extent is inclusive and upper extent strict. General
/// rotation keeps X half-size extended but stores Y, opposite5534F0's stores.
pub fn outside(placement: &Placement, extent_bits: [u32; 2], p: Precision) -> Result<bool> {
    let b = |bits| Binary::from_f32_bits(bits).map_err(Error::from);
    let s = &placement.shape;
    let (hx, hy) = if s.kind <= 1 {
        (b(s.radius_bits)?, b(s.radius_bits)?)
    } else {
        match orientation(placement.facing)? {
            3 => (b(s.bounds[2])?, b(s.bounds[3])?),
            2 => (b(s.bounds[3])?, b(s.bounds[2])?),
            _ => {
                let bounds = world_bounds(placement, p)?;
                let half = b(0x3f000000)?;
                let abs = |value: Binary| {
                    if value.is_negative_nonzero() {
                        value.negated()
                    } else {
                        value
                    }
                };
                (
                    abs(b(bounds[2])?.sub(&b(bounds[0])?, p)?).mul(&half, p)?,
                    b(abs(b(bounds[3])?.sub(&b(bounds[1])?, p)?)
                        .mul(&half, p)?
                        .store_f32()?)?,
                )
            }
        }
    };
    let zero = Binary::from_i32(0);
    for ((position, radius), extent) in placement
        .position
        .into_iter()
        .zip([hx, hy])
        .zip(extent_bits)
    {
        let position = b(position)?;
        if position.sub(&radius, p)?.compare(&zero).is_lt()
            || !position.add(&radius, p)?.compare(&b(extent)?).is_lt()
        {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(crate) fn mark(index: &mut SpatialIndex, head: &mut Option<ActorKey>, actor: ActorKey) -> Result<bool> {
    if index.query_link(actor)?.is_some() || *head == Some(actor) {
        return Ok(true);
    }
    index.set_query_link(actor, Some(head.unwrap_or(actor)))?;
    *head = Some(actor);
    Ok(false)
}
pub(crate) fn clear(index: &mut SpatialIndex, head: &mut Option<ActorKey>) -> Result<()> {
    while let Some(actor) = *head {
        *head = index.query_link(actor)?;
        index.set_query_link(actor, None)?;
    }
    Ok(())
}
///553BA0's temporary iterator copies the descriptor (including resnapping),
/// scans Y-major/X-minor, and clears only its own intrusive query links on every
/// exit. Nested scans observe earlier marks. Required policy errors unwind the
/// temporary marks while retaining other completed world effects.
pub fn first_hit<P: QueryPolicy>(
    index: &mut SpatialIndex,
    geometry: &QueryGeometry,
    prepared: &PreparedQuery,
    policy: &mut P,
) -> Result<Hit> {
    let mut query = prepared.clone();
    query.set_position(index, prepared.placement.position, policy)?;
    let mut head = None;
    let result = scan(index, geometry, &mut query, policy, &mut head);
    let cleanup = clear(index, &mut head);
    match (result, cleanup) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), Ok(())) | (Ok(_), Err(error)) => Err(error),
        (Err(error), Err(cleanup)) => Err(Error::World(format!(
            "{error}; query cleanup failed: {cleanup}"
        ))),
    }
}
fn scan<P: QueryPolicy>(
    index: &mut SpatialIndex,
    geometry: &QueryGeometry,
    query: &mut PreparedQuery,
    policy: &mut P,
    head: &mut Option<ActorKey>,
) -> Result<Hit> {
    let p = index.precision();
    let grid = index.grid().clone();
    let boundary = outside(&query.placement, geometry.extent_bits, p)?;
    let rect = tile_rectangle(&grid, &query.placement, p)?;
    //553BE2..BF8 selects large-only after iterator preparation. The definition
    //flag is distinct from actor flags, and this gate precedes intrusive marks.
    let large_only = if let Some(definition) = query.descriptor.source_definition {
        policy.definition_flags(definition).map_err(world_error)? & (1 << 10) == 0
    } else {
        false
    };
    if boundary && query.descriptor.mask != 0 {
        return Ok(Hit {
            kind: 3,
            ..Hit::default()
        });
    }
    let mut hit = Hit::default();
    for y in rect.min[1]..=rect.max[1] {
        for x in rect.min[0]..=rect.max[0] {
            let cell = [x, y];
            //558040 stores the list cursor before first tile-policy callbacks.
            let mut cursor = index.tile(cell)?.first_node;
            let restore = query.descriptor.mask & 0x400 != 0;
            if restore {
                query.descriptor.mask &= !0x400;
            }
            let blocked = CollisionView { index, policy }
                .blocked(cell, &mut query.descriptor)
                .map_err(world_error)?;
            if blocked
                && crate::shape_overlap::overlaps(
                    &geometry.tile_placement(index, cell)?,
                    &query.placement,
                    p,
                )?
            {
                if index.tile(cell)?.flags20 & 0x400 == 0 {
                    hit.kind = 1;
                    hit.cell = cell;
                    return Ok(hit);
                }
                let blocker = index.find_blocker(cell)?;
                hit.kind = 2;
                hit.reference = blocker
                    .map(|key| index.member(key).map(|m| m.reference))
                    .transpose()?
                    .unwrap_or(0);
                if index.resolve_reference(hit.reference).is_some() {
                    return Ok(hit);
                }
            }
            if restore {
                query.descriptor.mask |= 0x400;
            }
            if !query.descriptor.ignore_all {
                while let Some(node) = cursor {
                    let node = index.node(node)?.clone();
                    // Native iterator advances before mark/filter/shape callbacks.
                    cursor = node.next;
                    let actor = node.actor;
                    if large_only {
                        let member = index.member(actor)?;
                        if !crate::geometry::marks_blocker(
                            member.actor.flags,
                            member.blocking.radius_bits,
                            grid.cell_size_bits,
                            p,
                        )? {
                            continue;
                        }
                    }
                    if mark(index, head, actor)? {
                        continue;
                    }
                    if index.member(actor)?.actor.flags & query.required_actor_flags
                        != query.required_actor_flags
                    {
                        continue;
                    }
                    if collision::excluded(
                        actor,
                        &mut query.descriptor,
                        &mut CollisionView { index, policy },
                    )
                    .map_err(world_error)?
                    {
                        continue;
                    }
                    let placement = index.member(actor)?.placement(true);
                    if crate::shape_overlap::overlaps(&query.placement, &placement, p)? {
                        hit.kind = 2;
                        hit.reference = index.member(actor)?.reference;
                        return Ok(hit);
                    }
                }
            }
        }
    }
    Ok(Hit::default())
}

#[cfg(test)]
#[path = "query_tests.rs"]
mod tests;
