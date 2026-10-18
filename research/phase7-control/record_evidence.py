from pathlib import Path
import sys,subprocess,json,hashlib
ROOT=Path(__file__).resolve().parents[2];OUT=Path(__file__).parent
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def main():
 commands=[]
 for name,cmd in [('native.log',[sys.executable,str(OUT/'native_control.py')]),('pins.log',[sys.executable,str(OUT/'inspect_native.py')]),('tests.log',['cargo','test','-p','aa-navigation','--test','control_native']),('clippy.log',['cargo','clippy','-p','aa-navigation','--all-targets','--no-deps','--','-D','warnings'])]:
  p=subprocess.run(cmd,cwd=ROOT,text=True,capture_output=True);log=OUT/name;log.write_text(p.stdout+p.stderr);assert p.returncode==0,p.stdout+p.stderr;commands.append({'command':cmd,'exit_code':0,'log':name,'sha256':H(log)})
 sources=['crates/aa-navigation/src/control.rs','crates/aa-navigation/tests/control_native.rs']+[str(p.relative_to(ROOT)).replace('\\','/')for p in OUT.iterdir()if p.suffix in ['.py','.md','.json']and p.name not in ['control-evidence.json','independent-review.json']]
 doc={'schema':1,'passed':True,'native_cases':31,'rust_tests':2,'commands':commands,'source_sha256':{p:H(ROOT/p)for p in sorted(sources)},'dependency_sha256':{p:H(ROOT/p)for p in ['crates/aa-navigation/src/lib.rs','crates/aa-navigation/src/startup.rs','research/phase4-rng/original_rng.py']},'scope':'Concrete tutorial AI control/flags/actor-command routing with explicit actual virtual service boundaries; no full tutorial or component behavior claim.','independent_review':'Separate peer report required.'}
 (OUT/'control-evidence.json').write_text(json.dumps(doc,indent=2)+'\n');print({'passed':True,'sha256':H(OUT/'control-evidence.json')})
if __name__=='__main__':main()
