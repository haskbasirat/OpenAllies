//! Fast finite x87 arithmetic: [`Fast`] is a `Copy` register value with the
//! same results as [`Binary`], bit for bit, without heap allocation.
//!
//! Every operation takes the precision control explicitly, like `Binary`:
//! the exact result is rounded once to 24, 53 or 64 significand bits (round
//! to nearest, ties to even) and must stay in the normal extended exponent
//! domain (top bit -16382..=16383), else `ExtendedExponentRange`. Signed
//! zeros, errors and the binary32 store (gradual underflow, overflow error)
//! follow `Binary` exactly.
//!
//! A value is held in one of two canonical forms: a host `f64` when it is
//! exactly zero or a normal binary64, otherwise sign, normalized 64-bit
//! significand and extended exponent. Two implementations, chosen per
//! operation:
//! - **Host.** At `Double53`, with both operands in the `f64` form, the host
//!   binary64 operation (IEEE, round to nearest even; SSE2, no excess
//!   precision, no contraction) is used when its result is zero by exact
//!   cancellation or a zero operand, or finite with magnitude >= 2^-1021. In
//!   that range the binary64 result is the exact result rounded once to 53
//!   bits, which is what PC53 computes: binary64 only differs from PC53
//!   through its smaller exponent range, i.e. for results that overflow or
//!   fall in (or round from) its subnormal range, and those go to the integer
//!   path. `Single24` uses the host binary32 operations under the same rule
//!   (operands normal binary32, results >= 2^-125). The binary32 store of a
//!   PC53 value is the native double rounding (exact -> 53 bits in the
//!   register -> 24 bits by FST m32); `f64 as f32` performs exactly that
//!   second rounding on the register value. Never compute a PC53 expression
//!   with host f32 operations.
//! - **Integer.** Otherwise the exact result is formed in `u128` (products
//!   exactly; sums, quotients and roots with guard bits plus a sticky bit
//!   placed so that no rounding boundary can fall between the stand-in and
//!   the exact value) and rounded once.
//!
//! `tests/fast.rs` proves both paths against `Binary` over the native
//! fixtures and randomized operands (ties, both exponent-range edges, every
//! precision) and benchmarks them.
use crate::{Binary, Error, Precision, Result};
use num_traits::ToPrimitive;
use std::cmp::Ordering;

const MIN_TOP: i64 = -16382;
const MAX_TOP: i64 = 16383;
/// 2^-1021 and 2^-125: host results at least this large are PC53 / PC24 results.
const SAFE64: f64 = f64::from_bits(0x0020_0000_0000_0000);
const SAFE32: f32 = f32::from_bits(0x0100_0000);

/// A finite x87 extended value.
#[derive(Clone, Copy, Debug)]
pub struct Fast(Repr);

#[derive(Clone, Copy, Debug)]
enum Repr {
    /// Exactly zero (signed) or a normal binary64.
    Host(f64),
    /// Everything else (never zero).
    Wide(Wide),
}

/// `sig * 2^exp`, `sig` normalized (bit 63 set) unless zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Wide {
    negative: bool,
    exp: i32,
    sig: u64,
}

fn wanted(precision: Precision) -> u32 {
    precision.significand_bits() as u32
}

/// Round `value * 2^unit` (`value > 0`) once to `bits` significand bits,
/// ties to even, then check the normal extended exponent domain.
fn round(negative: bool, value: u128, unit: i64, bits: u32) -> Result<Wide> {
    debug_assert!(value != 0);
    let have = 128 - value.leading_zeros();
    let (mut q, mut unit) = if have > bits {
        let shift = have - bits;
        let q = value >> shift;
        let rem = value & ((1u128 << shift) - 1);
        let half = 1u128 << (shift - 1);
        let up = rem > half || (rem == half && q & 1 == 1);
        (q + u128::from(up), unit + i64::from(shift))
    } else {
        (value, unit)
    };
    if q >> bits != 0 {
        q >>= 1;
        unit += 1;
    }
    let qbits = 128 - q.leading_zeros();
    let sig = (q << (64 - qbits)) as u64;
    let exp = unit - i64::from(64 - qbits);
    if !(MIN_TOP..=MAX_TOP).contains(&(exp + 63)) {
        return Err(Error::ExtendedExponentRange);
    }
    Ok(Wide { negative, exp: exp as i32, sig })
}

