//! Original `KKC_CAIStateRepair::Think` (`5239A0`, regiment state
//! `cai_repair` at Mind+160, vtable A32BA4 slot +C) with the Tai repair
//! action constructor `51E870`.
//!
//! Order (the diagnostic log line is omitted):
//! 1. `KKC_TAIState::Think` base (`529E60`). A target (state+14) that does
//!    not resolve to a non-null entry goes to Mind virtual +140(GTID).
//! 2. A target with a component (+7C) is replaced by the nearest of its
//!    members (iterator `4364C0`, flag clear: every non-null member) that passes `4D1490`
//!    (binary32 squared distance to the organization, strictly nearer than the
//!    stored best, which starts at FLT_MAX); the first non-null member is the
//!    fallback, and no member at all gives up.
//! 3. Give up (Mind virtual +140(state+14)) when the chosen object has a
//!    zero +60, +8 bit 14 clear, organization virtual +34(3) false, or
//!    (+100 bit 0x200000 clear, `4D0940` false and `44DD70` true on the
//!    organization owner, virtual +108).
//! 4. Out of reach (`51FBC0` false): unless state+10 is 1, it is set, an
//!    inactive move state is ordered to the object (`520320(0, object, 0,
//!    null point)`) and Mind virtual +B8; then the move state thinks.
//! 5. In reach: a set state+10 is cleared, the move state is left (`529C20`)
//!    and Mind virtual +B8. Without state+18 a nonzero +60 passing `5617C0`
//!    goes to Mind virtual +168. Otherwise each flagged organization member
//!    (iterator flag set: +80 object +1C byte set) accepting the repair action's definition (virtual +38) gets
//!    the repair action (`51E870`: ordinal 3, target, state+18), else a move
//!    action (`4CA760`, ordinal 5) to its formation slot (component +BC/+C0
//!    plus the +78 slot offsets, live precision, stored binary32) with the
//!    Mind+2B4/+2B8 words; then the next update is scheduled in 1.0 s and
//!    the repair action is released.
use super::{
    MOVE_OFFSET, leave_active,
    stack_action::{ActionCommand, ActionWorld, MOVE_ACTION, REPAIR_ACTION},
};
use crate::{
    Arena, MindId, Result, StateId, think_base,
    x87::{n, num, store},
};
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

/// binary32 1.0: re-think delay while repairing.
pub const REPAIR_DELAY_BITS: u32 = 0x3f80_0000;
/// `4364C0` end-of-iteration index.
pub const ITERATION_END: u32 = 0x7fff_ffff;
const FLT_MAX: u32 = 0x7f7f_ffff;

