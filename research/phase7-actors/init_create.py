"""Full GActor4D6F10 caller ordering; external leaf/world services are observed."""
from pathlib import Path
import sys,struct,json,random
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-actors'))
from trace_components import Machine as Base,P,V,H
from unicorn.x86_const import *
F=lambda n:struct.unpack('<I',struct.pack('<f',n))[0]
class Machine(Base):
 def __init__(self,c):
  super().__init__({**c,'mode':'full'});self.c=c;self.record=0x2030000;self.child=0x2040000;self.childdef=0x2041000;self.manager=0x2050000
  self.w(self.actor+0x14,0x90001);self.w(self.actor+0xe8,7 if c['owner'] else 0);self.w(self.definition+0x1ac,c['number_definition']);self.w(self.definition+0x1b0,int(c['auto_number']))
  self.w(self.definition+0x268,self.childdef if c['linked'] else 0)
  self.w(self.record+0x38,c['reason']);self.w(self.record+0x51,int(c['start'])|(int(c['nonblocking'])<<8))
  for i,x in enumerate(c['anchor']):self.w(self.definition+0x270+4*i,x)
  for i,x in enumerate(c['child_anchor']):self.w(self.childdef+0x270+4*i,x)
  for i,x in enumerate(c['position']):self.w(self.actor+0x20+4*i,x)
  for i,x in enumerate(c['facing']):self.w(self.actor+0x2c+4*i,x)
  self.w(0xb364c0,self.manager);self.w(self.manager+8,9);self.w(self.manager+0x2000c,self.child if c['child_resolves'] else 0);self.w(self.child+0x14,0x90002)
  self.w(self.child+0x60,0x2042000 if c['child_body'] else 0)
  self.w(0x200400c,99999);self.w(0x2005000+0x48,0x2005100);self.w(0x2005000+0x4c,1 if c['view_count'] else 0);self.w(0x2005100,1 if c['view_first'] else 0)
  self.u.reg_write(UC_X86_REG_FPCW,{24:0x7f,53:0x27f,64:0x37f}[c['precision']]);self.events=[]
 def hook(self,u,pc,size,user):
  self.visited[pc]=size;ecx=u.reg_read(UC_X86_REG_ECX)
  if pc==0x4e9fc0:self.events.append({'kind':'object'});self.ret(int(self.c['base_ok']),4)
  elif pc==0x4d49b0:
   self.ret(1 if self.r(self.actor+0x100)&0xfe000000 else 0,4)
  elif pc==0x4d64f0:
   index,enabled=self.arg(),bool(self.arg(1));self.events.append({'kind':'set_state','index':index,'enable':enabled});self.w(self.actor+0x100,self.r(self.actor+0x100)|(1<<index));self.ret(pop=8)
  elif pc==0x4f4a20:self.events.append({'kind':'number_definition','definition':self.arg(1)});self.ret(pop=8)
  elif pc==0x4e4980:self.events.append({'kind':'owner_number','owner':ecx});self.ret(pop=4)
  elif pc==0x4d0720:self.ret(0x2005000)
  elif pc==0x563d50:self.events.append({'kind':'character_start'});self.ret()
  elif pc==0x4cfc80:self.events.append({'kind':'default_stance'});self.ret()
  elif pc==0x4c8c70:
   u.mem_write(ecx,bytes(0x54));self.w(ecx+0x1c,0x2004010);self.ret(ecx)
  elif pc==0x23ff100:
   caller=self.r(u.reg_read(UC_X86_REG_ESP))
   if caller==0x4d7075 and self.c['mutate_owner']:
    for i,x in enumerate(self.c['mutated_position']):self.w(self.actor+0x20+4*i,x)
    for i,x in enumerate(self.c['mutated_facing']):self.w(self.actor+0x2c+4*i,x)
   if caller==0x4d30c4:self.events.append({'kind':'assembly_completed'})
   self.ret(7 if self.c['owner'] else 0)
  elif pc==0x4fa030:
   r=self.arg();self.events.append({'kind':'linked_create','position':[self.r(r+8+4*i)for i in range(3)],'facing':[self.r(r+0x14+4*i)for i in range(2)],'owner':self.r(r+0x28),'parent':self.r(r+0x48),'nonblocking':bool(u.mem_read(r+0x52,1)[0])});self.ret(self.child if self.c['child_created'] else 0,4)
  elif pc==0x562200:self.events.append({'kind':'copy_health'});self.ret(pop=8)
  elif pc==0x54bfc0:self.events.append({'kind':'register_construction','owner':self.arg()});self.ret(pop=8)
  elif pc==0x4cf480 and self.arg()==4 and not self.c['mask']&((1<<7)|(1<<8)|(1<<23)):
   self.ret(pop=4)
  else:super().hook(u,pc,size,user)
 def run_full(self):
  self.w(self.stack,self.stop);self.w(self.stack+4,self.record);self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.reg_write(UC_X86_REG_ECX,self.actor)
  try:self.u.emu_start(0x4d6f10,self.stop,count=100000)
  except Exception as e:raise RuntimeError((hex(self.u.reg_read(UC_X86_REG_EIP)),self.c,self.events)) from e
  assert self.u.reg_read(UC_X86_REG_EIP)==self.stop and self.u.reg_read(UC_X86_REG_ESP)==self.stack+8
  events=[{k:v for k,v in e.items() if k!='record'}for e in self.events]
  return {'result':bool(self.u.reg_read(UC_X86_REG_EAX)&255),'events':events,'present':[s for s in range(31) if self.r(self.actor+0x60+s*4)],'low':self.r(self.actor+0x100),'high':self.r(self.actor+0x104),'linked':self.r(self.actor+0x168),'parent':self.r(self.child+0x16c)}
