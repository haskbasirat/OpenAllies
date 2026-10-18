from pathlib import Path
import json,hashlib,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
source=['crates/aa-runtime/src/native_camera_base.rs','crates/aa-runtime/src/native_camera_base_tests.rs']
files=['native-base.json','pin-audit.json','native_base.py','audit_base.py','make_isolated.py','isolated.rs','isolated-base-final.exe','isolated-tests-final.log','native-final-replay.log','production-check-01.log','README.md']
assert '10 passed; 0 failed'in(HERE/'isolated-tests-final.log').read_text(encoding='utf-8-sig')
check=(HERE/'production-check-01.log').read_text(encoding='utf-8-sig');assert 'Finished'in check and 'error['not in check
doc=dict(schema=1,scope='Same-camera874B40 numeric/known-byte/list owner producer; no nativeABI/globalNiinstancecount/whole7016A0/directory/renderer/VWorld/AD readiness claim.',
 source={p:H(ROOT/p)for p in source},evidence={p:H(HERE/p)for p in files},native=dict(paired_rows=99,executions=198,decoded_pins=822,required_source_read_prefixes=27),
 rust=dict(base_tests=2,exact_parent_reset_pose_ni_target_base_tests=10,production_check='PASS15.88s; existing warnings retained'),
 shared_parent_extension='Private base Fields+unbound initializer; word/put route owned list metadata without scalar mirrors. revisionArc/targetLinks and other authors methods preserved.',
 remaining=['Actual7016A0 debug/root/property/render-list constructor','Actual camera directory/captured handle','Positive current reference/parent/controller mutation if reached','Actual GPU/view/UI completion'])
data=(json.dumps(doc,indent=2)+'\n').encode();p=HERE/'findings.json'
if '--write'in sys.argv:p.write_bytes(data)
else:assert p.read_bytes()==data
print(hashlib.sha256(data).hexdigest())