/// Integer square root (floor) of a `u128`.
fn isqrt(n: u128) -> u128 {
    if n < 2 {
        return n;
    }
    // The binary64 estimate is within 2^11 of the root for n < 2^128; start
    // above it and run integer Newton steps down to floor(sqrt(n)).
    let mut x = ((n as f64).sqrt() as u128).saturating_add(1 << 12).min(1u128 << 64);
    loop {
        let y = (x + n / x) >> 1;
        if y >= x {
            break;
        }
        x = y;
    }
    debug_assert!(x * x <= n && (x + 1).checked_mul(x + 1).is_none_or(|s| s > n));
    x
}

impl Wide {
    const fn zero(negative: bool) -> Self {
        Self { negative, exp: 0, sig: 0 }
    }
    fn from_parts(negative: bool, sig: u64, exp: i64) -> Self {
        if sig == 0 {
            return Self::zero(negative);
        }
        let lz = sig.leading_zeros();
        Self { negative, exp: (exp - i64::from(lz)) as i32, sig: sig << lz }
    }
    fn from_f64_bits(bits: u64) -> Self {
        let field = (bits >> 52) & 2047;
        let sig = (bits & 0x000f_ffff_ffff_ffff) | if field == 0 { 0 } else { 1 << 52 };
        let exp = if field == 0 { -1074 } else { field as i64 - 1075 };
        Self::from_parts(bits >> 63 != 0, sig, exp)
    }
    /// The value as a binary64 when it is exactly zero or a normal binary64.
    fn host64(&self) -> Option<f64> {
        if self.sig == 0 {
            return Some(if self.negative { -0.0 } else { 0.0 });
        }
        let top = i64::from(self.exp) + 63;
        if self.sig & 0x7ff != 0 || !(-1022..=1023).contains(&top) {
            return None;
        }
        Some(f64::from_bits((u64::from(self.negative) << 63) | (((top + 1023) as u64) << 52) | ((self.sig >> 11) & 0x000f_ffff_ffff_ffff)))
    }
    fn rounded(&self, precision: Precision) -> Result<Self> {
        if self.sig == 0 {
            return Ok(*self);
        }
        round(self.negative, u128::from(self.sig), i64::from(self.exp), wanted(precision))
    }
    fn add(&self, other: &Self, precision: Precision) -> Result<Self> {
        if self.sig == 0 && other.sig == 0 {
            return Ok(Self::zero(self.negative && other.negative));
        }
        if self.sig == 0 {
            return other.rounded(precision);
        }
        if other.sig == 0 {
            return self.rounded(precision);
        }
        let (a, b) = if (self.exp, self.sig) >= (other.exp, other.sig) { (self, other) } else { (other, self) };
        let d = i64::from(a.exp) - i64::from(b.exp);
        const S: i64 = 62;
        let av = u128::from(a.sig) << S;
        let (bv, sticky) = if d <= S {
            (u128::from(b.sig) << (S - d), false)
        } else if d - S < 64 {
            let s = (d - S) as u32;
            (u128::from(b.sig >> s), b.sig & ((1u64 << s) - 1) != 0)
        } else {
            (0, true)
        };
        let mut unit = i64::from(a.exp) - S;
        // A lost tail: double the scale and stand in an odd last bit, so the
        // exact value and the stand-in share every rounding boundary.
        let (av, bv) = if sticky {
            unit -= 1;
            (av << 1, (bv << 1) | 1)
        } else {
            (av, bv)
        };
        let magnitude = if a.negative == b.negative { av + bv } else { av - bv };
        if magnitude == 0 {
            return Ok(Self::zero(false));
        }
        round(a.negative, magnitude, unit, wanted(precision))
    }
    fn mul(&self, other: &Self, precision: Precision) -> Result<Self> {
        let negative = self.negative ^ other.negative;
        if self.sig == 0 || other.sig == 0 {
            return Ok(Self::zero(negative));
        }
        let product = u128::from(self.sig) * u128::from(other.sig);
        round(negative, product, i64::from(self.exp) + i64::from(other.exp), wanted(precision))
    }
    fn div(&self, other: &Self, precision: Precision) -> Result<Self> {
        if other.sig == 0 {
            return Err(Error::DivisionByZero);
        }
        let negative = self.negative ^ other.negative;
        if self.sig == 0 {
            return Ok(Self::zero(negative));
        }
        let d = u128::from(other.sig);
        let n = u128::from(self.sig) << 64;
        let (q1, r1) = (n / d, n % d);
        let n2 = r1 << 2;
        let (q2, r2) = (n2 / d, n2 % d);
        // floor(4 * 2^64 * a / b) with an odd stand-in for a nonzero tail.
        let q = (((q1 << 2) | q2) << 1) | u128::from(r2 != 0);
        round(negative, q, i64::from(self.exp) - i64::from(other.exp) - 67, wanted(precision))
    }
    fn sqrt(&self, precision: Precision) -> Result<Self> {
        if self.sig == 0 {
            return Ok(*self);
        }
        if self.negative {
            return Err(Error::NegativeSquareRoot);
        }
        // value = n * 2^e with e even and n in [2^126, 2^128).
        let shifted = i64::from(self.exp) - 64;
        let (n, e) = if shifted.rem_euclid(2) == 0 { (u128::from(self.sig) << 64, shifted) } else { (u128::from(self.sig) << 63, shifted + 1) };
        let mut root = isqrt(n);
        let mut rem = n - root * root;
        for _ in 0..2 {
            rem <<= 2;
            let trial = (root << 2) | 1;
            root <<= 1;
            if rem >= trial {
                rem -= trial;
                root |= 1;
            }
        }
        let v = (root << 1) | u128::from(rem != 0);
        round(false, v, e / 2 - 3, wanted(precision))
    }
    fn remainder_trunc(&self, divisor: &Self) -> Result<Self> {
        if divisor.sig == 0 {
            return Err(Error::DivisionByZero);
        }
        if self.sig == 0 {
            return Ok(*self);
        }
        let (rem, unit) = if self.exp < divisor.exp {
            // |dividend| < |divisor|: the dividend itself.
            (self.sig, self.exp)
        } else {
            let d = u128::from(divisor.sig);
            let mut r = u128::from(self.sig) % d;
            let mut shift = self.exp - divisor.exp;
            while shift > 0 && r != 0 {
                let step = shift.min(64);
                r = (r << step) % d;
                shift -= step;
            }
            (r as u64, divisor.exp)
        };
        if rem == 0 {
            return Ok(Self::zero(self.negative));
        }
        let lz = rem.leading_zeros();
        let exp = i64::from(unit) - i64::from(lz);
        if !(MIN_TOP..=MAX_TOP).contains(&(exp + 63)) {
            return Err(Error::ExtendedExponentRange);
        }
        Ok(Self { negative: self.negative, exp: exp as i32, sig: rem << lz })
    }
    fn compare(&self, other: &Self) -> Ordering {
        match (self.sig == 0, other.sig == 0) {
            (true, true) => return Ordering::Equal,
            (true, false) => return if other.negative { Ordering::Greater } else { Ordering::Less },
            (false, true) => return if self.negative { Ordering::Less } else { Ordering::Greater },
            _ => {}
        }
        if self.negative != other.negative {
            return if self.negative { Ordering::Less } else { Ordering::Greater };
        }
        let order = (self.exp, self.sig).cmp(&(other.exp, other.sig));
        if self.negative { order.reverse() } else { order }
    }
    fn store_f32(&self) -> Result<u32> {
        let sign = u32::from(self.negative) << 31;
        if self.sig == 0 {
            return Ok(sign);
        }
        let mut top = i64::from(self.exp) + 63;
        let target = (top - 23).max(-149);
        let shift = target - i64::from(self.exp);
        debug_assert!(shift > 0);
        let mut rounded: u64 = if shift >= 65 {
            0
        } else {
            let v = u128::from(self.sig);
            let s = shift as u32;
            let q = v >> s;
            let rem = v & ((1u128 << s) - 1);
            let half = 1u128 << (s - 1);
            (q + u128::from(rem > half || (rem == half && q & 1 == 1))) as u64
        };
        if top < -126 {
            return Ok(sign | rounded as u32);
        }
        if rounded >> 24 != 0 {
            rounded >>= 1;
            top += 1;
        }
        if top > 127 {
            return Err(Error::Binary32Overflow);
        }
        Ok(sign | (((top + 127) as u32) << 23) | (rounded as u32 & 0x7f_ffff))
    }
    fn truncate_i32(&self) -> Result<i32> {
        let magnitude: u64 = if self.sig == 0 {
            0
        } else if self.exp >= 0 {
            if self.exp >= 64 || self.sig.leading_zeros() < self.exp as u32 {
                return Err(Error::IntegerRange);
            }
            self.sig << self.exp
        } else if self.exp <= -64 {
            0
        } else {
            self.sig >> (-self.exp)
        };
        let value = u32::try_from(magnitude).map_err(|_| Error::IntegerRange)?;
        if self.negative {
            if value > 0x8000_0000 {
                return Err(Error::IntegerRange);
            }
            Ok(0u32.wrapping_sub(value) as i32)
        } else {
            i32::try_from(value).map_err(|_| Error::IntegerRange)
        }
    }
    fn to_x87_extended_bits(self) -> u128 {
        let sign = u128::from(self.negative) << 79;
        if self.sig == 0 {
            return sign;
        }
        sign | (((i64::from(self.exp) + 63 + 16383) as u128) << 64) | u128::from(self.sig)
    }
}

