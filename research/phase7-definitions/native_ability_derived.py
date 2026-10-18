"""Execute original target parser, Ability default filters and effect flags/power."""
from pathlib import Path
import sys,json,struct,itertools
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE));from probe_lists import Machine as Base
from native_ground_definitions import P,V,H,bits
from unicorn.x86_const import *
class Machine(Base):
 def __init__(self):super().__init__();self.names={};self.lookups=[];self.failure=None
 def hook(self,u,pc,n,z):
  if pc==0x48d500:
   p=self.arg();units=[]
   while(unit:=self.u.mem_read(p,2))!=b'\0\0':units.append(struct.unpack('<H',unit)[0]);p+=2
   name=''.join(map(chr,units));self.lookups.append(name)
   if name not in self.names:self.failure=name;u.emu_stop();return
   obj=self.alloc(16);self.w(obj+12,self.names[name]);self.ret(obj,pop=4)
  elif pc==0x410c40:self.failure='native diagnostic';u.emu_stop()
  else:super().hook(u,pc,n,z)
 def target(self,raw,initial=(0,0)):
  self.names={'a':0,'b':1,'body':12,'actor':13,'organization':9,'invulnerable':14,'high':31,'wrap':32,'above':65}
  p=self.alloc(8);self.w(p,initial[0]);self.w(p+4,initial[1]);s=self.alloc(4);self.w(s,self.string(list(map(ord,raw))));self.invoke(0x4c5bf0,p,[s]);return[self.r(p),self.r(p+4)]
 def defaults(self,pair):
  p=self.alloc(8);self.w(p,pair[0]);self.w(p+4,pair[1]);self.u.reg_write(UC_X86_REG_EDI,p);self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.emu_start(0x4bf411,0x4bf431,count=10000);return[self.r(p),self.r(p+4)]
 def power(self,flags,effects):
  p=self.alloc(0x9c);self.w(p+0x54,flags);last=0
  for kind,amount in reversed(effects):
   effect=self.alloc(0x20);self.w(effect+4,kind);self.w(effect+0x18,amount);node=self.alloc(8);self.w(node,effect);self.w(node+4,last);last=node
  self.w(p+0x68,last);self.invoke(0x4bee20,p,[]);return[self.r(p+0x54),self.r(p+0x64)]
def main():
 rows=[];defaults=[];powers=[];visited={}
 for raw in ['', 'a','+a','!a','-a','a,b','a b','a,,  b',',a,',' +a, -b','a,!a','high,!wrap,+above','body,actor,!invulnerable','organization']:
  for initial in [(0,0),(0x80000000,0x10)]:
   m=Machine();value=m.target(raw,initial);rows.append({'raw':raw,'registry':m.names,'initial':initial,'output':value,'lookups':m.lookups});visited.update(m.visited)
 for req in [0,1<<9,1<<12,1<<13,1<<14,(1<<9)|(1<<14),0xffffffff]:
  for forbidden in [0,1<<12,0xffffffff]:
   m=Machine();value=m.defaults([req,forbidden]);defaults.append({'initial':[req,forbidden],'output':value});visited.update(m.visited)
 for flags in [0,0x7ff,0xffffffff]:
  for effects in [[],[(4,bits(12))],[(5,bits(3)),(4,bits(12))],[(4,0),(4,bits(20))],[(0,bits(3)),(13,0)]]:
   m=Machine();value=m.power(flags,effects);powers.append({'flags':flags,'effects':effects,'output':value});visited.update(m.visited)
 spans=[]
 for a,n in sorted(visited.items()):
  if not P.base<=a<P.base+P.image_size:continue
  if spans and spans[-1][1]>=a:spans[-1][1]=max(spans[-1][1],a+n)
  else:spans.append([a,a+n])
 pins=[]
 for a,b in spans:
  raw=P.range(a,b-a);assert raw==V.range(a,b-a);pins.append({'start':hex(a),'end':hex(b),'sha256':H(raw)})
 doc={'schema':1,'passed':True,'producer_sha256':H(Path(__file__).read_bytes()),'machine_sha256':H((HERE/'probe_lists.py').read_bytes()),'original_sha256':H(P.b),'v14_sha256':H(V.b),'target_cases':rows,'default_cases':defaults,'power_cases':powers,'ranges':pins,'seams':['Target registry lookup returns supplied exactname/index pairs; native tokenizer, prefix, shifts, ORs and cleanup execute unchanged. Registry domain initialization has separate source proof.','Ability default target bits4BF411..4BF431 and full4BEE20 effect flag/main amount finalizer execute. Effects linked-list nodes are prepared from typed loaded kind and amount, not guessed source tags.','Memory/string pool allocation seams match earlier native list parser fixtures. No full4BF990 completion claimed by these isolated fragments.']}
 (HERE/'native-ability-derived.json').write_text(json.dumps(doc,indent=2));print('passed',len(rows),len(defaults),len(powers))
if __name__=='__main__':main()
