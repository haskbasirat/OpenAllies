//! Original `KKC_CAIStateKill::Think` (`520730`, regiment state `cai_kill` at
//! Mind+C0, vtable A32B5C slot +C).
//!
//! Order (the diagnostic log line is omitted):
//! 1. `KKC_TAIState::Think` base (`529E60`).
//! 2. The target GTID (state+10) is resolved through the GTID registry (no
//!    removed check). A resolved target that the organization component
//!    accepts (`57A500(target, 0, 0)`) is pursued: while current (`52A000`)
//!    the siege state (Mind+F8) is activated for it in mode 1 (`51DA20`,
//!    [`super::siege_target::activate_for`]); unless that succeeded the move
//!    state (Mind+68) thinks (a tail jump to its virtual +C).
//! 3. Otherwise Mind virtual +140(state+10, reread).
use super::{
    MOVE_OFFSET, SIEGE_OFFSET,
    siege_target::{SiegeTargetWorld, activate_for},
};
use crate::{Arena, Result, StateId, is_current, think_base};

/// Services in native order.
pub trait KillThinkWorld: SiegeTargetWorld {
    /// Kill state+10 (target GTID).
    fn kill_target(&mut self, state: StateId) -> Result<u32>;
}

/// Complete `520730`. Returns the base Think's wrong-activity flag.
pub fn think(state: StateId, arena: &mut Arena, world: &mut impl KillThinkWorld) -> Result<bool> {
    let wrong = think_base(state, arena)?;
    let mind = arena.state(state)?.parent;
    let gtid = world.kill_target(state)?;
    if let Some(target) = world.resolve(gtid)?
        && world.component_57a500(mind, target, false, false, arena)?
    {
        if is_current(state, arena)? {
            let siege = world.embedded_state(mind, SIEGE_OFFSET)?;
            if activate_for(siege, 1, Some(target), arena, world)? {
                return Ok(wrong);
            }
        }
        let mind = arena.state(state)?.parent;
        world.embedded_think(mind, MOVE_OFFSET, arena)?;
        return Ok(wrong);
    }
    let gtid = world.kill_target(state)?;
    world.mind_virtual_140(mind, gtid, arena)?;
    Ok(wrong)
}
