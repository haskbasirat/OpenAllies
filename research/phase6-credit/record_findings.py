"""Bind kill-credit production, native fixtures and focused/full crate checks."""
from pathlib import Path
import hashlib,json,re,subprocess
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
commands=[];count=0
for name,command in [('tests.log',['cargo','test','-p','aa-body']),('clippy.log',['cargo','clippy','-p','aa-body','--all-targets','--','-D','warnings'])]:
 run=subprocess.run(command,cwd=ROOT,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
 (HERE/name).write_bytes(run.stdout);assert run.returncode==0,run.stdout.decode(errors='replace')
 commands.append({'command':command,'exit_code':run.returncode,'log':name,'sha256':sha(HERE/name)})
 if name=='tests.log':count=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',run.stdout.decode())))
doc=json.loads((HERE/'credit-native-oracle.json').read_text())
assert doc['oracle_sha256']==sha(HERE/'original_credit.py') and doc['case_count']==len(doc['fixtures'])==186
files=[ROOT/'crates/aa-body'/p for p in ['src/credit.rs','src/lib.rs','tests/native_credit.rs']]
files += [HERE/p for p in ['README.txt','inspect_native.py','native-ranges.json','original_credit.py','credit-native-oracle.json','record_findings.py']]
files += sorted(HERE.glob('*.asm.txt'))
report={'schema':1,'passed':True,'component':'Native kill credit, statistics and experience amount',
 'original_sha256':doc['original_sha256'],'v14_sha256':doc['v14_sha256'],'native_cases':186,'precision_modes':['Single24','Double53','Extended64'],
 'focused_rust_tests':3,'entire_body_crate_tests':count,'commands':commands,
 'source_sha256':{p.relative_to(ROOT).as_posix():sha(p) for p in files},
 'dependency_sha256':{p:sha(ROOT/p) for p in ['crates/aa-numeric/src/lib.rs','research/phase4-body/original_body.py']},
 'limits':['Explicit actor registry/component views and world callbacks; not an integrated playable world.',
           'Derived actor value and actual reward/promotion/ownership/campaign/death-trigger application remain required leaves.',
           'Result errors preserve earlier credit/statistics; automatic retry is unsafe.',
           'Native pointer/allocation/hash-layout/string and FPU exception lifecycle parity are not claimed.',
           'Visited pins include hook-entry bytes, not instructions behind replaced leaves.'],
 'independent_review':'Separate independent-review.json; producer does not self-approve peer review.'}
path=HERE/'findings.json';path.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'passed':True,'native_cases':186,'body_tests':count,'sha256':sha(path)}))
