//! Original MobileDeployment contributor 575FB0, predicate 575C00 and
//! state gate 575B70. Only action bit 34 (actor+F4 bit 2) is changed.
//! 4D0230(false) uses Body+10 / Body+14 at the explicit x87 precision;
//! absent Body returns +0.0 (9F32DC), not a fabricated full-health ratio.
use crate::{
    ActorId,
    ability_flags::{ActorFields, Error, Result, set_action},
    state,
};
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BodyHealth {
    /// Body+10 and Body+14, raw binary32 words.
    pub health_bits: u32,
    pub maximum_bits: u32,
}

/// Required live arena reads. None means a genuinely absent component;
/// unavailable payloads must return Err, never a fabricated default.
pub trait World {
    fn fields(&self, actor: ActorId) -> Result<ActorFields>;
    fn set_action_bits(&mut self, actor: ActorId, bits: u64) -> Result<()>;
    /// ActorData+548 non-null deployment definition reference.
    fn has_pack_reference(&self, actor: ActorId) -> Result<bool>;
    /// ActorData+534 unpack_to_actor reference (575BB0).
    fn has_unpack_reference(&self, actor: ActorId) -> Result<bool>;
    /// ActorData+54C, read only after the reference is non-null.
    fn pack_health_min_bits(&self, actor: ActorId) -> Result<u32>;
    /// Properties component actor+78, flags+1C (4CF110).
    fn properties_flags(&self, actor: ActorId) -> Result<Option<u32>>;
    /// Body component actor+60, 4D0230(false).
    fn body_health(&self, actor: ActorId) -> Result<Option<BodyHealth>>;
    /// actor+70 is the Mind component pointer.
    fn has_mind(&self, actor: ActorId) -> Result<bool>;
}

enum Number {
    Finite(Binary),
    Infinite(bool),
    Unordered,
}
fn number(bits: u32) -> Result<Number> {
    if bits & 0x7f80_0000 == 0x7f80_0000 {
        return Ok(if bits & 0x007f_ffff != 0 {
            Number::Unordered
        } else {
            Number::Infinite(bits >> 31 != 0)
        });
    }
    Binary::from_f32_bits(bits)
        .map(Number::Finite)
        .map_err(|e| Error::World(e.to_string()))
}
fn compare(a: &Number, b: &Number) -> Option<Ordering> {
    use Number::*;
    match (a, b) {
        (Unordered, _) | (_, Unordered) => None,
        (Finite(a), Finite(b)) => Some(a.compare(b)),
        (Infinite(a), Infinite(b)) => Some(b.cmp(a)),
        (Infinite(true), _) | (_, Infinite(false)) => Some(Ordering::Less),
        (Infinite(false), _) | (_, Infinite(true)) => Some(Ordering::Greater),
    }
}
fn health_ratio(body: Option<BodyHealth>, precision: Precision) -> Result<Number> {
    let Some(body) = body else { return number(0) };
    let numerator = number(body.health_bits)?;
    let denominator = number(body.maximum_bits)?;
    let negative = (body.health_bits ^ body.maximum_bits) >> 31 != 0;
    use Number::*;
    match (&numerator, &denominator) {
        (Unordered, _) | (_, Unordered) | (Infinite(_), Infinite(_)) => Ok(Unordered),
        (Infinite(_), _) => Ok(Infinite(negative)),
        (_, Infinite(_)) => number(if negative { 0x8000_0000 } else { 0 }),
        (Finite(a), Finite(b)) if b.is_zero() => Ok(if a.is_zero() {
            Unordered
        } else {
            Infinite(negative)
        }),
        (Finite(a), Finite(b)) => a
            .div(b, precision)
            .map(Finite)
            .map_err(|e| Error::World(e.to_string())),
    }
}
fn group13(bits: u64) -> Result<Option<u8>> {
    state::group(bits, 13)
        .map(|v| v.map(|d| d.index))
        .map_err(|e| Error::World(e.to_string()))
}

/// 575B70: group13 must be absent or NORMAL(27); construction bit21 clear.
pub fn state_permits(world: &impl World, actor: ActorId) -> Result<bool> {
    if !matches!(group13(world.fields(actor)?.state_bits)?, None | Some(27)) {
        return Ok(false);
    }
    Ok(world.fields(actor)?.state_bits & (1 << 21) == 0)
}

/// 575BB0: unpack target must be non-null, then optionally apply the same
/// group13/bit21 gate as 575B70. No health, Properties or Mind test occurs.
pub fn can_unpack(world: &impl World, actor: ActorId, check_state: bool) -> Result<bool> {
    Ok(world.has_unpack_reference(actor)? && (!check_state || state_permits(world, actor)?))
}

/// 575C00. TEST AH,1 accepts threshold < 1.0 OR unordered. The final
/// TEST AH,41 / JP accepts ratio > threshold OR unordered, not equality.
/// NaN payload/exception flags are not exposed; these are the exact masked
/// x87 branch outcomes, including zero-divisor and infinity cases.
pub fn can_pack(
    world: &impl World,
    actor: ActorId,
    check_state_and_health: bool,
    precision: Precision,
) -> Result<bool> {
    if !world.has_pack_reference(actor)? {
        return Ok(false);
    }
    let threshold = number(world.pack_health_min_bits(actor)?)?;
    if !matches!(
        compare(&threshold, &number(0x3f80_0000)?),
        None | Some(Ordering::Less)
    ) {
        return Ok(false);
    }
    if world
        .properties_flags(actor)?
        .is_some_and(|flags| flags & 0x800 != 0)
    {
        return Ok(false);
    }
    if !check_state_and_health {
        return Ok(true);
    }
    if !state_permits(world, actor)? {
        return Ok(false);
    }
    let threshold = number(world.pack_health_min_bits(actor)?)?;
    let ratio = health_ratio(world.body_health(actor)?, precision)?;
    Ok(matches!(
        compare(&ratio, &threshold),
        None | Some(Ordering::Greater)
    ))
}

/// 575FB0. The first group13 lookup is performed even though its return is
/// discarded. Mind absence is tested only after 575C00(true) succeeds.
pub fn contribute(world: &mut impl World, actor: ActorId, precision: Precision) -> Result<()> {
    let _ = group13(world.fields(actor)?.state_bits)?;
    let enabled = can_pack(world, actor, true, precision)? && !world.has_mind(actor)?;
    let mut bits = world.fields(actor)?.action_bits;
    set_action(&mut bits, 34, enabled)?;
    world.set_action_bits(actor, bits)
}
