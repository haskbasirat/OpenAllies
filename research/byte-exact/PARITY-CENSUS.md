# Berlin film parity census: everything between us and 266 of 266

Census of 2026-11-01 against main at c2b4099 plus the uncommitted working tree of that day (the
supply/fog rewrite, combat and scheduler edits were in it). Addresses are AA.exe. Line numbers are
of that tree and will drift; function names are given so they can be found again.

**Where it stands: 32 of 266 checkpoints, first difference at tick 561 (film (731, 52277), ours
(731, 28237)).** The S count is still right at 561: the getters run, one tick off.

The stream is the key to reading all of this. Every getter draws once, so both sides walk the same
LCG chain and the k-th getters have the same state whatever their tick. A checkpoint hash therefore
depends on *how many getters run before each clock entry*. 266 of 266 means: the same 6986 getters
in the same updates. Everything below matters to the film hash exactly as far as it moves a getter.

## 1. How to read and reproduce it

```
CARGO_TARGET_DIR=target/s-parity-census AA_GAME_ROOT=<game root> AA_PARITY_CENSUS_DIR=<dir> \
  cargo test -p aa-parity --features play --test berlin_native_state --release --offline -- --ignored --nocapture
```

`tests/berlin_native_state.rs` plays the film four ways and writes each report as JSON to the dir.
`AA_PARITY_WATCH=<native gtid hex>:<from>:<to>,...` prints things side by side;
`AA_PARITY_CENSUS_RUNS=free,forced-rng` selects runs. One run takes about 3 s.

| run | what is put back on the native values after every sampled tick |
|---|---|
| free | nothing: the replay as it stands. After the first difference it is mostly knock-on |
| forced-rng | GWorld+D4. Each getter window starts from the same state, so its first difference is exact |
| forced-rng-economy | also Kingdom +1C4. Gave the same getters as forced-rng: nothing here depends on the stock |
| forced-all | also position, direction, height, anchor, health. Mover speed and Mind state stay ours, so the movement rows there are only a hint |

What the harness does now (`crates/aa-parity/src`):

* `state_compare`: every lane runs to the end of the film and collapses into causes. A signature is
  the field, the kind of thing and how the two sides differ. `first-on` counts the things a cause is
  the first difference of, and a cause never first on anything is printed `KNOCK-ON`. `getter_census`
  compares the getters per sampled window and names each native getter by site and chain.
* `replay::Pairing`: native things created in battle are paired with ours by definition, in order.
  Without it one slot shift (cause K1) sends every later order and comparison to the wrong thing.
  The pairing is a diagnostic: the gate (`berlin_replay`) does not use it, so it still shows the
  shift's refusals.
* `mapping`: the fixes in section 2.

**What each claim rests on.** Native facts come from the sampler trace of the original (every tick
to 1200, then every 8th) and the pseudocode. "Proven" on a port means its own fixture replay; none of
these ports has run against this film. Each cause's size is an estimate. Causes M4 to M7 are read from
the first place they show: later movement causes can hide behind them, so **re-run this census after
each movement step**.

## 2. Fixed in the mapper (this round): 28 to 32, all 50 orders resolve

Every unresolved film order in the brief is explained. Only `group_behavior_enable` lacks an aa-play command.

| symptom in the gate | cause | now |
|---|---|---|
| "unpack truck actors, position and facing" (first at 482) | the film's unpacks have flags 0x44: position, no auxiliary words, so `has(3)` failed. And the selection is usually the HQ regiment, not the truck | position only. Delivery 0x4D9B60 passes a command to the organization's leader (+0x7C, +0x44) when the selected actor lacks the command's component |
| "recruit buildings and object definition" (first at 962) | the same delivery: selection is the HQ regiment. The native chain 0x4D9BE8, 0x4CE6BA, 0x583BF8 is identical to a recruit on the building | leader, as above |
| "formation organizations and FormationData" (first at 1217) | `PlayIds::formation` was a stub | slot through `Registries::formation_ids`. Slot 2 = `regiment_column` is confirmed by the measured speed (M6) |
| thing slots 13413, 13416, 13428, 13430, 13434, 13450, 13454, 13466 | knock-on: the recruit was unresolved or, later, the id shift K1 | resolve with the pairing; K1 removes the need for it |
| "not enough resources" at 1706 | knock-on of the unresolved recruits | gone |
| "not your building" at 3146, "not your truck" at 1226 and 1322, "missing requirements" at 3242, 3723, 3844 and 4325 | K1: the slot names another thing here | gone with the pairing, remain in the gate until K1 |
| "group_behavior_enable" no equivalent | real: X2 | stays `NoEquivalent` |
| "cannot attack an own unit" at 3394 and 4282 (new) | the target's owner differs here: X1 | still refused |

The Mapper now counts order fields aa-play cannot carry (`ReplayReport::dropped`): 11 recruits carry a
ParameterSet, cause K2.

**Already exact over the whole film, by this comparison:**

* Production lane: **0 differences in 1617 sampled ticks** (queue length, deadline, paused).
* Creation: in the forced-rng run, all 77 native creations happen in the same tick here, except the
  unpacks (M2), the later members of two regiments (from 4017 and 4402; a regeneration phase from a
  drawn bucket, inferred) and `group` (X2). The native list includes the buildings an unpack makes.
