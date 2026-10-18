//! Original `KKC_CAIStateBombard::Think` (`5207F0`, regiment state
//! `cai_bombard` at Mind+D4, vtable A3348C slot +C) with the Tai attack
//! action constructor `51E7A0`.
//!
//! Order (the diagnostic log line is omitted):
//! 1. `KKC_TAIState::Think` base (`529E60`). The target (state+10) resolves
//!    to its registry entry or null; a non-null target is usable.
//! 2. A target failing its virtual +5C(organization owner (+108), 1): with
//!    +8 bit 29 clear the alert `42D470([B39D8C], 0x26, owner, &target
//!    position, organization name (+24))` is posted and Mind virtual
//!    +140(target GTID) ends the Think; with bit 29 set it is not usable.
//! 3. With a named order (state+14 CString length nonzero) and no usable
//!    target the owner is read again; without any target, a point
//!    (state+18/+1C) whose region (`42A530`) passes `54F3C0(owner)` ends the
//!    Think through Mind virtual +140(state+10).
//! 4. The relation is `4D4FF0(target, 3, 1)` for a usable target, else
//!    `4D5180(&point, 1)`: 1 approaches, 0 fires, anything else goes to Mind
//!    virtual +140(target GTID) (target) or +144 (point).
//! 5. Approach: an active move state thinks; else the target (non-null, +8
//!    bit 29 clear) is ordered with `520320(0, target, 0, null point)` or the
//!    point with `520440(0, point, 0, null point)`, then the move state
//!    thinks.
//! 6. Fire: an active move state sets state+20; the move state is left
//!    (`529C20`); each organization member accepting virtual +34(1) gets the
//!    attack action (`51E7A0(target)`, ordinal 1) while state+10 resolves to
//!    a non-null entry, else an ordinal 1 action at the point (flag 4, words
//!    copied through the x87 stack); a refusing member gets, while state+20
//!    is set, a move action (`4CA760`) to its formation slot (component
//!    +BC/+C0 plus the +78 offsets, live precision, stored binary32) with the
//!    Mind+2B4/+2B8 words. Then state+20 = 0 and the next update is
//!    scheduled in 1.0 s.
use super::{
    MOVE_OFFSET, leave_active,
    stack_action::{ActionCommand, ActionWorld, MOVE_ACTION},
    x87_copy,
};
use crate::{
    Arena, MindId, Result, StateId, think_base,
    x87::{n, num, store},
};
use aa_numeric::Precision;

/// binary32 1.0: re-think delay while firing.
pub const BOMBARD_DELAY_BITS: u32 = 0x3f80_0000;
/// `4364C0` end-of-iteration index.
pub const ITERATION_END: u32 = 0x7fff_ffff;
/// `42D470` alert kind.
pub const ALERT_KIND: u32 = 0x26;
/// Ordinal of the attack actions.
pub const ATTACK_ACTION: u32 = 1;
const BIT29: u32 = 0x2000_0000;

