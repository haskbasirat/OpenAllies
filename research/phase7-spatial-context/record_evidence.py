from pathlib import Path
import json,hashlib
ROOT=Path(__file__).resolve().parents[2];HERE=Path(__file__).parent
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def main():
 f=json.loads((HERE/'native-context.json').read_text());v=json.loads((HERE/'validation-record.json').read_text())
 assert len(f['cases'])==384 and len(f['goal_cases'])==672 and v['passed'] and f['producer_sha256']==H(HERE/'original_context.py')
 assert f['query_fixture_sha256']==H(ROOT/'research/phase7-spatial/native-query.json')
 assert f['query_producer_sha256']==H(ROOT/'research/phase7-spatial/original_query.py')
 source=['crates/aa-spatial/src/actor_context.rs','crates/aa-spatial/src/actor_context_tests.rs']
 evidence=['README.txt','original_context.py','native-context.json','validation-record.json','rust-tests.log','clippy.log','record_evidence.py']
 deps=['crates/aa-spatial/src/query.rs','crates/aa-spatial/src/index.rs','crates/aa-spatial/src/index_world.rs','crates/aa-spatial/src/collision.rs','crates/aa-spatial/src/geometry.rs','crates/aa-numeric/src/lib.rs','research/phase7-spatial/original_query.py','research/phase7-spatial/original_index.py','research/phase7-spatial/native-query.json','research/phase4-rng/original_rng.py']
 report={'schema':1,'passed':True,'scope':'Actual554350 constructor source-proven to share5531F0 callee, ordinary immutable actor domain;5522B0 facing setter',
 'native_constructor_cases':384,'native_facing_setter_cases':384,'native_caller_arithmetic_cases':672,'rust_tests_passed':3,'validation':v,
 'source_sha256':{p:H(ROOT/p)for p in source}|{str((HERE/p).relative_to(ROOT)).replace('\\','/'):H(HERE/p)for p in evidence},'dependency_sha256':{p:H(ROOT/p)for p in deps},
 'limits':['Ordinary immutable actor getters only; derived getter mutation and partially constructed descriptor observation are separate.',
 'Native default globals are varied and fully overwritten; their startup lifecycle is not implemented by this entry.',
 'Required definition and query services have no successful defaults; missing actors fail explicitly.',
 'Caller context and sweep kernels do not claim complete movement-world or multiplayer initialization.'],
 'independent_review':'Pending separate peer approval.'}
 p=HERE/'findings.json';p.write_text(json.dumps(report,indent=2)+'\n');print(H(p))
if __name__=='__main__':main()
