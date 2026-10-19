//! x87 register values under the game's control word, with every exception
//! masked (FPCW 0x007F / 0x027F / 0x037F), round to nearest. Finite values are
//! exact [`Binary`] at the caller's precision control. The masked responses
//! are the documented x87 default results (Intel SDM vol. 1, 4.8.3 and 8.5):
//!
//! * `FST/FSTP m32` of a value past binary32 range stores +-infinity
//!   (overflow masked, round to nearest); gradual underflow is exact
//!   ([`Binary::store_f32`]).
//! * invalid operations (inf - inf, 0 * inf, 0 / 0, inf / inf, sqrt of a
//!   negative nonzero value) give the QNaN "real indefinite" (sign set,
//!   significand 0xC000_0000_0000_0000; binary32 store 0xFFC00000);
//! * division of a nonzero finite value by zero gives a signed infinity;
//! * a NaN operand propagates: of two QNaNs the larger significand, on a tie
//!   the one with the positive sign (QEMU's x87 rule, which the Unicorn
//!   oracles run);
//! * `FCOM`/`FUCOM` with a NaN operand set C3 = C2 = C0 = 1 (unordered).
//!   Callers express every `test ah, mask; jcc` idiom through [`Relation`].
//!
//! A binary32 signaling NaN operand (never produced by x87 arithmetic) and
//! a finite result outside the normal extended exponent range are errors:
//! their native selection rules are not ported.
use aa_numeric::{Binary, Error as NumericError, Precision};
use std::cmp::Ordering;

pub type Result<T> = std::result::Result<T, String>;

const QUIET: u64 = 0x4000_0000_0000_0000;
const INTEGER: u64 = 0x8000_0000_0000_0000;

#[derive(Clone, Debug)]
pub enum Reg {
    Finite(Binary),
    Infinity { negative: bool },
    /// A quiet NaN: sign and the 64-bit register significand (integer and
    /// quiet bits set).
    NaN { negative: bool, significand: u64 },
}

/// Condition codes of `FCOM`/`FUCOM`/`FCOMPP`: C0 (less), C3 (equal), none
/// (greater), all three (unordered).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Relation {
    Less,
    Equal,
    Greater,
    Unordered,
}

fn numeric(error: NumericError) -> String {
    format!("x87 finite arithmetic: {error}")
}

/// Sign bit of a finite value, including the sign of zero.
fn finite_negative(value: &Binary) -> bool {
    if value.is_zero() {
        value.store_f32().is_ok_and(|bits| bits == 0x8000_0000)
    } else {
        value.is_negative_nonzero()
    }
}

fn signed_zero(negative: bool) -> Binary {
    let zero = Binary::from_i32(0);
    if negative { zero.negated() } else { zero }
}

impl Reg {
    /// Real indefinite, the masked invalid-operation result.
    pub const INDEFINITE: Reg = Reg::NaN { negative: true, significand: INTEGER | QUIET };

