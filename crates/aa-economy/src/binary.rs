//! Finite dyadic arithmetic with explicit x87 significand precision and nearest-even rounding.
//! No host float arithmetic or hardware FPU mode is used for emulated operations.
use crate::{EconomyError, F32Bits, X87Precision};
use num_bigint::BigUint;
use num_traits::{One, ToPrimitive, Zero};
use std::cmp::Ordering;

#[derive(Clone)]
pub(crate) struct Binary {
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
    pub(crate) fn from_bits(value: F32Bits) -> Self {
        let field = (value.0 >> 23) & 255;
        debug_assert!(field != 255);
        let magnitude = (value.0 & 0x7f_ffff) | if field == 0 { 0 } else { 1 << 23 };
        Self {
            negative: value.0 >> 31 != 0,
            magnitude: magnitude.into(),
            exponent: if field == 0 { -149 } else { field as i32 - 150 },
        }
    }

    pub(crate) fn from_i32(value: i32) -> Self {
        Self {
            negative: value < 0,
            magnitude: value.unsigned_abs().into(),
            exponent: 0,
        }
    }

    pub(crate) fn neg(mut self) -> Self {
        self.negative = !self.negative;
        self
    }

    pub(crate) fn rounded(mut self, precision: X87Precision) -> Self {
        let bits = self.magnitude.bits();
        let wanted = precision.significand_bits();
        if bits > wanted {
            let shift = bits - wanted;
            self.magnitude = rounded_shift(&self.magnitude, shift);
            self.exponent += shift as i32;
            if self.magnitude.bits() > wanted {
                self.magnitude >>= 1usize;
                self.exponent += 1;
            }
        }
        self
    }

    pub(crate) fn add(&self, other: &Self, precision: X87Precision) -> Self {
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

    pub(crate) fn sub(&self, other: &Self, precision: X87Precision) -> Self {
        self.add(&other.clone().neg(), precision)
    }

    pub(crate) fn mul(&self, other: &Self, precision: X87Precision) -> Self {
        Self {
            negative: self.negative ^ other.negative,
            magnitude: &self.magnitude * &other.magnitude,
            exponent: self.exponent + other.exponent,
        }
        .rounded(precision)
    }

    /// Correctly rounded quotient at the selected x87 precision (FDIV/FDIVR).
    /// Division by zero is outside the supported finite domain.
    pub(crate) fn div(&self, other: &Self, precision: X87Precision) -> Result<Self, EconomyError> {
        if other.magnitude.is_zero() {
            return Err(EconomyError::DivisionByZero);
        }
        if self.magnitude.is_zero() {
            return Ok(Self {
                negative: self.negative ^ other.negative,
                magnitude: BigUint::zero(),
                exponent: 0,
            });
        }
        let mut top = self.magnitude.bits() as i32 - other.magnitude.bits() as i32;
        let less = if top >= 0 {
            self.magnitude < (&other.magnitude << top as usize)
        } else {
            (&self.magnitude << (-top) as usize) < other.magnitude
        };
        if less {
            top -= 1;
        }
        // Two guard bits beyond the target plus an exact sticky remainder.
        let shift = precision.significand_bits() as i32 + 1 - top;
        let (numerator, denominator) = if shift >= 0 {
            (&self.magnitude << shift as usize, other.magnitude.clone())
        } else {
            (
                self.magnitude.clone(),
                &other.magnitude << (-shift) as usize,
            )
        };
        let quotient = &numerator / &denominator;
        let sticky = !(numerator - &quotient * &denominator).is_zero();
        let magnitude = (quotient << 1usize) | BigUint::from(u8::from(sticky));
        Ok(Self {
            negative: self.negative ^ other.negative,
            magnitude,
            exponent: self.exponent - other.exponent - shift - 1,
        }
        .rounded(precision))
    }

    pub(crate) fn is_zero(&self) -> bool {
        self.magnitude.is_zero()
    }

    /// Value of st(0) converted toward zero by the CRT _ftol (0x96CC4C).
    /// Results outside signed 32-bit range are rejected rather than emulating
    /// the 64-bit FISTP truncation into EAX.
    pub(crate) fn truncate_i32(&self) -> Result<i32, EconomyError> {
        let magnitude = if self.exponent >= 0 {
            &self.magnitude << self.exponent as usize
        } else {
            &self.magnitude >> (-self.exponent) as usize
        };
        let value = magnitude.to_u64().ok_or(EconomyError::IntegerRange)?;
        let signed = if self.negative {
            -(value as i128)
        } else {
            value as i128
        };
        i32::try_from(signed).map_err(|_| EconomyError::IntegerRange)
    }

    /// Exact floor toward negative infinity, as an i32 (CRT floor then _ftol).
    pub(crate) fn floor_i32(&self) -> Result<i32, EconomyError> {
        let (whole, fraction) = if self.exponent >= 0 {
            (&self.magnitude << self.exponent as usize, false)
        } else {
            let shift = (-self.exponent) as usize;
            let whole = &self.magnitude >> shift;
            let fraction = (&whole << shift) != self.magnitude;
            (whole, fraction)
        };
        let whole = whole.to_u64().ok_or(EconomyError::IntegerRange)? as i128;
        let value = if self.negative {
            -whole - i128::from(fraction)
        } else {
            whole
        };
        i32::try_from(value).map_err(|_| EconomyError::IntegerRange)
    }

    /// Exact floor toward negative infinity.
    pub(crate) fn floor(&self) -> Self {
        if self.exponent >= 0 {
            return self.clone();
        }
        let shift = (-self.exponent) as usize;
        let whole = &self.magnitude >> shift;
        let fraction = (&whole << shift) != self.magnitude;
        let magnitude = if self.negative && fraction {
            whole + BigUint::one()
        } else {
            whole
        };
        Self {
            negative: self.negative,
            magnitude,
            exponent: 0,
        }
    }

    pub(crate) fn is_negative_nonzero(&self) -> bool {
        self.negative && !self.magnitude.is_zero()
    }

    pub(crate) fn cmp(&self, other: &Self) -> Ordering {
        if self.magnitude.is_zero() && other.magnitude.is_zero() {
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

    pub(crate) fn store_f32(&self) -> Result<F32Bits, EconomyError> {
        let sign = u32::from(self.negative) << 31;
        if self.magnitude.is_zero() {
            return Ok(F32Bits(sign));
        }
        let mut top_exponent = self.magnitude.bits() as i32 - 1 + self.exponent;
        let target_exponent = (top_exponent - 23).max(-149);
        let mut rounded = if target_exponent > self.exponent {
            rounded_shift(&self.magnitude, (target_exponent - self.exponent) as u64)
        } else {
            &self.magnitude << (self.exponent - target_exponent) as usize
        };
        if top_exponent < -126 {
            return Ok(F32Bits(
                sign | rounded.to_u32().expect("subnormal significand fits"),
            ));
        }
        if rounded.bits() > 24 {
            rounded >>= 1usize;
            top_exponent += 1;
        }
        if top_exponent > 127 {
            return Err(EconomyError::NonFiniteResult);
        }
        Ok(F32Bits(
            sign | (((top_exponent + 127) as u32) << 23)
                | (rounded.to_u32().expect("binary32 significand fits") & 0x7f_ffff),
        ))
    }
}
