//! Original VC7.1 CRT wide-printf conversions used by trigger command strings
//! (CString::Format -> _vswprintf 96C7EC).  The command text is parsed back by
//! the UI command interpreter, so its digits are gameplay-visible.
//!
//! `%f` behaviour reproduced: the binary64 value is first rounded to 17
//! significant decimal digits (round-half-even on the exact value), then the
//! requested number of decimals is taken with round-half-up; a negative value
//! keeps its '-' even when it prints as zero.  research/phase8-triggers/
//! native-format.json executes the ORIGINAL _vswprintf on 15,716 inputs; this
//! model matches every case with |v| < 1e13.  The original's 17-digit stage is
//! not exact for larger values with fractions, so those (and non-finite
//! values) are explicit errors instead of guessed text.
use crate::manager::{Error, Result};

/// Little-endian base-1e9 big unsigned integer.
#[derive(Clone)]
struct Big(Vec<u32>);
impl Big {
    fn from_u64(mut v: u64) -> Self {
        let mut limbs = vec![];
        while v != 0 {
            limbs.push((v % 1_000_000_000) as u32);
            v /= 1_000_000_000;
        }
        Self(limbs)
    }
    fn mul_small(&mut self, m: u32) {
        let mut carry = 0u64;
        for limb in &mut self.0 {
            let v = u64::from(*limb) * u64::from(m) + carry;
            *limb = (v % 1_000_000_000) as u32;
            carry = v / 1_000_000_000;
        }
        while carry != 0 {
            self.0.push((carry % 1_000_000_000) as u32);
            carry /= 1_000_000_000;
        }
    }
    fn digits(&self) -> Vec<u8> {
        let mut out = vec![];
        for (i, limb) in self.0.iter().enumerate().rev() {
            let s = if i + 1 == self.0.len() {
                limb.to_string()
            } else {
                format!("{limb:09}")
            };
            out.extend(s.bytes().map(|b| b - b'0'));
        }
        if out.is_empty() {
            out.push(0);
        }
        out
    }
}

/// Exact decimal expansion: (digits, position of the decimal point from the left).
fn exact(bits: u64) -> (Vec<u8>, i64) {
    let exponent = ((bits >> 52) & 0x7ff) as i64;
    let fraction = bits & 0x000f_ffff_ffff_ffff;
    let (mantissa, e2) = if exponent == 0 {
        (fraction, -1074)
    } else {
        (fraction | (1 << 52), exponent - 1075)
    };
    let mut n = Big::from_u64(mantissa);
    if e2 >= 0 {
        let mut left = e2;
        while left >= 29 {
            n.mul_small(1 << 29);
            left -= 29;
        }
        n.mul_small(1 << left);
        let d = n.digits();
        let point = d.len() as i64;
        (d, point)
    } else {
        // m * 2^e2 = m * 5^-e2 / 10^-e2
        let mut left = -e2;
        while left >= 13 {
            n.mul_small(1_220_703_125);
            left -= 13;
        }
        n.mul_small(5u32.pow(left as u32));
        let d = n.digits();
        let point = d.len() as i64 + e2;
        (d, point)
    }
}

/// 17 significant digits, round-half-even on the exact expansion:
/// returns (S, X) with value ~= S * 10^X and S < 10^17.
fn seventeen(digits: &[u8], point: i64) -> (u64, i64) {
    let lead = digits.iter().take_while(|d| **d == 0).count();
    let digits = &digits[lead..];
    let point = point - lead as i64;
    let mut s: u64 = 0;
    for i in 0..17 {
        s = s * 10 + u64::from(digits.get(i).copied().unwrap_or(0));
    }
    let x = point - 17;
    if digits.len() > 17 {
        let first = digits[17];
        let rest = digits[18..].iter().any(|d| *d != 0);
        let up = first > 5 || (first == 5 && (rest || s % 2 == 1));
        if up {
            s += 1;
            if s == 100_000_000_000_000_000 {
                return (s / 10, x + 1);
            }
        }
    }
    (s, x)
}

