"""Independent reviewer rerun, writes only independent-output and review logs."""
from pathlib import Path
import importlib,sys,json,hashlib,time,subprocess
HERE=Path(__file__).parent;ROOT=HERE.parents[1]
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def main():
    out=HERE/'independent-output';out.mkdir(exist_ok=True)
    # The producers bind their oracle's bytes through OUT. Preserve an exact
    # source copy there while still importing the original module path.
    (out/'topology_oracle.py').write_bytes((HERE/'topology_oracle.py').read_bytes())
    ledger=json.loads((HERE/'topology-evidence.json').read_text())
    pins={k:H(ROOT/k)for k in ledger['source_sha256']|ledger['dependency_sha256']}
    # Module globals redirect output without changing producer identity or
    # overwriting frozen author fixtures. Replays still execute original code.
    sys.path.insert(0,str(HERE))
    reports=[]
    for name,output in [('topology_oracle','native-topology.json'),('synthetic_topology','synthetic-topology.json'),
        ('routes_oracle','native-map-routes.json'),('inspect_native','native-provenance.json')]:
        module=importlib.import_module(name);module.OUT=out
        start=time.monotonic()
        resumed = name=='topology_oracle' and '--reuse-completed-map-replay' in sys.argv
        if not resumed: module.main()
        assert json.loads((out/output).read_text())==json.loads((HERE/output).read_text()),output
        reports.append({'producer':name+'.py','output':'independent-output/'+output,'sha256':H(out/output),'identical_json':True,'seconds':None if resumed else time.monotonic()-start,
            'execution': 'Completed native80-map replay in prior invocation (session47919); resumed after output-directory source-copy omission stopped the synthetic producer.' if resumed else 'Executed in this invocation'})
    commands=[]
    for command,log in [(['cargo','test','-p','aa-navigation','--test','topology_native','--','--include-ignored','--nocapture'],'independent-tests.log'),
        (['cargo','clippy','-p','aa-navigation','--all-targets','--no-deps','--','-D','warnings'],'independent-clippy.log')]:
        result=subprocess.run(command,cwd=ROOT,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
        (HERE/log).write_bytes(result.stdout);assert result.returncode==0,result.stdout.decode(errors='replace')
        commands.append({'command':command,'exit_code':result.returncode,'log':log,'sha256':H(HERE/log)})
    assert all(H(ROOT/k)==v for k,v in pins.items()),'review source changed during run'
    report={'passed':True,'replays':reports,'commands':commands,'source_before_after_sha256':pins}
    (HERE/'independent-replay.json').write_text(json.dumps(report,indent=2)+'\n')
    print({'independent_replay_passed':True,'sha256':H(HERE/'independent-replay.json')})
if __name__=='__main__':main()
