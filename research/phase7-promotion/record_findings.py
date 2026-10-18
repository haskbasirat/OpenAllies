from pathlib import Path
import hashlib,json,subprocess
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
doc=json.loads((HERE/'native-promotion-fixtures.json').read_text());assert len(doc['cases'])==141 and doc['producer_sha256']==sha(HERE/'original_promotion.py')
commands=[]
for name,args in [('tests.log',['cargo','test','-p','aa-body','--test','native_promotion']),('clippy.log',['cargo','clippy','-p','aa-body','--all-targets','--','-D','warnings'])]:
 run=subprocess.run(args,cwd=ROOT,stdout=subprocess.PIPE,stderr=subprocess.STDOUT);(HERE/name).write_bytes(run.stdout);assert run.returncode==0,run.stdout.decode(errors='replace');commands.append({'command':args,'exit_code':0,'log':name,'sha256':sha(HERE/name)})
files=[ROOT/'crates/aa-body'/p for p in ['src/promotion.rs','tests/native_promotion.rs']]+[HERE/p for p in ['README.txt','inspect_native.py','native-ranges.json','original_promotion.py','native-promotion-fixtures.json','record_findings.py']]
report={'schema':1,'passed':True,'component':'Owned ExperienceComponent and native rank transitions','native_cases':141,'focused_tests':2,'original_sha256':doc['original_sha256'],'v14_sha256':doc['v14_sha256'],'commands':commands,'source_sha256':{p.relative_to(ROOT).as_posix():sha(p) for p in files},'dependency_sha256':{p:sha(ROOT/p) for p in ['crates/aa-numeric/src/lib.rs','crates/aa-body/src/lib.rs','crates/aa-body/src/credit.rs']},'limitations':['Actual modifier/rank lifecycle and notification/member/allegiance propagation remain required synchronous effects.','Rank tables are real inputs; no default current rank or thresholds inferred.','Failure preserves prior writes; no automatic retry.'],'independent_review':'Separate peer review required.'}
path=HERE/'findings.json';path.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'passed':True,'sha256':sha(path)}))
