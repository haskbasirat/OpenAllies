//! The table-based fast vector normalize `0x43A970` and its square-root
//! mantissa table, built at startup by `0x8693B0` (stored at `[B43DC4]` by
//! `0x869DC0`).
//!
//! Table entry `i < 128` is the binary32 mantissa field of the square root
//! of the binary32 `(i | 0x3F80) << 16` (1 + i/128); entry `128 + i` is the
//! one of `(i | 0x4000) << 16` (2 + i/64). Each root is the x87 `fsqrt`
//! stored to binary32.
//!
//! `0x43A970` stores the squared length `(x*x + y*y) + z*z` to binary32. A
//! zero word gives the factor 0.0; otherwise the exponent is halved
//! (16-bit arithmetic shift, odd exponents read the second half of the
//! table), the mantissa field comes from the table entry of the top eight
//! mantissa bits, and the factor is 1.0 divided by that binary32. Each
//! component becomes `factor * component` stored to binary32.
//!
//! Proven bit for bit (the table at PC24/53/64 and the normalize at
//! PC24/53/64) by research/phase8-combat/native_fast_normalize.py.
use aa_numeric::{Fast, Precision, Result};

/// `0x8693B0`: the 256 mantissa words.
pub fn sqrt_table() -> Result<[u32; 256]> {
    let mut table = [0u32; 256];
    for i in 0..128u32 {
        for (half, base) in [(0usize, 0x3f80u32), (128, 0x4000)] {
            let root = Fast::from_f32_bits((i | base) << 16)?.sqrt(Precision::Double53)?;
            table[half + i as usize] = root.store_f32()? & 0x007f_ffff;
        }
    }
    Ok(table)
}

/// `0x43A970` on binary32 words `v`, with the table of [`sqrt_table`].
pub fn fast_normalize(v: [u32; 3], table: &[u32; 256], p: Precision) -> Result<[u32; 3]> {
    let x = Fast::from_f32_bits(v[0])?;
    let y = Fast::from_f32_bits(v[1])?;
    let z = Fast::from_f32_bits(v[2])?;
    let length2 = x.mul(&x, p)?.add(&y.mul(&y, p)?, p)?.add(&z.mul(&z, p)?, p)?.store_f32()?;
    let factor = if length2 == 0 {
        Fast::from_f32_bits(0)?
    } else {
        let exponent = (length2 >> 23).wrapping_sub(0x7f);
        let mut mantissa = length2 & 0x007f_ffff;
        if exponent & 1 != 0 {
            mantissa |= 0x0080_0000;
        }
        let half = i32::from((exponent as u16 as i16) >> 1);
        let root = ((half + 0x7f) as u32).wrapping_shl(23) | table[(mantissa >> 16) as usize];
        Fast::from_i32(1).div(&Fast::from_f32_bits(root)?, p)?
    };
    Ok([
        factor.mul(&x, p)?.store_f32()?,
        factor.mul(&y, p)?.store_f32()?,
        factor.mul(&z, p)?.store_f32()?,
    ])
}
