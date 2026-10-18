from pathlib import Path
import sys,json
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE))
from probe_lists import Machine as Base
from native_ground_definitions import P,V,H
from unicorn.x86_const import *
class Machine(Base):
 def hook(self,u,pc,n,z):
  if pc==0x5b54d0:
   self.tag=self.r(u.reg_read(UC_X86_REG_ECX)+0x14);self.ret(0x2250000,pop=4)
  elif pc==0x405b80 and self.r(u.reg_read(UC_X86_REG_ESP))==0x48ca31:self.ret(pop=4)
  elif pc==0x405d60:self.ret()
  elif pc==0x4763c0:self.ret(pop=4)
  elif pc==0x5a77e0:self.ret(self.tag)
  else:super().hook(u,pc,n,z)
 def badges(self,tags):
  base=self.node('Base',1);self.add(base,self.node('Badge',4));node=self.node('Nation',base=base)
  for tag in tags:self.add(node,self.node('Badge',16,ordinal=tag))
  self.invoke(0x5b57c0,base,[node,0])
  data=self.alloc(0x80);array=self.alloc(256);self.w(data+0x3c,array);self.w(data+0x44,64)
  self.w(self.stack+0x18,data);self.w(self.stack+0xb8,node)
  self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.reg_write(UC_X86_REG_EBP,node);self.u.reg_write(UC_X86_REG_EBX,data)
  self.u.emu_start(0x48c9d9,0x48cb51,count=1000000);assert self.u.reg_read(UC_X86_REG_EIP)==0x48cb51
  return [self.r(array+4*i)for i in range(self.r(data+0x40))]
rows=[];visited={}
for tags in [[],[9],[9,3],[2,2],[11,2,9,5],list(range(30))]:
 m=Machine();out=m.badges(tags);assert out==tags,(tags,out);rows.append({'source_tags':tags,'stored':out});visited.update(m.visited)
spans=[]
for a,n in sorted(visited.items()):
 if not P.base<=a<P.base+P.image_size:continue
 if spans and spans[-1][1]>=a:spans[-1][1]=max(spans[-1][1],a+n)
 else:spans.append([a,a+n])
spans += [[0x48c050,0x48cd80]]
pins=[]
for a,b in spans:
 raw=P.range(a,b-a);assert raw==V.range(a,b-a);pins.append({'start':hex(a),'end':hex(b),'sha256':H(raw)})
d={'passed':True,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':H(P.b),'v14_sha256':H(V.b),'cases':rows,'ranges':pins,'scope':'Native NationData ordered Badge resource array at+3C, count+40; this is not a technology list.','seams':['Native48C9D9 count/iteration and append store execute. Array capacity is preallocated; reallocation not required.','5B54D0 field getter, CString formatting, resource-name translation and texture resource load are tagged seams; no texture decode or resource-existence assertion is implied.','Template/list copy and iterator helpers execute original code.']};(HERE/'native-nation-badges.json').write_text(json.dumps(d,indent=2));print('passed',len(rows),H((HERE/'native-nation-badges.json').read_bytes()))
