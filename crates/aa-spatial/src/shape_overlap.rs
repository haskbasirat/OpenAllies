//! Original46EF30 dispatch and rotated three-corner footprint overlap.
//! Authoritative shape words/facing are used directly, without render bounds,
//! normalized axes, replacement SAT tolerances, or a default x87 precision.
use crate::{Error, Result, collision::Shape, geometry::Placement};
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;
fn b(v: u32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(v)?)
}
///FUCOMPP equality, including unordered NaN and equal signed zeros. The
///native facing comparison does not require a finite arithmetic operand.
///This bool geometry API does not model x87 exception/status flag effects.
fn raw_equal(a:u32,b:u32)->bool {
    let nan=|v:u32|v&0x7f800000==0x7f800000&&v&0x007fffff!=0;
    !nan(a)&&!nan(b)&&(a==b||(a&0x7fffffff==0&&b&0x7fffffff==0))
}
fn store(v: Binary) -> Result<Binary> {
    b(v.store_f32()?)
}
fn validate(s: &Shape) -> Result<()> {
    if s.kind > 3 {
        return Err(Error::World(format!(
            "unsupported native overlap shape {}",
            s.kind
        )));
    }
    for v in std::iter::once(&s.radius_bits).chain(&s.bounds) {
        b(*v)?;
    }
    Ok(())
}
fn dot(a: &[Binary; 2], v: &[Binary; 2], p: Precision) -> Result<Binary> {
    Ok(a[1].mul(&v[1], p)?.add(&a[0].mul(&v[0], p)?, p)?)
}

///46E7D0→46DD10 mode0, called by46EF30 after doubling authored maxX/maxY.
///The native representation stores three consecutive corners; the fourth is
///derived by46E810. Asymmetric intermediate stores are deliberate.
pub fn footprint(placement: &Placement, p: Precision) -> Result<[[u32; 2]; 3]> {
    validate(&placement.shape)?;
    if placement.shape.kind < 2 {
        return Err(Error::World("rotated footprint requires native box".into()));
    }
    let fx = b(placement.facing[0])?;
    let fy = b(placement.facing[1])?;
    let width = store(b(placement.shape.bounds[2])?.add(&b(placement.shape.bounds[2])?, p)?)?;
    let length = store(b(placement.shape.bounds[3])?.add(&b(placement.shape.bounds[3])?, p)?)?;
    let half = b(0x3f000000)?;
    let forward_x = length.mul(&fx, p)?.mul(&half, p)?;
    let forward_y = store(length.mul(&fy, p)?)?.mul(&half, p)?;
    let stored_forward_y = store(forward_y.clone())?;
    let side_x = store(fy.negated().mul(&width, p)?.mul(&half, p)?)?;
    let side_y = store(fx.mul(&width, p)?.mul(&half, p)?)?;
    let x = b(placement.position[0])?;
    let y = b(placement.position[1])?;
    Ok([
        [
            x.sub(&forward_x, p)?.sub(&side_x, p)?.store_f32()?,
            store(y.sub(&forward_y, p)?)?.sub(&side_y, p)?.store_f32()?,
        ],
        [
            forward_x.add(&x, p)?.sub(&side_x, p)?.store_f32()?,
            store(forward_y.add(&y, p)?)?.sub(&side_y, p)?.store_f32()?,
        ],
        [
            forward_x.add(&x, p)?.add(&side_x, p)?.store_f32()?,
            store(stored_forward_y.add(&y, p)?)?
                .add(&side_y, p)?
                .store_f32()?,
        ],
    ])
}