    /// `FLD m32` (and every `Fop m32` operand conversion).
    pub fn load_f32(bits: u32) -> Result<Self> {
        let negative = bits >> 31 != 0;
        if (bits >> 23) & 0xff != 0xff {
            return Binary::from_f32_bits(bits).map(Reg::Finite).map_err(numeric);
        }
        let fraction = bits & 0x7f_ffff;
        if fraction == 0 {
            return Ok(Reg::Infinity { negative });
        }
        if fraction & 0x40_0000 == 0 {
            return Err(format!("binary32 {bits:#010x} is a signaling NaN: the x87 SNaN operand rules are not ported"));
        }
        Ok(Reg::NaN { negative, significand: INTEGER | (u64::from(fraction) << 40) })
    }
    /// `FILD m32`.
    pub fn from_i32(value: i32) -> Self {
        Reg::Finite(Binary::from_i32(value))
    }
    /// `FST/FSTP m32`.
    pub fn store_f32(&self) -> Result<u32> {
        match self {
            Reg::Finite(value) => match value.store_f32() {
                Ok(bits) => Ok(bits),
                Err(NumericError::Binary32Overflow) => Ok((u32::from(finite_negative(value)) << 31) | 0x7f80_0000),
                Err(e) => Err(numeric(e)),
            },
            Reg::Infinity { negative } => Ok((u32::from(*negative) << 31) | 0x7f80_0000),
            Reg::NaN { negative, significand } => {
                Ok((u32::from(*negative) << 31) | 0x7f80_0000 | ((significand >> 40) as u32 & 0x7f_ffff))
            }
        }
    }
    pub fn finite(&self) -> Option<&Binary> {
        match self {
            Reg::Finite(value) => Some(value),
            _ => None,
        }
    }
    /// Ordered equality with zero (`FTST`/`FUCOM 0` equal); false for NaN.
    pub fn is_zero(&self) -> bool {
        matches!(self, Reg::Finite(value) if value.is_zero())
    }
    fn negative(&self) -> bool {
        match self {
            Reg::Finite(value) => finite_negative(value),
            Reg::Infinity { negative } | Reg::NaN { negative, .. } => *negative,
        }
    }
    /// `FCHS`.
    pub fn negated(self) -> Self {
        match self {
            Reg::Finite(value) => Reg::Finite(value.negated()),
            Reg::Infinity { negative } => Reg::Infinity { negative: !negative },
            Reg::NaN { negative, significand } => Reg::NaN { negative: !negative, significand },
        }
    }
    /// `FABS`.
    pub fn abs(self) -> Self {
        match self {
            Reg::Finite(value) if finite_negative(&value) => Reg::Finite(value.negated()),
            Reg::Finite(value) => Reg::Finite(value),
            Reg::Infinity { .. } => Reg::Infinity { negative: false },
            Reg::NaN { significand, .. } => Reg::NaN { negative: false, significand },
        }
    }
    /// The propagated NaN of a two-operand instruction, if any operand is one.
    fn propagate(&self, other: &Self) -> Option<Self> {
        match (self, other) {
            (
                Reg::NaN { negative: n1, significand: s1 },
                Reg::NaN { negative: n2, significand: s2 },
            ) => Some(match s1.cmp(s2) {
                Ordering::Greater => self.clone(),
                Ordering::Less => other.clone(),
                Ordering::Equal => Reg::NaN { negative: *n1 && *n2, significand: *s1 },
            }),
            (Reg::NaN { .. }, _) => Some(self.clone()),
            (_, Reg::NaN { .. }) => Some(other.clone()),
            _ => None,
        }
    }
    /// `FADD`: `self + other` at precision control `p`.
    pub fn add(&self, other: &Self, p: Precision) -> Result<Self> {
        if let Some(nan) = self.propagate(other) {
            return Ok(nan);
        }
        Ok(match (self, other) {
            (Reg::Infinity { negative: a }, Reg::Infinity { negative: b }) => {
                if a == b { self.clone() } else { Reg::INDEFINITE }
            }
            (Reg::Infinity { .. }, _) => self.clone(),
            (_, Reg::Infinity { .. }) => other.clone(),
            (Reg::Finite(a), Reg::Finite(b)) => Reg::Finite(a.add(b, p).map_err(numeric)?),
            _ => unreachable!("NaN operands handled above"),
        })
    }
    /// `FSUB`: `self - other` at precision control `p`.
    pub fn sub(&self, other: &Self, p: Precision) -> Result<Self> {
        if let Some(nan) = self.propagate(other) {
            return Ok(nan);
        }
        Ok(match (self, other) {
            (Reg::Infinity { negative: a }, Reg::Infinity { negative: b }) => {
                if a != b { self.clone() } else { Reg::INDEFINITE }
            }
            (Reg::Infinity { .. }, _) => self.clone(),
            (_, Reg::Infinity { negative }) => Reg::Infinity { negative: !negative },
            (Reg::Finite(a), Reg::Finite(b)) => Reg::Finite(a.sub(b, p).map_err(numeric)?),
            _ => unreachable!("NaN operands handled above"),
        })
    }
    /// `FMUL`.
    pub fn mul(&self, other: &Self, p: Precision) -> Result<Self> {
        if let Some(nan) = self.propagate(other) {
            return Ok(nan);
        }
        let negative = self.negative() != other.negative();
        Ok(match (self, other) {
            (Reg::Finite(a), Reg::Finite(b)) => Reg::Finite(a.mul(b, p).map_err(numeric)?),
            (Reg::Finite(zero), _) | (_, Reg::Finite(zero)) if zero.is_zero() => Reg::INDEFINITE,
            _ => Reg::Infinity { negative },
        })
    }
    /// `FDIV`: `self / other`.
    pub fn div(&self, other: &Self, p: Precision) -> Result<Self> {
        if let Some(nan) = self.propagate(other) {
            return Ok(nan);
        }
        let negative = self.negative() != other.negative();
        Ok(match (self, other) {
            (Reg::Infinity { .. }, Reg::Infinity { .. }) => Reg::INDEFINITE,
            (Reg::Infinity { .. }, Reg::Finite(_)) => Reg::Infinity { negative },
            (Reg::Finite(_), Reg::Infinity { .. }) => Reg::Finite(signed_zero(negative)),
            (Reg::Finite(a), Reg::Finite(b)) if b.is_zero() => {
                if a.is_zero() { Reg::INDEFINITE } else { Reg::Infinity { negative } }
            }
            (Reg::Finite(a), Reg::Finite(b)) => Reg::Finite(a.div(b, p).map_err(numeric)?),
            _ => unreachable!("NaN operands handled above"),
        })
    }
    /// `FSQRT`.
    pub fn sqrt(&self, p: Precision) -> Result<Self> {
        Ok(match self {
            Reg::NaN { .. } | Reg::Infinity { negative: false } => self.clone(),
            Reg::Infinity { negative: true } => Reg::INDEFINITE,
            Reg::Finite(value) if value.is_negative_nonzero() => Reg::INDEFINITE,
            Reg::Finite(value) => Reg::Finite(value.sqrt(p).map_err(numeric)?),
        })
    }
    /// `FCOM`/`FUCOM` of ST0 = `self` against `other`.
    pub fn relation(&self, other: &Self) -> Relation {
        let order = match (self, other) {
            (Reg::NaN { .. }, _) | (_, Reg::NaN { .. }) => return Relation::Unordered,
            (Reg::Finite(a), Reg::Finite(b)) => a.compare(b),
            (Reg::Infinity { negative: a }, Reg::Infinity { negative: b }) => b.cmp(a),
            (Reg::Infinity { negative }, _) => if *negative { Ordering::Less } else { Ordering::Greater },
            (_, Reg::Infinity { negative }) => if *negative { Ordering::Greater } else { Ordering::Less },
        };
        match order {
            Ordering::Less => Relation::Less,
            Ordering::Equal => Relation::Equal,
            Ordering::Greater => Relation::Greater,
        }
    }
}

