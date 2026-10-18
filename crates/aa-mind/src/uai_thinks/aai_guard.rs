//! Original `KKC_AAIStateGuard::Think` (`51C900`, aircraft state
//! `aai_guard` at Mind+9C, vtable A324B4 slot +C) with its inline helpers
//! `51C540` (set the guard target) and the GTID list predicates `51A550` /
//! `517F30`.
//!
//! The aircraft attack state is Mind+E8 (status Mind+F4, +18 word Mind+100,
//! +30 word Mind+118); "idle" below means its +30 word is 0 or 4.
//!
//! Order:
//! 1. `529E60` ([`crate::think_base`]). A guarded object (state+10 GTID)
//!    that does not resolve or is removed (+8 bit 0) clears state+14, leaves
//!    an active idle attack state (`529C20`), activates the join state in
//!    mode 2 (`5176A0`) and schedules 1e-4 s.
//! 2. A live current target (state+14) with `4D4FF0(target, 1, 0) == 0` on
//!    the actor keeps attacking (attack state Think) when the guarded object
//!    has no denizen component (+88) or its threat list (`569300`) contains
//!    state+14; otherwise (and for any nonzero state+14 that fails) state+14
//!    is cleared and an active idle attack state is left.
//! 3. An active attack state that is not idle Thinks (done).
//! 4. Without a denizen component: join (mode 2) and 1e-4 s. Otherwise, the
//!    threat list is fetched; with an active attack state its +18 GTID, when
//!    listed (second `569300`), becomes the target (`51C540` on its object)
//!    and the Think ends; when not, state+14 is cleared and an idle attack
//!    state is left. The nearest live listed object passing
//!    `4D4FF0(object, 1, 0) == 0` (binary32 squared distance to the actor,
//!    unstored candidate against the stored best, start FLT_MAX, x87 at the
//!    control-word precision) is attacked: `51B790(0, object, 0)` on the
//!    attack state, `51C540(object)`, 1e-4 s. No candidate: join (mode 2)
//!    and 1e-4 s.
use super::{AircraftWorld, IMMEDIATE_BITS, aircraft, leave_if_active, status_at};
use crate::{
    Arena, MindId, Result, StateId, think_base,
    x87::{n, num, store},
};
use aa_numeric::Precision;
use std::cmp::Ordering;

/// binary32 FLT_MAX: the initial best squared distance.
const FLT_MAX: u32 = 0x7f7f_ffff;

/// Aircraft attack-state services shared by the guard (`51C900`) and escort
/// (`51CC10`) Thinks, whose states both keep their target GTID at +14.
pub trait AttackTargetWorld: AircraftWorld {
    /// State+14 (target GTID) and its write.
    fn state_target(&mut self, state: StateId) -> Result<u32>;
    fn set_state_target(&mut self, state: StateId, gtid: u32) -> Result<()>;
    /// Attack state +30 (Mind+118) and +18 (Mind+100) words.
    fn attack_word30(&mut self, mind: MindId) -> Result<u32>;
    fn attack_word18(&mut self, mind: MindId) -> Result<u32>;
    /// Move state +48 byte (Mind+7C).
    fn move_byte48(&mut self, mind: MindId) -> Result<u8>;
    /// `51B790(a, object, c)` on the attack state (not ported); the result
    /// is unused by these callers.
    fn attack_activate(
        &mut self,
        mind: MindId,
        a: u32,
        object: u32,
        c: u32,
        arena: &mut Arena,
    ) -> Result<()>;
}

/// Services of `51C900` in native order.
pub trait AirGuardThinkWorld: AttackTargetWorld {
    /// Guard state+10 (guarded GTID).
    fn guarded(&mut self, state: StateId) -> Result<u32>;
    /// Object +88 (denizen component), `None` when null.
    fn denizen_component(&mut self, object: u32) -> Result<Option<u32>>;
    /// `569300` on a denizen component: its pruned threat list, head first.
    fn component_threats(&mut self, component: u32) -> Result<Vec<u32>>;
}

/// Attack state active (2) with its +30 word 0 or 4.
pub(crate) fn attack_idle_leave(
    mind: MindId,
    arena: &mut Arena,
    world: &mut impl AttackTargetWorld,
) -> Result<()> {
    if status_at(world, mind, aircraft::ATTACK, arena)? == 2 {
        let word = world.attack_word30(mind)?;
        if word == 0 || word == 4 {
            leave_if_active(world, mind, aircraft::ATTACK, arena)?;
        }
    }
    Ok(())
}

/// The non-null part of `51C540(object)` / `51C5E0(object)` on the guard
/// or escort state: state+14 becomes the object's GTID; an inactive attack
/// state is activated on it (`51B790(0, object, 0)`); an active one on a
/// different +18 target is left and reactivated with the move state +48
/// byte.
pub fn target_object(
    state: StateId,
    object: u32,
    arena: &mut Arena,
    world: &mut impl AttackTargetWorld,
) -> Result<()> {
    let mind = arena.state(state)?.parent;
    let gtid = world.object_gtid(object)?;
    world.set_state_target(state, gtid)?;
    if status_at(world, mind, aircraft::ATTACK, arena)? != 2 {
        return world.attack_activate(mind, 0, object, 0, arena);
    }
    if gtid == world.attack_word18(mind)? {
        return Ok(());
    }
    leave_if_active(world, mind, aircraft::ATTACK, arena)?;
    let byte = world.move_byte48(mind)?;
    world.attack_activate(mind, 0, object, u32::from(byte), arena)
}

