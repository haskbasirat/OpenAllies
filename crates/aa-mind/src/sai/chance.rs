//! Native SAI chance roll `4A6540(rng, key, owner)` (called from `7B3F70`):
//!
//! `object = 7B9050(key)` on the owner; `a = binary32(7B9570(key+8))` and
//! `b = binary32(7B96E0(0) * [+808])` on that object. With `a > 0` the rate
//! is `r = binary32(pow(b / a, [+80C]))` (`_CIpow` `96D890`), lowered to
//! `[+814]` unless the unrounded power is below it, then raised to `[+810]`
//! unless above it; otherwise `r` is `[+814]` for `b > 0`, else `[+810]`. One world
//! RNG draw (`85D840`, unsigned, scaled by binary64 `1/(2^32-1)`) succeeds
//! when `r * [+804]` is not below it.
//!
//! Oracle: research/phase8-combat/native_chance.py.
use crate::{Error, Result, x87::n};
use aa_numeric::{
    Binary, Precision,
    crt_pow::{CrtEnvironment, ci_pow_x87},
};
use std::cmp::Ordering;

/// The binary32 fields of the `4A6540` object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChanceFields {
    pub field804: u32,
    pub field808: u32,
    pub field80c: u32,
    pub field810: u32,
    pub field814: u32,
}

pub trait ChanceWorld {
    /// `7B9050(key)` on the owner.
    fn lookup(&mut self, owner: u32, key: u32) -> Result<u32>;
    /// The key's `+8` word.
    fn key_word8(&mut self, key: u32) -> Result<u32>;
    /// `7B9570(word)` on the looked-up object (ST0).
    fn value_7b9570(&mut self, object: u32, word: u32) -> Result<Binary>;
    /// `7B96E0(0)` on the looked-up object (ST0).
    fn value_7b96e0(&mut self, object: u32) -> Result<Binary>;
    /// `85D840(rng)`: one draw of the given RNG state.
    fn draw(&mut self, rng: u32) -> Result<u32>;
}

/// Native `4A6540(rng, key, owner)`.
pub fn chance(
    world: &mut impl ChanceWorld,
    fields: ChanceFields,
    rng: u32,
    key: u32,
    owner: u32,
    crt: CrtEnvironment,
    p: Precision,
) -> Result<bool> {
    let object = world.lookup(owner, key)?;
    let word = world.key_word8(key)?;
    let a = world.value_7b9570(object, word)?.store_f32().map_err(num)?;
    let b = world
        .value_7b96e0(object)?
        .mul(&n(fields.field808)?, p)
        .and_then(|v| v.store_f32())
        .map_err(num)?;
    let zero = Binary::from_i32(0);
    let rate = if n(a)?.compare(&zero) == Ordering::Greater {
        let ratio = n(b)?.div(&n(a)?, p).map_err(num)?;
        // A libm error path (pow(0, 0) reports code 26) still leaves its
        // result in ST0; 4A6540 does not look at errno.
        let (power, _outcome) = ci_pow_x87(&ratio, &n(fields.field80c)?, crt)
            .map_err(|e| Error::World(format!("CRT pow 96D890: {e}")))?;
        let mut rate = power.store_f32().map_err(num)?;
        if power.compare(&n(fields.field814)?) != Ordering::Less {
            rate = fields.field814;
        }
        if n(rate)?.compare(&n(fields.field810)?) != Ordering::Greater {
            rate = fields.field810;
        }
        rate
    } else if n(b)?.compare(&zero) == Ordering::Greater {
        fields.field814
    } else {
        fields.field810
    };
    let draw = world.draw(rng)? as i32;
    let mut unit = Binary::from_i32(draw);
    if draw < 0 {
        unit = unit.add(&Binary::from_f64_bits(0x41f0_0000_0000_0000).map_err(num)?, p).map_err(num)?;
    }
    let unit = unit.mul(&Binary::from_f64_bits(0x3df0_0000_0010_0000).map_err(num)?, p).map_err(num)?;
    let product = n(rate)?.mul(&n(fields.field804)?, p).map_err(num)?;
    Ok(product.compare(&unit) != Ordering::Less)
}

fn num(e: aa_numeric::Error) -> Error {
    Error::World(e.to_string())
}
