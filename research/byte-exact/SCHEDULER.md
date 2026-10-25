# The native scheduler (roadmap item 10, F4)

What the original runs in one world update, every registration site of its five Updatable
queues, how aa-play's per-system ticks map onto them, and how to migrate aa-play from
`sim.rs`'s system-by-system loop to `aa_world::World::step` + `aa_world::queues` without
breaking the playable game.

Evidence:
* `docs/WORLD-ORDER.txt` and `crates/aa-world` (the update order and the queue
  containers, proven against the original x86 in research/phase2-world-order,
  phase6-queues and phase8-tick);
* the 2026-10-25 disassembly census in this file (`bx_regsites`: every direct call
  of 0x50AC00 / 0x50AC60 / 0x50ACC0 / 0x50AD20 / 0x50AD80);
* the live NativeSampler runs on the Berlin film (`SYNC_LOG.md` §10–11): queue
  bucket counts and cursors per update, and every 589F90 call with its update.

## 1. One world update, 0x4F9970

Record `r`'s decrees run in the admin tick before update `r + 1` (playback: right before
0x4F9970). Then:

| # | step | native |
|---|---|---|
| 1 | fog callback, A-log bool | 0x77D9D0 (SVar fog flag or GWorld+275) |
| 2 | phase transition 1 | 0x4F65B0(1): drains +100 (death queue) and +108 (removal requests) |
| 3 | first-tick hook (old tick 0 only) | 0x4F96D0 |
| 4 | clock | `tick++`; begin = old end; end = f32(i32 tick × step), step = GWorld+E8 = 1/16 |
| 5 | deferred payload list +1A4 | forward, `next` read after the callback, then freed (0x5147C0) |
| 6 | **queue 0** +1AC, **queue 1** +1C0, **queue 2** +1D4, **queue 3** +1E8 | each `vtbl+8` Update |
| 7 | phase 4, trigger manager | 0x4F65B0(4), 0x54C280 (GWorld+13C) |
| 8 | phase 5, **queue 4** +208 | 0x4F65B0(5), `vtbl+8` Update |
| 9 | phase 0, retained list +110 scan | 0x4F65B0(0); virtual 8C keeps, else F0/E0 destroy |

The S clock entry (0x76F835, GWorld+E0 before the update) is logged by the driver between
the decrees and 0x4F9970. Everything in the simulation happens in steps 5–9: there is no
"economy pass" or "movement pass", only Updatable deliveries in queue order.

## 2. The five queues

| id | GWorld | class (vtable) | kind | period | buckets (fresh / Berlin, measured) | Update delivers | registration |
|---|---|---|---|---|---|---|---|
| 0 | +1AC | 0x50ABA0(0), A30BE0 | periodic | ~1e-4 s | 1 / 1 | Updatable +14(elapsed = buckets × dt, data) | 0x50AC00 → 0x50B2C0 |
| 1 | +1C0 | 0x50ABA0(1), A30BE0 | periodic | 1 s | 16 / 16 | Updatable +14(16 × dt = 1 s, data) | 0x50AC60 → 0x50B2C0 |
| 2 | +1D4 | 0x50ABA0(2), A30BE0 | periodic | 5 s | 80 / 80 | Updatable +14(5 s, data) | none found in code (only Init, save 0x4F2DB0, restore 0x4FC600, Update) |
| 3 | +1E8 | 0x50B3A0, A30C24 | deferred | 5 s ring | 80 | 0x4F05C0 → Updatable +18(data) | 0x50ACC0 / 0x50AD20 → 0x50AF10 / 0x50BA50 |
| 4 | +208 | deferred, A30C24 | deferred | 5 s ring | 80 | 0x4F05B0 → Updatable +1C(data) | 0x50AD80 (Updatable vtable +4, "schedule in") |

Bucket counts are `ceil(period / step)` from GWorld Init 0x4F27E0 (`aa_world::initialization`),
and a loaded map restores each queue's cursor, count and entries (0x50B8B0 Load from the GWORLD
chunk). On Berlin the sampler reads q0 (cursor 0, 1 bucket), q1 (cursor = update mod 16, 16),
q2 (cursor = update mod 80, 80).

