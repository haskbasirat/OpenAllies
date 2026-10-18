Late ActorInit action refresh (4D7332 ->4D1350)
================================================
The old load helper handled Construction only and stopped when a real farm's
Structure contributor was reached first. The new thin load_actor_action_refresh
wrapper calls the existing full saved_actor_actions dispatcher on the same held
Actor, with its real inventory, component owners, source Bank and live PC.
Root owns the shared ActorInit hook and live-precision forwarding.

Actual capture: tutorial02 farm GTID2696, weurope_farm6x6_01, reached after real
component AfterInit in three separate immediate loads at PC24/53/64. Flags,
Object/zone storage and boxed component addresses stay the same. All source,
state and non-action fields stay unchanged. Current authored farm actions are
zero; no new action is invented to demonstrate the binding.

Native proof: nine actual-input/precision combinations, each executed in
separate original and v14 machines. Actual4D1350, Structure589A60,
Construction565140 bodies execute in native order. Because the actual farm's
old/new masks are both zero, native4D1350 correctly skips4CFCC0 publication.
This fixture does not newly execute changed-mask publication or its effects.
State-table descriptors/masks are projected from the prior native477FC0
registration fixture, which is hash-pinned. 4B2E60, if reached, is a labelled
source-SVar lookup transport using the captured source bool. UI/renderer
positive children remain required entries. The caller instruction window is
also pinned. See native-farm-action-refresh.json for executed/transport pins.

Validation passed:
- actual_farm_late_init_actions_use_structure_then_construction_on_same_storage
  1/1, all three live PCs, 102.37s; actual-capture-absolute.log.
- Fresh immutable native replay: nine rows /18 machines, native-replay.log.
- managed_farm_action_outputs_match_full_native_continuation: 1/1,0.09s,
  rust-native-replay.log, exact retained action-tests.exe.
Tests were built release/locked/offline,-j4 with LTO=false, codegen-units=16.
The shipping release profile was not changed.

Historical failed logs are retained. First actual test wrongly asserted whole
ActorInit completion and stopped at later FX42A590 after action success; that
test assertion was narrowed. A later capture passed all callback assertions
but used a crate-relative output path; exact-binary rerun used an absolute path.
Other failed builds were foreign mid-edit compiler snapshots. No failed log is
used as success evidence.

Limits: this proves the reached action-refresh binding on the actual farm. It
does not prove later ExistingFX/track/attachment children, full ActorInit,
mission start, a playable battle, multiplayer or original-save compatibility.
Existing full action-kernel proofs cover other bounded contributor cases;
they are reused, not newly executed by this fixture.

Replay (from rust-rebuild; set bundled Python dependencies on PYTHONPATH):
  python research/skirmish-20261016-load-actor-action-refresh/native_farm_action_refresh.py
  action-tests.exe load_actor_action_refresh::tests::managed_farm_action_outputs_match_full_native_continuation --exact --nocapture
The actual ignored test needs AA_GAME_ROOT. Leave AA_CAPTURE_ACTOR_ACTION_REFRESH
unset for immutable comparison; set an ABSOLUTE new output path only to capture.
