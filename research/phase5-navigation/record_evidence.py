"""Bind scoped navigation implementation, native execution corpus and tests."""
from pathlib import Path
import collections, hashlib, json, os, subprocess
ROOT=Path(__file__).resolve().parents[2];OUT=Path(__file__).parent
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def main():
 cargo=Path(os.environ['USERPROFILE'])/'.cargo/bin/cargo.exe'
 commands=[]
 for args,name in [(['test','-p','aa-navigation','--','--nocapture'],'rust-tests.log'),(['clippy','-p','aa-navigation','--all-targets','--','-D','warnings'],'clippy.log')]:
  run=subprocess.run([str(cargo),*args],cwd=ROOT,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
  log=OUT/name;log.write_text(run.stdout);commands.append({'command':[str(cargo),*args],'exit_code':run.returncode,'log':name,'sha256':sha(log)})
  if run.returncode:raise RuntimeError(run.stdout)
 fixture=json.loads((OUT/'native-navigation-fixtures.json').read_text());assert fixture['passed']
 ops=collections.Counter(r['input']['operation'] for r in fixture['fixtures']);n=fixture['case_count']
 assert n==len(fixture['fixtures'])==255 and ops['self_copy']==3
 source=[ROOT/'crates/aa-navigation'/p for p in ['Cargo.toml','src/lib.rs','tests/native_fixtures.rs','README.md']]
 source += [OUT/p for p in ['README.md','inspect_native.py','native-ranges.json','native_oracle.py','native-navigation-fixtures.json','record_evidence.py','client_samples.rs','client-samples.json']]
 source += sorted(OUT.glob('*-assembly.txt'))
 deps=[ROOT/p for p in ['crates/aa-numeric/src/lib.rs','crates/aa-numeric/tests/square_root.rs','research/phase5-numeric/sqrt_oracle.py','research/phase5-numeric/sqrt-vectors.json']]
 report={'schema':1,'passed':True,'component':'Original owned paths, activation/path events and prepared-node ActionGo',
  'route_solver_implemented':False,'full_navigation_claim':False,'original_sha256':fixture['original_sha256'],'v14_sha256':fixture['v14_sha256'],
  'rust_test_count':3,'native_fixture_executions':n,'scenarios_per_precision':85,'precision_modes':['Single24','Double53','Extended64'],
  'rust_operation_comparisons':n-ops['self_copy'],'native_self_copy_noop_contract_rows':ops['self_copy'],'operations':dict(ops),
  'commands':commands,'source_sha256':{p.relative_to(ROOT).as_posix():sha(p) for p in source},
  'dependency_sha256':{p.relative_to(ROOT).as_posix():sha(p) for p in deps},
  'independent_review':'separate independent-review.json; this producer does not issue its own peer gate',
  'supported':['All eight raw payload words, independent path ownership, metadata copy, clear and counter-driven tail pruning.',
   'Original activation field writes plus ordinary/formation path-event ordering and rejection behavior.',
   'Complete ActionGo outer control/arithmetic, required mutable native world leaves and explicit precision.',
   'Exactly one unrounded native integer-to-unit RNG conversion after Mover request and native timer formula.'],
  'limits':['No path solver, region geometry, route tie-breaks, factory defaults or direct command-to-Mover bypass.',
   'World callbacks remain implementations required from the host: UAI activation, formation, organization, geometry, Mover, shared RNG and timer.',
   'No game-thread default precision is inferred; all three modes are explicit tested operation inputs.',
   'Diagnostic logging is disabled in fixtures; allocator addresses/timing and corrupted native memory are not reproduced.',
   'Safe Rust disallows source/destination alias; original self-copy rows check its no-op contract rather than unsafe Rust aliases.',
   'Errors are nontransactional and must not be automatically retried; allocation failure preserves the previous path before commit.',
   'Stable precision/settings are supplied operation inputs, not arbitrary callback-mutated global settings.',
   'Original/native/release/runtime/profile files and frozen aa-movement were not modified.']}
 path=OUT/'navigation-evidence.json';path.write_text(json.dumps(report,indent=2)+'\n')
 print(json.dumps({'passed':True,'sha256':sha(path),'native_cases':n,'rust_operation_comparisons':n-3,'tests':3}))
if __name__=='__main__':main()
