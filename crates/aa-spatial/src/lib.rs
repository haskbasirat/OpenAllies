//! Bounded native ground-position and terrain-query sequencing.
//! World heightfields, shape ownership, tile locks and render notifications are
//! explicit boundaries. Dynamic cell collision lives in `collision`; no world
//! object registry, complete path solver or default FPU mode is supplied.
#![forbid(unsafe_code)]

use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
pub mod height_layers;
pub mod geometry;
pub mod index;
pub mod collision;
pub mod moving_state;
pub mod tile_walk;
pub mod range;
pub mod shape_overlap;
pub mod soft_range;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
    #[error(transparent)]
    Heightfield(#[from] aa_heightfield::Error),
    #[error("invalid native grid dimensions or cell count")]
    InvalidGrid,
    #[error("ground move bit12 without bit21 leaves native height uninitialized")]
    UninitializedNativeHeight,
    #[error("half-cell snapping requires a finite positive cell size")]
    InvalidCellSize,
    #[error("world boundary failed: {0}")]
    World(String),
    #[error("tile-walk candidate budget exhausted; cursor retains completed skips")]
    WalkBudget,
}
pub type Result<T> = std::result::Result<T, Error>;

/// Native actor fields only; bit patterns preserve signed zero exactly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Actor {
    pub flags: u32,
    pub position: [u32; 3],
    pub facing: [u32; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShapeKind {
    A,
    B,
}

/// Native six-word transformed-shape descriptor. The opaque shape identity is
/// supplied by actor virtual+0xC or +0x10; it is not a guessed bounding radius.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShapePlacement {
    pub shape: u32,
    pub facing: [u32; 2],
    pub position: [u32; 2],
    pub trailing_zero: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeightLayer {
    Base,
    Overlay,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeightMethod {
    Nearest,
    Interpolated,
}

/// Cell definition fields +0x1C and +0x68/+0x6C. Kind1 adds the deterministic
/// native per-cell offset; kind2 selects overlay without that offset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cell {
    pub overlay_kind: i32,
    pub offset_min: u32,
    pub offset_max: u32,
}

/// Stable world-grid snapshot for one operation, not a map-state constructor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainGrid {
    pub width: i32,
    pub height: i32,
    pub coordinate_shift: u32,
    pub cell_size: u32,
    pub cells: Vec<Cell>,
}
impl TerrainGrid {
    pub fn validate(&self) -> Result<()> {
        let count = self.width.checked_mul(self.height).filter(|n| *n > 0);
        if self.width <= 0
            || self.height <= 0
            || count.map(|n| n as usize) != Some(self.cells.len())
        {
            return Err(Error::InvalidGrid);
        }
        Ok(())
    }
    fn cell(&self, x: i32, y: i32) -> &Cell {
        &self.cells[(y * self.width + x) as usize]
    }
}

/// Required native leaves. Callbacks may mutate actor fields; later reads occur
/// in native order. Grid/settings are a stable snapshot, not callback-mutable.
/// A boundary error can follow earlier effects; operations are nontransactional.
pub trait World {
    /// 550190/550210 on world+34 or +38. Return the un-stored x87 value; the
    /// query explicitly performs the original binary32 store where required.
    fn sample_height(
        &mut self,
        actor: &mut Actor,
        layer: HeightLayer,
        method: HeightMethod,
        position: [u32; 2],
        precision: Precision,
    ) -> Result<Binary>;
    fn shape(&mut self, actor: &mut Actor, kind: ShapeKind) -> Result<u32>;
    /// 5539B0 boundary, not a tile-lock implementation. Its own bit26 predicate,
    /// overlap/ownership updates and geometry extents remain the world's job.
    fn update_tiles(
        &mut self,
        actor: &mut Actor,
        old_b: ShapePlacement,
        new_b: ShapePlacement,
        old_a: ShapePlacement,
        new_a: ShapePlacement,
    ) -> Result<()>;
    /// Actor virtual+0xC8, after all three position words have been committed.
    fn position_changed(&mut self, actor: &mut Actor) -> Result<()>;
    fn world_phase(&self) -> u32;
    /// [GWorld+0x130]+4; this is the manager target, NOT actor+4.
    fn presentation_available(&self) -> bool;
    /// 43B760 leaf of 50C250, gated after position_changed may mutate the actor.
    fn presentation_changed(&mut self, actor: &mut Actor) -> Result<()>;
}

