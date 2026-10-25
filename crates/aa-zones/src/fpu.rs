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

/// `fld a; fcomp b` order of two binary32 values (exact at any precision:
/// comparisons do not round). Non-finite operands fail like [`load`].
pub fn compare_bits(a: u32, b: u32) -> Result<Ordering> {
    let (x, y) = (f32::from_bits(a), f32::from_bits(b));
    if !x.is_finite() {
        return Err(Error::NonFinite(a));
    }
    if !y.is_finite() {
        return Err(Error::NonFinite(b));
    }
    // finite binary32 values: the host order is the numeric order (signed
    // zeros equal), as Binary::compare
    Ok(x.partial_cmp(&y).expect("finite"))
}

/// [`less`] of two binary32 loads.
pub fn less_bits(a: u32, b: u32) -> Result<bool> {
    Ok(compare_bits(a, b)? == Ordering::Less)
}

/// [`equal`] of two binary32 loads.
pub fn equal_bits(a: u32, b: u32) -> Result<bool> {
    Ok(compare_bits(a, b)? == Ordering::Equal)
}

/// Exact binary64 evaluation of the simulation thread's PC53 register
/// arithmetic (the fast path of the hot zone routines).
///
/// At precision control PC53 an x87 `fadd`/`fsub`/`fmul`/`fdiv` rounds the
/// exact result once to 53 significant bits, ties to even, keeping the
/// 15-bit extended exponent ([`Binary`] with [`Precision::Double53`]). IEEE
/// binary64 rounds the exact result the same way with an 11-bit exponent,
/// so the two agree whenever the exact result is zero or its magnitude lies
/// in the binary64 normal range `[2^-1022, 2^1024)`. Inside that range
/// `fstp dword` is the binary64 -> binary32 conversion (ties to even,
/// gradual underflow), FPREM's exact remainder is the binary64 `%` (exact,
/// dividend's sign) and `_ftol` of an in-range value is a truncation.
///
/// Every function returns `None` when its operands or result leave that
/// domain (non-finite load, a result outside the normal range, division or
/// remainder by zero, a binary32 overflow, a truncation outside i32); the
/// caller then runs the [`Binary`] model, which also raises the native
/// fault. Binary32 loads and the values the zone routines build from them
/// never leave the domain, so the fallback only guards the proof. Proven
/// against the Binary model in `tests/pc53_fast_path.rs` and by the PC53
/// cases of the native fixtures.
pub mod pc53 {
    /// `fld dword`.
    pub fn load(bits: u32) -> Option<f64> {
        let v = f32::from_bits(bits);
        v.is_finite().then_some(f64::from(v))
    }

    fn normal(r: f64) -> Option<f64> {
        (r.is_finite() && (r == 0.0 || r.abs() >= f64::MIN_POSITIVE)).then_some(r)
    }

    /// `fadd` (a zero sum of two finite values is an exact cancellation or
    /// an exact zero, never an underflow).
    pub fn add(a: f64, b: f64) -> Option<f64> {
        normal(a + b)
    }

    pub fn sub(a: f64, b: f64) -> Option<f64> {
        normal(a - b)
    }

    pub fn mul(a: f64, b: f64) -> Option<f64> {
        let r = a * b;
        if r == 0.0 && a != 0.0 && b != 0.0 {
            return None;
        }
        normal(r)
    }

    pub fn div(a: f64, b: f64) -> Option<f64> {
        if b == 0.0 {
            return None;
        }
        let r = a / b;
        if r == 0.0 && a != 0.0 {
            return None;
        }
        normal(r)
    }

    /// `_CIfmod`.
    pub fn fmod(x: f64, y: f64) -> Option<f64> {
        if y == 0.0 {
            return None;
        }
        normal(x % y)
    }

    /// `fstp dword`.
    pub fn store(v: f64) -> Option<u32> {
        let s = v as f32;
        s.is_finite().then_some(s.to_bits())
    }

    /// CRT `_ftol` inside the signed 32-bit range.
    pub fn ftol(v: f64) -> Option<i32> {
        let t = v.trunc();
        (t >= -2_147_483_648.0 && t < 2_147_483_648.0).then_some(t as i32)
    }

    /// `fild` of an i32 (exact).
    pub fn int(v: i32) -> f64 {
        f64::from(v)
    }

    /// [`super::cell_center`]: `(float)(cell << shift) + half`.
    pub fn cell_center(cell: i32, shift: u32, half: f64) -> Option<f64> {
        add(int(cell.wrapping_shl(shift)), half)
    }

    /// [`super::half_tile`].
    pub fn half_tile(tile_size_bits: u32) -> Option<f64> {
        mul(load(tile_size_bits)?, load(super::HALF_BITS)?)
    }
}
