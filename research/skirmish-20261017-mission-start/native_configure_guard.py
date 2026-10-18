"""After-KINGDOMS native4FCE05..31 ConfigureGWorld gate.

Executes actual59E520/511860 against the SAVED source Header90 Rules mode,
with an intentionally contradictory current Creator Rules mode. The reached
50E8B0 entry is a required boundary; no fake body return or completed mission.
Mode table payloads are the independently executed4981A0 constructor receipt.
"""
from pathlib import Path
import sys,json,itertools,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn.x86_const import *
ORIGINAL=h.P
H=lambda b:hashlib.sha256(b).hexdigest()
MODES=ROOT/'research/skirmish-20261016-load-zone-services/native-initial-zones.json'
class Required(Exception):pass
def one(image,pc,load_mode,header_mode):
 h.P=ORIGINAL if image=='original'else h.V;m=h.Machine(pc)
 table=m.alloc(7*4)
 for row in json.loads(MODES.read_text())['registration']:
  entry=m.alloc(12);payload=m.alloc(2);m.w32(entry+8,payload);m.w32(table+4*row['mode'],entry)
  m.u.mem_write(payload,bytes(row['payload']))
 m.w32(0xb3b198,table)
 world,resource,header,rules,creator=[m.alloc(n)for n in (0x300,0xb0,0x70,0x20,0x60)]
 m.w32(world+0x28,resource);m.w32(resource+0xa8,load_mode)
 m.w32(resource+0x90,header);m.w32(header+4,rules);m.w32(rules+4,header_mode)
 m.w32(resource+0x88,creator);m.w32(creator+0x24,6 if header_mode==0 else 0)
 reached=False
 def stop(_):
  nonlocal reached
  assert m.ecx()==creator+4;reached=True;raise Required()
 m.seams[0x50e8b0]=stop
 m.u.reg_write(UC_X86_REG_EDI,world);m.u.reg_write(UC_X86_REG_ESP,h.STACK)
 try:
  m.u.emu_start(0x4fce05,0x4fce31,count=1000)
  assert m.u.reg_read(UC_X86_REG_EIP)==0x4fce31
 except Required:pass
 expected=load_mode==1 and not json.loads(MODES.read_text())['registration'][header_mode]['payload'][0]
 assert reached==expected
 return {'required_50E8B0':reached,'creator_this_adjustment':4 if reached else None}
def main():
 rows=[]
 for image,pc,mode,saved in itertools.product(('original','v14'),h.PRECISION,(1,2,4),range(7)):
  rows.append({'image':image,'precision':pc,'input':{'load_mode':mode,'header_mode':saved},'output':one(image,pc,mode,saved)})
 doc={'schema':1,'scope':__doc__,'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,
  'producer_sha256':H(Path(__file__).read_bytes()),'mode_constructor_fixture_sha256':H(MODES.read_bytes()),
  'required_entry_not_body':'0x50e8b0','cases':rows,
  'executed_pins':[{'va':hex(a),'bytes':b}for a,b in sorted(h.VISITED.items())if a!=0x50e8b0]}
 p=HERE/'native-configure-guard.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'cases':len(rows),'pins':len(doc['executed_pins']),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