///46E810. A direct mathematical port of native corner choice and eight signed
///projections, retaining each original float32 store before later operations.
pub fn footprints_overlap(a: [[u32; 2]; 3], other: [[u32; 2]; 3], p: Precision) -> Result<bool> {
    let a = [a[0].map(b), a[1].map(b), a[2].map(b)].map(|v| {
        let [x, y] = v;
        Ok::<_, Error>([x?, y?])
    });
    let [a0, a1, a2] = a;
    let (a0, a1, a2) = (a0?, a1?, a2?);
    let other = [other[0].map(b), other[1].map(b), other[2].map(b)].map(|v| {
        let [x, y] = v;
        Ok::<_, Error>([x?, y?])
    });
    let [b0, b1, b2] = other;
    let (b0, b1, b2) = (b0?, b1?, b2?);
    //46E813..46E875: fourth-corner X stores before subtracting origin, Y does not.
    let a_four_x = store(a2[0].sub(&a1[0].sub(&a0[0], p)?, p)?)?;
    let a_four_y = a2[1].sub(&a1[1].sub(&a0[1], p)?, p)?;
    let b_four = [
        store(b2[0].sub(&b1[0].sub(&b0[0], p)?, p)?)?,
        store(b2[1].sub(&b1[1].sub(&b0[1], p)?, p)?)?,
    ];
    let axis_a = [store(a1[0].sub(&a0[0], p)?)?, store(a1[1].sub(&a0[1], p)?)?];
    let side_a = [
        store(a_four_x.sub(&a0[0], p)?)?,
        store(a_four_y.sub(&a0[1], p)?)?,
    ];
    let axis_b = [store(b1[0].sub(&b0[0], p)?)?, store(b1[1].sub(&b0[1], p)?)?];
    let side_b = [
        store(b_four[0].sub(&b0[0], p)?)?,
        store(b_four[1].sub(&b0[1], p)?)?,
    ];
    let first_negative = dot(&axis_b, &axis_a, p)?.is_negative_nonzero();
    let second_negative = dot(&axis_b, &side_a, p)?.is_negative_nonzero();
    //Native esp+8/+C and+10/+14 after its four orientation branches.
    let (corner, edge, side) = match (first_negative, second_negative) {
        (false, false) => (b0.clone(), axis_b, side_b),
        (false, true) => (
            b1.clone(),
            side_b,
            [store(b0[0].sub(&b1[0], p)?)?, store(b0[1].sub(&b1[1], p)?)?],
        ),
        (true, false) => (
            b_four,
            [side_b[0].clone().negated(), side_b[1].clone().negated()],
            axis_b,
        ),
        (true, true) => (
            b2,
            [axis_b[0].clone().negated(), axis_b[1].clone().negated()],
            [side_b[0].clone().negated(), side_b[1].clone().negated()],
        ),
    };
    let dx = store(corner[0].sub(&a0[0], p)?)?;
    //46E979 leaves relativeY in x87 until all following projection vertices exist.
    let dy = corner[1].sub(&a0[1], p)?;
    let neg_x = edge[0].clone().negated();
    let neg_y = edge[1].clone().negated();
    let low = [store(neg_x.sub(&dx, p)?)?, store(neg_y.sub(&dy, p)?)?];
    let middle = [
        store(neg_x.sub(&side[0], p)?.add(&axis_a[0], p)?.sub(&dx, p)?)?,
        store(
            store(neg_y.sub(&side[1], p)?)?
                .add(&axis_a[1], p)?
                .sub(&dy, p)?,
        )?,
    ];
    let high = [
        store(store(side_a[0].add(&axis_a[0], p)?.sub(&side[0], p)?)?.sub(&dx, p)?)?,
        store(
            side_a[1]
                .add(&axis_a[1], p)?
                .sub(&side[1], p)?
                .sub(&dy, p)?,
        )?,
    ];
    let last = [
        store(side_a[0].sub(&dx, p)?)?,
        store(side_a[1].sub(&dy, p)?)?,
    ];
    let zero = Binary::from_u32(0);
    for (point, axis, want_nonpositive) in [
        (&low, &axis_a, true),
        (&high, &axis_a, false),
        (&middle, &side_a, true),
        (&last, &side_a, false),
        (&low, &edge, true),
        (&high, &edge, false),
        (&middle, &side, true),
        (&last, &side, false),
    ] {
        let comparison = dot(point, axis, p)?.compare(&zero);
        if if want_nonpositive {
            comparison == Ordering::Greater
        } else {
            comparison == Ordering::Less
        } {
            return Ok(false);
        }
    }
    Ok(true)
}

