//! Original SAI actor-by-region list upkeep: `7A9940` (take an actor out
//! of its region's lists), `79E030` (an actor's record list and region
//! move), `7BD310` (the region move between the +0x28 and +0x34 lists),
//! `7BD2A0` / `7B99F0` (removals from the per-kingdom lists), `7BC330` /
//! `7BC470` (additions, growing the table first) and `7ADFF0` (the record
//! owner's +0x4C list: push at the head, or remove).
//!
//! The branch order, the argument checks and the list effects follow the
//! original instructions: `4D0900` pushes a node at the head (the tail takes
//! it when the list was empty) and `4F1430` removes the first node holding
//! the value. The region lookup `4CF010` (`7AA3D0` on `[B3652C]`), `79F390`,
//! the watcher updates `7BCDB0` / `7BB120` and the actor virtual +0x108 are
//! required world callbacks in native order. Proven by
//! research/phase8-ai/native_actor_map.py.
use crate::{Error, Result};
use serde::{Deserialize, Serialize};

/// A native object (pointer in the original); 0 is null.
pub type Ref = u32;

/// The two per-kingdom list tables of an SAI region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Table {
    /// Region +0x28 lists, count +0x2C.
    At28,
    /// Region +0x34 lists, count +0x38.
    At34,
}

/// A `{head, tail}` list of `{value, next}` nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum List {
    /// Region table entry (the list pointer stored there; 0 is null).
    Region(Ref),
    /// Record owner +0x4C.
    Owner(Ref),
}

pub trait ActorMapWorld {
    /// `4CF010(actor)`: the SAI region at the actor's position (0 when the
    /// SAI map is absent or not ready).
    fn region_of(&mut self, actor: Ref) -> Result<Ref>;
    /// Actor virtual +0x108.
    fn kingdom(&mut self, actor: Ref) -> Result<Ref>;
    /// Actor +0x140 (tested for zero).
    fn word_140(&mut self, actor: Ref) -> Result<u32>;
    /// Actor +0x14.
    fn handle_of(&mut self, actor: Ref) -> Result<u32>;
    /// Kingdom +0x188.
    fn kingdom_index(&mut self, kingdom: Ref) -> Result<i32>;
    /// `79F390(actor)`.
    fn test_79f390(&mut self, actor: Ref) -> Result<bool>;
    /// Actor +0x154 (`4D03C0`) and that record's +0xC.
    fn record(&mut self, actor: Ref) -> Result<Ref>;
    fn record_owner(&mut self, record: Ref) -> Result<Ref>;
    /// Region table count and entry.
    fn table_count(&mut self, region: Ref, table: Table) -> Result<i32>;
    fn table_list(&mut self, region: Ref, table: Table, index: i32) -> Result<Ref>;
    /// Region +0x20 count and +0x1C entries.
    fn watcher_count(&mut self, region: Ref) -> Result<i32>;
    fn watcher(&mut self, region: Ref, index: i32) -> Result<Ref>;
    /// `4F1430`: remove the first node holding the value; false when none.
    fn remove(&mut self, list: List, value: u32) -> Result<bool>;
    /// `4D0900` at the head (the tail takes the node when it was null).
    fn push_front(&mut self, list: List, value: u32) -> Result<()>;
    /// `7BCDB0(actor)` on a watcher (after a removal from +0x28).
    fn watcher_update(&mut self, watcher: Ref, actor: Ref) -> Result<()>;
    /// `7BB120(actor)` on a watcher (after an addition to +0x28).
    fn watcher_added(&mut self, watcher: Ref, actor: Ref) -> Result<()>;
    /// Record +0x14 store (the additions write binary32 -1.0).
    fn set_record_14(&mut self, record: Ref, bits: u32) -> Result<()>;
    /// Append a new empty list to the region table (count + 1; the native
    /// vector grows through `4849F0`).
    fn append_list(&mut self, region: Ref, table: Table) -> Result<()>;
}

/// The record +0x14 value the additions store.
pub const RECORD_14_RESET: u32 = 0xbf80_0000;

fn null(what: &str) -> Error {
    Error::World(format!("native null dereference: {what}"))
}

fn region_list<W: ActorMapWorld>(world: &mut W, region: Ref, table: Table, index: i32) -> Result<List> {
    match world.table_list(region, table, index)? {
        0 => Err(null("SAI region list")),
        list => Ok(List::Region(list)),
    }
}

/// `7BD2A0(actor, kingdom)` on a region: the actor's handle leaves the
/// +0x28 list of the kingdom index, then every +0x1C watcher, last first,
/// runs `7BCDB0(actor)`.
pub fn remove_28<W: ActorMapWorld>(region: Ref, actor: Ref, kingdom: Ref, world: &mut W) -> Result<()> {
    if kingdom == 0 || actor == 0 {
        return Ok(());
    }
    let index = world.kingdom_index(kingdom)?;
    if world.word_140(actor)? != 0 || index >= world.table_count(region, Table::At28)? {
        return Ok(());
    }
    let handle = world.handle_of(actor)?;
    let list = region_list(world, region, Table::At28, index)?;
    world.remove(list, handle)?;
    let mut watcher = world.watcher_count(region)? - 1;
    while watcher >= 0 {
        let w = world.watcher(region, watcher)?;
        world.watcher_update(w, actor)?;
        watcher -= 1;
    }
    Ok(())
}

