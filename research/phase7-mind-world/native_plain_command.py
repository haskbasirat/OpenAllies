"""Original4D9CE0 copy with parameter-object absent; no semantic hooks."""
from pathlib import Path
import json,hashlib,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine,original,modified
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
sha=lambda b:hashlib.sha256(b).hexdigest()
def main():
 m=Machine();m.u.mem_map(0,4096);m.u.mem_map(0x4000000,4096);visited={};rows=[]
 def hook(u,pc,size,_):visited[pc]=bytes(u.mem_read(pc,size))
 m.u.hook_add(UC_HOOK_CODE,hook)
 seed=0x9e3779b9
 for case in range(32):
  src,dst=0x4000000,0x4000100;m.u.mem_write(src,bytes(64));m.u.mem_write(dst,b'\xcd'*64);words=[]
  for off in range(8,0x30,4):
   seed=(seed*1664525+1013904223)&0xffffffff;words.append(seed);m.w(src+off,seed)
  m.w(src+0x38,seed^0xa5a5a5a5);m.u.mem_write(src+0x30,bytes([case%2]));m.u.mem_write(src+0x3c,bytes([(case>>1)%2]));m.w(src+4,17)
  m.reset(0);m.u.reg_write(UC_X86_REG_ECX,dst);m.w(m.STACK+4,src);regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
  for i,r in enumerate(regs):m.u.reg_write(r,0x11111111*(i+1))
  m.u.emu_start(0x4d9ce0,m.STOP,count=1000)
  assert m.u.reg_read(UC_X86_REG_ESP)==m.STACK+8
  assert [m.u.reg_read(r)for r in regs]==[0x11111111*(i+1)for i in range(4)]
  assert m.r(dst+4)==0 and m.r(src+4)==17 and m.r(dst+0x34)==0
  copied=[m.r(dst+off)for off in range(8,0x30,4)]
  assert words==copied
  rows.append({'words':words,'extra':m.r(src+0x38),'boolean':bool(case%2),'trailing':bool((case>>1)%2),'copied':copied,'extra_copied':m.r(dst+0x38),'boolean_copied':bool(m.u.mem_read(dst+0x30,1)[0]),'trailing_copied':bool(m.u.mem_read(dst+0x3c,1)[0])})
 for a,raw in visited.items():assert raw==original.range(a,len(raw))==modified.range(a,len(raw))
 moving=[]
 for kind,entry,offset in [('element',0x535e40,0x88),('regiment',0x5247b0,0x74),('aircraft',0x516ac0,0x40)]:
  for status in [0,1,2,3,17,0xffffffff]:
   m.w(src+offset,status);m.reset(0);m.u.reg_write(UC_X86_REG_ECX,src);m.u.emu_start(entry,m.STOP,count=100)
   assert m.u.reg_read(UC_X86_REG_ESP)==m.STACK+4
   moving.append({'kind':kind,'status':status,'result':bool(m.u.reg_read(UC_X86_REG_EAX)&255)})
 for a,raw in visited.items():assert raw==original.range(a,len(raw))==modified.range(a,len(raw))
 doc={'original_sha256':sha(original.b),'v14_sha256':sha(modified.b),'producer_sha256':sha(Path(__file__).read_bytes()),'instructions':[{'va':hex(a),'bytes':raw.hex()}for a,raw in sorted(visited.items())],'cases':rows,'moving':moving}
 p=HERE/'native-plain-command.json';p.write_text(json.dumps(doc,indent=2));print(json.dumps({'cases':len(rows),'sha256':sha(p.read_bytes())}))
if __name__=='__main__':main()
