"""Pin bounded application/menu/status/confirmation evidence, no battle mint."""
from pathlib import Path
import hashlib,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
def pin(p):
 p=ROOT/p;return dict(path=str(p.relative_to(ROOT)).replace('\\','/'),sha256=hashlib.sha256(p.read_bytes()).hexdigest())
owned=[
'crates/aa-runtime/src/application_menu_callbacks.rs',
'crates/aa-netcode/src/local_session_status.rs','crates/aa-netcode/src/local_session_status_core.rs',
'crates/aa-netcode/src/local_selected_map_confirm.rs',
'crates/aa-client/src/battle_menus.rs','crates/aa-client/src/battle_notifications.rs']
shared=[
'crates/aa-runtime/src/application_loading.rs','crates/aa-runtime/src/view_init_prefix.rs',
'crates/aa-runtime/src/view_init_prefix_tests.rs','crates/aa-runtime/src/view_init_world.rs',
'crates/aa-runtime/src/ui_world_init.rs','crates/aa-runtime/src/lib.rs',
'crates/aa-netcode/src/local_bootstrap.rs','crates/aa-netcode/src/local_world_view.rs',
'crates/aa-client/src/battle_controller.rs','crates/aa-client/src/battle_view_startup.rs',
'crates/aa-client/src/battle_ui_bridge.rs','crates/aa-client/src/tgi_frontend.rs',
'crates/aa-client/src/frontend.rs','crates/aa-client/src/main.rs']
fixtures=['native-menu-callbacks.json','native-session-status.json','native-selected-confirm.json','native-ui-argument.json']
artifacts=['native_menus.py','native_status.py','native_selected_confirm.py','native_ui_argument.py',
'kernel-tests.rs','kernel-tests.exe','kernel-tests-final.log','menu-source-tests.rs','menu-source-tests.exe','menu-source-tests-final.log',
'build_b8_tests.py','b8-source-pins.json','b8-tests.exe','b8-tests.log',
'build_selected_tests.py','selected-source-pins.json','selected-kernel-tests.exe','selected-kernel-tests.log',
'native-menus-replay.log','native-status-replay.log','native-confirm-replay.log','native-argument-replay.log',
'check-07.log']
doc=dict(schema=1,scope='Bounded actual FIFO/AppState/menu/SessionStatus/selected-map confirmation composition. No complete VWorld, native task readiness, first frame, or playable battle receipt.',
owned_sources=[pin(p)for p in owned],shared_successor_snapshots=[pin(p)for p in shared],
fixtures=[pin(str((HERE/n).relative_to(ROOT)))for n in fixtures],artifacts=[pin(str((HERE/n).relative_to(ROOT)))for n in artifacts],
native=dict(images=['original AA.exe','v14 AA.exe'],precisions=['Single24','Double53','Extended64'],
menus=dict(pairs=291,runs=582,executed_pins=85,stopped_show_entry='5C0440 is a required child boundary; no native animator acceptance'),
status=dict(pairs=135,runs=270,executed_pins=94,children='Mutex/CString/translation/timer/logger primitives are explicit transports/cutoffs; late failure is not native SEH'),
confirmation=dict(pairs=15,runs=30,executed_pins=90,stopped_entry='68F68D before SetLaunchSettings transport; whole selector dialog and Host not claimed'),
ui_argument=dict(pairs=9,runs=18,executed_pins=132,children='Actual40CF00/5C3000/task ctor/E8RET; camera/VWorld/publication/enqueue remain child seams'),
fresh_readonly_replays='All four producers replay immutable fixtures successfully; stop entries excluded from executed pins'),
rust=dict(exact_menu_status_kernel_tests=3,actual_source_three_menu_allocation_test=1,current_prefix_tests=5,exact_confirmation_kernel_test=1,total=10,
production_check='cargo check -p aa-client --bin AxisAlliesRust --release --locked --offline -j4 --target-dir target/s-settlement-completion: PASS11.82s',
qualification='Small source harnesses use pinned warm numeric/UI/JSON support, no full current client link. Actual LocalBootstrap status owner test saved for parent warm cohort; not claimed executed here.'),
invariants=[
'One retained App10C110 State on same SimulationThread as actual ActivatedSession/ApplicationViews; actual FIFO popped event retained on error with stop latch.',
'Typed weak menu handles name distinct actual TGI allocations. Actual Widget forests move between same Ui and owner; no loading15/17 is minted by paint or show.',
'Session12C status lock is distinct from world lock;148 writes before actual timer/diagnostic,14C only after successful diagnostic. Translation borrows same live TextManager.',
'Ordinary confirmation captures current actual AppState and rejects7; same GameApp mode1/source Bank/Creator/Header90/IDS/options/seed; actual ctor then784130(4,1), outer6 false04. Ordinary local map choice is explicit modern entry precondition, not a guessed dialog12A byte.',
'Notification.word38 explicitly reaches nativeB8; managed World Witness remains separate. Existing genuine7738B0 currently supplies revision value, so previous admitted numeric equality was not a distinct native argument proof.',
'Current product UI/prefix callers use explicit notification argument APIs. Legacy world.value adapters remain documented laboratory compatibility only.',
'PreWorld AD comes from actual retained442C00 State through weak reader, later adopted once. No missing View means false readiness.'
],
deliberate_modern_backend=[
'Actual modern UI presentation installs selected source-bound Widget forest immediately; legacy5C0440 D0 animator ABI is not claimed.',
'Initial real main menu0 enter is explicit; legacy splash/intro/staging navigation is omitted, without changing native Host settings/seed semantics.',
'Separate real Windows QPC timer anchors represent modern globalB396E0 and Node50 timers. No Node cached timer substitution.',
'Frontend GUI remains on its native thread; actual AppState/World stay on retained simulation thread, immutable UI work crosses sanctioned bridge.'
],
remaining_required=[
'No whole native UI/view initialization or runtime GUI/GPU acceptance from this report.',
'Non6 launch kinds and positive flag04 GameApp reset stay required; ordinary selected-map producer now selects actual6 without mode/settings changes.',
'Positive EndSession40CF30 teardown, positive reentrant kind9 AppState transition and other unimplemented notification kinds remain explicit required boundaries.',
'Native B3E1E0 event/log pointer transport is not synthesized by the modern queue.',
'773CE0 source fallback/legacy CRT formatting and native exception behavior beyond admitted current Localization/regular Windows timer are not independently ported by this bounded proof.'
])
(HERE/'findings.json').write_text(json.dumps(doc,indent=2)+'\n')
print(json.dumps(dict(findings_sha256=hashlib.sha256((HERE/'findings.json').read_bytes()).hexdigest(),native_runs=900,rust_tests=10)))
