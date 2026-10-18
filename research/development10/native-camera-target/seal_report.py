from pathlib import Path
import json,hashlib,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
source=['crates/aa-runtime/src/native_camera_target.rs','crates/aa-runtime/src/native_camera_target_tests.rs']
files=['native-target.json','pin-audit.json','native_target.py','audit_target.py','make_isolated.py','isolated.rs','isolated_full.rs','isolated-full-camera-v2.exe','isolated-full-tests-02.log','isolated-tests-02.log','native-final-replay.log','production-check-02.log','README.md']
assert '8 passed; 0 failed' in(HERE/'isolated-full-tests-02.log').read_text(encoding='utf-8-sig')
assert '2 passed; 0 failed' in(HERE/'isolated-tests-02.log').read_text(encoding='utf-8-sig')
check=(HERE/'production-check-02.log').read_text(encoding='utf-8-sig');assert 'Finished'in check and 'error['not in check
doc=dict(schema=1,scope='Concrete same-camera6FF130/6FEDB0 finite numeric/geometry producer with required current source/height/Ni children. No completedcamera/directory/VWorld/AD claim.',
 source={p:H(ROOT/p)for p in source},evidence={p:H(HERE/p)for p in files},native=dict(paired_rows=123,executions=246,decoded_pins=588,hardware_fsin_transports=174),
 rust=dict(child_tests=2,current_parent_reset_pose_ni_target_tests=8,production_check='PASS15.20s; warnings retained'),
 shared_parent_extension='Only nested target module, private Links, unbound initializer. Parent revisionArc/unbound_allocation change is the camera author\'s separately owned extension and was included in authoritative8/8 replay.',
 remaining=['Actual completed7016A0 constructor and shape authority','Captured actual camera-directory handle','Concrete current UI/map/World/height services and actual6FE050 owner binding'])
data=(json.dumps(doc,indent=2)+'\n').encode();p=HERE/'findings.json'
if '--write'in sys.argv:p.write_bytes(data)
else:assert p.read_bytes()==data
print(hashlib.sha256(data).hexdigest())
