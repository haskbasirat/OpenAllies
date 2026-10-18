//! Original `KKC_CAIStateSiege::Think` (`525930`, regiment state `cai_siege`
//! at Mind+F8, vtable A32CC8 slot +C) with the engage-state helper `51E000`.
//!
//! Order (the diagnostic log line is omitted):
//! 1. `KKC_TAIState::Think` base (`529E60`). A target (state+10) that does
//!    not resolve to a non-null registry entry goes to Mind virtual +140.
//! 2. The target's site (`51D130`, [`super::siege_target::site`]) must be
//!    the resolved site entry (state+14) whose `4CEDA0` anchor exists;
//!    otherwise Mind virtual +140(state+10) and the kill state (Mind+C0) is
//!    activated for the target in mode 1 (`51D920`).
//! 3. Siege reach `r` = binary32(SVar `[[B39AB0]+C4]` + `[B39F58]+74`). With
//!    the anchor at squared distance `d2` from the organization (live x87
//!    precision) and `r*r <= d2`: state+21 = 1, an inactive move state is
//!    ordered to the anchor (`520440(0, anchor, 0, null point)`), the move
//!    state thinks and state+20 = 1.
//! 4. In reach: state+21 = 0. With the kill state active on this target
//!    (Mind+D0 == state+10), Mind virtual +44's +14 byte set and SVar
//!    `[[B39AB0]+C8]` + state+24 below the game time (`[B36534]+E0`), state+20
//!    = 1. Without that kill state, move and kill are left (`529C20`), the
//!    kill state is activated for the target (`51D920(0)`) and state+20 = 1.
//! 5. state+20 set and Mind virtual +44's +4 word nonzero: its +14 byte and
//!    state+20 are cleared, state+24 = game time, the state+18 GTID list is
//!    cleared and refilled with the target (a target with a nonzero +90) or
//!    by `523700(info, &list, site)`, then `51E000` notifies the world.
//! 6. Each listed live object, while the organization's entry 4 exists, is
//!    in reach when `binary32(object +0 + binary32(organization +0) + entry
//!    +8)^2 >= d2`; then `51FBC0(object)` and Mind virtual +F0(object) both
//!    true end the Think. Finally the kill state thinks.
use super::{
    KILL_OFFSET, MOVE_OFFSET, leave_active,
    siege_target::{SiegeTargetWorld, site},
};
use crate::{
    Arena, Error, MindId, Result, StateId, think_base,
    x87::{n, num, store},
};
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

