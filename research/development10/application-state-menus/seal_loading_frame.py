"""Seal the bounded Loading/Game/real recurring frame composition successor."""
from pathlib import Path
import hashlib,json,shutil

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
def pin(p):
    path=ROOT/p
    return dict(path=str(path.relative_to(ROOT)).replace('\\','/'),sha256=hashlib.sha256(path.read_bytes()).hexdigest())
owned=[
 'crates/aa-runtime/src/application_menu_callbacks.rs',
 'crates/aa-netcode/src/local_live_frame.rs',
 'crates/aa-client/src/battle_menus.rs',
 'crates/aa-client/src/battle_notifications.rs',
]
shared=[
 'crates/aa-runtime/src/application_loading.rs',
 'crates/aa-runtime/src/application_state.rs',
 'crates/aa-netcode/src/local_world_view.rs',
 'crates/aa-netcode/src/local_frame_host.rs',
 'crates/aa-netcode/src/local_frame_task_effects.rs',
 'crates/aa-netcode/src/local_game_data.rs',
 'crates/aa-netcode/src/local_order_host.rs',
 'crates/aa-orders/src/payload/mod.rs',
 'crates/aa-client/src/battle_controller.rs',
 'crates/aa-client/src/battle_ui_bridge.rs',
 'crates/aa-client/src/frontend.rs',
 'crates/aa-client/src/tgi_frontend.rs',
]
snap=HERE/'loading-frame-source-v1'
snap.mkdir(exist_ok=True)
for p in owned+shared:
    target=snap/Path(p).name
    if target.exists():assert target.read_bytes()==(ROOT/p).read_bytes(),str(target)
    else:shutil.copyfile(ROOT/p,target)
