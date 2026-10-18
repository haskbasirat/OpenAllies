//! Original `KKC_CAIStateJoin::Think` (`521850`, regiment state `cai_join` at
//! Mind+24C, vtable A32EC4 slot +C) with the Tai helper `5217C0`.
//!
//! Order (diagnostic log lines omitted):
//! 1. `KKC_TAIState::Think` base (`529E60`). A target (state+10 GTID) that
//!    does not resolve to a non-null registry entry goes to Mind virtual
//!    +140(GTID).
//! 2. Not arrived (state+14 byte clear): the organization's entry 4 (+148
//!    array when the signed +14C count exceeds 4, else 0) is offered to the
//!    target's virtual +130(entry, 0); true ends the join through Mind
//!    virtual +144. Otherwise an inactive move state (Mind+68) is ordered to
//!    the target (`520320(0, target, 0, null point)`, the target re-resolved
//!    through `432E40`) and the move state thinks.
//! 3. Arrived: the organization component's members are walked with the
//!    `4364C0` iterator (non-null entries in index order). Gathering
//!    (state+15 byte): an inactive move state is ordered as in 2; each member
//!    whose GTID is not in the state+18 list (signed count +1C) has its Mind
//!    (+70) virtual +F4(target re-resolved); when every member was listed
//!    (or there are none) Mind virtual +16C ends the join, else the next
//!    update is scheduled in 1.0 s. Not gathering: no members goes to Mind
//!    virtual +16C; otherwise every member's Mind gets +F4(target) (`5217C0`)
//!    and the next update is scheduled in 1.0 s.
use super::{MOVE_OFFSET, RegimentWorld};
use crate::{Arena, Error, MindId, Result, StateId, think_base};

/// binary32 1.0: re-think delay while members join.
pub const JOIN_DELAY_BITS: u32 = 0x3f80_0000;
/// `4364C0` end-of-iteration index.
pub const ITERATION_END: u32 = 0x7fff_ffff;

/// Services in native order.
pub trait JoinThinkWorld: RegimentWorld {
    /// Join state+10 (target GTID).
    fn join_target(&mut self, state: StateId) -> Result<u32>;
    /// Join state+14 (arrived) and +15 (gathering) bytes.
    fn join_arrived(&mut self, state: StateId) -> Result<bool>;
    fn join_gathering(&mut self, state: StateId) -> Result<bool>;
    /// Join state+18 GTID list with its signed +1C count (a count <= 0
    /// lists nothing).
    fn join_listed(&mut self, state: StateId) -> Result<Vec<u32>>;
    /// Organization +14C (signed entry count).
    fn organization_count_14c(&mut self, mind: MindId) -> Result<i32>;
    /// Organization +148 array entry `index`.
    fn organization_entry_148(&mut self, mind: MindId, index: u32) -> Result<u32>;
    /// Target virtual +130(entry, flag).
    fn target_virtual_130(
        &mut self,
        target: u32,
        entry: u32,
        flag: bool,
        arena: &mut Arena,
    ) -> Result<bool>;
    /// Mind virtual +144.
    fn mind_virtual_144(&mut self, mind: MindId, arena: &mut Arena) -> Result<()>;
    /// Mind virtual +16C.
    fn mind_virtual_16c(&mut self, mind: MindId, arena: &mut Arena) -> Result<()>;
    /// `[B279C0]/[B279C4]`.
    fn null_point(&mut self) -> Result<[u32; 2]>;
    /// `520320` on the move state with its five stack words (the target
    /// entry is the second).
    fn move_order(&mut self, mind: MindId, args: [u32; 5], arena: &mut Arena) -> Result<()>;
    /// `4364C0` on an organization component member iterator (skip flag
    /// clear) positioned at `index` (-1 before the first): the next index
    /// with a non-null member, or [`ITERATION_END`].
    fn next_member_index(&mut self, mind: MindId, index: u32) -> Result<u32>;
    /// Organization component +28 array entry `index`.
    fn component_member(&mut self, mind: MindId, index: u32) -> Result<u32>;
    /// Member +70 (its Mind), `None` when null.
    fn member_mind(&mut self, member: u32) -> Result<Option<u32>>;
    /// Member Mind virtual +F4(target entry or 0).
    fn member_mind_virtual_f4(
        &mut self,
        member_mind: u32,
        target: u32,
        arena: &mut Arena,
    ) -> Result<()>;
}

