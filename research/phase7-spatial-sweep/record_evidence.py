"""Bind completed sweep evidence without self-approving independent review."""
from pathlib import Path
import hashlib,json
ROOT=Path(__file__).resolve().parents[2];HERE=Path(__file__).parent
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def main():
 f=json.loads((HERE/'native-sweep.json').read_text());v=json.loads((HERE/'validation-record.json').read_text())
 assert f['producer_sha256']==H(HERE/'original_sweep.py') and v['passed']
 assert f['query_fixture_sha256']==H(ROOT/'research/phase7-spatial/native-query.json')
 assert f['query_producer_sha256']==H(ROOT/'research/phase7-spatial/original_query.py')
 assert len(f['cases'])==1024;corridors=sum(len(c['output']['queries'])==2 for c in f['cases']);assert corridors==255
 paths=['crates/aa-spatial/src/sweep.rs','crates/aa-spatial/src/sweep_tests.rs']
 files=['README.txt','original_sweep.py','native-sweep.json','validation-record.json','clippy.log','record_evidence.py']
 deps=['crates/aa-spatial/src/query.rs','crates/aa-spatial/src/index.rs','crates/aa-spatial/src/index_world.rs','crates/aa-spatial/src/shape_overlap.rs','crates/aa-spatial/src/geometry.rs','crates/aa-spatial/src/collision.rs','crates/aa-numeric/src/lib.rs','research/phase7-spatial/original_query.py','research/phase7-spatial/original_index.py','research/phase7-spatial/native-query.json','research/phase4-rng/original_rng.py']
 report={'schema':1,'passed':True,'scope':'Native553CE0 endpoint and rectangle sweep using concrete first-hit geometry and required live policy',
 'native_cases':1024,'native_corridor_cases':corridors,'rust_tests_passed':2,'validation':v,
 'source_sha256':{p:H(ROOT/p)for p in paths}|{str((HERE/p).relative_to(ROOT)).replace('\\','/'):H(HERE/p)for p in files},'dependency_sha256':{p:H(ROOT/p)for p in deps},
 'limits':['Prepared descriptors and live QueryPolicy services are required;554350 caller context is not replaced by5531F0.',
 'Original endpoint/constructor/first-hit arithmetic executes natively; allocation and prepared actor policy leaves remain explicit fixture services.',
 'Finite geometry with explicit PC modes and ordinary immutable shapes; no whole-match or restored-map movement claim.',
 'Errors retain completed descriptor/world effects; first-hit temporary marks clean up without rolling back prior effects.'],
 'independent_review':'Pending a separate peer; producer does not self-approve.'}
 p=HERE/'findings.json';p.write_text(json.dumps(report,indent=2)+'\n');print(H(p))
if __name__=='__main__':main()