Rules, all ported in `aa_world::queues` and proven against the original:
* **Periodic add 0x50B2C0**: one world-RNG getter (0x50B2DF) + one draw (0x50B2E5), bucket =
  draw % buckets, **head insertion** into the bucket and into the owner's own list. Even the
  1-bucket queue 0 draws.
* **Periodic Update 0x50B330**: one bucket per update (`current`, then `current + 1 mod n`);
  every entry gets elapsed = n × dt. The current entry is kept in GWorld+228. Cancelling the
  current entry during its own callback moves that cursor to its successor, and the
  dispatcher then advances again, so **the successor is skipped this visit** (50B371..380).
* **Deferred add 0x50AF10** (relative): a positive delay below dt − ε is raised to it; when =
  begin + delay. **0x50BA50** (absolute): origin = max(begin, last end), rel = when − origin − ε,
  capped at 10 s, slot = current + ceil(rel / dt), the ring growing (0x50B710) when needed.
* **Deferred Update 0x50B020**: every event of the current slot is unlinked *before* its
  callback; new heads added by callbacks into the current slot are visited in the same pass;
  **every 1000th callback of an update is skipped**; last end = world end.
* **Gate**: the wrappers 0x50AC00/0x50AC60/0x50ACC0/0x50AD20/0x50AD80 first call the Updatable's
  `vtbl+8` (most return true: 0x6A71D0), so there is no "already registered" check unless the
  class supplies one (Supply InitExisting 0x58B0E0 tests comp+C itself).
* **Cancel all 0x50B250** (owner list, newest first) and `0x50B1C0` / `0x50B220` (by data).

Measured on Berlin (production, §6 below): queue 0 delivers newest registration first, and the
cursor skip is what delays the ger recruit to update 403.

## 3. Registration sites (2026-10-25 census, direct calls)

Names come from aa-actors `component_virtuals::leaves` (InitCreate +4 / InitExisting +8), the
crate docs that already port the function, or the vtable. Unnamed sites are marked.

**Queue 0 (every update)** via 0x50AC00:

