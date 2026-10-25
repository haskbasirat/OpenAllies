//! The native world queues of one world update 0x4F9970 and their leaves.
//! Owner: core.
//!
//! research/byte-exact/SCHEDULER.md: the original has no "economy pass" or
//! "production pass"; one update delivers the Updatables registered on five
//! queues (GWorld+1AC periodic every update, +1C0 periodic 1 s in 16 buckets,
//! +1D4 periodic 5 s in 80 buckets, +1E8 and +208 deferred five-second
//! rings), held here in `aa_world::queues` exactly as the original holds
//! them:
//! - a periodic registration (0x50AC00 / 0x50AC60 -> 0x50B2C0) logs the
//!   GetRNG 0x4F02B0 getter and draws the bucket (`draw % buckets`; queue 0
//!   has one bucket and still draws), then inserts at the head of the bucket
//!   and of the owner's list, so the newest registration is delivered first;
//! - removing the entry being delivered (0x50B250 during its own callback)
//!   moves the cursor to its successor, which the dispatcher then passes
//!   over this update (0x50B371..380);
//! - a deferred update unlinks each event before its callback and skips
//!   every 1000th callback (0x50B020).
//!
//! Migrated (SCHEDULER.md section 5, stage 2): queue 0 delivers the structure
//! production queues (Structure +94, 0x589420 registration, 0x58A460 ->
//! 0x589F90) and the kingdom economy ticks (kingdom +180, 0x4E5DF0, Tick
//! 0x4E9C40). Queue 1 holds the real entries (the saved lists, the
//! component InitCreate registrations of `crate::creation`, removed with
//! their things) but delivers to no leaf yet. The other owners still run as
//! their own passes in `sim.rs`, placed where their queues run.
//!
//! The saved entries are the map's saved periodic queues
//! (`GameMap::saved_queues`, the queue Load 0x50B8B0, no draws): kingdoms,
//! the kept things' components, and the components of map things aa-play
//! keeps no actor for (delivered without effect, so the list order and the
//! cursor skip stay native). Compare against the original: a world without
//! saved queues (a generated map, a test world) admits its kingdoms without a
//! draw in the order the one measured map shows (Berlin: the independents,
//! then the players, each in slot order, registered at the head), not from a
//! measured 0x4E5DF0 kingdom start.
use crate::map::{SavedOwner, SavedPeriodic};
use crate::trace::SKind;
use crate::world::{ActorId, KingdomId, OrgId, World};
use aa_world::initialization::ClockPlan;
use aa_world::queues::{Clock, Delivery, Queues};
use std::collections::BTreeMap;

/// GWorld+1AC: periodic, every update (one bucket).
pub const EVERY_TICK: u8 = 0;
/// GWorld+1C0: periodic, 1 s (16 buckets).
pub const EVERY_SECOND: u8 = 1;
/// GWorld+1D4: periodic, 5 s (80 buckets).
pub const EVERY_FIVE_SECONDS: u8 = 2;
/// GWorld+1E8: deferred (Updatable +18).
pub const DEFERRED: u8 = 3;
/// GWorld+208: deferred, after phase 5 (Updatable +1C; the minds' re-think).
pub const THINK: u8 = 4;

/// The simulation thread's x87 precision (PC53).
const PRECISION: aa_numeric::Precision = aa_numeric::Precision::Double53;

/// An Updatable of aa-play's world: the native object whose Updatable is
/// registered (the queue's owner).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Owner {
    /// Kingdom +180, the economy Tick 0x4E9C40.
    Kingdom(KingdomId),
    /// An actor's Structure component (+94), its production queue.
    Structure(ActorId),
    /// An actor's Mover (+6C).
    Mover(ActorId),
    /// An element's mind.
    Mind(ActorId),
    /// An actor's component in an aa-rules COMPONENTS slot (its Updatable
    /// at component +8).
    Component(ActorId, u8),
    /// An organization's Organization component's second Updatable (+7C,
    /// the queue-0 goal update 0x57C680).
    Org(OrgId),
    /// An organization thing's component in a COMPONENTS slot.
    OrgComponent(OrgId, u8),
    /// A kingdom's own component (its Properties).
    KingdomComponent(KingdomId, u8),
    /// A component of `GameMap::placed[index]` that aa-play keeps no actor
    /// for (scenery): delivered without effect, kept for the list order.
    Placed(u32, u8),
    /// A saved Updatable aa-play does not resolve (UPDATABLES table id).
    Saved(u32),
    /// An organization's mind (the Regiment / Organization Mind, its
    /// Updatable +8 on the deferred +208).
    OrgMind(OrgId),
}

