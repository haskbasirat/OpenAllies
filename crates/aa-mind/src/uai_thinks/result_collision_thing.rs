//! Original `KKC_UAIStateMove::ResultCollisionThing` (`536840`, UAI.cpp):
//! the element move state (Mind+7C) met a thing on its path and decides
//! whether the element goes after it.
//!
//! Order of the original:
//! 1. A null thing, or one whose virtual +B4 (actor category) is false, is
//!    not taken. The Mind actor's virtual +16C(thing) must accept it, and
//!    `4D4FF0(thing, 1, 0)` on the actor (relation) must answer 0.
//! 2. Engage state (Mind+12C) active (status Mind+138 == 2) while the kill
//!    state (Mind+F8, status Mind+104) is not: ENGAGE. `5358A0(thing)` runs
//!    on the engage state, then the Mind Updatable (Mind+8) virtual
//!    +4(0, 0) schedules the next Think; the result is true.
//! 3. Kill state not active: not taken. Kill state active: the kill target
//!    (Mind+108 GTID, `432E40`) is compared with the thing.
//!    * A resolved target nearer than the thing is kept: the squared
//!      distance `45D880` from the actor to the target is stored as binary32
//!      and the unstored squared distance to the thing must be below it.
//!    * Two decision records initialised by `52DE10`
//!      ([`KillDecision::initial`]) are filled by `5340C0(target, null,
//!      &record, 0)` for the target (null when it did not resolve) and then
//!      for the thing.
//!    * Target record ability null: KILL when the thing record has one.
//!      Thing record ability null: not taken. Different priorities (+18):
//!      KILL when the target's is lower or equal (signed). Equal priorities:
//!      scores (+C) further apart than `[A34770]` (1e-4): KILL unless the
//!      target's score is greater. Scores within 1e-4: squared distances
//!      (+14) within 1e-4 (`455000`) are not taken, otherwise KILL unless
//!      the target's is smaller.
//! 4. KILL: `5351C0(0, thing)` on the kill state, then Mind+8 virtual
//!    +4(0, 0); the result is true.
//!
//! The diagnostic logs `404F60`/`404F90` (UAI.cpp lines 0x618 and 0x635)
//! only read state and are omitted. All x87 arithmetic follows the original
//! instruction sequence at the precision `p`; a non-finite binary32 operand
//! is an error (the original's unordered compares are not reproduced).
use crate::{
    Result,
    uai_kill::KillDecision,
    x87::{n, num, store},
};
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

/// `[A34770]` (binary32 1e-4): scores this close count as equal.
pub const SCORE_EPSILON_BITS: u32 = 0x38d1_b717;
/// `[A02790]` (binary32 1e-4): the `455000` near-equal threshold.
pub const NEAR_EQUAL_BITS: u32 = 0x38d1_b717;
/// Status value of an active embedded state.
const ACTIVE: u32 = 2;

/// Services of `536840`, called in native order.
pub trait CollisionThingWorld {
    /// Thing virtual +B4 (true for an actor).
    fn thing_is_actor(&mut self, thing: u32) -> Result<bool>;
    /// Mind actor (`529C30`, Mind+4) virtual +16C(thing).
    fn actor_accepts(&mut self, thing: u32) -> Result<bool>;
    /// `4D4FF0(thing, a, b)` on the Mind actor (0 when it can attack).
    fn relation(&mut self, thing: u32, a: u32, b: u32) -> Result<i32>;
    /// Mind+138 (engage state +C status).
    fn engage_status(&mut self) -> Result<u32>;
    /// Mind+104 (kill state +C status).
    fn kill_status(&mut self) -> Result<u32>;
    /// Mind+108 (kill state +10, target GTID).
    fn kill_target(&mut self) -> Result<u32>;
    /// GTID registry `[B364C0]` lookup (`432E40`): `None` for a generation
    /// mismatch or a null entry.
    fn resolve(&mut self, gtid: u32) -> Result<Option<u32>>;
    /// Mind actor +20/+24 (binary32).
    fn actor_position(&mut self) -> Result<[u32; 2]>;
    /// Object +20/+24 (binary32).
    fn object_position(&mut self, object: u32) -> Result<[u32; 2]>;
    /// `5340C0(target, null, &record, 0)` on the Mind
    /// ([`crate::uai_kill::choose`]); `record` holds [`KillDecision::initial`].
    fn choose(&mut self, target: Option<u32>, record: &mut KillDecision) -> Result<u32>;
    /// `5358A0(thing)` on the engage state (Mind+12C; `uai_avoid::hand_to_kill`);
    /// its result is unused.
    fn engage(&mut self, thing: u32) -> Result<()>;
    /// `5351C0(mode, thing)` on the kill state (Mind+F8;
    /// `uai_avoid::kill_select`); its result is unused.
    fn kill(&mut self, mode: u32, thing: u32) -> Result<()>;
    /// Mind Updatable (Mind+8) virtual +4(delay, data).
    fn schedule(&mut self, delay_bits: u32, data: u32) -> Result<()>;
}