* The getter groups of creation (scale, mind jitter, InitCreate registrations, economy, countdown, HQ
  number) are right wherever the window is otherwise right (ledger, section 5).
* Economy lane: no rate or consumption of its own differs. Its differences are the tick of an unpack
  (M2) and of a city capture (F3).
* Combat: **the film has no damage and no death at all** (no health change at any sampled tick except
  the unpack swap, dead flag never set), and no native getter comes from combat or rout code. So they
  are not what stands between us and 266, except where they draw here: any draw from `combat.rs` or
  `morale.rs` would be an extra, and our getters carry no site to tell (open question 5). Morale itself
  is not in the sampler's trace, so that lane is unmeasured.

## 3. The causes, ordered

Dependencies decide the order. A lane can take its causes top-down; anything marked `needs` waits for
that cause. Numbers are the free run unless noted. Sizes: **S** at most one session, **M** 2 to 5,
**L** more than 5.

### Core lane

Files: `ids.rs`, `deploy.rs`, `creation.rs`, `production*.rs`, `setup.rs`, `map.rs`, `scheduler.rs`, `books.rs`, plus `world.rs` by agreement.

#### K1. An unpack takes a new id and swaps it (S, do first)

* **Shows as:** 60 things sit on another slot from tick 1002. First: native `ger_truck_artillery` 0x3462, ours 0x3460. The native slots never named are 0x3460, 0x3461, 0x3464, 0x346B, 0x346F, 0x3479: one per unpack.
* **Native:** 0x575C80 creates with record +0x24 = 0, so 0x4EE460 takes the next-fit id (0x4EFDB0). Then 0x4D6130 runs 0x4D5EA0, whose 0x4EFB90 swaps the two things' GTIDs, then 0x4F6760 destroys the old thing, which now holds the new id. Result: the building keeps the truck's id, the cursor has moved one, and the spare slot is free with its generation raised.
* **Port:** `aa_actors::registry::IdentityRegistry::swap_identities` (0x4EFB90), replayed by `crates/aa-actors/tests/native_registry.rs` on `research/phase4-actors/trace_registry.json`; outside its tests only a mechanics check calls it.
* **aa-play:** `ids.rs:294 replace_actor` removes the id and re-registers the same one, so the cursor stays. Called from `deploy.rs:156`.
* **Takes:** allocate, swap, remove in `replace_actor`. `Thing::Trailer` goes with the old thing as today.
* **Proof:** the replay needs no pairing and the gate's refusals go. Assert in `berlin_replay` that no order is unresolved except X2.

#### K2. A recruit order chooses its layout and elements (M)

* **Shows as:** 11 of 13 recruit orders carry `ParameterSet { definition, objects }`, for example `[713, 713, 713, 717, 714, 716]`. Ours builds the definition's own elements. First visible at 3992: three `ger_infantry_mortar` missing, regiment 9 members against 8 (order object 856, elements `[562, 562, 566, 566, 567, 565]`).
* **Native:** 0x61EBA0 flag 5, command +0x34; 0x583A70 takes it, 0x5836E0, 0x589E00 stores it on the item, and 0x582E80 / 0x4FBF40 spawn by it. The layout index is OrganizationLayout `[0xB3AB00]`, the objects are ThingData slots.
* **Port:** `aa_production::RecruitRequest` has `organization_layout` but no objects (`crates/aa-production/src/lib.rs:134`). Spawn is `aa_actors::organization_spawn`, `organization_recruitment`.
* **aa-play:** `Command::Recruit` (`world.rs:134`) has only `def`; `production.rs:467 recruit` passes layout 0; `production.rs:249 spawn_member` reads `Def::slots`.
* **Takes:** a new variant or field on `Command::Recruit` (shared file, aa-bevy and the AI build it, so additive), carried on the queue item, read by `spawn_member`. `mapping.rs` passes it and drops the `dropped` counter. Check the cost quote against the parameters.

#### K3. The map's saved deferred queues, and doodad timers (M)