fn rectangle_circle(shape: &Shape, radius: u32, delta: [u32; 2], p: Precision) -> Result<bool> {
    let outside = |value: u32, lo: u32, hi: u32| -> Result<Option<Binary>> {
        let value = b(value)?;
        let (lo, hi) = (b(lo)?, b(hi)?);
        if value.compare(&lo) == Ordering::Less {
            Ok(Some(lo.sub(&value, p)?))
        } else if value.compare(&hi) == Ordering::Greater {
            Ok(Some(value.sub(&hi, p)?))
        } else {
            Ok(None)
        }
    };
    let r = b(radius)?;
    match (
        outside(delta[0], shape.bounds[0], shape.bounds[2])?,
        outside(delta[1], shape.bounds[1], shape.bounds[3])?,
    ) {
        (None, None) => Ok(true),
        (Some(v), None) | (None, Some(v)) => Ok(v.compare(&r) == Ordering::Less),
        (Some(x), Some(y)) => Ok(r
            .mul(&r, p)?
            .compare(&y.mul(&y, p)?.add(&x.mul(&x, p)?, p)?)
            == Ordering::Greater),
    }
}
///46D9C0 aligns authored rectangle extents with the translated second shape.
///The 0.0001 epsilon is applied here only; rotated46E810 uses signed projections.
fn aligned_boxes(a: &Shape, other: &Shape, delta: [u32; 2], p: Precision) -> Result<bool> {
    let x = b(delta[0])?;
    let y = b(delta[1])?;
    let epsilon = b(0x38d1b717)?;
    Ok(x.add(&b(other.bounds[2])?, p)?
        .sub(&epsilon, p)?
        .compare(&b(a.bounds[0])?)
        != Ordering::Less
        && x.add(&b(other.bounds[0])?, p)?
            .add(&epsilon, p)?
            .compare(&b(a.bounds[2])?)
            != Ordering::Greater
        && y.add(&b(other.bounds[1])?, p)?
            .add(&epsilon, p)?
            .compare(&b(a.bounds[3])?)
            != Ordering::Greater
        && y.add(&b(other.bounds[3])?, p)?
            .sub(&epsilon, p)?
            .compare(&b(a.bounds[1])?)
            != Ordering::Less)
}

///46EF30. Shape ordering, local-frame stores and equal-facing fast path match
///the native dispatch. This is gameplay geometry, without a visual scale factor.
pub fn overlaps(a: &Placement, other: &Placement, p: Precision) -> Result<bool> {
    validate(&a.shape)?;
    validate(&other.shape)?;
    for v in a.position.iter().chain(&other.position) {
        b(*v)?;
    }
    let swap = a.shape.kind < other.shape.kind
        || (a.shape.kind == other.shape.kind
            && b(a.shape.radius_bits)?.compare(&b(other.shape.radius_bits)?) == Ordering::Less);
    let (a, other) = if swap { (other, a) } else { (a, other) };
    let dx = b(other.position[0])?.sub(&b(a.position[0])?, p)?;
    let dy = store(b(other.position[1])?.sub(&b(a.position[1])?, p)?)?;
    let delta = if a.shape.kind < 2 {
        [dx.store_f32()?, dy.store_f32()?]
    } else {
        let dx = store(dx)?;
        let (fx, fy) = (b(a.facing[0])?, b(a.facing[1])?);
        [
            dx.mul(&fy, p)?.sub(&dy.mul(&fx, p)?, p)?.store_f32()?,
            dy.mul(&fy, p)?.add(&dx.mul(&fx, p)?, p)?.store_f32()?,
        ]
    };
    //46EFD9 compares the raw facings even for points/circles; unordered is
    //a mismatch. Only the following kind gate can request rotated footprints.
    let same_facing = raw_equal(a.facing[0],other.facing[0])
        && raw_equal(a.facing[1],other.facing[1]);
    if other.shape.kind >= 2 && !same_facing {
        return footprints_overlap(footprint(a, p)?, footprint(other, p)?, p);
    }
    match a.shape.kind {
        0 => Ok(b(delta[0])?.is_zero() && b(delta[1])?.is_zero()),
        1 => {
            let r = if other.shape.kind == 0 {
                b(a.shape.radius_bits)?
            } else {
                b(other.shape.radius_bits)?.add(&b(a.shape.radius_bits)?, p)?
            };
            let x = b(delta[0])?;
            let y = b(delta[1])?;
            let distance = y
                .mul(&y, p)?
                .add(&x.mul(&x, p)?, p)?
                .add(&b(0x38d1b717)?, p)?;
            Ok(r.mul(&r, p)?.compare(&distance) != Ordering::Less)
        }
        2 | 3 => {
            if other.shape.kind < 2 {
                rectangle_circle(&a.shape, other.shape.radius_bits, delta, p)
            } else {
                aligned_boxes(&a.shape, &other.shape, delta, p)
            }
        }
        _ => unreachable!(),
    }
}

#[cfg(test)]
#[path = "shape_overlap_nonfinite_tests.rs"]
mod nonfinite_tests;
