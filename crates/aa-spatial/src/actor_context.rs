//! Original554350 constructor used by movement/path callers. Its final call is
//! directly5531F0(actor, strict), with no caller-added flags. The native default
//! shape/pose is fully overwritten for ordinary immutable actor getters.
use crate::{
    Result,
    index::{ActorKey, SpatialIndex},
    query::{PreparedQuery, QueryPolicy},
};
use aa_numeric::{Binary, Precision};

/// Exact ordinary554350 entry, distinct from callers that add extra mask bits
/// after5531F0 (for example the57C080 regiment probe). Source evidence executes
///554350 itself, including its initializer and original5531F0/5522D0 bodies.
///This accepts only a live actor in SpatialIndex's ordinary immutable-getter
///domain. It does not fabricate an empty query when the source actor is absent.
pub fn prepare<P: QueryPolicy>(
    index: &SpatialIndex,
    actor: ActorKey,
    strict: bool,
    policy: &P,
) -> Result<PreparedQuery> {
    PreparedQuery::for_actor(index, actor, strict, 0, policy)
}

///5522B0 writes facing directly and invalidates the native orientation cache.
///PreparedQuery computes orientation when used, so there is no stale cache.
///The original does not normalize this vector or replace zero direction.
pub fn set_facing(query: &mut PreparedQuery, facing: [u32; 2]) {
    query.placement.facing = facing;
}

///531170 stores both direction*distance products before adding the live actor
///position. The caller retains its native constructor/owner-read ordering.
pub fn current_actor_goal(
    position: [u32; 2],
    direction: [u32; 2],
    distance: u32,
    precision: Precision,
) -> Result<[u32; 2]> {
    goal(position, direction, distance, precision, true)
}
///5312C0 explicit-origin arithmetic keeps X product extended, while storing Y
///product before addition. This difference is observable near rounding edges.
pub fn explicit_origin_goal(
    position: [u32; 2],
    direction: [u32; 2],
    distance: u32,
    precision: Precision,
) -> Result<[u32; 2]> {
    goal(position, direction, distance, precision, false)
}
fn goal(
    position: [u32; 2],
    direction: [u32; 2],
    distance: u32,
    p: Precision,
    store_x: bool,
) -> Result<[u32; 2]> {
    let distance = Binary::from_f32_bits(distance)?;
    let mut x = Binary::from_f32_bits(direction[0])?.mul(&distance, p)?;
    if store_x {
        x = Binary::from_f32_bits(x.store_f32()?)?;
    }
    let y = Binary::from_f32_bits(
        Binary::from_f32_bits(direction[1])?
            .mul(&distance, p)?
            .store_f32()?,
    )?;
    Ok([
        x.add(&Binary::from_f32_bits(position[0])?, p)?
            .store_f32()?,
        y.add(&Binary::from_f32_bits(position[1])?, p)?
            .store_f32()?,
    ])
}

#[cfg(test)]
#[path = "actor_context_tests.rs"]
mod tests;
