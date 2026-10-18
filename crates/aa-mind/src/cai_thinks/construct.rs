//! Original `KKC_CAIStateConstruct::Think` (`51DB80`, regiment state
//! `cai_construct` at Mind+134, vtable A32A38 slot +C).
//!
//! Order (the diagnostic log line is omitted):
//! 1. `KKC_TAIState::Think` base (`529E60`).
//! 2. The organization's (`529C30`) entry 4 of its +148 array when its +14C
//!    count is (signed) greater than 4, else 0, and the construction point
//!    (state+18/+1C, binary32 words passed unchanged) go to `4F2420` on the
//!    map singleton `[B36534]`.
//! 3. Refused: state+10 (byte) = 1 and the move state (Mind+68) thinks (tail
//!    jump to its virtual +C).
//! 4. Accepted: state+10 = 0, then `57ACC0(state+14, &state+18, &state+20,
//!    state+28)` on the organization component; a zero result goes to Mind
//!    virtual +174, anything else to Mind virtual +170(result).
use super::{MOVE_OFFSET, RegimentWorld};
use crate::{Arena, MindId, Result, StateId, think_base};

/// Services in native order.
pub trait ConstructThinkWorld: RegimentWorld {
    /// Construct state+18/+1C (binary32 point words).
    fn construct_point(&mut self, state: StateId) -> Result<[u32; 2]>;
    /// Organization +14C (signed entry count).
    fn organization_count_14c(&mut self, mind: MindId) -> Result<i32>;
    /// Organization +148 array entry `index`.
    fn organization_entry_148(&mut self, mind: MindId, index: u32) -> Result<u32>;
    /// `4F2420(entry, x, y)` on the map singleton `[B36534]`.
    fn map_4f2420(&mut self, entry: u32, point: [u32; 2], arena: &mut Arena) -> Result<bool>;
    /// Construct state+10 byte.
    fn set_construct_byte10(&mut self, state: StateId, value: bool) -> Result<()>;
    /// Construct state+14 and +28 words.
    fn construct_word14(&mut self, state: StateId) -> Result<u32>;
    fn construct_word28(&mut self, state: StateId) -> Result<u32>;
    /// `57ACC0(word14, &state+18, &state+20, word28)` on the organization
    /// component: the service sees the state's +18..+24 words by reference.
    fn component_57acc0(
        &mut self,
        mind: MindId,
        word14: u32,
        state: StateId,
        word28: u32,
        arena: &mut Arena,
    ) -> Result<u32>;
    /// Mind virtual +170(value).
    fn mind_virtual_170(&mut self, mind: MindId, value: u32, arena: &mut Arena) -> Result<()>;
    /// Mind virtual +174.
    fn mind_virtual_174(&mut self, mind: MindId, arena: &mut Arena) -> Result<()>;
}

/// Complete `51DB80`. Returns the base Think's wrong-activity flag.
pub fn think(
    state: StateId,
    arena: &mut Arena,
    world: &mut impl ConstructThinkWorld,
) -> Result<bool> {
    let wrong = think_base(state, arena)?;
    let mind = arena.state(state)?.parent;
    let point = world.construct_point(state)?;
    let entry = if world.organization_count_14c(mind)? > 4 {
        world.organization_entry_148(mind, 4)?
    } else {
        0
    };
    if !world.map_4f2420(entry, point, arena)? {
        world.set_construct_byte10(state, true)?;
        let mind = arena.state(state)?.parent;
        world.embedded_think(mind, MOVE_OFFSET, arena)?;
        return Ok(wrong);
    }
    world.set_construct_byte10(state, false)?;
    let word28 = world.construct_word28(state)?;
    let word14 = world.construct_word14(state)?;
    let result = world.component_57acc0(mind, word14, state, word28, arena)?;
    let mind = arena.state(state)?.parent;
    if result == 0 {
        world.mind_virtual_174(mind, arena)?;
    } else {
        world.mind_virtual_170(mind, result, arena)?;
    }
    Ok(wrong)
}
