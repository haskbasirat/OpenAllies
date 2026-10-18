//! Native Actor list `+108/+10C` refresh `4D4CF0` and parent propagation `4D4E50`.
//!
//! This is distinct from PropertiesComponent numeric consequence refresh581420.
//! Storage and all dependencies are required; writes/free/allocation effects are
//! immediate, preserving errors and callback mutations without copyback.
use crate::{Error, Result};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct NodeId(usize);
impl NodeId {
    pub fn raw(self) -> usize {
        self.0
    }
}
#[derive(Clone, Debug)]
struct Node<T> {
    value: Option<T>,
    next: Option<Option<NodeId>>,
}
/// Authoritative growable actor list. Constructor head/tail are actual nulls;
/// newly allocated node fields remain explicitly unwritten until native stores.
/// NodeId is local to this owner; adapters must carry its owning actor identity.
#[derive(Clone, Debug)]
pub struct State<T> {
    head: Option<NodeId>,
    tail: Option<NodeId>,
    next_id: usize,
    nodes: BTreeMap<NodeId, Node<T>>,
}
impl<T> Default for State<T> {
    fn default() -> Self {
        Self::new()
    }
}
impl<T> State<T> {
    pub fn new() -> Self {
        Self {
            head: None,
            tail: None,
            next_id: 0,
            nodes: BTreeMap::new(),
        }
    }
    pub fn head(&self) -> Option<NodeId> {
        self.head
    }
    pub fn tail(&self) -> Option<NodeId> {
        self.tail
    }
    pub fn set_head(&mut self, head: Option<NodeId>) {
        self.head = head;
    }
    pub fn set_tail(&mut self, tail: Option<NodeId>) {
        self.tail = tail;
    }
    pub fn allocated_nodes(&self) -> usize {
        self.nodes.len()
    }
    pub fn allocate(&mut self) -> Result<NodeId> {
        let id = NodeId(self.next_id);
        self.next_id = self
            .next_id
            .checked_add(1)
            .ok_or_else(|| Error::Callback("actor list logical node identity exhausted".into()))?;
        self.nodes.insert(
            id,
            Node {
                value: None,
                next: None,
            },
        );
        Ok(id)
    }
    pub fn free(&mut self, id: NodeId) -> Result<()> {
        self.nodes
            .remove(&id)
            .ok_or_else(|| Error::Callback("actor list free references missing node".into()))?;
        Ok(())
    }
    pub fn value(&self, id: NodeId) -> Result<&T> {
        self.nodes
            .get(&id)
            .and_then(|n| n.value.as_ref())
            .ok_or_else(|| Error::Callback("actor list node value is missing/unwritten".into()))
    }
    pub fn next(&self, id: NodeId) -> Result<Option<NodeId>> {
        self.nodes
            .get(&id)
            .and_then(|n| n.next)
            .ok_or_else(|| Error::Callback("actor list node next is missing/unwritten".into()))
    }
    pub fn set_value(&mut self, id: NodeId, value: T) -> Result<()> {
        self.nodes
            .get_mut(&id)
            .ok_or_else(|| {
                Error::Callback("actor list value store references missing node".into())
            })?
            .value = Some(value);
        Ok(())
    }
    pub fn set_next(&mut self, id: NodeId, next: Option<NodeId>) -> Result<()> {
        self.nodes
            .get_mut(&id)
            .ok_or_else(|| Error::Callback("actor list next store references missing node".into()))?
            .next = Some(next);
        Ok(())
    }
}

pub trait World {
    type Actor: Copy;
    type Node: Copy;
    type Definition: Copy;
    type Organization: Copy;
    type Element: Copy;
    /// Stable pointer/definition identity; never compare display names.
    type Value: Eq;
    fn actor_head(&mut self, actor: Self::Actor) -> Result<Option<Self::Node>>;
    fn actor_tail(&mut self, actor: Self::Actor) -> Result<Option<Self::Node>>;
    fn store_actor_head(&mut self, actor: Self::Actor, node: Option<Self::Node>) -> Result<()>;
    fn store_actor_tail(&mut self, actor: Self::Actor, node: Option<Self::Node>) -> Result<()>;
    fn node_value(&mut self, node: Self::Node) -> Result<Self::Value>;
    fn node_next(&mut self, node: Self::Node) -> Result<Option<Self::Node>>;
    fn store_node_value(&mut self, node: Self::Node, value: Self::Value) -> Result<()>;
    fn store_node_next(&mut self, node: Self::Node, next: Option<Self::Node>) -> Result<()>;
    /// Native6F7810(8); no fields may be invented as initialized.
    fn allocate_node(&mut self, actor: Self::Actor) -> Result<Self::Node>;
    fn free_node(&mut self, actor: Self::Actor, node: Self::Node) -> Result<()>;
    fn has_source_component_a8(&mut self, actor: Self::Actor) -> Result<bool>;
    fn definition(&mut self, actor: Self::Actor) -> Result<Self::Definition>;
    fn definition_flags(&mut self, definition: Self::Definition) -> Result<u32>;
    fn definition_head_504(&mut self, definition: Self::Definition) -> Result<Option<Self::Node>>;
    fn organization(&mut self, actor: Self::Actor) -> Result<Option<Self::Organization>>;
    fn organization_member_count(&mut self, organization: Self::Organization) -> Result<i32>;
    fn organization_member(
        &mut self,
        organization: Self::Organization,
        index: i32,
    ) -> Result<Option<Self::Actor>>;
    fn element(&mut self, actor: Self::Actor) -> Result<Option<Self::Element>>;
    fn element_parent(&mut self, element: Self::Element) -> Result<Option<Self::Actor>>;
}

