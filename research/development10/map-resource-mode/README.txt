Development10: loaded-header flag, task mode, camera source priority and tail owner

Production additions (no parser, Catalog, restoration or native binary changes):
  crates/aa-runtime/src/map_resource_mode.rs
  crates/aa-runtime/src/load_tail_source.rs
  their focused tests; isolated aa-runtime lib exports.

1. World-start flag is NOT the task mode.
   59E520 follows MapResource+90 -> loaded KKC_MapHeader+4 -> WorldRules+4.
   511860 indexes the seven initialized WorldRuleMode payloads and reads byte0.
   Existing immutable native_initial_zones.py executes CRT9CCA10 ->4994D0
   ->4981A0 registration,59E520 and511860, both original/v14/all three PCs:
   126 mode rows,12 unrelated tile-zero rows,217 executed instruction pins.
   This work replays the actual registration names and both payload bytes.
   Source fixture SHA f8779e09d28a08e5a1c05325a0f00fed4648e8ba85ccce037d799c2ec9f271af.
   Modes0/1/3 have byte0=false,2/4/5/6 true; byte1 is true only4/5.
   Out-of-table modes are refused rather than pretending native OOB is safe.
   All three original map headers tested have mode0, therefore false.
   A diagnostic creator-only mode change leaves the header result unchanged.

2. Scenario launch/task temporary MapResource+A8.
   775EE0 launch.mode2 reads cached game_type at775F69 BEFORE the diagnostic.
   Cached comparisons to3/5 survive that service; current game_type is reread
   for the missing-resource/film branches. scenario_task retains that ordering.
   No scenario and current type!=4 is a failed prefix. Current type4 creates
   Film; otherwise cached type3 ->load task kind1,5 ->kind2,other ->kind0.
   59E6F0 stores supplied creator+1C, kind+20, zero+24; kind1 copies creator+50.
   Run5A4280 branches at5A4311: kind1 ->5A3E20 ->LoadMap mode2;
   kind2 ->5A3F20 ->mode4; all other kinds ->5A3EB0 ->mode1.
   Actual LoadMap5A2B90 stores that argument into Resource+A8 at5A2BF1.
   It saves the earlier A8 and restores it on normal return. The helper does
   not certify a persistent Resource owner, task completion or unwinding.
   native_map_resource_mode.py executes114 dispatch cases plus30 raw task-kind
   cases, both executable builds/all PCs,232 nonleaf executed instruction pins.
   Five cases explicitly intervene in diagnostic-seam storage after the cache;
   these are ordering probes, not application reentrancy evidence.
   Prior task/reset/sync services, task/film construction dispatch and real
   resource/world factory5A28B0 remain explicit service seams.
   Cutoff is5A2BF7 immediately after the mode store, not a full map load.

   Resource+90 is reconstructed by5A2A40 from the loaded MAP_HEADER. Later
   mode1 may replace Resource+88 creator with a selected creator clone; that
   does not replace Resource+90. The public header helper must borrow the
   actual retained loaded header. Session launch.mode, Session+F8, creator
   conditions and task kind do not supply this flag.
   The real local skirmish Session/selected-launch owner is a separate task.

3. Native camera starting priority, ActorData+1B4.
   4625A0 initializes an earlier constructor word to0, but464DB3 requires
   GetInteger(starting_location_priority).5B4ED0 throws if no field exists;
   therefore missing input never falls back to the constructor word.
   Integer vtableA487C8+10 points to58CB30 (load field+10 and return).
   464DC7 stores the result BEFORE464DCD rejects a negative signed integer
   via410C40. Zero is accepted despite the diagnostic wording 'positive'.
   native_starting_priority.py executes real5B4ED0 and58CB30 from464DB3:
   42 rows/44 executed pins across both images/all PCs, including missing,
   negative extrema, zero and largest positive integer.5B4750 lookup is an
   explicit typed-field seam. Missing-field cutoff is5B4F03 before exception
   construction; negative stops at required fatal410C40 after its store.
   No whole parser/ActorDataLoad/exception-unwinding success is claimed.
   The Rust addition binds an immutable input, not a mutable partial native
   allocation: it refuses negatives without publishing a source field value.
   Existing typed integer parser/provenance resolution is reused unchanged.

   starting_location_priority(Catalog,RuntimeDefinition) requires the SAME
   existing Bank token, verified complete source owner, source identities,
   canonical definition provenance/template chain/category and source PC.
   It neither allocates a new identity nor uses restoration PC. The input's
   actual source/declaration spans are retained. Public caller-supplied metadata
   cannot replace the canonical field. This is a controlled fresh immutable
   source projection, not a certificate for every4BD800 source pass.

4. One retained constructor-owned tail state.
   TailState::from_world_constructor() owns started_274 and CameraState.
   4F7D35 executes EBX=0;4F7FA7/AD store zero camera22C/230;
   4F803C stores false at274. Native constructor prefix executes18 cases:
   original/v14/all PCs with allocation fill0/A5/FF,166 executed pins.
   Embedded constructors and successful allocation are explicit nonalias
   seams. Cutoff4F8048 is before the remainder of World construction.
   This only certifies these direct stores. The same non-Clone/non-Default
   owner must be created before records and moved with the actual World.
   Recreating it at PostLoad would discard real earlier mutations.

