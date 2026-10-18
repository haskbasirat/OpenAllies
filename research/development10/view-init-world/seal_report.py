from pathlib import Path
import json,hashlib,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
paths=[
 'crates/aa-runtime/src/view_init_world.rs',
 'crates/aa-runtime/src/view_init_globals.rs',
 'crates/aa-runtime/src/view_terrain_slots.rs',
 'crates/aa-runtime/src/view_trigger_vfx.rs',
 'crates/aa-runtime/src/view_trigger_vfx_collect.rs',
 'crates/aa-runtime/src/view_trigger_vfx_collect_tests.rs',
]
evidence=['native-tail-fields.json','native-trigger-vfx.json','pin-audit.json','native_tail_fields.py','native_trigger_vfx.py','audit_tail.py','isolated_tail_fields.rs','isolated_trigger_vfx.rs','isolated-tail-fields.exe','isolated-trigger-vfx.exe','tail-fields-tests-01.log','trigger-tests-01.log','production-check-04.log','README.md']
assert '3 passed; 0 failed' in(HERE/'tail-fields-tests-01.log').read_text(encoding='utf-8-sig')
assert '2 passed; 0 failed' in(HERE/'trigger-tests-01.log').read_text(encoding='utf-8-sig')
check=(HERE/'production-check-04.log').read_text(encoding='utf-8-sig');assert 'Finished' in check and 'error[' not in check
doc=dict(schema=1,scope='Bounded retained VWorld composition and native leaf acceptance, NOT whole43F0A0/AD/battle readiness.',
 production={p:H(ROOT/p)for p in paths},evidence={p:H(HERE/p)for p in evidence},
 native=dict(paired_rows=93,executions=186,decoded_pins=246,images=['original','v14'],precisions=['Single24','Double53','Extended64']),
 rust=dict(isolated_tests_passed=5,production_typecheck='production-check-04.log; PASS14.17s; existing warnings retained',aggregate_genuine_world_gpu_completed=False),
 shared_extensions=['Inputs helper_globals is a separately approved sixth owner beyond prior five-owner bundle; later camera-render-source extension belongs to Leader.','PresentationParts lends actual creator_extents/camera_target/triggers; no snapshot.','FX author owns same State fx_commands field/getters; no AD/phase mutation.'],
 remaining=['Concrete environment+texture asset owner completion','Actual camera constructor/directory/captured6FF130 owner','Actual terrain+scene GPU integration against this same aggregate','Reached resource40 VFX child owner','Full concrete completion facade before AD may be written'])
data=(json.dumps(doc,indent=2)+'\n').encode();p=HERE/'findings.json'
if '--write'in sys.argv:p.write_bytes(data)
else:assert p.read_bytes()==data
print(hashlib.sha256(data).hexdigest())