artifacts=[
 'native_loading_return.py','native-loading-return.json','loading-return-replay.log',
 'audit_loading_return_source.py','loading-return-source-audit.json','loading-return-source-audit-replay.log',
 'loading-successor-tests.rs','loading-successor-tests.exe','loading-successor-tests-build.log','loading-successor-tests.log',
 'live-frame-check-02.log','live-client-frame-check-03.log','live-client-frame-check-05.log',
 'node4-disassembly.txt','node4-callback-disassembly.txt','loading-screen-tick-disassembly.txt',
 'user-ready-xrefs.txt','user-ready-disassembly.txt',
]
native=json.loads((HERE/'native-loading-return.json').read_text())
assert len(native['rows'])==72 and len(native['pins'])==139
assert '5 passed; 0 failed' in (HERE/'loading-successor-tests.log').read_text()
assert 'in 39.87s' in (HERE/'live-client-frame-check-05.log').read_text()
doc=dict(
 schema=1,
 scope='Concrete successor for Loading kind9 -> same AppState40CE10 -> actual node4 source Game forest, genuine UserReady input/prompt, and product recurring same-thread native Frame.run. This report proves bounded kernels and source composition, not observed GUI/frame/unfreeze or playable battle completion.',
 predecessor=pin(str((HERE/'findings.json').relative_to(ROOT))),
 owned_sources=[pin(p)for p in owned],
 shared_successor_snapshots=[pin(p)for p in shared],
 snapshot_qualification='Shared snapshots are read-only context pins at sealing time, not ownership or complete module acceptance. Other agents own CellWorld/GameData, native task protocol, source memory/reveal, and backend stages. Source/library checks refer to their logged compiler windows.',
 artifacts=[pin(str((HERE/p).relative_to(ROOT)))for p in artifacts],
 native=dict(pairs=72,runs=144,executed_instruction_pins=139,
   images=['original','v14'],precisions=['Single24','Double53','Extended64'],
   cases=dict(game_constructor=3,user_ready=12,loading_to_game_transition=9,ready_prompt=48),
   executed_scope='Actual608C60/40CA10,40CE10/40CAA0/608C80/40CA80,6E80F0/40D970, and6E8C00 prompt prefix through6E8CA9.',
   children='Diagnostics,5C0440 menu show,5C06B0 interface update,CString773CE0 status and SetVisible are explicit child transports/cutoffs.6E8CA9 is observed stop entry, excluded from executed pins; subsequent76E4C0 display tail is not executed.',
   static_audit='Eleven original/v14 byte windows plus actual node4 vtable and UserReady/app_status_finished literals. No extra execution count; node4 tick/leave/menu-finished trivial bodies are static evidence only.',
   replay='Fresh immutable native and static source audits passed; original fixtures are preserved.'),
 rust=dict(exact_source_tests=5,loading_rows=1320,application_state_rows=198,ready_prompt_rows=48,
   extra_tests=['Same actual AppState reentrant setter','Captured return4 immediately reborrows same Loading for OnLeave then Game OnEnter'],
   check='Final cargo check -p aa-client --bin AxisAlliesRust --release --locked --offline -j4 --target-dir target/s-settlement-completion PASS39.87s.',
   current_source_test='Pure test binary rebuilt from exact current production source and passed5/5 in0.01s; warm numeric/JSON support only, no full client link.',
   pending='Saved ignored actual-four-menu/ReadyButton regression awaits parent warm client binary. Historical old three-menu test is not a current four-menu acceptance.'),
 invariants=[
  'Loading prefix returns only the native captured+C scalar after actual17 callback/clear. Product immediately invokes SAME retained AppState setter before diagnostics/observers, allowing real same-Loading40CAA0 without aliasing.',
  'Actual node4 constructor608C60 binds distinct B3974C Game menu. Existing source UI/Game/game_interface.tgi:Game is loaded on same Ui; no PlaceholderWorld or fabricated live HUD children are used.',
  'Ordinary return4/kind1/single-player starts16false. Actual ReadyButton is shown only after lazy hidden/17/16 checks and real773CE0 finished status. Explicit source UserReady command6E80F0 writes SAME16; neither action certifies AD/17 or unfreezes Sim.',
  'Each retained SimulationThread job drains SAME actual FIFO, ticks Loading, then calls ONE actual Frame.run. Runtime keeps SAME ClockedState/World/Actors/queues/RNG and caller-lent SAME LocalBootstrap/Sim/Admin/clock/SyncA/SyncS/FrameControl.',
  'Recurring source calls actual session_frame_update, retained timing/heartbeat, same Frame task-effects304/315/775BF0 and canonical SVar inventory. No laboratory Driver/Gate/Timeline is reconstructed.',
  'Native GameData Current resolves reached ordinary references from SAME live owner. Unsupported domains return schema-source errors via fallible normalization; real NULL is not substituted for missing services.',
  'Actual music callback crosses sanctioned UI bridge and returns real Audio.play_music acceptance/decline for the exact immutable source ID. Audio audibility is not certified.',
  'UserReady jobs retain their real completion ticket/error and run on same simulation queue. Errors latch application/engine while retaining native prefixes; no direct Sim29 write or readiness setter.'
 ],
 deliberate_modern_backend=[
  'Actual UI source forests use immediate modern presentation instead of native5C0440 animation ABI.',
  'Source Game root is real, but code-built live HUD children and their gameplay callbacks remain separate required work.',
  'UI owns Widget forests/audio/GPU; same AppState/World/session stay on retained simulation thread. Only immutable request packets and opaque owner handles cross the sanctioned bridge.',
  'Product uses configured startup precisionDouble53, backed by existing actual startup role evidence; source Bank precision remains independent.',
  'Growable Rust owner/diagnostic collections have no new artificial memory, VRAM, bandwidth or reference-list ceilings.'
 ],
 remaining_required=[
  'No genuine complete StartedMission -> VWorld -> Loading -> first live frame/unfreeze has executed for this successor; current natural worker stops at stored Actor64 reveal before activation.',
  'Actual GUI Ready click, four source-menu runtime allocation test and real recurring Frame.run execution await the genuine current World/view path.',
  'Positive gameplay validity/decree callbacks, positive Node118/14C command pump, AVI source, film restart/stop and backup presentation remain precise required children. Builder owns ordinary gameplay/control successor.',
  'Positive end-session teardown, other launch/state/menu callback kinds and full legacy HUD are outside this bounded integration.',
  'Existing protocol kernels own readiness and unfreeze; neither Game menu show, framebuffer upload, paint nor source-only proof mints that readiness.'
 ])
out=HERE/'loading-frame-findings.json'
out.write_text(json.dumps(doc,indent=2)+'\n')
print(json.dumps(dict(report=str(out.relative_to(ROOT)).replace('\\','/'),sha256=hashlib.sha256(out.read_bytes()).hexdigest(),native_runs=144,exact_source_tests=5)))
