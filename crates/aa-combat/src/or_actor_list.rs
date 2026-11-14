//! Original cached operational-range actor list of an organization component
//! (`UAICachedORActorListTimeout`), read by element target choice
//! (`535910` -> `534700`) through the actor iterator.
//!
//! * `5781B0` on the component ([`use_cached`]): the SVar
//!   `UAICachedORActorListTimeout` (`4B2E60`, kind 2, its own static cache
//!   at `B3D6A0`) must be above 0.0, and `4CEDE0` (moving) on the component
//!   actor (+4) must be false.
//! * `57A780` on the component ([`refresh`]): the list at +C4 (head), +C8
//!   (tail) and +CC (refresh time) is rebuilt when it is empty or when
//!   `time - refresh` (world +E0, unstored at the thread precision) is not
//!   below the timeout (same SVar, static cache `B3D6A8`). The rebuild runs a
//!   stack iterator (`558720`, `5580F0(x, y, radius)` at the component
//!   actor's +20/+24 with its definition's +300 operational range, `558750`
//!   until null, `557FD0`), frees the old nodes from the head, and PREPENDS
//!   one 8-byte node {GTID +14, next} per actor found, so the list holds the
//!   query in reverse order. +CC then takes the world time.
//! * `558150(component)` on an actor iterator ([`begin`]): without the cache
//!   `5580F0(x, y, radius)` runs on the iterator with the `577F20` range
//!   (definition +300, loaded and stored as binary32); with it the iterator
//!   takes the component (+A4), the refreshed list (+9C) and its head (+A0).
//! * `558750` with +A4 set ([`next_cached`]): walks the nodes from +A0,
//!   resolving each GTID (`[B364C0]`; a mismatch or a null entry is skipped),
//!   and returns the first actor that `578470` on the component
//!   ([`crate::engagement::within_operating_radius`]) accepts.
use crate::Result;
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

/// Organization component +C4/+C8/+CC.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CachedActorList {
    /// Node GTIDs from the head (+C4) to the tail (+C8).
    pub gtids: Vec<u32>,
    /// +CC: world time of the last rebuild (binary32).
    pub refreshed_bits: u32,
}

/// Services of `5781B0`, `57A780` and `558150`, in native order.
pub trait OrActorListWorld {
    /// The SVar `UAICachedORActorListTimeout` (binary32).
    fn timeout(&mut self) -> Result<u32>;
    /// `4CEDE0` on the component actor (+4).
    fn moving(&mut self) -> Result<bool>;
    /// World (`[B36534]`) +E0 time (binary32).
    fn time(&mut self) -> Result<u32>;
    /// Component actor (+4) +20/+24 (binary32).
    fn position(&mut self) -> Result<[u32; 2]>;
    /// Component actor definition +300 (operational range, binary32).
    fn operational_range(&mut self) -> Result<u32>;
    /// The `57A780` stack iterator: `558720`, `5580F0(x, y, radius)`, then
    /// the +14 GTID of every actor `558750` returns, in order, until null
    /// (`557FD0` closes it).
    fn query(&mut self, position: [u32; 2], radius: u32) -> Result<Vec<u32>>;
    /// `5580F0(x, y, radius)` on the caller's iterator (`558150` without the
    /// cache).
    fn begin_live(&mut self, position: [u32; 2], radius: u32) -> Result<()>;
}

/// Services of the cached branch of `558750`.
pub trait CachedNextWorld {
    /// GTID registry `[B364C0]`: `None` for a generation mismatch or a null
    /// entry.
    fn resolve(&mut self, gtid: u32) -> Result<Option<u32>>;
    /// `578470(actor)` on the component
    /// ([`crate::engagement::within_operating_radius`]).
    fn within_range(&mut self, actor: u32) -> Result<bool>;
}

/// The iterator's +A0: the next node, as an index into
/// [`CachedActorList::gtids`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CachedCursor {
    pub next: usize,
}

fn n(bits: u32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(bits)?)
}

/// `5781B0`: true when the cached list is used.
pub fn use_cached(world: &mut impl OrActorListWorld) -> Result<bool> {
    // fcomp [9F32DC] (0.0); C0 or C3 alone: not above zero.
    if n(world.timeout()?)?.compare(&Binary::from_i32(0)) != Ordering::Greater {
        return Ok(false);
    }
    Ok(!world.moving()?)
}

/// `57A780`: rebuild the list when it is empty or out of date. Returns true
/// when it was rebuilt (the original returns the list address).
pub fn refresh(
    list: &mut CachedActorList,
    world: &mut impl OrActorListWorld,
    p: Precision,
) -> Result<bool> {
    let timeout = world.timeout()?;
    let now = world.time()?;
    if !list.gtids.is_empty() {
        let age = n(now)?.sub(&n(list.refreshed_bits)?, p)?;
        // fcomp [timeout]; C0 alone (below) keeps the list.
        if age.compare(&n(timeout)?) == Ordering::Less {
            return Ok(false);
        }
    }
    let radius = world.operational_range()?;
    let position = world.position()?;
    let found = world.query(position, radius)?;
    list.gtids.clear();
    list.gtids.extend(found.into_iter().rev());
    list.refreshed_bits = world.time()?;
    Ok(true)
}

/// `558150(component)` on an actor iterator: `None` after `5580F0` started a
/// live circle query, otherwise the cursor at the head of the refreshed
/// list.
pub fn begin(
    list: &mut CachedActorList,
    world: &mut impl OrActorListWorld,
    p: Precision,
) -> Result<Option<CachedCursor>> {
    if !use_cached(world)? {
        let position = world.position()?;
        // 577F20 loads +300; 558176 stores it as binary32.
        let radius = n(world.operational_range()?)?.store_f32()?;
        world.begin_live(position, radius)?;
        return Ok(None);
    }
    refresh(list, world, p)?;
    Ok(Some(CachedCursor::default()))
}

/// `558750` on an iterator in cached mode (+A4 set): the next actor of the
/// list within the operational range, or `None` at the end.
pub fn next_cached(
    cursor: &mut CachedCursor,
    list: &CachedActorList,
    world: &mut impl CachedNextWorld,
) -> Result<Option<u32>> {
    while let Some(&gtid) = list.gtids.get(cursor.next) {
        let actor = world.resolve(gtid)?;
        cursor.next += 1;
        if let Some(actor) = actor
            && world.within_range(actor)?
        {
            return Ok(Some(actor));
        }
    }
    Ok(None)
}
