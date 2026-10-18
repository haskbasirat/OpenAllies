"""Independent immutable-fixture replay and source review for cell collision."""
from pathlib import Path
import hashlib,json,re,struct,subprocess,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(HERE))
import native_collision as oracle
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
doc=json.loads((HERE/'native-collision-fixtures.json').read_text())
assert doc['source_sha256']==hashlib.sha256(oracle.original.b).hexdigest()
assert doc['v14_sha256']==hashlib.sha256(oracle.modified.b).hexdigest()
assert doc['producer_sha256']==sha(HERE/'native_collision.py')
assert len(doc['cases'])==399
for row in doc['ranges'].values():
 a,z=int(row['start'],16),int(row['end_exclusive'],16);raw=oracle.original.range(a,z-a)
 assert raw==oracle.modified.range(a,z-a) and hashlib.sha256(raw).hexdigest()==row['sha256']
constants={}
for address in [0xa033bc,0x9fbdb8,0x9fd59c,0x9fc0bc]:
 raw=oracle.original.range(address,4);assert raw==oracle.modified.range(address,4)
 constants[hex(address)]=struct.unpack('<I',raw)[0]
assert constants=={'0xa033bc':0x3e800000,'0x9fbdb8':0x3f000000,'0x9fd59c':0x40800000,'0x9fc0bc':0x38d1b717}
visited={};regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
for row in doc['cases']:
 machine=oracle.CollisionMachine(row['input'])
 reset=machine.reset
 def review_reset(state):
  reset(state)
  for i,r in enumerate(regs):machine.u.reg_write(r,0x11111111*(i+1))
 machine.reset=review_reset
 def observe(u,a,size,_):
  if 0x400000<=a<0xc00000:visited[a]=size
 machine.u.hook_add(UC_HOOK_CODE,observe)
 assert machine.run()==row['output'],row['input']['name']
 assert [machine.u.reg_read(r) for r in regs]==[0x11111111*(i+1) for i in range(4)],row['input']['name']
windows=[]
for a,n in sorted(visited.items()):
 raw=oracle.original.range(a,n);assert raw==oracle.modified.range(a,n)
 windows.append({'address':hex(a),'bytes':raw.hex()})
commands=[];count=0
for filename,args in [('independent-tests.log',['cargo','test','-p','aa-spatial']),('independent-clippy.log',['cargo','clippy','-p','aa-spatial','--all-targets','--','-D','warnings'])]:
 run=subprocess.run(args,cwd=ROOT,stdout=subprocess.PIPE,stderr=subprocess.STDOUT);(HERE/filename).write_bytes(run.stdout)
 assert run.returncode==0,run.stdout.decode(errors='replace')
 commands.append({'command':args,'exit_code':run.returncode,'log':filename,'sha256':sha(HERE/filename)})
 if filename=='independent-tests.log':count=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',run.stdout.decode())))
files=['crates/aa-spatial/src/collision.rs','crates/aa-spatial/tests/collision_native.rs','research/phase6-collision/native_collision.py','research/phase6-collision/native-collision-fixtures.json','research/phase6-collision/record_independent_review.py']
report={'schema':1,'passed':True,'component':'Dynamic tile scan, descriptor filter, and actual circle-probe geometry',
 'findings':[],'original_sha256':doc['source_sha256'],'v14_sha256':doc['v14_sha256'],'reexecuted_native_cases':399,
 'all_cases_nonvolatile_registers_preserved':True,'entire_spatial_crate_tests':count,'commands':commands,
 'source_sha256':{p:sha(ROOT/p) for p in files},'dependency_sha256':{p:sha(ROOT/p) for p in ['crates/aa-numeric/src/lib.rs','research/phase4-rng/original_rng.py']},
 'verified_constants':constants,'visited_windows':windows,
 'checks':['Independently reexecuted all399 original cases without overwriting fixtures, checked corpus producer/source pins and original/v14 code ranges and constants.',
 'Added observational original/v14 verification for every visited code window, and independently checked EBX/ESI/EDI/EBP preservation alongside the existing stack check.',
 'Reviewed descriptor callback order, cached hostility dispatch identity with fresh receiver, repeated allegiance and outer virtuals, and post-callback node.next reads.',
 'Reviewed native grid shift/truncation and separate halfcell store, cached radius eligibility integer, and placement snapshot before the virtual shape getter.',
 'Reviewed shape ordering and transforms, circle epsilon, binary32 coordinate stores, inclusive box interior and strict exterior contact.',
 'Verified the generation lookup adapter is an explicit required host boundary; supplied native registry entries are fixture data, not a proof of complete world binding.'],
 'limits':['Grid/settings are stable during a query; arbitrary live map replacement is outside this API.',
 'Virtual radius, shape, custom/source/category/allegiance/hostility callbacks and moving-exclusion4CEDE0 are explicit fixture leaves. A separate future moving-state module is not included in this review.',
 'Only point/circle/box against the circular cell probe is implemented. General box-box SAT is not claimed.',
 'Finite round-to-nearest precision is explicit; native FPU exception state, NaNs and native invalid pointer/cyclic linked-list behavior are outside tested parity.',
 'World errors can follow mutations and are not transactionally retried; missing world data must remain an error.',
 'Visited-window pins include intercepted seam-entry bytes and do not prove execution behind those seams.',
 'No game executable, runtime, asset, save or release file was changed.']}
path=HERE/'independent-review.json';path.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'passed':True,'cases':399,'spatial_tests':count,'sha256':sha(path)}))
