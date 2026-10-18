//! Original `KKC_CAIStateGuard::Think` (`5294F0`, regiment state `cai_guard`
//! at Mind+17C, vtable A32BC8 slot +C): guard an object (state+18 GTID) by
//! attacking the nearest threat from its threat list.
//!
//! Regiment Mind embedded states used: sleep +34 (status Mind+40), move +68
//! (Mind+74), +C0 (Mind+CC, target Mind+D0) and +D4 (Mind+E0, target
//! Mind+E4). The state metadata names +C0 `kill` (Think `520730`, activate
//! `51D920`) and +D4 `bombard` (see `cai_thinks`); the identifiers here keep
//! the earlier labels FOLLOW (= kill) and ATTACK (= bombard), which the
//! oracle fixtures also use as event names. `529C20` (leave a state whose
//! status is nonzero) and `52A810(sleep, 0)` are the Mind primitives.
//!
//! Order (diagnostic log lines omitted):
//! 1. `529E60`. A guarded GTID that does not resolve to a live object, or a
//!    live one while the Mind has a follow target (Mind+2C), goes to Mind
//!    virtual +140(GTID) and ends the Think.
//! 2. Without a resolvable home (Mind+28) the guarded object is "settled" when
//!    +158 holds, its denizen component (+88) exists with +20 clear and its
//!    +15C object's +98 passes `585160`.
//! 3. Guarding and not settled: the threat list is the denizen component's
//!    pruned list (`569300`), else the object's +118 list, else one entry, the
//!    GTID of its Mind's +90 object. An active follow (or attack) target that
//!    is not listed and is not the guarded Mind's +90 object goes to +140.
//!    The best live threat minimises the binary32 squared distance to the
//!    organization; the current target uses
//!    `max(sqrt(d2) - GuardOldTargetBias, 0)^2`. An empty list falls back to
//!    the guarded Mind's live +90 object. The current target being best keeps
//!    the active follow/attack state thinking. Otherwise a best threat is
//!    engaged: relation `4D4FF0(t, 3, 0) == 0` -> `527CE0(3, t)`, else leave
//!    all four states, `51D920(0, t)` on the follow state and on success
//!    state+14 = 0, state+10 = 4, Mind virtual +B8; then schedule 1e-4 s.
//!    Without a threat: a home (Mind+28) that does not resolve goes to
//!    +140(guarded GTID); Mind+30 sends `56AEC0(Mind+30, Mind+24)` to the
//!    home's denizen component; a non-current state ends; `51EE20` clears +14.
//! 4. A clear state+14 calls `527CE0(2, 0)` and sets it. A moving Mind thinks
//!    the move state; else, unless sleeping, all four states are left, sleep
//!    is activated in mode 0, state+10 = 1 and Mind virtual +B8; then the
//!    sleep state thinks.
use crate::{
    Arena, Error, MindId, Result, StateId, World, activate, is_current, think_base,
    x87::{n, num, store},
};
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

pub const SLEEP_OFFSET: u32 = 0x34;
pub const MOVE_OFFSET: u32 = 0x68;
/// The `kill` state (metadata name; labelled follow here).
pub const FOLLOW_OFFSET: u32 = 0xc0;
/// The `bombard` state (metadata name; labelled attack here).
pub const ATTACK_OFFSET: u32 = 0xd4;
/// binary32 1e-4: re-think delay after engaging a threat.
pub const ENGAGE_DELAY_BITS: u32 = 0x38d1_b717;
const FLT_MAX: u32 = 0x7f7f_ffff;

