"""Actual5035E0 recursive member/vacant slot and503390 definition checks."""
from pathlib import Path
import sys,json,random,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1];sys.path.insert(0,str(ROOT/'research/phase7-actors'))
from actor_states import Machine as Base,P,V,H
from unicorn.x86_const import *
class Machine(Base):
 def __init__(self,c):
  super().__init__();self.c=c;self.nodes=0x20f0000;self.events=[]
  self.w(self.prop(9)+0x38,self.list([self.prop(x)for x in c['requires']]))
  for key,a in c['actors'].items():
   id=int(key);ptr=self.actor(id);props=ptr+0x400;org=ptr+0x600
   self.w(ptr+0x78,props if a['properties']else 0);self.w(ptr+0x7c,org if a['members']is not None else 0)
   self.w(props+0x10,ptr+0x800);self.w(props+0x14,len(a['available']))
   for i,x in enumerate(a['available']):q=ptr+0x900+i*0x20;self.w(ptr+0x800+4*i,q);self.w(q+0xc,self.prop(x));self.w(q+0x10,i%2)
   if a['members']is not None:
    self.w(org+0x28,ptr+0xa00);self.w(org+0x2c,len(a['members']));self.w(org+0x18,ptr+0xb00);self.w(ptr+0xb28,ptr+0xc00);self.w(org+0x1c,ptr+0xd00)
    for i,x in enumerate(a['members']):self.w(ptr+0xa00+4*i,self.actor(x)if x else 0);self.w(ptr+0xc00+4*i,i);self.w(ptr+0xd00+4*i,self.definition_ptr(a['vacant'][i])if a['vacant'][i]else 0)
  for key,d in c['definitions'].items():ptr=self.definition_ptr(int(key));self.w(ptr+0x174,d['mask']);self.w(ptr+0x1d4,self.list([self.prop(x)for x in d['available']]))
 def actor(self,id):return 0x20a0000+id*0x1000
 def definition_ptr(self,id):return 0x20b0000+id*0x800
 def prop(self,id):return 0x20e0000+id*0x100
 def list(self,items):
  head=0
  for x in reversed(items):p=self.nodes;self.nodes+=8;self.w(p,x);self.w(p+4,head);head=p
  return head
 def hook(self,u,pc,size,data):
  if pc==0x5035e0:self.events.append(['actor',(self.arg()-0x20a0000)//0x1000])
  elif pc==0x503390:self.events.append(['definition',(self.arg()-0x20b0000)//0x800])
  elif pc==0x4cef80:self.events.append(['required',(u.reg_read(UC_X86_REG_ECX)-0x20a0000)//0x1000,(self.arg()-0x20e0000)//0x100])
  super().hook(u,pc,size,data)
 def run(self):
  self.w(self.stack,self.stop);self.w(self.stack+4,self.actor(1));self.w(self.stack+8,self.prop(9));self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.emu_start(0x5035e0,self.stop,count=100000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.stop and self.u.reg_read(UC_X86_REG_ESP)==self.stack+4
  return {'result':bool(self.u.reg_read(UC_X86_REG_EAX)&255),'events':self.events}
def main():
 rng=random.Random(0x5035e0);rows=[];observed={}
 for index in range(128):
  actors={}
  for id in range(1,6):
   members=([id+1,None,5]if id<4 else[])if rng.randrange(3)else None
   actors[str(id)]={'properties':bool(rng.randrange(2)),'available':[x for x in [1,2,3]if rng.randrange(2)],'members':members,'vacant':[rng.choice([None,1,2,3])for _ in members]if members is not None else[]}
  c={'actors':actors,'requires':rng.choice([[],[1],[1,2],[1,2,3],[2,2]]),'definitions':{str(i):{'mask':rng.choice([0,64,0x40036d]),'available':[x for x in [1,2,3]if rng.randrange(2)]}for i in range(1,4)}}
  m=Machine(c);rows.append({'input':c,'output':m.run()});observed.update(m.observed)
 for a,b in observed.items():assert b==P.range(a,len(b))==V.range(a,len(b))
 out={'schema':1,'cases':rows,'source_sha256':H(P.b),'v14_sha256':H(V.b),'producer_sha256':H(Path(__file__).read_bytes()),'instructions':[{'va':hex(a),'bytes':b.hex()}for a,b in sorted(observed.items())],'scope':'Full native recursive5035E0,503390,4CEF80,581280,5032D0 and45EEF0. No property/membership decision hooks. Valid acyclic initialized actor/definition layouts; prior slot instance duplicates can be disabled. Recursion/required-property entry events are observations, not replacements.'}
 p=HERE/'native-property-applicability.json';p.write_text(json.dumps(out,indent=2),encoding='utf-8');print(json.dumps({'cases':len(rows),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
