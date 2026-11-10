//! Native trigger comparison `541DE0(left, kind, right)` over every binary32
//! value, and the activation-time comparison `5425C0(kind, right)`.
//!
//! `541DE0` loads both binary32 arguments and compares them with `fcomp` or
//! `fucompp`: kinds 0..5 are `<`, `<=`, `==`, `>=`, `>`, `!=`; an unordered
//! pair (a NaN operand) makes every kind false except `!=`; signed zeros are
//! equal; any other kind returns false. [`crate::program::compare`] is the
//! same rule over finite values only (it refuses NaN and infinities).
//!
//! `5425C0` on a trigger element: the game time (`[B36534]+E0`) minus the
//! trigger's activation time (record `+4`), an x87 `fsub` at the caller's
//! precision stored to binary32 as the `541DE0` left argument. A difference
//! beyond binary32 range stores as an infinity (masked overflow, round to
//! nearest).
use crate::manager::{Error, Result};
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

fn is_nan(bits: u32) -> bool {
    bits & 0x7f80_0000 == 0x7f80_0000 && bits & 0x007f_ffff != 0
}

/// A total order key for non-NaN binary32 values (signed zeros equal).
fn key(bits: u32) -> u32 {
    if bits & 0x7fff_ffff == 0 {
        0x8000_0000
    } else if bits & 0x8000_0000 != 0 {
        !bits
    } else {
        bits | 0x8000_0000
    }
}

/// Native `541DE0(left, kind, right)` (binary32 bits).
pub fn compare_native(left: u32, kind: u32, right: u32) -> bool {
    let order = (!is_nan(left) && !is_nan(right)).then(|| key(left).cmp(&key(right)));
    match kind {
        0 => order == Some(Ordering::Less),
        1 => matches!(order, Some(Ordering::Less | Ordering::Equal)),
        2 => order == Some(Ordering::Equal),
        3 => matches!(order, Some(Ordering::Greater | Ordering::Equal)),
        4 => order == Some(Ordering::Greater),
        5 => order != Some(Ordering::Equal),
        _ => false,
    }
}

/// The binary32 store of `time - activation` (`fsub` at `p`, `fstp dword`).
/// Finite operands only.
pub fn elapsed_bits(time: u32, activation: u32, p: Precision) -> Result<u32> {
    let numeric = |bits| Binary::from_f32_bits(bits).map_err(|e| Error::State(e.to_string()));
    let difference = numeric(time)?
        .sub(&numeric(activation)?, p)
        .map_err(|e| Error::State(e.to_string()))?;
    match difference.store_f32() {
        Ok(bits) => Ok(bits),
        Err(aa_numeric::Error::Binary32Overflow) => Ok(if difference.is_negative_nonzero() {
            0xff80_0000
        } else {
            0x7f80_0000
        }),
        Err(e) => Err(Error::State(e.to_string())),
    }
}

/// Native `5425C0(kind, right)` with the game time and the trigger's
/// activation time.
pub fn activation_compare(time: u32, activation: u32, kind: u32, right: u32, p: Precision) -> Result<bool> {
    Ok(compare_native(elapsed_bits(time, activation, p)?, kind, right))
}