5. Concrete borrowed tail assembly (separate follow-on task).
   load_tail_binding::after_post_load takes the actual Staged, immutable source
   Catalog and retained opaque SessionRead owner. The existing load_tail_world
   World adapter is now generic over that borrowed session interface, with the
   old explicitly supplied SessionFacts remaining as the bounded fixture view.
   LocalBootstrap's implementation is owned/tested by the netcode author; no
   default player/name/Kingdom assignment is invented in this module.
   application_state is reread from SessionPhase. Player+24 is a live owner
   getter; the still-unbound+28 path remains an explicit error when reached.

   Attempt consumes its Catalog and moves the Bank into Staged. The additive
   starting_location_priority_in_bank entry therefore accepts that SAME moved
   Bank explicitly, validates source/token/provenance again and does not build
   a second graph. A regression verifies the boxed State's address is unchanged
   across that move and that the emptied old Catalog can no longer bind values.
   The real3map source test binds24/43/28 distinct source definitions. Most
   values are0; Berlin placeholder_corpsunpacked is1000 from its actual typed
   assignment. A forged provenance is refused. The test is source binding, not
   evidence that all three maps complete live restoration/PostLoad.

   The concrete adapter reads the actual boxed MapResource mode alias and the
   loaded header flag only at the native1/4 mode gate. GMap+C/+10 are the actual
   retained terrain's ceiled extent integers converted tobinary32 (556638..65B),
   not its shifted grid cell counts. Kingdom slots come from the retained
   World.kingdoms owner; head-first members come from the actual stored Kingdom
   actor_lists.by_class[3] (+284). It never reconstructs membership by an owner
   scan. Read-only snapshots are bound before the native interval; reentrant
   mutations during this assembly are outside this successful binding domain.
   The immutable-priority binding scan does not create/change membership.

   Mutable camera writes borrow the same retained TailState. The existing
   coordinator verifies PostLoaded record ownership and preserves native
   camera-store-before-log order. Preflight binding refusals are explicitly
   outside native invalid-pointer/exception-unwind parity. A fresh mode0 map
   header skips WorldStart without requiring unused economy/statistics inputs;
   a reached saved/metagame start remains an explicit required-service error.
   Conversion/publication/BeginPlaying and actual game updates remain separate.

Validation/replay (PowerShell, from rust-rebuild):
  $env:PYTHONPATH='E:/Games/Axis and Alies/reconstruction/tools/python-packages'
  python research/development10/map-resource-mode/native_map_resource_mode.py
  python research/development10/map-resource-mode/native_starting_priority.py
  python research/development10/map-resource-mode/native_tail_constructor.py
  python research/development10/map-resource-mode/native_gmap_extent.py
  python research/skirmish-20261016-load-zone-services/native_initial_zones.py
  All producers compare frozen JSON by default; --write is explicit generation.
  $env:CARGO_TARGET_DIR='target/s-settlement-completion'
  $env:AA_GAME_ROOT='E:/Games/Axis and Alies'
  cargo test -p aa-runtime --release --locked --offline -j4 --test map_resource_mode -- --include-ignored
  cargo test -p aa-runtime --release --locked --offline -j4 --lib load_tail_source::tests -- --include-ignored

Boundaries still requiring actual owners:
  selected local Session/player/bootstrap and full launch lifecycle;
  exact retained MapResource/GWorld+28 alias (another author);
  complete PostLoad then actual load_tail_world invocation;
  statistics/economy, native ordered Kingdom+284 lists and local player+24;
  subsequent conversion/publication/update/render/input/battle loop.
These helpers do not claim a playable battle, successful whole map load or
readiness. Existing world_load_tail/world_start_world kernels are reused.

Retained superseded attempts:
  priority-generate-v1.log: harness used nonexistent wide() helper; fixed by
  explicit UTF16 allocation before generation-v2. No fixture from failed run.
  priority-tests-v1.log: test-only attempted Deserialize on Provenance (which
  has no Deserialize); replaced with its concrete struct. Also contained a
  concurrent unrelated Context constructor error fixed by its owning author.
  priority-tests-v2.log is a pre-freeze run started before the TailState test;
  final test evidence is separately named, never overwriting prior results.
  priority-tests-v3..v6.log retained snapshots of concurrent shared edits:
  v3 saw a duplicate SessionPhase crate identity in another author's unit test;
  v4 saw spatial_world.or_removal_flags before its method landed;
  v5 saw the in-flight TriggerUpdate return-type mismatch;
  v6 saw Economy owner ActorId before its required .raw conversion landed.
  Those errors were fixed by their owners and are not current defects in the
  tail adapter. None of these failed builds is counted as final validation.

Current integration qualification:
  The actual LocalBootstrap owner now exposes a bound+28 only after its native
  765E10 writer completes. This adapter reads it live only when the corresponding
  native tail branch reaches that getter. Full773E30/BeginPlaying sequencing,
  record lifecycle completion and client publication remain separate root work.
  The immutable source tests below never replace that local-session lifecycle.
