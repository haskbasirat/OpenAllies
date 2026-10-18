"""Independently reproduce production transitions and bind the scoped review."""
from pathlib import Path
import hashlib,json,subprocess,sys
OUT=Path(__file__).resolve().parent;ROOT=OUT.parents[1]
sys.path.insert(0,str(OUT))
import native_production as native
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
document=json.loads((OUT/'native-production-fixtures.json').read_text())
assert document['producer_sha256']==sha(OUT/'native_production.py')
count=0
for case in document['cases']:
    machine=native.ProductionMachine(case['input'],dict(native.economy.MODES)[case['precision']])
    for step in case['steps']:
        actual=machine.action(step['input'])
        assert actual==step['output'],(case['input']['name'],case['precision'],step['input'],actual,step['output'])
        count+=1
logs=[]
for name,command in [
    ('independent-tests.log',['cargo','test','-p','aa-production','--','--nocapture']),
    ('independent-clippy.log',['cargo','clippy','-p','aa-production','--all-targets','--','-D','warnings']),
]:
    run=subprocess.run(command,cwd=ROOT,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
    (OUT/name).write_bytes(run.stdout)
    assert run.returncode==0,run.stdout.decode(errors='replace')
    logs.append({'command':command,'exit_code':run.returncode,'log':name,'sha256':sha(OUT/name)})
assert '4 passed; 0 failed' in (OUT/'independent-tests.log').read_text()
files=[ROOT/name for name in ['crates/aa-production/Cargo.toml','crates/aa-production/src/lib.rs','crates/aa-production/tests/native_queue.rs',
    'crates/aa-numeric/src/lib.rs','crates/aa-economy/src/lib.rs','research/evidence/economy-phase2-oracle.py']]
files += [OUT/name for name in ['README.txt','findings.json','native_production.py','native-production-fixtures.json','independent_review.py']]
report={'schema':1,'passed':True,'reviewer':'orders_spatial','production_edits_by_reviewer':False,
    'scope':'Native recruit admission, production FIFO/timing/accounting decisions with explicit resolved-world callbacks and constrained queued-item reentrancy.',
    'native_sequences_reexecuted':len(document['cases']),'native_transitions_reexecuted':count,
    'rust_tests':4,'strict_clippy_passed':True,'commands':logs,
    'source_sha256':{p.relative_to(ROOT).as_posix():sha(p)for p in files},'findings':[],
    'resolved_review_issues':[
        'Rust head snapshots do not model arbitrary mutation through aliased native item pointers. Public World documentation and README now explicitly require queued membership/order/item fields to remain stable while held; the head-ID guard is only partial diagnostics, not identity proof across reused IDs.',
        'Due research with a null property originally skipped fulfillment/refund but Rust called fulfill. The implementation now preserves the native skip, release and next-head behavior; three new original sequences verify it across precision modes.'
    ],
    'checks':[
        'Re-executed every original sequence in isolated Unicorn and compared whole queue state, economy state, ID counter, return values and callback trace against the committed fixture.',
        'Production oracle checks original/v14 bytes for every visited instruction, original callee-saved registers and return stack. Its metadata explicitly distinguishes intercepted boundary entries from executed bodies.',
        'Recruit admission retains separate payment/affordability owner capture and two quotes, with the second quote debited without an invented affordability recheck.',
        'Original duration FST leaves the extended value available for deadline addition; precision is supplied explicitly.',
        'Completion is checked before capacity/disabled/supply pause gates. Pause deadline updates, reason notifications and upkeep-copy ordering match native control flow.',
        'Head removal cancels scheduling before upkeep release; cancellation does not invent next-head startup; completion does advance the head.',
        'Existing aa-economy backs test debit/credit/consumption rather than a duplicate simplified arithmetic implementation.',
        'Callbacks are required and fallible. Error tests preserve payment/removal effects and stop later fulfillment/refunds without claiming rollback.'
    ],
    'limits':[
        'Actor/research fulfillment, supply, capacity, prices, authoritative command routing and actual scheduler/world ownership remain required services.',
        'Prepared research progression is supported; research request/pricing admission is not implemented.',
        'Context must remain stable during an operation. Arbitrary queued-item alias mutation/reentrant membership changes while held are unsupported; post-removal fulfillment/refund/release may enqueue new items.',
        'Native item+4C is uninitialized by its constructors. Rust zero initialization is an explicit safe definition, not a native heap-content parity claim.',
        'Malformed/nonfinite arithmetic and allocation failures use Rust errors. No full playable match or process-wide OOM recovery is claimed.'
    ]}
(OUT/'independent-review.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({'passed':True,'sequences':len(document['cases']),'transitions':count,'sha256':sha(OUT/'independent-review.json')}))
