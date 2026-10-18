"""Full original4CFCC0 and529D70 caller ordering with observed live services."""
from pathlib import Path
import sys,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase7-actors'))
from actor_states import Machine as Base,P,V,H
from unicorn.x86_const import *
class Machine(Base):
 def __init__(self,c):
  super().__init__();self.c=c;self.events=[];self.owner_calls=0
  self.parent=0x20d0000;self.org=0x20d1000;self.org_owner=0x20d2000;self.elem=0x20d3000;self.renderer=0x20d4000;self.vtable=0x20d5000
  self.w(self.thing,self.vtable);self.w(self.vtable+0x108,0x301fa00)
  self.w(self.thing+0x80,self.elem if c['element'] else 0);self.w(self.elem+0x10,self.parent if c['parent'] else 0)
  self.w(self.parent+0x7c,self.org);self.w(self.org+4,self.org_owner)
  self.set_byte(c['byte']);self.set_renderer(c['renderer'])
  self.w(0xb36534,self.world);self.w(self.world+0x130,0x20d7000);self.w(0xb36550,0x20d8000)
 def set_byte(self,b):self.u.mem_write(self.thing+0x4c,bytes([b]))
 def set_renderer(self,v):
  self.w(self.thing+0xc,0 if v is None else self.renderer)
  if v is not None:self.w(self.renderer+0x54,v)
 def hook(self,u,pc,size,user):
  if P.base<=pc<P.base+P.image_size:self.observed[pc]=bytes(u.mem_read(pc,size))
  if pc==0x4d1350:
   assert u.reg_read(UC_X86_REG_ECX)==self.org_owner;self.events.append(['rebuild',3])
   if self.c['parent_mutates']:self.set_byte(self.c['after_parent_byte'])
   self.ret()
  elif pc==0x301fa00:
   self.events.append(['owner',1]);v=self.c['owners'][self.owner_calls];self.owner_calls+=1
   if self.owner_calls==1 and self.c['owner_mutates']:self.set_byte(self.c['after_owner_byte'])
   self.ret(0 if v is None else 0x20da000+v*0x100)
  elif pc==0x76c7f0:
   v=self.arg();self.events.append(['owner_views',None if v==0 else (v-0x20da000)//0x100])
   if self.c['ui_mutates']:self.set_renderer(self.c['after_ui_renderer'])
   self.ret(pop=4)
  elif pc==0x50c8e0:assert self.arg()==self.thing;self.events.append(['selected',1]);self.ret(pop=4)
  else:super().hook(u,pc,size,user)
 def run(self):
  self.invoke(0x4cfcc0,self.thing)
  return {'events':self.events,'byte':self.u.mem_read(self.thing+0x4c,1)[0],'renderer':None if self.r(self.thing+0xc)==0 else self.r(self.renderer+0x54)}
def main():
 rows=[];observed={}
 for i in range(96):
  c={'element':i%3!=0,'parent':i%4!=0,'byte':[0,1,127,128,255][i%5],'renderer':[None,0,1<<17,0xffffffff][i%4], 'owners':[[None,None],[7,None],[7,8],[7,7]][i%4], 'parent_mutates':i%6==0,'after_parent_byte':1 if i%2 else 255,'owner_mutates':i%7==0,'after_owner_byte':1 if i%2 else 128,'ui_mutates':i%3==0,'after_ui_renderer':[None,0,1<<17][i%3]}
  m=Machine(c);rows.append({'input':c,'output':m.run()});observed.update(m.observed)
 # Force callback mutation paths independently of patterned combinations.
 for byte,after_byte,renderer,after_renderer in [(0,1,None,1<<17),(1,128,1<<17,None),(127,1,1<<17,None),(255,1,0,1<<17)]:
  c={'element':True,'parent':True,'byte':byte,'renderer':renderer,'owners':[7,None],'parent_mutates':True,'after_parent_byte':after_byte,'owner_mutates':False,'after_owner_byte':0,'ui_mutates':True,'after_ui_renderer':after_renderer}
  m=Machine(c);rows.append({'input':c,'output':m.run()});observed.update(m.observed)
 for a,b in observed.items():assert b==P.range(a,len(b))==V.range(a,len(b))
 out={'schema':1,'cases':rows,'original_sha256':H(P.b),'v14_sha256':H(V.b),'producer_sha256':H(Path(__file__).read_bytes()),'instructions':[{'va':hex(a),'bytes':b.hex()}for a,b in sorted(observed.items())],'scope':'Original full4CFCC0 and529D70. Parent action rebuild4D1350, virtual owner,76C7F0 owner UI refresh and50C8E0 selected-view refresh are observed required service seams. Callback mutations prove live byte/render rereads and forwarding second null owner. No arithmetic or scheduling substituted.'}
 p=HERE/'native-action-publication.json';p.write_text(json.dumps(out,indent=2),encoding='utf-8');print(json.dumps({'cases':len(rows),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()

