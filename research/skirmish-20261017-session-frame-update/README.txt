Native recurring Session update776540, 2026-10-17

Production entry:
  aa_netcode::local_bootstrap::session_frame_update::run(players,local)
  The two arguments are direct sibling loans from the retained LocalBootstrap.
  Owner identity is checked; no new Session, Node, task, clock, player list or
  synchronization log is made. The frame Host owner wires this entry separately.

Native order and admitted behavior:
  776540 ->775DB0(block0) ->76E3D0 ->76C4B0.
  NULL task returns from775DB0 only, never from the whole Session update.
  Unfinished positive task uses the SAME Arc/Mutex task status projection and
  the shared existing775DB0 predicate. Rust worker errors stay errors, rather
  than becoming a fabricated native failure-status3 or completed task.
  76E3D0 reads host identity before signed Session11C. A nonhost or11C<=0
  returns to the coordinator; a positive countdown stops at actual76C960.
  76C4B0 reads16C, host identity, Node2E0, then E4 in that order. A positive
  migration from state1 stores16C=true BEFORE stopping at actual76B570.
  The current active-game/countdown0 path can therefore return through both
  real guards, including the2E0 getter; task absence alone never permits success.

Canonical new field:
  LocalHost::Node retains raw u32 takeover_2e0; LocalClientBorrow lends it by
  reference. Actual75CEB2 clears EBX and75D1AE stores it to+2E0. The proof runs
  those two integer intervals and pins the surrounding constructor; it does
  NOT execute the entire75CE80 constructor. Intervening native calls obey the
  existing callee-saved EBX contract. No Reset write is invented.75C6C0 is the
  Node destructor, not Reset. Current concrete heartbeat/remote migration
  callbacks remain required and must eventually update this SAME retained
  word (native75955D and other positive writers), not a copied heartbeat DTO.

Proof:
  native_session_frame_update.py default is immutable replay. --write is only
  the explicit author regeneration option. No game files or profiles change.
  438 original/v14 executions across Single24/Double53/Extended64, plus12
  constructor integer-interval executions.97 distinct genuine instructions
  have identical original/v14 bytes. All case outputs, actual field-read
  footprints and native call order agree across images. Zero successful child
  seams: completed-task404F60, countdown76C960, migration76B570 STOP before
  their bodies. Stop entries are not counted as executed instructions.
  Fixture status0/1/2/3/MAX rows are explicitly typed raw task-status laboratory
  projections. They cannot mint a Rust Task/StartedMission/activation result.
  NULL rows use the public same-owner run entry in the Rust test; raw status
  rows separately replay the shared predicate then same-owner suffix.

Validation checkpoint:
  Native immutable replay passed438+12 executions/97 pins.
  aa-netcode production check passed35.77s.
  aa-netcode --lib --tests typecheck passed27.04s; final shared-helper source
  recheck passed6.85s. Final count/footprint-guard typecheck passed27.66s;
  the intervening failed Host RefCell/coercion snapshot is retained separately.
  These are compile checks, NOT executed Rust unit tests.
  The two new same-owner unit tests await the parent aa-netcode test cohort.
  No full runtime unit-test link or actual GUI battle was run by this author.

Remaining required services:
  Positive completed-task log/lock/worker7757F0+7738B0 callback bridge.
  Positive76C960 readiness and subsequent74B220 backup, real Session110 timer,
  current LaunchCountdownTime, countdown diagnostics/request emission.
  Positive76B570 migration request/real order-admin callback.
  Actual recurring transport/heartbeat outside this Session child.
  This is bounded recurring callback progress, not completed multiplayer,
  campaign, combat, whole application startup or a playable-battle claim.
