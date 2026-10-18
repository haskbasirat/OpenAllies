//! Original gameplay range predicates and actor wrapper capture order.
//! The supplied shape is actor virtual+0x14 (target shape), not a rendered mesh
//! or visual scale. Native edge/inversion quirks are retained deliberately.
use crate::{Error, Result, collision::Shape, geometry::Placement};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

fn b(bits: u32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(bits)?)
}
fn stored(value: Binary) -> Result<Binary> {
    b(value.store_f32()?)
}
fn compare_range(threshold: &Binary, distance: &Binary, inverted: bool) -> bool {
    if inverted {
        threshold.compare(distance) != Ordering::Greater
    } else {
        threshold.compare(distance) != Ordering::Less
    }
}
fn validate_shape(shape: &Shape) -> Result<()> {
    if shape.kind > 3 {
        return Err(Error::World(format!(
            "unsupported native range shape {}",
            shape.kind
        )));
    }
    b(shape.radius_bits)?;
    if shape.kind >= 2 {
        for &n in &shape.bounds {
            b(n)?;
        }
    }
    Ok(())
}

///46E290: both ordinary and inverted predicates include equality. Inversion
///is the native minimum-range mode, not the logical negation of a hit test.
pub fn point_delta_in_range(
    delta_bits: [u32; 2],
    range_bits: u32,
    inverted: bool,
    p: Precision,
) -> Result<bool> {
    let x = b(delta_bits[0])?;
    let y = b(delta_bits[1])?;
    let range = b(range_bits)?;
    let distance = y.mul(&y, p)?.add(&x.mul(&x, p)?, p)?;
    Ok(compare_range(&range.mul(&range, p)?, &distance, inverted))
}

///46E2D0's local rectangle versus inflated point. Inside the rectangle returns
///true even in minimum-range mode; preserving that behavior avoids changing
///native weapon/order range decisions at overlapping gameplay shapes.
fn rectangle_range(
    shape: &Shape,
    other_radius: u32,
    delta: [u32; 2],
    range: u32,
    inverted: bool,
    p: Precision,
) -> Result<bool> {
    let x = b(delta[0])?;
    let y = b(delta[1])?;
    let [min_x, min_y, max_x, max_y] = shape.bounds.map(b);
    let (min_x, min_y, max_x, max_y) = (min_x?, min_y?, max_x?, max_y?);
    let x_distance = if x.compare(&min_x) == Ordering::Less {
        Some(x.sub(&min_x, p)?)
    } else if x.compare(&max_x) == Ordering::Greater {
        Some(x.sub(&max_x, p)?)
    } else {
        None
    };
    let y_distance = if y.compare(&min_y) == Ordering::Less {
        Some(y.sub(&min_y, p)?)
    } else if y.compare(&max_y) == Ordering::Greater {
        Some(y.sub(&max_y, p)?)
    } else {
        None
    };
    if x_distance.is_none() && y_distance.is_none() {
        return Ok(true);
    }
    let radius = b(range)?.add(&b(other_radius)?, p)?;
    let absolute = |v: Binary| {
        if v.is_negative_nonzero() {
            v.negated()
        } else {
            v
        }
    };
    let (threshold, distance) = match (x_distance, y_distance) {
        (Some(x), Some(y)) => (
            radius.mul(&radius, p)?,
            y.mul(&y, p)?.add(&x.mul(&x, p)?, p)?,
        ),
        (Some(x), None) => (radius, absolute(x)),
        (None, Some(y)) => (radius, absolute(y)),
        (None, None) => unreachable!(),
    };
    Ok(compare_range(&threshold, &distance, inverted))
}

///46EB70. Primary is the higher-kind/larger-radius shape selected by46F100.
fn local_range(
    primary: &Shape,
    other: &Shape,
    delta: [u32; 2],
    range: u32,
    inverted: bool,
    p: Precision,
) -> Result<bool> {
    match primary.kind {
        0 => point_delta_in_range(delta, range, inverted, p),
        1 => {
            let radius = if other.kind == 0 {
                b(range)?.add(&b(primary.radius_bits)?, p)?
            } else {
                b(other.radius_bits)?
                    .add(&b(primary.radius_bits)?, p)?
                    .add(&b(range)?, p)?
            };
            let x = b(delta[0])?;
            let y = b(delta[1])?;
            let distance = y.mul(&y, p)?.add(&x.mul(&x, p)?, p)?;
            Ok(compare_range(&radius.mul(&radius, p)?, &distance, inverted))
        }
        2 | 3 => rectangle_range(primary, other.radius_bits, delta, range, inverted, p),
        _ => Err(Error::World("unsupported native range shape".into())),
    }
}

