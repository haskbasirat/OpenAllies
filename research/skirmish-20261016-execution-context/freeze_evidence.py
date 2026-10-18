"""Seal author evidence after immutable native replay and Windows Rust tests."""
from pathlib import Path
import json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
files=[
 'crates/aa-runtime/src/execution_context.rs',
 'crates/aa-runtime/src/load_creation_fx.rs',
 'crates/aa-runtime/tests/execution_context.rs',
 'crates/aa-runtime/tests/resource_worker.rs',
]
files += [str(p.relative_to(ROOT)).replace('\\','/') for p in HERE.iterdir()
          if p.name in ['native_execution_context.py','native_async_worker.py','native-execution-context.json','native-async-worker.json','README.md','author-worker-rust-replay-final.log']]
hashes={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest()for p in files}
fixtures=[json.loads((HERE/p).read_text())for p in ['native-execution-context.json','native-async-worker.json']]
log=(HERE/'author-worker-rust-replay-final.log').read_text()
assert '1 passed; 0 failed' in log and '3 passed; 0 failed' in log
pins={p['va']:p['bytes']for f in fixtures for p in f['instructions']}
doc=dict(schema=1,status='author checks passed; independent review pending',
 scope='actual captured application ID and real ASYNC single-job driver; bounded native caller/constructor fragments and required OS/lock/base/wait seams',
 native=dict(paired_rows=sum(len(f['rows'])for f in fixtures),executions=sum(f['native_executions']for f in fixtures),unique_executed_instruction_pins=len(pins),images=['original','v14'],precisions=['Single24','Double53','Extended64']),
 rust=dict(captured_identity_tests=1,actual_windows_worker_tests=3,failed=0),
 created_publication=dict(api='publish(&Catalog,&mut Record,&mut dyn ExecutionIds,&mut State<Command>,flag:u8)',production_check='passed',scope='root-owned sameManager/heldRecord integration follows; this author seal does not claim actual saved Created events were published',native_dependencies='frozen skirmish-20261016-fx-graph compilation/4296B0 command evidence plus existing constructor-vtable/currentE8 getter proofs'),
 limitations=['single-job Rust worker; not full recurrent TGC_Thread signals/manager lifecycle',
 'native emulator OS inputs are explicit laboratory answers, not actual native Windows execution',
 'actual Windows Rust tests call real Kernel32 IDs; no dynamic original-game worker CW measurement',
 'no precision choice from thread name or role',
 'Created true-application-thread timestamp still requires actual View+8C; view delivery/destruction remain explicit required services',
 'gameplay/map completion acceptance remains root-owned'],hashes=hashes)
(HERE/'findings.json').write_text(json.dumps(doc,indent=2)+'\n')
print(json.dumps(dict(native=doc['native'],rust=doc['rust'],findings_sha256=hashlib.sha256((HERE/'findings.json').read_bytes()).hexdigest())))
