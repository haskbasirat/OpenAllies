"""Native4D1350/57B390 and actual4CFCC0 over captured saved owner contexts.

Original/v14 separately execute at PC24/53/64. Recorded native ActorState
registration/masks and captured source resupply fields populate the definition
table; native state selection executes. Native availability, generation check,
empty-HQ recruitment and publication execute. Unavailable owner-view/renderer
services interrupt before their bodies/reads; no contributor is answered by a
synthetic success or a new command mask.
Each capture supplies an actual held Organization context. Native Actor+7C is
installed even though slot7 is temporarily absent from the serialized list of
the owner's current component slots. Unsupported resupply fields are guarded
against native reads, never answered as false.
"""
from pathlib import Path
import json,sys,hashlib,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h
from unicorn import UC_HOOK_CODE,UC_HOOK_MEM_READ
ORIGINAL=h.P
REG=ROOT/'research/phase7-actors/native-actor-states.json'
class Required(Exception):pass
class Owner(h.Machine):
 def __init__(self,c,image,pc):
  h.P=ORIGINAL if image=='original' else h.V;super().__init__(pc);self.c=c;self.events=[]
  self.actor=self.alloc(0x200);self.org=self.alloc(0x100);self.vt=self.alloc(0x200);self.definition=self.alloc(0x600)
  self.w32(self.actor,self.vt);self.w32(self.actor+4,self.definition);self.w32(self.vt+0x108,0x4d1e80);self.w32(self.vt+0x34,0x4d1f90)
  self.w32(self.actor+8,c['flags']);self.w32(self.actor+0x100,c['state']&0xffffffff,c['state']>>32)
  self.w32(self.actor+0xf0,c['actions']&0xffffffff,c['actions']>>32);self.w8(self.actor+0x4c,c['object4c'])
  self.w32(self.actor+0x7c,self.org);self.w32(self.org+4,self.actor)
  forbidden={4,10,12,13,14,15,16,19,21,26};assert not forbidden.intersection(c['components']),('unmodeled contributor/Wall',c['components'])
  if c['element'] is not None:
   e=c['element'];assert e['parent'] is None,'parent publication outside these captured contexts'
   element=self.alloc(0x40);self.w32(self.actor+0x80,element);self.w32(element+4,self.actor)
   self.w32(element+0x14,0x77777777 if e['definition'] else 0);self.w8(element+0x1c,e['ready'])
  self.members={c['owner']:self.actor};slots=self.alloc(max(4,len(c['members'])*4));self.w32(self.org+0x28,slots,len(c['members']))
  for i,member in enumerate(c['members']):
   if member is None:continue
   actor=self.members.get(member['id'])
   if actor is None:
    actor=self.alloc(0x200);self.members[member['id']]=actor;self.w32(actor,self.vt)
    self.w32(actor+0xf0,member['actions']&0xffffffff,member['actions']>>32)
   self.w32(slots+i*4,actor)
  self.w32(self.org+0x44,self.members[c['selected']] if c['selected'] else 0);self.w32(self.org+0xd0,c['reference_d0'])
  data=self.alloc(0x60);entries=self.alloc(max(16,16*len(c['groups'])));params=self.alloc(max(4,4*len(c['groups'])))
  self.w32(self.org+0x18,data);self.w32(self.org+0x1c,params);self.w32(data+0x1c,entries,len(c['groups']))
  for i,group in enumerate(c['groups']):
   self.w8(entries+i*16+13,group['locked']);parameter=self.alloc(0x220);abilities=self.alloc(max(4,4*len(group['abilities'])))
   self.w32(params+i*4,parameter);self.w32(parameter+0x1ec,abilities,len(group['abilities']))
   for j,flags in enumerate(group['abilities']):
    local=self.alloc(4);definition=self.alloc(0x60);self.w32(local,definition);self.w32(definition+0x54,flags);self.w32(abilities+j*4,local)
  kingdom=self.alloc(0x400) if c['owner_id'] else 0;self.w32(self.actor+0xe8,kingdom)
  if kingdom:
   assert not c['hqs'],'nonempty HQ services are outside these captured contexts'
   self.w32(kingdom+0x218,c['kingdom218']);self.w32(kingdom+0x2e0,0)
  manager=self.alloc(0x60004);self.w32(0xb364c0,manager)
  if c['reference_valid']:
   ref=c['reference_d0'];self.u.mem_write(manager+4+(ref&0xffff)*2,struct.pack('<H',ref>>16));self.w32(manager+0x20004+(ref&0xffff)*4,self.actor)
  registration=json.loads(REG.read_text())['registration'];game_data=self.alloc(0x400);array=self.alloc(42*4)
  self.w32(0xb36530,game_data);self.w32(game_data+0x2e4,array,42)
  self.u.mem_write(0xb3ac40,struct.pack('<31Q',*registration['masks']))
  self.unavailable_states=set()
  for row in registration['records']:
   index=row['index'];state=self.alloc(0x40);self.w32(array+index*4,state);self.w32(state+0xc,index);self.w32(state+0x2c,row['group']);self.w8(state+0x30,row['exclusive'])
   source=c['resupply'][index]
   if source is None:self.unavailable_states.add(state+0x1c)
   else:self.w8(state+0x1c,source)
  renderer=self.alloc(0x80) if c['renderer_link'] else 0;self.w32(self.actor+0xc,renderer)
  self.u.hook_add(UC_HOOK_CODE,self.observe)
  self.u.hook_add(UC_HOOK_MEM_READ,self.read)
 def read(self,u,access,address,size,value,_):
  assert not any(address<=point<address+size for point in self.unavailable_states),('unavailable source resupply field read',hex(address))
 def observe(self,u,pc,size,_):
  if pc==0x76c7f0:raise Required('owner_views_required')
  if pc==0x4cfd0e:raise Required('renderer_required')
  if pc==0x4d1350:self.events.append('rebuild')
  elif pc==0x57b390:self.events.append('organization')
  elif pc==0x4cfcc0:self.events.append('publication')
 def run(self):
  outcome='ok'
  try:self.call(0x4d1350,self.actor)
  except Required as err:outcome=str(err)
  masks=lambda p:self.r32(p+0xf0)|(self.r32(p+0xf4)<<32)
  return {'outcome':outcome,'actions':masks(self.actor),
    'member_actions':[masks(self.members[m['id']]) if m else None for m in self.c['members']]}
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def main():
 captured=HERE/'actual-saved-inputs.json';rows=[]
 for row in json.loads(captured.read_text()):
  for image in ('original','v14'):
   m=Owner(row['input'],image,row['precision']);rows.append({'image':image,'precision':row['precision'],'input':row['input'],'output':m.run()})
 assert {0x4d1350,0x57b390,0x4cfcc0,0x4d1e80,0x4d1f90,0x401060,0x4e1430,0x4e1e40}.issubset(h.VISITED), 'required native coverage missing'
 doc={'schema':1,'scope':__doc__,'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,
    'captured_inputs_sha256':sha(captured),'generator_sha256':sha(Path(__file__)),
    'state_registration_fixture_sha256':sha(REG),'instructions':[{'va':hex(a),'bytes':b} for a,b in sorted(h.VISITED.items())],'cases':rows}
 path=HERE/'native-owner-actions.json'
 if '--write' in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps({'cases':len(rows),'sha256':sha(path),'instructions':len(h.VISITED)}))
if __name__=='__main__':main()
