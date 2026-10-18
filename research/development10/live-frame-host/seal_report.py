"""Seal the bounded author validation report from existing logs and files.
Does not build, modify sources, execute native code or invent pending results.
"""
from pathlib import Path
import hashlib,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
CURRENT=HERE.parent/'current-svar-sync'
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def pinned(paths):return [dict(path=p,sha256=sha(ROOT/p))for p in paths]
def main():
 current_log=CURRENT/'tests-v3.log'
 current_pass=current_log.exists()and 'test result: ok. 3 passed; 0 failed;'in current_log.read_text()
 frame_log=(HERE/'netcode-tests-v1.log').read_text();assert 'test result: ok. 56 passed; 0 failed;'in frame_log
 orders=(HERE/'orders-gate-tests-v2.log').read_text();assert 'test result: ok. 3 passed; 0 failed;'in orders
 replay=json.loads((CURRENT/'native-current-svar.json').read_bytes())
 assert len(replay['rows'])==42 and replay['native_runs']==84 and len(replay['normalization'])==24
 audit=json.loads((HERE/'native-pin-audit-v3.json').read_bytes());assert audit['passed']
 own=['crates/aa-runtime/src/current_svar_sync.rs','crates/aa-runtime/src/current_svar_sync_tests.rs',
      'crates/aa-runtime/src/inactive_svar_dump.rs','crates/aa-runtime/src/frame_variables.rs',
      'crates/aa-netcode/src/local_frame_host.rs','crates/aa-netcode/src/local_frame_task_effects.rs',
      'crates/aa-netcode/src/local_all_tasks_services.rs','crates/aa-netcode/src/local_order_gate.rs',
      'crates/aa-netcode/src/local_node_commands.rs','crates/aa-orders/tests/lazy_gate.rs']
 shared=['crates/aa-orders/src/lib.rs','crates/aa-orders/src/net/admin.rs','crates/aa-netcode/src/local_order_host.rs',
         'crates/aa-netcode/src/local_bootstrap.rs','crates/aa-netcode/src/local_host.rs',
         'crates/aa-netcode/src/local_all_tasks_complete.rs','crates/aa-runtime/src/live_world_frame.rs',
         'crates/aa-runtime/src/initial_world_sync.rs','crates/aa-runtime/src/load_world.rs']
 evidence=['research/development10/live-frame-host/native-frame-variables.json',
           'research/development10/live-frame-host/native-command-queue.json',
           'research/development10/live-frame-host/native-order-gate.json',
           'research/development10/current-svar-sync/native-current-svar.json',
           'research/development10/live-frame-host/native-pin-audit-v3.json',
           'research/development10/current-svar-sync/independent-state-binding/source-review.json',
           'research/skirmish-20261017-current-svars/findings.json',
           'qa-output/development10/runtime46-inactive_svar_dump-current.log',
           'qa-output/development10/runtime46-canonical_svars-tests.log']
 parent_binary=ROOT/'qa-output/development10/runtime-tests46.exe'
 assert sha(parent_binary)=='5479a189fd13aefcec627a84cd7a63c6ddcd2914f9b5d6212b75b1738e2ea56c'
 for p in ['runtime46-inactive_svar_dump-current.log','runtime46-canonical_svars-tests.log']:
  assert 'test result: ok. 3 passed; 0 failed;'in(ROOT/'qa-output/development10'/p).read_text()
 doc=dict(schema=1,status='bounded_current_svar_component_tests_complete_whole_frame_pending'if current_pass else'focused_runtime_tests_pending',
  owner='/root/settlement_completion',owned_source_pins=pinned(own),
  shared_file_context_only_not_exclusive_freeze=pinned(shared),evidence_pins=pinned(evidence),
  validation=dict(netcode_unit_before_later_svar_timing_heartbeat_additions=56,lazy_gate_unit=3,
   actual_frame_variable_source=1,frame_variable_constructor_unit=1,existing_sim_tests=8,current_svar_unit=3 if current_pass else None,
   parent_current_svar_unit=3,parent_canonical_current_source_and_map_acceptance=3,inactive_inventory_compatibility=2,initial_world_sync_compatibility=3,
   whole_svar_native_runs=84,bounded_boolean_normalization_runs=24,
   frame_variable_native_runs=12,command_queue_native_runs=18,
   gate_native_cases=96,bounded_gate_constructor_store_projections=96),
  qualifications=[
   'No genuine StartedMission, activated frame, VWorld completion, or playable battle was produced by this component validation.',
   'Registration reference seals names/types/order, not current values; current data comes from the separate canonical State producer.',
   'Native SVar logger bodies execute disabled; arguments and cutoff prefixes are observed; Boolean normalization is a separate bounded post-allocation prefix.',
   'Current SVar positive SyncSink and actual-map source/moved-Bank acceptance passed in parent cohort46; source-native proof remains separately authored and pinned.',
   'Shared file pins record context and may be superseded by other authorized owners; they do not attribute their whole contents to this author.',
   'Actual factory/callers are trusted; RuntimeBorrow public fields are not hardened against deliberate hostile sibling swapping.',
   'Required services include positive command payload handlers, remote peers, AVI and actual view completion; outer752AE0 duration-history, timing and heartbeat extensions are separately authored and not certified by the earlier56-test checkpoint.',
   'Failed attempts and superseded fixtures remain on disk and are not successful acceptance evidence.'
  ])
 p=HERE/'findings.json';p.write_text(json.dumps(doc,indent=2)+'\n')
 print(json.dumps(dict(status=doc['status'],sha256=sha(p),runtime_tests=current_pass)))
if __name__=='__main__':main()
