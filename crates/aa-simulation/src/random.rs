//! Original RNG initialization, bounded draw and explicit binary32 float boundary.
//! Native seed sources and authoritative draw order are caller responsibilities.
use crate::{FloatBits, NativeRng};
use aa_numeric::{Binary, Precision};

#[derive(Debug, thiserror::Error)]
pub enum RandomError {
    #[error("random integer bound must be nonzero")]
    ZeroBound,
    #[error(transparent)]
    Arithmetic(#[from] aa_numeric::Error),
}

const DEFAULT_STATE: u32 = 0x8e4b_12a5;
impl Default for NativeRng {
    /// Original default constructor 0x85D870.
    fn default() -> Self {
        Self {
            state: DEFAULT_STATE,
        }
    }
}
impl NativeRng {
    /// Original seeded constructor 0x85D8A0, also used by assignment 0x85D880.
    /// Distinct from `new`, which preserves the older raw-state restore API.
    pub const fn from_native_seed(seed: u32) -> Self {
        let mixed = ((seed << 16) | seed) ^ DEFAULT_STATE;
        Self {
            state: if mixed == 0 { DEFAULT_STATE } else { mixed },
        }
    }
    pub fn reseed(&mut self, seed: u32) {
        *self = Self::from_native_seed(seed);
    }

    /// Native 0x40C8A0: one wide-output draw followed by unsigned remainder.
    /// No rejection sampling: replacing modulo would change native draw order.
    /// Zero is rejected before mutation, instead of native division-by-zero.
    pub fn next_bounded(&mut self, bound: u32) -> Result<u32, RandomError> {
        if bound == 0 {
            return Err(RandomError::ZeroBound);
        }
        Ok(self.next_u32() % bound)
    }
    /// One draw followed by the original binary32 GetFRand store boundary.
    /// Values can equal 1.0. This must not replace an unrounded x87 intermediate
    /// in a different native caller; the proven fragment ends at FSTP binary32.
    pub fn next_f32(&mut self, precision: Precision) -> Result<FloatBits, RandomError> {
        native_unit_float(self.next_u32(), precision)
    }
}

/// Inline 0x4F62BB..DF. FILD loads signed bits; the high-half path performs a
/// separately rounded addition of 2^32 before multiplication by the original
/// double constant (nearest binary64 reciprocal of u32::MAX), then stores f32.
pub fn native_unit_float(value: u32, precision: Precision) -> Result<FloatBits, RandomError> {
    let mut value_as_binary = Binary::from_i32(value as i32);
    if value & 0x8000_0000 != 0 {
        value_as_binary =
            value_as_binary.add(&Binary::from_f64_bits(0x41f0_0000_0000_0000)?, precision)?;
    }
    Ok(FloatBits(
        value_as_binary
            .mul(&Binary::from_f64_bits(0x3df0_0000_0010_0000)?, precision)?
            .store_f32()?,
    ))
}
