//! Original `52E6A0(x, y)` on the element move state: the point lies inside
//! one of the unit's two turning circles, so a single-node path cannot be
//! driven straight (called by the try-action `5300A0`).
//!
//! With `r` the locomotion (actor +6C) +30 turning radius, the actor
//! position `(px, py)` (+20/+24) and facing `(fx, fy)` (+2C/+30), at x87
//! precision `p`:
//! * `rr = r*r` and `a = r*fy` are stored to binary32, `b = r*fx` stays
//!   extended;
//! * the first centre `(px + a, py - b)` stays extended, the second
//!   `(px - a, b + py)` is stored to binary32;
//! * `y - (py - b)` is stored, `x - (px + a)` is not; the limit
//!   `rr + [A34770]` (1e-4) is stored;
//! * true when the squared distance to the first centre, or else to the
//!   second, is below the limit (ordered).
use crate::Result;
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

/// `[A34770]` (binary32 1e-4).
pub const TURN_EPSILON_BITS: u32 = 0x38d1_b717;

fn b(bits: u32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(bits)?)
}
fn store(v: &Binary) -> Result<u32> {
    Ok(v.store_f32()?)
}

/// Complete `52E6A0(point)`: `radius` is locomotion +30, `position` actor
/// +20/+24 and `facing` actor +2C/+30 (binary32 words).
pub fn within_turning_circles(
    radius: u32,
    position: [u32; 2],
    facing: [u32; 2],
    point: [u32; 2],
    p: Precision,
) -> Result<bool> {
    let r = b(radius)?;
    let rr = store(&r.mul(&r, p)?)?;
    let [px, py] = [b(position[0])?, b(position[1])?];
    let a = store(&r.mul(&b(facing[1])?, p)?)?;
    let bb = r.mul(&b(facing[0])?, p)?;
    let first_x = px.add(&b(a)?, p)?;
    let first_y = py.sub(&bb, p)?;
    let second_x = store(&px.sub(&b(a)?, p)?)?;
    let second_y = store(&bb.add(&py, p)?)?;
    let [x, y] = [b(point[0])?, b(point[1])?];
    let e = store(&y.sub(&first_y, p)?)?;
    let dx = x.sub(&first_x, p)?;
    let limit = b(store(&b(rr)?.add(&b(TURN_EPSILON_BITS)?, p)?)?)?;
    let e = b(e)?;
    let first = dx.mul(&dx, p)?.add(&e.mul(&e, p)?, p)?;
    if first.compare(&limit) == Ordering::Less {
        return Ok(true);
    }
    let dy2 = y.sub(&b(second_y)?, p)?;
    let dx2 = x.sub(&b(second_x)?, p)?;
    let second = dx2.mul(&dx2, p)?.add(&dy2.mul(&dy2, p)?, p)?;
    Ok(second.compare(&limit) == Ordering::Less)
}
