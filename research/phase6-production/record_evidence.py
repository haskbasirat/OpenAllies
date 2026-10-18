from pathlib import Path
import hashlib,json

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
fixture=json.loads((HERE/'native-production-fixtures.json').read_text(encoding='utf-8'))
assert fixture['producer_sha256']==sha(HERE/'native_production.py')
assert len(fixture['cases'])==231
assert sum(len(row['steps']) for row in fixture['cases'])==558
tests=(HERE/'rust-tests.log').read_text(encoding='utf-8-sig')
clippy=(HERE/'clippy.log').read_text(encoding='utf-8-sig')
assert '4 passed; 0 failed' in tests
assert 'Finished' in clippy and 'error:' not in clippy
paths=[HERE/'native_production.py',HERE/'native-production-fixtures.json',HERE/'README.txt',
       HERE/'native-oracle.log',HERE/'rust-tests.log',HERE/'clippy.log',
       ROOT/'crates/aa-production/Cargo.toml',ROOT/'crates/aa-production/src/lib.rs',
       ROOT/'crates/aa-production/tests/native_queue.rs']
report={'schema':1,'passed':True,'component':'native production queue and recruit admission',
        'original_sha256':fixture['original_sha256'],'v14_sha256':fixture['v14_sha256'],
        'native_sequences':231,'native_transitions':558,'rust_tests':4,
        'precision_modes':['Single24','Double53','Extended64'],
        'strict_clippy_passed':True,
        'implemented':['recruit request and organization enrollment gates','double price quote, captured owner and payment ordering',
                       'native FIFO insertion, duration, pause/deadline processing','completion and failure-refund callbacks',
                       'head/non-head cancellation, upkeep removal and organization slot lifetime'],
        'integration':'Tests use real aa-economy for affordability, debit/credit and coupled consumption; no aa-world modifications or fake actor fulfillment.',
        'required_boundaries':['authored pricing and recruitability','supply and actual capacity predicates','ownership/organization registries',
                               'actor/research fulfillment','scheduler links and native clock','command routing and authorization'],
        'explicit_differences':['Rust initializes native unspecified new-item +0x4C to zero before later organization detachment.',
                                'Finite checked input domain and fallible callbacks; no undefined native heap behavior.',
                                'Callbacks preserve live queue membership/order/item fields while an item is selected; recursive same-queue execution and raw item-storage alias mutation are unsupported.',
                                'Research queue progression is supported but research request/pricing admission is separate.'],
        'sha256':{p.relative_to(ROOT).as_posix():sha(p) for p in paths}}
(HERE/'findings.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'passed':True,'findings_sha256':sha(HERE/'findings.json')}))
