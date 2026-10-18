//! Original element state `uai_join` (Mind+164, vtable A34B8C):
//! `KKC_UAIStateJoin::Think` (`531AD0`, slot +C) and its activation helper
//! `52EAE0(mode, object)`.
//!
//! `531AD0` order (no diagnostics in this function):
//! 1. `529E60` ([`crate::think_base`]).
//! 2. The state+10 GTID is resolved in the registry `[B364C0]` (generation
//!    match and non-null entry). Unresolved: Mind virtual +140(GTID), done.
//! 3. The embedded move state (Mind+7C) Thinks.
//! 4. When the actor's current performer (+E4) is its locomotion (+6C) and
//!    state+10 (reread) still resolves, `576AD0(object, 1e-4)` runs on the
//!    locomotion; a null locomotion is a native null `this`.
use super::{IMMEDIATE_BITS, ThinkWorld, element, null};
use crate::{Arena, Result, StateId, activate, think_base};

/// Services of `52EAE0`.
pub trait JoinActivateWorld: ThinkWorld {
    /// Join state+10 write (GTID of the object to join).
    fn set_join_target(&mut self, state: StateId, gtid: u32) -> Result<()>;
}

/// `52EAE0(mode, object)` on the join state: state+10 becomes the object's
/// GTID (0 for none); a GTID that resolves while the actor has a locomotion
/// activates the state in `mode` (`52A810`, [`crate::activate`]).
pub fn activate_join(
    state: StateId,
    mode: u32,
    object: Option<u32>,
    arena: &mut Arena,
    world: &mut impl JoinActivateWorld,
) -> Result<bool> {
    let gtid = match object {
        Some(o) => world.object_gtid(o)?,
        None => 0,
    };
    world.set_join_target(state, gtid)?;
    if world.resolve(gtid)?.is_none() {
        return Ok(false);
    }
    let mind = arena.state(state)?.parent;
    if world.locomotion(mind)?.is_none() {
        return Ok(false);
    }
    activate(state, mode, arena, world)
}

/// Services of `531AD0` in native order.
pub trait JoinThinkWorld: JoinActivateWorld {
    /// Join state+10 (GTID of the object to join).
    fn join_target(&mut self, state: StateId) -> Result<u32>;
}

/// Complete `531AD0`. Returns the base Think's wrong-activity flag.
pub fn think(state: StateId, arena: &mut Arena, world: &mut impl JoinThinkWorld) -> Result<bool> {
    let wrong = think_base(state, arena)?;
    let mind = arena.state(state)?.parent;
    let gtid = world.join_target(state)?;
    if world.resolve(gtid)?.is_none() {
        world.mind_virtual_140(mind, gtid, arena)?;
        return Ok(wrong);
    }
    world.embedded_think(mind, element::MOVE, arena)?;
    if world.current_performer(mind)? != world.locomotion(mind)? {
        return Ok(wrong);
    }
    let gtid = world.join_target(state)?;
    if let Some(object) = world.resolve(gtid)? {
        let locomotion = world
            .locomotion(mind)?
            .ok_or_else(|| null("576AD0 on a null locomotion"))?;
        world.locomotion_follow(locomotion, object, IMMEDIATE_BITS)?;
    }
    Ok(wrong)
}
