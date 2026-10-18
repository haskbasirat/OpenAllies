# Phase 8 movement: requests to the lead

Written by the movement/pathfinding/formation agent. Nothing below has been
changed outside the files this agent owns.

## 1. No Cargo.toml / Cargo.lock change is needed this round

All new code uses existing dependencies (`aa-numeric`, `aa-trig`, `serde`,
`thiserror`, `serde_json`). `aa-movement` reimplements the 13-line
`0x552640` clamp locally instead of depending on `aa-navigation`, so the lock
file is untouched.

## 2. Terrain speed input (owner: restoration agent, `aa-rules/src/terrain_speed.rs`)

`aa_movement::ground_step` (`0x5774a0`) reads the terrain speed cap from
actor+0xF8 TerrainData+0x38 and floors a positive cap with the SVar
`TerrainMinimumMoveSpeed` (native default 0.5, registration `0x508f29`).
`aa-runtime::movement_world::MotionEffects::terrain_speed_limit` must return
those exact binary32 bits. Please have the owner of `terrain_speed.rs` expose a
source-proven function `actor -> TerrainData+0x38 bits` (the producer that
writes TerrainData+0x38, including `TERRAIN_SPEED_CAP` modifiers parsed by
`0x4c11e0`) with its own native oracle. Movement does not compute it.

## 3. Runtime storage needed before `mover::tick` can replace `MotionWorld::advance`

`aa-runtime::movement_world::MotionWorld::advance` calls the ground step
directly. The faithful entry is now `aa_movement::mover::tick` (`0x577e50`),
which first checks falling, active component, the Properties suspension bit,
the zero-speed arrival/failure stop and then refreshes the actor's
overlapping-thing list (`0x4d67f0`) before the ground/aircraft step. Wiring it
needs state that the runtime stores do not have yet (owners: actor/world
store owners, `aa-actors`, `aa-runtime/src/actor_world.rs`):

- actor +0x110/+0x114 overlapping-thing list: `Vec<aa_spatial::overlaps::OverlapEntry>`
  per actor (reference + expiry bits), head first;
- Properties component (actor+0x78) byte +0x1C (bit1 suspends movement);
- Mover falling words +0x58..+0x88 (already in `aa_actors::ground_states::MoverState::words`
  for +0x60..+0x84; +0x58, byte +0x5C and byte +0x6C are written only by the
  fall starts and are correctly absent until then);
- the Mover's "has schedule entry" (Mover+0xC) for `mover::restore_falling`;
- AIRCRAFT component (actor+0xBC) presence and its virtual +0x38 step
  (`0x5612f0` MoveTick is ported in round 2 as `aa_movement::aircraft::move_tick`;
  see item 10).

When these exist, this agent will add `MotionWorld::tick` (owned file) using
`mover::tick`, `falling::fall_step` and `aa_spatial::overlaps::refresh_overlaps`.

## 4. Organization component state for formations

`aa_movement::formation` needs, per organization: layout primary slot
(+0x18), group sizes (+0x1C records +8, count +0x20), slot count (+0x2C), the
member per slot (+0x28), leader (+0x40), current formation (+0x74: definition
byte +0x18 and offset table +0x10), slot positions (+0x78) and center
(+0xBC/+0xC0). Please route the Organization component payload (owner of
`aa-actors` Organization/Element components and `aa-runtime` actor stores) so
that `formation::compute_all_slot_positions` / `set_leader` can run on it. Its
RNG callback must draw from the single GWorld+0xD4 stream (`0x85d840`), the
same stream `aa-navigation::action_go` uses.

## 5. ActionGo formation gate

`aa_navigation::try_action_go` calls `TryActionWorld::formation_ready` for
mode 3. The original body of that gate is now
`aa_navigation::formation_follow::formation_speed` (`0x52ff00`). The owner of
the Element move runtime adapter (`aa-mind` element_move / `aa-runtime`
mind world) should implement `formation_ready` by calling it with the actual
organization/leader paths (Mind +0x44 views) and `MoverCatchupSpeed`.

## 6. Thread rule for path queries (for the scheduler owner)