impl From<Wide> for Fast {
    /// The canonical form.
    #[inline]
    fn from(w: Wide) -> Self {
        match w.host64() {
            Some(v) => Fast(Repr::Host(v)),
            None => Fast(Repr::Wide(w)),
        }
    }
}

impl PartialEq for Fast {
    fn eq(&self, other: &Self) -> bool {
        self.to_x87_extended_bits() == other.to_x87_extended_bits()
    }
}
impl Eq for Fast {}
impl std::hash::Hash for Fast {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.to_x87_extended_bits().hash(state);
    }
}

/// A host binary32 operand: exactly zero or a normal binary32.
#[inline]
fn as_f32(v: f64) -> Option<f32> {
    let f = v as f32;
    (f64::from(f) == v && (f == 0.0 || f.abs() >= f32::MIN_POSITIVE)).then_some(f)
}
#[inline]
fn accept64(r: f64, zero_ok: bool) -> Option<Fast> {
    if r == 0.0 {
        return zero_ok.then_some(Fast(Repr::Host(r)));
    }
    (r.is_finite() && r.abs() >= SAFE64).then_some(Fast(Repr::Host(r)))
}
#[inline]
fn accept32(r: f32, zero_ok: bool) -> Option<Fast> {
    if r == 0.0 {
        return zero_ok.then_some(Fast(Repr::Host(f64::from(r))));
    }
    (r.is_finite() && r.abs() >= SAFE32).then_some(Fast(Repr::Host(f64::from(r))))
}