fn null_member() -> Error {
    Error::World("null member Mind dereference".into())
}

/// The `432E40` re-resolution of state+10 (0 for a stale GTID).
fn target_entry(state: StateId, world: &mut impl JoinThinkWorld) -> Result<u32> {
    let gtid = world.join_target(state)?;
    Ok(world.resolve(gtid)?.unwrap_or(0))
}

/// The inactive-move order of steps 2 and 3.
fn order_move(state: StateId, arena: &mut Arena, world: &mut impl JoinThinkWorld) -> Result<()> {
    let mind = arena.state(state)?.parent;
    let moving = world.embedded_state(mind, MOVE_OFFSET)?;
    if arena.state(moving)?.status == 2 {
        return Ok(());
    }
    let null = world.null_point()?;
    let target = target_entry(state, world)?;
    world.move_order(mind, [0, target, 0, null[0], null[1]], arena)
}

/// `5217C0` on the join state: every member's Mind gets +F4(target).
pub fn join_all(state: StateId, arena: &mut Arena, world: &mut impl JoinThinkWorld) -> Result<()> {
    let mind = arena.state(state)?.parent;
    let mut index = world.next_member_index(mind, u32::MAX)?;
    while index != ITERATION_END {
        let member = world.component_member(mind, index)?;
        let member_mind = world.member_mind(member)?.ok_or_else(null_member)?;
        let target = target_entry(state, world)?;
        world.member_mind_virtual_f4(member_mind, target, arena)?;
        index = world.next_member_index(mind, index)?;
    }
    Ok(())
}

/// Complete `521850`. Returns the base Think's wrong-activity flag. A null
/// member Mind is the native null dereference.
pub fn think(state: StateId, arena: &mut Arena, world: &mut impl JoinThinkWorld) -> Result<bool> {
    let wrong = think_base(state, arena)?;
    let mind = arena.state(state)?.parent;
    let gtid = world.join_target(state)?;
    let Some(target) = world.resolve(gtid)? else {
        world.mind_virtual_140(mind, gtid, arena)?;
        return Ok(wrong);
    };
    if !world.join_arrived(state)? {
        let entry = if world.organization_count_14c(mind)? > 4 {
            world.organization_entry_148(mind, 4)?
        } else {
            0
        };
        if world.target_virtual_130(target, entry, false, arena)? {
            world.mind_virtual_144(mind, arena)?;
            return Ok(wrong);
        }
        order_move(state, arena, world)?;
        world.embedded_think(mind, MOVE_OFFSET, arena)?;
        return Ok(wrong);
    }
    let mut index = world.next_member_index(mind, u32::MAX)?;
    if world.join_gathering(state)? {
        order_move(state, arena, world)?;
        let mut all_listed = true;
        while index != ITERATION_END {
            let member = world.component_member(mind, index)?;
            let gtid = if member != 0 {
                world.object_gtid(member)?
            } else {
                0
            };
            if !world.join_listed(state)?.contains(&gtid) {
                if member == 0 {
                    return Err(null_member());
                }
                let target = target_entry(state, world)?;
                let member_mind = world.member_mind(member)?.ok_or_else(null_member)?;
                world.member_mind_virtual_f4(member_mind, target, arena)?;
                all_listed = false;
            }
            index = world.next_member_index(mind, index)?;
        }
        if all_listed {
            world.mind_virtual_16c(mind, arena)?;
            return Ok(wrong);
        }
    } else {
        if index == ITERATION_END {
            world.mind_virtual_16c(mind, arena)?;
            return Ok(wrong);
        }
        join_all(state, arena, world)?;
    }
    world.schedule(mind, JOIN_DELAY_BITS, 0, arena)?;
    Ok(wrong)
}
