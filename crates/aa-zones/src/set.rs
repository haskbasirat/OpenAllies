//! Interned zone sets (`.\Source\Sim\Zones.cpp`): the per-tile set of zone
//! sources `(actor, zone-type flags)` that covers a map tile.
//!
//! Native layout. A tile's `+4` word points at a reference-counted, globally
//! interned set object of one of two classes:
//! - leaf (vtable `0xA323E8`, 0x14 bytes): `{vtable, refs +4, actor +8,
//!   flags u16 +0xC, hash link +0x10}`, interned in the GWorld+0x19C table by
//!   `(actor, flags)` (`0x516360` lookup/create, `0x5162D0` constructor,
//!   `0x515FA0` insert, `0x514EF0` rehash, `0x516160`/`0x515C90` removal);
//! - union (vtable `0xA32388`, 0x20 bytes): `{vtable, refs +4, tail +8 (leaf or
//!   union), head +0xC (always a leaf), scratch back-link +0x10, hash link
//!   +0x14, hash sum +0x18, length +0x1C}`, interned in the GWorld+0x1A0 table
//!   (`0x5161C0` lookup/create, `0x515FF0` constructor, `0x514E90` insert,
//!   `0x514800` rehash, `0x5160D0`/`0x515D20` removal) under the structural
//!   equality `0x5154E0` (+ `0x514960`).
//!
//! Iteration order (every union virtual): the head, then the tail's head, and
//! so on, ending with the final leaf; [`ZonePool::entries`] returns it.
//!
//! Interning semantics proven by the oracle (research/phase8-zones/
//! native-zone-sets.json):
//! - `cons(tail, head)` returns the existing node with the same `(tail, head)`;
//!   when `tail` is a leaf it also returns an existing node `(head, tail)` (the
//!   swapped two-element list, `0x5154E0` second test). The deeper branch of
//!   `0x5154E0` (`0x514960`) needs a list holding one leaf twice, which the
//!   add/remove operations never build, so it never matches (not reached by the
//!   oracle either). Hence a two-element list keeps the order of the first
//!   equal set alive anywhere on the map, and longer lists inherit that order
//!   through their shared tails.
//! - Objects are destroyed when their reference count drops to zero (union
//!   destructor `0x5160D0` releases the head first, then the tail), so the
//!   interned order depends on which sets are alive. [`ZonePool`] keeps the
//!   same reference counts (tile references plus child references).
//!
//! Hash buckets, the auto-grow rehash sizes and object addresses are not
//! observable (each key has at most one live match) and are not modelled: the
//! native tables grow without bound, so no capacity limit is removed or kept.
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Native actor identity (the actor pointer; equality is identity).
pub type ActorKey = u32;

/// One leaf: a zone source actor and its zone-type flags (`1 << type`, OR-ed
/// for several zone types of the same actor).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Entry {
    pub actor: ActorKey,
    pub flags: u16,
}

/// Union node identifier inside a [`ZonePool`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NodeId(pub u32);

/// A non-empty interned set: a leaf (identified by value) or a union node.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SetRef {
    Leaf(Entry),
    Node(NodeId),
}

#[derive(Clone, Debug)]
struct Node {
    tail: SetRef,
    head: Entry,
    refs: u32,
    len: u32,
}

/// FxHash (the rustc hasher) for the lookup tables the zone code hits on
/// every tile change. No table's iteration order is observable (see the
/// module doc), so the hasher only changes the speed.
#[derive(Clone, Copy, Debug, Default)]
pub struct FxHasher {
    hash: u64,
}

impl FxHasher {
    fn add(&mut self, word: u64) {
        self.hash = (self.hash.rotate_left(5) ^ word).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }
}

impl std::hash::Hasher for FxHasher {
    fn write(&mut self, bytes: &[u8]) {
        for chunk in bytes.chunks(8) {
            let mut w = [0u8; 8];
            w[..chunk.len()].copy_from_slice(chunk);
            self.add(u64::from_le_bytes(w));
        }
    }
    fn write_u8(&mut self, i: u8) {
        self.add(u64::from(i));
    }
    fn write_u16(&mut self, i: u16) {
        self.add(u64::from(i));
    }
    fn write_u32(&mut self, i: u32) {
        self.add(u64::from(i));
    }
    fn write_u64(&mut self, i: u64) {
        self.add(i);
    }
    fn write_usize(&mut self, i: usize) {
        self.add(i as u64);
    }
    fn finish(&self) -> u64 {
        self.hash
    }
}

