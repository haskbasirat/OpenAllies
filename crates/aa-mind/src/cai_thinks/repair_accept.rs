//! Original repair-state accept `51ECF0(mode, object, keep)` (regiment state
//! `cai_repair` at Mind+160): whether the regiment takes a repair order on
//! `object`, called per order (the repair command `51F5A0` passes its mode,
//! target and the command's +30 byte).
//!
//! Order:
//! 1. A null object, object +8 bit 14 clear, the `529C00(mode)` eligibility
//!    (modes other than 0 and 1 ask state virtual +4) or organization
//!    (`529C30`) virtual +34(3) false refuse.
//! 2. The same order again (`52A000` current, state+14 equal to the object's
//!    +14 GTID and `keep` equal to state+18) is accepted: when the Mind +C
//!    word is zero the Mind update is scheduled in 0.6 s (`0x3F19999A`,
//!    data 0).
//! 3. An object with a +7C component: refused with `keep` set, when the
//!    organization owner (virtual +108) is short of upkeep (`44DD70`, owner
//!    +1F0 above 0.0), or when no non-null member of the component passes
//!    `4D1490` (`579F70`, iterator `4364C0` with the flag clear).
//! 4. Otherwise a zero +60 refuses; `keep` set, +100 bit 0x200000
//!    (`4C57F0`) or +100 bit 0x20000000 (`4D0940`) accepts; else the owner's
//!    upkeep shortage refuses.
//! 5. Accept: state+14 = the object's GTID (`4CF1B0`), state+18 = `keep`,
//!    and the result is the activation `52A810(mode)`.
use super::repair::ITERATION_END;
use crate::{Arena, MindId, Result, StateId, World, activate, is_current};
use aa_numeric::Binary;
use std::cmp::Ordering;

/// binary32 0.6: the re-think delay of an order repeated while current.
pub const REPEAT_DELAY_BITS: u32 = 0x3f19_999a;

/// Services in native order.
pub trait RepairAcceptWorld: World {
    /// Object +8.
    fn object_flags8(&mut self, object: u32) -> Result<u32>;
    /// Organization (`529C30`) virtual +34(value).
    fn organization_virtual_34(
        &mut self,
        mind: MindId,
        value: u32,
        arena: &mut Arena,
    ) -> Result<bool>;
    /// Repair state+14 (target GTID) and +18 byte.
    fn repair_target(&mut self, state: StateId) -> Result<u32>;
    fn repair_byte18(&mut self, state: StateId) -> Result<u8>;
    /// Store state+14 and then state+18.
    fn set_repair_order(&mut self, state: StateId, gtid: u32, byte18: u8) -> Result<()>;
    /// Object +14 (its GTID).
    fn object_gtid(&mut self, object: u32) -> Result<u32>;
    /// Mind +C word.
    fn mind_word_c(&mut self, mind: MindId) -> Result<u32>;
    /// Object +7C component, `None` when null.
    fn object_component(&mut self, object: u32) -> Result<Option<u32>>;
    /// Object +60 and +100 words.
    fn object_word60(&mut self, object: u32) -> Result<u32>;
    fn object_word100(&mut self, object: u32) -> Result<u32>;
    /// Organization virtual +108 (owner).
    fn organization_owner(&mut self, mind: MindId, arena: &mut Arena) -> Result<u32>;
    /// Owner +1F0 (binary32).
    fn owner_word1f0(&mut self, owner: u32) -> Result<u32>;
    /// `4364C0` on a member iterator of `component` at `index` (-1 before the
    /// first): the next index with a non-null member (with `flagged`, only a
    /// member whose +80 object's +1C byte is set), or [`ITERATION_END`].
    fn component_next_index(&mut self, component: u32, index: u32, flagged: bool) -> Result<u32>;
    /// Component +28 array entry `index`.
    fn component_member(&mut self, component: u32, index: u32) -> Result<u32>;
    /// `4D1490` on a member.
    fn member_4d1490(&mut self, member: u32, arena: &mut Arena) -> Result<bool>;
}

/// Native `44DD70`: owner +1F0 above 0.0 (`fcomp`, `test ah, 41`).
pub fn upkeep_short(word1f0: u32) -> Result<bool> {
    let value = Binary::from_f32_bits(word1f0).map_err(|e| crate::Error::World(e.to_string()))?;
    Ok(value.compare(&Binary::from_u32(0)) == Ordering::Greater)
}

fn owner_short(mind: MindId, arena: &mut Arena, world: &mut impl RepairAcceptWorld) -> Result<bool> {
    let owner = world.organization_owner(mind, arena)?;
    upkeep_short(world.owner_word1f0(owner)?)
}

/// Native `579F70` on a component: some non-null member passes `4D1490`.
pub fn has_ready_member(
    component: u32,
    arena: &mut Arena,
    world: &mut impl RepairAcceptWorld,
) -> Result<bool> {
    let mut index = world.component_next_index(component, u32::MAX, false)?;
    while index != ITERATION_END {
        let member = world.component_member(component, index)?;
        if member != 0 && world.member_4d1490(member, arena)? {
            return Ok(true);
        }
        index = world.component_next_index(component, index, false)?;
    }
    Ok(false)
}

/// Complete `51ECF0` on the repair state.
pub fn accept(
    state: StateId,
    mode: u32,
    object: Option<u32>,
    keep: u8,
    arena: &mut Arena,
    world: &mut impl RepairAcceptWorld,
) -> Result<bool> {
    let Some(object) = object else {
        return Ok(false);
    };
    if (world.object_flags8(object)? >> 14) & 1 == 0 {
        return Ok(false);
    }
    // 529C00
    if mode != 0 && mode != 1 && !world.can_activate(state, arena)? {
        return Ok(false);
    }
    let mind = arena.state(state)?.parent;
    if !world.organization_virtual_34(mind, 3, arena)? {
        return Ok(false);
    }
    if is_current(state, arena)?
        && world.repair_target(state)? == world.object_gtid(object)?
        && keep == world.repair_byte18(state)?
    {
        if world.mind_word_c(mind)? == 0 {
            world.schedule(mind, REPEAT_DELAY_BITS, 0, arena)?;
        }
        return Ok(true);
    }
    if let Some(component) = world.object_component(object)? {
        if keep != 0 || owner_short(mind, arena, world)? || !has_ready_member(component, arena, world)? {
            return Ok(false);
        }
    } else {
        if world.object_word60(object)? == 0 {
            return Ok(false);
        }
        let bypass = keep != 0
            || world.object_word100(object)? & 0x0020_0000 != 0
            || world.object_word100(object)? & 0x2000_0000 != 0;
        if !bypass && owner_short(mind, arena, world)? {
            return Ok(false);
        }
    }
    let gtid = world.object_gtid(object)?;
    world.set_repair_order(state, gtid, keep)?;
    activate(state, mode, arena, world)
}
