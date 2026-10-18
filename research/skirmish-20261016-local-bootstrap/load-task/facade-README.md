# Concrete local skirmish startup facade

Production entry: aa_netcode::local_skirmish_startup::Startup. It retains the
same LocalBootstrap, Notifications, selected SourceResource allocation and
actual queued Session124 task. The client supplies genuine Windows timer,
entropy, OS IDs, current application/scoped SAI and source-created services.
The facade owns its concrete diagnostic and true launch-dump adapter; callers
cannot inject a successful dump/gameplay callback to mint a completed task.

construct binds the native Sim/creator constructors once. host_and_launch
executes actual Host reset, local player/self-host, selected fields/settings,
7791F0 refresh, explicit user seed,776B20 finalization and77A950/77A750. A
failure leaves the same partially mutated Host inside Startup. Source profile,
GameSpy and lazy synchronization flags are sibling loans of the same SAI owner.

The real UI/controller entry is host_and_launch_current_text. Both forms use
one common algorithm; the older explicit-Localization form remains available
for diagnostic fixtures. Current-text mode borrows the same Catalog TextManager
only around first-player construction and implicit-AI view/name construction,
then releases it before selected-file/ScenarioFind can replace MAPSTRINGS.
There is no cloned current table, take/restore workaround or long Text RefCell
loan across77A950. Production compilation and actual09 main launch passed.
An isolated portable pointer/lifetime/error test is saved for final cohort
regression; no native gameplay constructor is inferred from its empty fixtures.

Source setup now calls replacement_flags_source::complete_source_suffix after
successful CV computation and before ResourceCatalog/SourceTransfer in both
the client controller and strict startup test. The same Bank retains the
already proved unchanged Placeholder fixed-point projection. This binds the
real post-CV4BC7F0 suffix that the earlier cold source omitted; it does not
certify unrelated source passes. A new private cold-transfer regression checks
that the first immutable placement read needs no diagnostic prewarm. That test
belongs to the parent's next shared library cohort; it is not yet accepted in
this document's checkpoint.

run_queued_resource_task consumes the completed source transfer (same Bank,
AbilityArena and caches) and actual scoped lifecycle. It starts a real scoped
OS worker, performs genuine OnCreated/Run identity binding, immediate Load,
PostLoad, finish_load, resource completion and StartMission. The exact joined
CompletedWorker<StartedMission> must match the queued task's Creator/source/mode
before SuccessfulTask can be returned.7757F0/7738B0/application activation are
subsequent mandatory steps, not part of this return type.

WorkerFailure retains returned Immediate Attempt, PostLoad/Tail Staged,
Resource/Mission LoadedMap, or failed SourceData owners when the lower-level
API returns them. Existing bare-error setup/loader/staging APIs can consume
owners without returning them; LowerLevelDidNotReturnGraph identifies those
specific loader/staging gaps. No universal exception/owner-retention claim is
made. A Rust refusal does not fabricate native failed-task status3 or clear124.

The ignored real-source acceptance test is
crates/aa-runtime/tests/playable_battle_startup.rs. It uses this facade and one
WorldReadyServices::Application (same GameApp/depot/process/glyph/film owners),
with the sealed actor-direction startup capability passed unchanged to the
worker. Reports are created with create_new; prior attempts are never replaced.

Actual Firefight checkpoints, all strict tests intentionally FAIL at a missing
reached service until every native gate is completed:
- attempt01: before this facade, true enabled SyncA dump needed a thread gate.
- attempt02: actual retained simulation-thread identity fixed that gate;
  source Header name lookup exposed omitted source MAPSTRINGS producer.
- attempt03: real native Host/main launch PASSED: E4=2,F8=12,queued124=0.
  Main17376/simulation30156/resource42656 were distinct actual OS identities.
  The same worker stopped at first Team18048 because a second map-text append
  duplicated team_0. Actual Immediate Attempt graph/report was retained.
  Parent/Leader then bound the native second clear/replacement, never blanket
  deduplication. Main launch completion alone is NOT a playable battle.
- attempt04: actual Host/main launch PASSED. All two Team and seven Kingdom
  immediate callbacks completed, with no lifecycle omissions. ConfigureGWorld
  stopped at required4F53D0 destruction of actual queued Kingdom1677 before
  final masks. This was a strict failure, not a completed task/battle.
- attempts05/06: no runtime result; respectively the new QA snapshot initially
  tried to format Queues without Debug (fixed to its actual snapshot API),
  then a peer's in-flight TaskSchema visibility change blocked compilation.
  Both failed logs remain separate from actual-run evidence.
- attempt07: same real Host/main launch and nine immediate callbacks passed;
  the worker again stopped at actual queued Kingdom1677 destruction (98.53s).
  Before dropping the stopped Attempt, the test captured its actual owned
  Kingdom/Team/World/SAI/spatial/scheduler graph in removal-owned.json and
  removal-prefix.json. The latter is a debug diagnostic, not a reconstruction
  or another runtime owner. These immutable reports are under
  research/development10/playable-startup/attempt07.*.json.
