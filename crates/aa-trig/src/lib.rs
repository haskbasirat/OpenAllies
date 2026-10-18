//! Original 4096-entry quantized trigonometry and rotation. Precision is explicit.
//! These operations do not supply navigation, collision or movement policy.

mod angle;
pub use angle::{signed_angle, signed_angle_extended};
pub mod turning;

use aa_numeric::{Binary, Precision};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
    #[error("nonfinite input is outside the supported domain")]
    NonFiniteInput,
    #[error("original x87 angle instructions require x86_64")]
    UnsupportedPlatform,
    #[error("angle computation produced a nonfinite result")]
    NonFiniteResult,
    #[error("original lookup would read outside its table at index {0}")]
    InvalidLookupIndex(i32),
}

const TABLE_LEN: i32 = 4096;
const TABLES: &[u8; 32768] = include_bytes!("../../../research/phase4-trig/lookup-tables.bin");

fn index(angle: u32, precision: Precision) -> Result<usize, Error> {
    let value = Binary::from_f32_bits(angle)?;
    let period = Binary::from_f32_bits(0x40c9_0fdb)?;
    let quotient = value.div(&period, precision)?.truncate_i32()?;
    let mut remainder = value.sub(
        &Binary::from_i32(quotient).mul(&period, precision)?,
        precision,
    )?;
    if remainder.is_negative_nonzero() {
        remainder = remainder.add(&period, precision)?;
    }
    let result = remainder
        .div(&period, precision)?
        .mul(&Binary::from_i32(TABLE_LEN), precision)?
        .truncate_i32()?;
    if !(0..TABLE_LEN).contains(&result) {
        return Err(Error::InvalidLookupIndex(result));
    }
    Ok(result as usize)
}

fn lookup(angle: u32, precision: Precision, sine: bool) -> Result<u32, Error> {
    let offset = (index(angle, precision)? + if sine { TABLE_LEN as usize } else { 0 }) * 4;
    Ok(u32::from_le_bytes(
        TABLES[offset..offset + 4].try_into().unwrap(),
    ))
}

pub fn lookup_sin(angle_bits: u32, precision: Precision) -> Result<u32, Error> {
    lookup(angle_bits, precision, true)
}

pub fn lookup_cos(angle_bits: u32, precision: Precision) -> Result<u32, Error> {
    lookup(angle_bits, precision, false)
}

/// Original 4454A0/4454E0 rotation, including table quantization and separate stores.
pub fn rotate(
    from: [u32; 2],
    angle_bits: u32,
    clockwise: bool,
    precision: Precision,
) -> Result<[u32; 2], Error> {
    let angle = angle_bits ^ if clockwise { 0x8000_0000 } else { 0 };
    let sin = Binary::from_f32_bits(lookup_sin(angle, precision)?)?;
    let cos = Binary::from_f32_bits(lookup_cos(angle, precision)?)?;
    let x = Binary::from_f32_bits(from[0])?;
    let y = Binary::from_f32_bits(from[1])?;
    let new_y = sin
        .mul(&x, precision)?
        .add(&cos.mul(&y, precision)?, precision)?
        .store_f32()?;
    let new_x = cos
        .mul(&x, precision)?
        .sub(&sin.mul(&y, precision)?, precision)?
        .store_f32()?;
    Ok([new_x, new_y])
}