fn b(bits: u32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(bits)?)
}
fn xy(actor: &Actor) -> [u32; 2] {
    [actor.position[0], actor.position[1]]
}

const HALF: u32 = 0x3f00_0000;
const MILLI: u32 = 0x3a83_126f;
const NEIGHBORS: [[i32; 2]; 8] = [
    [-1, -1],
    [-1, 0],
    [-1, 1],
    [0, 1],
    [1, 1],
    [1, 0],
    [1, -1],
    [0, -1],
];

/// Original4F0E50. Query position is re-read after sampler callbacks exactly as
/// the native pointer to actor+20 is re-read. Unknown flag bits are ignored.
pub fn query_actor_height<W: World>(
    actor: &mut Actor,
    flags: u32,
    grid: &TerrainGrid,
    precision: Precision,
    world: &mut W,
) -> Result<Binary> {
    let method = if flags & 0x10 != 0 {
        HeightMethod::Nearest
    } else {
        HeightMethod::Interpolated
    };
    let base_position = xy(actor);
    let base = b(world
        .sample_height(actor, HeightLayer::Base, method, base_position, precision)?
        .store_f32()?)?;
    let half = b(HALF)?;
    if flags & 5 == 5 {
        return Ok(base.sub(&half, precision)?);
    }
    let mut overlay = Binary::from_i32(0);
    if flags & 2 != 0 {
        grid.validate()?;
        let shift = grid.coordinate_shift & 31;
        let x = (b(actor.position[0])?.truncate_i32()? >> shift).clamp(0, grid.width - 1);
        let y = (b(actor.position[1])?.truncate_i32()? >> shift).clamp(0, grid.height - 1);
        let cell = grid.cell(x, y);
        let own_kind_one = cell.overlay_kind == 1;
        let selected = own_kind_one
            || cell.overlay_kind == 2
            || NEIGHBORS.iter().any(|[dx, dy]| {
                let nx = x + dx;
                let ny = y + dy;
                nx >= 0
                    && ny >= 0
                    && nx < grid.width
                    && ny < grid.height
                    && matches!(grid.cell(nx, ny).overlay_kind, 1 | 2)
            });
        if selected {
            let overlay_position = xy(actor);
            overlay = world.sample_height(
                actor,
                HeightLayer::Overlay,
                HeightMethod::Interpolated,
                overlay_position,
                precision,
            )?;
            if own_kind_one {
                let remainder = (y
                    .wrapping_mul(grid.width)
                    .wrapping_add(x)
                    .wrapping_mul(0x1273)
                    .wrapping_add(0x18b3)
                    % 0x1b55)
                    % 1000;
                let offset = b(cell.offset_max)?
                    .sub(&b(cell.offset_min)?, precision)?
                    .mul(&Binary::from_i32(remainder), precision)?
                    .mul(&b(MILLI)?, precision)?
                    .add(&b(cell.offset_min)?, precision)?;
                overlay = overlay.add(&offset, precision)?;
            }
        }
    }
    let total = b(overlay.add(&base, precision)?.store_f32()?)?;
    if flags & 1 != 0 {
        let floor = base.sub(&half, precision)?;
        if floor.compare(&total) == Ordering::Greater {
            return Ok(floor);
        }
    }
    Ok(total)
}

/// 46EE00's floor argument is binary64. Within this f32-input operation all
/// finite intermediates reaching the supported i32 floor domain are normal
/// binary64 or zero, so an explicit PC53 round has the same store value.
fn floor_qword_i32(value: Binary) -> Result<Binary> {
    let qword = value.mul(&Binary::from_i32(1), Precision::Double53)?;
    let trunc = qword.truncate_i32()?;
    let integer = Binary::from_i32(trunc);
    if qword.compare(&integer) == Ordering::Less {
        Ok(Binary::from_i32(
            trunc
                .checked_sub(1)
                .ok_or(aa_numeric::Error::IntegerRange)?,
        ))
    } else {
        Ok(integer)
    }
}

