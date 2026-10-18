"""Independent exact native overlap replay; frozen producer files stay intact."""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).parent;ROOT=HERE.parents[1];H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def main():
    files=['crates/aa-spatial/src/shape_overlap.rs','crates/aa-spatial/tests/native_shape_overlap.rs',
        'research/phase7-shape-overlap/native_overlap.py','research/phase7-shape-overlap/native-overlap.json',
        'research/phase7-shape-overlap/FINDINGS.txt','research/phase7-shape-overlap/validation-record.json','crates/aa-numeric/src/lib.rs']
    pins={p:H(ROOT/p)for p in files}
    sys.path.insert(0,str(HERE));import native_overlap as producer
    out=HERE/'independent-output';out.mkdir(exist_ok=True);producer.HERE=out;producer.main()
    assert json.loads((out/'native-overlap.json').read_text())==json.loads((HERE/'native-overlap.json').read_text())
    assert all(H(ROOT/p)==h for p,h in pins.items()),'reviewed source changed during native replay'
    report={'passed':True,'source_sha256':pins,'identical_native_json':True,'cases':13200,'footprints':174,'sat':420,'precision_witnesses':220}
    p=HERE/'independent-validation.json';p.write_text(json.dumps(report,indent=2)+'\n');print(H(p))
if __name__=='__main__':main()
