from pathlib import Path
import json,hashlib
HERE=Path(__file__).resolve().parent;BASE=HERE.parent;ROOT=BASE.parents[2]
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
f=json.loads((BASE/'findings.json').read_text());assert H(BASE/'findings.json')=='8b3d3a65c0dbcf6cf56bd2e922faa1692584b5295210336a1e1dd5127d57ed0e'
assert all(H(ROOT/p)==digest for p,digest in f['source_pins'].items())
paths=[HERE/n for n in ['review_capture.py','capture-method-review.json','replay_shared.py','shared-native-replay.json','shared-native-01.log','shared_isolated.rs','independent-shared.exe','shared-build-01.log','shared-tests-01.log']]
paths.append(ROOT/'crates/aa-runtime/src/native_render_properties.rs')
out={'schema':1,'review':'accepted bounded shared-default initialization','author_findings_sha256':H(BASE/'findings.json'),'author_pins_unchanged':True,
 'reviewer_executions':{'native_original_v14_all_pc':48,'paired_rows':24,'decoded_equal_executed_pins':628,'fresh_Windows_debuggee_launches':0,'exact_production_module_Rust_tests':5,'passed':5},
 'static_findings':[
 'The guard is stored before constructor traversal; refusal retains prior slots and the failed same allocation. Repeat guard does not grant completed property-set access.',
 'The sole Factory owns material ID progression. Each default and each895550 set retains the same property Arc; no property value reconstruction.',
 'Frozen embedded hardware reference binds both capture digests and bounded initial Fog seed; capture method is independently checked in the separate report.',
 'ProcessGlobals is a retained current initial owner, not evidence that native globals stay immutable after8C3F20. Any later proved native Fog writer must mutate this same owner before later constructors use it. The present module supplies no such later writer and its accepted prefix does not reach one.',
 'Portable Rust allocations/reference lifetimes deliberately replace native heap/refcount/vtable ABI and B45F14 counter; no ABI accounting claim.'],
 'limitations':[
 'Observation ends before881600 at8C3F20; remainder Ni init and later application execution not reviewed.',
 'No wholeCamera/renderer/AD/view/application/battle readiness claim.',
 'Future caller must preserve one application/process SharedSet; independent fresh instances are independent application models, not interchangeable material counters.',
 'Factory property proof has its own author coverage; this independent replay certifies the shared initializer and executes its real property children, plus reruns five current property/shared Rust tests.'],
 'source_pins':f['source_pins'],'review_evidence':{str(p.relative_to(ROOT)).replace('\\','/'):H(p)for p in paths}}
p=HERE/'shared-review.json';p.write_text(json.dumps(out,indent=1)+'\n');print(H(p))