#[cfg(test)]
#[path = "x87_masked_ray_tests.rs"]
mod ray_tests;

#[cfg(test)]
mod tests {
    use super::*;
    const P: Precision = Precision::Extended64;
    fn r(bits: u32) -> Reg {
        Reg::load_f32(bits).unwrap()
    }
    fn s(value: Reg) -> u32 {
        value.store_f32().unwrap()
    }
    const INF: u32 = 0x7f80_0000;
    const NINF: u32 = 0xff80_0000;
    const MAX: u32 = 0x7f7f_ffff;

    #[test]
    fn masked_default_results() {
        // Overflowing binary32 store: +-infinity.
        assert_eq!(s(r(MAX).mul(&r(0x4000_0000), P).unwrap()), INF);
        assert_eq!(s(r(MAX | 0x8000_0000).mul(&r(0x4000_0000), P).unwrap()), NINF);
        // Round to nearest keeps FLT_MAX + less than half an ulp finite.
        assert_eq!(s(r(MAX).add(&r(0x3f80_0000), P).unwrap()), MAX);
        // Invalid operations: real indefinite.
        assert_eq!(s(r(INF).sub(&r(INF), P).unwrap()), 0xffc0_0000);
        assert_eq!(s(r(INF).add(&r(NINF), P).unwrap()), 0xffc0_0000);
        assert_eq!(s(r(INF).mul(&r(0), P).unwrap()), 0xffc0_0000);
        assert_eq!(s(r(0x8000_0000).mul(&r(NINF), P).unwrap()), 0xffc0_0000);
        assert_eq!(s(r(0).div(&r(0), P).unwrap()), 0xffc0_0000);
        assert_eq!(s(r(INF).div(&r(NINF), P).unwrap()), 0xffc0_0000);
        assert_eq!(s(r(0xbf80_0000).sqrt(P).unwrap()), 0xffc0_0000);
        // Infinite arithmetic.
        assert_eq!(s(r(INF).add(&r(INF), P).unwrap()), INF);
        assert_eq!(s(r(0x3f80_0000).sub(&r(INF), P).unwrap()), NINF);
        assert_eq!(s(r(NINF).mul(&r(0xc000_0000), P).unwrap()), INF);
        assert_eq!(s(r(0x3f80_0000).div(&r(NINF), P).unwrap()), 0x8000_0000);
        assert_eq!(s(r(0xbf80_0000).div(&r(0), P).unwrap()), NINF);
        assert_eq!(s(r(0x3f80_0000).div(&r(0x8000_0000), P).unwrap()), NINF);
        assert_eq!(s(r(NINF).div(&r(0x8000_0000), P).unwrap()), INF);
        // NaN propagation and FCHS/FABS on NaN.
        assert_eq!(s(r(0xffc0_0000).add(&r(0x3f80_0000), P).unwrap()), 0xffc0_0000);
        assert_eq!(s(r(0x3f80_0000).mul(&r(0x7fc0_0001), P).unwrap()), 0x7fc0_0001);
        assert_eq!(s(r(0xffc0_0000).sub(&r(0x7fc0_0000), P).unwrap()), 0x7fc0_0000);
        assert_eq!(s(r(0xffc0_0000).sub(&r(0xffc0_0001), P).unwrap()), 0xffc0_0001);
        assert_eq!(s(r(0xffc0_0000).negated()), 0x7fc0_0000);
        assert_eq!(s(r(0xffc0_0000).abs()), 0x7fc0_0000);
        assert!(Reg::load_f32(0x7f80_0001).is_err());
    }

    #[test]
    fn relations() {
        assert_eq!(r(0xffc0_0000).relation(&r(0)), Relation::Unordered);
        assert_eq!(r(0).relation(&r(0x7fc0_0000)), Relation::Unordered);
        assert_eq!(r(INF).relation(&r(MAX)), Relation::Greater);
        assert_eq!(r(NINF).relation(&r(MAX | 0x8000_0000)), Relation::Less);
        assert_eq!(r(INF).relation(&r(INF)), Relation::Equal);
        assert_eq!(r(NINF).relation(&r(INF)), Relation::Less);
        assert_eq!(r(0x8000_0000).relation(&r(0)), Relation::Equal);
        assert!(!r(0xffc0_0000).is_zero());
    }
}