///46F100. Sort operands by original kind/radius, translate into the selected
///shape's local frame, and preserve its asymmetric intermediate float32 stores.
///The native rectangle branch uses the other shape's authored radius; this is
///not a replacement SAT algorithm or a circle inferred from render bounds.
pub fn shapes_in_range(
    a: &Placement,
    b_placement: &Placement,
    range_bits: u32,
    inverted: bool,
    p: Precision,
) -> Result<bool> {
    validate_shape(&a.shape)?;
    validate_shape(&b_placement.shape)?;
    b(range_bits)?;
    for value in a
        .position
        .iter()
        .chain(&a.facing)
        .chain(&b_placement.position)
        .chain(&b_placement.facing)
    {
        b(*value)?;
    }
    let swap = a.shape.kind < b_placement.shape.kind
        || (a.shape.kind == b_placement.shape.kind
            && b(a.shape.radius_bits)?.compare(&b(b_placement.shape.radius_bits)?)
                == Ordering::Less);
    let (primary, other) = if swap {
        (b_placement, a)
    } else {
        (a, b_placement)
    };
    let dy = stored(b(other.position[1])?.sub(&b(primary.position[1])?, p)?)?;
    let dx = b(other.position[0])?.sub(&b(primary.position[0])?, p)?;
    let delta = if primary.shape.kind <= 1 {
        [dx.store_f32()?, dy.store_f32()?]
    } else {
        let stored_dx = stored(dx.clone())?;
        let fx = b(primary.facing[0])?;
        let fy = b(primary.facing[1])?;
        [
            dx.mul(&fy, p)?.sub(&dy.mul(&fx, p)?, p)?.store_f32()?,
            dy.mul(&fy, p)?
                .add(&stored_dx.mul(&fx, p)?, p)?
                .store_f32()?,
        ]
    };
    //If two boxes have unequal facing,46F100 directly calls46E2D0. Otherwise
    //46EB70's box branch forwards precisely the same values to46E2D0.
    local_range(&primary.shape, &other.shape, delta, range_bits, inverted, p)
}

///46F230 supplies the original point descriptor: kind/radius0, facing(0,-1).
pub fn shape_point_in_range(
    shape: &Placement,
    point: [u32; 2],
    range_bits: u32,
    inverted: bool,
    p: Precision,
) -> Result<bool> {
    shapes_in_range(
        shape,
        &Placement {
            shape: Shape {
                kind: 0,
                radius_bits: 0,
                bounds: [0; 4],
            },
            facing: [0, 0xbf800000],
            position: point,
        },
        range_bits,
        inverted,
        p,
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorTransform {
    pub position: [u32; 2],
    pub facing: [u32; 2],
}
pub trait ActorRangeWorld {
    type ShapeHandle;
    ///Snapshot actor+20/+24 and+2C/+30 before invoking its range-shape getter.
    fn snapshot_transform(&mut self, actor_id: u32) -> Result<ActorTransform>;
    ///Actor virtual+14. Ordinary actors use definition+68 when byte+80 is set,
    ///otherwise definition+50. Derived overrides remain the caller's boundary.
    fn range_shape(&mut self, actor_id: u32) -> Result<Self::ShapeHandle>;
    ///Read the retained shape identity after the wrapper has run both getters.
    ///The original descriptor stores a pointer, so another getter may change
    ///that shape before the comparison; cloning at first getter is incorrect.
    fn read_shape(&self, handle: &Self::ShapeHandle) -> Result<Shape>;
}
fn capture<W: ActorRangeWorld>(id: u32, world: &mut W) -> Result<(W::ShapeHandle, ActorTransform)> {
    let snapshot = world.snapshot_transform(id)?;
    let shape = world.range_shape(id)?;
    Ok((shape, snapshot))
}
fn resolve<W: ActorRangeWorld>(
    (handle, snapshot): (W::ShapeHandle, ActorTransform),
    world: &W,
) -> Result<Placement> {
    Ok(Placement {
        shape: world.read_shape(&handle)?,
        facing: snapshot.facing,
        position: snapshot.position,
    })
}
///4EE800 captures target first, then source. Getter side effects can therefore
///change the source before its snapshot; each own descriptor retains pre-call XY.
pub fn actors_in_range<W: ActorRangeWorld>(
    source: u32,
    target: u32,
    range_bits: u32,
    inverted: bool,
    p: Precision,
    world: &mut W,
) -> Result<bool> {
    let target = capture(target, world)?;
    let source = capture(source, world)?;
    shapes_in_range(
        &resolve(source, world)?,
        &resolve(target, world)?,
        range_bits,
        inverted,
        p,
    )
}
///4EE7C0 copies the point arguments before capturing the source descriptor.
pub fn actor_point_in_range<W: ActorRangeWorld>(
    source: u32,
    point: [u32; 2],
    range_bits: u32,
    inverted: bool,
    p: Precision,
    world: &mut W,
) -> Result<bool> {
    let captured = capture(source, world)?;
    shape_point_in_range(&resolve(captured, world)?, point, range_bits, inverted, p)
}