/// Services in native order.
pub trait BombardThinkWorld: ActionWorld {
    /// Bombard state+10 (target GTID), +14 CString length, +18/+1C point
    /// (binary32 words) and +20 byte.
    fn bombard_target(&mut self, state: StateId) -> Result<u32>;
    fn bombard_label_length(&mut self, state: StateId) -> Result<u32>;
    fn bombard_point(&mut self, state: StateId) -> Result<[u32; 2]>;
    fn bombard_byte20(&mut self, state: StateId) -> Result<bool>;
    fn set_bombard_byte20(&mut self, state: StateId, value: bool) -> Result<()>;
    /// Organization virtual +108 (owner).
    fn organization_owner(&mut self, mind: MindId, arena: &mut Arena) -> Result<u32>;
    /// Organization virtual +24 (name).
    fn organization_name(&mut self, mind: MindId, arena: &mut Arena) -> Result<String>;
    /// Target virtual +5C(owner, flag).
    fn target_virtual_5c(
        &mut self,
        target: u32,
        owner: u32,
        flag: u32,
        arena: &mut Arena,
    ) -> Result<bool>;
    /// Object +8 flags and +20/+24 position words.
    fn object_flags8(&mut self, object: u32) -> Result<u32>;
    fn object_position(&mut self, object: u32) -> Result<[u32; 2]>;
    /// `42D470([B39D8C], kind, owner, &position, name)`.
    fn post_alert(
        &mut self,
        kind: u32,
        owner: u32,
        position: [u32; 2],
        name: &str,
        arena: &mut Arena,
    ) -> Result<()>;
    /// `42A530(x, y)` on the region map `[B36534]+30`.
    fn region_at(&mut self, point: [u32; 2], arena: &mut Arena) -> Result<u32>;
    /// `54F3C0(owner)` on a region.
    fn region_54f3c0(&mut self, region: u32, owner: u32, arena: &mut Arena) -> Result<bool>;
    /// `4D4FF0(target, a, b)` on the organization.
    fn relation(
        &mut self,
        mind: MindId,
        target: u32,
        a: u32,
        b: u32,
        arena: &mut Arena,
    ) -> Result<i32>;
    /// `4D5180(&state+18, b)` on the organization.
    fn point_relation(
        &mut self,
        mind: MindId,
        state: StateId,
        b: u32,
        arena: &mut Arena,
    ) -> Result<i32>;
    /// Mind virtual +144.
    fn mind_virtual_144(&mut self, mind: MindId, arena: &mut Arena) -> Result<()>;
    /// `[B279C0]/[B279C4]`.
    fn null_point(&mut self) -> Result<[u32; 2]>;
    /// `520320` (five words, the target second) and `520440` (six words) on
    /// the move state.
    fn move_order_object(&mut self, mind: MindId, args: [u32; 5], arena: &mut Arena) -> Result<()>;
    fn move_order_point(&mut self, mind: MindId, args: [u32; 6], arena: &mut Arena) -> Result<()>;
    /// Organization +7C component.
    fn organization_component(&mut self, mind: MindId) -> Result<u32>;
    /// `4364C0` on a member iterator of `component` (flag clear) positioned
    /// at `index` (-1 before the first): the next index with a non-null
    /// member, or [`ITERATION_END`].
    fn component_next_index(&mut self, component: u32, index: u32) -> Result<u32>;
    /// Component +28 array entry `index`.
    fn component_member(&mut self, component: u32, index: u32) -> Result<u32>;
    /// Member virtual +34(value).
    fn member_virtual_34(&mut self, member: u32, value: u32, arena: &mut Arena) -> Result<bool>;
    /// Mind+2B4/+2B8 words.
    fn sleep_point(&mut self, mind: MindId) -> Result<[u32; 2]>;
    /// Component +BC/+C0 (binary32) and +78 slot `index` (two binary32).
    fn formation_origin(&mut self, component: u32) -> Result<[u32; 2]>;
    fn formation_slot(&mut self, component: u32, index: u32) -> Result<[u32; 2]>;
}

/// `51E7A0(target)`: ordinal 1 with `4D9BF0(target)` (flag 1 and the
/// target GTID, or flag 1 cleared and 0 for a null target).
pub fn attack_action(definition: u32, target_gtid: Option<u32>) -> ActionCommand {
    let mut action = ActionCommand::new(ATTACK_ACTION, definition);
    match target_gtid {
        Some(gtid) => {
            action.flags |= 1;
            action.word10 = gtid;
        }
        None => action.flags &= !1,
    }
    action
}

