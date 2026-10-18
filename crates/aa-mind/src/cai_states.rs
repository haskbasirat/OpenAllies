//! Original small Mind-state Think dispatchers (slot +C of each state vtable).
//!
//! Each begins with the `KKC_TAIState::Think` base (`529E60`,
//! [`crate::think_base`]) and then delegates:
//! - `cai_retreat` `51E110`: embedded move state (Mind+68) Think.
//! - `uai_deploy` `52DF30`: embedded Element move state (Mind+7C) Think.
//! - `cai_regroup` `51DE10`: embedded sleep state (Mind+34) Think, then Mind
//!   virtual +150 when organization component+44 exists without flag bit 0
//!   at +8, else schedule 1.0 s.
//! - `cai_exhausted` `51E260`: when the Organization status (`578140`) is 2,
//!   finish this state with [`crate::pop`] if it is current (`52A000` on the
//!   Mind+270 state, which is this state), else
//!   schedule 1.0 s; otherwise the sleep state (Mind+34) Thinks.
//! - `cai_chasing` `523930`: `520D90(target)` true -> move state Think, false
//!   -> Mind virtual +140(target); target is state+10 (reread).
use crate::{Arena, MindId, Result, StateId, World, is_current, pop, think_base};

/// binary32 1.0 used by the regroup/exhausted retry schedules.
pub const ONE_SECOND_BITS: u32 = 0x3f80_0000;
/// Embedded state offsets used by these dispatchers.
pub const SLEEP_OFFSET: u32 = 0x34;
pub const REGIMENT_MOVE_OFFSET: u32 = 0x68;
pub const ELEMENT_MOVE_OFFSET: u32 = 0x7c;

/// Services in native order. `World::schedule` is the Mind Updatable (+8)
/// virtual +4 and `World` also drives [`crate::pop`].
pub trait StateThinkWorld: World {
    /// Virtual +C (Think) of the embedded state at Mind+`offset`.
    fn embedded_think(&mut self, mind: MindId, offset: u32, arena: &mut Arena) -> Result<()>;
    /// Organization (Mind actor) component+7C -> +44 object: `None` when null,
    /// else whether its +8 byte has bit 0 set.
    fn regroup_anchor_flag(&mut self, mind: MindId, arena: &Arena) -> Result<Option<bool>>;
    /// Mind virtual +150 (regroup continuation).
    fn mind_virtual_150(&mut self, mind: MindId, arena: &mut Arena) -> Result<()>;
    /// `578140` on the organization component+7C.
    fn organization_status(&mut self, mind: MindId, arena: &Arena) -> Result<u32>;
    /// State+10 word (chasing target).
    fn chase_target(&mut self, state: StateId, arena: &Arena) -> Result<u32>;
    /// `520D90(target)` on the chasing state.
    fn chase_continue(&mut self, state: StateId, target: u32, arena: &mut Arena) -> Result<bool>;
    /// Mind virtual +140(target).
    fn mind_virtual_140(&mut self, mind: MindId, target: u32, arena: &mut Arena) -> Result<()>;
}

fn mind_of(state: StateId, arena: &Arena) -> Result<MindId> {
    Ok(arena.state(state)?.parent)
}

/// `51E110`. Returns the base Think's wrong-activity flag.
pub fn retreat(
    state: StateId,
    arena: &mut Arena,
    world: &mut impl StateThinkWorld,
) -> Result<bool> {
    let wrong = think_base(state, arena)?;
    world.embedded_think(mind_of(state, arena)?, REGIMENT_MOVE_OFFSET, arena)?;
    Ok(wrong)
}

/// `52DF30` (Element).
pub fn deploy(state: StateId, arena: &mut Arena, world: &mut impl StateThinkWorld) -> Result<bool> {
    let wrong = think_base(state, arena)?;
    world.embedded_think(mind_of(state, arena)?, ELEMENT_MOVE_OFFSET, arena)?;
    Ok(wrong)
}

/// `51DE10`.
pub fn regroup(
    state: StateId,
    arena: &mut Arena,
    world: &mut impl StateThinkWorld,
) -> Result<bool> {
    let wrong = think_base(state, arena)?;
    let mind = mind_of(state, arena)?;
    world.embedded_think(mind, SLEEP_OFFSET, arena)?;
    let mind = mind_of(state, arena)?;
    if world.regroup_anchor_flag(mind, arena)? == Some(false) {
        world.mind_virtual_150(mind, arena)?;
    } else {
        world.schedule(mind, ONE_SECOND_BITS, 0, arena)?;
    }
    Ok(wrong)
}

/// `51E260`.
pub fn exhausted(
    state: StateId,
    arena: &mut Arena,
    world: &mut impl StateThinkWorld,
) -> Result<bool> {
    let wrong = think_base(state, arena)?;
    let mind = mind_of(state, arena)?;
    if world.organization_status(mind, arena)? == 2 {
        if is_current(state, arena)? {
            pop(mind, arena, world)?;
        } else {
            world.schedule(mind, ONE_SECOND_BITS, 0, arena)?;
        }
    } else {
        world.embedded_think(mind, SLEEP_OFFSET, arena)?;
    }
    Ok(wrong)
}

/// `523930`.
pub fn chasing(
    state: StateId,
    arena: &mut Arena,
    world: &mut impl StateThinkWorld,
) -> Result<bool> {
    let wrong = think_base(state, arena)?;
    let target = world.chase_target(state, arena)?;
    if world.chase_continue(state, target, arena)? {
        world.embedded_think(mind_of(state, arena)?, REGIMENT_MOVE_OFFSET, arena)?;
    } else {
        let target = world.chase_target(state, arena)?;
        world.mind_virtual_140(mind_of(state, arena)?, target, arena)?;
    }
    Ok(wrong)
}
