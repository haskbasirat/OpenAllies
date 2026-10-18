"""Read-only original/v14 saved flag1 PostCreate + empty4296B0 oracle.
Seamed positive property/owner effects are required boundaries, not their bodies.
"""
from pathlib import Path
import json,sys,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h
from unicorn import UC_HOOK_CODE,UC_HOOK_MEM_READ
from unicorn.x86_const import UC_X86_REG_EIP
ORIGINAL,V14=h.P,h.V
TABLES={1:0xa2a680,2:0xa2a2a0,3:0xa273f0,4:0xa299d8,5:0xa29780,6:0xa28a00,7:0xa28b88,9:0xa28e90}
class Stop(Exception):pass
class Machine(h.Machine):
 def __init__(self,c,pc):
  super().__init__(pc);self.c=c;self.trace=[];self.world_reads=[]
  self.actor=self.alloc(0x400);self.data=self.alloc(0x600);self.w32(self.actor,TABLES[c['class']]);self.w32(self.actor+4,self.data);self.w32(self.actor+8,0x80000000)
  self.w32(self.actor+0xe8,0x1234 if c.get('owner')else 0)
  for slot in c.get('present',[]):
   ptr=self.alloc(0x100);vt=self.alloc(0x40);self.w32(self.actor+0x60+4*slot,ptr);self.w32(ptr,vt,self.actor);self.w32(vt+12,0x581880 if slot==6 else (0x564be0 if slot==29 else 0x4d1e10))
  nodes=[self.alloc(8)for _ in c.get('properties',[])]
  self.w32(self.data+0x1d4,nodes[0]if nodes else 0)
  for i,(node,prop)in enumerate(zip(nodes,c.get('properties',[]))):self.w32(node,prop,nodes[i+1]if i+1<len(nodes)else 0)
  self.seams.update({0x5817a0:self.property,0x4e6180:self.owner})
  self.u.hook_add(UC_HOOK_CODE,self.observe)
 def property(self,m):
  self.trace.append(['property',self.arg(),self.arg(1),self.arg(2)])
  if self.c.get('stop_property')==len(self.trace):raise Stop()
  self.w32(self.actor+8,self.r32(self.actor+8)^1);self.ret(12)
 def owner(self,m):
  assert self.arg()==self.actor and self.ecx()==0x1234
  self.trace.append(['owner',0x1234])
  if self.c.get('stop_owner'):raise Stop()
  self.w32(self.actor+8,self.r32(self.actor+8)|(1<<28));self.ret(4)
 def observe(self,u,pc,n,_):
  if pc==0x4eb500:raise AssertionError('saved flag1 painted ground')
  if pc==0x5644f0:raise AssertionError('saved flag1 dereferenced null city params')
 def run(self):
  status='ok'
  try:
   self.call(h.P.u32(TABLES[self.c['class']]+0xec),self.actor,[0,1]);assert self.final_esp==h.STACK+12
  except Stop:status='required_stop'
  assert self.fpu_top()==0
  return {'status':status,'trace':self.trace,'flags':self.r32(self.actor+8)}
def main():
 cases=[]
 for cls in TABLES:
  cases.append({'name':f'class{cls}_empty','class':cls,'present':[6]if cls in[3,4]else[],'properties':[]})
 for cls in [3,4]:
  for props in [[],[19],[19,23,19]]:
   for owner in [False,True]:cases.append({'name':f'actor{cls}_{len(props)}props_owner{owner}','class':cls,'present':[0,6,29,30],'properties':props,'owner':owner})
  for stop in [1,2]:cases.append({'name':f'actor{cls}_partial{stop}','class':cls,'present':[6,29],'properties':[19,23,19],'owner':True,'stop_property':stop})
  cases.append({'name':f'actor{cls}_ownerstop','class':cls,'present':[6,29],'properties':[19,23],'owner':True,'stop_owner':True})
 rows=[];fx=[]
 for name,image in [('original',ORIGINAL),('v14',V14)]:
  h.P=image
  for pc in h.PRECISION:
   for c in cases:
    m=Machine(c,pc);rows.append({'image':name,'precision':pc,'input':c,'output':m.run()})
   # Native4296B0 null head: no GWorld is installed. Reading its mute pointer
   # would fault, so normal return proves this skipped dependency is unread.
   m=h.Machine(pc);head=m.alloc(8);m.w32(head,0,0);m.w32(0xb36534,0)
   m.call(0x4296b0,head,[0,1]);assert m.final_esp==h.STACK+12 and m.fpu_top()==0
   fx.append({'image':name,'precision':pc,'head':0,'world':0,'status':'ok'})
 h.P=ORIGINAL
 pins=[]
 for va,b in sorted(h.VISITED.items()):
  raw=bytes.fromhex(b);assert ORIGINAL.range(va,len(raw))==V14.range(va,len(raw))==raw; pins.append({'va':hex(va),'bytes':b})
 tables=[{'class':c,'vtable':hex(t),'post_create':hex(ORIGINAL.u32(t+0xec)),'virtual_b0':hex(ORIGINAL.u32(t+0xb0))}for c,t in TABLES.items()]
 for row in tables:assert V14.u32(int(row['vtable'],16)+0xec)==int(row['post_create'],16)
 out={'scope':'Saved PostCreate flag1 native dispatch/order and true null-head4296B0 only. Positive5817A0/4E6180 effects are observed seams; no property/Kingdom/FX callback-body or whole-world claim.','images':{'original':h.ORIGINAL_SHA,'v14':h.V14_SHA},'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'harness_sha256':hashlib.sha256(Path(h.__file__).read_bytes()).hexdigest(),'cases':rows,'empty_fx_cases':fx,'vtable_pins':tables,'instructions':pins}
 path=HERE/'native-post-create.json';path.write_text(json.dumps(out,indent=2)+'\n');print(json.dumps({'cases':len(rows),'empty_fx_cases':len(fx),'pins':len(pins),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