| site function | owner / when |
|---|---|
| 0x4E5DF0 | Kingdom economy Updatable (kingdom+180), at kingdom start → Tick 0x4E9C40 (economy, victory checks 4E9960/4E9A40) |
| 0x589420 | Structure (+94) production head start → 58A460 → 589F90 (queue processing, completion, spawn 582E80) |
| 0x58A770 | Structure-related (vtable A3FAF4), unnamed |
| 0x5773C0, 0x576B20, 0x576700, 0x576740, 0x5770A0 | Mover (+6C): ground order, requested speed, InitExisting, falling starts → Tick 0x577E50 |
| 0x57C680 | Organization (+7C) goal update / attach (also the in-place unpack's 8th draw) |
| 0x57DC60, 0x57DCB0, 0x57DD90, 0x57E730, 0x57F290 | Performer (+74): virtuals, target/turret intention, turret aim, start |
| 0x580BC0 | Projectile (+64) activation → 0x580920 |
| 0x567680, 0x567840 | Defender (+B0) ZOC (also queue 3) |
| 0x571CC0 / 0x571CD0 | Flare (+C8) InitCreate / InitExisting |
| 0x93E8E0 | MetaGame (+CC) InitExisting |
| 0x4DB1F0, 0x4DB890 (A28AE4), 0x4DC4B0, 0x4DC5D0 (A28DF8), 0x4E7F80 | GActor / kingdom level, unnamed |

**Queue 1 (1 s, 16 buckets)** via 0x50AC60:

| site function | owner / when |
|---|---|
| 0x563FD0 | shared InitCreate of Properties, Organization, Element, Supply, Construction, Detection, Lair, City (`add ecx,8; 50AC60(0)`) |
| 0x562310 / 0x561DF0 | Body InitCreate / post-load +30 |
| 0x56F250 / 0x56F280 | Economy InitCreate (when element+348, supply_required) / InitExisting |
| 0x5704D0 | Element InitExisting |
| 0x57C870 | Organization InitExisting |
| 0x58B0E0 | Supply InitExisting (only when comp+C == 0) → Tick 0x58B120 |
| 0x564D80 | Construction / Detection / Lair InitExisting → Detection tick (aa-runtime `live_detection_tick`) |
| 0x586F70 / 0x585940 | Settlement InitCreate / InitExisting |
| 0x58CA40, 0x58CB30 | Wall InitCreate, Wall +10 |
| 0x568750 / 0x5687E0 | Denizen InitCreate / InitExisting |
| 0x564300 | City candidates |

Ticks reached through queue 1: Properties 0x5818D0 (periodic heal/damage), City 0x5647A0
(capture by presence), Supply 0x58B120, Detection, Construction, Economy component.

**Queue 3 (deferred)** via 0x50ACC0 / 0x50AD20: Sun 0x4ED820 / 0x4EDB10 / 0x4EDDC0; Body
0x561A60 and corpse 0x562380; Character 0x563640, 0x563770, 0x563AA0, 0x563BA0, 0x563CA0,
0x563D50, 0x563DC0 (animation, fidget, stance timers); Defender 0x567680 / 0x567840 /
0x567960; Denizen 0x5689A0 / 0x5695B0; Detection 0x56EC10; Hq 0x575320 / 0x575A30;
MobileDeployment 0x5763F0 (pack); Performer 0x57ED20 / 0x57EE40 / 0x57F040 (ability
animation, turret wait); Disturbable / Breakable 0x5904A0, 0x5902C0, 0x58FA30, 0x58FF70;
0x51F890 and 0x9389A0 (unnamed).

**Queue 4 (deferred)** via 0x50AD80, which is the Updatable vtable +4 "schedule in" of 43
vtables. It is used by every mind's re-think: ActionGo 0x5300A0 schedules its Element mind
(+70, Updatable at +8) with (0.3 | 1.0) + 0.3u; the other UAI/CAI state thinks are inferred to
reschedule the same way. Direct
sites: Element 0x52D910 (command 6, disable) and 0x52DC10 (2.0 s).

## 4. aa-play today, mapped onto the native owners

`sim.rs::update` runs: clock → ai::think → economy → production → deploy → movement → combat
→ morale → supply → victory → player_orders → fog → special_ops → ids::sweep.

| aa-play system | native owner and queue | what differs today |
|---|---|---|
| economy::tick | Kingdom economy Updatable, queue 0 (0x4E5DF0 → 0x4E9C40), delivered among the other queue-0 entries newest first | runs as its own pass, before production |
| production::tick | Structure, queue 0 (0x589420), newest first, with the cursor skip | since 2026-10-25 a queue-0 list with the cursor skip (core, uncommitted); interleaving with the other queue-0 owners not yet |
| deploy::tick | MobileDeployment: queue 3 timers (0x5763F0), the unpack creation in place | own pass |
| movement::tick | Mover queue 0 (0x5773C0 registration, Tick 0x577E50); Element minds re-think on queue 4 (0x50AD80) | per-actor `next_think` timers; the registration draw is emitted (movement owner) |
| combat::tick | Performer queue 0, Projectile queue 0, UAI engage / kill thinks on queue 4, Body / corpse queue 3 | own pass |
| morale::tick | Organization (queue 1 via 0x563FD0) and the CAI thinks (queue 4), inferred | per tick |
| supply::tick | Supply queue 1 (16 buckets: one 1 s update per component, in its bucket's update) | every tick |
| fog::tick | Detection queue 1 + aa-zones | every tick |
| special_ops::tick | Properties queue 1 (0x5818D0), abilities (Performer queue 3) | every tick |
| victory::tick | inside the kingdom economy Tick 0x4E9C40 (4E9960 / 4E9A40) | own pass |
| ids::sweep | phase transitions: +100 / +108 drains, retained +110 scan | end of tick |

Exact only where one owner ticks once per update in a fixed order; everything with a 1 s or
5 s period or a Think delay needs the queues, because *which* update a component runs in is
decided by its bucket (queue 1: draw % 16) or its deferred slot. That is also why the queue-1
registration draws' **values** matter, not only their count, once queue 1 is ported.

## 5. Migration plan (keeps the game playable at every step)

Each stage is gated by `tests/berlin_replay.rs` (checkpoints) and `tests/berlin_native_state.rs`
(lanes), and re-baselines the aa-play tests once (ROADMAP R9).

1. **Scheduler in the world, shadow mode.** Add `World::sched: aa_world::queues::Queues`
   (initialized by `aa_world::initialization::ClockPlan`, 1/16/80 + two deferred rings) and route
   every registration that already makes its draw (production head start, Mover order, the
   creation queue-1 components, queue 0 for kingdoms) through `Queues::add_periodic` /
   `add_after`, logging the getter first. Systems still tick as today; the queues only hold
   entries. Gate: the S-log is unchanged (same draws, now real buckets).
2. **Queue 0 drives its owners.** Replace the `economy → production` passes and the Mover tick by
   one queue-0 dispatch per update (step 6 of §1). Its leaves are `economy::kingdom_tick(k)`,
   `production::process(structure)` and `movement::mover_tick(actor)`, called in list order with
   the cursor skip. Gate: the production / economy lanes at 402–403 and the 408 checkpoint.
3. **Queue 1.** Supply, Detection (fog), Properties (special_ops), City, Construction and
   Economy component ticks become 1 s deliveries in their buckets. aa-play's per-tick
   integrations become per-delivery with elapsed = 1 s. Gate: supply / fog lanes.
4. **Queue 4, then 3.** Mind re-thinks (movement, combat and morale thinks) become deferred
   events via `add_after`; Character / Performer / deploy / doodad timers go on queue 3. Gate:
   the movement / combat lanes.
5. **sim.rs → `aa_world::World::step`.** Phases and the +100 / +108 / +110 lists replace
   `ids::sweep`, and the trigger manager becomes a seam. `sim.rs` becomes the thin adapter: decrees,
   S clock entry, `World::step(callbacks)`.

Stages 2–4 can be split per owner. A system that is not yet a queue leaf keeps its own pass,
placed where its queue would run (after queue 0, before queue 4).

### 5a. Status (core, 2026-10-25, uncommitted)

`crates/aa-play/src/scheduler.rs`: `World::sched` (ClockPlan 16/s: 1/16/80 buckets and the two
deferred rings), `Owner` (Kingdom, Structure, Mover, Mind, Component(actor, slot), Org,
OrgComponent(org, slot), KingdomComponent(kingdom, slot), Placed(index, slot) for map things aa-play
keeps no actor for, Saved(table id)), `World::schedule_periodic` (getter logged, draw, head insert),
`schedule_after`, `unschedule` (50B250), `run_queue` (step API), and `scheduler::periodic` in place
of the economy / production passes (queues 0, 1, 2 in step 6).

* **Stage 2, done for production and kingdoms.** Queue-0 leaves `production::process` (589F90)
  and `economy::kingdom_tick` (4E9C40). The kingdoms come from the saved queue 0 (Berlin
  UPDATABLES [9,7,3,1,11,5] = map slots 4,3,1,0,5,2). Economy lane 402 → 458, where the next
  divergence is the Mover cursor skip (kingdom_4 skipped in 458 when truck 0x345B's Mover
  unregisters). Mover (0x577E50) and Organization +7C (0x57C680) leaves: movement owner.
* **Stage 3, the restore and core leaves are done.** Every saved periodic queue is restored
  (0x50B8B0, no draws) after the kept things and before ResolvePlaceholders. The owners are
  resolved through the kingdoms' Updatable ids (economy odd, the kingdom actor tail's Properties
  even) and the placed things' component Updatable ids. Berlin q1: 120 entries, all resolved,
  then 136 with the four Corps HQs' 5 × 563FD0. Creation registers the 563FD0 slots and Economy
  as real queue-1 entries; a destroyed thing's entries are cancelled at `ids::sweep`.
  Core leaves: Supply Tick → 58AE10(1 s), with InitExisting 58B0E0 → 58AE10(0) when a supplier
  first appears; City Tick 5647A0; Organization TickMaintenance 57C8E0 → shortfall 578660 and
  regeneration 57B5A0 (1 s). Supply lane 1 → 715 (the unmapped in-place unpack). Fog lane
  402 → 489, now that `supply::org_position` is the org's own position (`Organization::anchor`).
  Its next step is 57C800 follow-leader, a movement-owner item.
  Still passes: Detection (fog owner), Properties (combat owner), Construction, Economy
  component (refresh stays per update).
* Non-queue creation draws: HQ InitCreate's 0x4E2640 is the kingdom's random free HQ number and
  0x50AB10 is a random countdown phase. Neither registers anything.

## 6. Measured: the Berlin production order (refutes the bucket hypothesis)

NativeSampler with probes on 589F90 and after the recruit spawn 582E80 (trace
`traces/berlin2v2_0-production.txt`, QA copy):

* queue 0 has **1 bucket**, so a bucket split cannot delay anything;
* record 2's admin tick runs 589F90 for 0x3451, 0x344F, 0x344D (record order, from the head
  start); from update 3 queue 0 delivers **0x344D, 0x344F, 0x3451** (newest first);