/// Services in native order.
pub trait SiegeThinkWorld: SiegeTargetWorld {
    /// Siege state+10 (target GTID) and +14 (site GTID).
    fn siege_target(&mut self, state: StateId) -> Result<u32>;
    fn siege_site(&mut self, state: StateId) -> Result<u32>;
    /// `4CEDA0` on the site: its anchor object, `None` when null.
    fn site_anchor(&mut self, site: u32, arena: &mut Arena) -> Result<Option<u32>>;
    /// Object +20/+24 (binary32).
    fn object_position(&mut self, object: u32) -> Result<[u32; 2]>;
    /// Organization +20/+24 (binary32).
    fn organization_position(&mut self, mind: MindId) -> Result<[u32; 2]>;
    /// `[B39F58]+74` (binary32).
    fn siege_range_base(&mut self) -> Result<u32>;
    /// SVar table `[B39AB0]` +C4 and +C8 (binary32).
    fn svar_c4(&mut self) -> Result<u32>;
    fn svar_c8(&mut self) -> Result<u32>;
    /// `[B36534]+E0` (binary32 game time word).
    fn game_time(&mut self) -> Result<u32>;
    /// Siege state +20/+21 bytes and +24 stamp word.
    fn siege_attacking(&mut self, state: StateId) -> Result<bool>;
    fn set_siege_attacking(&mut self, state: StateId, value: bool) -> Result<()>;
    fn set_siege_approaching(&mut self, state: StateId, value: bool) -> Result<()>;
    fn siege_stamp(&mut self, state: StateId) -> Result<u32>;
    fn set_siege_stamp(&mut self, state: StateId, value: u32) -> Result<()>;
    /// `[B279C0]/[B279C4]`.
    fn null_point(&mut self) -> Result<[u32; 2]>;
    /// `520440` on the move state with its six stack words.
    fn move_order(&mut self, mind: MindId, args: [u32; 6], arena: &mut Arena) -> Result<()>;
    /// Kill state+10 (Mind+D0).
    fn kill_target(&mut self, mind: MindId) -> Result<u32>;
    /// `51D920(mode, target)` on the kill state; result unused here.
    fn kill_activate(
        &mut self,
        mind: MindId,
        mode: u32,
        target: u32,
        arena: &mut Arena,
    ) -> Result<bool>;
    /// Mind virtual +44 (siege info), `None` when null.
    fn mind_virtual_44(&mut self, mind: MindId, arena: &mut Arena) -> Result<Option<u32>>;
    /// Info +4 word and +14 byte.
    fn info_word4(&mut self, info: u32) -> Result<u32>;
    fn info_byte14(&mut self, info: u32) -> Result<bool>;
    fn set_info_byte14(&mut self, info: u32, value: bool) -> Result<()>;
    /// `763B90` on the state+18 GTID list (clear).
    fn siege_list_clear(&mut self, state: StateId) -> Result<()>;
    /// `4D19A0` push back of a `4CF1B0` handle (the object's GTID).
    fn siege_list_push(&mut self, state: StateId, gtid: u32) -> Result<()>;
    /// `523700(info, &state+18, site)` on the siege state.
    fn siege_fill(&mut self, state: StateId, info: u32, site: u32, arena: &mut Arena)
    -> Result<()>;
    /// State+18 list head node, a node's GTID and its (reread) next node.
    fn siege_list_head(&mut self, state: StateId) -> Result<Option<u32>>;
    fn list_node_gtid(&mut self, node: u32) -> Result<u32>;
    fn list_node_next(&mut self, node: u32) -> Result<Option<u32>>;
    /// Organization +14C (signed count) and +148 array entry `index`.
    fn organization_count_14c(&mut self, mind: MindId) -> Result<i32>;
    fn organization_entry_148(&mut self, mind: MindId, index: u32) -> Result<u32>;
    /// `4F3D90(entry)` on the world `[B36534]`.
    fn world_4f3d90(&mut self, entry: u32, arena: &mut Arena) -> Result<()>;
    /// Entry +8 (binary32 word).
    fn entry_word8(&mut self, entry: u32) -> Result<u32>;
    /// Organization virtual +0 and object virtual +0 (ST0 results).
    fn organization_virtual_0(&mut self, mind: MindId, arena: &mut Arena) -> Result<Binary>;
    fn object_virtual_0(&mut self, object: u32, arena: &mut Arena) -> Result<Binary>;
    /// `51FBC0(object)` on the Mind.
    fn target_in_range(&mut self, mind: MindId, object: u32, arena: &mut Arena) -> Result<bool>;
    /// Mind virtual +F0(object).
    fn mind_virtual_f0(&mut self, mind: MindId, object: u32, arena: &mut Arena) -> Result<bool>;
}

fn null_info() -> Error {
    Error::World("null Mind virtual +44 info dereference".into())
}

/// Squared distance from the organization to `object` at precision `p`.
fn distance2(
    mind: MindId,
    object: u32,
    world: &mut impl SiegeThinkWorld,
    p: Precision,
) -> Result<Binary> {
    let org = world.organization_position(mind)?;
    let at = world.object_position(object)?;
    let dy = num(n(at[1])?.sub(&n(org[1])?, p))?;
    let dx = num(n(at[0])?.sub(&n(org[0])?, p))?;
    num(num(dx.mul(&dx, p))?.add(&num(dy.mul(&dy, p))?, p))
}

/// The organization's entry 4 (+148 array when the signed +14C count
/// exceeds 4), else 0.
fn entry4(mind: MindId, world: &mut impl SiegeThinkWorld) -> Result<u32> {
    if world.organization_count_14c(mind)? > 4 {
        world.organization_entry_148(mind, 4)
    } else {
        Ok(0)
    }
}

/// `51E000` on the engage state: the world is told about entry 4.
pub fn engage_notify(
    mind: MindId,
    arena: &mut Arena,
    world: &mut impl SiegeThinkWorld,
) -> Result<()> {
    let entry = entry4(mind, world)?;
    if entry != 0 {
        world.world_4f3d90(entry, arena)?;
    }
    Ok(())
}

