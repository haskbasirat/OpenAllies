# Battle startup critical path — October 16, 2026

This is a read-only source audit of the current Development-09 continuation.
No SDK implementation, game files, production Rust, tests or releases were
changed. Native addresses identify original AA.exe behavior. Decompiled files
were read as leads; this audit adds no arithmetic or native execution proof.

The main delay is integration of the original object's complete lifecycle,
followed by connecting that initialized world to deterministic updates. The
asset viewer and isolated native-function tests do not execute that lifecycle.
There is no defensible finish date or whole-game percentage from these counts.

## Current gates

| Gate | What exists | What still blocks the next end-to-end result |
| --- | --- | --- |
| Definition completion | Canonical source inputs, component masks, layout and much of the native source math | Completed ActorData combat value at `+1E4`; its real dependencies and shared definition mutations must run in native manager order. Settlement `460940` fits TimeGate gameplay shapes, not Gamebryo mesh spheres. |
| Saved-record load | Native ID/source loading and owned saved component restoration; 20 of 80 maps have clean record sections and accepted later PostLoad in the stopped census | 577 record stops at completed `+1E4`, 151 at `4D64F0` state change, and 68 at `579A00` leader assignment. These are the measured first missing boundaries for 796 records, not 796 distinct missing functions or a complete gap census. |
| Immediate object lifecycle | Object `4EBE50`, Actor `4D7230`, Team `4E68C0`, Kingdom `4E5DF0` coordinator ports and native tests | None is bound to a production runtime `World` implementation. `4FAE80` must call `+E8 Init(NULL)`, check success, `+EC PostCreate(0,1)` and `4F9510` before the next saved record. The present loader advances to AwaitingPostLoad without doing these steps. Constructor identity registration is earlier and is a separate operation. |
| World completion | Queue ownership, later PostLoad, bounded camera tail and world-start statistics adapters | Conversion capture is not conversion execution. Destruction, fresh creation, transfers, actual world lists, SUN/MISC_DATA ownership, InitForLoadedMap, mission initialization and SAI activation/publication must be connected. `Staged::init_for_loaded_map` still returns an explicit error. |
| Ordered simulation | Native admin/world tick driver, queues, clock/RNG ownership and many subsystem ports | A production world must implement all required `clocked::Services` and driver network/decree services. Generic coordinators and test leaves do not provide a live battle. |
| Playable skirmish | Original menu and inspection renderer | Command selection/order UI, actual actor/component updates, combat, recruitment/economy, fog, AI, victory/defeat, restart and save/load must operate together over the initialized world. |
| Multiplayer, then campaign | Native subsystem research and bounded ports | Session/world transport, decree history, synchronization/recovery, lobby/hosting/joining/save transfers and real separate-PC tests come after live skirmish; campaign/WWII progression follows. Separate-internet testing remains unavailable under the last human answer. |

The exact Development-09 diagnostic census is
`qa-output/development09/native/managed-mode1-comparison.json`. All 80 maps
still have zero completed immediate Init, zero publication-ready and zero
battle-ready results in that census. The 20 record-clean results must not be
described as 20 playable maps.

## Nearest reusable world services and safe next implementation

The following existing ports should be reused where their native call paths
match, rather than replaced by fresh approximations:

* `aa_spatial::snap_half_cell`: `46EE00`, explicit precision and live XY.
* `aa_spatial::index_world::GroundSurface::sample`: `4F0E50` using real base
  and overlay heightfields plus the terrain-cell metadata.
* `aa_runtime::spatial_world::SpatialWorld`: authoritative ordinary Actor
  source geometry, placement ownership and `4EE4C0` membership. It currently
  rejects derived/non-Actor dispatch and saved membership bit26. Its preparation
  requires a live ID and source/component state, so it is not a universal Team,
  Kingdom or currently-loading-record adapter.
* `SpatialPlacement` under that module: `4F3C40`, `53CE40`, `4F46E0` and native
  spatial/query services. All `PlacementHost` callbacks remain required.
* `aa_zones::detection`: tile-to-actor detection, `4EA150` and `4EBC70`
  coordinators. Their real ZoneWorld/ActorDetectionWorld, memory and list owners
  still have to be supplied; a function existing does not mean it is integrated.