/// `7B99F0(actor, kingdom)` on a region: the handle leaves the +0x34 list of
/// the kingdom index, or else its +0x28 list when that exists.
pub fn remove_34<W: ActorMapWorld>(region: Ref, actor: Ref, kingdom: Ref, world: &mut W) -> Result<()> {
    if kingdom == 0 {
        return Ok(());
    }
    let index = world.kingdom_index(kingdom)?;
    if index >= world.table_count(region, Table::At34)? || world.word_140(actor)? != 0 {
        return Ok(());
    }
    let handle = world.handle_of(actor)?;
    let list = region_list(world, region, Table::At34, index)?;
    if world.remove(list, handle)? {
        return Ok(());
    }
    if world.table_count(region, Table::At28)? <= index {
        return Ok(());
    }
    let list = world.table_list(region, Table::At28, index)?;
    if list == 0 {
        return Ok(());
    }
    let handle = world.handle_of(actor)?;
    world.remove(List::Region(list), handle)?;
    Ok(())
}

/// `7A9940(actor, kingdom)`: with a kingdom (the argument, or else the
/// actor's) and +0x140 clear, `7B99F0` when `79F390` holds, else `7BD2A0`,
/// on the region `4CF010` found first.
pub fn remove_from_region<W: ActorMapWorld>(actor: Ref, kingdom: Ref, world: &mut W) -> Result<()> {
    let region = world.region_of(actor)?;
    let kingdom = if kingdom != 0 { kingdom } else { world.kingdom(actor)? };
    if kingdom == 0 || world.word_140(actor)? != 0 {
        return Ok(());
    }
    let first = world.test_79f390(actor)?;
    if region == 0 {
        // Both callees read a region field before any further call.
        return Err(null("4CF010 region"));
    }
    if first {
        remove_34(region, actor, kingdom, world)
    } else {
        remove_28(region, actor, kingdom, world)
    }
}

/// `7ADFF0(actor, add)` on a record owner: the actor's handle (0 for a null
/// actor) is pushed at the head of +0x4C, or removed from it.
pub fn owner_list<W: ActorMapWorld>(owner: Ref, actor: Ref, add: bool, world: &mut W) -> Result<()> {
    let handle = if actor == 0 { 0 } else { world.handle_of(actor)? };
    if add {
        world.push_front(List::Owner(owner), handle)
    } else {
        world.remove(List::Owner(owner), handle).map(drop)
    }
}

/// `7BD310(actor, flag)` on a region: with the flag `7BD2A0` then `7BC470`,
/// else `7B99F0` then `7BC330`, all with the actor's kingdom.
pub fn region_move<W: ActorMapWorld>(region: Ref, actor: Ref, flag: bool, world: &mut W) -> Result<()> {
    let kingdom = world.kingdom(actor)?;
    if flag {
        remove_28(region, actor, kingdom, world)?;
        add(region, actor, kingdom, Table::At34, world)
    } else {
        remove_34(region, actor, kingdom, world)?;
        add(region, actor, kingdom, Table::At28, world)
    }
}

/// `7BC330(actor, kingdom)` (+0x28, then `7BB120(actor)` on every +0x1C
/// watcher, last first) and `7BC470(actor, kingdom)` (+0x34): the actor's
/// +0x154 record (dereferenced) takes -1.0 at +0x14 when present; with a
/// kingdom, the table grows by empty lists until it holds the kingdom
/// index, and the actor's handle (0 for a null actor) is pushed at the head
/// of that index's list.
pub fn add<W: ActorMapWorld>(region: Ref, actor: Ref, kingdom: Ref, table: Table, world: &mut W) -> Result<()> {
    if actor == 0 {
        return Err(null("actor +0x154"));
    }
    let record = world.record(actor)?;
    if record != 0 {
        world.set_record_14(record, RECORD_14_RESET)?;
    }
    if kingdom == 0 {
        return Ok(());
    }
    let index = world.kingdom_index(kingdom)?;
    while world.table_count(region, table)? <= index {
        world.append_list(region, table)?;
    }
    let handle = world.handle_of(actor)?;
    let list = region_list(world, region, table, index)?;
    world.push_front(list, handle)?;
    if table == Table::At28 {
        let mut watcher = world.watcher_count(region)? - 1;
        while watcher >= 0 {
            let w = world.watcher(region, watcher)?;
            world.watcher_added(w, actor)?;
            watcher -= 1;
        }
    }
    Ok(())
}

/// `79E030(actor, flag)`: the record owner's list (`7ADFF0`) when the
/// actor's +0x154 record has an owner, then the region move (`7BD310`) when
/// `4CF010` finds a region.
pub fn refresh<W: ActorMapWorld>(actor: Ref, flag: bool, world: &mut W) -> Result<()> {
    let record = world.record(actor)?;
    let owner = if record == 0 { 0 } else { world.record_owner(record)? };
    if owner != 0 {
        owner_list(owner, actor, flag, world)?;
    }
    let region = world.region_of(actor)?;
    if region != 0 {
        region_move(region, actor, flag, world)?;
    }
    Ok(())
}
