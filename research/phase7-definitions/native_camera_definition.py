from pathlib import Path
import sys,json,struct
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE))
from probe_lists import Machine as Base
from native_ground_definitions import P,V,H,bits
from unicorn.x86_const import *
class Machine(Base):
 def __init__(self,flags,distance):
  super().__init__();self.flags=flags;self.distance=distance;self.u.mem_write(0x301e000,b'\xd9\x05'+struct.pack('<I',0x225f000)+b'\xc2\x04\x00');self.w(0x225f000,distance)
 def hook(self,u,pc,n,z):
  if pc==0x49dc40:self.w(u.reg_read(UC_X86_REG_ECX)+0x1c,self.flags);self.ret(pop=4)
  elif pc==0x5b54d0:p=self.alloc(4);self.w(p,self.string(list(map(ord,'camera_vfx'))));self.ret(p,pop=4)
  elif pc==0x4bdb70:self.ret(0x2250100,pop=4)
  elif pc==0x5b4ae0:u.reg_write(UC_X86_REG_EIP,0x301e000)
  else:super().hook(u,pc,n,z)
rows=[];visited={}
for flags in [0,0xa5a5a5a5,0xffffffff]:
 for distance in [0,bits(2.5),bits(-1),0x80000000]:
  m=Machine(flags,distance);data=m.alloc(0x178);m.invoke(0x49f1e0,data,[0]);out={'flags':m.r(data+0x1c),'distance':m.r(data+0x174),'vfx_identity':m.r(data+0x170)};assert out=={'flags':flags,'distance':distance,'vfx_identity':0x2250100};rows.append({'base_flags':flags,'distance':distance,'output':out});visited.update(m.visited)
spans=[]
for a,n in sorted(visited.items()):
 if not P.base<=a<P.base+P.image_size:continue
 if spans and spans[-1][1]>=a:spans[-1][1]=max(spans[-1][1],a+n)
 else:spans.append([a,a+n])
spans += [[0x49cc80,0x49ccc0],[0x49f1e0,0x49f29f],[0x4bdb70,0x4bdba0]]
pins=[]
for a,b in spans:
 raw=P.range(a,b-a);assert raw==V.range(a,b-a);pins.append({'start':hex(a),'end':hex(b),'sha256':H(raw)})
d={'passed':True,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':H(P.b),'v14_sha256':H(V.b),'cases':rows,'ranges':pins,'seams':['Whole derived CameraData49F1E0 executes; base ObjectData49DC40 flag marker and typed Float/string getters are explicit seams.','4BDB70 successful VFX identity is pre-resolved; this fixture/source snapshot does not claim rendered VFX loading.','Derived loader stores identity+170 and Float+174 and leaves base flags unchanged.']};(HERE/'native-camera-definition.json').write_text(json.dumps(d,indent=2));print('passed',len(rows),H((HERE/'native-camera-definition.json').read_bytes()))