* `aa_actors::component_virtuals`, `leaf_init`, `world_publication` and
  `ActorStore` ground/Body/Mind/Performer helpers cover portions of component
  lifecycle and PostCreate. They do not constitute all existing-load component
  dispatch, all properties, or publication, and some fresh-only methods cannot
  substitute for Init(NULL).

One bounded next module can be implemented independently: the saved-object
`4F4590` placement wrapper and reached `4F3640` coordinator, with every host
operation mandatory and proved against original/v14. This closes a concrete
unported service and enables a later real ObjectInit adapter. It does not by
itself initialize a world.

`4F4590` is not a direct call to the already-ported `4F3C40`. It constructs the
actual `A25B28` descriptor from the current definition, position, facing,
virtual `+B4/+108` and flags, and routes it through `4F3640`. The latter has
additional required-object, footprint and definition-dependent branches,
including return codes 5, 6 and 7. Calling `SpatialPlacement::verdict` directly
would lose behavior. Similarly, supplying a fresh-creation GACD block instead
of the original NULL wrapper would change inputs and writebacks. The wrapper's
own parameter order and source rereads require an instruction/oracle check;
the exported pseudocode alone is not a completed port.

The ObjectInit trait documents **11 mandatory service groups**: snapping,
placement, diagnostics, spatial membership, terrain height, blocking
notifications, footprint/detection, world-list insertion, memory retention,
memory cleanup and visibility refresh. This is a bounded grouping of its
interface, not the number of independent tasks left in the rewrite. Native
virtual class dispatch and current-record storage access also remain required.

## Measurable exit criteria

1. **First initialized world:** one selected real original map completes all
   record loads with immediate Init/PostCreate/publication in native order,
   true conversion drain, later PostLoad, loaded-map initialization, mission
   startup and required manager/world lists. No synthetic zero/default callback
   may satisfy an unbound service. Verify relevant object/state/list/queue/RNG
   transitions against the original; then expand coverage across all maps and
   save versions. This milestone still has no player-controlled battle claim.
2. **First ordered simulation:** that world performs its first and subsequent
   real ticks through required services, with native admin/decree order,
   authoritative clock, RNG and component scheduling. Compare a recorded native
   trace; no renderer delta may enter the simulation. A test clock tick alone
   does not satisfy this criterion.
3. **First playable skirmish:** a complete human-versus-AI match can start,
   select/order units, move/pathfind, attack/die, build/recruit, apply resource
   and fog rules, reach victory/defeat, save/reload and restart with original
   behavior. This is a better schedule calibration point than function count.
4. **Multiplayer acceptance:** two real processes first run the same skirmish
   deterministically; then verify lobby/host/join, ongoing commands,
   synchronization and save transfers on separate PCs and separate connections.
   Local mocked transport cannot certify the public network experience.
5. **Campaign acceptance:** original missions, scripted objectives, transitions,
   progression, WWII/metagame and original saves pass after skirmish and network
   foundations. This remains the user's lower priority.

Native fixture counts prove the measured functions, inputs and explicitly
hooked seam boundaries. One fixture may cover hundreds of numeric combinations
while its mandatory child services are test probes. Thus a large backend proof
count can be valuable and still produce no new playable capability. Future
progress reports should name the highest end-to-end exit criterion reached and
the next real blocking boundary, not equate lines/tests with game completion.

## Source locations checked

* `crates/aa-mapworld/src/restore.rs`: `load_record`, `apply_loaded`,
  `restore_records_with_mode`, `LoadCompletion`.
* `crates/aa-runtime/src/restoration.rs`: `Attempt::load_with_bindings`,
  `into_staged`, `Staged::post_load`, `init_for_loaded_map`.
* `crates/aa-actors/src/{object_init_null,actor_init_null,component_virtuals,leaf_init,world_publication,owned}.rs`.
* `crates/aa-mapworld/src/team_init.rs`; `crates/aa-economy/src/kingdom_init.rs`.
* `crates/aa-runtime/src/{spatial_world,spatial_policy,actor_world,world_start_world,load_tail_world,restore_lifecycle,clocked,simulation,skirmish_start}.rs`.
* `crates/aa-spatial/src/{lib,index_world,gworld_placement,height_layers}.rs`;
  `crates/aa-zones/src/detection.rs`.
* Original exported pseudocode `004f4590.c` and `004f3640.c`; existing Object
  Init native evidence under `research/skirmish-20261016-object-init-null/`.

No builds or new native executions were needed for this bounded audit.