/// Exact half-cell snapping (46EE00), including the extended reciprocal for X
/// but stored binary32 reciprocal for Y. Both floor results must fit i32.
pub fn snap_half_cell(
    candidate: &mut [u32; 2],
    cell_size: u32,
    precision: Precision,
) -> Result<()> {
    let cell = b(cell_size)?;
    if cell.compare(&Binary::from_i32(0)) != Ordering::Greater {
        return Err(Error::InvalidCellSize);
    }
    let half = b(cell.mul(&b(HALF)?, precision)?.store_f32()?)?;
    if half.is_zero() {
        return Err(Error::InvalidCellSize);
    }
    let reciprocal = Binary::from_i32(1).div(&half, precision)?;
    let reciprocal_stored = b(reciprocal.store_f32()?)?;
    let x = floor_qword_i32(
        reciprocal
            .mul(&b(candidate[0])?, precision)?
            .add(&b(HALF)?, precision)?,
    )?;
    candidate[0] = x.mul(&half, precision)?.store_f32()?;
    let y = floor_qword_i32(
        reciprocal_stored
            .mul(&b(candidate[1])?, precision)?
            .add(&b(HALF)?, precision)?,
    )?;
    candidate[1] = y.mul(&half, precision)?.store_f32()?;
    Ok(())
}

fn placement<W: World>(
    actor: &mut Actor,
    kind: ShapeKind,
    position: [u32; 2],
    world: &mut W,
) -> Result<ShapePlacement> {
    let facing = actor.facing;
    let shape = world.shape(actor, kind)?;
    Ok(ShapePlacement {
        shape,
        facing,
        position,
        trailing_zero: 0,
    })
}

/// Original4EE5C0 for a distinct candidate buffer (as supplied by4EEB40).
/// Getter callbacks may change actor fields; each placement captures fields
/// before its own getter. No aliasing between candidate and actor is inferred.
pub fn commit_position<W: World>(
    actor: &mut Actor,
    candidate: [u32; 3],
    world: &mut W,
) -> Result<()> {
    for value in candidate {
        b(value)?;
    }
    if actor.flags & (1 << 26) != 0 {
        let new_a = placement(actor, ShapeKind::A, [candidate[0], candidate[1]], world)?;
        let old_a_position = xy(actor);
        let old_a = placement(actor, ShapeKind::A, old_a_position, world)?;
        let new_b = placement(actor, ShapeKind::B, [candidate[0], candidate[1]], world)?;
        let old_b_position = xy(actor);
        let old_b = placement(actor, ShapeKind::B, old_b_position, world)?;
        world.update_tiles(actor, old_b, new_b, old_a, new_a)?;
    }
    actor.position = candidate;
    world.position_changed(actor)?;
    if world.world_phase() != 6 && world.presentation_available() {
        world.presentation_changed(actor)?;
    }
    Ok(())
}

/// Original4EEB40→4F0E50→4EE5C0. Z is queried at OLD actor XY, and stored before
/// optional candidate snapping. Bit12/no-bit21 rejects uninitialized native Z.
pub fn move_ground<W: World>(
    actor: &mut Actor,
    mut candidate: [u32; 2],
    grid: &TerrainGrid,
    precision: Precision,
    world: &mut W,
) -> Result<()> {
    let flags = actor.flags;
    if flags & (1 << 21) == 0 && flags & (1 << 12) != 0 {
        return Err(Error::UninitializedNativeHeight);
    }
    let query_flags = if flags & (1 << 21) != 0 { 0x123 } else { 0x122 };
    let height = query_actor_height(actor, query_flags, grid, precision, world)?.store_f32()?;
    // Re-read flags after height callbacks, matching the native instruction.
    if actor.flags & (1 << 16) != 0 {
        let cell_size = grid.cell_size;
        world.shape(actor, ShapeKind::A)?;
        snap_half_cell(&mut candidate, cell_size, precision)?;
    }
    commit_position(actor, [candidate[0], candidate[1], height], world)
}
pub mod index_world;
pub mod position;
pub mod query;
pub mod sweep;
pub mod actor_context;
pub mod source_exclusion;
pub mod overlaps;
pub mod placement;
pub mod water_areas;
pub mod gworld_placement;
pub mod loaded_object_placement;

pub mod creation_placement;
pub mod required_placement;
pub mod rectangle_actors;
pub mod doodad_clear;
pub mod circle_actors;
