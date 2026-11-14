//! Original `KKC_GMap::FindOpenPositionLine` `553FA0` and
//! `KKC_GMap::FindOpenPositionLineThenRadius` `554BF0` (Map.cpp): the free
//! spot on the line from a point back toward a `from` point, then (554BF0)
//! the radius search around it.
//!
//! `553FA0(point, facing, definition, from.x, from.y, non_blocking)` on the
//! GMap (`ret 0x18`):
//! 1. `552F70(definition + 38)` builds the query descriptor (shape copy,
//!    descriptor +18 source cleared); required actor flags +38 = 0x400, plus
//!    0x20000000 with `non_blocking`; mask +3C = 0x600 when definition +1C
//!    bit 21 is set, else 0x500; facing +44/+48, +54 = 0.
//! 2. The step is the definition +3C word (the shape radius), raised to the
//!    binary32 store of GMap +8 (cell size) * `[A033BC]` (0.25) when below
//!    it. The offset `point - from` is stored as binary32; its length (sqrt
//!    of `dy^2 + dx^2`) is stored, and when the unstored length is above 0
//!    the offset is scaled by `step / length` (unstored ratio, stored
//!    products).
//! 3. Loop: a copy of the point is clamped into the map (`552640`) and
//!    queried (`553BA0`, position +4C/+50). The `46EE00` grid snap between
//!    the two is guarded by descriptor +18, which `552F70` cleared, so it
//!    never runs here. A clear result ends the loop (found). A stored length
//!    of 0 or less ends it (not found). Otherwise the point steps back by
//!    the offset (stored), the length loses the step (stored, compared
//!    unstored) and at 0 or less the point becomes `from` and the length 0.
//! 4. `|point.x - from.x| + |point.y - from.y|` (unstored) not below
//!    `[A32B58]` (0.0002) with a found spot: the point takes the last clamped
//!    copy. Otherwise the point keeps its stepped value. AL is always 1.
//!
//! `554BF0(point, facing, definition, from.x, from.y, radius)` (GMap from
//! GWorld +30, `ret 0x18`): `553FA0(..., 1)` (its false branch cannot be
//! reached), then `554B30(&copy, facing, definition, radius, 0, 0)` on a
//! copy of the point; success stores the copy. AL is 1 either way; the
//! diagnostics (`404F60`, Map.cpp lines 0x7E5/0x7F3) only read state.
use crate::{Result, collision::Shape};
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

/// `[A033BC]` binary32 0.25.
const QUARTER: u32 = 0x3e80_0000;
/// `[A32B58]` binary32 0.0002.
const MOVED: u32 = 0x3951_b717;
/// Descriptor +38 bits written by `553FA0`.
pub const REQUIRED_FLAGS: u32 = 0x400;
pub const NON_BLOCKING_FLAG: u32 = 0x2000_0000;
/// Descriptor +3C masks: definition +1C bit 21 set / clear.
pub const WATER_MASK: u32 = 0x600;
pub const LAND_MASK: u32 = 0x500;

/// The descriptor fields `553FA0` hands to `553BA0`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenLineQuery {
    /// Definition +38 shape (`46E0A0` copy).
    pub shape: Shape,
    /// Descriptor +38.
    pub required_actor_flags: u32,
    /// Descriptor +3C.
    pub mask: u32,
    /// Descriptor +44/+48.
    pub facing: [u32; 2],
    /// Descriptor +4C/+50.
    pub position: [u32; 2],
}

/// Services of `553FA0`, in native order.
pub trait OpenLineWorld {
    /// GMap +8 (cell size, binary32).
    fn cell_size(&mut self) -> Result<u32>;
    /// `552640` on the GMap: clamp the point into the map interior.
    fn clamp_to_interior(&mut self, point: &mut [u32; 2]) -> Result<()>;
    /// `553BA0(&result, &descriptor)` on the GMap: the result's first word
    /// (0 when nothing is hit).
    fn first_hit(&mut self, query: &OpenLineQuery) -> Result<u32>;
}

/// Services of `554BF0` beyond `553FA0`.
pub trait OpenLineRadiusWorld: OpenLineWorld {
    /// `554B30(&point, facing, definition, radius, 0, 0)` on the GMap.
    fn find_open_radius(&mut self, point: &mut [u32; 2], facing: [u32; 2], radius: u32) -> Result<bool>;
}

fn n(bits: u32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(bits)?)
}
fn zero() -> Binary {
    Binary::from_i32(0)
}
/// `fcom [9F32DC]` then `fchs` below 0.0.
fn abs(value: Binary) -> Binary {
    if value.compare(&zero()) == Ordering::Less {
        value.negated()
    } else {
        value
    }
}