/// Services in native order.
pub trait RepairThinkWorld: ActionWorld {
    /// Repair state+14 (target GTID), +10 and +18 bytes.
    fn repair_target(&mut self, state: StateId) -> Result<u32>;
    fn repair_byte10(&mut self, state: StateId) -> Result<u8>;
    fn set_repair_byte10(&mut self, state: StateId, value: u8) -> Result<()>;
    fn repair_byte18(&mut self, state: StateId) -> Result<u8>;
    /// Object +7C component, `None` when null.
    fn object_component(&mut self, object: u32) -> Result<Option<u32>>;
    /// Organization +7C component.
    fn organization_component(&mut self, mind: MindId) -> Result<u32>;
    /// `4364C0` on a member iterator of `component` positioned at `index`
    /// (-1 before the first; `437940` starts one): the next index with a
    /// non-null member (with `flagged`, only a member whose +80 object's +1C
    /// byte is set), or [`ITERATION_END`].
    fn component_next_index(&mut self, component: u32, index: u32, flagged: bool) -> Result<u32>;
    /// Component +28 array entry `index`.
    fn component_member(&mut self, component: u32, index: u32) -> Result<u32>;
    /// `4D1490` on a member.
    fn member_4d1490(&mut self, member: u32, arena: &mut Arena) -> Result<bool>;
    /// Object and organization +20/+24 (binary32).
    fn object_position(&mut self, object: u32) -> Result<[u32; 2]>;
    fn organization_position(&mut self, mind: MindId) -> Result<[u32; 2]>;
    /// Object +60, +8 and +100 words.
    fn object_word60(&mut self, object: u32) -> Result<u32>;
    fn object_flags8(&mut self, object: u32) -> Result<u32>;
    fn object_word100(&mut self, object: u32) -> Result<u32>;
    /// Organization virtual +34(value).
    fn organization_virtual_34(
        &mut self,
        mind: MindId,
        value: u32,
        arena: &mut Arena,
    ) -> Result<bool>;
    /// `4D0940` on the object.
    fn object_4d0940(&mut self, object: u32, arena: &mut Arena) -> Result<bool>;
    /// Organization virtual +108 (owner).
    fn organization_owner(&mut self, mind: MindId, arena: &mut Arena) -> Result<u32>;
    /// `44DD70` on the owner.
    fn owner_44dd70(&mut self, owner: u32, arena: &mut Arena) -> Result<bool>;
    /// `51FBC0(object)` on the Mind.
    fn target_in_range(&mut self, mind: MindId, object: u32, arena: &mut Arena) -> Result<bool>;
    /// `[B279C0]/[B279C4]`.
    fn null_point(&mut self) -> Result<[u32; 2]>;
    /// `520320` on the move state with its five stack words (the object is
    /// the second).
    fn move_order(&mut self, mind: MindId, args: [u32; 5], arena: &mut Arena) -> Result<()>;
    /// `5617C0` on the object's +60 word.
    fn check_5617c0(&mut self, word60: u32, arena: &mut Arena) -> Result<bool>;
    /// Mind virtual +168.
    fn mind_virtual_168(&mut self, mind: MindId, arena: &mut Arena) -> Result<()>;
    /// Member virtual +38(definition).
    fn member_virtual_38(
        &mut self,
        member: u32,
        definition: u32,
        arena: &mut Arena,
    ) -> Result<bool>;
    /// Mind+2B4/+2B8 words.
    fn sleep_point(&mut self, mind: MindId) -> Result<[u32; 2]>;
    /// Component +BC/+C0 (binary32 formation origin).
    fn formation_origin(&mut self, component: u32) -> Result<[u32; 2]>;
    /// Component +78 slot `index` (two binary32 offsets).
    fn formation_slot(&mut self, component: u32, index: u32) -> Result<[u32; 2]>;
}

fn distance2(
    mind: MindId,
    object: u32,
    world: &mut impl RepairThinkWorld,
    p: Precision,
) -> Result<Binary> {
    let org = world.organization_position(mind)?;
    let at = world.object_position(object)?;
    let dy = num(n(at[1])?.sub(&n(org[1])?, p))?;
    let dx = num(n(at[0])?.sub(&n(org[0])?, p))?;
    num(num(dx.mul(&dx, p))?.add(&num(dy.mul(&dy, p))?, p))
}

/// Step 2: the nearest repairable member of `component` (or the first
/// non-null member, or `None`).
fn nearest_member(
    mind: MindId,
    component: u32,
    arena: &mut Arena,
    world: &mut impl RepairThinkWorld,
    p: Precision,
) -> Result<Option<u32>> {
    let mut chosen = None;
    let mut best = FLT_MAX;
    let mut index = world.component_next_index(component, u32::MAX, false)?;
    while index != ITERATION_END {
        let member = world.component_member(component, index)?;
        if member != 0 {
            chosen.get_or_insert(member);
            if world.member_4d1490(member, arena)? {
                let d2 = distance2(mind, member, world, p)?;
                if d2.compare(&n(best)?) == Ordering::Less {
                    best = store(d2)?;
                    chosen = Some(member);
                }
            }
        }
        index = world.component_next_index(component, index, false)?;
    }
    Ok(chosen)
}

