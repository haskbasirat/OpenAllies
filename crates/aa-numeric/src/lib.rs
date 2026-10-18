//! Finite dyadic x87 arithmetic, with explicit significand precision and
//! round-to-nearest/ties-to-even. No host floating-point operations are used.
//!
//! Extracted from the independently reviewed aa-economy binary.rs; that frozen
//! crate is unchanged. This version adds exact rational division, binary64
//! input and checked normal-extended exponent bounds. Extended subnormal or
//! nonfinite arithmetic, other rounding modes, and exception flags are outside
//! this API. Binary32 stores include gradual underflow. No default PC is implied.
#![forbid(unsafe_code)]

use num_bigint::BigUint;
use num_traits::{One, ToPrimitive, Zero};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Precision {
    Single24,
    Double53,
    Extended64,
}
impl Precision {
    pub fn significand_bits(self) -> u64 {
        match self {
            Self::Single24 => 24,
            Self::Double53 => 53,
            Self::Extended64 => 64,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("nonfinite input is outside the finite arithmetic domain")]
    NonFiniteInput,
    #[error("division by zero")]
    DivisionByZero,
    #[error("square root of a negative nonzero value")]
    NegativeSquareRoot,
    #[error("result is outside the supported normal x87 exponent domain")]
    ExtendedExponentRange,
    #[error("result overflows binary32")]
    Binary32Overflow,
    #[error("truncated result is outside signed32-bit integer range")]
    IntegerRange,
}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub struct Binary {
    negative: bool,
    magnitude: BigUint,
    exponent: i32,
}

fn rounded_shift(value: &BigUint, shift: u64) -> BigUint {
    if shift == 0 {
        return value.clone();
    }
    let quotient = value >> shift as usize;
    let remainder = value - (&quotient << shift as usize);
    let half = BigUint::one() << (shift - 1) as usize;
    if remainder > half || (remainder == half && quotient.bit(0)) {
        quotient + BigUint::one()
    } else {
        quotient
    }
}

impl Binary {
    pub fn from_f32_bits(bits: u32) -> Result<Self> {
        let field = (bits >> 23) & 255;
        if field == 255 {
            return Err(Error::NonFiniteInput);
        }
        Ok(Self {
            negative: bits >> 31 != 0,
            magnitude: ((bits & 0x7f_ffff) | if field == 0 { 0 } else { 1 << 23 }).into(),
            exponent: if field == 0 { -149 } else { field as i32 - 150 },
        })
    }
    pub fn from_f64_bits(bits: u64) -> Result<Self> {
        let field = (bits >> 52) & 2047;
        if field == 2047 {
            return Err(Error::NonFiniteInput);
        }
        Ok(Self {
            negative: bits >> 63 != 0,
            magnitude: ((bits & 0x000f_ffff_ffff_ffff) | if field == 0 { 0 } else { 1 << 52 })
                .into(),
            exponent: if field == 0 {
                -1074
            } else {
                field as i32 - 1075
            },
        })
    }
    pub fn from_u32(value: u32) -> Self {
        Self {
            negative: false,
            magnitude: value.into(),
            exponent: 0,
        }
    }
    pub fn from_i32(value: i32) -> Self {
        Self {
            negative: value < 0,
            magnitude: value.unsigned_abs().into(),
            exponent: 0,
        }
    }
    pub fn negated(mut self) -> Self {
        self.negative = !self.negative;
        self
    }
    pub fn is_zero(&self) -> bool {
        self.magnitude.is_zero()
    }
    pub fn is_negative_nonzero(&self) -> bool {
        self.negative && !self.is_zero()
    }