impl Fast {
    #[inline]
    pub const fn zero(negative: bool) -> Self {
        Fast(Repr::Host(if negative { -0.0 } else { 0.0 }))
    }
    #[inline]
    fn wide(&self) -> Wide {
        match self.0 {
            Repr::Host(v) => Wide::from_f64_bits(v.to_bits()),
            Repr::Wide(w) => w,
        }
    }
    #[inline]
    fn host(&self) -> Option<f64> {
        match self.0 {
            Repr::Host(v) => Some(v),
            Repr::Wide(_) => None,
        }
    }
    #[inline]
    pub fn from_f64_bits(bits: u64) -> Result<Self> {
        let field = (bits >> 52) & 2047;
        if field == 2047 {
            return Err(Error::NonFiniteInput);
        }
        if field != 0 || bits << 1 == 0 {
            return Ok(Fast(Repr::Host(f64::from_bits(bits))));
        }
        Ok(Fast(Repr::Wide(Wide::from_f64_bits(bits))))
    }
    #[inline]
    pub fn from_f32_bits(bits: u32) -> Result<Self> {
        if (bits >> 23) & 255 == 255 {
            return Err(Error::NonFiniteInput);
        }
        // every binary32 is a normal binary64 or zero
        Ok(Fast(Repr::Host(f64::from(f32::from_bits(bits)))))
    }
    #[inline]
    pub fn from_f32(value: f32) -> Result<Self> {
        Self::from_f32_bits(value.to_bits())
    }
    #[inline]
    pub fn from_f64(value: f64) -> Result<Self> {
        Self::from_f64_bits(value.to_bits())
    }
    #[inline]
    pub fn from_u32(value: u32) -> Self {
        Fast(Repr::Host(f64::from(value)))
    }
    #[inline]
    pub fn from_i32(value: i32) -> Self {
        Fast(Repr::Host(f64::from(value)))
    }
    #[inline]
    pub fn negated(self) -> Self {
        match self.0 {
            Repr::Host(v) => Fast(Repr::Host(-v)),
            Repr::Wide(mut w) => {
                w.negative = !w.negative;
                Fast(Repr::Wide(w))
            }
        }
    }
    #[inline]
    pub fn is_zero(&self) -> bool {
        matches!(self.0, Repr::Host(v) if v == 0.0)
    }
    #[inline]
    pub fn is_negative(&self) -> bool {
        match self.0 {
            Repr::Host(v) => v.is_sign_negative(),
            Repr::Wide(w) => w.negative,
        }
    }
    #[inline]
    pub fn is_negative_nonzero(&self) -> bool {
        self.is_negative() && !self.is_zero()
    }

