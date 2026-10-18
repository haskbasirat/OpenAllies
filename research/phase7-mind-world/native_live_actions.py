"""Actual4D1350 /571640 /530510 /4CFCC0 from five-faction source projections.
Actor+34=4D1F90 and Properties+1C=4CF110 execute directly; no command/action,
modifier, or publication semantic hook. Unsupported linked regiment/UI branches
are absent by the explicit input, not converted into success.
"""
from pathlib import Path
import sys,json,hashlib,copy
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase7-mind'))
from native_action_mask import MaskMachine,cases,ap,cp,pl,mp,VT,original,modified
from native_lifecycle import LifecycleMachine
from unicorn.x86_const import *
sha=lambda b:hashlib.sha256(b).hexdigest()
class Native(MaskMachine):
 def __init__(self,row):
  c=copy.deepcopy(next(cases()));c.update(mutations=[])
  c['actors']={'101':{'flags':row['flags'],'element':201,'organization':None,'performer':301,'mover':True,'component9c':False,'word108':0,'word118':0,'componentac':None,'action_bits':0xdeadbeef55555555}}
  c['elements']={'201':None};c['organizations']={};c['abilities']={'301':row['abilities']};c['players']={'401':row['modifier']}
  super().__init__(c)
  self.w(ap(101)+0x70,mp(1));self.w(ap(101)+0x78,pl(401)-0x1c)
  self.w(cp(201)+4,ap(101));self.w(cp(201)+0x14,0);self.w(VT+0xb4,0x530510);self.w(VT+0x34,0x4d1f90);self.w(VT+0x108,0x4d1e80)
 def hook(self,u,pc,size,data):
  # Bypass every MaskMachine semantic hook. Only reusable memory allocator
  # hooks remain, and none is reached in this chain.
  LifecycleMachine.hook(self,u,pc,size,data)
 def run(self):
  self.reset(0);self.u.reg_write(UC_X86_REG_ECX,ap(101));regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
  for i,r in enumerate(regs):self.u.reg_write(r,0x11111111*(i+1))
  self.u.emu_start(0x4d1350,self.STOP,count=10000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+4
  assert[self.u.reg_read(r)for r in regs]==[0x11111111*(i+1)for i in range(4)]
  assert not self.events
  return self.r(ap(101)+0xf0)|(self.r(ap(101)+0xf4)<<32)
def main():
 inputs=json.loads((HERE/'live-action-inputs.json').read_text());rows=[];visited={}
 for row in inputs:
  m=Native(row);rows.append({'input':row,'actions':m.run()});visited.update(m.visited)
 for a,b in visited.items():assert b==original.range(a,len(b))==modified.range(a,len(b)),hex(a)
 disabled=[]
 for flags in [0,1,1<<29,0xffffffff]:
  for bits in [0,1<<25,1<<26,1<<34,1<<36,1<<33,(1<<25)|(1<<34),0xffffffffffffffff]:
   m=Native(inputs[0]);m.w(ap(101)+8,flags);m.w(ap(101)+0x100,bits&0xffffffff);m.w(ap(101)+0x104,bits>>32)
   m.reset(0);m.u.reg_write(UC_X86_REG_ECX,ap(101));m.u.emu_start(0x4d12c0,m.STOP,count=1000)
   assert m.u.reg_read(UC_X86_REG_ESP)==m.STACK+4
   disabled.append({'flags':flags,'state':bits,'result':bool(m.u.reg_read(UC_X86_REG_EAX)&255)});visited.update(m.visited)
 for a,b in visited.items():assert b==original.range(a,len(b))==modified.range(a,len(b)),hex(a)
 d={'method':__doc__,'original_sha256':sha(original.b),'v14_sha256':sha(modified.b),'producer_sha256':sha(Path(__file__).read_bytes()),'source_projection_sha256':sha((HERE/'live-action-inputs.json').read_bytes()),'instructions':[{'va':hex(a),'bytes':b.hex()}for a,b in sorted(visited.items())],'cases':rows,'disabled':disabled}
 p=HERE/'native-live-actions.json';p.write_text(json.dumps(d,indent=2));print(json.dumps({'cases':len(rows),'sha256':sha(p.read_bytes())}))
if __name__=='__main__':main()