impl Owner {
    /// The queue owner key: kind << 56 | component slot << 48 | id.
    pub fn key(self) -> u64 {
        let (kind, slot, id) = match self {
            Owner::Kingdom(k) => (1u64, 0u64, k),
            Owner::Structure(a) => (2, 0, a),
            Owner::Mover(a) => (3, 0, a),
            Owner::Mind(a) => (4, 0, a),
            Owner::Component(a, s) => (5, u64::from(s), a),
            Owner::Org(o) => (6, 0, o),
            Owner::OrgComponent(o, s) => (7, u64::from(s), o),
            Owner::KingdomComponent(k, s) => (8, u64::from(s), k),
            Owner::Placed(i, s) => (9, u64::from(s), i),
            Owner::Saved(id) => (10, 0, id),
            Owner::OrgMind(o) => (11, 0, o),
        };
        kind << 56 | slot << 48 | u64::from(id)
    }
    pub fn from_key(key: u64) -> Option<Self> {
        let id = (key & 0xFFFF_FFFF) as u32;
        let slot = ((key >> 48) & 0xFF) as u8;
        Some(match key >> 56 {
            1 => Owner::Kingdom(id),
            2 => Owner::Structure(id),
            3 => Owner::Mover(id),
            4 => Owner::Mind(id),
            5 => Owner::Component(id, slot),
            6 => Owner::Org(id),
            7 => Owner::OrgComponent(id, slot),
            8 => Owner::KingdomComponent(id, slot),
            9 => Owner::Placed(id, slot),
            10 => Owner::Saved(id),
            11 => Owner::OrgMind(id),
            _ => return None,
        })
    }
}

/// Component slots an Updatable can sit in (aa-rules COMPONENTS bits).
const SLOTS: u8 = 32;

