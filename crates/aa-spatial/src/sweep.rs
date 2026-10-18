//! Original553CE0 endpoint-then-rectangle spatial sweep. The source algorithm
//! retains supplied facing and its native scalar stores; it is not a ray march.
use crate::{
    Result,
    collision::{Descriptor, Shape},
    geometry::Placement,
    index::SpatialIndex,
    query::{self, Hit, PreparedQuery, QueryGeometry, QueryPolicy},
};
use aa_numeric::{Binary, Precision};
fn b(bits: u32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(bits)?)
}
fn stored(value: Binary) -> Result<Binary> {
    b(value.store_f32()?)
}

///46EB30→46E0E0; constructor selects SQUARE when width and length compare equal.
fn rectangle(width: u32, length: u32, p: Precision) -> Result<Shape> {
    let width = b(width)?;
    let length = b(length)?;
    let half = b(0x3f000000)?;
    let negative_width = width.mul(&b(0xbf000000)?, p)?;
    let positive_width = width.mul(&half, p)?;
    let negative_length = length.mul(&b(0xbf000000)?, p)?;
    let positive_length = length.mul(&half, p)?;
    let radius = positive_length
        .mul(&positive_length, p)?
        .add(&negative_width.mul(&negative_width, p)?, p)?
        .sqrt(p)?;
    Ok(Shape {
        kind: if width.compare(&length).is_eq() { 2 } else { 3 },
        radius_bits: radius.store_f32()?,
        bounds: [
            negative_width.store_f32()?,
            negative_length.store_f32()?,
            positive_width.store_f32()?,
            positive_length.store_f32()?,
        ],
    })
}
/// Pure continuation rectangle after a clear endpoint. The caller supplies the
/// original position snapshot and stored XY deltas, not the resnapped position.
fn corridor(
    query: &PreparedQuery,
    start: [u32; 2],
    goal: [u32; 2],
    delta: [u32; 2],
    p: Precision,
) -> Result<PreparedQuery> {
    let s = &query.placement.shape;
    let radius = if s.kind == 2 || s.kind == 3 {
        if b(s.bounds[2])?.compare(&b(s.bounds[3])?).is_lt() {
            s.bounds[2]
        } else {
            s.bounds[3]
        }
    } else {
        s.radius_bits
    };
    let distance_squared = stored(
        b(delta[0])?
            .mul(&b(delta[0])?, p)?
            .add(&b(delta[1])?.mul(&b(delta[1])?, p)?, p)?,
    )?;
    let length = distance_squared.sqrt(p)?.store_f32()?;
    let width = b(radius)?.add(&b(radius)?, p)?.store_f32()?;
    let midpoint = [
        stored(b(start[0])?.add(&b(goal[0])?, p)?)?
            .mul(&b(0x3f000000)?, p)?
            .store_f32()?,
        b(goal[1])?
            .add(&b(start[1])?, p)?
            .mul(&b(0x3f000000)?, p)?
            .store_f32()?,
    ];
    Ok(PreparedQuery {
        placement: Placement {
            shape: rectangle(width, length, p)?,
            facing: query.placement.facing,
            position: midpoint,
        },
        descriptor: Descriptor {
            source_actor: query.descriptor.source_actor,
            ignore_actor: query.descriptor.ignore_actor,
            ignore_definition: query.descriptor.ignore_definition,
            mask: query.descriptor.mask,
            ..Descriptor::default()
        },
        required_actor_flags: query.required_actor_flags,
    })
}
///553CE0 mutates the external descriptor while testing the endpoint, then calls
///552250 again to restore its original XY. Completed effects are not rolled back
///on errors: if an endpoint service fails, the descriptor remains at that stage.
pub fn sweep<P: QueryPolicy>(
    index: &mut SpatialIndex,
    geometry: &QueryGeometry,
    query: &mut PreparedQuery,
    goal: [u32; 2],
    policy: &mut P,
) -> Result<Hit> {
    let p = index.precision();
    let start = query.placement.position;
    let dy = b(goal[1])?.sub(&b(start[1])?, p)?;
    let dx = b(goal[0])?.sub(&b(start[0])?, p)?;
    let delta = [dx.store_f32()?, dy.store_f32()?];
    if dx
        .mul(&dx, p)?
        .add(&dy.mul(&dy, p)?, p)?
        .compare(&b(0x38d1b717)?)
        .is_lt()
    {
        return query::first_hit(index, geometry, query, policy);
    }
    query.set_position(index, goal, policy)?;
    let endpoint = query::first_hit(index, geometry, query, policy)?;
    query.set_position(index, start, policy)?;
    if endpoint.blocked() {
        return Ok(endpoint);
    }
    let corridor = corridor(query, start, goal, delta, p)?;
    query::first_hit(index, geometry, &corridor, policy)
}

#[cfg(test)]
#[path = "sweep_tests.rs"]
mod tests;