/// Services in native order (`World` drives the Mind primitives).
pub trait GuardThinkWorld: World {
    /// Guard state+18 (guarded GTID).
    fn guarded(&mut self, state: StateId) -> Result<u32>;
    /// GTID registry entry for a matching generation (`None` when the
    /// generation differs or the entry is null).
    fn resolve(&mut self, gtid: u32) -> Result<Option<u32>>;
    /// Object+8 bit 0 (removed).
    fn object_removed(&mut self, object: u32) -> Result<bool>;
    /// Object+14 (its GTID).
    fn object_gtid(&mut self, object: u32) -> Result<u32>;
    /// Mind virtual +140(gtid).
    fn mind_virtual_140(&mut self, mind: MindId, gtid: u32, arena: &mut Arena) -> Result<()>;
    /// Object virtual +158.
    fn object_virtual_158(&mut self, object: u32) -> Result<bool>;
    /// Object virtual +15C, `None` when null.
    fn object_virtual_15c(&mut self, object: u32) -> Result<Option<u32>>;
    /// Object+88 (denizen component), `None` when null.
    fn denizen_component(&mut self, object: u32) -> Result<Option<u32>>;
    /// Denizen component+20 byte.
    fn component_byte20(&mut self, component: u32) -> Result<bool>;
    /// Object+98, `None` when null.
    fn object_word98(&mut self, object: u32) -> Result<Option<u32>>;
    /// `585160`.
    fn check_585160(&mut self, value: u32) -> Result<bool>;
    /// `569300` on a denizen component: its threat list (+24) after pruning
    /// dead heads, as GTIDs head first.
    fn component_threats(&mut self, component: u32) -> Result<Vec<u32>>;
    /// Object+118 GTID list, head first.
    fn object_threats(&mut self, object: u32) -> Result<Vec<u32>>;
    /// Object+70 (its Mind), `None` when null.
    fn object_mind(&mut self, object: u32) -> Result<Option<u32>>;
    /// Object Mind virtual +90, `None` when null.
    fn object_mind_virtual_90(&mut self, object_mind: u32) -> Result<Option<u32>>;
    /// Embedded state+10 word (Mind+D0 follow target, Mind+E4 attack target).
    fn state_target(&mut self, mind: MindId, offset: u32) -> Result<u32>;
    /// Identity of the embedded state at Mind+`offset`.
    fn embedded_state(&mut self, mind: MindId, offset: u32) -> Result<StateId>;
    /// Object +20/+24 position (binary32).
    fn object_position(&mut self, object: u32) -> Result<[u32; 2]>;
    /// Organization (`529C30`) +20/+24 position.
    fn organization_position(&mut self, mind: MindId) -> Result<[u32; 2]>;
    /// SVar `GuardOldTargetBias` (binary32).
    fn guard_old_target_bias(&mut self) -> Result<u32>;
    /// Virtual +C (Think) of the embedded state at Mind+`offset`.
    fn embedded_think(&mut self, mind: MindId, offset: u32, arena: &mut Arena) -> Result<()>;
    /// `4D4FF0(object, 3, 0)` on the organization.
    fn relation(&mut self, mind: MindId, object: u32) -> Result<i32>;
    /// `527CE0(mode, object)` on the guard state.
    fn guard_527ce0(
        &mut self,
        state: StateId,
        mode: u32,
        object: u32,
        arena: &mut Arena,
    ) -> Result<()>;
    /// `51D920(mode, object)` on the follow state.
    fn follow_activate(
        &mut self,
        mind: MindId,
        mode: u32,
        object: u32,
        arena: &mut Arena,
    ) -> Result<bool>;
    /// Mind virtual +B8.
    fn mind_virtual_b8(&mut self, mind: MindId, arena: &mut Arena) -> Result<()>;
    /// Guard state+10 word and +14 byte.
    fn set_guard_word10(&mut self, state: StateId, value: u32) -> Result<()>;
    fn guard_byte14(&mut self, state: StateId) -> Result<bool>;
    fn set_guard_byte14(&mut self, state: StateId, value: bool) -> Result<()>;
    /// `56AEC0(word30, word24)` on a home's denizen component (`None`: the
    /// native call runs on a null component).
    fn home_settle(&mut self, component: Option<u32>, word30: u32, word24: u32) -> Result<()>;
    /// `51EE20` on the guard state.
    fn guard_51ee20(&mut self, state: StateId, arena: &mut Arena) -> Result<bool>;
}

