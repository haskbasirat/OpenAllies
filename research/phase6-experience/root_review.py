"""Independent parent review runner; never rewrites producer fixtures."""
from pathlib import Path
import sys,json,hashlib,subprocess
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
from original_experience import Machine,IMAGE,ACCEPTED
def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()
doc=json.loads((HERE/'native-experience-fixtures.json').read_text());assert len(doc['cases'])==249
visited={}
for row in reversed(doc['cases']):
 m=Machine(row['input']);assert m.run()==row['output'],row['input']['name'];visited.update(m.visited)
for a,n in visited.items():assert IMAGE.at(a,n)==ACCEPTED.at(a,n),hex(a)
result=subprocess.run(['cargo','test','-p','aa-body','--test','native_experience','--release','--target','x86_64-pc-windows-msvc'],cwd=ROOT,text=True,capture_output=True,encoding='utf-8')
(HERE/'independent-root-tests.log').write_text(result.stdout+result.stderr,encoding='utf-8');assert result.returncode==0
files=['crates/aa-body/src/experience.rs','crates/aa-body/tests/native_experience.rs','research/phase6-experience/original_experience.py','research/phase6-experience/native-experience-fixtures.json','research/phase6-experience/004d01e0.asm.txt','research/phase6-experience/004ce770.asm.txt','research/phase6-experience/00578710.asm.txt']
report={'schema':1,'passed':True,'reviewer':'root original scope; visual_audit cycle-fix acceptance','native_cases_reexecuted':249,'value_cases_without_behavior_hooks':165,'routing_cases_with_promotion_leaf':84,'visited_instruction_windows':len(visited),'rust_tests_passed':2,
 'source_sha256':{f:digest(ROOT/f) for f in files},'findings':[],
 'reviewed_contracts':['Exact zero/regiment replacement then wall,combat,component88 addition; retains extended result and explicit PC.','Threshold, component priority, cached regiment identity, reread member array/count and post-leader experience pointer match inspected instructions.','Boolean leader guards preserve terminating self-leader calls. Repeated routes with no intervening promotion callback are rejected; successful callbacks invalidate all active keys so changed ancestor routes are reevaluated.','Callback and unsupported numeric errors retain earlier effects; live views must supply actual coherent fields.'],
 'limits':['Only argument=false actor-value path.','571A60 promotion thresholds/effects are a required callback, not implemented by this routing helper.','Callback-producing recursive loops cannot safely be classified as permanently cyclic; no arbitrary callback count limit is imposed.','No complete actor/world/promotion, multiplayer, or gameplay parity claim.'],
 'test_log_sha256':digest(HERE/'independent-root-tests.log')}
(HERE/'independent-review.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'passed':True,'cases':249,'tests':2,'sha256':digest(HERE/'independent-review.json')}))
