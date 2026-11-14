//! Original aircraft attack vector turn `0x516B30` (cdecl `(out*, in*,
//! angle)`), used by the dog-fight phases of `0x51A920` with the angles
//! +pi/4 (`0x3F490FDB`) and -pi/4 (`0xBF490FDB`).
//!
//! The sine (`0x85DAA0`) and cosine (`0x85DA80`) are the original 4096-entry
//! table words ([`aa_trig::lookup_sin`] / [`aa_trig::lookup_cos`]). Store
//! points, in native order:
//! - `-y` (`fchs`, exact) is stored to binary32;
//! - `(-y) * sin` and `x * sin` are each stored to binary32;
//! - `cos * y` is stored to binary32;
//! - `out.x = binary32(cos * x + binary32((-y) * sin))`, with `cos * x` kept
//!   on the x87 stack at the caller's precision;
//! - `out.y = binary32(binary32(cos * y) + binary32(x * sin))`.
//!
//! The input is read completely before the output is written, so `out` may
//! alias `in`.
use crate::{Error, Result};
use aa_numeric::{Binary, Precision};

fn b(bits: u32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(bits)?)
}

fn table(error: aa_trig::Error) -> Error {
    Error::Turning(error.to_string())
}

/// `0x516B30`: the planar vector `v` (binary32 bits) turned by
/// `angle_bits`, as the two binary32 words it writes.
pub fn turn_vector(v: [u32; 2], angle_bits: u32, precision: Precision) -> Result<[u32; 2]> {
    let p = precision;
    let sin = b(aa_trig::lookup_sin(angle_bits, p).map_err(table)?)?;
    let x = b(v[0])?;
    let neg_y = b(v[1] ^ 0x8000_0000)?;
    let neg_y_sin = neg_y.mul(&sin, p)?.store_f32()?;
    let x_sin = x.mul(&sin, p)?.store_f32()?;
    let cos = b(aa_trig::lookup_cos(angle_bits, p).map_err(table)?)?;
    let cos_x = cos.mul(&x, p)?;
    let cos_y = cos.mul(&b(v[1])?, p)?.store_f32()?;
    let out_x = cos_x.add(&b(neg_y_sin)?, p)?.store_f32()?;
    let out_y = b(cos_y)?.add(&b(x_sin)?, p)?.store_f32()?;
    Ok([out_x, out_y])
}
