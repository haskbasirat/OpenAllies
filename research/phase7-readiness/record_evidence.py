"""Bind native preparation/readiness source, tests and declared live boundaries."""
from pathlib import Path
import hashlib,json,subprocess
ROOT=Path(__file__).resolve().parents[2];OUT=Path(__file__).parent
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def main():
 commands=[]
 for command,name in [(['cargo','test','-p','aa-navigation','--test','preparation_native','--test','startup_native','--','--include-ignored','--nocapture'],'rust-tests.log'),(['cargo','clippy','-p','aa-navigation','--all-targets','--no-deps','--','-D','warnings'],'clippy.log')]:
  run=subprocess.run(command,cwd=ROOT,stdout=subprocess.PIPE,stderr=subprocess.STDOUT);log=OUT/name;log.write_bytes(run.stdout);assert run.returncode==0,run.stdout.decode(errors='replace')
  commands.append({'command':command,'exit_code':0,'log':name,'sha256':sha(log)})
 prep=json.loads((OUT/'native-preparation.json').read_text());start=json.loads((OUT/'native-startup.json').read_text())
 assert prep['passed']and start['passed']and len(prep['fixtures'])==42 and len(prep['maps'])==3 and len(start['fixtures'])==17
 assert prep['producer_sha256']==sha(OUT/'prepare_oracle.py')and start['producer_sha256']==sha(OUT/'startup_oracle.py')
 paths=['crates/aa-navigation/src/preparation.rs','crates/aa-navigation/src/startup.rs','crates/aa-navigation/tests/preparation_native.rs','crates/aa-navigation/tests/startup_native.rs']
 docs=['README.md','prepare_oracle.py','startup_oracle.py','native-preparation.json','native-startup.json','inspect_native.py','native-provenance.json','record_evidence.py']
 deps=['crates/aa-navigation/src/lib.rs','crates/aa-navigation/src/topology.rs','crates/aa-navigation/src/region_search.rs','research/phase7-navigation/topology_oracle.py','research/phase7-navigation/native-topology.json','research/phase7-navigation/synthetic-topology.json','research/phase4-rng/original_rng.py']
 report={'schema':1,'passed':True,'scope':'Fresh native7ABC30 graph preparation and required-service7A0D40 outer startup','native_preparation_cases':42,'source_maps':3,'native_startup_cases':17,'rust_tests':4,'commands':commands,
 'source_sha256':{p:sha(ROOT/p)for p in paths}|{str((OUT/p).relative_to(ROOT)).replace('\\','/'):sha(OUT/p)for p in docs},'dependency_sha256':{p:sha(ROOT/p)for p in deps},
 'limits':['Fresh preparation factory, not mutation of existing native pointer ownership.','Graph prepared byte is distinct from SAI ready; factory leaves graphready=false.','Startup World services require actual Kingdom/AI/registry/seen-state behavior, not permissive successful defaults.','Errors may follow native ready=true; runtime must retain failure latch and never infer success from that byte.','No full SAI/world startup, match, old-world static analysis or flag1 cache population claim.'],
 'independent_review':'Separate peer report; producer does not self-approve.'}
 path=OUT/'readiness-evidence.json';path.write_text(json.dumps(report,indent=2)+'\n');print({'passed':True,'sha256':sha(path)})
if __name__=='__main__':main()
