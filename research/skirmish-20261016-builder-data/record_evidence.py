"""Pin bounded Builder source/ordering proof without changing production."""
from pathlib import Path
import hashlib
import json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
synthetic=json.loads((HERE/'native-builder-data.json').read_text())
real=json.loads((HERE/'native-real-builder-data.json').read_text())
capture=json.loads((HERE/'actual-source-inputs.json').read_text())
log=(HERE/'replay-tests.log').read_text();assert '5 passed; 0 failed' in log
inputs=[ROOT/'crates/aa-rules/src/native_builder_data.rs',ROOT/'crates/aa-rules/tests/native_builder_data.rs',
        HERE/'native_builder_data.py',HERE/'native-builder-data.json',HERE/'native-real-builder-data.json',
        HERE/'actual-source-inputs.json',HERE/'capture-tests.log',HERE/'replay-tests.log',
        HERE/'capture-list-declaration-assumption.log',HERE/'capture-template-binding-assumption.log',HERE/'README.txt',Path(__file__)]
out={'schema':1,'feature':'current source Builder ActorData+504/+508 canonical arena',
     'original_sha256':synthetic['original_sha256'],'v14_sha256':synthetic['v14_sha256'],
     'addresses':['00467F08..00468007 Builder reference creation','00473AD0 deferred prepend','00475230 deferred dispatch',
                  '00461820 Builder thunk','004604C0 target check and prepend','004BDBA0 typed ActorData check'],
     'native_generated_cases':len(synthetic['cases']),'native_real_source_cases':len(real['cases']),
     'native_typed_actor_probes':len(synthetic['typed_actor_probes']),
     'precision_modes':['Single24','Double53','Extended64'],
     'rust_tests':{'passed':5,'failed':0,'mode':'release/locked/offline -j4 --include-ignored'},
     'real_source':{'canonical_actor_data':len(capture['input']['actors']),
                    'standard_actor_data':1028,'derived_random_actor_data':56,
                    'builder_components':sum(a['builder_present'] for a in capture['input']['actors']),
                    'nonempty_lists':sum(bool(a['targets']) for a in capture['input']['actors']),
                    'references':len(capture['output']['order'])},
     'fixed_review_blocker':'RandomActorData19 inherits actual ActorData constructor/load and IsActorData; now included and tested.',
     'allocator':'explicit successful fixed allocator; arbitrary reentry/exceptions and failure not claimed',
     'partial_failures':'unknown/noBody checks precede allocation/stores; all earlier callback nodes/head/tail retained; later callbacks not executed',
     'identity':'canonical owners by registered source Provenance; targets current final Find once; canonical getters never Translate again',
     'preservation':['No parser or global loader changes','No manifest/dependency changes','No original files or releases changed','No SDK implementation copied'],
     'limitations':['Builder-only deferred subsequence conditional on relevant LoadData and unrelated deferred success/no Builder-visible mutation',
                    'Does not certify complete global475230 or reconstruct historical partial source-manager states',
                    'Callback lookup4BDBA0 supplied as final typed seam; separate18probes execute actual4BDBA0 and native vtable predicates',
                    'Constructor zero stores/loader creation/RandomActor inheritance byte pinned; full constructors/LoadData not reexecuted',
                    'Initial synthetic fixture declaration/binding failures retained; final five tests pass with assertions intact',
                    'Runtime4D4CF0 integration, complete Init/world publication and gameplay remain root-owned work'],
     'inputs':[{'path':str(p.relative_to(ROOT)),'sha256':sha(p)} for p in inputs]}
(HERE/'findings.json').write_text(json.dumps(out,indent=2)+'\n')
print(json.dumps({'sha256':sha(HERE/'findings.json'),'real_source':out['real_source']}))