/// The world's queues (`World::sched`).
pub struct Scheduler {
    pub queues: Queues,
    /// Kingdoms already on queue 0 (restored or admitted).
    kingdoms: std::collections::BTreeSet<KingdomId>,
    /// Queue errors (none expected; kept for diagnostics, at most 64).
    pub notes: Vec<String>,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Scheduler {
    /// GWorld Init 0x4F27E0: the clock plan of GameTicksPerGameSecond 16
    /// (step 1/16) and its five queues: 1, 16, 80 buckets and two
    /// five-second deferred rings.
    pub fn new() -> Self {
        let plan = ClockPlan::from_tick_rate(16f32.to_bits(), PRECISION).expect("the stock tick rate");
        let queues = plan.initialize_queues(plan.new_clock(), PRECISION).expect("the stock world queues");
        Self { queues, kingdoms: Default::default(), notes: Vec::new() }
    }
    fn note(&mut self, what: &str, e: impl std::fmt::Display) {
        if self.notes.len() < 64 {
            self.notes.push(format!("{what}: {e}"));
        }
    }
    /// The owners of queue `id`, bucket by bucket, in delivery order.
    pub fn entries(&self, id: u8) -> Vec<Vec<Owner>> {
        self.queues
            .snapshot()
            .into_iter()
            .find(|q| q.id == id)
            .map(|q| q.buckets.into_iter().map(|b| b.into_iter().filter_map(|(o, _)| Owner::from_key(o)).collect()).collect())
            .unwrap_or_default()
    }
}

impl World {
    /// 0x50AC00 (queue 0) / 0x50AC60 (queue 1) -> 0x50B2C0: the getter, one
    /// world draw for the bucket, head insertion.
    pub fn schedule_periodic(&mut self, queue: u8, owner: Owner, data: u32) {
        self.trace.log(SKind::RngGetter, self.native_rng.state);
        if let Err(e) = self.sched.queues.add_periodic(queue, owner.key(), data, &mut self.native_rng) {
            self.sched.note("periodic registration", e);
        }
    }
    /// 0x50ACC0 / 0x50AD80 -> 0x50AF10 (queues 3 and 4): `delay` seconds
    /// after the world begin time, no draw.
    pub fn schedule_after(&mut self, queue: u8, owner: Owner, data: u32, delay: f32) {
        if let Err(e) = self.sched.queues.add_after(queue, owner.key(), data, delay.to_bits()) {
            self.sched.note("deferred registration", e);
        }
    }
    /// 0x50B250: every event of the owner leaves the queues, newest first.
    pub fn unschedule(&mut self, owner: Owner) {
        if let Err(e) = self.sched.queues.cancel_owner(owner.key()) {
            self.sched.note("cancel", e);
        }
    }
    pub fn is_scheduled(&self, owner: Owner) -> bool {
        self.sched.queues.has_owner(owner.key())
    }
}

/// The queues read the world clock (GWorld+E0 begin, +E4 end, +E8 step).
pub fn sync_clock(world: &mut World) {
    world.sched.queues.clock = Clock { begin_bits: world.clock.begin.0, end_bits: world.clock.end.0, step_bits: world.clock.step.0 };
}

/// The map things as the saved queues load (setup): the aa-play owner of
/// each kept placed thing, and the kingdom of each map slot.
pub struct Restoring<'a> {
    /// Placed index -> the actor it became, or its organization (a placed
    /// organization thing).
    pub things: &'a BTreeMap<usize, Kept>,
    pub slot_kingdom: &'a [Option<KingdomId>],
}

/// What a kept placed thing became.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kept {
    Actor(ActorId),
    Org(OrgId),
}

fn saved_owner(world: &World, owner: SavedOwner, r: &Restoring<'_>) -> Option<Owner> {
    match owner {
        SavedOwner::Placed { index, slot } => match r.things.get(&index) {
            Some(Kept::Actor(a)) => Some(Owner::Component(*a, slot)),
            Some(Kept::Org(o)) => Some(Owner::OrgComponent(*o, slot)),
            // A thing aa-play keeps no actor for stays on the list; a
            // destroyed one (a kingdom left out of the battle) leaves it.
            None => world.ids.placed(index).is_some().then_some(Owner::Placed(index as u32, slot)),
        },
        SavedOwner::Kingdom { slot } => r.slot_kingdom.get(slot).copied().flatten().map(Owner::Kingdom),
        SavedOwner::KingdomComponent { slot, component } => r.slot_kingdom.get(slot).copied().flatten().map(|k| Owner::KingdomComponent(k, component)),
        // 0x50B8B0 discards a null id.
        SavedOwner::Unknown(id) => u32::try_from(id).ok().map(Owner::Saved),
    }
}

/// The queue Load 0x50B8B0 of the map's saved periodic queues (after the
/// map things are restored, before the placeholders are resolved): each
/// queue is replaced by its saved lists and cursor, no draws.
pub fn restore_saved(world: &mut World, saved: &[SavedPeriodic], r: &Restoring<'_>) {
    for (id, q) in saved.iter().enumerate().take(3) {
        let buckets: Vec<Vec<(u64, u32)>> =
            q.buckets.iter().map(|b| b.iter().filter_map(|&(o, data)| saved_owner(world, o, r).map(|o| (o.key(), data))).collect()).collect();
        let kingdoms: Vec<KingdomId> =
            buckets.iter().flatten().filter_map(|&(key, _)| if let Some(Owner::Kingdom(k)) = Owner::from_key(key) { Some(k) } else { None }).collect();
        let queue = aa_world::queues::restoration::SavedQueue { id: id as u8, kind: aa_world::queues::Kind::Periodic, current: q.current, buckets };
        if let Err(e) = world.sched.queues.restore_saved(queue) {
            world.sched.note("saved queue restore", e);
            continue;
        }
        world.sched.kingdoms.extend(kingdoms);
    }
}

