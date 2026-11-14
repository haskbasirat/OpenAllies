//! Original kingdom memory record update `4ECB90` (thiscall kingdom,
//! `(actor, keep_owner)`, `ret 8`), the re-reveal that
//! [`crate::ownership`] calls per kingdom that saw the actor
//! (`ownership::World::reveal`).
//!
//! Nothing happens (AL 0) for a null actor, an actor whose `+190` word is
//! nonzero, a kingdom whose virtual `+8C` is false or an empty record slot
//! (kingdom `+5C` array at the actor's `+18C` index). With `keep_owner` set,
//! a record whose `+14` already equals the kingdom's virtual `+108` is left
//! (AL 0). Otherwise, between the two notifications `79DEB0` / `79E7B0` on
//! `[B36528]` (made only when that object exists and its `+20` byte is set;
//! each re-reads the record slot), the record is either refreshed with
//! `4EC4A0(record)` (`keep_owner` clear) or gets `+14` = virtual `+108`
//! (called again); AL 1.

use crate::Result;

/// The reads and calls of `4ECB90`, in native order.
pub trait MemoryWorld {
    type Actor: Copy;
    type Record: Copy;
    /// Fact: actor `+190` word.
    fn actor_word_190(&mut self, actor: Self::Actor) -> Result<u32>;
    /// Kingdom virtual `+8C`: AL.
    fn kingdom_8c(&mut self) -> Result<bool>;
    /// Fact: actor `+18C` index.
    fn actor_index(&mut self, actor: Self::Actor) -> Result<u32>;
    /// Fact: kingdom `+5C` slot `index` (null is `None`).
    fn record(&mut self, index: u32) -> Result<Option<Self::Record>>;
    /// Kingdom virtual `+108`.
    fn kingdom_108(&mut self) -> Result<u32>;
    /// Fact: record `+14`.
    fn record_word_14(&mut self, record: Self::Record) -> Result<u32>;
    /// Record `+14` store.
    fn set_record_word_14(&mut self, record: Self::Record, value: u32) -> Result<()>;
    /// `4EC4A0(record)` on the kingdom.
    fn refresh_record(&mut self, record: Self::Record) -> Result<()>;
    /// Fact: `[B36528]` exists and its `+20` byte is set.
    fn notifications(&mut self) -> Result<bool>;
    /// `79DEB0(slot, actor)` / `79E7B0(slot, actor)` on `[B36528]`.
    fn notify_before(&mut self, slot: Option<Self::Record>, actor: Self::Actor) -> Result<()>;
    fn notify_after(&mut self, slot: Option<Self::Record>, actor: Self::Actor) -> Result<()>;
}

/// `4ECB90`: the returned AL.
pub fn update_memory<W: MemoryWorld + ?Sized>(
    world: &mut W,
    actor: Option<W::Actor>,
    keep_owner: bool,
) -> Result<bool> {
    let Some(actor) = actor else { return Ok(false) };
    if world.actor_word_190(actor)? != 0 || !world.kingdom_8c()? {
        return Ok(false);
    }
    let index = world.actor_index(actor)?;
    let Some(record) = world.record(index)? else { return Ok(false) };
    if keep_owner && world.record_word_14(record)? == world.kingdom_108()? {
        return Ok(false);
    }
    if world.notifications()? {
        let slot = world.record(index)?;
        world.notify_before(slot, actor)?;
    }
    if keep_owner {
        let owner = world.kingdom_108()?;
        world.set_record_word_14(record, owner)?;
    } else {
        world.refresh_record(record)?;
    }
    if world.notifications()? {
        let slot = world.record(index)?;
        world.notify_after(slot, actor)?;
    }
    Ok(true)
}