    #[inline]
    pub fn add(&self, other: &Self, precision: Precision) -> Result<Self> {
        if let (Some(a), Some(b)) = (self.host(), other.host()) {
            match precision {
                Precision::Double53 => {
                    if let Some(r) = accept64(a + b, true) {
                        return Ok(r);
                    }
                }
                Precision::Single24 => {
                    if let (Some(a), Some(b)) = (as_f32(a), as_f32(b))
                        && let Some(r) = accept32(a + b, true)
                    {
                        return Ok(r);
                    }
                }
                Precision::Extended64 => {}
            }
        }
        self.wide().add(&other.wide(), precision).map(Fast::from)
    }
    #[inline]
    pub fn sub(&self, other: &Self, precision: Precision) -> Result<Self> {
        self.add(&other.negated(), precision)
    }
    #[inline]
    pub fn mul(&self, other: &Self, precision: Precision) -> Result<Self> {
        if let (Some(a), Some(b)) = (self.host(), other.host()) {
            match precision {
                Precision::Double53 => {
                    if let Some(r) = accept64(a * b, a == 0.0 || b == 0.0) {
                        return Ok(r);
                    }
                }
                Precision::Single24 => {
                    if let (Some(a), Some(b)) = (as_f32(a), as_f32(b))
                        && let Some(r) = accept32(a * b, a == 0.0 || b == 0.0)
                    {
                        return Ok(r);
                    }
                }
                Precision::Extended64 => {}
            }
        }
        self.wide().mul(&other.wide(), precision).map(Fast::from)
    }
    #[inline]
    pub fn div(&self, other: &Self, precision: Precision) -> Result<Self> {
        if let (Some(a), Some(b)) = (self.host(), other.host()) {
            if b == 0.0 {
                return Err(Error::DivisionByZero);
            }
            match precision {
                Precision::Double53 => {
                    if let Some(r) = accept64(a / b, a == 0.0) {
                        return Ok(r);
                    }
                }
                Precision::Single24 => {
                    if let (Some(a), Some(b)) = (as_f32(a), as_f32(b))
                        && let Some(r) = accept32(a / b, a == 0.0)
                    {
                        return Ok(r);
                    }
                }
                Precision::Extended64 => {}
            }
        }
        self.wide().div(&other.wide(), precision).map(Fast::from)
    }
    /// Correctly rounded FSQRT at `precision`; -0 stays -0.
    #[inline]
    pub fn sqrt(&self, precision: Precision) -> Result<Self> {
        if let Some(a) = self.host() {
            if a == 0.0 {
                return Ok(*self);
            }
            if a < 0.0 {
                return Err(Error::NegativeSquareRoot);
            }
            match precision {
                Precision::Double53 => {
                    if let Some(r) = accept64(a.sqrt(), false) {
                        return Ok(r);
                    }
                }
                Precision::Single24 => {
                    if let Some(a) = as_f32(a)
                        && let Some(r) = accept32(a.sqrt(), false)
                    {
                        return Ok(r);
                    }
                }
                Precision::Extended64 => {}
            }
        }
        self.wide().sqrt(precision).map(Fast::from)
    }
    /// Exact remainder after a quotient truncated toward zero (FPREM); the
    /// dividend's sign, no PC rounding.
    pub fn remainder_trunc(&self, divisor: &Self) -> Result<Self> {
        self.wide().remainder_trunc(&divisor.wide()).map(Fast::from)
    }
    /// Numeric comparison; signed zeros compare equal.
    #[inline]
    pub fn compare(&self, other: &Self) -> Ordering {
        if let (Some(a), Some(b)) = (self.host(), other.host()) {
            return a.partial_cmp(&b).expect("finite");
        }
        self.wide().compare(&other.wide())
    }
    /// FST m32: round to binary32 (gradual underflow); overflow is an error.
    #[inline]
    pub fn store_f32(&self) -> Result<u32> {
        match self.0 {
            Repr::Host(v) => {
                let r = v as f32;
                if r.is_finite() { Ok(r.to_bits()) } else { Err(Error::Binary32Overflow) }
            }
            Repr::Wide(w) => w.store_f32(),
        }
    }
    /// The binary32 store as a host value.
    #[inline]
    pub fn to_f32(&self) -> Result<f32> {
        self.store_f32().map(f32::from_bits)
    }
    /// The value as a host binary64 when it is one exactly (subnormals
    /// included), else None.
    pub fn to_f64_exact(&self) -> Option<f64> {
        if let Some(v) = self.host() {
            return Some(v);
        }
        let w = self.wide();
        let top = i64::from(w.exp) + 63;
        if !(-1074..-1022).contains(&top) {
            return None;
        }
        let shift = -1074 - i64::from(w.exp);
        if shift >= 64 || w.sig & ((1u64 << shift) - 1) != 0 {
            return None;
        }
        Some(f64::from_bits((u64::from(w.negative) << 63) | (w.sig >> shift)))
    }
    /// Integer conversion toward zero (no binary32 store first).
    #[inline]
    pub fn truncate_i32(&self) -> Result<i32> {
        if let Some(v) = self.host() {
            let t = v.trunc();
            return if (-2_147_483_648.0..=2_147_483_647.0).contains(&t) { Ok(t as i32) } else { Err(Error::IntegerRange) };
        }
        self.wide().truncate_i32()
    }
    /// Value of an 80-bit register image (`FSTP TBYTE`, low 80 bits);
    /// pseudo-denormals and unnormals by value.
    pub fn from_x87_extended_bits(bits: u128) -> Result<Self> {
        let negative = (bits >> 79) & 1 != 0;
        let field = ((bits >> 64) & 0x7fff) as i64;
        let significand = bits as u64;
        if field == 0x7fff {
            return Err(Error::NonFiniteInput);
        }
        if significand == 0 {
            return Ok(Self::zero(negative));
        }
        let unit = if field == 0 { -16382 - 63 } else { field - 16383 - 63 };
        let value = Wide::from_parts(negative, significand, unit);
        if !(MIN_TOP..=MAX_TOP).contains(&(i64::from(value.exp) + 63)) {
            return Err(Error::ExtendedExponentRange);
        }
        Ok(value.into())
    }
    /// The 80-bit register encoding.
    pub fn to_x87_extended_bits(&self) -> u128 {
        self.wide().to_x87_extended_bits()
    }
    /// The integer path alone (no host shortcut), for the differential tests.
    #[doc(hidden)]
    pub fn integer_path(&self, other: &Self, operation: char, precision: Precision) -> Result<Self> {
        let (a, b) = (self.wide(), other.wide());
        match operation {
            '+' => a.add(&b, precision),
            '-' => a.add(&Wide { negative: !b.negative, ..b }, precision),
            '*' => a.mul(&b, precision),
            '/' => a.div(&b, precision),
            _ => a.sqrt(precision),
        }
        .map(Fast::from)
    }
    /// The same value as a [`Binary`].
    pub fn to_binary(&self) -> Binary {
        let w = self.wide();
        Binary { negative: w.negative, magnitude: w.sig.into(), exponent: if w.sig == 0 { 0 } else { w.exp } }
    }
    /// A [`Binary`] value (at most 64 significant bits from the public API);
    /// None when it would need more.
    pub fn from_binary(value: &Binary) -> Option<Self> {
        if value.magnitude.bits() > 64 {
            return None;
        }
        let sig = value.magnitude.to_u64()?;
        Some(Wide::from_parts(value.negative, sig, i64::from(value.exponent)).into())
    }
}