/// [`FxHasher`] builder.
pub type FxBuild = std::hash::BuildHasherDefault<FxHasher>;

/// The two global intern tables (GWorld+0x19C leaves, GWorld+0x1A0 unions).
#[derive(Clone, Debug, Default)]
pub struct ZonePool {
    nodes: Vec<Option<Node>>,
    free: Vec<u32>,
    by_pair: HashMap<(SetRef, Entry), NodeId, FxBuild>,
    leaves: HashMap<Entry, u32, FxBuild>,
}

/// The entries of a set in native iteration order ([`ZonePool::iter`]).
pub struct SetIter<'a> {
    pool: &'a ZonePool,
    cur: Option<SetRef>,
}

impl Iterator for SetIter<'_> {
    type Item = Entry;
    fn next(&mut self) -> Option<Entry> {
        match self.cur? {
            SetRef::Leaf(e) => {
                self.cur = None;
                Some(e)
            }
            SetRef::Node(id) => {
                let n = self.pool.nodes[id.0 as usize].as_ref().expect("chain checked by ZonePool::iter");
                self.cur = Some(n.tail);
                Some(n.head)
            }
        }
    }
}

/// One interned object as reported by [`ZonePool::live`]: its entries in
/// iteration order and its reference count.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LiveSet {
    pub entries: Vec<Entry>,
    pub refs: u32,
}

impl ZonePool {
    pub fn new() -> Self {
        Self::default()
    }

    fn node(&self, id: NodeId) -> Result<&Node> {
        self.nodes
            .get(id.0 as usize)
            .and_then(Option::as_ref)
            .ok_or(Error::StaleZoneSet(id.0))
    }

    fn node_mut(&mut self, id: NodeId) -> Result<&mut Node> {
        self.nodes
            .get_mut(id.0 as usize)
            .and_then(Option::as_mut)
            .ok_or(Error::StaleZoneSet(id.0))
    }

    /// Native iteration order of a set (head first, final leaf last).
    pub fn entries(&self, set: Option<SetRef>) -> Result<Vec<Entry>> {
        let mut out = Vec::new();
        let mut cur = set;
        while let Some(s) = cur {
            match s {
                SetRef::Leaf(e) => {
                    out.push(e);
                    cur = None;
                }
                SetRef::Node(id) => {
                    let n = self.node(id)?;
                    out.push(n.head);
                    cur = Some(n.tail);
                }
            }
        }
        Ok(out)
    }

