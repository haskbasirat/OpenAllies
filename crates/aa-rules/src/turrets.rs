//! Original ActorData Turret definitions.
//!
//! `KKC_ActorData::LoadData` (464BD0) reads the PerformerComponent fields, then
//! for each authored `Turret` child allocates 0x34 bytes, runs the constructor
//! 4C7660 and `LoadData(node, ActorData)` 4C7820, and appends the pointer to the
//! ActorData vector at +1F8 (count +1FC) BEFORE the Ability list is loaded.
//! Consumers: Performer constructor 57F4D0 reads the count and, through
//! 45DC30, field +4; Ability LoadData 4BF990 binds `turret_ids` through 45DD50
//! (index of the first turret whose Bone pointer matches, else -1).
//!
//! Native data errors go through 410C40, which throws: the actor definition
//! fails to load. They are reported here as [`TurretDataError`]; never as
//! defaults. Bone pointers become canonical Bone registry identities.
use crate::{
    actors::{ActorCatalog, ResolvedNode},
    native::{Error, Result, TypedValue},
};
use aa_numeric::{Binary, Precision};
use serde::Serialize;
use std::cmp::Ordering;

/// 9FC064: binary32 degrees-to-radians multiplier.
pub const DEGREES_TO_RADIANS_BITS: u32 = 0x3c8e_fa35;
/// 9FBDB8: 0.5, applied to the yaw range before the radians multiply.
pub const HALF_BITS: u32 = 0x3f00_0000;
/// 9F32DC (0), A2508C (360), A24D8C (-90), A24D88 (90).
pub const ZERO_BITS: u32 = 0;
pub const FULL_TURN_DEGREES_BITS: u32 = 0x43b4_0000;
pub const PITCH_MIN_DEGREES_BITS: u32 = 0xc2b4_0000;
pub const PITCH_MAX_DEGREES_BITS: u32 = 0x42b4_0000;

