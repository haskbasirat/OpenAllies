"""Execute original460000 element post-link validation and4C2310 storage."""
from pathlib import Path
import sys,json,itertools
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE))
from native_ground_definitions import Machine as Base,P,V,H
from unicorn.x86_const import *
class Machine(Base):
 def hook(self,u,pc,n,z):
  if pc==0x4bdba0:self.ret(self.element,pop=8)
  elif pc==0x410c40:self.failed=True;u.emu_stop()
  else:super().hook(u,pc,n,z)
 def bind(self,is_element,has_property,mobile,count):
  actor=self.definition;self.element=self.alloc(0x600);layout=self.alloc(0x20);definition=self.alloc(0x60);slots=self.alloc(0x20);array=self.alloc(8);prop=self.alloc(0x20)
  self.w(actor+0x2e8,definition);self.w(actor+0x2f0,layout);self.w(layout+8,definition);self.w(layout+12,array);self.w(layout+16,1);self.w(layout+20,2);self.w(definition+0x1c,slots);self.w(definition+0x20,1);self.w(slots+4,prop);self.w(slots+8,count)
  self.w(self.element+0x174,(int(is_element)<<8)|(int(mobile)<<19));
  if has_property:
   linked=self.alloc(8);self.w(linked,prop);self.w(self.element+0x1d4,linked)
  arg=self.alloc(4);self.w(arg,self.string(list(map(ord,'member'))));self.failed=False
  self.w(self.stack,self.stop);self.w(self.stack+4,arg);self.w(self.stack+8,0);self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.reg_write(UC_X86_REG_ECX,actor);self.u.emu_start(0x460000,self.stop,count=100000)
  return {'accepted':not self.failed,'stored_member':self.r(array)==self.element}
rows=[];visited={}
for inputs in itertools.product([False,True],[False,True],[False,True],[1,3]):
 m=Machine();out=m.bind(*inputs);assert out['accepted']==(inputs[0] and inputs[1] and(not inputs[2] or inputs[3]<=1)),(inputs,out);rows.append({'is_element':inputs[0],'has_property':inputs[1],'mobile_deployment':inputs[2],'multiplicity':inputs[3],**out});visited.update(m.visited)
spans=[]
for a,n in sorted(visited.items()):
 if not P.base<=a<P.base+P.image_size:continue
 if spans and spans[-1][1]>=a:spans[-1][1]=max(spans[-1][1],a+n)
 else:spans.append([a,a+n])
spans += [[0x466eb0,0x46707c],[0x461750,0x461765],[0x473ad0,0x473b00]]
pins=[]
for a,b in spans:
 raw=P.range(a,b-a);assert raw==V.range(a,b-a);pins.append({'start':hex(a),'end':hex(b),'sha256':H(raw)})
d={'passed':True,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':H(P.b),'v14_sha256':H(V.b),'cases':rows,'ranges':pins,'seams':['4BDBA0 actor definition lookup supplies resolved fixture identity. Original460000,4C2310 and45EEF0 membership check execute; native diagnostic stops invalid cases.','Organization category-list and HQ/research extras are absent in fixture. Scalar source Slot.element_IDS path/deferredcallback assignments are statically pinned. No unrelated definition side effects are claimed.']};(HERE/'native-layout-parameters.json').write_text(json.dumps(d,indent=2));print('passed',len(rows))