* **Shows as:** the first missing getter: tick 583, 165 in the film range (group "disturbable delay"). Every one shifts the whole chain by one, so in the free run this is what turns timing differences into value differences.
* **Native:** the queue Load replaces all five queues with the map's saved lists (0x50BF10 / 0x50BFF0); `UPDATABLES` holds them in the order 3, 4, 0, 1, 2. **Berlin saves 796 events on queue 3, in 97 slots, and none on queue 4** (measured by this test), so on this reading of the code (open question 1) the 6190 draws of InitExisting 5904A0 are real but their events do not survive. Queue 3 delivers a Disturbable's 0x5902C0 (Updatable +0x18): 0x5901F0 scans a circle of `affect_radius` for a moving thing (0x4CEDE0); only when disturbed does it check its time, run the FX (vf +0xC), and draw (getter 0x590323, one draw) `now + min + u * (max - min)`. Then it re-arms with `update_time` through 0x50ACC0. Breakable 0x58FA30 likewise.
* **Port:** `aa-runtime/src/live_doodad_tick.rs` is a binding of 0x5902C0 with the disturbed branch as an explicit error, and 0x58FA30 unbound; `aa_spatial::circle_actors`, `moving_state`. **No oracle:** a port and fixture for 0x5902C0 and 0x58FA30 are part of this cause.
* **aa-play:** `map.rs:673 saved_periodic` reads queues 3 and 4 and throws them away; `scheduler.rs:239 restore_saved` takes 3. `creation.rs:151 restored` draws for 5904A0 / 58F960 and registers nothing, which is right as long as the Load replaces them. `deliver` (`scheduler.rs:354`) has no doodad leaf.
* **Takes:** restore queue 3 (deferred kind; `aa_world::queues::restoration::restore_saved` keeps the saved ring length and zeroes the last-end time, as 0x50BF10 does); `Owner::Placed` delivery to a doodad leaf that needs data on `Def` (`update_time`, `affect_radius`, min, max) and a spatial query over moving actors.
* **Watch:** the 796 owners have not been listed: not all need be doodads, and each kind needs a leaf or an explicit refusal. Doodad FX need no simulation effect, but check the Action list (0x42A590) for anything that does.
* **needs** M4 for exact `4CEDE0` moving and positions before the getter count can match; it can land first and be checked in the windows before 583.

#### K4. The Character component (M to L)

* **Shows as:** 256 getters in the film range, first at 1202 (`rus_infantry_co` creation): 48 at creation (0x4D6FE0), 91 at node reached (0x530DA9), 100 in the Character's deferred update, the rest from Mind states.
* **Native:** 0x563BA0 (default stance) draws idle repetitions (0x49B4D0) and arms 0x50ACC0 with `repetitions * animation length`; 0x563CA0 draws a fidget (0x45EB70) and arms with its length; 0x563E10 is the deferred update (+0x18) with its data cases. Reached through 0x4CFC80 when the Mover stops, and from the Minds.
* **Port:** `aa_animation::runtime` (`default_stance`, `actor_default_stance`, `idle_repetitions`, `start_move`), fixture `native-runtime.json` 720 cases, bindings 65; `aa_actors::ground_states::CharacterState`; 0x563CA0 and 0x563E10 need checking against it. `crates/aa-bevy/src/battle_view.rs:484` has a view-only copy, which must stay view-only.
* **aa-play:** nothing in the simulation. `creation.rs` omits the InitCreate draw.
* **Takes:** new `character.rs`: per-actor `CharacterState`, the animation sets and lengths from data **in the simulation** (a length decides a re-arm tick, so it cannot live in the renderer; aa-bevy can read the state), deferred delivery in `deliver`, the draw in `creation.rs`, and a call from movement when a Mover stops (see M4). The interface to agree first is `character::mover_stopped(world, actor)`.
* **needs** M4 for the node-reached and stop calls to fall on the right tick; the creation and timer parts can be checked earlier.

#### K5. Heights (S)

* **Shows as:** all film long on all things: 1.9543319 against 1.95433, 2.2136512 against 2.21365. `map.rs:382` reads heights from the renderer's JSON, whose text is rounded, and `map.rs:216 height_at` interpolates in f32.
* **Native / port:** 0x4F0E50, `aa_spatial` (crates/aa-spatial/src/lib.rs:162), `research/phase5-heightfield` fixtures, `aa_mapworld::terrain`. ROADMAP lists this gap.
* **Takes:** heights from the RMP terrain through the port. No getter depends on it directly as far as the film shows (height does not enter a Mover step), so it can go any time. The renderer shares `heights`: check it still lines up.

### Movement lane

Files: `movement.rs`, `path.rs`, `formation.rs`, `minds.rs`, `aa-movement`, `aa-navigation`. Today `movement.rs` is a plain f32 reimplementation that follows the ports but calls none of them except the solver and `formation`. This lane owns **6200 of the 6986 getters** (the Mover registration and ActionGo pairs).

The order is M1, then M2 and M3, then M4, then the rest. In forced-rng every window up to tick 551 is equal (76 getters), so the structure holds that far and geometry is what breaks it first.

#### M1. The Mover ground step on the exact port (S to M)

* **Shows as:** first difference in the lane, tick 484, the truck's first turn: native direction (0.99802, -0.06285), ours (0.99805, -0.06246). Position differs from the same tick (< 1 mm, growing).
* **Native:** 0x5774A0, turning through 0x445400 and 0x4454A0 / 0x4454E0, which are quantized and CRT-based.
* **Port:** `aa_movement::ground_step_original` with `OriginalTurning`, `mover::{tick, arrival_stop, failure_stop}`; fixtures 186 + 297 ground steps, Mover 1251 (`research/phase4-movement`, `phase8-movement/native-mover.json`).
* **aa-play:** `movement.rs:2521 ground_step` with `signed_angle` / `rotate` at `:927` / `:930` (host `atan2`, `sin_cos`, renormalize).
* **Takes:** an adapter implementing `aa_movement::GroundWorld` over `ActorMove` + `collision()`. PC53, PE pending = true. Per-tick cost: `aa_numeric::Fast` is what `interpolate_native` already uses. Keep `collision()` as the seam until M5.

#### M2. Unpack as the native states do it (M)

