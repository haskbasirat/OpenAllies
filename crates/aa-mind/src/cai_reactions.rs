//! Original company/unit Mind reactions called from combat and the kill
//! states (Source/Sim/Tai/CAI.cpp, UAI.cpp).
//!
//! - `51F9F0` exhausted and `51F920` wavering morale reactions (called by
//!   aa_combat::morale): notify the SAI actor record, then rout through
//!   `52A810(rout state, 2)` unless blocked.
//! - `5206A0` `KKC_CAIStateKill::OnEnter`: activate the move state toward the
//!   kill target.
//! - `52E830` UAI engage reaction delay (SVar `EngageReactionTimeRange`).
//! - `527240` regiment Mind target priority (virtual +1AC).
use crate::{
    Arena, MindId, Result, StateId, World, activate, is_current,
    x87::{n, num, store, unit_random},
};
use aa_numeric::Precision;
use std::cmp::Ordering;

/// SAI actor record (`actor+154`, refcounted) fields read by the reactions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaiRecord {
    pub word10: u32,
    pub word3c: u32,
    /// Word at the record's +30 pointer, `None` when the pointer is null.
    pub state30: Option<u32>,
}

/// Services of the morale reactions (plus the Mind `World` for `52A810`).
pub trait ReactionWorld: World {
    /// `[B36534]+D8` session mode (6 skips the SAI notification).
    fn session_mode(&mut self) -> Result<u32>;
    /// `4D03C0`: the Mind actor's SAI record.
    fn sai_record(&mut self, mind: MindId) -> Result<Option<SaiRecord>>;
    /// `7B6800(record)` on record+C (thunk to `7B5210`).
    fn notify_sai(&mut self, mind: MindId) -> Result<()>;
    /// `4D1080` on the Mind actor.
    fn actor_blocked(&mut self, mind: MindId) -> Result<bool>;
}

/// `51F9F0`: returns true when the rout state was activated.
pub fn exhausted(
    mind: MindId,
    engage_state: StateId,
    rout_state: StateId,
    arena: &mut Arena,
    world: &mut impl ReactionWorld,
) -> Result<bool> {
    if world.session_mode()? != 6
        && let Some(record) = world.sai_record(mind)?
        && record.state30.is_some_and(|s| s != 2)
    {
        world.notify_sai(mind)?;
    }
    if is_current(engage_state, arena)? || is_current(rout_state, arena)? {
        return Ok(false);
    }
    if world.actor_blocked(mind)? {
        return Ok(false);
    }
    activate(rout_state, 2, arena, world)
}

/// `51F920`: returns true when the rout state was activated.
pub fn wavering(
    mind: MindId,
    rout_state: StateId,
    arena: &mut Arena,
    world: &mut impl ReactionWorld,
) -> Result<bool> {
    if world.session_mode()? != 6
        && let Some(record) = world.sai_record(mind)?
        && record.word10 != record.word3c
    {
        world.notify_sai(mind)?;
    }
    if world.actor_blocked(mind)? {
        return Ok(false);
    }
    activate(rout_state, 2, arena, world)
}

/// Services of `5206A0`.
pub trait KillEnterWorld {
    /// GTID registry lookup of state+10.
    fn resolve(&mut self, gtid: u32) -> Result<Option<u32>>;
    /// `520320(0, target, 0, [B279C0], [B279C4])` on the move state Mind+68.
    fn activate_move_to_actor(&mut self, target: Option<u32>) -> Result<bool>;
}

/// `5206A0`: sets status 2 and starts the move; returns the move result
/// (false is where the original logs "KILL activating MOVE failed").
pub fn kill_on_enter(
    state: StateId,
    target_gtid: u32,
    arena: &mut Arena,
    world: &mut impl KillEnterWorld,
) -> Result<bool> {
    arena.state_mut(state)?.status = 2;
    let target = world.resolve(target_gtid)?;
    world.activate_move_to_actor(target)
}

/// `0xA34770` binary32 1e-4: minimum reaction delay.
const MINIMUM_DELAY_BITS: u32 = 0x38d1_b717;

/// `52E830`: draws `u * EngageReactionTimeRange` from the world RNG and
/// returns the delay passed to the Mind scheduler (binary32 bits).
pub fn engage_reaction_delay(world_random: u32, range_bits: u32, p: Precision) -> Result<u32> {
    let delay = num(unit_random(world_random, p)?.mul(&n(range_bits)?, p))?;
    if delay.compare(&n(MINIMUM_DELAY_BITS)?) == Ordering::Less {
        return Ok(MINIMUM_DELAY_BITS);
    }
    store(delay)
}

/// Services of `527240`.
pub trait PriorityWorld {
    /// `4D03F0` on the target.
    fn target_4d03f0(&mut self, target: u32) -> Result<bool>;
    /// Target Mind (`+70`) virtual +58, `None` without a Mind.
    fn target_mind_58(&mut self, target: u32) -> Result<Option<bool>>;
    /// Mind virtual +4C (current kill target).
    fn current_target(&mut self) -> Result<Option<u32>>;
    /// SVar `KillTargetIsHighestPriority`.
    fn kill_target_is_highest(&mut self) -> Result<bool>;
    /// Target +60 present and definition +290/+291 bytes both zero.
    fn plain_target(&mut self, target: u32) -> Result<bool>;
}

/// `527240`: priority class 1..6 of `target` for this Mind.
pub fn target_priority(target: u32, world: &mut impl PriorityWorld) -> Result<i32> {
    let a = world.target_4d03f0(target)?;
    let b = world.target_mind_58(target)?.unwrap_or(false);
    let current = world.current_target()? == Some(target);
    if current && world.kill_target_is_highest()? {
        return Ok(6);
    }
    if a {
        if current {
            return Ok(4);
        }
        return Ok(if world.plain_target(target)? { 1 } else { 2 });
    }
    Ok(if current || b { 5 } else { 3 })
}
