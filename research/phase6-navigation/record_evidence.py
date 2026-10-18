"""Bind Phase6 routing/controller implementation and the complete native corpus."""
from pathlib import Path
import hashlib,json,os,re,subprocess
ROOT=Path(__file__).resolve().parents[2];OUT=Path(__file__).parent
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def main():
 cargo=Path(os.environ['USERPROFILE'])/'.cargo/bin/cargo.exe';commands=[];test_count=0
 for args,name in [(['test','-p','aa-navigation','--','--nocapture'],'rust-tests.log'),(['clippy','-p','aa-navigation','--all-targets','--','-D','warnings'],'clippy.log')]:
  run=subprocess.run([str(cargo),*args],cwd=ROOT,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
  log=OUT/name;log.write_text(run.stdout);commands.append({'command':[str(cargo),*args],'exit_code':run.returncode,'log':name,'sha256':sha(log)})
  if run.returncode:raise RuntimeError(run.stdout)
  if name=='rust-tests.log':test_count=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',run.stdout)))
 sources=[ROOT/'crates/aa-navigation'/p for p in ['Cargo.toml','README.md','src/lib.rs','src/endpoint.rs','src/materialize.rs','src/solver.rs','src/result_plan.rs','tests/endpoint_fixtures.rs','tests/materialize_fixtures.rs','tests/solve_fixtures.rs','tests/result_plan_fixtures.rs']]
 files=['README.md','inspect_native.py','native-provenance.json','record_evidence.py']
 groups={'endpoint':258,'materialize':159,'solve':72,'result-plan':249}
 for group,count in groups.items():
  producer={'result-plan':'result_plan','solve':'solve','endpoint':'endpoint','materialize':'materialize'}[group]+'_oracle.py'
  files += [producer,group+'-fixtures.json',group+'-ranges.json']
  doc=json.loads((OUT/(group+'-fixtures.json')).read_text());assert doc['case_count']==count==len(doc['fixtures']);assert doc['oracle_sha256']==sha(OUT/producer)
 sources += [OUT/p for p in files]+sorted(OUT.glob('*-assembly.txt'))
 deps=['crates/aa-numeric/src/lib.rs','crates/aa-spatial/src/tile_walk.rs','crates/aa-navigation/src/search.rs','crates/aa-navigation/src/cost.rs','research/phase6-pathsearch/native_search.py','research/phase6-pathsearch/native-search-fixtures.json','research/phase6-pathcost/native-cost-fixtures.json','research/phase5-navigation/native_oracle.py','research/phase3-terrain/terrain_uv_oracle.py','research/phase4-rng/original_rng.py']
 report={'schema':1,'passed':True,'component':'Native endpoint adjustment, predecessor materialization, composed Solve and ResultPlan',
 'native_executions':groups,'total_executions':sum(groups.values()),'precision_modes':['Single24','Double53','Extended64'],
 'whole_solve_hooks':['node allocation/free only'],'whole_solve_region_pather':False,
 'rust_test_count_entire_navigation_crate':test_count,'commands':commands,
 'source_sha256':{p.relative_to(ROOT).as_posix():sha(p) for p in sources},'dependency_sha256':{p:sha(ROOT/p) for p in deps},
 'limitations':['Authoritative actual-map grid/object binding remains separate.','Region topology53DC00/53F320 remains an explicit required boundary.','Dynamic actor collision and path-cost inputs remain host responsibilities; whole-Solve integration uses static collision masks.','ResultPlan requires continuation, offset and outer scheduler/controller leaves; no command-to-Mover shortcut.','Precision is explicit; no native game-thread precision default or full FPU exception lifecycle inferred.','Native pointer/loop-invalid domains return explicit errors; errors can follow state changes and are not retry-safe.'],
 'independent_review':'Separate independent-review.json; producer does not self-approve its peer gate'}
 path=OUT/'navigation-evidence.json';path.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'passed':True,'sha256':sha(path),'native_cases':sum(groups.values()),'rust_tests':test_count}))
if __name__=='__main__':main()
