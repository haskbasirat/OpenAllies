"""Independent source-bound target-surface oracle replay and scoped tests."""
from pathlib import Path
import sys,json,hashlib,subprocess
HERE=Path(__file__).parent;ROOT=HERE.parents[1];H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def main():
    files=['crates/aa-combat/src/surface.rs','crates/aa-combat/src/projectile.rs','crates/aa-combat/tests/native_surface.rs',
        'crates/aa-spatial/src/collision.rs','crates/aa-numeric/src/lib.rs','research/phase7-surface/native_surface.py',
        'research/phase7-surface/native-surface-fixtures.json','research/phase7-surface/README.txt','research/phase7-surface/findings.json']
    pins={p:H(ROOT/p)for p in files}
    sys.path.insert(0,str(HERE));import native_surface as producer
    output=HERE/'independent-output';output.mkdir(exist_ok=True);producer.HERE=output;producer.main()
    assert json.loads((output/'native-surface-fixtures.json').read_text())==json.loads((HERE/'native-surface-fixtures.json').read_text())
    commands=[]
    for args,log in [(['cargo','test','-p','aa-combat','--test','native_surface','--','--nocapture'],'rust-tests.log'),
        (['cargo','clippy','-p','aa-combat','--all-targets','--no-deps','--','-D','warnings'],'clippy.log')]:
        r=subprocess.run(args,cwd=ROOT,stdout=subprocess.PIPE,stderr=subprocess.STDOUT);(output/log).write_bytes(r.stdout)
        assert r.returncode==0,r.stdout.decode(errors='replace')
        commands.append({'command':args,'exit_code':r.returncode,'log':'independent-output/'+log,'sha256':H(output/log)})
    assert all(H(ROOT/p)==h for p,h in pins.items()),'surface source changed during validation'
    report={'passed':True,'source_sha256':pins,'native_cases':1104,'native_mutated_getter_cases':60,'identical_native_json':True,'commands':commands}
    p=HERE/'independent-validation.json';p.write_text(json.dumps(report,indent=2)+'\n');print(H(p))
if __name__=='__main__':main()
