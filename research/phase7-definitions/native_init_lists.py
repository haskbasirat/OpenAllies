from pathlib import Path
import sys,json
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE))
from probe_lists import Machine as Base
from native_ground_definitions import P,V,H
from unicorn.x86_const import *
class Machine(Base):
 def __init__(self):super().__init__();self.technology=0
 def hook(self,u,pc,n,z):
  if pc==0x5b54d0:
   tag=self.r(u.reg_read(UC_X86_REG_ECX)+0x14);p=self.string(list(map(ord,str(tag))));r=self.alloc(4);self.w(r,p);self.ret(r,pop=4)
  elif pc==0x463fa0:
   p=self.arg();self.ret(int(bytes(u.mem_read(p,self.r(p-12)*2)).decode('utf-16-le'))+0x20000,pop=4)
  elif pc==0x4c42d0:self.w(u.reg_read(UC_X86_REG_ECX)+0x24,0xa5a5a5a5);self.ret(pop=4)
  elif pc==0x5b5380:self.ret(self.technology,pop=4)
  else:super().hook(u,pc,n,z)
 def property_list(self,tags):
  base=self.node('Base',1);self.add(base,self.node('Property',4));node=self.node('PropertiesComponent',base=base)
  for tag in tags:self.add(node,self.node('Property',16,ordinal=tag))
  self.invoke(0x5b57c0,base,[node,0]);name=self.alloc(4);self.w(name,self.string(list(map(ord,'Property'))));head=self.alloc(8)
  self.w(self.stack,self.stop)
  for i,value in enumerate([node,name,head]):self.w(self.stack+4+4*i,value)
  self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.emu_start(0x4c4cf0,self.stop,count=1000000);assert self.u.reg_read(UC_X86_REG_EIP)==self.stop
  values=[];p=self.r(head)
  while p:values.append(self.r(p)-0x20000);p=self.r(p+4)
  return values
rows=[];visited={}
for tags in [[],[9],[9,1],[1,1],[9,3,5,3],list(range(20))]:
 m=Machine();out=m.property_list(tags);assert out==list(reversed(tags));rows.append({'authored':tags,'native':out});visited.update(m.visited)
tech=[]
for flag in [0,1]:
 m=Machine();m.technology=flag;data=m.alloc(0x60);m.invoke(0x4c48c0,data,[0]);out={'input':flag,'technology':bytes(m.u.mem_read(data+0x5c,1))[0],'target_flags':m.r(data+0x24)};assert out['technology']==flag and out['target_flags']==0xa5a5a5a5;tech.append(out);visited.update(m.visited)
spans=[]
for a,n in sorted(visited.items()):
 if not P.base<=a<P.base+P.image_size:continue
 if spans and spans[-1][1]>=a:spans[-1][1]=max(spans[-1][1],a+n)
 else:spans.append([a,a+n])
# Static source caller/field assignments and original list append implementation.
spans += [[0x464d13,0x464d8a],[0x465089,0x4650cc],[0x46545c,0x46547a],[0x4654d4,0x4655e6],[0x465f79,0x4660cf],[0x4c48c0,0x4c48e2],[0x4c4030,0x4c4190],[0x42f7d0,0x42f850],[0x4625ee,0x462670]]
pins=[]
for a,b in spans:
 raw=P.range(a,b-a);assert raw==V.range(a,b-a);pins.append({'start':hex(a),'end':hex(b),'sha256':H(raw)})
d={'passed':True,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':H(P.b),'v14_sha256':H(V.b),'property_lists':rows,'technology_cases':tech,'ranges':pins,'seams':['Whole4C4CF0 executes native list counting/iteration and prepend allocation; field getter and shared Property lookup are pre-resolved tagged leaves.','Whole SharedProperty4C48C0 executes; base4C42D0 writes an unrelated target-flags marker and bool getter supplies technology. Technology is separate byte+5C, not target flags+24.','Numbering/trailer/mana/turret-count caller assignments are statically pinned; existing typed TGI scalar/string/point proofs bind their source values. No whole Actor InitCreate execution is claimed by this definition projection.']};(HERE/'native-init-lists.json').write_text(json.dumps(d,indent=2));print('passed',len(rows),len(tech),H((HERE/'native-init-lists.json').read_bytes()))