- attempt08: compile-only failure in an in-flight recurring-frame adapter;
  no runtime reports or success claim. Its separate log is retained.
- attempt09: current TextManager entry and UI PC24 main launch PASSED; source
  provenance remains the verified receipt and resource worker PC53. A newly
  added lifecycle variable exposed an earlier strict gate:
  SupplyProvideUnderSiege SVar override order is unbound. Worker stopped in
  5.48s before World/immediate callbacks, so this run does not yet certify the
  subsequently ported Kingdom-removal path. Exact immutable main/worker
  reports are under attempt09.*.json; earlier attempts are unchanged.
- attempt10: current TextManager/UI PC24 main launch PASSED. The corrected
  registered Supply SVar override passed; ConfigureGWorld completed, including
  actual removal of unused Kingdoms, and OBJECTIVES completed. All nine Team/
  Kingdom immediate callbacks and DynamicThing1682 completed. DynamicThing1683
  then stopped after Object Load because its saved owner1677 had been removed:
  `Actor owner 0x0000068d is not currently registered (native4D8DAA load error)`.
  The same stopped Immediate graph was returned by WorkerFailure. This is a
  strict failure, not StartedMission or activation. Runtime was55.84s; immutable
  reports are attempt10.main-launch.json and attempt10.worker.json. Read-only
  native inspection identifies4D8DAA ->5A0910 as a LoadCancelException caught
  by4FAE80; the Rust plain error currently bypasses that established catch.
  Any corrective catch/classification work needs its own native-backed test.
- attempt11: the exact missing-owner error now enters the established native
  cancel catch. The actual map skipped start_loc1683..1687 for the removed
  Kingdoms1677..1681, then continued to DynamicThing1688 Init. Configuration
  and OBJECTIVES completed;10 immediate callbacks completed. New reached
  refusal: `actual Bank has no completed Placeholder fixed-point storage`
  inside actor creation. The same Immediate graph was returned. This75.39s
  run used the frozen pre-canonical-SVar-transition binary
  5ab11662aeedbaf18cf41e60632bd8e29e382b5061c4299699975f6cc5c8f315;
  it does not certify the newer in-flight shared variable producer. Native
  owner/catch proof60rows plus Rust2/2 and full15 library tests are under
  research/skirmish-20261017-actor-owner-cancel. Reports remain immutable
  attempt11.main-launch.json/attempt11.worker.json. No StartedMission or
  application activation was returned.
- attempt12: post-CV source suffix passed, fixing the cold moved Bank. The
  worker completed566 immediate callbacks and reached City16098 at dynamic
  ordinal567. It stopped at required563FE0/source5B4/zone8, with the same
  Immediate graph returned (93.36s). This was the first run of the new canonical
  SVar factory; full native source parity is a separate proof, not this test's
  claim. Ten unused start locations were correctly canceled.
- attempt13: the new shared CityAfterInit kernel and actual City/zone adapter
  passed the previous boundary and entered real Zone8 movement. It stopped at
  `zone same spatial-index tile flags owner` (76.67s); the same enclosing City
  Init remained incomplete, so completion count stayed566. The strict test
  failed and retained the Immediate graph. New City native78-row/all-PC proof,
  baseline component regression and production checks are documented under
  research/skirmish-20261017-city-after-init. Actual14 tests the newly bound
  same-index flags20 read; no StartedMission/activation has been returned.

The test invokes Host on its application/main thread with a real separate
retained simulation thread, so the logger's mismatch branch is proved for
that execution. The original UI's complete upstream Host execution-role
schedule has not been certified; the positive equality branch now borrows
actual SAI+5C/+5D, not fixed flags. Full native recurring simulation/network
thread-loop behavior remains separate work.

Attempts through07 used explicit PC53 for their diagnostic main path;09 uses
PC24 to match the actual UI controller role, with PC53 on the resource worker.
This does not change the independently retained definition-time source PC.

The facade's Notifications owner uses the narrow native interactive-post-init
Application+30 kind2 projection (4196EC stores1,41A4EA stores2), not the older
diagnostic rawkind0 input. Its producer24-row original/v14/all-PC instruction
slice fixture is in research/skirmish-20261017-simulation-execution. This field
projection does not certify completion of the whole original Application Init.

Later actual BeginPlaying/film/AI callbacks now derive a sealed weak World
identity from the same current WorldFrameInputs and validate it against the
retained Sim alias. They do not infer globalWorld absence from Sim-null: native
4F7D00 publishes B36534 before the outer76AA90 Sim+3C store. An old generic
Sim-only roster refuses at the reached Kingdom getter until a true global
owner is supplied. These later callback fixes do not turn attempt07's queued
Kingdom-removal failure into a successful worker or battle.

The only manifest change was parent-approved internal aa-rules/aa-mapworld
path dependencies in aa-netcode, with those same packages added to its lock
list by offline Cargo metadata; no external package/version change.
All agent builds use explicit release lto=false/codegen-units=16 overrides
and target/s-local-bootstrap, not the final thin-LTO release build.