/// `45D880(b)` on `a`: `(b.x - a.x)^2 + (b.y - a.y)^2`, unstored.
fn distance2(a: [u32; 2], b: [u32; 2], p: Precision) -> Result<Binary> {
    let dy = num(n(b[1])?.sub(&n(a[1])?, p))?;
    let dx = num(n(b[0])?.sub(&n(a[0])?, p))?;
    let xx = num(dx.mul(&dx, p))?;
    let yy = num(dy.mul(&dy, p))?;
    num(xx.add(&yy, p))
}

/// `|a - b|` at precision `p` (`fsub`, then `fchs` below `[9F32DC]` 0.0).
fn difference(a: u32, b: u32, p: Precision) -> Result<Binary> {
    let d = num(n(a)?.sub(&n(b)?, p))?;
    Ok(if d.compare(&Binary::from_i32(0)) == Ordering::Less {
        d.negated()
    } else {
        d
    })
}

/// `455000(a, b)`: `|a - b| < [A02790]`, unstored.
pub fn near_equal(a: u32, b: u32, p: Precision) -> Result<bool> {
    Ok(difference(a, b, p)?.compare(&n(NEAR_EQUAL_BITS)?) == Ordering::Less)
}

/// The record comparison of `536840` (from `5369B0`): true when the thing
/// (record `thing`) is taken over the current target (record `target`).
pub fn thing_preferred(target: &KillDecision, thing: &KillDecision, p: Precision) -> Result<bool> {
    if target.ability == 0 {
        return Ok(thing.ability != 0);
    }
    if thing.ability == 0 {
        return Ok(false);
    }
    if target.priority != thing.priority {
        return Ok(target.priority <= thing.priority);
    }
    if difference(target.score, thing.score, p)?.compare(&n(SCORE_EPSILON_BITS)?)
        == Ordering::Less
    {
        if near_equal(target.distance2, thing.distance2, p)? {
            return Ok(false);
        }
        return Ok(n(target.distance2)?.compare(&n(thing.distance2)?) != Ordering::Less);
    }
    Ok(n(target.score)?.compare(&n(thing.score)?) != Ordering::Greater)
}

/// Complete `536840(thing)` on the element move state at x87 precision `p`;
/// returns the original's byte result (true when Engage or Kill started).
pub fn result_collision_thing(
    thing: Option<u32>,
    world: &mut impl CollisionThingWorld,
    p: Precision,
) -> Result<bool> {
    let Some(thing) = thing else {
        return Ok(false);
    };
    if !world.thing_is_actor(thing)? || !world.actor_accepts(thing)? {
        return Ok(false);
    }
    if world.relation(thing, 1, 0)? != 0 {
        return Ok(false);
    }
    let engage_active = world.engage_status()? == ACTIVE;
    let kill_active = world.kill_status()? == ACTIVE;
    if !kill_active {
        if engage_active {
            world.engage(thing)?;
            world.schedule(0, 0)?;
            return Ok(true);
        }
        return Ok(false);
    }
    let gtid = world.kill_target()?;
    let target = world.resolve(gtid)?;
    if let Some(t) = target {
        let actor = world.actor_position()?;
        let kept = store(distance2(actor, world.object_position(t)?, p)?)?;
        let to_thing = distance2(actor, world.object_position(thing)?, p)?;
        if to_thing.compare(&n(kept)?) != Ordering::Less {
            return Ok(false);
        }
    }
    let mut target_record = KillDecision::initial();
    let mut thing_record = KillDecision::initial();
    world.choose(target, &mut target_record)?;
    world.choose(Some(thing), &mut thing_record)?;
    if !thing_preferred(&target_record, &thing_record, p)? {
        return Ok(false);
    }
    world.kill(0, thing)?;
    world.schedule(0, 0)?;
    Ok(true)
}
