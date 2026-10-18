//! x87 helpers for the zone code (precision control is an explicit input; the
//! simulation thread runs at PC53). Values held in registers are
//! [`aa_numeric::Binary`] rounded to the precision control; `fstp dword`
//! stores go through [`f32_store`].
use crate::{Error, Result};
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

/// Binary32 0.5 at `0x9FBDB8`.
pub const HALF_BITS: u32 = 0x3F00_0000;
/// Binary32 -1.0 at `0x9FDC48` (inactive zone radius).
pub const MINUS_ONE_BITS: u32 = 0xBF80_0000;
/// Binary32 0.25 at `0xA033BC`.
pub const QUARTER_BITS: u32 = 0x3E80_0000;
/// Binary32 1.0 at `0x9F32E0`.
pub const ONE_BITS: u32 = 0x3F80_0000;
/// Binary32 1e-4 at `0xA3AA10` (map clamp margin).
pub const CLAMP_MARGIN_BITS: u32 = 0x38D1_B717;

/// `fld dword`: a finite binary32 (NaN/Inf are outside the ported domain).
pub fn load(bits: u32) -> Result<Binary> {
    Binary::from_f32_bits(bits).map_err(|_| Error::NonFinite(bits))
}

pub fn f32_store(v: &Binary) -> Result<u32> {
    Ok(v.store_f32()?)
}

pub fn add(a: &Binary, b: &Binary, p: Precision) -> Result<Binary> {
    Ok(a.add(b, p)?)
}

pub fn sub(a: &Binary, b: &Binary, p: Precision) -> Result<Binary> {
    Ok(a.sub(b, p)?)
}

pub fn mul(a: &Binary, b: &Binary, p: Precision) -> Result<Binary> {
    Ok(a.mul(b, p)?)
}

pub fn div(a: &Binary, b: &Binary, p: Precision) -> Result<Binary> {
    Ok(a.div(b, p)?)
}

/// CRT `_ftol` (`0x96CC4C`): truncation toward zero of ST0. Values outside
/// the signed 32-bit range (where the original returns the low dword of a
/// 64-bit conversion) are rejected.
pub fn ftol(v: &Binary) -> Result<i32> {
    Ok(v.truncate_i32()?)
}

/// `_CIfmod` (`0x96DB7A`, FPREM loop): exact remainder with the dividend's sign.
pub fn fmod(x: &Binary, y: &Binary) -> Result<Binary> {
    Ok(x.remainder_trunc(y)?)
}

/// `a < b` as tested by `fcom` + `test ah, 0x41 / jne` patterns (ordered).
pub fn less(a: &Binary, b: &Binary) -> bool {
    a.compare(b) == Ordering::Less
}

pub fn equal(a: &Binary, b: &Binary) -> bool {
    a.compare(b) == Ordering::Equal
}

/// `(float)(cell << shift) + half` kept in a register (`fild` exact, `fadd`
/// rounded to the precision control).
pub fn cell_center(cell: i32, shift: u32, half: &Binary, p: Precision) -> Result<Binary> {
    let shifted = cell.wrapping_shl(shift);
    add(&Binary::from_i32(shifted), half, p)
}

/// `map.tile_size * 0.5` kept in a register.
pub fn half_tile(tile_size_bits: u32, p: Precision) -> Result<Binary> {
    mul(&load(tile_size_bits)?, &load(HALF_BITS)?, p)
}