def main():
 rng=random.Random(0x4d6f10);rows=[];visited={}
 for precision in [24,53,64]:
  for n in range(64):
   c={'precision':precision,'mask':[0x40036d,0x4202c0,0,0x7fffffff][n%4],'low':[0,1<<21,1<<25,1<<26][(n//4)%4],'high':16 if n%17==0 else 0,'present':[2] if n%7==0 else [],'city':n%2==0,'base_ok':n%23!=0,'fail_slot':[0,4,30,-1][n%4] if n%13==0 else -1,
    'owner':n%2==0,'number_definition':5 if n%3==0 else 0,'auto_number':n%5!=0,'reason':4 if n%7==0 else 0,'start':n%3!=0,'view_count':n%5!=0,'view_first':n%7!=0,'nonblocking':n%2==0,
    'linked':n%2==1,'child_created':n%7!=0,'child_resolves':n%11!=0,'child_body':n%3!=0,'mutate_owner':n%9==0,
    'anchor':[F(rng.uniform(-100,100))for _ in range(3)],'child_anchor':[F(rng.uniform(-100,100))for _ in range(3)],'position':[F(rng.uniform(-1000,1000))for _ in range(3)],'facing':[F(rng.uniform(-1,1))for _ in range(2)],'mutated_position':[F(rng.uniform(-1000,1000))for _ in range(3)],'mutated_facing':[F(rng.uniform(-1,1))for _ in range(2)]}
   m=Machine(c);rows.append({'input':c,'output':m.run_full()});visited.update(m.visited)
 instructions=[]
 for a,n in sorted(visited.items()):
  if P.base<=a<P.base+P.image_size:
   raw=P.range(a,n);assert raw==V.range(a,n);instructions.append({'va':hex(a),'bytes':raw.hex()})
 out={'schema':1,'original_sha256':H(P.b),'v14_sha256':H(V.b),'producer_sha256':H(Path(__file__).read_bytes()),'dependency_sha256':H((ROOT/'research/phase4-actors/trace_components.py').read_bytes()),'cases':rows,'instructions':instructions,'scope':'Full4D6F10 with actual4D21A0 selection/requirement/list helper chain, actual anchor arithmetic/4BE740 andgenerationcheckedlinkedlookup. Object init, registered actor-state operation, actual component constructors/virtualmethods, owner/numbering,Character/defaultstance,linked-world creation andBody copy are explicit seams. All selected seams have separate component evidence; this test establishes caller ordering and finite numeric stores. Owner query mutation stresses offset-beforeowner/position-afterowner sequencing. UI process globalB36528 isnull; full UI-agent branch not claimed.'}
 p=HERE/'native-init-create.json';p.write_text(json.dumps(out,indent=2));print(json.dumps({'cases':len(rows),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
