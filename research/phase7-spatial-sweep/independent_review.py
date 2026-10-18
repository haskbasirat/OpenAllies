from pathlib import Path
import sys,json,hashlib,subprocess,time
ROOT=Path(__file__).resolve().parents[2];OUT=Path(__file__).parent
sys.path.insert(0,str(ROOT/'research/phase7-spatial'))
sys.path.insert(0,str(OUT))
from original_query import Query,original,modified
from original_sweep import Sweep
from unicorn.x86_const import UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP
REGS=[(UC_X86_REG_EBX,0x12345678),(UC_X86_REG_ESI,0x34567891),(UC_X86_REG_EDI,0x56789123),(UC_X86_REG_EBP,0x78912345)]
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
class Checks:
 def reset(self,*args,**kw):
  super().reset(*args,**kw)
  for reg,value in REGS:self.u.reg_write(reg,value)
 def check(self):
  assert all(self.u.reg_read(r)==v for r,v in REGS)
class ReviewQuery(Checks,Query):pass
class ReviewSweep(Checks,Sweep):pass

ledger=json.loads((OUT/'findings.json').read_text())
for field in ('source_sha256','dependency_sha256'):
 for name,digest in ledger[field].items():assert sha(ROOT/name)==digest,name
snapshot={name:sha(ROOT/name)for group in ('source_sha256','dependency_sha256')for name in ledger[group]}
queries=json.loads((ROOT/'research/phase7-spatial/native-query.json').read_text())
sweeps=json.loads((OUT/'native-sweep.json').read_text())
assert queries['producer_sha256']==sha(ROOT/'research/phase7-spatial/original_query.py')
assert sweeps['producer_sha256']==sha(OUT/'original_sweep.py')
visited={};qcount=0;scount=0;corridors=0
for row in reversed(queries['cases']):
 m=ReviewQuery(row['input'],{'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[row['precision']])
 assert m.prepared_initial==row['initial'] and m.geometry==row['geometry']
 for step in row['steps']:
  assert m.run(step['point'])==step['native'];m.check();qcount+=1
 visited.update(m.visited)
print(json.dumps({'query_steps':qcount}),flush=True)
for row in reversed(sweeps['cases']):
 m=ReviewSweep(row['input'],{'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[row['precision']])
 assert m.prepared_initial==row['initial'] and m.geometry==row['geometry']
 assert m.sweep(row['goal'])==row['output'];m.check();scount+=1
 corridors+=len(m.query_calls)==2;visited.update(m.visited)
for doc in (queries,sweeps):
 for pin in doc['executed_windows']:
  at=int(pin['va'],16);data=bytes.fromhex(pin['bytes']);assert original.range(at,len(data))==data==modified.range(at,len(data))
for at,n in visited.items():assert original.range(at,n)==modified.range(at,n)
for name,digest in snapshot.items():assert sha(ROOT/name)==digest,name
run=[(['cargo','test','-p','aa-spatial','query::tests','--','--nocapture'],'review-query-tests.log'),(['cargo','test','-p','aa-spatial','sweep::tests','--','--nocapture'],'review-sweep-tests.log'),(['cargo','clippy','-p','aa-spatial','--all-targets','--no-deps','--','-D','warnings'],'review-clippy.log')]
for cmd,name in run:
 r=subprocess.run(cmd,cwd=ROOT,text=True,capture_output=True);(OUT/name).write_text(r.stdout+r.stderr);assert r.returncode==0,(cmd,r.stdout+r.stderr)
asm={str(p.relative_to(ROOT)):sha(p)for p in OUT.glob('review-*.asm.txt')}
report=dict(schema=1,passed=True,assembly_pins=asm,reviewed_semantics=['553BE2 definition bit10 gates large-only before intrusive marks, distinct from actor flags.','553CE0 short-distance comparison retains extended deltas; corridor uses stored deltas and squared-distance store.','Endpoint setter/query/restore order and failure partial effects preserved.','Corridor constructor radius and asymmetric midpoint stores retained.','553090 reset versus explicit descriptor field copies verified.','Query mark cleanup retains outside pre-existing marks and clears only owned marks.'],reviewer='navigation',findings=[],scope='Independent query large-only source-definition repair and whole553CE0 endpoint/corridor composition review. Not a review of new554350 adapter or whole live map integration.',source_snapshot=snapshot,validation=dict(native_query_steps=qcount,native_sweep_cases=scount,actual_corridor_cases=corridors,nonvolatile_abi_assertions=qcount+scount,visited_instruction_windows=len(visited),commands=[cmd for cmd,_ in run]),review_files={str(p.relative_to(ROOT)):sha(p)for p in [Path(__file__),*[OUT/n for _,n in run]]},limits=ledger['limits'])
(OUT/'independent-review.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'passed':True,'sha256':sha(OUT/'independent-review.json'),'cases':qcount+scount}),flush=True)
