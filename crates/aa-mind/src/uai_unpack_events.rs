//! Original element unpack state (Mind+188, vtable A347E0) Enter and Leave
//! virtuals; the Leave is shared with the reattach state (Mind+178):
//!
//! * `52EB90` (+18 Enter, unpack): status 2; unless the actor is within 0.1
//!   of the unpack point state+10/+14 (`52A240`,
//!   [`crate::element_command::near_actor`]), the move state (Mind+7C) is
//!   ordered there with `52DF60(0, point, 1, 0, [B279C0])` ([`enter`]).
//! * `52EB50` (+1C Leave): an active (nonzero) move state is left
//!   (`529C20`), then status 0 ([`leave`]).
use crate::{
    Arena, MindId, Result, StateId, element_command::near_actor,
    uai_thinks::{ThinkWorld, element, leave_if_active},
};
use aa_numeric::Precision;

/// Services of `52EB90` beyond [`ThinkWorld`].
pub trait UnpackEnterWorld: ThinkWorld {
    /// Unpack state +10/+14 (binary32 point).
    fn unpack_point(&mut self, state: StateId) -> Result<[u32; 2]>;
    /// The Mind actor +20/+24 (binary32).
    fn actor_position(&mut self, mind: MindId) -> Result<[u32; 2]>;
    /// `52DF60(mode, x, y, a4, a5, null x, null y)` on the move state
    /// (Mind+7C), movement-owned; the result is unused here.
    fn move_order(&mut self, mind: MindId, args: [u32; 7], arena: &mut Arena) -> Result<()>;
}

/// `52EB90` on the unpack state.
pub fn enter(
    state: StateId,
    arena: &mut Arena,
    world: &mut impl UnpackEnterWorld,
    p: Precision,
) -> Result<()> {
    arena.state_mut(state)?.status = 2;
    let mind = arena.state(state)?.parent;
    let point = world.unpack_point(state)?;
    if near_actor(world.actor_position(mind)?, point, p)? {
        return Ok(());
    }
    let null = world.null_point()?;
    let point = world.unpack_point(state)?;
    world.move_order(mind, [0, point[0], point[1], 1, 0, null[0], null[1]], arena)
}

/// `52EB50` on the unpack or reattach state.
pub fn leave(state: StateId, arena: &mut Arena, world: &mut impl ThinkWorld) -> Result<()> {
    let mind = arena.state(state)?.parent;
    leave_if_active(world, mind, element::MOVE, arena)?;
    arena.state_mut(state)?.status = 0;
    Ok(())
}