See README.txt "Threading rule". Path queries may be computed on worker
threads only from an immutable per-tick snapshot and committed in native call
order; the region-search traversal epoch, tile-search epoch, intrusive spatial
query links and strategy/collision references are shared native state and
must be advanced/committed in that order. No worker-thread path code exists
yet; the rule constrains whoever adds it.

## 7. Sticky precision-exception state (answered)

Answered by the lead: the simulation runs at PC53 and PE is set on every
tick. Movement callers pass `precision_exception_pending = true` to
`ground_step_original` / `OriginalTurning`; under PC53 this matches the
PC53-only provider.

## 8. SAI GoalBase (done, per your request)

`aa_navigation::sai_state::GoalBase` now has public native fields `status`
(+0x08), `fuzzy` (+0x18), `costs` (+0x1C/+0x20, owned Vec), `cost_capacity_word`
(+0x24), `inertia` (+0x28) and `priority` (+0x38), plus `status_signed()`.
The serialized layout is unchanged for the phase7 fixtures. The AI agent can
map them directly onto `aa_mind::sai::goal_priority::Goal`.

# Round 2

## 9. CAI regiment move: bind `CompanyThinkWorld` (owners: aa-mind / aa-runtime mind world)

`aa_navigation::company_move` ports the CAI move state (`0x520320`,
`0x520440`, `0x5276b0` and the member dispatch `0x525740` family) and Element
return-to-slot (`0x52e9d0` / `0x52e900`). It expects its services through
`CompanyWorld`, `CompanyThinkWorld` and `SlotReturnWorld`. Please have the
Mind/Regiment owner implement those traits on live state:

- the Mind state framework `0x529c00` / `0x52a810` / `0x52a000`;
- group objects `0x4ddee0` / `0x4de6c0`;
- `0x51fbc0` target-in-range, `0x52cae0` region test, `0x4cede0`, `0x44dea0`;
- the time source `0x76b9a0`;
- `Solve` (`aa_navigation::solver::solve`);
- `prepare_leader_query` (`0x554350`). With no organization leader, the
  original reads the null page; Rust returns an error, and the caller must
  not treat that as success;
- member Mind `+0x100` / `+0x10c` (`aa_navigation::move_path_event`);
- member orders `0x4ca760` / `0x4d9b60`.

The world RNG draws in `0x5233f0` must come from the single GWorld+0xD4
stream. These ports are proven by replay only, and nothing calls them live
yet.

## 10. Aircraft MoveTick (owner: tick integrator)

`aa_movement::aircraft::move_tick` (`0x5612F0`) is ported and proven
(`native-aircraft.json`, 1281 cases). The tick integrator should implement
`TickWorld::aircraft_step` by calling it. Its callbacks are not ported:

- leg builder `0x55F230`;
- follow planner `0x560A70`;
- formation teardown `0x55EDC0`;
- the Aircraft constructor and destructor `0x55E8E0` / `0x55EA70`.

`aa_movement::falling::commit_pose` (`0x576BB0`) is now public so that the
aircraft pose commit can use it.

## 11. Static SAI topology builder: caller duties (owner: map loader / SAI)

`aa_navigation::topology_builder::build` (`0x7ACB00`) reproduces the topology
shipped in all 80 maps at PC53/PC64, but not at PC24. That fits the PC53
correction. The loader must:

- call `0x7AC9A0` (clear) semantics first;
- supply the maximum rectangle width (`0x7AA180`, CRT pow via FYL2X/F2XM1,
  not ported). Ten older maps need a historical 27/28;
- run the primary-island post-load step in the native order;
- own the `0x79FF10` / `0x5D85C0` teardown.

The 0x1000-rectangle message is a warning, not a cap. The island-flood
recursion is an explicit heap stack.

## 12. Water areas (owners: aa-mapworld, aa-heightfield, map loader, solver users)

`aa_spatial::water_areas` ports the `0x539E30` / `0x4F4050` flood fill,
`0x539C40` IsIndexBlocked, `0x5559F0` stroke and `0x555C10` PaintTerrain
(`native-water-areas.json`, 417 runs including 30 floods on original maps).

- **aa-mapworld:** `TerrainCell.byte_2c` is the water-area id (tile+0x2C).
  Please name or document it that way. Tile +0x24 / +0x28 / +0x2D are the
  solver link, visit stamp and arrival direction.
