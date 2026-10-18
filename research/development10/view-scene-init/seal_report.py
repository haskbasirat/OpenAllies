from pathlib import Path
import hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
paths=[ROOT/'crates/aa-runtime/src/view_scene_init.rs',ROOT/'crates/aa-runtime/src/view_scene_init_tests.rs',
 HERE/'native_scene.py',HERE/'native-scene.json',HERE/'audit_scene.py',HERE/'pin-audit.json',HERE/'isolated.rs',HERE/'world_revision_kernel.rs',
 HERE/'isolated-tests.exe',HERE/'isolated-tests-07.log',HERE/'production-check-03.log',HERE/'README.md']
pins={str(p.relative_to(ROOT)).replace('\\','/'):hashlib.sha256(p.read_bytes()).hexdigest()for p in paths}
fixture=json.loads((HERE/'native-scene.json').read_text())
assert 'test result: ok. 3 passed;'in (HERE/'isolated-tests-07.log').read_text(encoding='utf-16'if(HERE/'isolated-tests-07.log').read_bytes().startswith(b'\xff\xfe')else'utf-8-sig')
doc=dict(schema=1,status='bounded scene CPU implementation validated; real camera/backend/full-view readiness remain separate',
 scope='43F948..43FCA4; five442C00 constructor fields; actualNi transforms/light normalization',
 validation=dict(native_executions=282,instruction_pins=len(fixture['pins']),rust_tests_passed=3,rust_scene_rows=39,rust_numeric_rows=72,
 production_check='cargo check -p aa-runtime --lib --release --locked --offline -j4 final production-check-03.log; final source exact-source Rust3/3 replay passed'),
 required=['real current7010D0 camera reset and164 node owner','actualsameSceneRenderer backend binding','remainingterrain/finalization/resource/UI completion beforeAD/74FF50','native teardown for reinitialization'],
 limits=['finite successful-allocation semantic domain','noSDK ABI/debug reference-count or pool-thread lifecycle parity','noapplicationstartup or activebattle certification','12native block cutoffs notRust replayed'],
 pins=pins)
data=(json.dumps(doc,indent=2)+'\n').encode();out=HERE/'findings.json'
if '--write'in sys.argv:out.write_bytes(data)
else:assert out.read_bytes()==data
print(json.dumps(dict(sha256=hashlib.sha256(data).hexdigest(),pins=len(pins))))
