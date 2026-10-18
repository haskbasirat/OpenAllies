"""Original4CF300 fresh nonnull Params/full-flag caller, required effect prefixes.
No native ground paint/City/Property/Kingdom effect-body completion claim.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn import UC_HOOK_CODE
P,V=h.P,h.V;H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
VT={0:0xa3b0e8,2:0xa3b4d8,3:0xa3ce88,'4org':0xa335a0,'4air':0xa32710,'4elem':0xa35810,
5:0xa3df40,6:0xa3e3dc,7:0xa3d434,8:0xa3c6f0,9:0xa3c690,10:0xa3c14c,11:0xa400c0,
13:0xa3fac8,15:0xa3b950,16:0xa3e6d0,17:0xa3c974,18:0xa3b2a8,19:0xa3cd00,
20:0xa3bbd4,21:0xa3eb58,22:0xa3c568,24:0xa40100,25:0xa3cc6c,29:0xa3b6f8,30:0xa3ca4c}
class Stop(Exception):pass
class Machine(h.Machine):
 def __init__(self,c,pc):
  super().__init__(pc);self.c=c;self.trace=[]
  self.actor=self.alloc(0x400);self.data=self.alloc(0x600);self.params=self.alloc(0x60)
  world=self.alloc(0x500);settings=self.alloc(0x100)
  self.w32(0xb36534,world);self.w32(world+0x28,settings);self.w32(settings+0xa8,c['mode'])
  self.w32(self.actor,0xa273f0,self.data,0x80000000);self.w32(self.data+0x174,c['mask'])
  self.w32(self.data+0x5b4,c['radius']);self.w32(self.params+0x34,c['arg34'])
  self.w32(self.actor+0xe8,0x1234 if c['owner'] else 0)
  for slot in c['present']:
   p=self.alloc(0x100);self.w32(self.actor+0x60+4*slot,p)
   self.w32(p,VT['4org' if slot==4 else slot],self.actor)
  nodes=[self.alloc(8) for _ in c['properties']]
  self.w32(self.data+0x1d4,nodes[0] if nodes else 0)
  for i,(node,value) in enumerate(zip(nodes,c['properties'])):self.w32(node,value,nodes[i+1] if i+1<len(nodes) else 0)
  self.seams[0x5817a0]=self.property;self.seams[0x4e6180]=self.owner
  self.u.hook_add(UC_HOOK_CODE,self.observe)
 def property(self,m):
  self.trace.append(['property',self.arg(),self.arg(1),self.arg(2)])
  if self.c['stop_property']==len(self.trace):raise Stop()
  self.w32(self.actor+8,self.r32(self.actor+8)^1);self.ret(12)
 def owner(self,m):
  assert self.arg()==self.actor and self.ecx()==0x1234
  self.trace.append(['owner',0x1234])
  if self.c['stop_owner']:raise Stop()
  self.w32(self.actor+8,self.r32(self.actor+8)|(1<<28));self.ret(4)
 def observe(self,u,pc,size,_):
  if pc==0x4eb500:self.trace.append(['ground']);raise Stop()
  if pc==0x5644f0:
   assert self.arg()==self.actor
   self.trace.append(['city',self.arg(1),'component' if self.arg(2) else 'postpass']);raise Stop()
 def run(self):
  status='ok'
  try:
   self.call(0x4cf300,self.actor,[self.params,self.c['flag']]);assert self.final_esp==h.STACK+12
  except Stop:status='required_stop'
  assert self.fpu_top()==0
  return {'status':status,'trace':self.trace,'flags':self.r32(self.actor+8)}
def main():
 cases=[]
 for flag in [0,1,0x100,0x101]:
  for mode in [0,1]:
   for variant in ['empty','properties','owner','partial_property','partial_owner','city_component','city_postpass','city_radius']:
    c={'flag':flag,'mode':mode,'present':[],'mask':0,'radius':0,'arg34':0xffffffff,'owner':False,'properties':[],'stop_property':0,'stop_owner':False}
    if variant in ['properties','owner','partial_property','partial_owner']:c.update(present=[0,2,6,7,30],properties=[19,23,19],owner=variant!='properties')
    if variant=='partial_property':c['stop_property']=2
    if variant=='partial_owner':c['stop_owner']=True
    if variant=='city_component':c.update(present=[6,29],properties=[19],arg34=7)
    if variant in ['city_postpass','city_radius']:c.update(mask=1<<29,radius=0x3f800000 if variant=='city_radius' else 0,arg34=23)
    cases.append(c)
 rows=[]
 for image,label in [(P,'original'),(V,'v14')]:
  h.P=image
  for pc in h.PRECISION:
   for c in cases:rows.append({'image':label,'precision':pc,'input':c,'output':Machine(c,pc).run()})
 h.P=P
 half=len(rows)//2
 for a,b in zip(rows[:half],rows[half:]):assert a['input']==b['input'] and a['output']==b['output']
 pins=[];cutoffs={0x4eb500,0x5644f0}
 for address,raw in sorted(h.VISITED.items()):
  value=bytes.fromhex(raw);assert P.range(address,len(value))==V.range(address,len(value))==value
  pins.append({'va':hex(address),'bytes':raw,'observed_cutoff':address in cutoffs})
 tables=[]
 for slot,address in VT.items():
  target=P.u32(address+12);assert V.u32(address+12)==target
  assert target==(0x581880 if slot==6 else 0x564be0 if slot==29 else 0x4d1e10)
  tables.append({'slot':slot,'vtable':hex(address),'post_create':hex(target)})
 out={'schema':1,'producer_sha256':H(Path(__file__)),'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,
  'cases':rows,'component_vtables':tables,'instructions':pins,
  'scope':__doc__+' Property5817A0 and owner4E6180 are explicit synthetic effects/required boundaries; ground4EB500 and City5644F0 are pre-instruction cutoffs. No whole fresh Init/world/battle claim.'}
 path=HERE/'native-fresh-post-create.json'
 if '--write' in sys.argv:path.open('x').write(json.dumps(out,indent=1)+'\n')
 else:assert json.loads(path.read_text())==out
 print(json.dumps({'rows':len(rows),'pairs':half,'pins':len(pins),'sha256':H(path)}))
if __name__=='__main__':main()
