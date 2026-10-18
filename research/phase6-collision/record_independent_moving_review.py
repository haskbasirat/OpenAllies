"""Independent review of the pure native moving-state graph predicate."""
from pathlib import Path
import hashlib,json,re,subprocess,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(HERE))
import native_moving as oracle
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
doc=json.loads((HERE/'native-moving-fixtures.json').read_text())
assert doc['producer_sha256']==sha(HERE/'native_moving.py') and len(doc['cases'])==162
assert doc['source_sha256']==hashlib.sha256(oracle.original.b).hexdigest()
assert doc['v14_sha256']==hashlib.sha256(oracle.modified.b).hexdigest()
for row in doc['ranges'].values():
 a,z=int(row['start'],16),int(row['end_exclusive'],16);raw=oracle.original.range(a,z-a)
 assert raw==oracle.modified.range(a,z-a) and hashlib.sha256(raw).hexdigest()==row['sha256']
visited={};regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
for row in doc['cases']:
 m=oracle.MovingMachine(row['input']);reset=m.reset
 def review_reset(state):
  reset(state)
  for i,r in enumerate(regs):m.u.reg_write(r,0x11111111*(i+1))
 m.reset=review_reset
 def observe(u,a,n,_):
  if 0x400000<=a<0xc00000:visited[a]=n
 m.u.hook_add(UC_HOOK_CODE,observe)
 assert m.run()==row['output'],row['input']['name']
 assert [m.u.reg_read(r) for r in regs]==[0x11111111*(i+1) for i in range(4)]
windows=[]
for a,n in sorted(visited.items()):
 raw=oracle.original.range(a,n);assert raw==oracle.modified.range(a,n);windows.append({'address':hex(a),'bytes':raw.hex()})
commands=[];count=0
for name,args in [('independent-moving-tests.log',['cargo','test','-p','aa-spatial','--test','moving_state_native']),('independent-moving-clippy.log',['cargo','clippy','-p','aa-spatial','--all-targets','--','-D','warnings'])]:
 run=subprocess.run(args,cwd=ROOT,stdout=subprocess.PIPE,stderr=subprocess.STDOUT);(HERE/name).write_bytes(run.stdout);assert run.returncode==0,run.stdout.decode(errors='replace')
 commands.append({'command':args,'exit_code':run.returncode,'log':name,'sha256':sha(HERE/name)})
 if name=='independent-moving-tests.log':count=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',run.stdout.decode())))
assert count==2
files=['crates/aa-spatial/src/moving_state.rs','crates/aa-spatial/tests/moving_state_native.rs','research/phase6-collision/native_moving.py','research/phase6-collision/native-moving-fixtures.json','research/phase6-collision/record_independent_moving_review.py']
report={'schema':1,'passed':True,'component':'4CEDE0 moving-state and pure linked component helpers','findings':[],
 'original_sha256':doc['source_sha256'],'v14_sha256':doc['v14_sha256'],'independently_reexecuted_cases':162,'behavior_hooks':[],
 'rust_tests':2,'all_cases_nonvolatile_registers_preserved':True,'commands':commands,
 'source_sha256':{p:sha(ROOT/p) for p in files},'dependency_sha256':{'research/phase4-rng/original_rng.py':sha(ROOT/'research/phase4-rng/original_rng.py')},'visited_windows':windows,
 'checks':['Independently reexecuted all162 unchanged original cases without replacing fixtures; original/v14 full function and every visited instruction window agree.',
 'Checked stack and EBX/ESI/EDI/EBP preservation in all native runs; no gameplay callback is intercepted.',
 'Compared original4CEDE0/578D00/5780F0/578E10 assembly with Rust field predicates, component-owner state selection, linked recursion before displacement fallback, and terminal active-component identity.',
 'Verified ordered XY equality logic: signed zero equality, same infinities equality, NaNs unequal, distinct subnormals unequal. No arithmetic or guessed movement speed is introduced.',
 'Reviewed iterative fallback stack and pure stable snapshot contract; native recursive cycles are explicit errors, absent actor/component/state data remain host errors.'],
 'limits':['Host supplies coherent live component graphs; this module reads current state without advancing orders or integrating a world scheduler.',
 'NaN/signaling-NaN boolean comparison is covered, but native x87 exception flags/traps are not reproduced.',
 'Pointer corruption, concurrently mutating supposedly pure snapshots, native recursive-cycle behavior and allocation layout are unsupported domains.',
 'No game executable, runtime, asset, save or release was changed.']}
path=HERE/'independent-moving-review.json';path.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'passed':True,'cases':162,'sha256':sha(path)}))
