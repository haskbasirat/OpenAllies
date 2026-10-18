from pathlib import Path
import sys,json,struct
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE))
from probe_lists import Machine as Base
from native_ground_definitions import P,V,H
from unicorn.x86_const import *
class Machine(Base):
 def __init__(self):super().__init__();self.tags=[];self.enums=[]
 def hook(self,u,pc,n,z):
  if pc==0x4976b0:
   pointer=self.arg(1);name=P.range(pointer,120).decode('utf-16-le').split('\0')[0];self.enums.append({'kind':self.arg(),'name':name});self.ret(pop=12)
  elif pc==0x4bf990:
   assert self.arg()==0
   tag=self.r(self.arg(2)+0x14);self.tags.append(tag);definition=self.alloc(0x9c);self.w(definition+0xc,tag);self.w(definition+0x24,0x3f800000);self.w(u.reg_read(UC_X86_REG_ECX),definition);self.ret(pop=12)
  else:super().hook(u,pc,n,z)
 def faction(self,tags):
  base=self.node('Base',1);self.add(base,self.node('Ability',4));node=self.node('Faction',base=base)
  for tag in tags:self.add(node,self.node('Ability',16,ordinal=tag))
  self.invoke(0x5b57c0,base,[node,0])
  data=self.alloc(0x60);array=self.alloc(100);self.w(data+0x24,array);self.w(data+0x2c,25);self.w(data+8,self.string(list(map(ord,'faction'))))
  self.w(self.stack+0x6c,node);self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.reg_write(UC_X86_REG_EBP,node);self.u.reg_write(UC_X86_REG_EDI,data);self.u.reg_write(UC_X86_REG_EBX,0)
  self.u.emu_start(0x48bc25,0x48bd1c,count=1000000);assert self.u.reg_read(UC_X86_REG_EIP)==0x48bd1c
  count=self.r(data+0x28);records=[]
  for i in range(count):
   local=self.r(array+i*4);definition=self.r(local);records.append({'tag':self.r(definition+0xc),'range_bits':self.r(definition+0x24)})
  return records
rows=[];visited={}
for tags in [[],[9],[9,10],[2,2],[11,2,9,5],[0,1,2,3,4,5,6]]:
 m=Machine();out=m.faction(tags);assert m.tags==tags,(tags,m.tags,out);assert [v['tag']for v in out]==tags;assert all(v['range_bits']==0x7f7fffff for v in out);rows.append({'source_tags':tags,'loaded_tags':m.tags,'stored':out});visited.update(m.visited)
m=Machine();m.invoke(0x497af0,0x2240000,[]);enums=m.enums;visited.update(m.visited)
spans=[]
for a,n in sorted(visited.items()):
 if not P.base<=a<P.base+P.image_size:continue
 if spans and spans[-1][1]>=a:spans[-1][1]=max(spans[-1][1],a+n)
 else:spans.append([a,a+n])
spans += [[0x48bae0,0x48bfa1]]
pins=[]
for a,b in spans:
 raw=P.range(a,b-a);assert raw==V.range(a,b-a);pins.append({'start':hex(a),'end':hex(b),'sha256':H(raw)})
d={'passed':True,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':H(P.b),'v14_sha256':H(V.b),'faction_cases':rows,'effect_kinds':enums,'ranges':pins,'scope':'Constructor-input/table-order projection only; not completed special-operation ability use.','seams':['OriginalFaction48BC25 listcount/iteration, actualLocalAbility ctor4BE9B0, append and maxrange write execute. Full4BF990 definition load is an explicit pre-resolved identity seam here.','Original Effect enum initialization497AF0 executes; registration leaf captures names/kinds. Main power comes from separately executed original4BEE20 fixtures.','Rendering/UI/localization/FX/SUMMON execution not implemented by this constructor projection.']};(HERE/'native-faction-constructors.json').write_text(json.dumps(d,indent=2));print('passed',len(rows),len(enums),H((HERE/'native-faction-constructors.json').read_bytes()))
