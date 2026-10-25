//! Scheduler prototype (research/byte-exact/SCHEDULER.md): the Berlin film's
//! three HQ recruits driven by `aa_world::queues` exactly as the original
//! drives them, reproducing the measured completion updates 402 / 402 / 403.
//!
//! Native facts (NativeSampler on the Berlin film, 2026-10-25):
//! * queue 0 (GWorld+1AC) has 1 bucket, queue 1 16, queue 2 80;
//! * the recruits start in record 2's admin tick (589420 → 50AC00(0)) in
//!   record order: kingdom_4's corps 0x3451, the ger corps 0x344F,
//!   kingdom_1's corps 0x344D, drawing the world RNG at 0x4F1E05E7,
//!   0x0D3F1594, 0x20E7783D;
//! * from update 3 on, the queue-0 Update 50B330 calls 589F90 newest first:
//!   0x344D, 0x344F, 0x3451; every queue has deadline 25.0625 (0x41C88000);
//! * a queue completes when the update's begin time reaches the deadline
//!   (update 402: begin 25.0625); an emptied queue unregisters (589420 →
//!   50B250), and removing the entry being dispatched moves the native cursor
//!   to its successor, which the dispatcher then skips (50B371..380).
//!   So in update 402 0x344D completes, 0x344F is skipped and 0x3451
//!   completes; 0x344F completes in update 403.
//!
//! The world here owns its queues, as aa-play's would: it dispatches with the
//! step-by-step API (`begin_update` / `next_delivery` / `end_update`) and runs
//! each leaf with `&mut` access to itself between deliveries.
use aa_numeric::Precision;
use aa_simulation::NativeRng;
use aa_world::queues::{Clock, Delivery, Kind, Queues};

const STEP: f32 = 0.0625;
const DEADLINE: u32 = 0x41C8_8000;

/// A miniature host world: its scheduler and the production leaf's effects.
struct MiniWorld {
    queues: Queues,
    rng: NativeRng,
    update: u32,
    /// (update, owner) of every completion and every queue-0 delivery.
    completed: Vec<(u32, u64)>,
    delivered: Vec<(u32, u64)>,
}

impl MiniWorld {
    /// 589420 → 50AC00(0): a production head start registers the Structure
    /// on queue 0 (one world-RNG draw for the bucket).
    fn start_production(&mut self, owner: u64) {
        self.queues.add_periodic(0, owner, 0, &mut self.rng).unwrap();
    }
    /// 58A460 → 589F90: the Structure's queue-0 leaf.
    fn process(&mut self, event: Delivery) {
        self.delivered.push((self.update, event.owner));
        if f32::from_bits(self.queues.clock.begin_bits) >= f32::from_bits(DEADLINE) {
            self.completed.push((self.update, event.owner));
            // the emptied queue unregisters (589420 → 50B250)
            self.queues.cancel_owner(event.owner).unwrap();
        }
    }
    /// 4F9970's clock advance and its queue-0 Update.
    fn step(&mut self, update: u32) {
        self.update = update;
        self.queues.clock.begin_bits = ((update - 1) as f32 * STEP).to_bits();
        self.queues.clock.end_bits = (update as f32 * STEP).to_bits();
        let mut dispatch = self.queues.begin_update(0).unwrap();
        while let Some(event) = self.queues.next_delivery(&mut dispatch).unwrap() {
            self.process(event);
        }
        self.queues.end_update(dispatch).unwrap();
    }
}

#[test]
fn berlin_recruits_complete_in_402_402_403() {
    let mut queues = Queues::new(
        Clock {
            begin_bits: 0.0625f32.to_bits(),
            end_bits: 0.125f32.to_bits(),
            step_bits: STEP.to_bits(),
        },
        Precision::Double53,
    )
    .unwrap();
    for (id, kind, count) in [(0, Kind::Periodic, 1), (1, Kind::Periodic, 16), (2, Kind::Periodic, 80)] {
        queues.initialize(id, kind, count).unwrap();
    }
    let mut world = MiniWorld {
        queues,
        rng: NativeRng { state: 0x4F1E_05E7 },
        update: 2,
        completed: Vec::new(),
        delivered: Vec::new(),
    };
    // Record 2's admin tick: the three production head starts, in record order.
    let mut states = Vec::new();
    for owner in [0x3451u64, 0x344F, 0x344D] {
        states.push(world.rng.state);
        world.start_production(owner);
    }
    assert_eq!(states, [0x4F1E_05E7, 0x0D3F_1594, 0x20E7_783D], "the native getter states");
    for update in 3..=404u32 {
        world.step(update);
    }
    let at = |u: u32| -> Vec<u64> { world.delivered.iter().filter(|d| d.0 == u).map(|d| d.1).collect() };
    assert_eq!(at(3), [0x344D, 0x344F, 0x3451], "newest registration first, as 589F90 runs in update 3");
    assert_eq!(at(402), [0x344D, 0x3451], "0x344F is skipped after 0x344D's queue unregisters");
    assert_eq!(world.completed, [(402, 0x344D), (402, 0x3451), (403, 0x344F)]);
}
