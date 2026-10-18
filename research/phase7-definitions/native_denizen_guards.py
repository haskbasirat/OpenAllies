from pathlib import Path
import sys,json,struct
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE))
from probe_lists import Machine as Base
from native_ground_definitions import P,V,H,bits
from unicorn.x86_const import *
class Machine(Base):
 def hook(self,u,pc,n,z):
  if pc==0x5b5130:u.reg_write(UC_X86_REG_EIP,0x301e000)
  elif pc==0x410c40:self.failed=True;u.emu_stop()
  elif pc==0x404f60:self.ret()
  else:super().hook(u,pc,n,z)
 def ranges(self,cw,guard,operational,inventory):
  self.u.reg_write(UC_X86_REG_FPCW,cw);self.failed=False;p=self.alloc(0x5c0);self.w(p+0x364,guard);self.w(p+0x35c,0x2251000 if inventory else 0);self.w(0x225f000,operational);self.u.mem_write(0x301e000,b'\xd9\x05'+struct.pack('<I',0x225f000)+b'\xc2\x04\x00');self.u.reg_write(UC_X86_REG_EBP,p);self.u.reg_write(UC_X86_REG_EDI,0x2250000);self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.emu_start(0x4670c6,0x467145,count=10000)
  if self.failed:return None
  assert self.u.reg_read(UC_X86_REG_EIP)==0x467145
  self.u.emu_start(0x4676ad,0x4676e6,count=10000);assert self.u.reg_read(UC_X86_REG_EIP)==0x4676e6
  return [self.r(p+0x364),self.r(p+0x368)]
rows=[];visited={}
for pc,cw in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
 for guard in [-1.0,-0.0,0.0,2.0,10.0,16777216.0]:
  for operational in [-1.0,-0.0,guard,guard+1,guard+2]:
   for inventory in [False,True]:
    m=Machine();out=m.ranges(cw,bits(guard),bits(operational),inventory);rows.append({'precision':pc,'guard_bits':bits(guard),'operational_bits':bits(operational),'inventory_present':inventory,'output_bits':out});visited.update(m.visited)
spans=[]
for a,n in sorted(visited.items()):
 if not P.base<=a<P.base+P.image_size:continue
 if spans and spans[-1][1]>=a:spans[-1][1]=max(spans[-1][1],a+n)
 else:spans.append([a,a+n])
spans += [[0x4670c6,0x467145],[0x467350,0x4674c7],[0x4676ad,0x4676e6],[0x461770,0x461820],[0x463740,0x463800],[0x463800,0x463a60],[0x9fdd44,0x9fdd48]]
pins=[]
for a,b in spans:
 raw=P.range(a,b-a);assert raw==V.range(a,b-a);pins.append({'start':hex(a),'end':hex(b),'sha256':H(raw)})
d={'passed':True,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':H(P.b),'v14_sha256':H(V.b),'cases':rows,'ranges':pins,'seams':['Original4670C6..467145 default/validation arithmetic executes with typed operational-range getter and storedguard input. Diagnostics stop the invalid constructor projection.','Original4676AD..4676E6 executes with explicit pre-callback Inventory list presence; native empty-list diagnostic logging is a leaf. Only guard_range is cleared, not operational range.','Inventory list creation/case-sensitive merge and Organization/DenizenGroup deferred-reference loaders are statically pinned. This snapshot is not full DenizenData component/AI/Runway/Deployment loading.']};(HERE/'native-denizen-guards.json').write_text(json.dumps(d,indent=2));print('passed',len(rows),H((HERE/'native-denizen-guards.json').read_bytes()))
