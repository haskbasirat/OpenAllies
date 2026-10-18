from pathlib import Path
import json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
fixture=json.loads((HERE/'native-saved-actor-properties.json').read_text())
files=[ROOT/'crates/aa-runtime/src/saved_actor_properties.rs',
       ROOT/'crates/aa-runtime/src/saved_actor_properties_tests.rs',
       HERE/'native_saved_actor_properties.py',HERE/'actual-saved-inputs.json',
       HERE/'native-saved-actor-properties.json',HERE/'README.txt',HERE/'pin_findings.py',
       ROOT/'research/skirmish-20261016-actor-property-core/native_actor_property_core.py',
       ROOT/'research/skirmish-20261016-actor-property-core/native-actor-property-core.json']
doc={'schema':1,'scope':'Native4D4CF0 build-target list and4D4E50 direct-parent refresh on actual saved Actor storage',
     'production_frozen':True,'native_cases':len(fixture['cases']),
     'unmodified_saved_cases':sum(not row['input']['name'].startswith('probe_') for row in fixture['cases']),
     'explicit_storage_probe_cases':sum(row['input']['name'].startswith('probe_') for row in fixture['cases']),
     'portable_identity_source_cycle_checks':9,
     'original_sha256':'5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26',
     'v14_sha256':'3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1',
     'precisions':sorted({row['precision'] for row in fixture['cases']}),
     'images':sorted({row['image'] for row in fixture['cases']}),
     'proven':['actual per-owner list clear/free,head/tail,nodes and member union',
       'canonical Builder504 source references with independent actualA8/174 gates',
       'direct Element parentActor propagation and generation-checked member lookup',
       'retained clear/free/member-union stores before portable source/identity/cycle refusals'],
     'not_proven':['all unrelated global source deferrals','allocation failure/reentry',
       'native corrupt-pointer exception parity','immediate Actor Init/PostCreate/publication',
       'live battle startup or ticking'],
     'files':{str(p.relative_to(ROOT)).replace('\\','/'):sha(p) for p in files}}
path=HERE/'findings.json';path.write_text(json.dumps(doc,indent=2)+'\n')
print(json.dumps({'native_cases':doc['native_cases'],'sha256':sha(path)}))
