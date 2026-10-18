from pathlib import Path
import json,hashlib,collections
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
native=json.loads((HERE/'native-saved-leader.json').read_text())
files=[ROOT/'crates/aa-runtime/src/saved_organization_leader.rs',
       ROOT/'crates/aa-runtime/src/saved_organization_leader_tests.rs',
       HERE/'native_saved_leader.py',HERE/'actual-saved-inputs.json',
       HERE/'native-saved-leader.json',HERE/'README.txt',HERE/'pin_findings.py']
doc={'schema':1,'scope':'Saved579A00 borrowed actual Organization/Element/Thing/canonical LayoutFormation/RNG adapter',
     'production_frozen':True,'native_cases':len(native['cases']),
     'unmodified_saved_cases':sum(not row['input']['name'].startswith('probe_') for row in native['cases']),
     'explicit_storage_probe_cases':sum(row['input']['name'].startswith('probe_') for row in native['cases']),
     'outcomes':dict(collections.Counter(row['output']['outcome'] for row in native['cases'])),
     'portable_identity_source_checks':12,
     'precisions':sorted({row['precision'] for row in native['cases']}),
     'images':sorted({row['image'] for row in native['cases']}),
     'original_sha256':native['original_sha256'],'v14_sha256':native['v14_sha256'],
     'proven':['leader and per-axis offset stores preserve partial failures in actual saved allocation',
       'canonical current layout/formation fields and original native XY facing/position',
       'full generation identity and required Element header owner checks',
       'same actual LoadContext RNG stream using85D840 native arithmetic',
       'actual native-message sink fields and explicit Mind+114 boundary'],
     'not_proven':['concrete Mind+114 execution','immediate saved Actor Init/PostCreate/publication',
       'native malformed-pointer exception parity','arbitrary reentry in saved adapter',
       'nonfinite arithmetic','every CRT formatted diagnostic string','playable battle'],
     'files':{str(p.relative_to(ROOT)).replace('\\','/'):sha(p) for p in files}}
out=HERE/'findings.json';out.write_text(json.dumps(doc,indent=2)+'\n');print(json.dumps({'sha256':sha(out),'native_cases':len(native['cases'])}))