fn fire(
    state: StateId,
    target: Option<u32>,
    arena: &mut Arena,
    world: &mut impl BombardThinkWorld,
    p: Precision,
) -> Result<()> {
    let mind = arena.state(state)?.parent;
    let moving = world.embedded_state(mind, MOVE_OFFSET)?;
    if arena.state(moving)?.status == 2 {
        world.set_bombard_byte20(state, true)?;
    }
    leave_active(world, mind, MOVE_OFFSET, arena)?;
    let component = world.organization_component(mind)?;
    let mut index = world.component_next_index(component, u32::MAX)?;
    while index != ITERATION_END {
        let member = world.component_member(component, index)?;
        if world.member_virtual_34(member, 1, arena)? {
            let gtid = world.bombard_target(state)?;
            let action = if world.resolve(gtid)?.is_some() {
                let gtid = match target {
                    Some(t) => Some(world.object_gtid(t)?),
                    None => None,
                };
                attack_action(world.action_definition(ATTACK_ACTION)?, gtid)
            } else {
                let mut action =
                    ActionCommand::new(ATTACK_ACTION, world.action_definition(ATTACK_ACTION)?);
                let point = world.bombard_point(state)?;
                action.flags |= 4;
                action.words18_28[0] = x87_copy(point[0]);
                action.words18_28[1] = x87_copy(point[1]);
                action
            };
            world.issue_action(member, &action, arena)?;
            world.release_action(&action, arena)?;
        } else if world.bombard_byte20(state)? {
            let formation = world.organization_component(mind)?;
            let origin = world.formation_origin(formation)?;
            let slot = world.formation_slot(formation, index)?;
            let y = num(n(origin[1])?.add(&n(slot[1])?, p))?;
            let x = num(n(origin[0])?.add(&n(slot[0])?, p))?;
            let words = world.sleep_point(mind)?;
            let location = [store(x)?, store(y)?];
            let definition = world.action_definition(MOVE_ACTION)?;
            let order = ActionCommand::move_to(definition, location, 0, words);
            world.issue_action(member, &order, arena)?;
            world.release_action(&order, arena)?;
        }
        index = world.component_next_index(component, index)?;
    }
    world.set_bombard_byte20(state, false)?;
    world.schedule(mind, BOMBARD_DELAY_BITS, 0, arena)
}

fn approach(
    state: StateId,
    target: Option<u32>,
    arena: &mut Arena,
    world: &mut impl BombardThinkWorld,
) -> Result<()> {
    let mind = arena.state(state)?.parent;
    let moving = world.embedded_state(mind, MOVE_OFFSET)?;
    if arena.state(moving)?.status != 2 {
        let null = world.null_point()?;
        match target {
            Some(t) if world.object_flags8(t)? & BIT29 == 0 => {
                world.move_order_object(mind, [0, t, 0, null[0], null[1]], arena)?;
            }
            _ => {
                let point = world.bombard_point(state)?;
                world.move_order_point(
                    mind,
                    [0, point[0], point[1], 0, null[0], null[1]],
                    arena,
                )?;
            }
        }
    }
    world.embedded_think(mind, MOVE_OFFSET, arena)
}

/// Complete `5207F0` at x87 precision `p`. Returns the base Think's
/// wrong-activity flag.
pub fn think(
    state: StateId,
    arena: &mut Arena,
    world: &mut impl BombardThinkWorld,
    p: Precision,
) -> Result<bool> {
    let wrong = think_base(state, arena)?;
    let mind = arena.state(state)?.parent;
    let gtid = world.bombard_target(state)?;
    let target = world.resolve(gtid)?;
    let mut usable = target.is_some();
    if let Some(t) = target {
        let owner = world.organization_owner(mind, arena)?;
        if !world.target_virtual_5c(t, owner, 1, arena)? {
            if world.object_flags8(t)? & BIT29 == 0 {
                let position = world.object_position(t)?;
                let name = world.organization_name(mind, arena)?;
                let owner = world.organization_owner(mind, arena)?;
                world.post_alert(ALERT_KIND, owner, position, &name, arena)?;
                let gtid = world.object_gtid(t)?;
                world.mind_virtual_140(mind, gtid, arena)?;
                return Ok(wrong);
            }
            usable = false;
        }
    }
    let by_target = if world.bombard_label_length(state)? != 0 && !usable {
        let owner = world.organization_owner(mind, arena)?;
        if target.is_none() {
            let point = world.bombard_point(state)?;
            let region = world.region_at(point, arena)?;
            if world.region_54f3c0(region, owner, arena)? {
                let gtid = world.bombard_target(state)?;
                world.mind_virtual_140(mind, gtid, arena)?;
                return Ok(wrong);
            }
        }
        false
    } else {
        usable
    };
    let relation = match (by_target, target) {
        (true, Some(t)) => world.relation(mind, t, 3, 1, arena)?,
        _ => world.point_relation(mind, state, 1, arena)?,
    };
    match relation {
        1 => approach(state, target, arena, world)?,
        0 => fire(state, target, arena, world, p)?,
        _ => match (by_target, target) {
            (true, Some(t)) => {
                let gtid = world.object_gtid(t)?;
                world.mind_virtual_140(mind, gtid, arena)?;
            }
            _ => world.mind_virtual_144(mind, arena)?,
        },
    }
    Ok(wrong)
}
