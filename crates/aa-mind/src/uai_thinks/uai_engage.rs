//! Original `KKC_UAIStateEngage::Think` (`536E10`, element state
//! `uai_engage` at Mind+12C, vtable A34B14 slot +C) with its inline helper
//! `5358A0` (hand an engage choice to the kill state).
//!
//! Element Mind embedded states used: move +7C (status Mind+88) and kill +F8
//! (status Mind+104, target GTID Mind+108).
//!
//! Order:
//! 1. `529E60` ([`crate::think_base`]).
//! 2. The engage target (state+10 GTID) is resolved. With a live target and
//!    an organization (actor+80 -> +10), `5210D0(target)` on the
//!    organization Mind's engage state (`578D00(org+7C)`+1CC) returning false
//!    sets the reselect byte state+18.
//! 3. A kill state that is not active (status != 2) also sets state+18.
//! 4. With state+18 set, it is cleared and `535910(target, 0)` picks an
//!    object. None: an active kill state is left (`529C20`) and the member
//!    returns to its slot (`52E9D0`, movement-owned). Otherwise `5358A0`:
//!    unless the kill state is active on that object's GTID already, an
//!    active move state is left and `5351C0(0, object)` runs on the kill
//!    state; then, when the kill state was not active at step 3, the engage
//!    reaction delay (`52E830`, [`crate::cai_reactions::engage_reaction_delay`])
//!    is scheduled and the Think ends.
//! 5. A kill target (Mind+108) that resolves: the kill state Thinks.
//!    Otherwise an active kill state is left and the member returns to its
//!    slot (`52E9D0`).
use super::{ThinkWorld, element, leave_if_active, status_at};
use crate::{Arena, MindId, Result, StateId, cai_reactions::engage_reaction_delay, think_base};
use aa_numeric::Precision;

/// Services of `536E10` in native order.
pub trait EngageThinkWorld: ThinkWorld {
    /// Engage state+10 (target GTID).
    fn engage_target(&mut self, state: StateId) -> Result<u32>;
    /// Engage state+18 byte (reselect) and its write.
    fn reselect(&mut self, state: StateId) -> Result<bool>;
    fn set_reselect(&mut self, state: StateId, value: bool) -> Result<()>;
    /// Actor +80 membership -> +10 organization (`None` when either is null).
    fn organization(&mut self, mind: MindId) -> Result<Option<u32>>;
    /// `5210D0(target)` on the organization Mind's engage state
    /// (`578D00(organization+7C)` + 1CC).
    fn regiment_engage_keeps(&mut self, organization: u32, target: u32) -> Result<bool>;
    /// `535910(target, 0)` on the engage state (not ported): the chosen
    /// object, `None` when null.
    fn engage_select(
        &mut self,
        state: StateId,
        target: Option<u32>,
        arena: &mut Arena,
    ) -> Result<Option<u32>>;
    /// Kill state+10 (Mind+108, kill target GTID).
    fn kill_target(&mut self, mind: MindId) -> Result<u32>;
    /// `5351C0(mode, object)` on the kill state (not ported); its result is
    /// unused by `5358A0`.
    fn kill_select(
        &mut self,
        mind: MindId,
        mode: u32,
        object: u32,
        arena: &mut Arena,
    ) -> Result<()>;
    /// `52E9D0` on the engage state: return to the formation slot
    /// (movement-owned).
    fn return_to_slot(&mut self, state: StateId, arena: &mut Arena) -> Result<()>;
    /// One `85D840` draw on the world RNG (`4F02B0`).
    fn world_random(&mut self) -> Result<u32>;
    /// SVar `EngageReactionTimeRange` (binary32).
    fn engage_reaction_range(&mut self) -> Result<u32>;
}

/// `5358A0(object)` on the engage state for a non-null object (the only
/// form `536E10` reaches).
fn hand_to_kill(
    mind: MindId,
    object: u32,
    arena: &mut Arena,
    world: &mut impl EngageThinkWorld,
) -> Result<bool> {
    if status_at(world, mind, element::KILL, arena)? == 2
        && world.object_gtid(object)? == world.kill_target(mind)?
    {
        return Ok(false);
    }
    if status_at(world, mind, element::MOVE, arena)? == 2 {
        leave_if_active(world, mind, element::MOVE, arena)?;
    }
    world.kill_select(mind, 0, object, arena)?;
    Ok(true)
}

/// Leave an active kill state (`529C20`) and return to the slot (`52E9D0`).
fn give_up(
    state: StateId,
    mind: MindId,
    arena: &mut Arena,
    world: &mut impl EngageThinkWorld,
) -> Result<()> {
    if status_at(world, mind, element::KILL, arena)? == 2 {
        leave_if_active(world, mind, element::KILL, arena)?;
    }
    world.return_to_slot(state, arena)
}

/// Complete `536E10` at x87 precision `p`. Returns the base Think's
/// wrong-activity flag.
pub fn think(
    state: StateId,
    arena: &mut Arena,
    world: &mut impl EngageThinkWorld,
    p: Precision,
) -> Result<bool> {
    let wrong = think_base(state, arena)?;
    let mind = arena.state(state)?.parent;
    let organization = world.organization(mind)?;
    let gtid = world.engage_target(state)?;
    let target = world.resolve(gtid)?;
    if let (Some(t), Some(org)) = (target, organization)
        && !world.regiment_engage_keeps(org, t)?
    {
        world.set_reselect(state, true)?;
    }
    let kill_active = status_at(world, mind, element::KILL, arena)? == 2;
    if !kill_active {
        world.set_reselect(state, true)?;
    }
    if world.reselect(state)? {
        world.set_reselect(state, false)?;
        let Some(object) = world.engage_select(state, target, arena)? else {
            give_up(state, mind, arena, world)?;
            return Ok(wrong);
        };
        if hand_to_kill(mind, object, arena, world)? && !kill_active {
            let random = world.world_random()?;
            let range = world.engage_reaction_range()?;
            let delay = engage_reaction_delay(random, range, p)?;
            world.schedule(mind, delay, 0, arena)?;
            return Ok(wrong);
        }
    }
    let kill = world.kill_target(mind)?;
    if world.resolve(kill)?.is_some() {
        world.embedded_think(mind, element::KILL, arena)?;
        return Ok(wrong);
    }
    give_up(state, mind, arena, world)?;
    Ok(wrong)
}
