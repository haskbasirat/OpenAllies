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
//! 0x4E9C40). The other owners still run as their own passes in `sim.rs`,
//! placed where their queues run.
//!
//! The kingdom entries are the map's saved queue 0 (`GameMap::queue0_kingdoms`,
//! the queue Load 0x50B8B0, no draws). Compare against the original: a world
//! without saved queues (a generated map, a test world) admits its kingdoms
//! without a draw in the order the one measured map shows (Berlin: the
//! independents, then the players, each in slot order, registered at the
//! head), not from a measured 0x4E5DF0 kingdom start.
use crate::trace::SKind;
use crate::world::{ActorId, KingdomId, OrgId, World};
use aa_world::initialization::ClockPlan;
use aa_world::queues::{Clock, Delivery, Queues};

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
    /// An actor's component in an aa-rules COMPONENTS slot.
    Component(ActorId, u8),
    /// An organization's Organization component (+7C).
    Org(OrgId),
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
            _ => return None,
        })
    }
}

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

/// The map's saved queue 0 (the queue Load 0x50B8B0): `order` is the
/// kingdoms in list order (head first), no draws. Replaces queue 0.
pub fn restore_kingdoms(world: &mut World, order: &[KingdomId]) {
    let saved = aa_world::queues::restoration::SavedQueue {
        id: EVERY_TICK,
        kind: aa_world::queues::Kind::Periodic,
        current: 0,
        buckets: vec![order.iter().map(|&k| (Owner::Kingdom(k).key(), 0)).collect()],
    };
    if let Err(e) = world.sched.queues.restore_saved(saved) {
        world.sched.note("queue 0 restore", e);
    }
    world.sched.kingdoms.extend(order.iter().copied());
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

fn deliver(world: &mut World, event: Delivery) {
    let elapsed = event.elapsed_bits.unwrap_or(world.clock.step.0);
    match Owner::from_key(event.owner) {
        Some(Owner::Kingdom(k)) => crate::economy::kingdom_tick(world, k, elapsed),
        Some(Owner::Structure(a)) => crate::production::process(world, a),
        // Not yet leaves: their systems still run as passes.
        _ => {}
    }
}

/// Step 6 of 0x4F9970 for queue 0, with the core passes around it: the
/// economy contributions and the dead structures' queues before, the
/// shortfall attrition and the HUD copy after.
pub fn queue_zero(world: &mut World) {
    sync_clock(world);
    crate::economy::refresh(world);
    crate::production::sweep(world);
    admit_kingdoms(world);
    run_queue(world, EVERY_TICK);
    crate::economy::after_queue(world);
}
