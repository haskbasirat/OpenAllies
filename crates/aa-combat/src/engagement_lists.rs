//! Native actor engagement lists: `{head, tail}` lists of `4D0900` nodes
//! (`[GTID, next]`) that targeting, rout and supply read.
//!
//! * `4D3990(attacker)`: SAI notice, OrganizationComponent `57A3C0`, then the
//!   attacker's GTID made the head of actor `+118` / `+11C`.
//! * `4D3A20(thing)` / `4D0D70(thing)`: the thing's GTID made the head of, or
//!   its first node removed (`4F1430`) from, actor `+120` / `+124`.
//! * `4D4EF0`, `4D4E70`, `4D4F50`: collectors that prepend (or, for the
//!   organization members of `4D4E70`, append) GTIDs to a caller's list.
//!
//! A null thing stands for GTID 0. Oracle:
//! research/phase8-combat/native_engagement_lists.py.
use crate::Result;
use std::collections::VecDeque;

/// A native `{head, tail}` node list, head first.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct List {
    pub values: VecDeque<u32>,
}

impl List {
    /// `4D0900(&value, head)` stored as the head; the tail is set when null.
    pub fn push_head(&mut self, value: u32) {
        self.values.push_front(value);
    }
    /// `4D4E70`'s insert: a `4D0900(&value, 0)` node after the tail, or the
    /// head of a list without a tail.
    pub fn push_tail(&mut self, value: u32) {
        self.values.push_back(value);
    }
    /// `4F1430(&value)`: unlinks the first node holding `value` (the tail
    /// moves back when it was that node). True when one was removed.
    pub fn remove_first(&mut self, value: u32) -> bool {
        match self.values.iter().position(|&v| v == value) {
            Some(index) => {
                self.values.remove(index);
                true
            }
            None => false,
        }
    }
}

pub trait World {
    /// Thing `+14` GTID.
    fn gtid(&mut self, thing: u32) -> Result<u32>;
    /// `[B36528]` is set and its `+20` byte is nonzero.
    fn notices_enabled(&mut self) -> Result<bool>;
    /// Actor `+E8`.
    fn kingdom_object(&mut self, actor: u32) -> Result<Option<u32>>;
    /// `79EA90(kingdom object)` on `[B36528]`.
    fn notice_target(&mut self, kingdom_object: Option<u32>) -> Result<Option<u32>>;
    /// `7B1ED0(actor)` on the `79EA90` result.
    fn notice_attacked(&mut self, notice: u32, actor: u32) -> Result<()>;
    /// Actor `+7C` OrganizationComponent.
    fn organization(&mut self, actor: u32) -> Result<Option<u32>>;
    /// `57A3C0(attacker)` on an OrganizationComponent.
    fn organization_attacked(&mut self, organization: u32, attacker: Option<u32>) -> Result<()>;
    /// OrganizationComponent `+2C` (read once by `4D4E70`).
    fn member_count(&mut self, organization: u32) -> Result<i32>;
    /// OrganizationComponent `+28[index]`.
    fn member(&mut self, organization: u32, index: i32) -> Result<Option<u32>>;
    /// Actor `+98` component.
    fn component98(&mut self, actor: u32) -> Result<Option<u32>>;
    /// Component `+28` count (read once by `4D4EF0`).
    fn component98_count(&mut self, component: u32) -> Result<i32>;
    /// Component `+24[index]`.
    fn component98_entry(&mut self, component: u32, index: i32) -> Result<Option<u32>>;
    /// Actor `+88` component.
    fn component88(&mut self, actor: u32) -> Result<Option<u32>>;
    /// The values of the component's `+40` node list, head first.
    fn component88_nodes(&mut self, component: u32) -> Result<Vec<Option<u32>>>;
    /// The GTIDs of an object's `+C` node list, head first.
    fn gtid_nodes(&mut self, object: u32) -> Result<Vec<u32>>;
    /// Registry `[B364C0]+4+index*2` generation word.
    fn registry_generation(&mut self, index: u32) -> Result<u16>;
    /// Registry `[B364C0]+20004+index*4` object.
    fn registry_object(&mut self, index: u32) -> Result<Option<u32>>;
}

fn gtid_or_zero(world: &mut impl World, thing: Option<u32>) -> Result<u32> {
    match thing {
        Some(thing) => world.gtid(thing),
        None => Ok(0),
    }
}

/// Native `4D3990(attacker)` on `actor`, with `list118` its `+118` list.
pub fn add_attacker(world: &mut impl World, actor: u32, attacker: Option<u32>, list118: &mut List) -> Result<()> {
    if world.notices_enabled()? {
        let object = world.kingdom_object(actor)?;
        if let Some(notice) = world.notice_target(object)? {
            world.notice_attacked(notice, actor)?;
        }
    }
    if let Some(organization) = world.organization(actor)? {
        world.organization_attacked(organization, attacker)?;
    }
    let gtid = gtid_or_zero(world, attacker)?;
    list118.push_head(gtid);
    Ok(())
}

/// Native `4D3A20(thing)` on an actor, with `list120` its `+120` list.
pub fn add_to_120(world: &mut impl World, thing: Option<u32>, list120: &mut List) -> Result<()> {
    let gtid = gtid_or_zero(world, thing)?;
    list120.push_head(gtid);
    Ok(())
}

/// Native `4D0D70(thing)` on an actor, with `list120` its `+120` list.
pub fn remove_from_120(world: &mut impl World, thing: Option<u32>, list120: &mut List) -> Result<bool> {
    let gtid = gtid_or_zero(world, thing)?;
    Ok(list120.remove_first(gtid))
}

/// Native `4D4EF0(list, actor)`: the `+98` component entries, last first,
/// each made the head.
pub fn collect_component98(world: &mut impl World, list: &mut List, actor: u32) -> Result<()> {
    let Some(component) = world.component98(actor)? else {
        return Ok(());
    };
    let mut index = world.component98_count(component)?.wrapping_sub(1);
    while index >= 0 {
        if let Some(entry) = world.component98_entry(component, index)? {
            let gtid = world.gtid(entry)?;
            list.push_head(gtid);
        }
        index -= 1;
    }
    Ok(())
}

/// Native `4D4E70(list, actor)`: the organization members, last first, each
/// appended after the tail.
pub fn collect_members(world: &mut impl World, list: &mut List, actor: u32) -> Result<()> {
    let Some(organization) = world.organization(actor)? else {
        return Ok(());
    };
    let mut index = world.member_count(organization)?.wrapping_sub(1);
    while index >= 0 {
        if let Some(member) = world.member(organization, index)? {
            let gtid = world.gtid(member)?;
            list.push_tail(gtid);
        }
        index -= 1;
    }
    Ok(())
}

/// Native `4D4F50(list, actor, members)`: for every object of the `+88`
/// component's `+40` list, every GTID of its `+C` list that the registry
/// still resolves (generation word equal, object non-null) puts that
/// object's own GTID at the head, followed by `4D4E70` when `members`.
pub fn collect_component88(world: &mut impl World, list: &mut List, actor: u32, members: bool) -> Result<()> {
    let Some(component) = world.component88(actor)? else {
        return Ok(());
    };
    for object in world.component88_nodes(component)?.into_iter().flatten() {
        for gtid in world.gtid_nodes(object)? {
            let index = gtid & 0xFFFF;
            if (gtid >> 16) as u16 != world.registry_generation(index)? {
                continue;
            }
            let Some(resolved) = world.registry_object(index)? else {
                continue;
            };
            let own = world.gtid(resolved)?;
            list.push_head(own);
            if members {
                collect_members(world, list, resolved)?;
            }
        }
    }
    Ok(())
}
