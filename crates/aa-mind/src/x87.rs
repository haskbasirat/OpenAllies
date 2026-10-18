//! Small exact x87 helpers shared by the AI ports. All arithmetic uses
//! aa_numeric::Binary with explicit precision control.
use crate::{Error, Result};
use aa_numeric::{Binary, Precision};

/// `0x9FDAA0`: binary64 2^32.
const TWO_POW_32_F64: u64 = 0x41f0_0000_0000_0000;
/// `0x9FDA98`: binary64 nearest to 1/(2^32-1) (= 2^-32 + 2^-64), NOT 2^-32.
const UNIT_SCALE_F64: u64 = 0x3df0_0000_0010_0000;

pub(crate) fn num<T>(value: aa_numeric::Result<T>) -> Result<T> {
    value.map_err(|e| Error::World(format!("x87 arithmetic: {e}")))
}
pub(crate) fn n(bits: u32) -> Result<Binary> {
    num(Binary::from_f32_bits(bits))
}
pub(crate) fn store(value: Binary) -> Result<u32> {
    num(value.store_f32())
}

/// `45D400` and its inlined copies (e.g. 52B4DF): `fild` of the signed
/// `85D840` word, `+2^32` when negative, times the binary64 constant at
/// `0x9FDA98` (2^-32 + 2^-64). Returned unstored and rounded per PC.
pub fn unit_random(value: u32, p: Precision) -> Result<Binary> {
    let mut x = Binary::from_i32(value as i32);
    if (value as i32) < 0 {
        x = num(x.add(&num(Binary::from_f64_bits(TWO_POW_32_F64))?, p))?;
    }
    num(x.mul(&num(Binary::from_f64_bits(UNIT_SCALE_F64))?, p))
}

/// Exact value of a normal (or zero) 80-bit x87 register image. Denormal,
/// unnormal, infinite and NaN images are rejected rather than approximated.
pub fn binary_from_x87(significand: u64, sign_exponent: u16) -> Result<Binary> {
    let exponent = i32::from(sign_exponent & 0x7fff);
    let mut value = if significand == 0 && exponent == 0 {
        Binary::from_i32(0)
    } else {
        if exponent == 0 || exponent == 0x7fff || significand >> 63 == 0 {
            return Err(Error::World(format!(
                "unsupported x87 register image {significand:#x}/{sign_exponent:#x}"
            )));
        }
        let p = Precision::Extended64;
        let high = Binary::from_u32((significand >> 32) as u32);
        let wide = num(high.mul(&num(Binary::from_f64_bits(TWO_POW_32_F64))?, p))?;
        let mut v = num(wide.add(&Binary::from_u32(significand as u32), p))?;
        // Scale by 2^(exponent-16383-63) in binary64-representable steps.
        let mut k = exponent - 16383 - 63;
        while k != 0 {
            let step = k.clamp(-1000, 1000);
            let factor = num(Binary::from_f64_bits(((step + 1023) as u64) << 52))?;
            v = num(v.mul(&factor, p))?;
            k -= step;
        }
        v
    };
    if sign_exponent & 0x8000 != 0 {
        value = value.negated();
    }
    Ok(value)
}
