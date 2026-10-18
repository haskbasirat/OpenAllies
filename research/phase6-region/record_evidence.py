"""Bind the recovered region prepass without claiming graph/topology parity."""
from pathlib import Path
import hashlib,json,os,subprocess
ROOT=Path(__file__).resolve().parents[2];OUT=Path(__file__).parent
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def main():
 cargo=Path(os.environ['USERPROFILE'])/'.cargo/bin/cargo.exe';commands=[]
 for args,name in [(['test','-p','aa-navigation','--test','region_prepass','--','--nocapture'],'rust-tests.log'),(['clippy','-p','aa-navigation','--all-targets','--','-D','warnings'],'clippy.log')]:
  run=subprocess.run([str(cargo),*args],cwd=ROOT,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT);log=OUT/name;log.write_text(run.stdout)
  commands.append({'command':[str(cargo),*args],'exit_code':run.returncode,'log':name,'sha256':sha(log)})
  if run.returncode:raise RuntimeError(run.stdout)
 fixture=json.loads((OUT/'prepass-fixtures.json').read_text());assert fixture['passed'] and fixture['case_count']==len(fixture['fixtures'])==168;assert fixture['oracle_sha256']==sha(OUT/'prepass_oracle.py')
 sources=[ROOT/'crates/aa-navigation/src/region.rs',ROOT/'crates/aa-navigation/tests/region_prepass.rs']+[OUT/p for p in ['README.md','prepass_oracle.py','prepass-fixtures.json','prepass-ranges.json','prepass-assembly.txt','record_evidence.py']]
 deps=['crates/aa-numeric/src/lib.rs','crates/aa-navigation/src/lib.rs','research/phase6-navigation/materialize_oracle.py','research/phase3-terrain/terrain_uv_oracle.py']
 report={'schema':1,'passed':True,'component':'Original53DC00 region path prepass','native_cases':168,'rust_tests':1,'precision_modes':['Single24','Double53','Extended64'],'graph_search_evidence':'graph-evidence.json (separate scope)','actual_map_region_topology_implemented':False,
  'commands':commands,'source_sha256':{p.relative_to(ROOT).as_posix():sha(p) for p in sources},'dependency_sha256':{p:sha(ROOT/p) for p in deps},
  'seams':fixture['seams'],'limits':['Region lookup/ordered graph route and dynamic geometry remain required host services.','Temporary linked-list allocation/free becomes safe Rust owned storage.','Failure may retain already emitted path nodes; no rollback or implicit reset.','This evidence does not cover full region-search heap or world-load topology reconstruction.'],
  'independent_review':'Separate peer report; producer does not self-approve.'}
 path=OUT/'region-evidence.json';path.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'passed':True,'native_cases':168,'sha256':sha(path)}))
if __name__=='__main__':main()
