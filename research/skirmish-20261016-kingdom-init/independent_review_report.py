from pathlib import Path
import json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
author=json.loads((HERE/'findings.json').read_text())
for path,expected in author['sha256'].items():assert sha(ROOT/path)==expected,path
replay=json.loads((HERE/'independent-replay.json').read_text())
for path,expected in replay['frozen_source_and_fixture_sha256'].items():assert sha(ROOT/path)==expected,path
text=(HERE/'README.txt').read_text();assert 'growable, not a gameplay cap' in text
doc={'schema':1,'date':'2026-10-16','reviewer':'denizen_actions','decision':'accepted within the declared full control/order coordinator scope','blocking_findings':[],
 'author_findings_sha256':sha(HERE/'findings.json'),'all_author_source_evidence_pins_verified':True,'author_pinned_file_count':len(author['sha256']),
 'production_and_native_fixture_preserved':True,'production_edits_made':False,
 'native_replay':replay,'rust_validation':{'test':'cargo test -p aa-economy --test kingdom_init --release --locked --offline -j4','passed':1,'failed':0,'native_rows_compared':99,'target':'target/s-recruit-backrefs','strict_scoped_clippy':'passed without allowances'},
 'reviewed_contracts':[
  {'area':'full-body boundary','result':'4E5DF0 first calls ActorInit4D7230 with incoming argument; false returns immediately. Diagnostic precedes current unsigned world-version<22 and word184 scheduling gate. Outer LoadThing bit7 remains caller responsibility; required effects have no default success methods.'},
  {'area':'flow count and store order','result':'Signed live GameData+38 checked before loop and after each complete row. Per index native stores1A0 then1AC then1B8 zero; vector suffix and stockpiles survive. Shrinking count midrow completes that row; growing count is observed next row. Rust callback order matches instructions.'},
  {'area':'Team ownership','result':'Same selected Team identity receives head prepend, conditional empty tail initialization and wrapping+68 increment, after the flow phase. Allocation-failure behavior is explicitly outside the claim.'},
  {'area':'definition iteration and null heads','result':'UNIT19 thenTRUCK42; current node identity remains separate from definition. First category null is an explicit native46C8C0 fault boundary, not silently skipped. Later empty categories skip normally. Next-node links read only after restrictions/ownership callbacks.'},
  {'area':'recruit filters and item type','result':'Element mask bit8 gates all pointer reads; primary+2C8 head, secondary+C head, then+17C short-circuit. Present Element with missing+2C8 owner errors even if+17C might exist. Accepted item is plainA29650 with definition+4/available byte+8=1; no RecruitSingle substitution.'},
  {'area':'SAI/world/major iteration','result':'Word190 zero and active SAI gate79E170 (native ret4), then mandatory4F8AF0. Live major+154/+158 null skipping uses4F0680/4F06B0 semantics, observes later count/slot mutation, retains selected other identity through related/228/22C callbacks in native order.'},
  {'area':'partial failure','result':'Ten interrupted callback points preserve completed prefix writes/events and prevent subsequent phases. Two native null-pointer fault boundaries are explicit Rust errors. This does not prove C++ unwind/destructor behavior or allocations failing mid operation.'},
  {'area':'precision','result':'Original/v14 body replayed atPC24/53/64. Body contains integer/pointer operations and positive-zero stores; Rust passes the explicit incoming precision to ActorInit. Descendant numeric behavior is a required host service and is not claimed by coordinator fixtures.'}
 ],
 'resolved_nonblocking_note':'README corrected native43-entry initial temporary-vector capacity: it was already growable, so no removed hard cap is claimed.',
 'not_proven':['No actual runtime Init host/binding installed or accepted here','ActorInit, scheduling, restrictions, recruit ownership, SAI, world and relation effects are recorded required seams, not complete implementations','Allocator/CString/vector service failure and C++ exception unwind/destructor parity','No playable skirmish, world publication, multiplayer, campaign or release success'],
 'review_artifacts':{name:sha(HERE/name) for name in ['independent_replay.py','independent-replay.json','independent-replay.log','independent-rust-test.log','independent-clippy.log']}}
(HERE/'independent-review.json').write_text(json.dumps(doc,indent=2)+'\n')
(HERE/'independent-review.txt').write_text('Accepted within the declared coordinator scope; no blocking findings.\nIndependently replayed198 original/v14 executions (99 rows),401 executed native instructions.\nRust scoped replay1/1 and strict scoped Clippy passed; all7 author evidence pins verified.\nSource and fixtures preserved. Live read/order, flow and stock preservation, Team list/counter,\ncategory/null-head behavior, lazy predicates, major-vector mutation and failure prefixes match.\nNo runtime Init binding, descendant effect completion, allocator/unwind or playable game claim.\nSee independent-review.json for the complete contract audit and hashes.\n')
print(json.dumps({'review_sha256':sha(HERE/'independent-review.json'),'author_pins_verified':len(author['sha256'])}))
