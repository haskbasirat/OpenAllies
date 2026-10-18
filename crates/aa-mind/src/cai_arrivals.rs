//! The one retained Regiment arrival list at native +2BC/+2C0.
//!
//!4D0900 nodes contain a raw GTID and next link. A typed growable linked list
//!preserves that logical order and head/tail lifetime without native pointers
//!or a second count/head/tail mirror. Allocation failure remains the Rust
//!allocator boundary; the original null-allocation path dereferences null.
use serde::Serialize;
use std::collections::{LinkedList, linked_list};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct List {
    nodes: LinkedList<u32>,
}
impl List {
    ///Only the proven empty526260 constructor or saved zero-count list uses
    ///this empty representation. Runtime publication checks the actual owner.
    pub fn from_constructor() -> Self {
        Self {
            nodes: LinkedList::new(),
        }
    }
    ///528C20 appends each saved GTID in serialized order, including duplicates
    ///and zero payloads. Consumes the decoded payload rather than retaining it.
    pub fn from_saved(values: Vec<u32>) -> Self {
        Self {
            nodes: values.into_iter().collect(),
        }
    }
    ///524B80 allocates one4D0900 node, makes it the head, and sets the tail only
    ///when the old list was empty. LinkedList owns both links, with no cap.
    pub fn push_front(&mut self, gtid: u32) {
        self.nodes.push_front(gtid)
    }
    ///524F90..524FB3 unlinks and frees one current head; an exhausted list also
    ///has no tail. Sleep resolves the peeked GTID BEFORE calling this method.
    pub fn pop_front(&mut self) -> Option<u32> {
        self.nodes.pop_front()
    }
    pub fn front(&self) -> Option<u32> {
        self.nodes.front().copied()
    }
    pub fn back(&self) -> Option<u32> {
        self.nodes.back().copied()
    }
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
    pub fn len(&self) -> usize {
        self.nodes.len()
    }
    pub fn iter(&self) -> linked_list::Iter<'_, u32> {
        self.nodes.iter()
    }
}
impl From<Vec<u32>> for List {
    fn from(values: Vec<u32>) -> Self {
        Self::from_saved(values)
    }
}

///The exact524F53..524FE1 consumer, split only at actual owner operations.
///The registry lookup precedes current-head unlink/free; membership callbacks
///run after the node has gone, and may add further arrivals to the same list.
pub trait DrainWorld {
    fn front(&mut self, mind: crate::MindId, arena: &crate::Arena) -> crate::Result<Option<u32>>;
    fn resolve(&mut self, gtid: u32) -> crate::Result<Option<u32>>;
    fn pop_front(&mut self, mind: crate::MindId, arena: &crate::Arena) -> crate::Result<()>;
    fn member_organization(&mut self, object: u32) -> crate::Result<u32>;
    fn organization(&mut self, mind: crate::MindId, arena: &crate::Arena) -> crate::Result<u32>;
    fn member_arrived(
        &mut self,
        mind: crate::MindId,
        object: u32,
        arena: &mut crate::Arena,
    ) -> crate::Result<()>;
}
pub fn drain(
    mind: crate::MindId,
    arena: &mut crate::Arena,
    world: &mut impl DrainWorld,
) -> crate::Result<()> {
    while let Some(gtid) = world.front(mind, arena)? {
        let object = world.resolve(gtid)?;
        world.pop_front(mind, arena)?;
        if let Some(object) = object
            && world.member_organization(object)? == world.organization(mind, arena)?
        {
            world.member_arrived(mind, object, arena)?;
        }
    }
    Ok(())
}
