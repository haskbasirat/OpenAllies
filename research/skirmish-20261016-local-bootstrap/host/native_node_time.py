"""Isolated native NetNode constructor field stores and765410 time prefix.
Not the whole constructor/frame.59ABA0 is injected OS-derived milliseconds;
its integer kernel and real QPC ownership have separate receipts. FPU case
initialization resets status/tags explicitly between independent runs.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parent))
from native_bootstrap import BootstrapMachine,PE0,PE1,PC,NETNODE,proof_ranges
from unicorn.x86_const import *
class TimeMachine(BootstrapMachine):
 def hook(self,u,pc,size,user):
  if getattr(self,'bootstrap_ready',False)and pc==0x59aba0:
   self.ret(value=self.milliseconds);return
  super().hook(u,pc,size,user)
def main():
 rows=[];constructors=[];visited={}
 for image in ('original','v14'):
  m=TimeMachine(image)
  m.u.reg_write(UC_X86_REG_ESI,NETNODE)
  m.u.mem_write(NETNODE+0x58,b'\xa5'*4);m.u.mem_write(NETNODE+0x64,b'\xa5')
  for a,b in [(0x765c51,0x765c58),(0x765d05,0x765d09)]:
   m.u.emu_start(a,b,count=10);assert m.u.reg_read(UC_X86_REG_EIP)==b
  constructors.append({'image':image,'cached58':m.r(NETNODE+0x58),'behind64':m.rb(NETNODE+0x64)})
  assert m.r(NETNODE+0x58)==0x5368d4a5 and m.rb(NETNODE+0x64)==0
  for p in PC:
   for ms in (0,1,999,1000,16777217,0x7fffffff,0x80000001,0xffffffff):
    m.fpu(p);m.u.reg_write(UC_X86_REG_FPSW,0);m.u.reg_write(UC_X86_REG_FPTAG,0xffff)
    m.milliseconds=ms;m.w(m.stack,m.stop);m.w(m.stack+4,NETNODE);m.w(m.stack+8,0);m.u.reg_write(UC_X86_REG_ESP,m.stack)
    m.u.emu_start(0x765410,0x765452,count=100)
    assert m.u.reg_read(UC_X86_REG_EIP)==0x765452
    rows.append({'image':image,'precision':p,'milliseconds':ms,'cached58':m.r(NETNODE+0x58)})
  visited.update(m.visited)
 a=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original'];b=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14'];assert a==b
 out={'passed':True,'scope':'Exact independent constructor stores765C51..58/765D05..09 and first765410..765452 cached-time write, not full Node ctor or frame','producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'original_sha256':hashlib.sha256(PE0.b).hexdigest(),'v14_sha256':hashlib.sha256(PE1.b).hexdigest(),'constructors':constructors,'cases':rows,'executed_windows':proof_ranges(visited)}
 (HERE/'native-node-time.json').write_text(json.dumps(out,indent=2)+'\n');print('passed',len(rows),'time rows')
if __name__=='__main__':main()