* all three deadlines are 25.0625, and completion needs begin ≥ deadline (update 402);
* update 402: 0x344D completes and its emptied queue unregisters (589420 → 50B250), the cursor
  moves to 0x344F, and the dispatcher skips it; 0x3451 completes. 0x344F completes in 403.

`crates/aa-parity/tests/scheduler_prototype.rs` reproduces exactly this with `aa_world::queues`.
The core owner has since emulated it in `production::tick` (uncommitted), and the replay's
creation draws per update now match the film (402: 28 + moves, 403: 14 + moves).

## 7. Interface (to agree with the core owner, who owns sim.rs / world.rs)

The obstacle is borrowing. A queue leaf needs `&mut World`, and leaves register and cancel on
the same queues while a dispatch is running. `Queues::update(callbacks)` cannot hand out the
world that owns the queues. Prototype (2026-10-25):

* **aa-world, additive:** `Queues::begin_update(id) -> Dispatch`,
  `next_delivery(&mut Dispatch) -> Option<Delivery>`, `end_update(Dispatch)` and `abort_update`.
  It is the same state machine as `update` (which now runs through it), with the native
  post-callback advance applied at the next step. So the host keeps the queues inside its world and
  runs each leaf with `&mut World` in between. `tests/queues_stepper.rs` replays the 69 native
  queue sequences through it.