/// A map without saved queues: its kept things' queue-1 components
/// (`crate::creation::queue_one_slots`) join queue 1 as an InitExisting
/// would, in `order` (restore order), with their buckets from a stream of
/// their own instead of the world's. Compare against the original: not a
/// measured path (no film of a map without saved queues reaches it).
pub fn admit_restored(world: &mut World, order: &[Kept]) {
    let data = world.data.clone();
    let mut buckets = aa_simulation::NativeRng::default();
    for &kept in order {
        let (def, made) = match kept {
            Kept::Actor(a) => (world.actors[a as usize].def, Owner::Component as fn(u32, u8) -> Owner),
            Kept::Org(o) => (world.orgs[o as usize].def, Owner::OrgComponent as fn(u32, u8) -> Owner),
        };
        let id = match kept {
            Kept::Actor(a) => a,
            Kept::Org(o) => o,
        };
        for slot in crate::creation::queue_one_slots(data.def(def)) {
            if let Err(e) = world.sched.queues.add_periodic(EVERY_SECOND, made(id, slot as u8).key(), 0, &mut buckets) {
                world.sched.note("restored registration", e);
            }
        }
    }
}

/// A destroyed thing's Updatables leave the queues (its components'
/// destructors, 0x50B250 each).
pub fn release_actor(world: &mut World, a: ActorId) {
    for owner in [Owner::Structure(a), Owner::Mover(a), Owner::Mind(a)] {
        world.unschedule(owner);
    }
    for slot in 0..SLOTS {
        world.unschedule(Owner::Component(a, slot));
    }
}
pub fn release_org(world: &mut World, o: OrgId) {
    world.unschedule(Owner::Org(o));
    world.unschedule(Owner::OrgMind(o));
    for slot in 0..SLOTS {
        world.unschedule(Owner::OrgComponent(o, slot));
    }
}
/// A placed thing aa-play keeps no actor for is destroyed (a resolved
/// placeholder).
pub fn release_placed(world: &mut World, index: usize) {
    for slot in 0..SLOTS {
        world.unschedule(Owner::Placed(index as u32, slot));
    }
}

/// Kingdoms not yet on queue 0 join it at the head without a world draw:
/// the independents, then the players, each in index order (see the module
/// documentation).
pub fn admit_kingdoms(world: &mut World) {
    if world.sched.kingdoms.len() >= world.kingdoms.len() && world.kingdoms.iter().all(|k| world.sched.kingdoms.contains(&k.id)) {
        return;
    }
    let mut order: Vec<KingdomId> = (0..world.kingdoms.len() as KingdomId).filter(|k| !world.sched.kingdoms.contains(k)).collect();
    order.sort_by_key(|&k| !world.kingdoms[k as usize].independent);
    for k in order {
        // Queue 0 has one bucket: the bucket draw is not the world's.
        let mut bucket = aa_simulation::NativeRng::default();
        if let Err(e) = world.sched.queues.add_periodic(EVERY_TICK, Owner::Kingdom(k).key(), 0, &mut bucket) {
            world.sched.note("kingdom admission", e);
        }
        world.sched.kingdoms.insert(k);
    }
}

/// One queue's Update (`vtbl+8`): every delivery to its leaf, in the native
/// order, the leaves running with the whole world.
pub fn run_queue(world: &mut World, id: u8) {
    let mut dispatch = match world.sched.queues.begin_update(id) {
        Ok(d) => d,
        Err(e) => {
            world.sched.note("begin update", e);
            return;
        }
    };
    loop {
        match world.sched.queues.next_delivery(&mut dispatch) {
            Ok(Some(event)) => deliver(world, event),
            Ok(None) => break,
            Err(e) => {
                world.sched.note("delivery", e);
                world.sched.queues.abort_update(dispatch);
                return;
            }
        }
    }
    if let Err(e) = world.sched.queues.end_update(dispatch) {
        world.sched.note("end update", e);
    }
}

