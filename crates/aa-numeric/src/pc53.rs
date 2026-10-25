//! Host binary64 operations that are the x87 operations at precision control
//! PC53, for hot loops that would otherwise go through [`crate::Fast`].
//!
//! At PC53 the x87 rounds every result once to 53 significand bits (ties to
//! even) but keeps the extended exponent range. For binary64 operands the
//! host binary64 result is the same value whenever it is an exact zero, or
//! finite with magnitude at least 2^-1021: then neither binary64 gradual
//! underflow nor overflow touched the rounding. Each helper returns None
//! outside that range (and for division by zero), and the caller takes the
//! general [`crate::Fast`] path instead, from the same operands. A zero
//! result is exact for addition (cancellation never rounds), but for a
//! product or quotient only with a zero operand; any other zero underflowed.
//! [`store`] is FST m32 of such a value: binary64 to binary32 is the native
//! single rounding of the 53-bit register value, with gradual underflow.

/// 2^-1021, the smallest magnitude accepted for a nonzero result.
pub const SAFE: f64 = f64::from_bits(0x0020_0000_0000_0000);

#[inline]
fn normal(r: f64) -> bool {
    r.is_finite() && r.abs() >= SAFE
}
/// FADD.
#[inline]
pub fn add(a: f64, b: f64) -> Option<f64> {
    let r = a + b;
    (r == 0.0 || normal(r)).then_some(r)
}
/// FSUB (`a + -b`, the same value and zero sign).
#[inline]
pub fn sub(a: f64, b: f64) -> Option<f64> {
    add(a, -b)
}
/// FMUL.
#[inline]
pub fn mul(a: f64, b: f64) -> Option<f64> {
    let r = a * b;
    let ok = if r == 0.0 {
        a == 0.0 || b == 0.0
    } else {
        normal(r)
    };
    ok.then_some(r)
}
/// FDIV; None for a zero divisor (the general path reports it).
#[inline]
pub fn div(a: f64, b: f64) -> Option<f64> {
    if b == 0.0 {
        return None;
    }
    let r = a / b;
    let ok = if r == 0.0 { a == 0.0 } else { normal(r) };
    ok.then_some(r)
}
/// FLD m32 of a finite binary32 (every one is exact in binary64).
#[inline]
pub fn load(bits: u32) -> Option<f64> {
    let v = f32::from_bits(bits);
    v.is_finite().then_some(f64::from(v))
}
/// FST m32; None when the store overflows binary32.
#[inline]
pub fn store(v: f64) -> Option<f32> {
    let s = v as f32;
    s.is_finite().then_some(s)
}
