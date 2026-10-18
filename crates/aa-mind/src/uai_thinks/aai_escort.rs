//! Original `KKC_AAIStateEscort::Think` (`51CC10`, aircraft state
//! `aai_escort` at Mind+D0, vtable A3266C slot +C) with the reachable
//! (non-null) part of its target setter `51C5E0`
//! ([`super::aai_guard::target_object`]).
//!
//! State+10 is the escorted group (a pointer; its +0C is a GTID list) and
//! state+14 the current target GTID. Mind+30 is a second group.
//!
//! Order:
//! 1. `529E60` ([`crate::think_base`]). A live target keeps attacking (the
//!    attack state, Mind+E8, Thinks). A dead nonzero one clears state+14,
//!    leaves an active attack state (`529C20`) and re-escorts the group on
//!    the move state (`516DD0(group)`, movement-owned).
//! 2. No group: join (`5176A0(2)`) and 1e-4 s.
//! 3. An active attack state with a zero +18 word (Mind+100) Thinks; with a
//!    nonzero one the target is dropped as in 1.
//! 4. The group members (live GTIDs) recently attacked (`4D0440`) offer their
//!    body (+60) +28 record (attacker GTID, +10 binary32 value): the first
//!    live attacker, then any with a strictly greater value, is kept. A live
//!    kept attacker is targeted (`51C5E0`) and 1e-4 s scheduled; else the
//!    same for the Mind+30 group; else `5684E0(group)` true joins (mode 2,
//!    1e-4 s) and false schedules 0.6 s.
use super::{
    IMMEDIATE_BITS, POINT_SIX_BITS,
    aai_guard::{AttackTargetWorld, target_object},
    aircraft, leave_if_active, null, status_at,
};
use crate::{Arena, MindId, Result, StateId, think_base};
use std::cmp::Ordering;

/// Services of `51CC10` in native order.
pub trait EscortThinkWorld: AttackTargetWorld {
    /// Escort state+10 (group), `None` when null.
    fn escort_group(&mut self, state: StateId) -> Result<Option<u32>>;
    /// `516DD0(group)` on the aircraft move state (movement-owned).
    fn move_escort(&mut self, mind: MindId, group: Option<u32>, arena: &mut Arena) -> Result<()>;
    /// Group +0C GTID list, head first.
    fn group_members(&mut self, group: u32) -> Result<Vec<u32>>;
    /// `4D0440` on an object (recently attacked).
    fn recently_attacked(&mut self, object: u32) -> Result<bool>;
    /// Object body (+60) +28 record: attacker GTID and its +10 binary32
    /// value; `None` when the body is null (a native null read).
    fn attacker_record(&mut self, object: u32) -> Result<Option<[u32; 2]>>;
    /// `5684E0` on the group.
    fn group_5684e0(&mut self, group: Option<u32>) -> Result<bool>;
}

fn host(bits: u32) -> f32 {
    f32::from_bits(bits)
}

/// Step 4 for one group: the kept attacker GTID (0 when none).
fn scan(group: u32, world: &mut impl EscortThinkWorld) -> Result<u32> {
    let mut best = [0_u32, 0_u32];
    for gtid in world.group_members(group)? {
        let Some(object) = world.resolve(gtid)? else {
            continue;
        };
        if !world.recently_attacked(object)? {
            continue;
        }
        let record = world
            .attacker_record(object)?
            .ok_or_else(|| null("body +28 record read"))?;
        if world.resolve(record[0])?.is_none() {
            continue;
        }
        if world.resolve(best[0])?.is_some()
            && host(record[1]).partial_cmp(&host(best[1])) != Some(Ordering::Greater)
        {
            continue;
        }
        best = record;
    }
    Ok(best[0])
}

fn drop_target(
    state: StateId,
    mind: MindId,
    arena: &mut Arena,
    world: &mut impl EscortThinkWorld,
) -> Result<()> {
    world.set_state_target(state, 0)?;
    if status_at(world, mind, aircraft::ATTACK, arena)? == 2 {
        leave_if_active(world, mind, aircraft::ATTACK, arena)?;
    }
    let group = world.escort_group(state)?;
    world.move_escort(mind, group, arena)
}

/// Complete `51CC10`. Returns the base Think's wrong-activity flag.
pub fn think(state: StateId, arena: &mut Arena, world: &mut impl EscortThinkWorld) -> Result<bool> {
    let wrong = think_base(state, arena)?;
    let mind = arena.state(state)?.parent;
    let current = world.state_target(state)?;
    if world.resolve(current)?.is_some() {
        world.embedded_think(mind, aircraft::ATTACK, arena)?;
        return Ok(wrong);
    }
    if current != 0 {
        drop_target(state, mind, arena, world)?;
    }
    let Some(group) = world.escort_group(state)? else {
        world.join_activate(mind, 2, arena)?;
        world.schedule(mind, IMMEDIATE_BITS, 0, arena)?;
        return Ok(wrong);
    };
    if status_at(world, mind, aircraft::ATTACK, arena)? == 2 {
        if world.attack_word18(mind)? == 0 {
            world.embedded_think(mind, aircraft::ATTACK, arena)?;
            return Ok(wrong);
        }
        drop_target(state, mind, arena, world)?;
    }
    let attacker = scan(group, world)?;
    let mut found = world.resolve(attacker)?;
    if found.is_none() {
        let second = arena.mind(mind)?.words_24_30[3];
        if second != 0 {
            let attacker = scan(second, world)?;
            found = world.resolve(attacker)?;
        }
    }
    if let Some(object) = found {
        target_object(state, object, arena, world)?;
        world.schedule(mind, IMMEDIATE_BITS, 0, arena)?;
        return Ok(wrong);
    }
    let group = world.escort_group(state)?;
    if world.group_5684e0(group)? {
        world.join_activate(mind, 2, arena)?;
        world.schedule(mind, IMMEDIATE_BITS, 0, arena)?;
    } else {
        world.schedule(mind, POINT_SIX_BITS, 0, arena)?;
    }
    Ok(wrong)
}
