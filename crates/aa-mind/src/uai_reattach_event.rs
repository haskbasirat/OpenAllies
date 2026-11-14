//! Original `KKC_UAI::EventReattach` (`52F160`, element Mind; UAI.cpp line
//! 0x1970): the element is told to rejoin its organization.
//!
//! Order of the original:
//! 1. An actor whose +8 flags have bit 0 or bit 29 set: false.
//! 2. The reattach state (Mind+178) is current (`52A000`): when Mind+C is
//!    0 the next update is scheduled in 0.6 s (Mind+8 virtual +4(0.6, 0));
//!    true.
//! 3. Otherwise (after the diagnostic "unit %d reattaching", omitted)
//!    `52D6D0(2)` on the reattach state ([`crate::state_helpers::activate_if_member`])
//!    and its result.
use crate::{
    Arena, MindId, Result, StateId, is_current,
    state_helpers::{MemberActivateWorld, activate_if_member},
};

/// Element Mind offset of the reattach state.
pub const REATTACH_OFFSET: u32 = 0x178;
/// binary32 0.6: the re-think delay of a current reattach state.
pub const REATTACH_DELAY_BITS: u32 = 0x3f19_999a;
/// Activation mode `52F160` passes to `52D6D0`.
pub const REATTACH_MODE: u32 = 2;

/// Services of `52F160` beyond [`MemberActivateWorld`].
pub trait ReattachEventWorld: MemberActivateWorld {
    /// The reattach state at Mind+178.
    fn reattach_state(&mut self, mind: MindId) -> Result<StateId>;
    /// Actor +8 flags.
    fn actor_flags8(&mut self, mind: MindId) -> Result<u32>;
    /// Mind+C (the Updatable word read before rescheduling).
    fn mind_word_c(&mut self, mind: MindId) -> Result<u32>;
}

/// Complete `52F160`.
pub fn event_reattach(
    mind: MindId,
    arena: &mut Arena,
    world: &mut impl ReattachEventWorld,
) -> Result<bool> {
    let flags = world.actor_flags8(mind)?;
    if flags & 1 != 0 || flags & (1 << 29) != 0 {
        return Ok(false);
    }
    let state = world.reattach_state(mind)?;
    if is_current(state, arena)? {
        if world.mind_word_c(mind)? == 0 {
            world.schedule(mind, REATTACH_DELAY_BITS, 0, arena)?;
        }
        return Ok(true);
    }
    activate_if_member(state, REATTACH_MODE, arena, world)
}