fn status_at(
    world: &mut impl GuardThinkWorld,
    mind: MindId,
    offset: u32,
    arena: &Arena,
) -> Result<u32> {
    let s = world.embedded_state(mind, offset)?;
    Ok(arena.state(s)?.status)
}
/// `529C20`: leave a state whose status is nonzero.
fn leave_all(world: &mut impl GuardThinkWorld, mind: MindId, arena: &mut Arena) -> Result<()> {
    for offset in [FOLLOW_OFFSET, ATTACK_OFFSET, MOVE_OFFSET, SLEEP_OFFSET] {
        let s = world.embedded_state(mind, offset)?;
        if arena.state(s)?.status != 0 {
            world.leave(s, arena)?;
        }
    }
    Ok(())
}
fn live(world: &mut impl GuardThinkWorld, gtid: u32) -> Result<Option<u32>> {
    match world.resolve(gtid)? {
        Some(o) if !world.object_removed(o)? => Ok(Some(o)),
        _ => Ok(None),
    }
}
fn guarded_object(object: Option<u32>) -> Result<u32> {
    object.ok_or_else(|| Error::World("null guarded object dereference".into()))
}

/// The threat list selection of step 3.
fn threats(world: &mut impl GuardThinkWorld, object: u32) -> Result<Vec<u32>> {
    if let Some(component) = world.denizen_component(object)? {
        return world.component_threats(component);
    }
    let list = world.object_threats(object)?;
    if !list.is_empty() {
        return Ok(list);
    }
    let Some(mm) = world.object_mind(object)? else {
        return Ok(list);
    };
    if world.object_mind_virtual_90(mm)?.is_none() {
        return Ok(list);
    }
    let target = world.object_mind_virtual_90(mm)?;
    let gtid = match target {
        Some(t) => world.object_gtid(t)?,
        None => 0,
    };
    Ok(vec![gtid])
}

/// Whether an unlisted follow/attack target is the guarded Mind's +90 object.
fn is_guarded_target(world: &mut impl GuardThinkWorld, object: u32, gtid: u32) -> Result<bool> {
    let Some(mm) = world.object_mind(object)? else {
        return Ok(false);
    };
    let mine = world.object_mind_virtual_90(mm)?;
    Ok(mine == world.resolve(gtid)?)
}