/// Complete `553FA0`; returns the original AL (always true).
/// `water` is definition +1C bit 21; `shape` is definition +38.
#[allow(clippy::too_many_arguments)]
pub fn open_line(
    point: &mut [u32; 2],
    facing: [u32; 2],
    shape: Shape,
    water: bool,
    from: [u32; 2],
    non_blocking: bool,
    world: &mut impl OpenLineWorld,
    p: Precision,
) -> Result<bool> {
    let mut step = shape.radius_bits;
    let mut query = OpenLineQuery {
        shape,
        required_actor_flags: REQUIRED_FLAGS | if non_blocking { NON_BLOCKING_FLAG } else { 0 },
        mask: if water { WATER_MASK } else { LAND_MASK },
        facing,
        position: [0, 0],
    };
    let mut dx = n(point[0])?.sub(&n(from[0])?, p)?.store_f32()?;
    let mut dy = n(point[1])?.sub(&n(from[1])?, p)?.store_f32()?;
    let quarter = n(world.cell_size()?)?.mul(&n(QUARTER)?, p)?;
    // fcomp: C0 alone (below) takes the stored quarter cell.
    if n(step)?.compare(&quarter) == Ordering::Less {
        step = quarter.store_f32()?;
    }
    let squares = n(dy)?.mul(&n(dy)?, p)?.add(&n(dx)?.mul(&n(dx)?, p)?, p)?;
    let length = squares.sqrt(p)?;
    let mut left = length.store_f32()?;
    if length.compare(&zero()) == Ordering::Greater {
        let ratio = n(step)?.div(&n(left)?, p)?;
        dx = n(dx)?.mul(&ratio, p)?.store_f32()?;
        dy = n(dy)?.mul(&ratio, p)?.store_f32()?;
    }
    let mut found = true;
    let mut clamped;
    loop {
        clamped = *point;
        world.clamp_to_interior(&mut clamped)?;
        query.position = clamped;
        if world.first_hit(&query)? == 0 {
            break;
        }
        if n(left)?.compare(&zero()) != Ordering::Greater {
            found = false;
            break;
        }
        point[0] = n(point[0])?.sub(&n(dx)?, p)?.store_f32()?;
        point[1] = n(point[1])?.sub(&n(dy)?, p)?.store_f32()?;
        let rest = n(left)?.sub(&n(step)?, p)?;
        left = rest.store_f32()?;
        if rest.compare(&zero()) != Ordering::Greater {
            *point = from;
            left = 0;
        }
    }
    let ax = abs(n(point[0])?.sub(&n(from[0])?, p)?);
    let ay = abs(n(point[1])?.sub(&n(from[1])?, p)?);
    let moved = ay.add(&ax, p)?;
    if moved.compare(&n(MOVED)?) != Ordering::Less && found {
        *point = clamped;
    }
    Ok(true)
}

/// Complete `554BF0`; returns the original AL (always true).
#[allow(clippy::too_many_arguments)]
pub fn open_line_then_radius(
    point: &mut [u32; 2],
    facing: [u32; 2],
    shape: Shape,
    water: bool,
    from: [u32; 2],
    radius: u32,
    world: &mut impl OpenLineRadiusWorld,
    p: Precision,
) -> Result<bool> {
    open_line(point, facing, shape, water, from, true, world, p)?;
    let mut copy = *point;
    if world.find_open_radius(&mut copy, facing, radius)? {
        *point = copy;
    }
    Ok(true)
}

/// `[A3AA10]` binary32 1e-4: a shorter offset leaves the facing (0, 0).
const SHORT: u32 = 0x38d1_b717;
/// The turn between ring candidates (-pi/4) and between rings (-2pi/3).
pub const CANDIDATE_TURN: u32 = 0xbf49_0fdb;
pub const RING_TURN: u32 = 0xc006_0a92;

/// Services of `554500`.
pub trait OpenSpotWorld {
    /// Definition virtual +28(&scratch): its alternate shape (six words).
    fn alternate_shape(&mut self) -> Result<Shape>;
    /// Definition virtual +30 (nonzero: the definition may snap).
    fn definition_virtual_30(&mut self) -> Result<u32>;
    /// Definition +1E byte.
    fn definition_byte_1e(&mut self) -> Result<u8>;
    /// `[[B36534]+30]+8` (GMap cell size, binary32).
    fn cell_size(&mut self) -> Result<u32>;
    /// `553BA0(&result, &descriptor)` on the GMap: the result's first word.
    fn first_hit(&mut self, query: &OpenLineQuery) -> Result<u32>;
    /// `85DAA0(angle)` and `85DA80(angle)`: the 4096-entry table words
    /// (`aa_trig::lookup_sin` / `lookup_cos`).
    fn table_sin(&mut self, angle_bits: u32) -> Result<u32>;
    fn table_cos(&mut self, angle_bits: u32) -> Result<u32>;
}

