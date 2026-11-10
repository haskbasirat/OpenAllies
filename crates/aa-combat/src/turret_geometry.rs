//! Turret definition geometry.
//!
//! `0x4C76A0` turns the turret definition's mount offset (definition +24,
//! +28) by a facing vector (cos, sin). The four products and the two sums
//! stay on the x87 stack at the running precision control; each output word
//! is stored to binary32 once:
//!
//! - `out[0] = (x * offset_x) - (y * offset_y)`
//! - `out[1] = (y * offset_x) + (x * offset_y)`
//!
//! `offset_x`/`offset_y` are the binary32 words at definition +24/+28 and
//! `x`/`y` the facing words. Proven bit for bit at PC24, PC53 and PC64 by
//! research/phase8-combat/native_turret_offset.py.
use crate::Result;
use aa_body::F32;
use aa_numeric::{Fast, Precision};

/// `0x4C76A0`: the mount offset (definition +24, +28) turned by `facing`
/// (cos, sin), both stored once to binary32 after the x87 chain.
pub fn turned_offset(offset: [F32; 2], facing: [F32; 2], precision: Precision) -> Result<[F32; 2]> {
    let ox = Fast::from_f32_bits(offset[0].0)?;
    let oy = Fast::from_f32_bits(offset[1].0)?;
    let x = Fast::from_f32_bits(facing[0].0)?;
    let y = Fast::from_f32_bits(facing[1].0)?;
    let first = x.mul(&ox, precision)?.sub(&y.mul(&oy, precision)?, precision)?;
    let second = y.mul(&ox, precision)?.add(&x.mul(&oy, precision)?, precision)?;
    Ok([F32(first.store_f32()?), F32(second.store_f32()?)])
}
