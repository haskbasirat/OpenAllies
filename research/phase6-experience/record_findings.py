from pathlib import Path
import hashlib,json,re,subprocess
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
doc=json.loads((HERE/'native-experience-fixtures.json').read_text());assert len(doc['cases'])==249 and doc['producer_sha256']==sha(HERE/'original_experience.py')
commands=[];count=0
for name,args in [('tests.log',['cargo','test','-p','aa-body']),('clippy.log',['cargo','clippy','-p','aa-body','--all-targets','--','-D','warnings'])]:
 run=subprocess.run(args,cwd=ROOT,stdout=subprocess.PIPE,stderr=subprocess.STDOUT);(HERE/name).write_bytes(run.stdout);assert run.returncode==0,run.stdout.decode(errors='replace')
 commands.append({'command':args,'exit_code':run.returncode,'log':name,'sha256':sha(HERE/name)})
 if name=='tests.log':count=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',run.stdout.decode())))
files=[ROOT/'crates/aa-body'/p for p in ['src/experience.rs','src/lib.rs','tests/native_experience.rs','README.md']]
files += [HERE/p for p in ['README.txt','original_experience.py','native-experience-fixtures.json','inspect_native.py','native-ranges.json','record_findings.py']]+sorted(HERE.glob('*.asm.txt'))
report={'schema':1,'passed':True,'component':'Native actor component value and experience routing','original_sha256':doc['original_sha256'],'v14_sha256':doc['v14_sha256'],
 'value_cases':165,'award_cases':84,'precision_modes':['Single24','Double53','Extended64'],'focused_tests':2,'entire_body_tests':count,'commands':commands,
 'source_sha256':{p.relative_to(ROOT).as_posix():sha(p) for p in files},'dependency_sha256':{p:sha(ROOT/p) for p in ['crates/aa-numeric/src/lib.rs','crates/aa-body/src/credit.rs','research/phase4-body/original_body.py']},
 'limits':['Actual actor/regiment component binding remains a required world adapter.','Promotion application571A60 remains a required callback; no guessed rank thresholds/effects.','Argument=true578C00 resupply value path is not implemented.','Errors preserve earlier effects; cycle detection applies only to unchanged routes between mutating callbacks, and callback-producing loops have no arbitrary cap.','Native heap layouts, invalid pointers and FPU exception flags are not emulated.'],
 'independent_review':'Pending separate peer review; this producer is not an independent approval.'}
path=HERE/'findings.json';path.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'passed':True,'cases':249,'body_tests':count,'sha256':sha(path)}))
