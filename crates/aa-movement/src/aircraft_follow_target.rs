//! Original aircraft move-state target follow `518A30` (thiscall state,
//! `(force)`, `ret 4`), run by the move Think `51A720`
//! ([`crate::aircraft_move_state::think`]).
//!
//! The followed object is the state `+14` GTID's `[B364C0]` slot (the
//! generation is checked, the slot is not: the original dereferences it).
//! Its `+20`/`+24` are copied to state `+38`/`+3C`. Unless forced, nothing
//! more happens while the destination `+18`/`+1C`/`+20` compares equal to
//! (`+38`, `+3C`, 0.0).
//!
//! Then, on the x87 stack at the caller's precision with the native store
//! points (`B`, `C` = the object's `+20`/`+24`):
//! 1. `gap = binary32(binary32(sqrt(dx*dx + dy*dy)) - owner radius) - object
//!    radius`, with `dx`/`dy` the owner's `+20`/`+24` minus `B`/`C`, the radii
//!    the actors' virtual 0 (ST0); `gap` is stored to binary32 and the
//!    unstored value compared: a negative gap ends unless forced.
//! 2. With the object's speed (virtual +18, stored) not equal to 0.0 the
//!    point leads the object: with `v` the owner's speed (virtual +18,
//!    unstored), `X = B + (1/v) * (speed * +2C * gap)` (unstored) and
//!    `C = binary32(C + binary32(binary32(speed * +30 * gap) * (1/v)))`;
//!    otherwise `X = B`.
//! 3. When `(+40 - X)^2 + (+44 - C)^2` (unstored) is not below 1.0, or when
//!    forced, `516BA0(binary32(X), C, object)` re-targets the state (the
//!    object passed is the GTID's slot again, null on a generation mismatch).
use crate::Result;
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

/// The reads and calls of `518A30`, in native order.
pub trait FollowTargetWorld {
    type Object: Copy;
    /// The `+14` GTID's slot. The original dereferences it unchecked, so a
    /// world without a live object returns an error.
    fn followed(&mut self) -> Result<Self::Object>;
    /// Fact: object `+20`/`+24`.
    fn object_position(&mut self, object: Self::Object) -> Result<[u32; 2]>;
    /// State `+38`/`+3C` stores.
    fn set_followed_position(&mut self, xy: [u32; 2]) -> Result<()>;
    /// Fact: state `+18`/`+1C`/`+20`.
    fn destination(&mut self) -> Result<[u32; 3]>;
    /// Fact: the owner's (`529C30`) `+20`/`+24`.
    fn owner_position(&mut self) -> Result<[u32; 2]>;
    /// Owner virtual 0 (ST0).
    fn owner_radius(&mut self) -> Result<Binary>;
    /// Object virtual 0 (ST0).
    fn object_radius(&mut self, object: Self::Object) -> Result<Binary>;
    /// Object virtual +18 (ST0).
    fn object_speed(&mut self, object: Self::Object) -> Result<Binary>;
    /// Owner virtual +18 (ST0).
    fn owner_speed(&mut self) -> Result<Binary>;
    /// Fact: object `+2C`/`+30`.
    fn object_heading(&mut self, object: Self::Object) -> Result<[u32; 2]>;
    /// Fact: state `+40`/`+44`.
    fn commanded(&mut self) -> Result<[u32; 2]>;
    /// `516BA0(x, y, slot)` on the state; `slot` is the GTID's table entry
    /// on a generation match (possibly null), else null.
    fn retarget(&mut self, xy: [u32; 2]) -> Result<()>;
}

fn b(bits: u32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(bits)?)
}

fn stored(value: &Binary) -> Result<Binary> {
    b(value.store_f32()?)
}

/// `fucomp`-style equality of two binary32 words (+0 equals -0; a NaN is
/// unequal to everything).
fn equal(a: u32, c: u32) -> bool {
    let nan = |x: u32| x & 0x7f80_0000 == 0x7f80_0000 && x & 0x007f_ffff != 0;
    !nan(a) && !nan(c) && (a == c || (a | c) & 0x7fff_ffff == 0)
}

/// `518A30(force)`: the followed object.
pub fn follow_target<W: FollowTargetWorld + ?Sized>(
    world: &mut W,
    force: bool,
    precision: Precision,
) -> Result<W::Object> {
    let p = precision;
    let object = world.followed()?;
    let [bx, cy] = world.object_position(object)?;
    world.set_followed_position([bx, cy])?;
    let [dx0, dy0, dz0] = world.destination()?;
    if equal(dx0, bx) && equal(dy0, cy) && equal(dz0, 0) && !force {
        return Ok(object);
    }
    let [ox, oy] = world.owner_position()?;
    let dy = b(oy)?.sub(&b(cy)?, p)?;
    let dx = b(ox)?.sub(&b(bx)?, p)?;
    let distance = stored(&dx.mul(&dx, p)?.add(&dy.mul(&dy, p)?, p)?.sqrt(p)?)?;
    let after_owner = stored(&distance.sub(&world.owner_radius()?, p)?)?;
    let gap_unstored = after_owner.sub(&world.object_radius(object)?, p)?;
    let gap = stored(&gap_unstored)?;
    if gap_unstored.compare(&b(0)?) == Ordering::Less && !force {
        return Ok(object);
    }
    let speed_bits = world.object_speed(object)?.store_f32()?;
    let mut c = cy;
    let x = if equal(speed_bits, 0) {
        b(bx)?
    } else {
        let v = world.owner_speed()?;
        let speed = b(speed_bits)?;
        let [h2c, h30] = world.object_heading(object)?;
        let lead_y = stored(&speed.mul(&b(h30)?, p)?.mul(&gap, p)?)?;
        let lead_x = speed.mul(&b(h2c)?, p)?.mul(&gap, p)?;
        let inverse = b(0x3f80_0000)?.div(&v, p)?;
        let lead_y = stored(&lead_y.mul(&inverse, p)?)?;
        let x = inverse.mul(&lead_x, p)?.add(&b(bx)?, p)?;
        c = b(cy)?.add(&lead_y, p)?.store_f32()?;
        x
    };
    let [cx2, cy2] = world.commanded()?;
    let ddy = b(cy2)?.sub(&b(c)?, p)?;
    let ddx = b(cx2)?.sub(&x, p)?;
    let d2 = ddx.mul(&ddx, p)?.add(&ddy.mul(&ddy, p)?, p)?;
    if d2.compare(&b(0x3f80_0000)?) == Ordering::Less && !force {
        return Ok(object);
    }
    world.retarget([x.store_f32()?, c])?;
    Ok(object)
}