/// The 0x34-byte native definition. Every float is the binary32 store the
/// original performs; the Bone pointer at +0 is a canonical registry identity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TurretDefinition {
    /// +0 Bone registry entry named by `turret_ids` (42C070 on b36530+580).
    pub bone: BoneIdentity,
    /// +4 yaw_start_degrees * 9FC064 (read by 45DC30).
    pub yaw_start_bits: u32,
    /// +8 yaw_rotation_per_sec_degrees * 9FC064.
    pub yaw_rate_bits: u32,
    /// +C yaw_rotation_range_degrees * 0.5 * 9FC064.
    pub yaw_half_range_bits: u32,
    /// +10 FCOS of the unrounded +C product.
    pub yaw_half_range_cos_bits: u32,
    /// +14 pitch_start_degrees * 9FC064.
    pub pitch_start_bits: u32,
    /// +18 pitch_rotation_per_sec_degrees * 9FC064.
    pub pitch_rate_bits: u32,
    /// +1C pitch_rotation_range_start_degrees * 9FC064.
    pub pitch_range_start_bits: u32,
    /// +20 pitch_rotation_range_end_degrees * 9FC064.
    pub pitch_range_end_bits: u32,
    /// +24/+28 raw `offset` point2 words (integer moves, no x87).
    pub offset_bits: [u32; 2],
    /// +2C `reset_time` (fstp of the getter result).
    pub reset_time_bits: u32,
    /// +30 `can_reset`.
    pub can_reset: bool,
    /// +31 `can_preaim`.
    pub can_preaim: bool,
}
/// Canonical Bone registry identity. Equality stands for native pointer
/// equality of the registered Bone definition.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BoneIdentity {
    pub id: String,
}
/// Thrown native data errors (format strings at the cited addresses).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, thiserror::Error)]
pub enum TurretDataError {
    #[error("Turret bone unrecognized (A25120)")]
    BoneUnrecognized,
    #[error("Duplicate turret (A250D0)")]
    DuplicateBone,
    #[error("yaw_rotation_per_sec_degrees outside 0..360 (A24FD0)")]
    YawRateRange,
    #[error("pitch_rotation_per_sec_degrees outside 0..360 (A24EC8)")]
    PitchRateRange,
    #[error("yaw_rotation_range_degrees outside 0..360 (A24DD8)")]
    YawRangeRange,
    #[error("pitch_rotation_range_start_degrees outside -90..90 (A24CC0)")]
    PitchRangeStartRange,
    #[error("pitch_rotation_range_end_degrees outside -90..90 (A24BB0)")]
    PitchRangeEndRange,
    #[error("pitch_rotation_range_end_degrees before start (A24AA8)")]
    PitchRangeEndBeforeStart,
    #[error("yaw_start_degrees outside 0..360 (A249E0)")]
    YawStartRange,
    #[error("pitch_start_degrees outside pitch range (A24880)")]
    PitchStartRange,
}
#[derive(Debug, thiserror::Error)]
pub enum TurretLoadError {
    #[error("native Turret data error: {0}")]
    Data(TurretDataError),
    #[error(transparent)]
    Source(#[from] Error),
}
impl From<TurretLoadError> for Error {
    fn from(value: TurretLoadError) -> Self {
        match value {
            TurretLoadError::Source(e) => e,
            TurretLoadError::Data(e) => Error::Unsupported(format!("native Turret data error: {e}")),
        }
    }
}
impl From<aa_numeric::Error> for TurretLoadError {
    fn from(value: aa_numeric::Error) -> Self {
        Self::Source(Error::Numeric(value))
    }
}

/// Typed TGI getters in native call order. A missing field throws natively
/// ("Node doesn't contain field."); implementations return an error.
pub trait TurretSource {
    fn string(&mut self, name: &str) -> Result<String>;
    fn fixed(&mut self, name: &str) -> Result<u32>;
    fn flag(&mut self, name: &str) -> Result<bool>;
    fn point2(&mut self, name: &str) -> Result<[u32; 2]>;
    /// 42C070 on the Bone registry: `None` for an empty or unregistered IDS.
    fn bone(&mut self, ids: &str) -> Result<Option<BoneIdentity>>;
}

fn is_nan(bits: u32) -> bool {
    bits & 0x7f80_0000 == 0x7f80_0000 && bits & 0x007f_ffff != 0
}
/// x87 FCOMP of two binary32 values; `None` is unordered.
fn compare(a: u32, b: u32) -> Option<Ordering> {
    if is_nan(a) || is_nan(b) {
        return None;
    }
    let key = |v: u32| -> i64 {
        let magnitude = i64::from(v & 0x7fff_ffff);
        if v >> 31 != 0 { -magnitude } else { magnitude }
    };
    Some(key(a).cmp(&key(b)))
}
/// `fcomp low; test ah,5; jnp error` then `fcomp high; test ah,41h; jne ok`:
/// only an ordered value below `low` or above `high` fails. Unordered passes.
fn outside(value: u32, low: u32, high: u32) -> bool {
    compare(value, low) == Some(Ordering::Less) || compare(value, high) == Some(Ordering::Greater)
}
fn finite_input(name: &str, bits: u32) -> std::result::Result<Binary, TurretLoadError> {
    if is_nan(bits) {
        // Native validation lets NaN through and propagates it; TGI fixed text
        // cannot produce NaN and x87 FCOS NaN payloads are not proven here.
        return Err(Error::Unsupported(format!(
            "Turret field {name} is NaN; native NaN propagation is not ported"
        ))
        .into());
    }
    Ok(Binary::from_f32_bits(bits)?)
}
fn radians(name: &str, bits: u32, pc: Precision) -> std::result::Result<u32, TurretLoadError> {
    Ok(finite_input(name, bits)?
        .mul(&Binary::from_f32_bits(DEGREES_TO_RADIANS_BITS)?, pc)?
        .store_f32()?)
}

/// Original 4C7660 followed by 4C7820. `existing` is the ActorData +1F8
/// vector so far (the new turret is appended by the caller after success).
pub fn load_turret(
    source: &mut impl TurretSource,
    existing: &[TurretDefinition],
    pc: Precision,
) -> std::result::Result<TurretDefinition, TurretLoadError> {
    use TurretDataError as E;
    let ids = source.string("turret_ids")?;
    let bone = source
        .bone(&ids)?
        .ok_or(TurretLoadError::Data(E::BoneUnrecognized))?;
    if existing.iter().any(|t| t.bone == bone) {
        return Err(TurretLoadError::Data(E::DuplicateBone));
    }
    let full = FULL_TURN_DEGREES_BITS;
    let yaw_rate = source.fixed("yaw_rotation_per_sec_degrees")?;
    if outside(yaw_rate, ZERO_BITS, full) {
        return Err(TurretLoadError::Data(E::YawRateRange));
    }
    let yaw_rate_bits = radians("yaw_rotation_per_sec_degrees", yaw_rate, pc)?;
    let pitch_rate = source.fixed("pitch_rotation_per_sec_degrees")?;
    if outside(pitch_rate, ZERO_BITS, full) {
        return Err(TurretLoadError::Data(E::PitchRateRange));
    }
    let pitch_rate_bits = radians("pitch_rotation_per_sec_degrees", pitch_rate, pc)?;
    let yaw_range = source.fixed("yaw_rotation_range_degrees")?;
    if outside(yaw_range, ZERO_BITS, full) {
        return Err(TurretLoadError::Data(E::YawRangeRange));
    }
    // 4C79DD..4C79FA: fmul 0.5; fmul 9FC064; fst [+C]; fcos; fstp [+10].
    let half_range = finite_input("yaw_rotation_range_degrees", yaw_range)?
        .mul(&Binary::from_f32_bits(HALF_BITS)?, pc)?
        .mul(&Binary::from_f32_bits(DEGREES_TO_RADIANS_BITS)?, pc)?;
    let yaw_half_range_bits = half_range.store_f32()?;
    let yaw_half_range_cos_bits = fcos_binary32(yaw_range, pc, &half_range)?;
    let pitch_start_range = source.fixed("pitch_rotation_range_start_degrees")?;
    if outside(pitch_start_range, PITCH_MIN_DEGREES_BITS, PITCH_MAX_DEGREES_BITS) {
        return Err(TurretLoadError::Data(E::PitchRangeStartRange));
    }
    let pitch_range_start_bits =
        radians("pitch_rotation_range_start_degrees", pitch_start_range, pc)?;
    let pitch_end_range = source.fixed("pitch_rotation_range_end_degrees")?;
    if outside(pitch_end_range, PITCH_MIN_DEGREES_BITS, PITCH_MAX_DEGREES_BITS) {
        return Err(TurretLoadError::Data(E::PitchRangeEndRange));
    }
    // 4C7A98: fld end; fcomp start; test ah,5; jp ok.
    if compare(pitch_end_range, pitch_start_range) == Some(Ordering::Less) {
        return Err(TurretLoadError::Data(E::PitchRangeEndBeforeStart));
    }
    let pitch_range_end_bits = radians("pitch_rotation_range_end_degrees", pitch_end_range, pc)?;
    let yaw_start = source.fixed("yaw_start_degrees")?;
    if outside(yaw_start, ZERO_BITS, full) {
        return Err(TurretLoadError::Data(E::YawStartRange));
    }
    let yaw_start_bits = radians("yaw_start_degrees", yaw_start, pc)?;
    let pitch_start = source.fixed("pitch_start_degrees")?;
    if outside(pitch_start, pitch_start_range, pitch_end_range) {
        return Err(TurretLoadError::Data(E::PitchStartRange));
    }
    let pitch_start_bits = radians("pitch_start_degrees", pitch_start, pc)?;
    let offset_bits = source.point2("offset")?;
    let reset_time_bits = source.fixed("reset_time")?;
    if is_nan(reset_time_bits) {
        // FSTP would quiet a signaling NaN; text cannot produce NaN.
        return Err(Error::Unsupported("Turret reset_time is NaN".into()).into());
    }
    let can_reset = source.flag("can_reset")?;
    let can_preaim = source.flag("can_preaim")?;
    Ok(TurretDefinition {
        bone,
        yaw_start_bits,
        yaw_rate_bits,
        yaw_half_range_bits,
        yaw_half_range_cos_bits,
        pitch_start_bits,
        pitch_rate_bits,
        pitch_range_start_bits,
        pitch_range_end_bits,
        offset_bits,
        reset_time_bits,
        can_reset,
        can_preaim,
    })
}

/// 45DD50: index of the first turret whose Bone matches, else -1.
pub fn turret_index(turrets: &[TurretDefinition], bone: Option<&BoneIdentity>) -> i32 {
    bone.and_then(|bone| turrets.iter().position(|t| &t.bone == bone))
        .map_or(-1, |i| i as i32)
}

// ---------------------------------------------------------------------------
// x87 FCOS of the unrounded half-range product, stored to binary32.
//
// The x87 FCOS result carries a 64-bit significand regardless of precision
// control; FSTP then rounds to binary32. This evaluates cos exactly in
// Q8.120 fixed point (truncation error < 2^-113) and rounds to binary32. The
// hardware error bound used for the refusal window is 2^-60 relative plus
// 2^-60 absolute (covers the 66-bit pi reduction near pi/2). Inside that
// window the result is reported as unsupported instead of guessed.

const FRACTION: u32 = 120;

fn mul_q(a: u128, b: u128) -> u128 {
    // (a*b) >> FRACTION with a 256-bit intermediate, truncating. Callers keep
    // every product below 2^(128+FRACTION).
    let (a1, a0) = (a >> 64, a & u128::from(u64::MAX));
    let (b1, b0) = (b >> 64, b & u128::from(u64::MAX));
    let p00 = a0 * b0;
    let p01 = a0 * b1;
    let p10 = a1 * b0;
    let p11 = a1 * b1;
    let middle = (p00 >> 64) + (p01 & u128::from(u64::MAX)) + (p10 & u128::from(u64::MAX));
    let low = (p00 & u128::from(u64::MAX)) | ((middle & u128::from(u64::MAX)) << 64);
    let high = p11 + (p01 >> 64) + (p10 >> 64) + (middle >> 64);
    (high << (128 - FRACTION)) | (low >> FRACTION)
}

/// Exact product `f * 0.5 * DEG` as (significand, power-of-two exponent),
/// rounded to the x87 precision-control width exactly like FMUL.
fn half_range_product(yaw_range: u32, pc: Precision) -> Option<(u64, i32)> {
    let field = (yaw_range >> 23) & 0xff;
    let fraction = u64::from(yaw_range & 0x7f_ffff);
    let (m, e) = if field == 0 {
        (fraction, -149)
    } else {
        (fraction | 1 << 23, field as i32 - 150)
    };
    if m == 0 {
        return Some((0, 0));
    }
    let degrees = u64::from(DEGREES_TO_RADIANS_BITS & 0x7f_ffff | 1 << 23);
    let mut significand = m * degrees;
    let mut exponent = e - 1 + (((DEGREES_TO_RADIANS_BITS >> 23) & 0xff) as i32 - 150);
    let width = 64 - significand.leading_zeros();
    let wanted = pc.significand_bits() as u32;
    if width > wanted {
        let shift = width - wanted;
        let quotient = significand >> shift;
        let remainder = significand & ((1 << shift) - 1);
        let half = 1 << (shift - 1);
        significand = if remainder > half || (remainder == half && quotient & 1 == 1) {
            quotient + 1
        } else {
            quotient
        };
        exponent += shift as i32;
    }
    Some((significand, exponent))
}

fn fcos_binary32(
    yaw_range: u32,
    pc: Precision,
    checked: &Binary,
) -> std::result::Result<u32, TurretLoadError> {
    let (significand, exponent) = half_range_product(yaw_range, pc)
        .ok_or_else(|| Error::Unsupported("Turret FCOS input".into()))?;
    // Cross-check the integer product against the shared x87 arithmetic.
    let rebuilt = Binary::from_u32((significand >> 32) as u32)
        .mul(&Binary::from_f64_bits(0x41f0_0000_0000_0000)?, Precision::Extended64)?
        .add(&Binary::from_u32(significand as u32), Precision::Extended64)?
        .mul(
            &Binary::from_f64_bits(((i64::from(exponent) + 1023) as u64) << 52)?,
            Precision::Extended64,
        )?;
    if significand != 0 && rebuilt.compare(checked) != Ordering::Equal {
        return Err(Error::Unsupported("Turret FCOS input reconstruction mismatch".into()).into());
    }
    let shift = i64::from(FRACTION) + i64::from(exponent);
    let x: u128 = if significand == 0 {
        0
    } else if shift >= 0 {
        if i64::from(64 - significand.leading_zeros()) + shift > 123 {
            return Err(Error::Unsupported("Turret FCOS input out of range".into()).into());
        }
        u128::from(significand) << shift
    } else if -shift >= 128 {
        0
    } else {
        u128::from(significand) >> (-shift)
    };
    // Validation bounds |x| <= 180 * DEG < 4.
    if x >= 4u128 << FRACTION {
        return Err(Error::Unsupported("Turret FCOS input exceeds pi".into()).into());
    }
    let square = mul_q(x, x);
    let one = 1u128 << FRACTION;
    let mut positive = one;
    let mut negative = 0u128;
    let mut term = one;
    let mut k = 1u128;
    loop {
        term = mul_q(term, square) / ((2 * k - 1) * (2 * k));
        if term == 0 {
            break;
        }
        if k % 2 == 1 {
            negative += term;
        } else {
            positive += term;
        }
        k += 1;
    }
    let (sign, magnitude) = if positive >= negative {
        (0u32, positive - negative)
    } else {
        (1u32, negative - positive)
    };
    if magnitude == 0 {
        return Err(Error::Unsupported("Turret FCOS result underflow".into()).into());
    }
    let top = 127 - magnitude.leading_zeros(); // msb index
    if top < 23 + 8 {
        return Err(Error::Unsupported("Turret FCOS result too small".into()).into());
    }
    let ulp_shift = top - 23;
    let quotient = magnitude >> ulp_shift;
    let remainder = magnitude & ((1u128 << ulp_shift) - 1);
    let half = 1u128 << (ulp_shift - 1);
    let distance = remainder.abs_diff(half);
    let window = (magnitude >> 60) + (one >> 60) + 64;
    if distance <= window {
        return Err(Error::Unsupported(
            "Turret FCOS result lies inside the hardware binary32 rounding ambiguity window".into(),
        )
        .into());
    }
    let mut q = quotient + u128::from(remainder > half);
    let mut power = top as i32 - FRACTION as i32;
    if q >> 24 != 0 {
        q >>= 1;
        power += 1;
    }
    if !(-126..=127).contains(&power) {
        return Err(Error::Unsupported("Turret FCOS result outside binary32 normals".into()).into());
    }
    Ok(sign << 31 | ((power + 127) as u32) << 23 | (q as u32 & 0x7f_ffff))
}

struct NodeSource<'a, 'n> {
    catalog: &'a ActorCatalog,
    node: &'n ResolvedNode<'a>,
    pc: Precision,
}
impl NodeSource<'_, '_> {
    fn value(&self, name: &str) -> Result<TypedValue> {
        self.node.field(name, self.pc)?.ok_or_else(|| {
            Error::Unsupported(format!("Turret field {name} missing (native throws)"))
        })
    }
}
impl TurretSource for NodeSource<'_, '_> {
    fn string(&mut self, name: &str) -> Result<String> {
        match self.value(name)? {
            TypedValue::String { value, .. } => Ok(value),
            _ => Err(Error::Unsupported(format!("Turret {name} must be string"))),
        }
    }
    fn fixed(&mut self, name: &str) -> Result<u32> {
        self.value(name)?.fixed_bits()
    }
    fn flag(&mut self, name: &str) -> Result<bool> {
        self.value(name)?.boolean()
    }
    fn point2(&mut self, name: &str) -> Result<[u32; 2]> {
        match self.value(name)? {
            TypedValue::Point { bits, .. } if bits.len() == 2 => Ok([bits[0], bits[1]]),
            _ => Err(Error::Unsupported(format!("Turret {name} must be point2"))),
        }
    }
    fn bone(&mut self, ids: &str) -> Result<Option<BoneIdentity>> {
        self.catalog.native_bone(ids, self.pc)
    }
}

impl ActorCatalog {
    /// 42C070 on the Bone registry (b36530+580, registered from `[Bone]`
    /// roots). Empty or unregistered IDS returns null. Case-insensitive like
    /// the shipped data requires (`turret_ids = Turret` vs `IDS = turret`).
    pub fn native_bone(&self, ids: &str, pc: Precision) -> Result<Option<BoneIdentity>> {
        if ids.is_empty() {
            return Ok(None);
        }
        let mut found = None;
        for node in self.resolved_definitions("Bone")? {
            if let Some(TypedValue::String { value, .. }) = node.field("IDS", pc)?
                && value.eq_ignore_ascii_case(ids)
            {
                if found.is_some() {
                    return Err(Error::Unsupported(format!(
                        "Bone IDS {ids} has duplicate registrations; native overwrite order unproven"
                    )));
                }
                found = Some(BoneIdentity { id: value });
            }
        }
        Ok(found)
    }
    /// 464BD0 Turret list: PerformerComponent `Turret` children in authored
    /// order, each through 4C7660/4C7820 against the turrets loaded so far.
    pub fn native_turrets(&self, id: &str, pc: Precision) -> Result<Vec<TurretDefinition>> {
        let actor = self.resolved_actor(id)?;
        let Some(performer) = actor.child("PerformerComponent")? else {
            return Ok(Vec::new());
        };
        let mut turrets: Vec<TurretDefinition> = Vec::new();
        for node in performer.children("Turret")? {
            let mut source = NodeSource {
                catalog: self,
                node: &node,
                pc,
            };
            let turret = load_turret(&mut source, &turrets, pc).map_err(|e| match e {
                TurretLoadError::Data(d) => {
                    Error::Unsupported(format!("Turret of actor {id}: native data error: {d}"))
                }
                TurretLoadError::Source(e) => e,
            })?;
            turrets.push(turret);
        }
        Ok(turrets)
    }
}