/// Complete `525930` at x87 precision `p`. Returns the base Think's
/// wrong-activity flag. A null Mind virtual +44 info is the native null
/// dereference.
pub fn think(
    state: StateId,
    arena: &mut Arena,
    world: &mut impl SiegeThinkWorld,
    p: Precision,
) -> Result<bool> {
    let wrong = think_base(state, arena)?;
    let mind = arena.state(state)?.parent;
    let gtid = world.siege_target(state)?;
    let Some(target) = world.resolve(gtid)? else {
        world.mind_virtual_140(mind, gtid, arena)?;
        return Ok(wrong);
    };
    let target_site = site(state, Some(target), arena, world)?;
    let site_gtid = world.siege_site(state)?;
    let mut anchor = None;
    let site_entry = world.resolve(site_gtid)?;
    if let Some(entry) = site_entry {
        anchor = world.site_anchor(entry, arena)?;
    }
    let (Some(site_entry), Some(anchor)) = (site_entry, anchor) else {
        return give_up(state, target, arena, world).map(|()| wrong);
    };
    if target_site != Some(site_entry) {
        return give_up(state, target, arena, world).map(|()| wrong);
    }
    let reach = store(num(
        n(world.svar_c4()?)?.add(&n(world.siege_range_base()?)?, p)
    )?)?;
    let d2 = distance2(mind, anchor, world, p)?;
    let r2 = num(n(reach)?.mul(&n(reach)?, p))?;
    if r2.compare(&d2) != Ordering::Greater {
        world.set_siege_approaching(state, true)?;
        let moving = world.embedded_state(mind, MOVE_OFFSET)?;
        if arena.state(moving)?.status != 2 {
            let null = world.null_point()?;
            let at = world.object_position(anchor)?;
            world.move_order(mind, [0, at[0], at[1], 0, null[0], null[1]], arena)?;
        }
        world.embedded_think(mind, MOVE_OFFSET, arena)?;
        world.set_siege_attacking(state, true)?;
        return Ok(wrong);
    }
    world.set_siege_approaching(state, false)?;
    let kill = world.embedded_state(mind, KILL_OFFSET)?;
    if arena.state(kill)?.status == 2 && world.kill_target(mind)? == world.siege_target(state)? {
        let info = world.mind_virtual_44(mind, arena)?.ok_or_else(null_info)?;
        if world.info_byte14(info)? {
            let due = num(n(world.svar_c8()?)?.add(&n(world.siege_stamp(state)?)?, p))?;
            if due.compare(&n(world.game_time()?)?) == Ordering::Less {
                world.set_siege_attacking(state, true)?;
            }
        }
    } else {
        leave_active(world, mind, MOVE_OFFSET, arena)?;
        leave_active(world, mind, KILL_OFFSET, arena)?;
        world.kill_activate(mind, 0, target, arena)?;
        world.set_siege_attacking(state, true)?;
    }
    if world.siege_attacking(state)? {
        let info = world.mind_virtual_44(mind, arena)?.ok_or_else(null_info)?;
        if world.info_word4(info)? != 0 {
            let info = world.mind_virtual_44(mind, arena)?.ok_or_else(null_info)?;
            world.set_info_byte14(info, false)?;
            world.set_siege_attacking(state, false)?;
            let time = world.game_time()?;
            world.set_siege_stamp(state, time)?;
            world.siege_list_clear(state)?;
            if world.object_word90(target)? != 0 {
                let gtid = world.object_gtid(target)?;
                world.siege_list_push(state, gtid)?;
            } else {
                let info = world.mind_virtual_44(mind, arena)?.ok_or_else(null_info)?;
                world.siege_fill(state, info, site_entry, arena)?;
            }
            engage_notify(mind, arena, world)?;
        }
    }
    let mut node = world.siege_list_head(state)?;
    while let Some(current) = node {
        let gtid = world.list_node_gtid(current)?;
        let object = world.resolve(gtid)?;
        let entry = entry4(mind, world)?;
        if let Some(object) = object
            && entry != 0
        {
            let extra = world.entry_word8(entry)?;
            let own = store(world.organization_virtual_0(mind, arena)?)?;
            let theirs = world.object_virtual_0(object, arena)?;
            let reach = store(num(num(theirs.add(&n(own)?, p))?.add(&n(extra)?, p))?)?;
            let d2 = distance2(mind, object, world, p)?;
            let r2 = num(n(reach)?.mul(&n(reach)?, p))?;
            if r2.compare(&d2) != Ordering::Less
                && world.target_in_range(mind, object, arena)?
                && world.mind_virtual_f0(mind, object, arena)?
            {
                return Ok(wrong);
            }
        }
        node = world.list_node_next(current)?;
    }
    world.embedded_think(mind, KILL_OFFSET, arena)?;
    Ok(wrong)
}

/// The `525C71` exit: Mind virtual +140(state+10), then `51D920(1, target)`.
fn give_up(
    state: StateId,
    target: u32,
    arena: &mut Arena,
    world: &mut impl SiegeThinkWorld,
) -> Result<()> {
    let mind = arena.state(state)?.parent;
    let gtid = world.siege_target(state)?;
    world.mind_virtual_140(mind, gtid, arena)?;
    world.kill_activate(mind, 1, target, arena)?;
    Ok(())
}