/// `552250` / the inline copy at `554685`: the point snaps to the half cell
/// (`46EE00`) when the definition (descriptor +18, never null here) has a
/// nonzero virtual +30 and +1E bit 0 set.
fn place(point: &mut [u32; 2], world: &mut impl OpenSpotWorld, p: Precision) -> Result<()> {
    if world.definition_virtual_30()? != 0 && world.definition_byte_1e()? & 1 != 0 {
        let cell = world.cell_size()?;
        crate::snap_half_cell(point, cell, p)?;
    }
    Ok(())
}

/// `(x, y)` turned by the table angle: `x*cos - y*sin`, `y*cos + sin*x`
/// (sine stored, cosine as loaded), both results stored.
fn turn(v: [u32; 2], angle: u32, world: &mut impl OpenSpotWorld, p: Precision) -> Result<[u32; 2]> {
    let s = n(world.table_sin(angle)?)?;
    let c = n(world.table_cos(angle)?)?;
    let x = n(v[0])?.mul(&c, p)?.sub(&n(v[1])?.mul(&s, p)?, p)?;
    let y = n(v[1])?.mul(&c, p)?.add(&s.mul(&n(v[0])?, p)?, p)?;
    Ok([x.store_f32()?, y.store_f32()?])
}

/// Complete `554500(point, direction, definition, radius, scale,
/// non_blocking, alternate)` on the GMap (`ret 0x1C`); returns the AL.
///
/// The descriptor is `553000(definition + 38, definition)` (source = the
/// definition), with the definition's virtual +28 shape when `alternate`,
/// required flags 0x400 (| 0x20000000) and mask 0x600 / 0x500 by definition
/// +1C bit 21 (`water`). The point itself is tried first (`552250` with the
/// snap, facing = `direction`): clear answers true with nothing changed.
/// Then up to four rings: the ring offset (0 for the first, then
/// `direction`, then turned by -2pi/3 per ring) times `scale` (stored) plus
/// eight candidates at `radius` along a running unit vector (from
/// `direction`, turned by -pi/4 after every blocked candidate, never reset).
/// Each candidate is `stored(stored(v * radius + offset) + point)`, snapped
/// like the point; its facing is the stored offset times the stored
/// `1 / length` when the unstored length is above `[A3AA10]`, else (0, 0).
/// The first clear candidate becomes the point and its facing the direction
/// (true); four blocked rings answer false.
#[allow(clippy::too_many_arguments)]
pub fn open_spot(
    point: &mut [u32; 2],
    direction: &mut [u32; 2],
    shape: Shape,
    water: bool,
    radius: u32,
    scale: u32,
    non_blocking: bool,
    alternate: bool,
    world: &mut impl OpenSpotWorld,
    p: Precision,
) -> Result<bool> {
    let shape = if alternate { world.alternate_shape()? } else { shape };
    let mut query = OpenLineQuery {
        shape,
        required_actor_flags: REQUIRED_FLAGS | if non_blocking { NON_BLOCKING_FLAG } else { 0 },
        mask: if water { WATER_MASK } else { LAND_MASK },
        facing: *direction,
        position: *point,
    };
    place(&mut query.position, world, p)?;
    if world.first_hit(&query)? == 0 {
        return Ok(true);
    }
    let mut unit = *direction;
    let mut ring_vector = [0u32; 2];
    for ring in 0..4 {
        let offset = [
            n(ring_vector[0])?.mul(&n(scale)?, p)?.store_f32()?,
            n(ring_vector[1])?.mul(&n(scale)?, p)?.store_f32()?,
        ];
        for _ in 0..8 {
            let along = [
                n(unit[0])?.mul(&n(radius)?, p)?.add(&n(offset[0])?, p)?.store_f32()?,
                n(unit[1])?.mul(&n(radius)?, p)?.add(&n(offset[1])?, p)?.store_f32()?,
            ];
            let mut candidate = [
                n(along[0])?.add(&n(point[0])?, p)?.store_f32()?,
                n(along[1])?.add(&n(point[1])?, p)?.store_f32()?,
            ];
            place(&mut candidate, world, p)?;
            let squares = n(along[1])?
                .mul(&n(along[1])?, p)?
                .add(&n(along[0])?.mul(&n(along[0])?, p)?, p)?;
            let length = squares.sqrt(p)?;
            query.facing = if length.compare(&n(SHORT)?) == Ordering::Greater {
                let inverse = n(Binary::from_i32(1).div(&length, p)?.store_f32()?)?;
                [
                    n(along[0])?.mul(&inverse, p)?.store_f32()?,
                    n(along[1])?.mul(&inverse, p)?.store_f32()?,
                ]
            } else {
                [0, 0]
            };
            query.position = candidate;
            if world.first_hit(&query)? == 0 {
                *point = candidate;
                *direction = query.facing;
                return Ok(true);
            }
            unit = turn(unit, CANDIDATE_TURN, world, p)?;
        }
        ring_vector = if ring == 0 { *direction } else { turn(ring_vector, RING_TURN, world, p)? };
    }
    Ok(false)
}