    /// [`Self::entries`] without allocating: the chain is checked first, so
    /// a stale node fails before any entry is visited, as with `entries`.
    pub fn iter(&self, set: Option<SetRef>) -> Result<SetIter<'_>> {
        let mut cur = set;
        while let Some(SetRef::Node(id)) = cur {
            cur = Some(self.node(id)?.tail);
        }
        Ok(SetIter { pool: self, cur: set })
    }

    /// Set length (virtual `+0x10`: leaf `0x4F0500` = 1, union `0x5160B0` = +0x1C).
    pub fn len(&self, set: SetRef) -> Result<u32> {
        Ok(match set {
            SetRef::Leaf(_) => 1,
            SetRef::Node(id) => self.node(id)?.len,
        })
    }

    /// Tail (`+8`) of a union node; a leaf has none.
    pub fn tail(&self, set: SetRef) -> Result<SetRef> {
        match set {
            SetRef::Leaf(e) => Err(Error::StaleLeaf(e.actor, e.flags)),
            SetRef::Node(id) => Ok(self.node(id)?.tail),
        }
    }

    pub fn refs(&self, set: SetRef) -> Result<u32> {
        match set {
            SetRef::Leaf(e) => self.leaves.get(&e).copied().ok_or(Error::StaleLeaf(e.actor, e.flags)),
            SetRef::Node(id) => Ok(self.node(id)?.refs),
        }
    }

    /// `0x516360`: intern a leaf; `None` for flags 0 (the native returns null
    /// without allocating).
    fn leaf(&mut self, actor: ActorKey, flags: u16) -> Option<SetRef> {
        if flags == 0 {
            return None;
        }
        let e = Entry { actor, flags };
        self.leaves.entry(e).or_insert(0);
        Some(SetRef::Leaf(e))
    }

    fn incref(&mut self, s: SetRef) -> Result<()> {
        match s {
            SetRef::Leaf(e) => {
                let r = self.leaves.get_mut(&e).ok_or(Error::StaleLeaf(e.actor, e.flags))?;
                *r = r.wrapping_add(1);
            }
            SetRef::Node(id) => {
                let n = self.node_mut(id)?;
                n.refs = n.refs.wrapping_add(1);
            }
        }
        Ok(())
    }

    /// Native decrement-and-destroy (`refs -= 1; if 0: virtual +0(1)`).
    fn decref(&mut self, s: SetRef) -> Result<()> {
        match s {
            SetRef::Leaf(e) => {
                let r = self.leaves.get_mut(&e).ok_or(Error::StaleLeaf(e.actor, e.flags))?;
                *r = r.wrapping_sub(1);
                if *r == 0 {
                    // 0x516340 -> 0x516160: remove from the leaf table.
                    self.leaves.remove(&e);
                }
            }
            SetRef::Node(id) => {
                let n = self.node_mut(id)?;
                n.refs = n.refs.wrapping_sub(1);
                if n.refs == 0 {
                    // 0x516270 -> 0x5160D0: unlink, release head (+0xC), then tail (+8).
                    let n = self.nodes[id.0 as usize].take().ok_or(Error::StaleZoneSet(id.0))?;
                    self.by_pair.remove(&(n.tail, n.head));
                    self.free.push(id.0);
                    self.decref(SetRef::Leaf(n.head))?;
                    self.decref(n.tail)?;
                }
            }
        }
        Ok(())
    }

    /// `0x5161C0(tail, head)`: interned union node (see module doc).
    fn cons(&mut self, tail: SetRef, head: Entry) -> Result<SetRef> {
        if let Some(&id) = self.by_pair.get(&(tail, head)) {
            return Ok(SetRef::Node(id));
        }
        if let SetRef::Leaf(t) = tail
            && let Some(&id) = self.by_pair.get(&(SetRef::Leaf(head), t))
        {
            return Ok(SetRef::Node(id));
        }
        let len = self.len(tail)?.wrapping_add(1);
        let node = Node { tail, head, refs: 0, len };
        let id = match self.free.pop() {
            Some(i) => {
                self.nodes[i as usize] = Some(node);
                NodeId(i)
            }
            None => {
                self.nodes.push(Some(node));
                NodeId((self.nodes.len() - 1) as u32)
            }
        };
        // 0x515FF0 constructor: tail +8 first, then head +0xC.
        self.incref(tail)?;
        self.incref(SetRef::Leaf(head))?;
        self.by_pair.insert((tail, head), id);
        Ok(SetRef::Node(id))
    }

    fn head_tail(&self, id: NodeId) -> Result<(Entry, SetRef)> {
        let n = self.node(id)?;
        Ok((n.head, n.tail))
    }

    /// `0x516290`: re-cons the heads of the walked prefix (nearest first) onto
    /// `new`.
    fn rebuild(&mut self, prefix: &[Entry], mut new: SetRef) -> Result<SetRef> {
        for head in prefix.iter().rev() {
            new = self.cons(new, *head)?;
        }
        Ok(new)
    }

    /// Set virtual `+0x48` (leaf `0x516410`, union `0x516480`): add `flags` to
    /// `actor`'s entry (OR) in place, or prepend a new entry. A new actor with
    /// flags 0 dereferences the null leaf `0x516360` returns
    /// ([`Error::NativeNullLeaf`]).
    pub fn add(&mut self, set: SetRef, actor: ActorKey, flags: u16) -> Result<Option<SetRef>> {
        match set {
            SetRef::Leaf(e) => {
                if e.actor == actor {
                    return Ok(self.leaf(actor, e.flags | flags));
                }
                let leaf = self.leaf(actor, flags).ok_or(Error::NativeNullLeaf)?;
                let SetRef::Leaf(l) = leaf else { unreachable!() };
                self.cons(set, l).map(Some)
            }
            SetRef::Node(start) => {
                let mut prefix = Vec::new();
                let mut node = start;
                loop {
                    let (head, tail) = self.head_tail(node)?;
                    if head.actor == actor {
                        let leaf = self.leaf(actor, head.flags | flags).ok_or(Error::NativeNullLeaf)?;
                        let SetRef::Leaf(l) = leaf else { unreachable!() };
                        let new = self.cons(tail, l)?;
                        return self.rebuild(&prefix, new).map(Some);
                    }
                    match tail {
                        SetRef::Leaf(t) if t.actor == actor => {
                            let leaf = self.leaf(actor, t.flags | flags).ok_or(Error::NativeNullLeaf)?;
                            let new = self.cons(leaf, head)?;
                            return self.rebuild(&prefix, new).map(Some);
                        }
                        SetRef::Node(next) => {
                            prefix.push(head);
                            node = next;
                        }
                        SetRef::Leaf(_) => {
                            let leaf = self.leaf(actor, flags).ok_or(Error::NativeNullLeaf)?;
                            let SetRef::Leaf(l) = leaf else { unreachable!() };
                            return self.cons(set, l).map(Some);
                        }
                    }
                }
            }
        }
    }

    /// Set virtual `+0x4C` (leaf `0x516460`, union `0x516560`): XOR `flags` out
    /// of `actor`'s entry; an entry whose flags equal `flags` is removed.
    /// The leaf version does not compare the actor: it yields
    /// `(actor, leaf.flags ^ flags)`. The union version walks into the final
    /// leaf when no entry has `actor` (memory corruption in the original):
    /// [`Error::NativeAbsentActor`].
    pub fn remove(&mut self, set: SetRef, actor: ActorKey, flags: u16) -> Result<Option<SetRef>> {
        match set {
            SetRef::Leaf(e) => Ok(self.leaf(actor, e.flags ^ flags)),
            SetRef::Node(start) => {
                let mut prefix = Vec::new();
                let mut node = start;
                let (head0, _) = self.head_tail(node)?;
                if head0.actor != actor {
                    loop {
                        let (head, tail) = self.head_tail(node)?;
                        match tail {
                            SetRef::Leaf(t) if t.actor == actor => {
                                let new = if t.flags == flags {
                                    SetRef::Leaf(head)
                                } else {
                                    let leaf = self
                                        .leaf(actor, t.flags ^ flags)
                                        .ok_or(Error::NativeNullLeaf)?;
                                    self.cons(leaf, head)?
                                };
                                return self.rebuild(&prefix, new).map(Some);
                            }
                            SetRef::Node(next) => {
                                prefix.push(head);
                                node = next;
                                if self.head_tail(node)?.0.actor == actor {
                                    break;
                                }
                            }
                            SetRef::Leaf(_) => return Err(Error::NativeAbsentActor),
                        }
                    }
                }
                let (head, tail) = self.head_tail(node)?;
                let new = if head.flags == flags {
                    tail
                } else {
                    let leaf = self.leaf(actor, head.flags ^ flags).ok_or(Error::NativeNullLeaf)?;
                    let SetRef::Leaf(l) = leaf else { unreachable!() };
                    self.cons(tail, l)?
                };
                self.rebuild(&prefix, new).map(Some)
            }
        }
    }

    /// Tile reference update of `0x54FA10`/`0x54FA80`: when the new set differs,
    /// increment it first, then release the old one.
    pub fn replace(&mut self, slot: &mut Option<SetRef>, new: Option<SetRef>) -> Result<()> {
        if *slot != new {
            if let Some(n) = new {
                self.incref(n)?;
            }
            if let Some(o) = *slot {
                self.decref(o)?;
            }
            *slot = new;
        }
        Ok(())
    }

    /// `0x54FA10` set part: `tile.set ? set.+0x48(actor, flags) :
    /// 0x516360(actor, flags)`, then the reference update.
    pub fn tile_add(&mut self, slot: &mut Option<SetRef>, actor: ActorKey, flags: u16) -> Result<()> {
        let new = match *slot {
            None => self.leaf(actor, flags),
            Some(s) => self.add(s, actor, flags)?,
        };
        self.replace(slot, new)
    }

    /// `0x54FA80` set part: `tile.set.+0x4C(actor, flags)` (no null check: an
    /// empty tile dereferences null, [`Error::NativeNullSet`]).
    pub fn tile_remove(&mut self, slot: &mut Option<SetRef>, actor: ActorKey, flags: u16) -> Result<()> {
        let s = slot.ok_or(Error::NativeNullSet)?;
        let new = self.remove(s, actor, flags)?;
        self.replace(slot, new)
    }

    /// Every interned object (leaves and unions) with its entries and refs.
    /// Leaves created by `0x516360` and never referenced stay interned with 0.
    pub fn live(&self) -> Result<Vec<LiveSet>> {
        let mut out = Vec::new();
        for (e, r) in &self.leaves {
            out.push(LiveSet { entries: vec![*e], refs: *r });
        }
        for (i, n) in self.nodes.iter().enumerate() {
            if let Some(n) = n {
                out.push(LiveSet {
                    entries: self.entries(Some(SetRef::Node(NodeId(i as u32))))?,
                    refs: n.refs,
                });
            }
        }
        out.sort();
        Ok(out)
    }
}
