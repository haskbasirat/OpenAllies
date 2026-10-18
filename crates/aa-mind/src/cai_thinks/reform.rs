//! Original `KKC_CAIStateReform::Think` (`51DE70`, regiment state
//! `cai_reform` at Mind+1B8, vtable A32A80 slot +C).
//!
//! Order (the diagnostic log line is omitted):
//! 1. `KKC_TAIState::Think` base (`529E60`).
//! 2. While current (`52A000`): the game time (`[B36534]+E0`, binary32) minus
//!    the reform start (state+10, binary32), computed at the live x87
//!    precision, not below the reform timeout (`[[B39AB0]+B4]`, binary32;
//!    unordered counts as below) ends the reform through Mind virtual +178.
//! 3. Schedule the next update in 2.0 s.
//! 4. A sleep point (Mind+2B4/+2B8) different from the null point
//!    (`[B279C0]/[B279C4]`, unordered counts as different) is sent to the
//!    organization component (`57BF30`) and reset to the null point (y copied
//!    through the x87 stack, x moved as a word).
//! 5. `4CEDE0` on the organization component's +40 object: false goes to
//!    Mind virtual +11C, true schedules the next update in 1.0 s.
//!
//! Non-finite time, start or timeout values are outside the exact x87 model
//! and return an error instead of a guessed comparison.
use super::{RegimentWorld, x87_copy};
use crate::{
    Arena, MindId, Result, StateId, is_current, think_base,
    x87::{n, num},
};
use aa_numeric::Precision;
use std::cmp::Ordering;

/// binary32 2.0: reform re-think delay.
pub const REFORM_DELAY_BITS: u32 = 0x4000_0000;
/// binary32 1.0: delay once the component +40 object passes `4CEDE0`.
pub const SETTLED_DELAY_BITS: u32 = 0x3f80_0000;

/// Services in native order.
pub trait ReformThinkWorld: RegimentWorld {
    /// Map singleton `[B36534]` +E0 (binary32 game time).
    fn game_time(&mut self) -> Result<u32>;
    /// Reform state+10 (binary32 start time).
    fn reform_start(&mut self, state: StateId) -> Result<u32>;
    /// SVar table `[B39AB0]` +B4 -> binary32 reform timeout.
    fn reform_timeout(&mut self) -> Result<u32>;
    /// Mind virtual +178.
    fn mind_virtual_178(&mut self, mind: MindId, arena: &mut Arena) -> Result<()>;
    /// Mind+2B4/+2B8 (binary32).
    fn sleep_point(&mut self, mind: MindId) -> Result<[u32; 2]>;
    fn set_sleep_point(&mut self, mind: MindId, point: [u32; 2]) -> Result<()>;
    /// `[B279C0]/[B279C4]`.
    fn null_point(&mut self) -> Result<[u32; 2]>;
    /// `57BF30(x, y)` on the organization component.
    fn organization_move(&mut self, mind: MindId, point: [u32; 2], arena: &mut Arena)
    -> Result<()>;
    /// Organization component +40 word.
    fn component_word40(&mut self, mind: MindId) -> Result<u32>;
    /// `4CEDE0` on that object.
    fn check_4cede0(&mut self, object: u32, arena: &mut Arena) -> Result<bool>;
    /// Mind virtual +11C.
    fn mind_virtual_11c(&mut self, mind: MindId, arena: &mut Arena) -> Result<()>;
}

fn host(bits: u32) -> f32 {
    f32::from_bits(bits)
}

/// Complete `51DE70` at x87 precision `p`. Returns the base Think's
/// wrong-activity flag.
pub fn think(
    state: StateId,
    arena: &mut Arena,
    world: &mut impl ReformThinkWorld,
    p: Precision,
) -> Result<bool> {
    let wrong = think_base(state, arena)?;
    let mind = arena.state(state)?.parent;
    if is_current(state, arena)? {
        let time = world.game_time()?;
        let start = world.reform_start(state)?;
        let elapsed = num(n(time)?.sub(&n(start)?, p))?;
        let timeout = world.reform_timeout()?;
        if elapsed.compare(&n(timeout)?) != Ordering::Less {
            world.mind_virtual_178(mind, arena)?;
            return Ok(wrong);
        }
    }
    world.schedule(mind, REFORM_DELAY_BITS, 0, arena)?;
    let mind = arena.state(state)?.parent;
    let point = world.sleep_point(mind)?;
    let null = world.null_point()?;
    if host(point[0]) != host(null[0]) || host(point[1]) != host(null[1]) {
        world.organization_move(mind, point, arena)?;
        let null = world.null_point()?;
        world.set_sleep_point(mind, [null[0], x87_copy(null[1])])?;
    }
    let object = world.component_word40(mind)?;
    if world.check_4cede0(object, arena)? {
        world.schedule(mind, SETTLED_DELAY_BITS, 0, arena)?;
    } else {
        world.mind_virtual_11c(mind, arena)?;
    }
    Ok(wrong)
}