/// COMPONENTS slots of the queue-1 leaves.
pub const ORGANIZATION_SLOT: u8 = 7;
pub const SUPPLY_SLOT: u8 = 11;
pub const CITY_SLOT: u8 = 29;

fn deliver(world: &mut World, event: Delivery) {
    let elapsed = event.elapsed_bits.unwrap_or(world.clock.step.0);
    match Owner::from_key(event.owner) {
        Some(Owner::Kingdom(k)) => crate::economy::kingdom_tick(world, k, elapsed),
        Some(Owner::Structure(a)) => crate::production::process(world, a),
        // SupplyComponent Tick -> 0x58AE10.
        Some(Owner::Component(a, SUPPLY_SLOT)) => crate::supply::supplier_tick(world, a, f32::from_bits(elapsed)),
        // City Tick 0x5647A0.
        Some(Owner::Component(a, CITY_SLOT)) => crate::supply::city_tick(world, a),
        // The Denizen Updatable: the runway release 0x5695B0 (deferred,
        // vtable +0x18) and the Tick 0x56E860 (queue 1).
        Some(Owner::Component(a, crate::denizen::SLOT)) if event.queue == DEFERRED => crate::denizen::runway_release(world, a),
        Some(Owner::Component(a, crate::denizen::SLOT)) => crate::denizen::tick(world, a, f32::from_bits(elapsed)),
        // KKC_OrganizationComponent::TickMaintenance 0x57C8E0.
        // (movement owner) the same Updatable's queue-0 goal step (data 1,
        // 0x57D820 -> 0x57C080).
        Some(Owner::OrgComponent(o, ORGANIZATION_SLOT)) if event.data == 1 => crate::movement::org_tick(world, o),
        Some(Owner::OrgComponent(o, ORGANIZATION_SLOT)) => crate::supply::org_maintenance(world, o, f32::from_bits(elapsed)),
        // (movement owner) Mover Tick 0x577E50.
        Some(Owner::Mover(a)) => crate::movement::mover_tick(world, a),
        // (movement owner) the element Mind's queue-4 Update 0x531D30.
        Some(Owner::Mind(a)) => crate::movement::mind_tick(world, a),
        // Not yet leaves: their systems still run as passes.
        _ => {}
    }
}

/// Step 6 of 0x4F9970, its last queue: the deferred +1E8 (Updatable +18
/// callbacks; every 1000th callback of an update is skipped).
pub fn deferred(world: &mut World) {
    run_queue(world, DEFERRED);
}

/// Step 8 of 0x4F9970: the deferred +208 after phase 5 (Updatable +1C, the
/// minds' re-think "schedule in").
pub fn think(world: &mut World) {
    run_queue(world, THINK);
}

/// Step 6 of 0x4F9970 for the periodic queues 0, 1, 2 (every update, 1 s,
/// 5 s), with the core passes around them: the economy contributions and
/// the dead structures' queues before, the HUD copy after.
pub fn periodic(world: &mut World) {
    sync_clock(world);
    crate::economy::refresh(world);
    crate::production::sweep(world);
    admit_kingdoms(world);
    run_queue(world, EVERY_TICK);
    run_queue(world, EVERY_SECOND);
    run_queue(world, EVERY_FIVE_SECONDS);
    crate::economy::after_queue(world);
}

#[cfg(test)]
mod tests {
    use super::Owner;

    #[test]
    fn owner_keys_round_trip() {
        let owners = [
            Owner::Kingdom(5),
            Owner::Structure(70_000),
            Owner::Mover(3),
            Owner::Mind(u32::MAX),
            Owner::Component(12, 29),
            Owner::Org(4),
            Owner::OrgComponent(4, 7),
            Owner::KingdomComponent(2, 6),
            Owner::Placed(6000, 22),
            Owner::Saved(523),
            Owner::OrgMind(9),
        ];
        let keys: Vec<u64> = owners.iter().map(|o| o.key()).collect();
        for (o, k) in owners.iter().zip(&keys) {
            assert_eq!(Owner::from_key(*k), Some(*o));
        }
        let mut unique = keys.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), keys.len());
        assert_eq!(Owner::from_key(0), None);
    }
}
