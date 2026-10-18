//! Original free-placement search `0x554850` with the shape query constructor
//! `0x552f70`, and the definition wrapper radius rule of `0x554ad0`.
//!
//! The search first tests the point itself, then rings `k = 1..=rings` at
//! distance `k * radius` along the direction, rotating the offset (and, unless
//! the direction is kept, the facing) eight times by the native table values
//! for -pi/4. The first clear first-hit query (`0x553ba0`) moves the point.
use crate::{
    Result,
    collision::{Descriptor, Shape},
    geometry::Placement,
    index::SpatialIndex,
    query::{PreparedQuery, QueryGeometry, QueryPolicy, first_hit},
};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// `0x85daa0(0xbf490fdb)` and `0x85da80(0xbf490fdb)`: the native 4096-entry
/// table lookups of -pi/4 (index 3584) return binary32 0xbf3504ef and
/// 0x3f3504f7 under every precision (oracle-recorded; aa-trig owns the
/// tables). Used exactly as returned.
pub const ROTATION_SIN_BITS: u32 = 0xbf35_04ef;
pub const ROTATION_COS_BITS: u32 = 0x3f35_04f7;

fn b(bits: u32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(bits)?)
}
fn store(value: &Binary) -> Result<u32> {
    Ok(value.store_f32()?)
}

/// Inputs of `0x554850` beyond the point and direction.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlacementSearch {
    /// Definition +0x38 blocking shape (copied by `0x552f70`).
    pub shape: Shape,
    pub radius_bits: u32,
    /// Signed ring count; fewer than one tests only the point.
    pub rings: i32,
    /// Native byte argument 6: keep the caller's direction/facing.
    pub keep_direction: bool,
    /// Native byte 7: required actor flags also include 0x20000000.
    pub strict: bool,
    /// Native byte 8 (definition +0x1C bit21): mask 0x600 instead of 0x500.
    pub large: bool,
}

/// `0x554ad0` radius: definition +0x3C unless below cell size * 0.25 (then
/// that binary32 product).
pub fn wrapper_radius(definition_radius_bits: u32, cell_size_bits: u32, p: Precision) -> Result<u32> {
    let floor = b(cell_size_bits)?.mul(&b(0x3e80_0000)?, p)?;
    if b(definition_radius_bits)?.compare(&floor) == Ordering::Less {
        return store(&floor);
    }
    Ok(definition_radius_bits)
}

/// Complete `0x554850`. Returns the native AL; on success `point` (and,
/// without `keep_direction`, `direction`) hold the accepted placement.
pub fn find_free_placement<P: QueryPolicy>(
    index: &mut SpatialIndex,
    geometry: &QueryGeometry,
    point: &mut [u32; 2],
    direction: &mut [u32; 2],
    search: &PlacementSearch,
    policy: &mut P,
) -> Result<bool> {
    let p = index.precision();
    let mut descriptor = Descriptor::default();
    descriptor.mask |= if search.large { 0x600 } else { 0x500 };
    let mut query = PreparedQuery {
        placement: Placement { shape: search.shape.clone(), facing: *direction, position: [0, 0] },
        descriptor,
        required_actor_flags: 0x400 | if search.strict { 0x2000_0000 } else { 0 },
    };
    query.set_position(index, *point, policy)?;
    query.placement.facing = *direction;
    if !first_hit(index, geometry, &query, policy)?.blocked() {
        return Ok(true);
    }
    let sin = b(ROTATION_SIN_BITS)?;
    let cos = b(ROTATION_COS_BITS)?;
    let mut ring: i32 = 1;
    while ring <= search.rings {
        let mut turned = *direction;
        let distance = Binary::from_i32(ring).mul(&b(search.radius_bits)?, p)?;
        let mut offset = [
            store(&distance.mul(&b(direction[0])?, p)?)?,
            store(&distance.mul(&b(direction[1])?, p)?)?,
        ];
        for _ in 0..8 {
            let candidate = [
                store(&b(offset[0])?.add(&b(point[0])?, p)?)?,
                store(&b(offset[1])?.add(&b(point[1])?, p)?)?,
            ];
            query.placement.position = candidate;
            if !search.keep_direction {
                query.placement.facing = turned;
            }
            if !first_hit(index, geometry, &query, policy)?.blocked() {
                *point = [
                    store(&b(offset[0])?.add(&b(point[0])?, p)?)?,
                    store(&b(offset[1])?.add(&b(point[1])?, p)?)?,
                ];
                if !search.keep_direction {
                    *direction = turned;
                }
                return Ok(true);
            }
            let [ox, oy] = offset.map(b);
            let (ox, oy) = (ox?, oy?);
            offset = [
                store(&ox.mul(&cos, p)?.sub(&oy.mul(&sin, p)?, p)?)?,
                store(&oy.mul(&cos, p)?.add(&sin.mul(&ox, p)?, p)?)?,
            ];
            let [dx, dy] = turned.map(b);
            let (dx, dy) = (dx?, dy?);
            turned = [
                store(&dx.mul(&cos, p)?.sub(&dy.mul(&sin, p)?, p)?)?,
                store(&dy.mul(&cos, p)?.add(&sin.mul(&dx, p)?, p)?)?,
            ];
        }
        ring += 1;
    }
    Ok(false)
}

/// Original `0x554b30` with the definition shape (+0x38) branch: radius =
/// definition +0x3C (the radius word of that +0x38 blocking shape), raised to
/// the stored cell size * 0.25; ring count =
/// truncated `distance / radius` clamped to the native 1..=20; ring step =
/// stored `distance / rings`; the direction is kept.
#[allow(clippy::too_many_arguments)]
pub fn find_placement_within<P: QueryPolicy>(
    index: &mut SpatialIndex,
    geometry: &QueryGeometry,
    point: &mut [u32; 2],
    direction: &mut [u32; 2],
    shape: Shape,
    definition_radius_bits: u32,
    distance_bits: u32,
    strict: bool,
    large: bool,
    policy: &mut P,
) -> Result<bool> {
    let p = index.precision();
    let floor = store(&b(index.grid().cell_size_bits)?.mul(&b(0x3e80_0000)?, p)?)?;
    let radius = if b(definition_radius_bits)?.compare(&b(floor)?) == Ordering::Less {
        floor
    } else {
        definition_radius_bits
    };
    let rings = b(distance_bits)?.div(&b(radius)?, p)?.truncate_i32()?.clamp(1, 20);
    let step = store(&b(distance_bits)?.div(&Binary::from_i32(rings), p)?)?;
    let search = PlacementSearch { shape, radius_bits: step, rings, keep_direction: true, strict, large };
    find_free_placement(index, geometry, point, direction, &search, policy)
}

#[cfg(test)]
#[path = "placement_tests.rs"]
mod tests;
