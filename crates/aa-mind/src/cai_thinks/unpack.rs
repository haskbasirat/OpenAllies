//! Original `KKC_CAIStateUnpack::Think` (`526E60`, regiment state
//! `cai_unpack` at Mind+280, vtable A32B10 slot +C) with the Tai action
//! constructor `51E800`.
//!
//! Order (the diagnostic log line is omitted):
//! 1. `KKC_TAIState::Think` base (`529E60`); without a unit (organization
//!    component +44) the Think ends.
//! 2. An active move state (Mind+68) thinks instead.
//! 3. An inactive sleep state is activated in mode 0 (`52A810`).
//! 4. A unit whose Mind (+70) virtual +7C or `4D1050` holds is settled: the
//!    sleep state thinks and the next update is scheduled in 1.0 s.
//! 5. A unit passing `4D1080` goes to Mind virtual +1B0(1).
//! 6. With fewer than two attempts (state+20, signed) the unpack action
//!    (`51E800`: ordinal 0xB with the state+10..+1C points) is delivered
//!    (`4D9B60`); accepted counts an attempt and continues as in 4.
//!    Otherwise `tai_error_unpack|s` is posted and Mind virtual +1B0(0).
use super::{
    MOVE_OFFSET, SLEEP_OFFSET,
    stack_action::{ActionCommand, UNPACK_ACTION},
    unit_order::{UNPACK_ERROR_KEY, UnitOrderWorld, report_error},
};
use crate::{Arena, Error, MindId, Result, StateId, activate, think_base};

/// binary32 1.0: re-think delay while unpacking.
pub const UNPACK_DELAY_BITS: u32 = 0x3f80_0000;
/// Attempts (state+20) allowed before the error.
pub const MAX_ATTEMPTS: i32 = 2;

/// Services in native order.
pub trait UnpackThinkWorld: UnitOrderWorld {
    /// Unit +70 (its Mind), `None` when null.
    fn unit_mind(&mut self, unit: u32) -> Result<Option<u32>>;
    /// Unit Mind virtual +7C.
    fn unit_mind_virtual_7c(&mut self, unit_mind: u32, arena: &mut Arena) -> Result<bool>;
    /// `4D1050` on the unit.
    fn unit_4d1050(&mut self, unit: u32, arena: &mut Arena) -> Result<bool>;
    /// Unpack state+10..+1C (two binary32 points).
    fn unpack_points(&mut self, state: StateId) -> Result<[u32; 4]>;
    /// Unpack state+20 (attempt count).
    fn unpack_attempts(&mut self, state: StateId) -> Result<u32>;
    fn set_unpack_attempts(&mut self, state: StateId, value: u32) -> Result<()>;
    /// Mind virtual +1B0(flag).
    fn mind_virtual_1b0(&mut self, mind: MindId, flag: u32, arena: &mut Arena) -> Result<()>;
}

/// Complete `526E60`. Returns the base Think's wrong-activity flag. A null
/// unit Mind is the native null dereference.
pub fn think(state: StateId, arena: &mut Arena, world: &mut impl UnpackThinkWorld) -> Result<bool> {
    let wrong = think_base(state, arena)?;
    let mind = arena.state(state)?.parent;
    let Some(unit) = world.component_unit(mind)? else {
        return Ok(wrong);
    };
    let moving = world.embedded_state(mind, MOVE_OFFSET)?;
    if arena.state(moving)?.status == 2 {
        world.embedded_think(mind, MOVE_OFFSET, arena)?;
        return Ok(wrong);
    }
    let sleep = world.embedded_state(mind, SLEEP_OFFSET)?;
    if arena.state(sleep)?.status != 2 {
        activate(sleep, 0, arena, world)?;
    }
    let unit_mind = world
        .unit_mind(unit)?
        .ok_or_else(|| Error::World("null unit Mind dereference".into()))?;
    let settled =
        world.unit_mind_virtual_7c(unit_mind, arena)? || world.unit_4d1050(unit, arena)?;
    if !settled {
        if world.unit_4d1080(unit, arena)? {
            world.mind_virtual_1b0(mind, 1, arena)?;
            return Ok(wrong);
        }
        let mut issued = false;
        if (world.unpack_attempts(state)? as i32) < MAX_ATTEMPTS {
            let points = world.unpack_points(state)?;
            let action = ActionCommand::unpack(world.action_definition(UNPACK_ACTION)?, points);
            issued = world.issue_action(unit, &action, arena)?;
            world.release_action(&action, arena)?;
        }
        if !issued {
            report_error(mind, UNPACK_ERROR_KEY, arena, world)?;
            world.mind_virtual_1b0(mind, 0, arena)?;
            return Ok(wrong);
        }
        let attempts = world.unpack_attempts(state)?;
        world.set_unpack_attempts(state, attempts.wrapping_add(1))?;
    }
    world.embedded_think(mind, SLEEP_OFFSET, arena)?;
    world.schedule(mind, UNPACK_DELAY_BITS, 0, arena)?;
    Ok(wrong)
}
