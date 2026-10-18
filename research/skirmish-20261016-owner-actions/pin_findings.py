from pathlib import Path
import json,hashlib,collections
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
native=json.loads((HERE/'native-owner-actions.json').read_text())
files=[ROOT/'crates/aa-runtime/src/saved_owner_actions.rs',ROOT/'crates/aa-runtime/src/saved_owner_actions_tests.rs',
    HERE/'native_owner_actions.py',HERE/'actual-saved-inputs.json',HERE/'native-owner-actions.json',HERE/'README.txt',HERE/'pin_findings.py']
doc={'schema':1,'scope':'Saved owner4D1350 rebuild through actual SavedActions/held Organization; native executed coordinator/ground/publication prefixes',
    'production_frozen':True,'native_cases':len(native['cases']),
    'unmodified_saved_cases':sum(not row['input']['name'].startswith('probe_') for row in native['cases']),
    'explicit_probe_cases':sum(row['input']['name'].startswith('probe_') for row in native['cases']),
    'outcomes':dict(collections.Counter(row['output']['outcome'] for row in native['cases'])),
    'instruction_pins':len(native['instructions']),'original_sha256':native['original_sha256'],'v14_sha256':native['v14_sha256'],
    'proven':['actual ownerF0/F4 clear before gates and reached native ground aggregation',
        'same actual held Organization/Actor/typed-list ownership and unchanged member masks in bounded shapes',
        'real saved Kingdom metadata, native empty-HQ services and generation validity predicate',
        'changed-mask-only native4CFCC0 and required owner-view/renderer errors after stores',
        'signed Object4C publication gate and unavailable resupply source read refusal'],
    'not_proven':['full battle Init/PostCreate/publication/ticking','actual Mind/special contributor execution in these owner shapes',
        'nonempty HQ eligibility','actual UI/render publication','native malformed-pointer exception equivalence','arbitrary reentry'],
    'files':{str(p.relative_to(ROOT)).replace('\\','/'):sha(p) for p in files}}
path=HERE/'findings.json';path.write_text(json.dumps(doc,indent=2)+'\n');print(json.dumps({'sha256':sha(path),'native_cases':len(native['cases'])}))