* **Shows as:** the first move's destination: native (160.67825, 366.33093), the order's point; ours (161, 366), snapped. That changes the path from tick 483 (the node at 525 differs, the arrival at 552 comes a tick early), then the unpack tick: native 587, one update after the Mover's arrival at 586; ours in the arrival update. The same offset moves every knock-on: the 8 creation getters per unpack, the Supply registration at the end of the unpacking (715 against 714), the trailer, the HQ's health and ZOD at the swap, and the economy (`provided`, then the money stock, 1031 sampled ticks).
* **Native:** regiment `KKC_CAIStateUnpack` 0x526E60 (action 0x51E800, ordinal 0xB, delivered to the leader, then the 1.0 s re-think); element `KKC_UAIStateUnpack` 0x5372B0, placement 0x52EC20 (0x554500 in rings, **not ported**), 0x52EBF0 (point = definition +0x538/+0x53C + L), move state 0x52DF60; then 0x575C80 and 0x4F3640, which is where the building snaps (`Thing` tile_locked, axis_locked). All on queue 4.
* **Port:** `aa_mind::cai_thinks::unpack` 240 cases, `aa_mind::uai_thinks::uai_unpack` 540 cases.
* **aa-play:** `deploy.rs:38` calls `placement` at command time and gives the mover the snapped point; `deploy.rs:39` and `movement.rs:3394` refuse spots by rules not found natively (`spot_clear`, `deploy_spot_problem`); `deploy.rs:190 tick` is a pass that starts the unpack from `arrived()` (`:139`).
* **Takes:** `Order::Deploy` carries the order's point and the snapped placement is computed when the building is made; the Think on `mind_tick` (with M4's Mind host or, as a first step, an equivalent think that keeps the queue-4 timing). Port 0x52EC20. Touches `deploy.rs` (core) and `movement.rs`: **one agent does both**.
* Facing: every Berlin unpack is axis_locked. See the note in `mapping.rs` on the zero direction.

#### M3. The organization during and after the unpack (S)

* **Shows as:** native 0x3457 keeps walking to the building at the catch-up speed after 587 (160.69958, 160.72137, ...), ours jumps. The group "organization goal 57C680" (4 getters at 587, 888, 1066, 1289) is never produced here.
* **Native:** 0x57C680 gates on Mind +0x64 / +0x7C; the regiment's unpack state is still active, so the goal moves and 0x50AC00(1) registers (0x4D6174, 0x57D8A0, 0x57CC8B, 0x57C800).
* **Port:** `aa_actors::organization_interpolation`, `element_position::organization_moved`; already used by `movement.rs:3187`.
* **aa-play:** `deploy.rs:170` sets the organization `Order::Idle` *before* `member_replaced` (`:174`), so `org_active` (`movement.rs:3141`) is false and `set_goal` returns.
* **Takes:** keep the deploy order until the regiment's unpack state ends. Goes with M2.

#### M4. The element Mind on the exact ports (L, the centre of everything)

* **Shows as:** the windows. In forced-rng 437 of 1617 windows differ; the first are ours one tick before native (552 / 554, 570 / 573, ...). Every ActionGo pair is 0x5773C0 registration + ActionGo's draw: 3081 + 2173 + 508 + 100 + ... getters.
* **Native:** Mind update 0x531D30 on queue 4; move state Think 0x537B90 / 0x537AF0, continuation 0x5379C0, forward check 0x537150, 0x536F40 collision result, 0x532B50 get back to path, 0x530AE0 node reached, 0x52FD90 turn offset, ActionGo 0x5300A0, Mover order 0x5773C0, formation follow 0x52FF00.
* **Port:** `aa_mind::element_move` 384 cases (37 trait methods), `element_move_continuation` 219 + 348 (24), `node_reached` 276 (14), `element_update` compiled, **no fixture** (15), `aa_navigation::action_go` (`crates/aa-navigation/src/lib.rs:274`), `formation_follow` 498, `result_plan` 249 (16). Reference bindings: `aa-runtime/src/live_element_mind.rs`, `movement_world.rs`.
* **aa-play:** `movement.rs:1982 element_think`, `:2076 action_go`, `:2264 get_back_to_path`, `:2341 node_reached`, `:2143 formation_speed`; `minds.rs` hosts only the aircraft Mind (`HostedKind`).
* **Takes:** a `HostedKind::Element` Mind per element, Mind arena states, and the trait impls over `MovementState` (about 100 methods; many are thin). `Owner::Mind` delivery runs `element_update`. Do it behind the same `movement::tick` façade: aa-bevy reads `Motion`.
* **Check after:** that forced-rng windows are equal up to the first regiment move (1442), with M1 to M3 in. Then re-run this census.

#### M5. Low-level avoidance (M)

