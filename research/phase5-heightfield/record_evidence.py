"""Bind finite native heightfield samplers, execution corpus and Rust checks."""
from pathlib import Path
import hashlib,json,os,subprocess
ROOT=Path(__file__).resolve().parents[2];OUT=Path(__file__).parent
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def main():
 cargo=Path(os.environ['USERPROFILE'])/'.cargo/bin/cargo.exe';commands=[]
 for args,name in [(['test','-p','aa-heightfield','--','--nocapture'],'rust-tests.log'),(['clippy','-p','aa-heightfield','--all-targets','--','-D','warnings'],'clippy.log')]:
  run=subprocess.run([str(cargo),*args],cwd=ROOT,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
  log=OUT/name;log.write_text(run.stdout);commands.append({'command':[str(cargo),*args],'exit_code':run.returncode,'log':name,'sha256':sha(log)})
  if run.returncode:raise RuntimeError(run.stdout)
 fixture=json.loads((OUT/'native-heightfield-fixtures.json').read_text());assert fixture['passed']
 assert fixture['case_count']==len(fixture['fixtures'])==735
 assert fixture['oracle_sha256']==sha(OUT/'original_heightfield.py')
 sources=[ROOT/'crates/aa-heightfield'/p for p in ['Cargo.toml','src/lib.rs','tests/native_fixtures.rs','README.md']]
 sources += [OUT/p for p in ['README.md','inspect_native.py','native-ranges.json','original_heightfield.py','native-heightfield-fixtures.json','executed-ranges.json','record_evidence.py']]
 sources += sorted(OUT.glob('*-assembly.txt'))
 report={'schema':1,'passed':True,'component':'Original native nearest/interpolated terrain heightfield samplers',
  'native_map_grid_mapping_implemented':False,'original_sha256':fixture['original_sha256'],'v14_sha256':fixture['v14_sha256'],
  'rust_test_count':2,'native_fixture_executions':735,'scenarios_per_precision':245,
  'precision_modes':['Single24','Double53','Extended64'],'return_comparison':'Exact80-bit value plus explicit binary32 store',
  'commands':commands,'source_sha256':{p.relative_to(ROOT).as_posix():sha(p) for p in sources},
  'dependency_sha256':{'crates/aa-numeric/src/lib.rs':sha(ROOT/'crates/aa-numeric/src/lib.rs')},
  'supported':['Original550190 signed truncate/wrapping bias/masked arithmetic shift/clamp.',
   'Original550210 signed remainder threshold branches, axis averages and asymmetric bilinear stores.',
   'Extended returns preserved for the native caller; finite exact FPREM remainder.'],
  'limits':['No actual map-to-heightfield construction or base/overlay world integration.',
   'Finite defined arithmetic only; invalid native pointer/index domains return errors.',
   'Game-thread precision and exception status lifecycle are not inferred.',
   'Caller must retain the Binary result until its own native store boundary.',
   'Original game, accepted v14, existing releases and frozen crates remain unchanged.'],
  'independent_review':'Separate independent-review.json; this producer does not issue its own peer gate'}
 path=OUT/'heightfield-evidence.json';path.write_text(json.dumps(report,indent=2)+'\n')
 print(json.dumps({'passed':True,'sha256':sha(path),'native_cases':735,'tests':2}))
if __name__=='__main__':main()
