from pathlib import Path
import hashlib,json,subprocess,sys
ROOT=Path(__file__).resolve().parents[2];OUT=Path(__file__).parent
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def main():
 commands=[]
 for file in ['inspect_native.py','state_oracle.py','controller_oracle.py','actor_oracle.py','ai_oracle.py']:
  p=subprocess.run([sys.executable,str(OUT/file)],cwd=ROOT,text=True,capture_output=True)
  (OUT/(file+'.log')).write_text(p.stdout+p.stderr);assert p.returncode==0,p.stdout+p.stderr
  commands.append({'command':[sys.executable,'research/phase7-sai/'+file],'exit_code':p.returncode,'log':file+'.log','sha256':H(OUT/(file+'.log'))})
 for name,cmd in [('tests.log',['cargo','test','-p','aa-navigation','--test','sai_state_native','--test','ai_startup_native']),('clippy.log',['cargo','clippy','-p','aa-navigation','--all-targets','--no-deps','--','-D','warnings'])]:
  p=subprocess.run(cmd,cwd=ROOT,text=True,capture_output=True);(OUT/name).write_text(p.stdout+p.stderr);assert p.returncode==0,p.stdout+p.stderr
  commands.append({'command':cmd,'exit_code':p.returncode,'log':name,'sha256':H(OUT/name)})
 source=['crates/aa-navigation/src/sai_state.rs','crates/aa-navigation/src/ai_startup.rs','crates/aa-navigation/tests/sai_state_native.rs','crates/aa-navigation/tests/ai_startup_native.rs']+[str(p.relative_to(ROOT)).replace('\\','/')for p in OUT.iterdir()if p.suffix in ['.py','.md','.json']and p.name not in ['sai-evidence.json','independent-review.json']]
 dependencies=['crates/aa-navigation/src/lib.rs','crates/aa-navigation/src/startup.rs','crates/aa-navigation/src/topology.rs','crates/aa-navigation/src/region_search.rs','research/phase7-navigation/topology_oracle.py','research/phase7-navigation/synthetic-topology.json','research/phase4-rng/original_rng.py']
 report={'schema':1,'passed':True,'scope':'Concrete owned79E170 region/controller registrations and fresh complete7B4830 AIPlayer initialization; exact GActor leaves. Live profile/visibility source binding and special-actor aggregate transitions remain separate.','native_counts':{'region_registration_sequences':21,'region_registrations':168,'classification':88,'seen_lookup':48,'controller_sequences':6,'seen_table':16,'actor_activity':192,'whole_ai_startup':18},'rust_tests':5,'commands':commands,'source_sha256':{p:H(ROOT/p)for p in sorted(source)},'dependency_sha256':{p:H(ROOT/p)for p in dependencies},'independent_review':'Separate peer gate required; this producer does not self-approve.'}
 (OUT/'sai-evidence.json').write_text(json.dumps(report,indent=2)+'\n');print({'passed':True,'sha256':H(OUT/'sai-evidence.json')})
if __name__=='__main__':main()