/// Complete `5294F0` at x87 precision `p`. Returns the base Think's
/// wrong-activity flag.
pub fn think(
    state: StateId,
    arena: &mut Arena,
    world: &mut impl GuardThinkWorld,
    p: Precision,
) -> Result<bool> {
    let wrong = think_base(state, arena)?;
    let mind = arena.state(state)?.parent;
    let guarded_gtid = world.guarded(state)?;
    let guarded = world.resolve(guarded_gtid)?;
    if guarded_gtid != 0 {
        let alive = match guarded {
            Some(o) => !world.object_removed(o)?,
            None => false,
        };
        if !alive || arena.mind(mind)?.words_24_30[2] != 0 {
            world.mind_virtual_140(mind, guarded_gtid, arena)?;
            return Ok(wrong);
        }
    }
    let mut settled = false;
    let home = arena.mind(mind)?.words_24_30[1];
    if world.resolve(home)?.is_none()
        && let Some(o) = guarded
        && world.object_virtual_158(o)?
    {
        let t = world.object_virtual_15c(o)?;
        if let Some(component) = world.denizen_component(o)?
            && !world.component_byte20(component)?
            && let Some(t) = t
            && let Some(x) = world.object_word98(t)?
            && world.check_585160(x)?
        {
            settled = true;
        }
    }
    if world.guarded(state)? != 0 && !settled {
        let object = guarded_object(guarded)?;
        let list = threats(world, object)?;
        let mut current = 0_u32;
        if status_at(world, mind, FOLLOW_OFFSET, arena)? == 2 {
            current = world.state_target(mind, FOLLOW_OFFSET)?;
            if !list.contains(&current) && !is_guarded_target(world, object, current)? {
                let gtid = world.state_target(mind, FOLLOW_OFFSET)?;
                world.mind_virtual_140(mind, gtid, arena)?;
                return Ok(wrong);
            }
        } else if status_at(world, mind, ATTACK_OFFSET, arena)? == 2 {
            let target = world.state_target(mind, ATTACK_OFFSET)?;
            if target != 0 {
                current = target;
                if !list.contains(&current) && !is_guarded_target(world, object, current)? {
                    world.mind_virtual_140(mind, target, arena)?;
                    return Ok(wrong);
                }
            }
        }
        let mut best: Option<u32> = None;
        if list.is_empty() {
            if let Some(mm) = world.object_mind(object)? {
                best = world.object_mind_virtual_90(mm)?;
                if let Some(b) = best
                    && world.object_removed(b)?
                {
                    best = None;
                }
            }
        } else {
            let mut best_score = FLT_MAX;
            for &gtid in &list {
                let Some(o) = live(world, gtid)? else {
                    continue;
                };
                let at = world.object_position(o)?;
                let org = world.organization_position(mind)?;
                let dy = num(n(at[1])?.sub(&n(org[1])?, p))?;
                let dx = num(n(at[0])?.sub(&n(org[0])?, p))?;
                let d2 = num(num(dx.mul(&dx, p))?.add(&num(dy.mul(&dy, p))?, p))?;
                let d2 = store(d2)?;
                let score = if world.resolve(current)? == Some(o) {
                    let d = store(num(n(d2)?.sqrt(p))?)?;
                    let bias = world.guard_old_target_bias()?;
                    let mut v = num(n(d)?.sub(&n(bias)?, p))?;
                    if v.compare(&Binary::from_i32(0)) == Ordering::Less {
                        v = Binary::from_i32(0);
                    }
                    num(v.mul(&v, p))?
                } else {
                    n(d2)?
                };
                if score.compare(&n(best_score)?) == Ordering::Less {
                    best_score = store(score)?;
                    best = Some(o);
                }
            }
        }
        if let Some(cur) = world.resolve(current)?
            && best == Some(cur)
        {
            if status_at(world, mind, FOLLOW_OFFSET, arena)? == 2 {
                world.embedded_think(mind, FOLLOW_OFFSET, arena)?;
                return Ok(wrong);
            }
            if status_at(world, mind, ATTACK_OFFSET, arena)? == 2
                && world.state_target(mind, ATTACK_OFFSET)? != 0
            {
                world.embedded_think(mind, ATTACK_OFFSET, arena)?;
                return Ok(wrong);
            }
        }
        if let Some(t) = best {
            if world.relation(mind, t)? == 0 {
                world.guard_527ce0(state, 3, t, arena)?;
            } else {
                leave_all(world, mind, arena)?;
                if world.follow_activate(mind, 0, t, arena)? {
                    world.set_guard_byte14(state, false)?;
                    world.set_guard_word10(state, 4)?;
                    world.mind_virtual_b8(mind, arena)?;
                }
            }
            world.schedule(mind, ENGAGE_DELAY_BITS, 0, arena)?;
            return Ok(wrong);
        }
        let home = arena.mind(mind)?.words_24_30[1];
        if home != 0 {
            if world.resolve(home)?.is_none() {
                let gtid = world.guarded(state)?;
                world.mind_virtual_140(mind, gtid, arena)?;
                return Ok(wrong);
            }
            let words = arena.mind(mind)?.words_24_30;
            if words[3] != 0 {
                let component = match world.resolve(home)? {
                    Some(o) => world.denizen_component(o)?,
                    None => return Err(Error::World("null home object dereference".into())),
                };
                world.home_settle(component, words[3], words[0])?;
            }
            if !is_current(state, arena)? {
                return Ok(wrong);
            }
        }
        if world.guard_51ee20(state, arena)? {
            world.set_guard_byte14(state, false)?;
        }
    }
    if !world.guard_byte14(state)? {
        world.guard_527ce0(state, 2, 0, arena)?;
        world.set_guard_byte14(state, true)?;
    }
    if status_at(world, mind, MOVE_OFFSET, arena)? == 2 {
        world.embedded_think(mind, MOVE_OFFSET, arena)?;
        return Ok(wrong);
    }
    if status_at(world, mind, SLEEP_OFFSET, arena)? != 2 {
        leave_all(world, mind, arena)?;
        let sleep = world.embedded_state(mind, SLEEP_OFFSET)?;
        activate(sleep, 0, arena, world)?;
        world.set_guard_word10(state, 1)?;
        world.mind_virtual_b8(mind, arena)?;
    }
    world.embedded_think(mind, SLEEP_OFFSET, arena)?;
    Ok(wrong)
}
