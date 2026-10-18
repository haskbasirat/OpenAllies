"""Original4E85C1..4E86C3 Kingdom fields/resources prefix. Executes original4CDC70/4CDBB0 and clamp comparisons, with explicit resolved registry lookup and localization allocation seams. Input registry ordinal mapping is r0=0,r1=1,r2=2."""
from pathlib import Path
import sys,json,struct,hashlib
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'research/phase6-kingdoms'))
from native_kingdoms import Machine as Base,P,V,VISITED
from unicorn.x86_const import *
W=lambda *v:struct.pack('<'+'I'*len(v),*v)
S=lambda v:struct.pack('<H',len(v))+v.encode('utf-16le')
class Machine(Base):
 def __init__(self,payload,maximum):
  super().__init__(payload,28);self.r.at=4
  data=self.alloc(0x400);self.write(0xb36530,data);self.write(data+0x38,3)
  self.color_registry=self.alloc(8);self.resources_registry=self.alloc(8);self.write(0xb3aa60,self.color_registry);self.write(0xb3a660,self.resources_registry);self.color=self.alloc(16)
  self.resources={}
  for i in range(3):p=self.alloc(16);self.write(p+12,i);self.resources['r'+str(i)]=p
  settings=self.alloc(0x400);self.write(0xb39ab0,settings);v=self.alloc(4);self.write(v,maximum);self.write(settings+0x1b8,v)
  for off,capacity in [(0x1d0,32),(0x1c4,8)]:self.write(self.object+off,self.alloc(capacity*4));self.write(self.object+off+4,0);self.write(self.object+off+8,capacity)
 def hook(self,u,pc,size,arg):
  if pc in (0x4e7f80,0x4cdbb0,0x45f180,0x4849f0):
   raw=P.range(pc,size);assert raw==V.range(pc,size);VISITED[pc]=raw.hex();return
  if pc==0x48d500:
   p=self.arg();n=self.read(p-12);key=bytes(u.mem_read(p,n*2)).decode('utf16');registry=u.reg_read(UC_X86_REG_ECX)
   self.ret(4,value=self.color if registry==self.color_registry else self.resources.get(key,0));return
  super().hook(u,pc,size,arg)
 def run(self):
  self.write(self.stack+0x14,0);self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.reg_write(UC_X86_REG_EBP,self.object);self.u.reg_write(UC_X86_REG_EDI,self.world);self.u.reg_write(UC_X86_REG_EBX,0)
  self.u.emu_start(0x4e85c1,0x4e86c3,count=100000);assert self.u.reg_read(UC_X86_REG_EIP)==0x4e86c3 and self.r.at==len(self.r.b)
  def vec(off):return [self.read(self.read(self.object+off)+i*4)for i in range(self.read(self.object+off+4))]
  return {'words':[self.read(self.object+o)for o in [0x188,0x18c,0x190]],'relation_values':vec(0x1d0),'resource_values':vec(0x1c4),'color_is_resolved':self.read(self.object+0x1e8)==self.color,'consumed':self.r.at}
rows=[]
values=[0,0x80000000,0xbf800000,0x3f800000,0x47c34f80,0x4f000000,0x7f800000,0xff800000,0x7fc01234,0x7f801234]
for maximum in [0,0xbf800000,0x3f800000,0x47c34f80,0x7f800000,0x7fc01234]:
 for value in values:
  pairs=[('r2',value),('r0',0x3e800000),('r1',0x40800000),('r2',value)]
  payload=W(0,1,2,3)+S('kingdom_fixture')+S('blue')+W(4,0,3,2,1)+W(len(pairs))+b''.join(S(k)+W(v)for k,v in pairs)
  m=Machine(payload,maximum);native=m.run();rows.append({'payload_hex':payload.hex(),'maximum_bits':maximum,'native':native})
p=Path(__file__).with_name('native-kingdom-prefix.json');p.write_text(json.dumps({'scope':__doc__,'original_sha256':hashlib.sha256(P.b).hexdigest(),'v14_sha256':hashlib.sha256(V.b).hexdigest(),'cases':rows,'instructions':[{'va':hex(a),'bytes':b}for a,b in sorted(VISITED.items())]},indent=2)+'\n');print({'cases':len(rows),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()})