* **Shows as:** ActionGo groups from ResultPlan 0x5336B0 (124), 0x533140 (107), 0x536F40 (27), 0x536AD0 (13), 0x531520 (11), first at 403 and 1469. Members stand or move differently from 1224.
* **Native:** low-path solver 0x53C170, `ResultPlan` 0x5336B0, `ResultAvoidStatic` 0x534A40, `ResultCollision` 0x536F40, 0x536AD0, continuation 0x531520; collision scan 0x4D5600 / 0x552340, overlap list 0x4D67F0.
* **Port:** `aa_navigation::low_path` 411, `uai_collision` 636, `result_plan` 249; `aa_spatial::overlaps` 192 + 2304 + 576. Host helpers in `aa-runtime/src/spatial_world.rs`.
* **aa-play:** `movement.rs:2657 blocked` and `:2600 collision`: the module header says "Simplified": a sidestep node or a tile re-route.
* **Takes:** tile actor lists in native order, overlap lists, the solver over them. **needs** M4 (these are its callbacks).

#### M6. Formation choice: property and turn (S to M)

* **Shows as:** order 1217, formation slot 2: native speed 2.5 to 3.125 by tick 1224 (`regiment_column`, `SPEED_MAX` +0.25), ours stays 2.5. 5079 speed fields. The data: `regiment_aggressive` `SPEED_MAX` -0.25, `regiment_defensive` `ATTACK_EFFICIENCY_DESIRED` -0.25, `regiment_column` `SPEED_MAX` +0.25 and `ATTACK_EFFICIENCY_DESIRED` -0.50, the others none.
* **Native:** command 0x21 on the Organization component: 0x57BFF0 / 0x577F30 / 0x57BF50 (Organization +0x70, remove the old property, apply the Formation's +0x24), then Mover refresh 0x576C70 / 0x576850.
* **Port:** `aa_actors::organization_formation_fields::set_formation_fields`; `aa_movement::mover::refresh_max_speed` and `refresh_turn_rate` (882 of the Mover set); `aa_movement::formation` 477. `aa_rules::native::layout_data` records the property, whose modifier load 0x4C42D0 is **not ported** there.
* **aa-play:** `player_orders.rs:97` records the index, and on a standing regiment issues a `Move` to the anchor (`:108`), an extra that should be read against 0x57BFF0: it is not in the fields port; `movement.rs:507 max_speed` has no formation term; `data.rs:1720` loads positions only.
* **Takes:** formation modifiers in `Def::formations`, a term in `max_speed`. The `player_orders.rs` part is the UI owner's file: same agent, by agreement.

#### M7. Recruits walking out; regiment orders (M)

* **Shows as:** `rus_infantry_co` destination: native (159, 360), ours (159, 359.5). Second member: native (159, 359), ours (158.29, 359.29). The first regiment's move at 1442 runs on: at 3296 ours is 23 m ahead of native.
* **Native:** 0x4FACA0, Mind vf +0x198 0x52F320, `KKC_UAIStateDeploy` 0x52DE40 / 0x52F560 / 0x52F7A0; regiment `KKC_CAIStateMove`: 0x520320, 0x520440, 0x5276B0, 0x525740, 0x51F7A0; slots 0x5797E0.
* **Port:** `aa_navigation::company_move` 756 + 660 (49 trait methods), `aa_movement::formation` 477; `aa_navigation::solver` in use.
* **aa-play:** `movement.rs:2948 walk_member`, `:2971 begin_walk`, `:1744 order_think`, `:1880 dispatch`, `:1200 start`.
* **Takes:** M4's host for the regiment Mind too. **needs** M4. Size stands only after M4: this is where hidden causes are most likely.

### Fog and supply lane

Files: `zones.rs`, `fog.rs`, `supply.rs`. These lanes move getters only through F3 and the supply end of an unpack. They can run in parallel with everything, and they were being edited during this census.

| id | size | shows as | native | port | aa-play | takes |
|---|---|---|---|---|---|---|
| **F1** ZOC zones | M | zone 4 missing on 8 buildings from tick 1 and 10 organizations from 402 (radius 20 / 18): 13,752 fields | 0x566DB0, 0x579DD0 via 0x4D69A0(4) | aa-zones `zone`, `set`, `solver` (189 of 247 functions proven, 9 fixtures) | `zones.rs:72`: "Not bound" | bind type 4 with its creators. A building's own zone 4 appears when its unpacking ends (715) |
| **F2** membership | S after F1 | zone 6 on trucks (402..1288, 1397 fields) and members and leaders (4083 + 2722 fields), absent natively | 0x5709D0 sets byte +0x1C when leader or inside the org's zone 4; 0x56EA30 / 0x56EC10 then make no zones | aa-zones `component` | `zones.rs:1119 membership_of` uses a control_range circle in place of zone 4; `leader_of` `:1130` | the real zone 4 test; find why the truck's byte is unset at creation (`zones.rs:676`) |
| **F3** city zone and capture | M | zone 8 missing on 4 city centres from tick 1 (radius 42), 6468 fields; 3 getters "564300 -> queue 1" from 1767; **capture tick**: native kingdom 1's income steps at 1448, 1784, 3600, 4288, and even with positions forced ours differs over 1720..1776, 3576..3592, 4272..4280 | City component; entered through 0x514E20, 0x564300 | aa-zones `zone.rs:440`, `tile.rs:151`. The doc cites 0x563E10 as creator, which is the Character's update: **find the creator** among vtable-only code | `supply.rs:728 city_admissions` admits by a radius scan once per update ("compare against the original" in its doc), `supply.rs:751` registers, `supply.rs:911 city_tick` (0x5647A0) | bind type 8; 0x564300 from the zone's enter callback; then compare the capture ticks. **needs** M4 for the tick |
| **F4** supply at unpack | S | zone 0 here at creation (radius -1 at tile 0,0), absent natively until the end of the unpacking (586..1416, 403 fields); active differs transiently | 0x576319, 0x4D3068, 0x563FD0, 0x58B0E0, 0x4D69A0(0) | aa-economy `supply` 270 / 2064 | `deploy.rs:207`, `supply.rs` | create the slot at 0x58B0E0 only |
| **F5** ZOD of a built thing | S | zone 6 radius 16 natively, 12 here on 2 buildings (1512..3600) | 0x56F140, 0x56EA10 | aa-zones `component` | `zones.rs:1112 under_construction`; `:273` | the radius rule after unpacking, against data |
| **F6** reverse RZOD | S | zone 1 here on the target and on regiments from 3304 (231 + 77 fields). The native trace has **no zone 1 at all** in this film | 0x56EDC0, 0x56EFC0, `EngagementShowReverseRZOD` | aa-zones `component` | `zones.rs:62` | likely X1's engagement; check the SVar. Re-measure after X1 |

Not causes: "zone 6 tile (position differs too)" and "supply stock +18" (a one-second phase from the drawn bucket: forced-rng moves its first tick from 718 to 1023) are knock-ons of movement and the chain.

### Combat lane

Files: `combat.rs`, `morale.rs`. No health or death difference exists in the film; morale is not sampled. No getter site of `combat.rs:1950`, `:2003` or `morale.rs` occurs natively in it.

#### X1. Kill orders (M)

* **Shows as:** five `kill` orders (3297, 3393, 3929, 4217, 4281). 3393 and 4281 are refused here "cannot attack an own unit" (`movement.rs:1119`). With positions forced to native the 3393 refusal goes, so that one is a knock-on: ours is 23 m ahead by then and takes the city first (M7, F3). 4281 stays. The target block is at full health natively at 3296 and 3392. Whether native refuses such a target at all is not established: that check is aa-play's.
* **Native:** `KKC_CAIStateKill` and the element's approach; the kill decision 0x5340C0 does not draw in this film.
* **Port:** `aa_mind::cai_thinks::kill` 400, `aa_mind::uai_kill` 840, `cai_engage` and `aa_combat` (183 proven).
* **aa-play:** `movement.rs:1115`, `combat.rs`.
* **needs** M4 and M7.

#### X2. Group behaviour (M)

* **Shows as:** order 4201 `group_behavior_enable` then a `move` on two regiments. Native creates a `group` thing at 4201 (one id, one scale draw) and moves them as a group. No equivalent here.
* **Native:** selection command 0x4ED360: cases 0x25 / 0x26 set selection group +0x18, then `KKC_GWorld::CreateGGroup` 0x4FBE10, 0x4E0940, 0x4E2940, 0x4DE740, 0x4DFE20. `group_formations.tgi`.
* **Port:** only `aa-runtime/src/player_order_guess.rs` (the guess kernel of 0x4DE740). **No port** of the group thing or its Mind: oracle and port are part of this.
* **Takes:** new selection state in `Mapper` for the decree, a new `Command`, a group Mind. **needs** M4. Last: it affects the last 338 ticks only.

## 4. Who can work in parallel

```
core:      K1 ─ K2 ──────────────┐
           K3 (map, scheduler, doodad leaf)
           K5
           K4 ── character.rs ── agreed call with movement
movement:  M1 ─ M2 + M3 ─ M4 ─ M5 ─ M7 ─ X1 ─ X2
                          └ M6 (with UI owner)
fog:       F1 ─ F2      F4   F5   F6       F3 (after M4)
```

* **First round (no two touch one file):** core K1 + K2; core K3; movement M1 + M2 + M3 (which takes `deploy.rs` for that step: tell core); fog F1, F2, F4, F5. K5 fits any free slot.
* **Shared files, by agreement:** `world.rs` (K2's `Command`), `scheduler.rs` (K3 and K4 add `deliver` arms: additive), `creation.rs` (K4's draw).
* **The check for each step** is this census, not a new tool: the getter window table for that group must rise, and the gate's matched count should move. Expect the gate's number to stay low until M1 to M4, K3 and K4 are all in: the first missing doodad draw is at 583 and the first Character draw at 1202, and the windows before 1202 only need K3 and M1 to M3. **The realistic next gate, as an expectation: every checkpoint to 1190 (70 of 266).**
* **Timing tests** in aa-movement and aa-play are known flakes.

## 5. The getter ledger

Every native getter of the film range by group: how many, first tick, how many fell in equal windows, and how many windows it was the first difference of, free and forced-rng. In the free run the first difference is mostly a later getter than the real cause. Forced-rng is the exact one.

| native group | native | first | cause | equal free | first diff free / from | equal forced-rng | first diff forced-rng / from |
|---|---:|---:|---|---:|---:|---:|---:|
| production start 589420 -> queue 0 | 15 | 2 | done | 4 | 4 / 747 | 8 | 0 |
| HQ number 4E2640 | 4 | 402 | done | 3 | 0 | 4 | 0 |
| component InitCreate 563FD0 -> queue 1 | 216 | 402 | done | 18 | 0 | 42 | 3 / 1392 |
| economy InitCreate 56F250 -> queue 1 | 8 | 402 | done | 3 | 0 | 4 | 0 |
| element mind jitter 530E50 | 60 | 402 | done | 3 | 0 | 9 | 0 |
| mover countdown phase 50AB10 | 12 | 402 | done | 6 | 0 | 10 | 0 |
| random scale InitCreate 4E9FC0 | 77 | 402 | done; unpack tick M2 | 9 | 21 / 587 | 19 | 3 / 587 |
| mover order 5773C0 -> queue 0 | 3081 | 403 | M4 | 15 | 352 / 553 | 145 | 114 / 554 |
| ActionGo from 537150 | 2173 | 421 | M4 | 9 | 0 | 95 | 21 / 1408 |
| ActionGo from continuation 5379C0 | 508 | 483 | M4 | 1 | 0 | 24 | 12 / 1536 |
| ActionGo from get-back-to-path 532B50 | 100 | 525 | M4 | 1 | 0 | 7 | 0 |
| ActionGo from low path 5336B0 | 124 | 403 | M5 | 3 | 0 | 10 | 0 |
| ActionGo from 533140 | 107 | 1469 | M5 | 0 | 0 | 7 | 1 / 4224 |
| ActionGo from 536F40 | 27 | 1452 | M5 | 0 | 0 | 0 | 0 |
| ActionGo from queue wait 536AD0 | 13 | 1801 | M5 | 0 | 0 | 0 | 0 |
| ActionGo from 531520 | 11 | 1801 | M5 | 0 | 0 | 0 | 0 |
| ActionGo from other callers (0x4EABEE, 0x52DC7A, 0x4CEE78, 0x5350AA, 0x50AF65, 0x50ADAC, 0x5314FE, 0x6F77EF) | 18 | 505 | M4, X1 | 1 | 0 | 2 | 0 |
| disturbable delay 5902C0 | 165 | 583 | K3 | 0 | 26 / 583 | 6 | 6 / 583 |
| organization goal 57C680 -> queue 0 | 4 | 587 | M3 | 0 | 0 | 0 | 0 |
| unpack end 4D21A0 -> component 563FD0 -> queue 1 | 4 | 715 | M2 | 0 | 4 / 715 | 0 | 3 / 715 |
| character 563BA0 at creation | 48 | 1202 | K4 | 0 | 0 | 4 | 0 |
| character 563BA0 from node reached 530AE0 | 91 | 1230 | K4, M4 | 0 | 18 / 1312 | 1 | 7 / 1232 |
| character 563CA0 queue 1 tick 563E10 | 63 | 1278 | K4 | 0 | 10 / 1584 | 0 | 4 / 3552 |
| character 563BA0 queue 1 tick 563E10 | 37 | 1390 | K4 | 0 | 9 / 1904 | 0 | 0 |
| character 563BA0 from Mind states (0x531EF1, 0x5324AF, 0x52FC45, 0x52DC68) | 17 | 1651 | K4, M4 | 0 | 2 / 1696 | 1 | 1 / 1808 |
| 564300 -> queue 1 | 3 | 1767 | F3 | 0 | 0 | 0 | 0 |
| (aa-play draws where native stops) | 0 | | M4 | | 44 / 552 | | 262 / 552 |

Totals, film range: native 6986. Free run ours 7959, 1127 of 1617 windows equal, 176 short, 270 long, first getter in another tick at index 76 (native 553, ours 552). Forced-rng: ours 7686, 1180 equal. S counts: film 12174, ours 13105; 198 of 266 intervals differ in count; the first are at 595 (28 / 26), 714, 731. The native trace's getter chains are stack scans, so a group name can be wrong for a few entries; the site is exact.

## 6. Every divergence by lane

Free run; `forced` columns show first tick / fields in the forced-rng and forced-all runs. Movement signatures are folded by thing and field; the motion classes (who moves, how far) are in the JSON. `first-on` 0 means knock-on.

### Existence: first 586, 90 of 1617 sampled ticks

| signature | first | last | ticks | things | first-on | forced-rng | forced-all | cause |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| missing / extra `*_trailer_*` | 586 | 1504 | 6 | 4 | 4 | 586..1504 | 587..1288 | M2 tick |
| extra `rus_infantry`, `_mg`, `ger_infantry`, `_mg`, `_mortar`, `rus_infantry_at` | 1256 | 4528 | 35 | 28 | 28 | gone | 4136/14 | one sample early: the org's regeneration phase from a drawn bucket, so K3 and the chain |
| organization members: aa-play has more | 1256 | 4528 | 31 | 4 | 0 | gone | 4136/14 | same |
| missing `ger_infantry_mortar`; members: native has more | 3992 | 4536 | 69 | 5 | 3 | 3992/69 | 3992/69 | K2 |
| missing `rus_infantry` | 4408 | 4536 | 3 | 3 | 3 | gone | gone | chain, K2 |

Native `group` at 4201 is X2: it has no `A` line, as it is not class 3, so only the creation list shows it.

### Economy: first 587, 1031 sampled ticks

| signature | first | last | ticks | kingdoms | forced-rng | forced-all | cause |
|---|---:|---:|---:|---:|---:|---:|---|
| stockpiled +1C4 money | 587 | 4536 | 1031 | 3 | 587/2311 | 588/43 | M2 tick, then integrated: a one-off delta of 0.0135 |
| provided +1A0, effective +1AC: money, ammo, oil | 888 | 4280 | 83 | 3 | 888 | 587 | M2 tick; spans 1664..1776, 3384..3592, 4104..4280: F3 capture |

Forced-all shrinks the stock difference to 43 fields, so no rate is wrong: no economy cause of its own. Shortage +1F0, consumed +1B8 and the stocks of the other six resources never differ.

### Production: none. 0 of 1617 sampled ticks.

### Movement: first 1, 1617 sampled ticks

| signature | first | last | ticks | fields | things | first-on | forced-rng | forced-all | cause |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---|
| building, other, member, truck height z | 1 | 4536 | 1617 | 32,498 | 31 | 25 | same | same | K5 |
| truck facing | 484 | 3472 | 406 | 448 | 6 | 0 | 484/446 | 484/443 | M1 |
| truck position | 484 | 3472 | 360 | 508 | 6 | 0 | 484/513 | 484/450 | M1, M2 |
| organization position | 484 | 4536 | 718 | 2980 | 10 | 1 | 484/2950 | artefact | M3, M4 |
| truck speed +20 | 552 | 1392 | 9 | 19 | 5 | 0 | 552/20 | 552/11 | M4 |
| building facing, position, speed | 586 | 1504 | 5 | 13 | 3 | 0 | 586 | 886 | M2 tick |
| member position | 1224 | 4536 | 325 | 5891 | 46 | 10 | 1224/5886 | 1224/5566 | M6, M7 |
| member speed +20 | 1224 | 4536 | 385 | 5079 | 45 | 8 | 1224/5063 | 1224/5091 | M6 |
| member facing | 1264 | 4536 | 410 | 5727 | 45 | 4 | 1272/5703 | 1272/5100 | M7 |

### Combat: first 586, 6 sampled ticks

Building and truck health and maximum at 586, 888 and on to 1504: the HQ swap one tick off (500 against 2000). M2.

### Supply: first 586, 623 sampled ticks

| signature | first | last | ticks | fields | forced-rng | forced-all | cause |
|---|---:|---:|---:|---:|---:|---:|---|
| building zone 0: aa-play has it, native none | 586 | 1416 | 403 | 403 | 586/398 | 588/401 | F4 |
| building supply stock +18 | 718 | 2848 | 288 | 426 | 1023/179 | 717/440 | chain (bucket phase) |
| building zone 0: active differs | 1016 | 3600 | 43 | 43 | 1016/47 | 715/14 | F4, chain |

### Fog: first 1, 1617 sampled ticks

| signature | first | last | ticks | fields | things | first-on | cause |
|---|---:|---:|---:|---:|---:|---:|---|
| building zone 4: native has it | 1 | 4536 | 1617 | 8787 | 8 | 4 | F1 |
| building zone 8: native has it | 1 | 4536 | 1617 | 6468 | 4 | 4 | F3 |
| organization zone 4: native has it | 402 | 4536 | 1216 | 4965 | 10 | 10 | F1 |
| truck zone 6: aa-play has it | 402 | 1288 | 727 | 1397 | 4 | 4 | F2 |
| organization zone 6: tile, position differs too | 530 | 4536 | 478 | 753 | 10 | 0 | knock-on |
| building, truck detection ZOD radius +14 | 586 | 1504 | 6 | 6 | 3 | 0 | M2 tick |
| building zone 6: aa-play has it | 586 | 4536 | 1032 | 2722 | 4 | 0 | F2 |
| member zone 6: aa-play has it | 1208 | 4536 | 417 | 4083 | 46 | 14 | F2 |
| truck, member zone 6: tile, active, position differs too | 1384 | 4536 | 365 | 1908 | 12 | 0 | knock-on |
| building zone 6: radius | 1504 | 3600 | 30 | 31 | 2 | 0 | F5 |
| building, organization zone 1: aa-play has it | 3304 | 4536 | 155 | 308 | 4 | 2 | F6 |

### RNG: first 552, 964 sampled ticks

GWorld+D4. Section 5 has it by getter. Before 552 every window is equal; state compare differs only after the chain is off by a getter (583, K3).

## 7. Open questions

1. **Queue 3 owners.** K3 rests on reading the code: 0x50BF10 clears and loads, and `saved_periodic` passes over those lists. The 796 entries are measured; that they include the doodads that draw at 583 is the first thing to confirm (resolve the owners, or dump queue 3 with the sampler).
2. **The creator of zone 8.** See F3.
3. **What X1 will show** once a regiment arrives on time: whether an attack on the block starts, whether the 4281 refusal goes, and whether the zone 1 row (F6) goes.
4. **M7's size.** Every movement row after 1442 is read from a regiment that is by then 23 m off. Expect new causes.
5. **`Trace` has no site.** Our getters cannot be named as native's are. A `SKind::RngGetter` site label (a `&'static str` beside the state, core) would turn "aa-play draws where native stops" into names. Requested.
