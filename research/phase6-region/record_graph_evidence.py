"""Bind native concrete region graph search, not native topology construction."""
from pathlib import Path
import hashlib,json,os,subprocess
ROOT=Path(__file__).resolve().parents[2];OUT=Path(__file__).parent
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def main():
 cargo=Path(os.environ['USERPROFILE'])/'.cargo/bin/cargo.exe';commands=[]
 for args,name in [(['test','-p','aa-navigation','--test','region_search_native','--','--nocapture'],'graph-rust-tests.log'),(['clippy','-p','aa-navigation','--all-targets','--','-D','warnings'],'graph-clippy.log')]:
  run=subprocess.run([str(cargo),*args],cwd=ROOT,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT);log=OUT/name;log.write_text(run.stdout)
  commands.append({'command':[str(cargo),*args],'exit_code':run.returncode,'log':name,'sha256':sha(log)})
  if run.returncode:raise RuntimeError(run.stdout)
 fixture=json.loads((OUT/'graph-fixtures.json').read_text());assert fixture['passed'] and fixture['case_count']==len(fixture['fixtures'])==96;assert fixture['oracle_sha256']==sha(OUT/'graph_oracle.py')
 sources=[ROOT/'crates/aa-navigation/src/region_search.rs',ROOT/'crates/aa-navigation/tests/region_search_native.rs']+[OUT/p for p in ['README.md','graph_oracle.py','graph-fixtures.json','graph-ranges.json','inspect_graph.py','graph-provenance.json','graph-full-assembly.txt','record_graph_evidence.py']]
 deps=['crates/aa-numeric/src/lib.rs','crates/aa-navigation/src/lib.rs','crates/aa-navigation/src/region.rs','research/phase4-rng/original_rng.py']
 report={'schema':1,'passed':True,'component':'Original concrete flag0 region search53F320/53EDE0','native_cases':96,'rust_tests':2,'precision_modes':['Single24','Double53','Extended64'],'graph_search_implemented':True,'actual_map_region_topology_implemented':False,'cached_route_flag1_implemented':False,
  'commands':commands,'source_sha256':{p.relative_to(ROOT).as_posix():sha(p) for p in sources},'dependency_sha256':{p:sha(ROOT/p) for p in deps},'seams':fixture['seams'],
  'limits':['Caller supplies original ordered topology, region categories and optional per-node cached distances.','Native world-load topology construction and world-to-region lookup remain required.','Native cached-route flag1 tables are not implemented; prepass uses flag0.','Missing/cyclic predecessor chains return explicit errors after prior effects instead of invalid memory access.','Allocation addresses/timing are not reproduced.','Finite numeric inputs and explicit x87 precision; no full FPU exception lifecycle inferred.'],
  'independent_review':'Separate peer report; producer does not self-approve.'}
 path=OUT/'graph-evidence.json';path.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'passed':True,'native_cases':96,'sha256':sha(path)}))
if __name__=='__main__':main()