* **aa-parity:** `tests/scheduler_prototype.rs` is a miniature world that owns its queues and
  dispatches that way.

Proposed aa-play shape (core-owned new file `scheduler.rs`):

```rust
pub enum Owner { Kingdom(KingdomId), Structure(ActorId), Mover(ActorId), Mind(ActorId),
                 Component(ActorId, ComponentSlot), Org(OrgId) }   // u64 key <-> Owner
impl World {
    /// 50AC00 / 50AC60: getter logged, one draw, head insert.
    pub fn schedule_periodic(&mut self, queue: u8, owner: Owner, data: u32);
    /// 50AF10 (queues 3 / 4): no draw.
    pub fn schedule_after(&mut self, queue: u8, owner: Owner, data: u32, delay: f32);
    /// 50B250.
    pub fn unschedule(&mut self, owner: Owner);
    /// Step 6 / 8 of 4F9970: begin_update, then each delivery to its leaf, then end_update.
    fn run_queue(&mut self, queue: u8);
}
```

Each system exposes a leaf per owner kind (`production::process(world, actor, elapsed)` and so
on) instead of a whole-world `tick`. The draw-logging wrapper keeps the S-log emulation in one
place.

## 8. Open items

* Name the unnamed queue-0 sites (0x4DB1F0, 0x4DB890, 0x4DC4B0, 0x4DC5D0, 0x4E7F80, 0x58A770)
  and queue-3 sites (0x51F890, 0x9389A0).
* Queue 2: no registration site in code. Confirm with a restored save whether anything is ever
  on it.
* The trigger manager 0x54C280 (step 7) is a seam in aa-world; campaigns need it.
* aa-runtime already binds several queue leaves natively (`live_*_tick.rs`). When an aa-play system
  becomes a leaf, reuse those ports.
