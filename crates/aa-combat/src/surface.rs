//! Original target-hit position4EEEE0/4EEE60 and shape projection46E150.
//! Gameplay target shape comes from actor virtual+14, never the rendered mesh.
use crate::Result;
use aa_body::F32;
use aa_numeric::{Binary, Precision};
use aa_spatial::collision::Shape;
use std::cmp::Ordering;
fn n(f: F32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(f.0)?)
}
fn b(bits: u32) -> Result<Binary> {
    n(F32(bits))
}
fn clamp(value: Binary, low: u32, high: u32) -> Result<Binary> {
    if value.compare(&b(low)?) == Ordering::Less {
        b(low)
    } else if value.compare(&b(high)?) == Ordering::Greater {
        b(high)
    } else {
        Ok(value)
    }
}
/// Original46E150. The box X projection stores f32 before clamping; Y remains
/// extended. Unknown/point kinds read startup globals supplied by the caller.
pub fn shape_projection(
    shape: &Shape,
    facing: [F32; 2],
    direction: [F32; 2],
    default_point: [F32; 2],
    p: Precision,
) -> Result<[F32; 2]> {
    match shape.kind {
        1 => {
            let direction = crate::projectile::normalized_xy(
                [F32::ZERO; 3],
                [direction[0], direction[1], F32::ZERO],
                p,
            )?;
            Ok([
                F32(n(direction[0])?
                    .mul(&b(shape.radius_bits)?, p)?
                    .store_f32()?),
                F32(n(direction[1])?
                    .mul(&b(shape.radius_bits)?, p)?
                    .store_f32()?),
            ])
        }
        2 | 3 => {
            let x = b(n(facing[1])?
                .mul(&n(direction[0])?, p)?
                .sub(&n(direction[1])?.mul(&n(facing[0])?, p)?, p)?
                .store_f32()?)?;
            let y = n(facing[1])?
                .mul(&n(direction[1])?, p)?
                .add(&n(facing[0])?.mul(&n(direction[0])?, p)?, p)?;
            let x = clamp(x, shape.bounds[0], shape.bounds[2])?;
            let y = clamp(y, shape.bounds[1], shape.bounds[3])?;
            let out_x = n(facing[0])?
                .mul(&y, p)?
                .add(&n(facing[1])?.mul(&x, p)?, p)?;
            let out_y = y
                .mul(&n(facing[1])?, p)?
                .sub(&n(facing[0])?.mul(&x, p)?, p)?;
            Ok([F32(out_x.store_f32()?), F32(out_y.store_f32()?)])
        }
        _ => Ok(default_point),
    }
}
/// Native46E4F0: translate the input into a placed shape's coordinates,
/// preserve both binary32 delta stores, run46E150, then translate back.
pub fn placed_projection(
    placement: &aa_spatial::geometry::Placement,
    point: [u32; 2],
    default_point: [u32; 2],
    precision: Precision,
) -> Result<[u32; 2]> {
    let mut delta = [F32::ZERO; 2];
    for i in 0..2 {
        delta[i] = F32(b(point[i])?.sub(&b(placement.position[i])?, precision)?.store_f32()?);
    }
    let offset = shape_projection(
        &placement.shape,
        placement.facing.map(F32),
        delta,
        default_point.map(F32),
        precision,
    )?;
    Ok([
        b(placement.position[0])?.add(&n(offset[0])?, precision)?.store_f32()?,
        n(offset[1])?.add(&b(placement.position[1])?, precision)?.store_f32()?,
    ])
}
pub trait World {
    /// Original actor virtual0, extended return; may mutate actor state.
    fn actor_radius(&mut self, actor: u32) -> Result<Binary>;
    fn actor_facing(&self, actor: u32) -> Result<[F32; 2]>;
    /// Original virtual+14. Ordinary actor chooses definition+68 if target override
    /// byte+80 is set, otherwise soft shape+50. No object-scale substitution.
    fn target_shape(&mut self, actor: u32) -> Result<Shape>;
    fn actor_position(&self, actor: u32) -> Result<[F32; 3]>;
    fn default_point(&self) -> Result<[F32; 2]>;
    fn target_height_ratio(&self) -> Result<F32>;
    /// Original actor virtual+8. Z and ratio are captured before this callback.
    fn actor_height(&mut self, actor: u32) -> Result<Binary>;
}
/// Original4EEE60. The direction is negated before multiplying by actor radius;
/// its X negation stores f32 first. Facing precedes target-shape lookup, but
/// actor position is read afterwards, preserving getter side effects.
pub fn ground_position(
    actor: u32,
    direction: [F32; 2],
    world: &mut impl World,
    p: Precision,
) -> Result<[F32; 3]> {
    let radius = world.actor_radius(actor)?;
    let neg = F32(0xbf800000);
    let x = b(n(direction[0])?.mul(&n(neg)?, p)?.store_f32()?)?.mul(&radius, p)?;
    let y = n(direction[1])?.mul(&n(neg)?, p)?.mul(&radius, p)?;
    let facing = world.actor_facing(actor)?;
    let input = [F32(x.store_f32()?), F32(y.store_f32()?)];
    let shape = world.target_shape(actor)?;
    let fallback = if matches!(shape.kind, 1..=3) {
        [F32::ZERO; 2]
    } else {
        world.default_point()?
    };
    let offset = shape_projection(&shape, facing, input, fallback, p)?;
    let position = world.actor_position(actor)?;
    Ok([
        F32(n(offset[0])?.add(&n(position[0])?, p)?.store_f32()?),
        F32(n(offset[1])?.add(&n(position[1])?, p)?.store_f32()?),
        F32::ZERO,
    ])
}
/// Original4EEEE0. Input Z is ignored by the original surface caller.
pub fn hit_position(
    actor: u32,
    direction: [F32; 3],
    world: &mut impl World,
    p: Precision,
) -> Result<[F32; 3]> {
    let surface = ground_position(actor, [direction[0], direction[1]], world, p)?;
    let z = world.actor_position(actor)?[2];
    let ratio = world.target_height_ratio()?;
    let height = world.actor_height(actor)?;
    let out_z = height
        .mul(&n(ratio)?, p)?
        .add(&n(z)?.add(&n(surface[2])?, p)?, p)?;
    Ok([surface[0], surface[1], F32(out_z.store_f32()?)])
}