fn insert<W: World>(world: &mut W, actor: W::Actor, source: W::Node) -> Result<()> {
    let old = world.actor_head(actor)?;
    let node = world.allocate_node(actor)?;
    let value = world.node_value(source)?;
    world.store_node_value(node, value)?;
    world.store_node_next(node, old)?;
    let tail = world.actor_tail(actor)?;
    world.store_actor_head(actor, Some(node))?;
    if tail.is_none() {
        world.store_actor_tail(actor, Some(node))?;
    }
    Ok(())
}
/// Original4364C0 iterator with attached-only byte zero. It rereads current
/// signed count and slot after every skipped null. The selected slot is read
/// again by the caller, as native does after the iterator returns.
fn next_member<W: World>(world: &mut W, org: W::Organization, index: i32) -> Result<Option<i32>> {
    let mut index = index.wrapping_add(1);
    while index < world.organization_member_count(org)? {
        if world.organization_member(org, index)?.is_some() {
            return Ok(Some(index));
        }
        index = index.wrapping_add(1);
    }
    Ok(None)
}
fn refresh_one<W: World>(world: &mut W, actor: W::Actor) -> Result<()> {
    while let Some(head) = world.actor_head(actor)? {
        let next = world.node_next(head)?;
        world.store_actor_head(actor, next)?;
        if next.is_none() {
            world.store_actor_tail(actor, None)?;
        }
        world.free_node(actor, head)?;
    }
    if world.has_source_component_a8(actor)? {
        let definition = world.definition(actor)?;
        if world.definition_flags(definition)? & (1 << 18) != 0 {
            let mut source = world.definition_head_504(definition)?;
            while let Some(node) = source {
                insert(world, actor, node)?;
                source = world.node_next(node)?;
            }
        }
    }
    if let Some(org) = world.organization(actor)? {
        let mut index = -1;
        while let Some(selected) = next_member(world, org, index)? {
            index = selected;
            let member = world.organization_member(org, index)?.ok_or_else(|| {
                Error::Callback(
                    "selected actor list member became null (native unchecked read)".into(),
                )
            })?;
            let mut source = world.actor_head(member)?;
            while let Some(node) = source {
                let mut destination = world.actor_head(actor)?;
                let mut found = false;
                if destination.is_some() {
                    let value = world.node_value(node)?;
                    while let Some(other) = destination {
                        if world.node_value(other)? == value {
                            found = true;
                            break;
                        }
                        destination = world.node_next(other)?;
                    }
                }
                if !found {
                    insert(world, actor, node)?;
                }
                source = world.node_next(node)?;
            }
        }
    }
    Ok(())
}
fn parent<W: World>(world: &mut W, actor: W::Actor) -> Result<Option<W::Actor>> {
    let Some(element) = world.element(actor)? else {
        return Ok(None);
    };
    world.element_parent(element)
}
/// Entire4D4CF0 and its terminal4D4E50 parent propagation. Tail iteration
/// replaces native recursion without changing any read/write/service order.
/// No cycle/default-success guard is invented; the host must provide a valid
/// native actor graph or stop with its explicit missing-dependency error.
pub fn refresh<W: World>(world: &mut W, mut actor: W::Actor) -> Result<()> {
    loop {
        refresh_one(world, actor)?;
        let Some(next) = parent(world, actor)? else {
            return Ok(());
        };
        actor = next;
    }
}
///4D4E50 only: absent Element or null parent performs no list refresh.
pub fn refresh_parent<W: World>(world: &mut W, actor: W::Actor) -> Result<()> {
    if let Some(parent) = parent(world, actor)? {
        refresh(world, parent)?;
    }
    Ok(())
}
