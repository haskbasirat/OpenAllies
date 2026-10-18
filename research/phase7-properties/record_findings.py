from pathlib import Path
import hashlib,json,subprocess
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
doc=json.loads((HERE/'native-properties-fixtures.json').read_text());assert len(doc['cases'])==420 and doc['producer_sha256']==sha(HERE/'original_properties.py')
commands=[]
for name,args in [('tests.log',['cargo','test','-p','aa-body','--test','native_properties']),('clippy.log',['cargo','clippy','-p','aa-body','--all-targets','--','-D','warnings'])]:
 run=subprocess.run(args,cwd=ROOT,stdout=subprocess.PIPE,stderr=subprocess.STDOUT);(HERE/name).write_bytes(run.stdout);assert run.returncode==0,run.stdout.decode(errors='replace');commands.append({'command':args,'exit_code':0,'log':name,'sha256':sha(HERE/name)})
files=[ROOT/'crates/aa-body'/p for p in ['src/properties.rs','tests/native_properties.rs']]+[HERE/p for p in ['README.txt','original_properties.py','native-properties-fixtures.json','record_findings.py','inspect_domain.py','domain-evidence.json']]
report={'schema':1,'passed':True,'component':'Concrete Properties/Modifier constructors and native getters','native_cases':420,'focused_tests':2,'original_sha256':doc['original_sha256'],'v14_sha256':doc['v14_sha256'],'commands':commands,'source_sha256':{p.relative_to(ROOT).as_posix():sha(p) for p in files},'dependency_sha256':{p:sha(ROOT/p) for p in ['crates/aa-numeric/src/lib.rs','crates/aa-body/src/lib.rs']},'limitations':['Property activation/removal/recomputation is separate from constructor/getter scope.','Actual loaded definition-domain count/global minimum/actor effect identities are required.','Only finite round-to-nearest arithmetic; explicit invalid offset/division/nonfinite errors.'],'independent_review':'Separate peer review required.'}
path=HERE/'findings.json';path.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'passed':True,'sha256':sha(path)}))
