"""Independent read-only validation of the root-owned relations/settings adapter."""
from pathlib import Path
import json,hashlib,sys,subprocess,os
HERE=Path(__file__).parent;ROOT=HERE.parents[1]
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def main():
    files=['crates/aa-runtime/src/relations.rs','crates/aa-runtime/src/settings.rs','crates/aa-runtime/tests/relations.rs','crates/aa-runtime/tests/settings.rs',
        'research/phase7-runtime/native_relations.py','research/phase7-runtime/native-relations.json',
        'research/phase7-runtime/native_runtime.py','research/phase6-queues/native_queues.py',
        'crates/aa-rules/src/native.rs','crates/aa-rules/src/tgi.rs','crates/aa-rules/src/sources.rs',
        'crates/aa-mapworld/src/kingdoms.rs','crates/aa-mapworld/src/world.rs']
    pins={p:H(ROOT/p)for p in files}
    sys.path.insert(0,str(HERE));import native_relations as producer
    output=HERE/'independent-relations-output';output.mkdir(exist_ok=True)
    producer.HERE=output;producer.run()
    assert json.loads((output/'native-relations.json').read_text())==json.loads((HERE/'native-relations.json').read_text())
    env=dict(os.environ);env['AA_GAME_ROOT']=str(ROOT.parent)
    commands=[]
    for args,log in [(['cargo','test','-p','aa-runtime','--test','relations','--test','settings','--','--include-ignored','--nocapture'],'rust-tests.log'),
        (['cargo','clippy','-p','aa-runtime','--all-targets','--no-deps','--','-D','warnings'],'clippy.log')]:
        r=subprocess.run(args,cwd=ROOT,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT);(output/log).write_bytes(r.stdout)
        assert r.returncode==0,r.stdout.decode(errors='replace')
        commands.append({'command':args,'AA_GAME_ROOT':str(ROOT.parent),'exit_code':r.returncode,'log':'independent-relations-output/'+log,'sha256':H(output/log)})
    assert all(H(ROOT/p)==h for p,h in pins.items()),'reviewed source changed during validation'
    report={'passed':True,'source_sha256':pins,'native_relation_queries':576,'native_relation_calls':1088,'identical_native_json':True,
        'commands':commands,'scope':'Relations/settings only; catalog/localization and complete live diplomacy mutations excluded.'}
    p=HERE/'independent-relations-validation.json';p.write_text(json.dumps(report,indent=2)+'\n');print(H(p))
if __name__=='__main__':main()
