from pathlib import Path
import sys,json,struct
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE))
from native_ability_derived import Machine as TargetBase
from probe_lists import Machine as Base
from native_ground_definitions import P,V,H,bits
from unicorn.x86_const import *
class Targets(TargetBase):
 def flags(self,raw,initial):
  self.names={'a':0,'b':1,'high':31,'wrap':32,'above':65,'+a':4,'!a':6,'-a':8}
  p=self.alloc(4);self.w(p,initial);ref=self.alloc(4);self.w(ref,self.string(list(map(ord,raw))));self.invoke(0x4c5a80,p,[ref]);return self.r(p)
class Resistance(Base):
 def __init__(self,amount,identity):
  super().__init__();self.amount=amount;self.identity=identity;self.enums=[];self.lookup_order=[]
  self.u.mem_write(0x301e000,b'\xd9\x05'+struct.pack('<I',0x225f000)+b'\xc2\x04\x00');self.w(0x225f000,amount)
 def hook(self,u,pc,n,z):
  if pc==0x4feaf0:self.w(u.reg_read(UC_X86_REG_ECX)+8,5);self.ret(pop=4)
  elif pc==0x5b5130:self.lookup_order.append('amount');u.reg_write(UC_X86_REG_EIP,0x301e000)
  elif pc==0x5b54d0:
   self.lookup_order.append('damage_type');p=self.alloc(4);self.w(p,self.string(list(map(ord,'resolved_damage'))));self.ret(p,pop=4)
  elif pc==0x48d500:self.ret(self.identity,pop=4)
  elif pc==0x4c0f50:self.enums.append({'index':self.arg(),'name':P.range(self.arg(1),120).decode('utf-16-le').split('\0')[0]});self.ret(pop=12)
  else:super().hook(u,pc,n,z)
 def load(self,cw):
  self.u.reg_write(UC_X86_REG_FPCW,cw);p=self.alloc(20);self.invoke(0x4fe3b0,p,[]);self.invoke(0x501290,p,[0]);return {'kind':self.r(p+8),'amount':self.r(p+12),'damage_identity':self.r(p+16),'field_order':self.lookup_order}
target_rows=[];resist=[];visited={}
for raw in ['', 'a','a,b','a b',',a,','a,,  b','high,wrap,above','+a','!a','-a','+a,!a,-a']:
 for initial in [0,0x81000000]:
  m=Targets();out=m.flags(raw,initial);target_rows.append({'raw':raw,'initial':initial,'registry':m.names,'output':out,'lookups':m.lookups});visited.update(m.visited)
for pc,cw in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
 for amount in [0,bits(.2),bits(-.2),bits(1),bits(5),0x80000000]:
  for identity in [0x20001,0x20202]:
   m=Resistance(amount,identity);out=m.load(cw);assert out['amount']==amount and out['damage_identity']==identity and out['kind']==5;resist.append({'precision':pc,'amount':amount,'damage_identity':identity,'output':out});visited.update(m.visited)
m=Resistance(0,1);m.invoke(0x4c1060,0x2240000,[]);enums=m.enums;visited.update(m.visited)
spans=[]
for a,n in sorted(visited.items()):
 if not P.base<=a<P.base+P.image_size:continue
 if spans and spans[-1][1]>=a:spans[-1][1]=max(spans[-1][1],a+n)
 else:spans.append([a,a+n])
spans += [[0x501290,0x501346],[0x4c1060,0x4c1105],[0x4c4030,0x4c4190],[0x4c42d0,0x4c4640]]
pins=[]
for a,b in spans:
 raw=P.range(a,b-a);assert raw==V.range(a,b-a);pins.append({'start':hex(a),'end':hex(b),'sha256':H(raw)})
d={'passed':True,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':H(P.b),'v14_sha256':H(V.b),'target_flag_cases':target_rows,'resistance_cases':resist,'modifier_kinds':enums,'ranges':pins,'seams':['Whole4C5A80 native tokenizer, lookups, wrapping shifts and OR execute; typed registry returns explicit ID/index pairs. No Ability prefix/default handling is used.','Whole501290 executes with actual ctor4FE3B0; base4FEAF0 type5, typed amount/string and DamageType registry are explicit pre-resolved leaves. Amount is stored unchanged, not interpreted as a factor by this loader.','Modifier enum4C1060 executes and registration captures names/indices. Property modifier append and metadata caller ranges are statically pinned.']};(HERE/'native-property-definitions.json').write_text(json.dumps(d,indent=2));print('passed',len(target_rows),len(resist),len(enums),H((HERE/'native-property-definitions.json').read_bytes()))
