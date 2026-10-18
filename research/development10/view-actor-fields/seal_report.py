from pathlib import Path
import json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
files=[ROOT/'crates/aa-client/src/view_actor_fields.rs',ROOT/'crates/aa-client/src/view_actor_fields_tests.rs']
files += [HERE/n for n in ['native_actor.py','native-actor-fields.json','audit_actor.py','pin-audit.json','isolated-actor-final.exe','build-final.log','tests-final.log','native-final-replay.log','README.md','runtime-loans-check-01.log']]
out=dict(schema=1,status='accepted bounded VActor453F10 tail; source integration separate',
 boundary='453F43..4541B7 after separately retained455050 base',
 native=dict(paired_rows=99,executions=198,precision=['Single24','Double53','Extended64'],decoded_equal_executed_pins=269,source_cutoffs=23,seams=['455050 separately owned base','6F7810 allocation transport','6F7430 free transport'],actual_children=['source virtual38 4F0500/4C9FA0','57DBE0/45DC90','766DE0/4849F0']),
 rust=dict(exact_source_isolated_tests=2,passed=2,whole_client_check='Not claimed; SDK owns concrete source/Directory integration. No cold client unit link launched.',runtime_shared_loan_check='PASS43.67s: opaque Grid+AC same-owner additions'),
 ownership='BC..154 scalar tail and two sole growable item/list metadata owners; no base copy, count mirror or fake numeric pointer. Unknown fields/padding retained.',
 shared_additions=['live_scene.rs declares newview_actor_fields module after existingview_object_init (approved by SDK owner)',
 'view_init_prefix.rs crate-only disjoint object_init_parts grid+actualAC byte loan',
 'view_init_world.rs opaque ObjectInitParts only aggregate-minted after sameWorld validation; lazyAC getter, actualGrid mutable getter, sameWorld guard; no readiness capability'],
 limitations=['Current B322A4/A8/AC and point globals require real caller owners; module supplies no white/zero defaults',
 'No455050 base,451F90 derivedInit, source binding, Thing+C publication, GPU or view/application readiness acceptance',
 'Finite pitch binary32 spill only, no signalling-NaN hardware claim. Raw scalar copies preserve all bits.',
 'Native pointer/heap ABI, OOM/unwinding and destruction equivalence excluded; Rust growable allocation replaces fixed widths without artificial capacities',
 'Changing source counts in fixture are explicitly external storage interventions; no invented application callback/reentrancy claim'],
 pins={str(p.relative_to(ROOT)).replace('\\','/'):H(p)for p in files})
p=HERE/'findings.json';p.write_text(json.dumps(out,indent=1)+'\n');print(H(p))
