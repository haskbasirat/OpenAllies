//! Economy parts of the world start `0x4F1E10` (runs once: world+0x274).
//!
//! 1. For every non-null kingdom of world+0x154: `0x4E3690` promotes the
//!    loaded statistics and `0x58EA00` clears mission scores 47..=55
//!    ([`KingdomStatistics::start_world`]).
//! 2. For every GTID-table object in index order (`0x4EFBE0`) that answers
//!    virtual +0xB4: actor+0x13C becomes 0x38 when different (`0x4CEEC0` /
//!    `0x4CEED0`), and when its owner (virtual +0x108) has word +0x190 == 0 the
//!    owner statistics track it (`(+0x32C).0x58F010(actor)`).
//! 3. Every non-null kingdom without statistics snapshots (+0x338 empty)
//!    takes one at once: `0x4E37D0(0, force = 1)` stores PC(0 + timer), then
//!    timer +0x334 = +0 and a snapshot (`0x58F530` + `0x45F240`).
//!
//! Actor fields, ownership, tracking and the snapshot object are callbacks.
use crate::statistics::KingdomStatistics;
use crate::{EconomyError, F32Bits};

/// The 0x38 value world start stores in actor+0x13C.
pub const WORLD_START_FIELD_13C: u32 = 0x38;

/// Kingdom state touched by world start.
#[derive(Debug)]
pub struct StartingKingdom<'a> {
    pub statistics: &'a mut KingdomStatistics,
    /// +0x334 snapshot timer.
    pub snapshot_timer: &'a mut F32Bits,
    /// +0x338 snapshot list is non-empty.
    pub has_snapshots: bool,
}

pub trait WorldStart {
    /// GTID-table objects in index order.
    fn objects(&mut self) -> Result<Vec<u32>, EconomyError>;
    /// Object virtual +0xB4.
    fn object_active(&mut self, object: u32) -> Result<bool, EconomyError>;
    /// Actor+0x13C (`0x4CEEC0`).
    fn field_13c(&mut self, actor: u32) -> Result<u32, EconomyError>;
    /// `0x4CEED0(value)`.
    fn set_field_13c(&mut self, actor: u32, value: u32) -> Result<(), EconomyError>;
    /// Virtual +0x108 owner. The standalone slice API uses a kingdom index;
    /// the owned API uses the same host identity returned by `kingdoms`.
    fn owner(&mut self, actor: u32) -> Result<Option<usize>, EconomyError>;
    /// Kingdom+0x190.
    fn kingdom_word_190(&mut self, kingdom: usize) -> Result<u32, EconomyError>;
    /// `(kingdom+0x32C).0x58F010(actor)`.
    fn statistics_track(&mut self, kingdom: usize, actor: u32) -> Result<(), EconomyError>;
    /// `0x58F530` snapshot pushed on kingdom+0x338 (`0x45F240`).
    fn statistics_snapshot(&mut self, kingdom: usize) -> Result<(), EconomyError>;
}

/// `0x4F1E10` (economy parts). `started` is world+0x274; `kingdoms` is the
/// world kingdom list with null entries as `None`.
pub fn world_start(
    started: &mut bool,
    kingdoms: &mut [Option<StartingKingdom<'_>>],
    resource_count: usize,
    world: &mut impl WorldStart,
) -> Result<(), EconomyError> {
    if *started {
        return Ok(());
    }
    *started = true;
    for kingdom in kingdoms.iter_mut().flatten() {
        kingdom.statistics.start_world(resource_count);
    }
    for object in world.objects()? {
        if !world.object_active(object)? {
            continue;
        }
        if world.field_13c(object)? != WORLD_START_FIELD_13C {
            world.set_field_13c(object, WORLD_START_FIELD_13C)?;
        }
        if let Some(owner) = world.owner(object)?
            && world.kingdom_word_190(owner)? == 0
        {
            world.statistics_track(owner, object)?;
        }
    }
    for (index, kingdom) in kingdoms.iter_mut().enumerate() {
        let Some(kingdom) = kingdom else { continue };
        if !kingdom.has_snapshots {
            *kingdom.snapshot_timer = F32Bits::ZERO;
            world.statistics_snapshot(index)?;
            kingdom.has_snapshots = true;
        }
    }
    Ok(())
}

/// Owned-world adapter for `4F1E10`. Each callback borrows the authoritative
/// owner for one operation, avoiding simultaneous mutable borrows of the same
/// statistics through `StartingKingdom` and tracking/snapshot callbacks.
pub trait OwnedWorldStart: WorldStart {
    fn started(&mut self) -> Result<bool, EconomyError>;
    fn set_started(&mut self, value: bool) -> Result<(), EconomyError>;
    /// World+154 order, preserving null entries. IDs are host identities.
    fn kingdoms(&mut self) -> Result<Vec<Option<usize>>, EconomyError>;
    /// `4E3690` then `58EA00`, against the kingdom's owned statistics.
    fn promote_and_reset_statistics(
        &mut self,
        kingdom: usize,
        resource_count: usize,
    ) -> Result<(), EconomyError>;
    fn has_snapshots(&mut self, kingdom: usize) -> Result<bool, EconomyError>;
    fn reset_snapshot_timer(&mut self, kingdom: usize) -> Result<(), EconomyError>;
}

/// `4F1E10` using exclusively host-owned state. No list/statistics payload is
/// cloned or published. Callback failure preserves all earlier native effects.
pub fn world_start_owned(
    resource_count: usize,
    world: &mut impl OwnedWorldStart,
) -> Result<(), EconomyError> {
    if world.started()? {
        return Ok(());
    }
    world.set_started(true)?;
    for kingdom in world.kingdoms()?.into_iter().flatten() {
        world.promote_and_reset_statistics(kingdom, resource_count)?;
    }
    for object in world.objects()? {
        if !world.object_active(object)? {
            continue;
        }
        if world.field_13c(object)? != WORLD_START_FIELD_13C {
            world.set_field_13c(object, WORLD_START_FIELD_13C)?;
        }
        if let Some(owner) = world.owner(object)?
            && world.kingdom_word_190(owner)? == 0
        {
            world.statistics_track(owner, object)?;
        }
    }
    // Native creates a new kingdom iterator for this pass.
    for kingdom in world.kingdoms()?.into_iter().flatten() {
        if !world.has_snapshots(kingdom)? {
            world.reset_snapshot_timer(kingdom)?;
            world.statistics_snapshot(kingdom)?;
        }
    }
    Ok(())
}