- **aa-heightfield** (this agent's crate): `sample_flags` is now documented
  as the per-vertex water-area byte grid at heightfield+0x44, where 0xFF means
  shared. No action is needed.
- **Map loader:** please find where the original restores GMap+0x50 area
  record counts at load. The Rust code takes the counts as input.
- **Solver users:** `FloodSolver::busy` models the single process-wide byte
  `0xB3D43D`. Every `0x4F4050` user must share one instance, including the
  ZoneSolver callers `0x540590` / `0x540690` / `0x5406d0` and `0x4f6b50`.
- **Brush stages:** `0x5526C0`, `0x559FB0`, `0x5528F0`, `0x551140` and
  `0x551B40` are traits. They belong to the terrain-brush owner.
- **Random-map callers:** `0x5972F0` and `0x597530` are not ported.

The test `water_areas_original_map_floods` reads
`reconstruction/assets/original/Data` and is not `#[ignore]`.

## 13. formation_ready (backlog, unchanged)

The Element-move `formation_ready` adapter lives in aa-mind, not in this
agent's files, so it stays in research/phase8-integration-backlog.md (item 5
above).

## 14. GoalBase <-> aa_mind GoalState / Roster (owner: AI agent, aa-mind)

aa-mind depends on aa-navigation, so an aa-navigation -> aa-mind conversion
would create a dependency cycle. The conversion therefore belongs in aa-mind.
`aa_navigation::sai_state::GoalBase` now exposes everything it needs as named
public fields. The serialized phase7 layout is unchanged.

- `GoalState` maps field for field:
  - `status` = `GoalBase::status` (+08);
  - `costs` = `GoalBase::costs` (+1C/+20);
  - `travel` = `GoalBase::travel` (+30, renamed from `scalars30_34[0]`);
  - `current` = `GoalBase::current_priority` (+34, renamed from `scalars30_34[1]`);
  - `base` = `GoalBase::priority` (+38).
- `Roster.goal_actors[goal]` maps to the +0C/+10/+14 list.
  - GoalBase stores only the raw header words; head and tail are native node
    pointers.
  - A conversion is therefore exact only while `actor_list_unlinked()` holds,
    which is the state 0x7A66E0 constructs. That case gives
    `ActorList::from_actors([], actor_count())`.
  - With non-null head or tail, return an error. Nodes must not be invented.
- Fields GoalState does not carry stay on GoalBase:
  - `fuzzy` +18, `inertia` +28, `byte2c`, `cost_capacity_word` +24, `controller`.
  - A GoalState -> GoalBase write-back must preserve them.

# Round 3

## 15. Runtime hosts for the new adapters (owners: runtime / actor store / aa-mind)

The adapters live in `aa-runtime/src/spatial_world.rs` and are wired to the
proven aa-spatial ports. Their remaining leaves are host traits, and none of
them has a default.

- **`low_path_world::PlacementHost`** needs these leaves:
  - 0x4F2220 structure at a point;
  - 0x4D36F0;
  - structure virtual +0x168;
  - 0x585280 and 0x585160 (both on structure +0x98);
  - object virtual +0x100;
  - actor virtual +0x164;
  - GMap tile +0x10 (SAI region node) and node +8;
  - actor +0x7C and object +0x40.
- **`low_path_world::LowPathHost`** extends PlacementHost and adds:
  - source virtual +0;
  - the actor +0x110/+0x114 list as `Vec<OverlapEntry>` (also item 3);
  - GWorld+0xE0 time;
  - 0x4D10A0.
- **Region prepass and endpoint hosts:** call
  `SpatialPlacement::verdict` for `region::World::blocked_here` (0x53DC00)
  and `SpatialPlacement::endpoint` for `endpoint::World::acceptable`
  (0x53CE40).
- **aa-mind `world_4f46e0`:** call `SpatialPlacement::nearest` (0x4F46E0).
- **`UaiCollisionWorld` host (aa-mind):** use
  `uai_collision_world::{shapes_overlap, shapes_in_range, tile_shape, probe,
  ignore_thing, actor_moving}`. The 0x4CE6E0 ignore actor is resolved by the
  host.
- **AI agent's UAI host state:** UAIStateMove +0x78 (last low-path solve
  time). Noted in the lead backlog.
