"""Execute native command mask rebuilding and concrete Element/Organization scans."""
from pathlib import Path
import sys,json,random,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase7-actors'));from actor_states import Machine as Base,P,V,H
from unicorn.x86_const import *
OTHERS={26:0x5722d0,16:0x582c40,21:0x583fc0,14:0x585ae0,13:0x589a60,10:0x569c70,15:0x565140,19:0x575fb0}
# Flare's virtual+38 is read from its original constructor vtable.
row=next(r for r in json.loads((ROOT/'research/phase7-actors/component-dispatch.json').read_text())['rows'] if r['slot']==26)
OTHERS[26]=struct.unpack('<I',P.range(int(row['vtable'],16)+0x38,4))[0]
class Machine(Base):
 def __init__(self,c):
  super().__init__();self.c=c;self.events=[];self.org=0x2085000;self.elem=0x2086000;self.parentorg=0x2087000;self.parent=0x20d9000;self.owner=0x20b0000;self.vt=0x20b8000;self.members=0x20e0000;self.groups=0x20f0000
  self.register();self.events=[];self.logical={self.thing:1,self.parent:2,self.owner:7}
  self.w(self.vt+0x108,0x301f700);self.w(self.vt+0x34,0x4d1f90)
  self.w(self.thing,self.vt);self.w(self.thing+4,self.definition);self.w(self.thing+8,c['flags']);self.w(self.thing+0x100,c['state']&0xffffffff);self.w(self.thing+0x104,c['state']>>32);self.setmask(self.thing,c['actions'])
  self.w(self.parent+8,c.get('parent_flags',0));self.w(self.parent,self.vt);self.w(self.parent+4,self.definition);self.w(self.parent+0x100,c['parent_state']&0xffffffff);self.w(self.parent+0x104,c['parent_state']>>32);self.w(self.parent+0x7c,self.parentorg)
  self.w(self.owner+0x218,c['owner218']);self.w(0xb364c0,self.manager)
  self.w(self.manager+8,9);self.w(self.manager+0x2000c,self.owner if c['valid_ref'] else 0)
  for entry in self.records:self.u.mem_write(entry['address']+0x1c,bytes([int(c['resupply'][entry['index']])]))
  self.w(self.thing+0x80,self.elem if c['has_element'] else 0);self.w(self.elem+4,self.thing);self.w(self.elem+0x14,0x1234 if c['element_definition'] else 0);self.w(self.elem+0x10,self.parent if c['element_parent'] else 0);self.w(self.elem+0x1c,int(c['element_ready']))
  self.w(self.thing+0x7c,self.org if c['has_org'] else 0)
  for org,owner in [(self.org,self.thing),(self.parentorg,self.parent)]:
   self.w(org+4,owner);self.w(org+0x28,self.members);self.w(org+0x2c,len(c['members']));self.w(org+0x18,self.groups);self.w(org+0x1c,self.groups+0x1000);self.w(org+0xd0,0x90002);self.w(org+0x44,0 if c['selected'] is None else 0x20d0000+c['selected']*0x100)
  for i,mask in enumerate(c['members']):
   actor=0x20d0000+i*0x100;self.logical[actor]=10+i;self.w(self.members+i*4,actor if mask is not None else 0);self.w(actor,self.vt);self.setmask(actor,mask or 0)
  self.w(self.groups+0x20,len(c['groups']));self.w(self.groups+0x1c,self.groups+0x100)
  for i,group in enumerate(c['groups']):
   self.u.mem_write(self.groups+0x100+i*16+13,bytes([int(group['locked'])]));param=self.groups+0x2000+i*0x200;arr=param+0x2000;self.w(self.groups+0x1000+i*4,param);self.w(param+0x1ec,arr);self.w(param+0x1f0,len(group['classes']))
   for k,flags in enumerate(group['classes']):ref=arr+0x80+k*0x40;self.w(arr+k*4,ref);self.w(ref,ref+4);self.w(ref+4+0x54,flags)
  self.w(self.thing+0x90,0x20b9000 if c['wall'] else 0)
  for slot in c['other']:
   comp=0x20ba000+slot*0x100;self.w(self.thing+0x60+slot*4,comp);self.w(comp,self.vt+0x400);self.w(comp+4,self.thing)
  self.w(self.vt+0x400+0x38,0x301f720)
  self.w(self.thing+0x70,0x20be000 if c['mind'] else 0);self.w(0x20be000,self.vt+0x500);self.w(0x20be004,self.thing);self.w(self.vt+0x500+0xb4,0x301f740)
 def mask(self,a):return self.r(a+0xf0)|(self.r(a+0xf4)<<32)
 def setmask(self,a,bits):self.w(a+0xf0,bits&0xffffffff);self.w(a+0xf4,bits>>32)
 def hook(self,u,pc,size,user):
  if P.base<=pc<P.base+P.image_size:self.observed[pc]=bytes(u.mem_read(pc,size))
  if pc==0x4d1350:return
  if pc==0x301f700:
   actor=u.reg_read(UC_X86_REG_ECX);self.events.append(['owner',self.logical[actor]])
   if self.c.get('rebind_element_owner'):self.w(self.elem+4,self.parent)
   self.ret(self.owner if self.c['owner'] else 0)
  elif pc==0x4e1430:self.events.append(['can_recruit']);self.ret(int(self.c['eligible']),4)
  elif pc==0x4e1e40:self.events.append(['recruit_count']);self.ret(self.c['recruit_count'],4)
  elif pc==0x4cfcc0:self.events.append(['changed',self.mask(self.thing)]);self.ret()
  elif pc==0x301f740:self.events.append(['mind']);self.setmask(self.thing,self.mask(self.thing)|self.c['mind_mask']);self.ret()
  elif pc==0x301f720 or pc in OTHERS.values():
   slot=26 if pc==0x301f720 else next(s for s,a in OTHERS.items()if a==pc);self.events.append(['other',slot]);self.setmask(self.thing,self.mask(self.thing)|(1<<slot));self.ret()
  else:super().hook(u,pc,size,user)
 def run(self):
  entry={'rebuild':0x4d1350,'organization':0x57b390,'element':0x571640}[self.c['mode']];this={'rebuild':self.thing,'organization':self.org,'element':self.elem}[self.c['mode']]
  try:self.invoke(entry,this)
  except Exception as e:raise RuntimeError((hex(self.u.reg_read(UC_X86_REG_EIP)),self.c,self.events))from e
  return {'actions':self.mask(self.thing),'parent_actions':self.mask(self.parent),'events':self.events}
