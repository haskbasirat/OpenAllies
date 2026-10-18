//! Original `KKC_AAIStateRecon::Think` (`5180B0`, aircraft state `aai_recon`
//! at Mind+B4, vtable A324D8 slot +C).
//!
//! Order: `529E60` ([`crate::think_base`]); a set state+10 byte activates
//! the aircraft join state in mode 2 (`5176A0`) and schedules 1e-4 s;
//! otherwise the aircraft move state (Mind+34) Thinks and 0.6 s is scheduled.
use super::{AircraftWorld, IMMEDIATE_BITS, POINT_SIX_BITS, aircraft};
use crate::{Arena, Result, StateId, think_base};

/// Services of `5180B0`.
pub trait ReconThinkWorld: AircraftWorld {
    /// Recon state+10 byte (return to the formation).
    fn recon_done(&mut self, state: StateId) -> Result<bool>;
}

/// Complete `5180B0`. Returns the base Think's wrong-activity flag.
pub fn think(state: StateId, arena: &mut Arena, world: &mut impl ReconThinkWorld) -> Result<bool> {
    let wrong = think_base(state, arena)?;
    let mind = arena.state(state)?.parent;
    if world.recon_done(state)? {
        world.join_activate(mind, 2, arena)?;
        world.schedule(mind, IMMEDIATE_BITS, 0, arena)?;
    } else {
        world.embedded_think(mind, aircraft::MOVE, arena)?;
        world.schedule(mind, POINT_SIX_BITS, 0, arena)?;
    }
    Ok(wrong)
}
