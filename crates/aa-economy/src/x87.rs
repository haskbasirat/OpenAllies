//! A finite x87 register value for native ports outside this crate.
//!
//! Loads are exact (`FLD dword`), every arithmetic result is rounded to the
//! selected precision-control significand, `store_f32` is `FSTP dword` and
//! `round_f64` is `FSTP qword` followed by a reload. Non-finite inputs are
//! rejected; exponent-range effects of the 80-bit format are not modelled.
use crate::binary::Binary;
use crate::{EconomyError, F32Bits, X87Precision};
use std::cmp::Ordering;

#[derive(Clone)]
pub struct Register(Binary);

impl Register {
    /// `FLD dword`.
    pub fn load(value: F32Bits) -> Result<Self, EconomyError> {
        if !value.is_finite() {
            return Err(EconomyError::NonFiniteInput);
        }
        Ok(Self(Binary::from_bits(value)))
    }
    /// `FILD dword`.
    pub fn load_i32(value: i32) -> Self {
        Self(Binary::from_i32(value))
    }
    pub fn add(&self, other: &Self, precision: X87Precision) -> Self {
        Self(self.0.add(&other.0, precision))
    }
    pub fn sub(&self, other: &Self, precision: X87Precision) -> Self {
        Self(self.0.sub(&other.0, precision))
    }
    pub fn mul(&self, other: &Self, precision: X87Precision) -> Self {
        Self(self.0.mul(&other.0, precision))
    }
    /// Ordered comparison (`FCOM` without the unordered case).
    pub fn compare(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
    /// `FSTP qword` then `FLD qword` (53-bit significand).
    pub fn round_f64(self) -> Self {
        Self(self.0.rounded(X87Precision::Double53))
    }
    /// Exact floor toward negative infinity (CRT `floor` on an integral-range
    /// double returns an exact integer).
    pub fn floor(&self) -> Self {
        Self(self.0.floor())
    }
    /// `FSTP dword`.
    pub fn store_f32(&self) -> Result<F32Bits, EconomyError> {
        self.0.store_f32()
    }
}