def main():
 rng=random.Random(0x4d1350);rows=[];observed={}
 states=[0,1<<12,1<<4,1<<19,1<<10,1<<26,1<<36,1<<25,1<<34]
 for index in range(96):
  count=index%5;members=[None if (index+i)%7==0 else rng.getrandbits(36)for i in range(count)];selected=next((i for i,x in enumerate(members)if x is not None),None)
  base={'flags':(1 if index%17==0 else 0)|(1<<29 if index%11==0 else 0),'state':states[index%len(states)],'parent_state':states[(index+3)%len(states)],'actions':rng.getrandbits(42),'owner':index%3!=0,'owner218':index%2,'resupply':[rng.choice([False,True])for _ in range(42)],'members':members,'selected':selected if index%2 else None,'valid_ref':index%3!=0,'groups':[{'locked':bool((index+i)%2),'classes':[rng.randrange(16)for _ in range((index+i)%4)]}for i in range(index%4)],'eligible':index%3==0,'recruit_count':[-1,0,1,9][index%4],'has_element':index%3!=0,'has_org':index%4!=0,'element_definition':index%3!=0,'element_parent':index%2==0,'element_ready':index%5!=0,'wall':index%19==0,'mind':index%2==0,'mind_mask':rng.getrandbits(36),'other':[s for i,s in enumerate(OTHERS)if (index+i)%17==0]}
  for mode in ['rebuild','organization','element']:
   c={**base,'mode':mode};m=Machine(c);rows.append({'input':c,'output':m.run()});observed.update(m.observed)
 for dead_parent in [False,True]:
  c={**rows[0]['input'],'mode':'element','flags':0 if dead_parent else 1,'parent_flags':int(dead_parent),'parent_state':1<<12,'owner':True,'owner218':1,'resupply':[True]*42,'members':[1,2],'has_element':True,'element_definition':True,'element_parent':True,'element_ready':True,'rebind_element_owner':True}
  m=Machine(c);rows.append({'input':c,'output':m.run()});observed.update(m.observed)
 for a,b in observed.items():assert b==P.range(a,len(b))==V.range(a,len(b))
 out={'schema':1,'cases':rows,'original_sha256':H(P.b),'v14_sha256':H(V.b),'producer_sha256':H(Path(__file__).read_bytes()),'instructions':[{'va':hex(a),'bytes':b.hex()}for a,b in sorted(observed.items())],'scope':'Full4D1350,571640,57B390 plus native57B2E0/57ABE0/57A030/57A730/57A5E0/578A30/579580, original4364C0 member iteration, ordinary4D1F90 availability, generation checked401060. Native owner virtual, Kingdom recruitment services, Mind/non-ground contributors and4CFCC0 publication are explicit observed behavior seams. No fake field inference; all vectors are bounded fixtureworld inputs.'}
 p=HERE/'native-ability-flags.json';p.write_text(json.dumps(out,indent=2));print(json.dumps({'cases':len(rows),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
