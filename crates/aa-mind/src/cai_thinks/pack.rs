//! Original `KKC_CAIStatePack::Think` (`527090`, regiment state `cai_pack` at
//! Mind+2A4, vtable A32B34 slot +C).
//!
//! Order (the diagnostic log line is omitted):
//! 1. `KKC_TAIState::Think` base (`529E60`); without a unit (organization
//!    component +44) the Think ends.
//! 2. A unit with flag 0x2000000 at +100 is packed: the sleep state thinks
//!    and the next update is scheduled in 1.0 s.
//! 3. A unit failing `4D1080` goes to Mind virtual +1B4.
//! 4. Otherwise the pack action (ordinal 0xC) is delivered (`4D9B60`):
//!    accepted continues as in 2; refused posts `tai_error_pack|s` and goes
//!    to Mind virtual +1B4.
use super::{
    RegimentWorld, SLEEP_OFFSET,
    stack_action::{ActionCommand, PACK_ACTION},
    unit_order::{PACK_ERROR_KEY, UnitOrderWorld, report_error},
};
use crate::{Arena, MindId, Result, StateId, think_base};

/// binary32 1.0: re-think delay once packed or packing.
pub const PACK_DELAY_BITS: u32 = 0x3f80_0000;
/// Unit +100 flag: packed.
pub const PACKED_FLAG: u32 = 0x0200_0000;

/// Services in native order.
pub trait PackThinkWorld: UnitOrderWorld {
    /// Unit +100 word.
    fn unit_word100(&mut self, unit: u32) -> Result<u32>;
    /// Mind virtual +1B4.
    fn mind_virtual_1b4(&mut self, mind: MindId, arena: &mut Arena) -> Result<()>;
}

fn settle(mind: MindId, arena: &mut Arena, world: &mut impl RegimentWorld) -> Result<()> {
    world.embedded_think(mind, SLEEP_OFFSET, arena)?;
    world.schedule(mind, PACK_DELAY_BITS, 0, arena)
}

/// Complete `527090`. Returns the base Think's wrong-activity flag.
pub fn think(state: StateId, arena: &mut Arena, world: &mut impl PackThinkWorld) -> Result<bool> {
    let wrong = think_base(state, arena)?;
    let mind = arena.state(state)?.parent;
    let Some(unit) = world.component_unit(mind)? else {
        return Ok(wrong);
    };
    if world.unit_word100(unit)? & PACKED_FLAG == 0 {
        if !world.unit_4d1080(unit, arena)? {
            world.mind_virtual_1b4(mind, arena)?;
            return Ok(wrong);
        }
        let action = ActionCommand::new(PACK_ACTION, world.action_definition(PACK_ACTION)?);
        let issued = world.issue_action(unit, &action, arena)?;
        world.release_action(&action, arena)?;
        if !issued {
            report_error(mind, PACK_ERROR_KEY, arena, world)?;
            world.mind_virtual_1b4(mind, arena)?;
            return Ok(wrong);
        }
    }
    settle(mind, arena, world)?;
    Ok(wrong)
}
