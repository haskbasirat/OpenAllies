//! Original aircraft attack-state aim point `51B0C0` (fastcall on the
//! `aai_attack` state, Mind+E8), run by the attack Think `51B8C0`
//! ([`crate::uai_thinks::aai_attack`]) in modes 0/1 for a live target.
//!
//! The target is the state `+18` GTID's `[B364C0]` slot (generation checked,
//! slot dereferenced unchecked). On the x87 stack at the caller's precision,
//! with `TX`/`TY` the target's `+20`/`+24` and the owner from `529C30`:
//! 1. `t = binary32(binary32(sqrt(binary32(dx*dx + dy*dy))) / v)` with
//!    `dx`/`dy` = `TX`/`TY` minus the owner's `+20`/`+24` and `v` the owner's
//!    speed (virtual +18, unstored);
//! 2. with `s` the target's speed (virtual +18, unstored) and its `+2C`/`+30`
//!    heading: `A = binary32(s * +2C)`, `B = binary32(s * +30)`; the aim point
//!    `+24 = binary32(binary32(A * t) + TX)`, `+28 = binary32(binary32(B * t)
//!    + TY)`, `+2C = 0`;
//! 3. `D = binary32((+3C - aim x)^2 + (+40 - aim y)^2)`. The plan `51A920`
//!    follows (a tail call) when the owner's reach (virtual +104, unstored)
//!    times 0.5 is below `D`, or else when the owner's flight (`+BC`) is free
//!    (`+90` set or `+34` zero) and the state's `+14` byte is clear.
use crate::Result;
use crate::x87::{n, num, store};
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

/// The reads and calls of `51B0C0`, in native order.
pub trait AttackAimWorld {
    type Object: Copy;
    /// The `+18` GTID's slot; the original dereferences it unchecked, so a
    /// world without a live target returns an error.
    fn target(&mut self) -> Result<Self::Object>;
    /// Fact: object `+20`/`+24`.
    fn object_position(&mut self, object: Self::Object) -> Result<[u32; 2]>;
    /// Fact: the owner's `+20`/`+24`.
    fn owner_position(&mut self) -> Result<[u32; 2]>;
    /// Owner virtual +18 (ST0).
    fn owner_speed(&mut self) -> Result<Binary>;
    /// Object virtual +18 (ST0).
    fn object_speed(&mut self, object: Self::Object) -> Result<Binary>;
    /// Fact: object `+2C`/`+30`.
    fn object_heading(&mut self, object: Self::Object) -> Result<[u32; 2]>;
    /// State `+24`/`+28`/`+2C` stores.
    fn set_aim_point(&mut self, point: [u32; 3]) -> Result<()>;
    /// Fact: state `+3C`/`+40`.
    fn previous_point(&mut self) -> Result<[u32; 2]>;
    /// Owner virtual +104 (ST0).
    fn owner_reach(&mut self) -> Result<Binary>;
    /// Fact: the owner's flight `+90` byte and `+34` word.
    fn flight_byte_90(&mut self) -> Result<u8>;
    fn flight_word_34(&mut self) -> Result<u32>;
    /// Fact: state `+14` byte.
    fn retarget_pending(&mut self) -> Result<u8>;
    /// `51A920` on the state.
    fn plan(&mut self) -> Result<()>;
}

/// `fstp dword` then `fld dword`.
fn stored(value: Binary) -> Result<Binary> {
    n(store(value)?)
}

/// `51B0C0`.
pub fn aim<W: AttackAimWorld + ?Sized>(world: &mut W, precision: Precision) -> Result<()> {
    let p = precision;
    let target = world.target()?;
    let [tx, ty] = world.object_position(target)?;
    let [ox, oy] = world.owner_position()?;
    let dy = num(n(ty)?.sub(&n(oy)?, p))?;
    let dx = num(n(tx)?.sub(&n(ox)?, p))?;
    let square = stored(num(num(dx.mul(&dx, p))?.add(&num(dy.mul(&dy, p))?, p))?)?;
    let distance = stored(num(square.sqrt(p))?)?;
    let v = world.owner_speed()?;
    let t = stored(num(distance.div(&v, p))?)?;
    let s = world.object_speed(target)?;
    let [h2c, h30] = world.object_heading(target)?;
    let a = stored(num(s.mul(&n(h2c)?, p))?)?;
    let c = stored(num(s.mul(&n(h30)?, p))?)?;
    let a = stored(num(a.mul(&t, p))?)?;
    let c = stored(num(c.mul(&t, p))?)?;
    let px = store(num(a.add(&n(tx)?, p))?)?;
    let py = store(num(c.add(&n(ty)?, p))?)?;
    world.set_aim_point([px, py, 0])?;
    let [qx, qy] = world.previous_point()?;
    let dy = num(n(qy)?.sub(&n(py)?, p))?;
    let dx = num(n(qx)?.sub(&n(px)?, p))?;
    let d = stored(num(num(dx.mul(&dx, p))?.add(&num(dy.mul(&dy, p))?, p))?)?;
    let half = num(world.owner_reach()?.mul(&n(0x3f00_0000)?, p))?;
    let replan = half.compare(&d) == Ordering::Less
        || ((world.flight_byte_90()? != 0 || world.flight_word_34()? == 0)
            && world.retarget_pending()? == 0);
    if replan {
        world.plan()?;
    }
    Ok(())
}
