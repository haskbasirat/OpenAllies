"""Seal this bounded task/queue proof without modifying native fixtures."""
from pathlib import Path
import hashlib,json,shutil
here=Path(__file__).resolve().parent;root=here.parents[1]
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
owned=['crates/aa-runtime/src/resource_task.rs','crates/aa-runtime/src/resource_task_tests.rs']
snapshot=here/'source-snapshot';snapshot.mkdir(exist_ok=True)
pins={}
for name in owned:
 p=root/name;dest=snapshot/p.name
 if dest.exists():assert dest.read_bytes()==p.read_bytes()
 else:shutil.copyfile(p,dest)
 pins[name]=sha(p)
for name in ['native_resource_task.py','native_task_wrappers.py','native_queue_signal.py','audit_globals.py','native-resource-task.json','native-task-wrappers.json','native-queue-signal.json','global-direct-audit.json','standalone_task.rs','build_probe.py','resource_objects-production.rs','resource-objects-source-pin.json','normalizer-source-pin.json']:
 pins[name]=sha(here/name)
exe=root/'target/s-borrowed-leader/resource-task-tests.exe';frozen=here/'frozen-resource-task-tests.exe'
if frozen.exists():assert frozen.read_bytes()==exe.read_bytes()
else:shutil.copyfile(exe,frozen)
pins[frozen.name]=sha(frozen)
before=json.loads((here/'probe-before-06.json').read_text(encoding='utf-8-sig'))
after=json.loads((here/'probe-after-06.json').read_text(encoding='utf-8-sig'))
assert before==after
report={
 'scope':'TaskLoad constructor, same task fields/references, queue ordering/removal/repriority, request/run/block coordinator and auto-reset queue notification backend; no decoder/GPU or battle readiness claim.',
 'pins':pins,
 'native':[
  dict(fixture='native-resource-task.json',paired_rows=36,separate_original_v14_executions=72,instruction_pins=557,scope='Real constructor and real queue/list kernels; declared allocation, atomics, mutex, signal and diagnostic transports.'),
  dict(fixture='native-task-wrappers.json',paired_rows=66,separate_original_v14_executions=132,instruction_pins=639,scope='Real request/run/block + Texture3C OR4/OR6. Resource4C has explicit return/required-stop transport; no decoder or GPU proof. Cache and running waiter positive children stop explicitly.'),
  dict(fixture='native-queue-signal.json',paired_rows=6,separate_original_v14_executions=12,instruction_pins=76,scope='Real queue signal constructor/SetEvent. Unnamed auto-reset event is initially clear. Native Signal+4 byte remains poisoned until SetEvent, so it is not represented as constructor zero.')],
 'precision':['Single24','Double53','Extended64'],
 'native_replay':'All three immutable default replays passed; fixture bytes unchanged.',
 'rust':dict(passed=5,failed=0,seconds=0.02,log='rust-tests-06.log',build='build-06.log',source_pins_unchanged=True,host_scope='Exact Task/ExecutionContext sources. Exact ResourceObjects production text with only its unrelated integration-test module declaration excluded; exact pure normalized_key mechanically extracted. No parser/manager/thread success stub.'),
 'production_check':dict(command='cargo check -p aa-runtime --lib --release --locked --offline -j2',target='target/s-borrowed-leader-check',passed=True,seconds=16.51,log='production-check-02.log',scope='Production API/source check, not an actual completed resource request or startup execution.'),
 'current_default_priority':dict(address='B282F0',fresh_pe_seed=1,direct_reads=35,scope='Both PEs pinned; all direct-address references are reads. This is a bounded direct-address audit, not absence of arbitrary alias writers or perpetual native immutability.'),
 'shared_external_sequence':dict(address='B3DA08',fresh_pe_seed=0,api='Application::publish_load_task_sequence(catalog,bank,selected,&Inputs)->ExternalSequence',validation='Inputs same selected Creator/source plus exact source Arc in SAME Application map cache; allocation-only guard has no Find/refcount/stream effects.',status='Runtime bridge compiled and same-counter laboratory test passed. Actual specialized local task glue is owned by Globals and pending its separate freeze/execution. No actual later counter parity is asserted before that integration.'),
 'required_boundaries':[
  'Resource4C source decode/model/texture backend and GPU acknowledgement are independent real services.',
  'Model3C Ni cache child remains required after actual OR6 stores.',
  'Already-running task waiter5B1FD0 and heterogeneous worker/dequeue scheduling remain required.',
  'C++ thrown-exception diagnostic/status3/rethrow5B1B1D..5B1B53 is not emulated by portable required-service Result errors. Earlier task/flag effects are preserved; Rust temporary guard cleanup is separately qualified.',
  'Native final-reference deleting destructor remains a required service; no zero-ref successful retirement is fabricated.'
 ],
 'historical_failures_retained':['build-05.log: peer ResourceObjects added unrelated integration tests to standalone host; fixed host exclusion only.','rust-tests-03.log: test audit incorrectly counted a declared host cache mutation as a CPU write; production unchanged.'],
 'shared_source_qualification':'Application bridge and map-cache guard are additive shared-file edits. Resource facade/Manager implementation is owned and proved separately by State; this report does not claim its whole-file source/native equivalence.'
}
(here/'findings.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(dict(findings_sha256=sha(here/'findings.json'),source=pins[owned[0]],tests=pins[owned[1]],frozen_exe=pins[frozen.name])))
