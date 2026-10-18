SAI world-startup binding (work in progress; no playable/readiness seal)

Authority and scope
Original AA.exe and accepted v14 instructions are the behavior authority. The
adapter never infers AI slots from a human player's AI byte, map rendering,
source-rule names, or a count of currently supported controllers. WorldReady
must receive the actual retained manager, player arena, map, ActorStore,
SpatialWorld, major Kingdom registry, ZoneMap, source Bank and world revision.

Owned implementation
load_sai_world leases actual manager fields into the existing 7A0D40 kernel;
there is no State snapshot/copyback. Map loads native topology from the
untouched TERRAIN suffix and prepares adjacency, areas, bottlenecks and route
cache from the same terrain/spatial/zone owners. The currently supported depth
requires one registered Integer assignment matching the actual Settings raw
value/type/provenance. A separate private attempted capability prevents a
failed native ready-byte prefix from authorizing a retry.

load_sai_players constructs one allocation arena beside the application SAI.
The manager +60 vector remains the only slot mapping; +4C and +54 are distinct
constructor-empty work lists. Profile source order, deferred Ego head links,
localization, native profile filtering and the private SAI RNG are retained.
SessionPlayer proxies validate the exact Session allocation lifetime. The
79EA90 profile lookup returns the first matching actual nonnull slot.

load_sai_startup::initialize is the concrete outer world facade. 7B4830 mutates
the SAME boxed player's Initializing/controller/goals. The old aa-navigation
initialize APIs remain wrappers over the common in-place algorithm. Fallible
source getters run at the reached script, exploration, Commander/recruitment,
RecruitSingle and neighbor stages. A refusal retains prior published state.
RecruitSingle goal references are not Actor GTIDs. Generic goals have one
payload owner; pending and group collections contain indices into it.

load_sai_actor uses installed Mind +28 (not Body), actual Element ownership,
Structure/Wall container dispatch and current GTID generations. Runtime vtable
and source-category/storage checks precede dispatch. The complete category3/4
branches and selected ordinary paths use actual player-slot contents. Missing
controller effects are never represented by an invented empty vector.

Current explicit limits
The whole-world facade has compiled but has not yet run on a completely
post-loaded actual map ActorStore. Root's current strict load is still blocked
before that point. Positive Commander ability allocation references, nonempty
RecruitSingle allocation references, positive ordinary category0 controller notifications,
matching-controller notifications, Flare retirement, MARKER side effects and
positive Wall container getters remain required when reached. Active AI
Think/fiber/controller execution and application/mission activation are not
completed by this component. A manager ready byte is not a successful startup
or playable-battle certificate. Ready=true AddPlayer preserves slot/worklist
publication and then reports the remaining required initializer/notifications.

Evidence denominators (each row executes separately on original and v14)
Outer startup:17 rows/34 machines,255 executed pins; named child/virtual seams.
Map constructor:9/18,52 executed pins plus allocator entry.
Actual tutorial topology/preparation:3/6 all runtime PC,source PC53,1652 executed
pins; source input is untouched original map suffix/current static tile facts.
Actor category3/4:162/324,179 executed pins,no seams.
AIPlayer constructor:9/18,521 executed pins,one allocator entry.
Inactive AddPlayer:18/36,764 executed pins,three explicit leaf entries.
Profile selection:48/96,187 executed pins,three explicit leaf entries.
Runtime/source getters:228/456,13 executed pins,no seams; ctor stores/table
slots are separately static pins, not a claim those ctor bodies executed here.
Full AI initializer:54 complete rows +3 neighbor-prefix +9 source-prefix
cutoffs =66 rows/132 machines,1979 executed pins,four named leaf entries.
Cutoffs establish retained state, not a native exception or completed startup.
Category0 per-region land/water and separate manager90 lists:36/72,742
executed pins,two allocator entries; real4CF010 region lookup, member water
predicates, sparse Kingdom slots and duplicate prepends, actual zero AI slots
and zero region-player count in these explicit native fixtures. Runtime uses
actual constructor-null Actor154; positive SAIActor ownership remains required.
Ordinary Actor predicates/selected empty-listener branches:708/1416,305 executed
pins,two temporary allocation leaf entries. Positive Wall/controller paths are
excluded. Counts/pins belong to each fixture; they are not a union denominator.

Validation status
All listed native fixtures have passed immutable replay. Current final Rust
navigation tests pass4/4 (three in-place/source-prefix tests plus old wrapper).
Current runtime9/9 passed, including actual25profile/75Ego source projection,
actual tutorial map all3PC and stale Settings refusal, native predicate/list
replays and same-allocation prefix guards (sai-final-nine-tests.log). Baseline
8/8 is retained separately. The current whole-runtime release check passed
(map-list-check.log). findings.json pins exact source, proofs and result logs.
Earlier failed compile/harness attempts are retained as attempts, not native
behavior failures. No original archive/profile/save/release writes were made.