/// `%.{decimals}f` of a binary64 value, as the original VC7.1 CRT prints it.
pub fn format_f(bits: u64, decimals: usize) -> Result<String> {
    let negative = bits >> 63 != 0;
    let magnitude = bits & !(1u64 << 63);
    if (magnitude >> 52) == 0x7ff {
        return Err(Error::State("non-finite %f argument is outside the proven CRT domain".into()));
    }
    if f64::from_bits(magnitude) >= 1e13 || decimals > 9 {
        return Err(Error::State(
            "%f argument >= 1e13 is outside the proven VC7.1 CRT domain".into(),
        ));
    }
    let scaled: u128 = if magnitude == 0 {
        0
    } else {
        let (digits, point) = exact(magnitude);
        let (s, x) = seventeen(&digits, point);
        let k = x + decimals as i64;
        if k >= 0 {
            u128::from(s) * 10u128.pow(k as u32)
        } else if -k > 30 {
            0
        } else {
            let d = 10u128.pow((-k) as u32);
            let q = u128::from(s) / d;
            let r = u128::from(s) % d;
            if 2 * r >= d { q + 1 } else { q }
        }
    };
    let unit = 10u128.pow(decimals as u32);
    let mut text = String::new();
    if negative {
        text.push('-');
    }
    text.push_str(&(scaled / unit).to_string());
    if decimals > 0 {
        text.push('.');
        text.push_str(&format!("{:0width$}", scaled % unit, width = decimals));
    }
    Ok(text)
}

/// `%d`.
pub fn format_d(value: i32) -> String {
    value.to_string()
}

/// Exact sum of two binary32 values rounded to the x87 precision control,
/// then stored as binary64 (`fld a; fadd b; fstp qword`).
pub fn x87_add_f32_to_f64(a: u32, b: u32, precision: aa_numeric::Precision) -> Result<u64> {
    let fin = |v: u32| (v >> 23) & 0xff != 0xff;
    if !fin(a) || !fin(b) {
        return Err(Error::State("non-finite x87 operand outside the proven domain".into()));
    }
    let parts = |v: u32| -> (bool, i128, i32) {
        let e = ((v >> 23) & 0xff) as i32;
        let f = i128::from(v & 0x7f_ffff);
        if e == 0 {
            (v >> 31 != 0, f, -149)
        } else {
            (v >> 31 != 0, f | (1 << 23), e - 150)
        }
    };
    let (sa, ma, ea) = parts(a);
    let (sb, mb, eb) = parts(b);
    let e = ea.min(eb);
    // Exponent span of binary32 is < 300 bits; use a sticky reduction when the
    // alignment would overflow i128.
    let span = ea.max(eb) - e;
    let (va, vb, e) = if span <= 100 {
        let va = ma << (ea - e);
        let vb = mb << (eb - e);
        (if sa { -va } else { va }, if sb { -vb } else { vb }, e)
    } else {
        // The smaller operand only contributes a sticky bit below 64+ bits.
        let (big_s, big_m, big_e, small_s, small_m) = if ea > eb {
            (sa, ma, ea, sb, mb)
        } else {
            (sb, mb, eb, sa, ma)
        };
        let shift = 90;
        let big = big_m << shift;
        let small = if small_m != 0 { 1 } else { 0 };
        let vb = if small_s { -small } else { small };
        (if big_s { -big } else { big }, vb, big_e - shift)
    };
    let sum = va + vb;
    if sum == 0 {
        // x87 round-to-nearest: exact zero sum of opposite signs is +0.
        let sign = if sa && sb { 1u64 << 63 } else { 0 };
        return Ok(sign);
    }
    let negative = sum < 0;
    let mag = sum.unsigned_abs();
    let round = |mag: u128, e: i32, bits: u32| -> (u128, i32) {
        let len = 128 - mag.leading_zeros();
        if len <= bits {
            return (mag, e);
        }
        let drop = len - bits;
        let q = mag >> drop;
        let rem = mag & ((1u128 << drop) - 1);
        let half = 1u128 << (drop - 1);
        let q = if rem > half || (rem == half && q & 1 == 1) { q + 1 } else { q };
        if 128 - q.leading_zeros() > bits {
            (q >> 1, e + drop as i32 + 1)
        } else {
            (q, e + drop as i32)
        }
    };
    let pc = precision.significand_bits() as u32;
    let (m, e) = round(mag, e, pc);
    let (m, e) = round(m, e, 53);
    let len = 128 - m.leading_zeros();
    let exponent = e + len as i32 - 1;
    if !(-1022..=1023).contains(&exponent) {
        return Err(Error::State("x87 sum outside the binary64 normal range".into()));
    }
    let fraction = ((m << (53 - len)) as u64) & 0x000f_ffff_ffff_ffff;
    Ok((u64::from(negative) << 63) | (((exponent + 1023) as u64) << 52) | fraction)
}
