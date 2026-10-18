"""Original4D919C caller dispatch order; component-body call is a recording seam."""
from pathlib import Path
import sys,struct,json,hashlib
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'research/phase6-actor-payloads'))
from native_actor_payload import TailMachine,P,V
from unicorn.x86_const import *
class Machine(TailMachine):
 def __init__(self):super().__init__(struct.pack('<II',0x7fffffff,0x7fffffff));self.calls=[]
 def hook(self,u,pc,size,data):
  if pc==0x4d8290:self.calls.append(self.arg(1));self.ret(pop=16);return
  super().hook(u,pc,size,data)
m=Machine();m.w(m.stack+0x20,14);m.w(m.stack+0xa8,m.world);m.u.reg_write(UC_X86_REG_ESP,m.stack);m.u.reg_write(UC_X86_REG_ESI,m.thing);m.u.reg_write(UC_X86_REG_EBX,0)
m.u.emu_start(0x4d919c,0x4d9237,count=10000);assert m.u.reg_read(UC_X86_REG_EIP)==0x4d9237
raw=P.range(0x4d919c,0x4d9237-0x4d919c);assert raw==V.range(0x4d919c,len(raw))
out={'scope':__doc__,'calls':m.calls,'bytes_read':m.at,'source_range_sha256':hashlib.sha256(raw).hexdigest(),'original_sha256':hashlib.sha256(P.b).hexdigest()}
p=Path(__file__).with_name('native-component-order.json');p.write_text(json.dumps(out,indent=2)+'\n');print(out)