/// Step 3's eligibility of the chosen object.
fn repairable(
    mind: MindId,
    object: u32,
    arena: &mut Arena,
    world: &mut impl RepairThinkWorld,
) -> Result<bool> {
    if world.object_word60(object)? == 0 || (world.object_flags8(object)? >> 14) & 1 == 0 {
        return Ok(false);
    }
    if !world.organization_virtual_34(mind, 3, arena)? {
        return Ok(false);
    }
    if world.object_word100(object)? & 0x0020_0000 == 0 && !world.object_4d0940(object, arena)? {
        let owner = world.organization_owner(mind, arena)?;
        if world.owner_44dd70(owner, arena)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Complete `5239A0` at x87 precision `p`. Returns the base Think's
/// wrong-activity flag.
pub fn think(
    state: StateId,
    arena: &mut Arena,
    world: &mut impl RepairThinkWorld,
    p: Precision,
) -> Result<bool> {
    let wrong = think_base(state, arena)?;
    let mind = arena.state(state)?.parent;
    let gtid = world.repair_target(state)?;
    let Some(target) = world.resolve(gtid)? else {
        world.mind_virtual_140(mind, gtid, arena)?;
        return Ok(wrong);
    };
    let mut chosen = Some(target);
    if let Some(component) = world.object_component(target)? {
        chosen = nearest_member(mind, component, arena, world, p)?;
    }
    let object = match chosen {
        Some(object) if repairable(mind, object, arena, world)? => object,
        _ => {
            let gtid = world.repair_target(state)?;
            world.mind_virtual_140(mind, gtid, arena)?;
            return Ok(wrong);
        }
    };
    let in_range = world.target_in_range(mind, object, arena)?;
    let moving = world.repair_byte10(state)?;
    if !in_range {
        if moving != 1 {
            world.set_repair_byte10(state, 1)?;
            let move_state = world.embedded_state(mind, MOVE_OFFSET)?;
            if arena.state(move_state)?.status != 2 {
                let null = world.null_point()?;
                world.move_order(mind, [0, object, 0, null[0], null[1]], arena)?;
            }
            world.mind_virtual_b8(mind, arena)?;
        }
        world.embedded_think(mind, MOVE_OFFSET, arena)?;
        return Ok(wrong);
    }
    if moving != 0 {
        world.set_repair_byte10(state, 0)?;
        leave_active(world, mind, MOVE_OFFSET, arena)?;
        world.mind_virtual_b8(mind, arena)?;
    }
    if world.repair_byte18(state)? == 0 {
        let word60 = world.object_word60(object)?;
        if word60 != 0 && world.check_5617c0(word60, arena)? {
            world.mind_virtual_168(mind, arena)?;
            return Ok(wrong);
        }
    }
    let byte18 = world.repair_byte18(state)?;
    let definition = world.action_definition(REPAIR_ACTION)?;
    let repair = ActionCommand::repair(definition, world.object_gtid(object)?, byte18);
    let component = world.organization_component(mind)?;
    let mut index = world.component_next_index(component, u32::MAX, true)?;
    while index != ITERATION_END {
        let member = world.component_member(component, index)?;
        if world.member_virtual_38(member, repair.definition, arena)? {
            let member = world.component_member(component, index)?;
            world.issue_action(member, &repair, arena)?;
        } else {
            let words = world.sleep_point(mind)?;
            let formation = world.organization_component(mind)?;
            let origin = world.formation_origin(formation)?;
            let slot = world.formation_slot(formation, index)?;
            let y = num(n(origin[1])?.add(&n(slot[1])?, p))?;
            let x = num(n(origin[0])?.add(&n(slot[0])?, p))?;
            let location = [store(x)?, store(y)?];
            let definition = world.action_definition(MOVE_ACTION)?;
            let order = ActionCommand::move_to(definition, location, 0, words);
            let member = world.component_member(component, index)?;
            world.issue_action(member, &order, arena)?;
            world.release_action(&order, arena)?;
        }
        index = world.component_next_index(component, index, true)?;
    }
    world.schedule(mind, REPAIR_DELAY_BITS, 0, arena)?;
    world.release_action(&repair, arena)?;
    Ok(wrong)
}