    fn checked(mut self) -> Result<Self> {
        if self.is_zero() {
            self.exponent = 0;
            return Ok(self);
        }
        let top = i64::from(self.exponent) + self.magnitude.bits() as i64 - 1;
        if !(-16382..=16383).contains(&top) {
            return Err(Error::ExtendedExponentRange);
        }
        Ok(self)
    }
    fn rounded(mut self, precision: Precision) -> Result<Self> {
        let bits = self.magnitude.bits();
        let wanted = precision.significand_bits();
        if bits > wanted {
            let shift = bits - wanted;
            self.magnitude = rounded_shift(&self.magnitude, shift);
            self.exponent = self
                .exponent
                .checked_add(shift as i32)
                .ok_or(Error::ExtendedExponentRange)?;
            if self.magnitude.bits() > wanted {
                self.magnitude >>= 1usize;
                self.exponent = self
                    .exponent
                    .checked_add(1)
                    .ok_or(Error::ExtendedExponentRange)?;
            }
        }
        self.checked()
    }
    pub fn add(&self, other: &Self, precision: Precision) -> Result<Self> {
        let exponent = self.exponent.min(other.exponent);
        let left = &self.magnitude << (self.exponent - exponent) as usize;
        let right = &other.magnitude << (other.exponent - exponent) as usize;
        let (negative, magnitude) = if self.negative == other.negative {
            (self.negative, left + right)
        } else {
            match left.cmp(&right) {
                Ordering::Greater => (self.negative, left - right),
                Ordering::Less => (other.negative, right - left),
                Ordering::Equal => (false, BigUint::zero()),
            }
        };
        Self {
            negative,
            magnitude,
            exponent,
        }
        .rounded(precision)
    }
    pub fn sub(&self, other: &Self, precision: Precision) -> Result<Self> {
        self.add(&other.clone().negated(), precision)
    }
    pub fn mul(&self, other: &Self, precision: Precision) -> Result<Self> {
        Self {
            negative: self.negative ^ other.negative,
            magnitude: &self.magnitude * &other.magnitude,
            exponent: self
                .exponent
                .checked_add(other.exponent)
                .ok_or(Error::ExtendedExponentRange)?,
        }
        .rounded(precision)
    }
    pub fn div(&self, other: &Self, precision: Precision) -> Result<Self> {
        if other.is_zero() {
            return Err(Error::DivisionByZero);
        }
        if self.is_zero() {
            return Ok(Self {
                negative: self.negative ^ other.negative,
                magnitude: BigUint::zero(),
                exponent: 0,
            });
        }
        // Exact floor(log2(magnitude numerator / magnitude denominator)).
        let mut top = self.magnitude.bits() as i32 - other.magnitude.bits() as i32;
        let less = if top >= 0 {
            self.magnitude < (&other.magnitude << top as usize)
        } else {
            (&self.magnitude << (-top) as usize) < other.magnitude
        };
        if less {
            top -= 1;
        }
        let shift = precision.significand_bits() as i32 - 1 - top;
        let (numerator, denominator) = if shift >= 0 {
            (&self.magnitude << shift as usize, other.magnitude.clone())
        } else {
            (
                self.magnitude.clone(),
                &other.magnitude << (-shift) as usize,
            )
        };
        let mut quotient = &numerator / &denominator;
        let twice_remainder = (numerator - &quotient * &denominator) << 1usize;
        if twice_remainder > denominator || (twice_remainder == denominator && quotient.bit(0)) {
            quotient += BigUint::one();
        }
        let exponent = self
            .exponent
            .checked_sub(other.exponent)
            .and_then(|e| e.checked_sub(shift))
            .ok_or(Error::ExtendedExponentRange)?;
        Self {
            negative: self.negative ^ other.negative,
            magnitude: quotient,
            exponent,
        }
        .rounded(precision)
    }
    /// Exact finite remainder after a quotient truncated toward zero (FPREM).
    /// The dividend supplies the sign, including signed zero. No PC rounding.
    pub fn remainder_trunc(&self, divisor: &Self) -> Result<Self> {
        if divisor.is_zero() {
            return Err(Error::DivisionByZero);
        }
        let exponent = self.exponent.min(divisor.exponent);
        let numerator = &self.magnitude << (self.exponent - exponent) as usize;
        let denominator = &divisor.magnitude << (divisor.exponent - exponent) as usize;
        Self {
            negative: self.negative,
            magnitude: numerator % denominator,
            exponent,
        }
        .checked()
    }
    /// Correctly rounded finite FSQRT value in the requested significand mode.
    /// No host floating-point operation is used. Exception flags are not modeled.
    pub fn sqrt(&self, precision: Precision) -> Result<Self> {
        if self.is_zero() {
            // FSQRT preserves negative zero.
            return Ok(self.clone());
        }
        if self.negative {
            return Err(Error::NegativeSquareRoot);
        }
        let top = i64::from(self.exponent) + self.magnitude.bits() as i64 - 1;
        // Floor division matters for odd negative exponents (e.g. sqrt(0.5)).
        let exponent = top.div_euclid(2) - (precision.significand_bits() as i64 - 1);
        let scale = i64::from(self.exponent) - 2 * exponent;
        let (numerator, denominator) = if scale >= 0 {
            (&self.magnitude << scale as usize, BigUint::one())
        } else {
            (self.magnitude.clone(), BigUint::one() << (-scale) as usize)
        };
        // floor(sqrt(N/D)) == floor(sqrt(floor(N/D))). Compare against the
        // exact midpoint squared; this also handles a carry to the next binade.
        let mut root = (&numerator / &denominator).sqrt();
        let twice_midpoint = (&root << 1usize) + BigUint::one();
        let midpoint_squared = &denominator * &twice_midpoint * &twice_midpoint;
        let four_numerator = numerator << 2usize;
        if four_numerator > midpoint_squared || (four_numerator == midpoint_squared && root.bit(0))
        {
            root += BigUint::one();
        }
        Self {
            negative: false,
            magnitude: root,
            exponent: i32::try_from(exponent).map_err(|_| Error::ExtendedExponentRange)?,
        }
        .rounded(precision)
    }
    /// Numeric comparison; signed zeros compare equal. Neither operand rounds.
    pub fn compare(&self, other: &Self) -> Ordering {
        if self.is_zero() && other.is_zero() {
            return Ordering::Equal;
        }
        if self.negative != other.negative {
            return if self.negative {
                Ordering::Less
            } else {
                Ordering::Greater
            };
        }
        let exponent = self.exponent.min(other.exponent);
        let order = (&self.magnitude << (self.exponent - exponent) as usize)
            .cmp(&(&other.magnitude << (other.exponent - exponent) as usize));
        if self.negative {
            order.reverse()
        } else {
            order
        }
    }
    pub fn store_f32(&self) -> Result<u32> {
        let sign = u32::from(self.negative) << 31;
        if self.is_zero() {
            return Ok(sign);
        }
        let mut top = self.magnitude.bits() as i32 - 1 + self.exponent;
        let target = (top - 23).max(-149);
        let mut rounded = if target > self.exponent {
            rounded_shift(&self.magnitude, (target - self.exponent) as u64)
        } else {
            &self.magnitude << (self.exponent - target) as usize
        };
        if top < -126 {
            return Ok(sign | rounded.to_u32().expect("subnormal significand fits"));
        }
        if rounded.bits() > 24 {
            rounded >>= 1usize;
            top += 1;
        }
        if top > 127 {
            return Err(Error::Binary32Overflow);
        }
        Ok(sign
            | (((top + 127) as u32) << 23)
            | (rounded.to_u32().expect("binary32 significand fits") & 0x7f_ffff))
    }
    /// Exact value of an 80-bit x87 extended register image (as written by
    /// `FSTP TBYTE`, little-endian in the low 80 bits). Pseudo-denormals and
    /// unnormals are accepted by value; infinities/NaNs are rejected.
    pub fn from_x87_extended_bits(bits: u128) -> Result<Self> {
        let negative = (bits >> 79) & 1 != 0;
        let field = ((bits >> 64) & 0x7fff) as i32;
        let significand = bits as u64;
        if field == 0x7fff {
            return Err(Error::NonFiniteInput);
        }
        let exponent = if field == 0 {
            -16382 - 63
        } else {
            field - 16383 - 63
        };
        Self {
            negative,
            magnitude: significand.into(),
            exponent,
        }
        .checked()
    }
    /// Correctly rounded 2^y for a finite binary32 `y`, at `precision`.
    /// Integral exponents are exact. Otherwise 2^frac(y) is bracketed with
    /// 256 fraction bits by successive integer square roots (2^(2^-i)); None
    /// is returned when the bracket straddles a rounding boundary. This is a
    /// mathematical reference, not a port of any particular libm routine.
    pub fn exp2_binary32(y_bits: u32, precision: Precision) -> Result<Option<Self>> {
        let y = Self::from_f32_bits(y_bits)?;
        if y.is_zero() {
            return Ok(Some(Self::from_i32(1)));
        }
        let m = y.magnitude.to_u64().expect("binary32 significand");
        let signed = if y.negative { -(m as i128) } else { m as i128 };
        let power = |n: i128| -> Result<Self> {
            let exponent = i32::try_from(n).map_err(|_| Error::ExtendedExponentRange)?;
            Self {
                negative: false,
                magnitude: BigUint::one(),
                exponent,
            }
            .checked()
        };
        if y.exponent >= 0 {
            let n = signed
                .checked_shl(y.exponent as u32)
                .filter(|v| v.unsigned_abs() < 1 << 20)
                .ok_or(Error::ExtendedExponentRange)?;
            return power(n).map(Some);
        }
        let k = (-y.exponent) as u32;
        let q = signed.div_euclid(1i128 << k);
        let r = signed.rem_euclid(1i128 << k) as u128;
        if r == 0 {
            return power(q).map(Some);
        }
        const P: usize = 256;
        let one = BigUint::one() << P;
        let (mut lo, mut hi) = (one.clone(), one.clone());
        let (mut c_lo, mut c_hi) = (BigUint::from(2u32) << P, BigUint::from(2u32) << P);
        for i in 1..=k {
            c_lo = (&c_lo << P).sqrt();
            c_hi = (&c_hi << P).sqrt() + BigUint::one();
            if (r >> (k - i)) & 1 != 0 {
                lo = (&lo * &c_lo) >> P;
                let product = &hi * &c_hi;
                let floor = &product >> P;
                hi = if (&floor << P) == product {
                    floor
                } else {
                    floor + BigUint::one()
                };
            }
        }
        let exponent = i32::try_from(q - P as i128).map_err(|_| Error::ExtendedExponentRange)?;
        let low = Self {
            negative: false,
            magnitude: lo,
            exponent,
        }
        .rounded(precision)?;
        let high = Self {
            negative: false,
            magnitude: hi,
            exponent,
        }
        .rounded(precision)?;
        Ok((low.compare(&high).is_eq()).then_some(low))
    }
    /// Exact 80-bit x87 extended encoding (sign, 15-bit biased exponent,
    /// explicit integer bit and 63 fraction bits) of a value whose significand
    /// already fits 64 bits, as left in ST0 or written by `FSTP TBYTE`. Returns
    /// None instead of rounding (callers compare register results exactly).
    pub fn to_x87_extended_bits(&self) -> Option<u128> {
        let sign = u128::from(self.negative) << 79;
        if self.is_zero() {
            return Some(sign);
        }
        let bits = self.magnitude.bits();
        if bits > 64 {
            return None;
        }
        let top = i64::from(self.exponent) + bits as i64 - 1;
        if !(-16382..=16383).contains(&top) {
            return None;
        }
        let significand = (&self.magnitude << (64 - bits) as usize).to_u64()?;
        Some(sign | (((top + 16383) as u128) << 64) | u128::from(significand))
    }
    /// Integer conversion toward zero, without a prior binary32 store. Rejects
    /// overflow rather than returning the native indefinite INT_MIN sentinel.
    pub fn truncate_i32(&self) -> Result<i32> {
        let magnitude = if self.exponent >= 0 {
            &self.magnitude << self.exponent as usize
        } else {
            &self.magnitude >> (-self.exponent) as usize
        };
        let value = magnitude.to_u32().ok_or(Error::IntegerRange)?;
        if self.negative {
            if value > 0x8000_0000 {
                return Err(Error::IntegerRange);
            }
            Ok((0u32.wrapping_sub(value)) as i32)
        } else {
            i32::try_from(value).map_err(|_| Error::IntegerRange)
        }
    }
}

pub mod crt_pow;
