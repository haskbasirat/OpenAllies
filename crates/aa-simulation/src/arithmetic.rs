use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Preserve IEEE-754 bit patterns through snapshots, including signed zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FloatBits(pub u32);

impl FloatBits {
    pub const ZERO: Self = Self(0);
    pub const fn from_f32(value: f32) -> Self {
        Self(value.to_bits())
    }
    pub const fn as_f32(self) -> f32 {
        f32::from_bits(self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum RuleError {
    #[error("provider arithmetic currently supports finite quantities only")]
    NonFinite,
    #[error("clock step must be finite and positive")]
    InvalidClockStep,
}

/// Exact integer primitive at original AA.exe VA 0x85D840.
///
/// The returned value is bits 16..47 of the *wide* product plus increment.
/// Returning only the updated state's high 16 bits would be incorrect.
/// Native seed mixing and binary32 GetFRand conversion are in `random`.
/// Authoritative call-site order and the source of game seeds remain unported.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeRng {
    pub state: u32,
}

impl NativeRng {
    pub const fn new(seed: u32) -> Self {
        // Legacy laboratory API: this restores raw state, without native mixing.
        Self { state: seed }
    }
    pub fn next_u32(&mut self) -> u32 {
        let wide = u64::from(self.state) * 0x41c6_4e6d + 0x3039;
        self.state = wide as u32;
        (wide >> 16) as u32
    }
}

/// The clock-field fragment of 0x4F9970, NOT the complete GWorld update.
///
/// Other calls/phases and game-speed/admin-clock scheduling are not reproduced.
/// The stock step is 1/16 second. Custom steps are explicit inputs; original
/// game parity is established only by the checked native oracle cases.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeClock {
    pub tick: u32,
    pub begin: FloatBits,
    pub end: FloatBits,
    pub step: FloatBits,
}

impl NativeClock {
    pub fn stock() -> Self {
        Self::with_step(FloatBits::from_f32(0.0625)).expect("stock step")
    }
    pub fn with_step(step: FloatBits) -> Result<Self, RuleError> {
        if !step.as_f32().is_finite() || step.as_f32() <= 0.0 {
            return Err(RuleError::InvalidClockStep);
        }
        Ok(Self {
            tick: 0,
            begin: FloatBits::ZERO,
            end: FloatBits::ZERO,
            step,
        })
    }
    pub fn advance_fragment(&mut self) {
        self.tick = self.tick.wrapping_add(1);
        self.begin = self.end;
        // Native FILD reads the incremented field as signed i32, then multiplies
        // the stored float step and stores f32. Avoid an intermediate f32 cast
        // of the integer and avoid host FMA/x87 precision dependence.
        self.end = signed_integer_times_positive_float(self.tick as i32, self.step);
    }
    ///4F9970 with caller-proven x87 precision. Arbitrary stored steps can round
    /// at the multiply and again at the final binary32 store, unlike the exact
    /// product shortcut used by the stock dyadic-step laboratory above.
    pub fn advance_fragment_at_precision(
        &mut self,
        precision: aa_numeric::Precision,
    ) -> aa_numeric::Result<()> {
        self.tick = self.tick.wrapping_add(1);
        self.begin = self.end;
        self.end = FloatBits(
            aa_numeric::Binary::from_i32(self.tick as i32)
                .mul(&aa_numeric::Binary::from_f32_bits(self.step.0)?, precision)?
                .store_f32()?,
        );
        Ok(())
    }
}

fn round_shift_even(value: u64, shift: u32) -> u64 {
    if shift == 0 {
        return value;
    }
    if shift >= 64 {
        return 0;
    }
    let quotient = value >> shift;
    let remainder = value & ((1u64 << shift) - 1);
    let half = 1u64 << (shift - 1);
    quotient + u64::from(remainder > half || (remainder == half && quotient & 1 != 0))
}

/// One exact product followed by round-to-nearest, ties-to-even f32 storage.
/// Multiplying a 24-bit significand by a 32-bit signed magnitude fits in u64.
fn signed_integer_times_positive_float(integer: i32, value: FloatBits) -> FloatBits {
    let sign = if integer < 0 { 0x8000_0000 } else { 0 };
    let exponent_field = (value.0 >> 23) & 0xff;
    let fraction = value.0 & 0x7f_ffff;
    let significand = if exponent_field == 0 {
        fraction
    } else {
        fraction | 0x80_0000
    };
    let exponent = if exponent_field == 0 {
        -149
    } else {
        exponent_field as i32 - 150
    };
    let product = u64::from(significand) * u64::from(integer.unsigned_abs());
    if product == 0 {
        return FloatBits(sign);
    }
    let bits = 64 - product.leading_zeros();
    let mut unbiased = bits as i32 - 1 + exponent;
    if unbiased < -126 {
        let shift = exponent + 149;
        let fraction = if shift >= 0 {
            product << shift as u32
        } else {
            round_shift_even(product, (-shift) as u32)
        };
        return FloatBits(sign | fraction as u32);
    }
    let mut rounded = if bits > 24 {
        round_shift_even(product, bits - 24)
    } else {
        product << (24 - bits)
    };
    if rounded == 1 << 24 {
        rounded >>= 1;
        unbiased += 1;
    }
    if unbiased > 127 {
        return FloatBits(sign | 0x7f80_0000);
    }
    FloatBits(sign | (((unbiased + 127) as u32) << 23) | (rounded as u32 & 0x7f_ffff))
}

/// Isolated raw/effective-provider arithmetic from 0x4E3940.
///
/// This is NOT a full kingdom economy: the surrounding 0x4E1F60 shortage/upkeep
/// rebalance is deliberately absent, so these rows must remain independent
/// arithmetic fixtures. No recruitment, production event or money debit is
/// implied by updating a row.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderState {
    pub raw: FloatBits,
    pub effective: FloatBits,
    pub limited: bool,
    pub maximum: i32,
}

impl ProviderState {
    pub fn new(raw: FloatBits, limited: bool, maximum: i32) -> Result<Self, RuleError> {
        if !raw.as_f32().is_finite() {
            return Err(RuleError::NonFinite);
        }
        let cap = maximum as f32;
        let effective = if limited && cap < raw.as_f32() {
            FloatBits::from_f32(cap)
        } else {
            raw
        };
        Ok(Self {
            raw,
            effective,
            limited,
            maximum,
        })
    }
    pub fn accumulated(self, delta: FloatBits) -> Result<Self, RuleError> {
        if !delta.as_f32().is_finite() || !self.raw.as_f32().is_finite() {
            return Err(RuleError::NonFinite);
        }
        let raw = self.raw.as_f32() + delta.as_f32();
        if !raw.is_finite() {
            return Err(RuleError::NonFinite);
        }
        Self::new(FloatBits::from_f32(raw), self.limited, self.maximum)
    }
}