/// `51C540(object)` on the guard state; a null object clears state+14 and
/// leaves an active idle attack state.
pub fn set_target_object(
    state: StateId,
    object: Option<u32>,
    arena: &mut Arena,
    world: &mut impl AttackTargetWorld,
) -> Result<()> {
    match object {
        Some(object) => target_object(state, object, arena, world),
        None => {
            world.set_state_target(state, 0)?;
            let mind = arena.state(state)?.parent;
            attack_idle_leave(mind, arena, world)
        }
    }
}

fn join_soon(mind: MindId, arena: &mut Arena, world: &mut impl AirGuardThinkWorld) -> Result<()> {
    world.join_activate(mind, 2, arena)?;
    world.schedule(mind, IMMEDIATE_BITS, 0, arena)
}

fn live(world: &mut impl AirGuardThinkWorld, gtid: u32) -> Result<Option<u32>> {
    world.resolve(gtid)
}

/// Complete `51C900` at x87 precision `p`. Returns the base Think's
/// wrong-activity flag.
pub fn think(
    state: StateId,
    arena: &mut Arena,
    world: &mut impl AirGuardThinkWorld,
    p: Precision,
) -> Result<bool> {
    let wrong = think_base(state, arena)?;
    let mind = arena.state(state)?.parent;
    let gtid = world.guarded(state)?;
    let guarded = match world.resolve(gtid)? {
        Some(g) if !world.object_removed(g)? => g,
        _ => {
            world.set_state_target(state, 0)?;
            attack_idle_leave(mind, arena, world)?;
            join_soon(mind, arena, world)?;
            return Ok(wrong);
        }
    };
    let current = world.state_target(state)?;
    let mut drop = current != 0;
    if live(world, current)?.is_some() {
        let object = world.resolve(current)?;
        if world.relation(mind, object, 1, 0)? == 0 {
            match world.denizen_component(guarded)? {
                None => {
                    world.embedded_think(mind, aircraft::ATTACK, arena)?;
                    return Ok(wrong);
                }
                Some(component) => {
                    let list = world.component_threats(component)?;
                    let target = world.state_target(state)?;
                    if list.contains(&target) {
                        world.embedded_think(mind, aircraft::ATTACK, arena)?;
                        return Ok(wrong);
                    }
                    drop = true;
                }
            }
        }
    }
    if drop {
        world.set_state_target(state, 0)?;
        attack_idle_leave(mind, arena, world)?;
    }
    if status_at(world, mind, aircraft::ATTACK, arena)? == 2 {
        let word = world.attack_word30(mind)?;
        if word != 0 && word != 4 {
            world.embedded_think(mind, aircraft::ATTACK, arena)?;
            return Ok(wrong);
        }
    }
    let Some(component) = world.denizen_component(guarded)? else {
        join_soon(mind, arena, world)?;
        return Ok(wrong);
    };
    let list = world.component_threats(component)?;
    if status_at(world, mind, aircraft::ATTACK, arena)? == 2 {
        let attacked = world.attack_word18(mind)?;
        if world.component_threats(component)?.contains(&attacked) {
            let gtid = world.attack_word18(mind)?;
            world.set_state_target(state, gtid)?;
            let object = world.resolve(gtid)?;
            set_target_object(state, object, arena, world)?;
            return Ok(wrong);
        }
        world.set_state_target(state, 0)?;
        attack_idle_leave(mind, arena, world)?;
    }
    let mut best: Option<(u32, u32)> = None;
    for gtid in list {
        let Some(object) = live(world, gtid)? else {
            continue;
        };
        if world.relation(mind, Some(object), 1, 0)? != 0 {
            continue;
        }
        let [ax, ay] = world.actor_position(mind)?;
        let [ox, oy] = world.object_position(object)?;
        let dy = num(n(oy)?.sub(&n(ay)?, p))?;
        let dx = num(n(ox)?.sub(&n(ax)?, p))?;
        let d2 = num(num(dx.mul(&dx, p))?.add(&num(dy.mul(&dy, p))?, p))?;
        let stored = best.map_or(FLT_MAX, |(_, s)| s);
        if d2.compare(&n(stored)?) == Ordering::Less {
            best = Some((object, store(d2)?));
        }
    }
    let Some((object, _)) = best else {
        join_soon(mind, arena, world)?;
        return Ok(wrong);
    };
    world.attack_activate(mind, 0, object, 0, arena)?;
    set_target_object(state, Some(object), arena, world)?;
    world.schedule(mind, IMMEDIATE_BITS, 0, arena)?;
    Ok(wrong)
}
