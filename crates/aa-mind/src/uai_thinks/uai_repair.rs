//! Original `KKC_UAIStateRepair::Think` (`5359D0`, element state
//! `uai_repair` at Mind+148, vtable A34B68 slot +C): repair the object whose
//! GTID is state+14.
//!
//! Order (the native diagnostic line is omitted):
//! 1. `529E60` ([`crate::think_base`]). The state+14 GTID is resolved.
//! 2. Unless state+18 is set: a live target with a body (+60) whose `5617C0`
//!    holds (fully repaired) goes to Mind virtual +168, done.
//! 3. A set state+10 byte is cleared and Mind virtual +B8 runs.
//! 4. No target or no body, or `4D4FF0(target, 1, 0)` on the actor nonzero:
//!    Mind virtual +140(state+14), done.
//! 5. `5340C0(target, null, &record, 0)` ([`crate::uai_kill::choose`]) on a
//!    record whose only initialised words are ability (+0) = 0 and status
//!    (+8) = 3; the Think reads back only those two words.
//!    - status 0: `57F290(ability, 1, target, 0)` on the actor performer
//!      (+74); true cancels the Mind's pending updates (`50B250` on Mind+8),
//!      false goes to Mind virtual +140.
//!    - status 3: Mind virtual +140.
//!    - status 1: `52D660` on the move state (stop action, movement-owned),
//!      then 0.6 s is scheduled.
//!    - otherwise the move state (Mind+7C) Thinks; when the actor's current
//!      performer (+E4) is its locomotion (+6C), the locomotion follows the
//!      target (`576AD0`) with `4C8270(4CF110(actor))` of the ability minus
//!      `[A34770]` (1e-4), x87 at the control-word precision, stored binary32.
use super::{IMMEDIATE_BITS, POINT_SIX_BITS, ThinkWorld, element, null};
use crate::{
    Arena, MindId, Result, StateId, think_base,
    x87::{n, num, store},
};
use aa_numeric::{Binary, Precision};

/// The two `5340C0` record words read back by `5359D0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RepairChoice {
    /// Record +0 (ability), `None` when null.
    pub ability: Option<u32>,
    /// Record +8 (status, the value returned by `5340C0`).
    pub status: u32,
}

/// Services of `5359D0` in native order.
pub trait RepairThinkWorld: ThinkWorld {
    /// Repair state+14 (target GTID), +18 byte (keep repairing) and +10 byte
    /// (notify) with its write.
    fn repair_target(&mut self, state: StateId) -> Result<u32>;
    fn keep_repairing(&mut self, state: StateId) -> Result<bool>;
    fn notify(&mut self, state: StateId) -> Result<bool>;
    fn set_notify(&mut self, state: StateId, value: bool) -> Result<()>;
    /// Object +60 (body), `None` when null.
    fn body(&mut self, object: u32) -> Result<Option<u32>>;
    /// `5617C0` on a body (health +10 not below +14).
    fn body_whole(&mut self, body: u32) -> Result<bool>;
    /// Mind virtuals +168 and +B8.
    fn mind_virtual_168(&mut self, mind: MindId, arena: &mut Arena) -> Result<()>;
    fn mind_virtual_b8(&mut self, mind: MindId, arena: &mut Arena) -> Result<()>;
    /// `4D4FF0(target, 1, 0)` on the actor (0 = allowed).
    fn relation(&mut self, mind: MindId, target: u32) -> Result<i32>;
    /// `5340C0(target, null, &record, 0)` on the Mind.
    fn choose(&mut self, mind: MindId, target: u32) -> Result<RepairChoice>;
    /// `57F290(ability, 1, target, 0)` on the actor performer (+74).
    fn performer_use(&mut self, mind: MindId, ability: u32, target: u32) -> Result<bool>;
    /// `50B250` on Mind+8 (cancel the pending updates).
    fn cancel_updates(&mut self, mind: MindId, arena: &mut Arena) -> Result<()>;
    /// `52D660` on the move state (stop action, movement-owned).
    fn move_stop(&mut self, mind: MindId, arena: &mut Arena) -> Result<()>;
    /// `4CF110` actor player.
    fn player(&mut self, mind: MindId) -> Result<u32>;
    /// `4C8270(player)` on an ability, unstored ST0.
    fn ability_value(&mut self, ability: u32, player: u32) -> Result<Binary>;
}

/// Complete `5359D0` at x87 precision `p`. Returns the base Think's
/// wrong-activity flag.
pub fn think(
    state: StateId,
    arena: &mut Arena,
    world: &mut impl RepairThinkWorld,
    p: Precision,
) -> Result<bool> {
    let wrong = think_base(state, arena)?;
    let mind = arena.state(state)?.parent;
    let gtid = world.repair_target(state)?;
    let target = world.resolve(gtid)?;
    if !world.keep_repairing(state)?
        && let Some(t) = target
        && let Some(body) = world.body(t)?
        && world.body_whole(body)?
    {
        world.mind_virtual_168(mind, arena)?;
        return Ok(wrong);
    }
    if world.notify(state)? {
        world.set_notify(state, false)?;
        world.mind_virtual_b8(mind, arena)?;
    }
    let Some(target) = target else {
        let gtid = world.repair_target(state)?;
        world.mind_virtual_140(mind, gtid, arena)?;
        return Ok(wrong);
    };
    if world.body(target)?.is_none() || world.relation(mind, target)? != 0 {
        let gtid = world.repair_target(state)?;
        world.mind_virtual_140(mind, gtid, arena)?;
        return Ok(wrong);
    }
    let choice = world.choose(mind, target)?;
    match choice.status {
        0 => {
            let ability = choice
                .ability
                .ok_or_else(|| null("57F290 reads a null ability"))?;
            if world.performer_use(mind, ability, target)? {
                world.cancel_updates(mind, arena)?;
                return Ok(wrong);
            }
        }
        3 => {}
        1 => {
            world.move_stop(mind, arena)?;
            world.schedule(mind, POINT_SIX_BITS, 0, arena)?;
            return Ok(wrong);
        }
        _ => {
            world.embedded_think(mind, element::MOVE, arena)?;
            if world.current_performer(mind)? == world.locomotion(mind)? {
                let player = world.player(mind)?;
                let ability = choice
                    .ability
                    .ok_or_else(|| null("4C8270 on a null ability"))?;
                let value = world.ability_value(ability, player)?;
                let delay = store(num(value.sub(&n(IMMEDIATE_BITS)?, p))?)?;
                let locomotion = world
                    .locomotion(mind)?
                    .ok_or_else(|| null("576AD0 on a null locomotion"))?;
                world.locomotion_follow(locomotion, target, delay)?;
            }
            return Ok(wrong);
        }
    }
    let gtid = world.repair_target(state)?;
    world.mind_virtual_140(mind, gtid, arena)?;
    Ok(wrong)
}
