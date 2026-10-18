"""Original562380 corpse initialization with explicit ability/world leaves."""
from pathlib import Path
import copy,hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-body'))
from original_body import Machine as Base,IMAGE,ACCEPTED,initial,environment,bits,OFFSETS,pe
from unicorn.x86_const import *
BODYVT=0x20b0000;CAPABILITY=0x20b1000;ALLEGIANCE=0x20b1100;DIMENSION=0x20b1200
OWNER=0x20b2000;MODIFIER=0x20b3000;ABILITY=0x20b4000;VALUES=0x20b5000;RETURN=0x20b6000

class Machine(Base):
 def __init__(self,c):
  self.c=copy.deepcopy(c);self.in_setup=True;self.height_index=0;self.dimension_index=0;self.visited={}
  super().__init__(c['body'],c['environment'],{'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[c['precision']],[])
  self.in_setup=False;self.mutations=copy.deepcopy(c['mutations'])
  self.w(self.BODY,BODYVT);self.w(BODYVT+0x38,CAPABILITY)
  self.w(0x2091000+0x108,ALLEGIANCE);self.w(0x2091000+8,DIMENSION)
  # Separate fixed stubs avoid reusing a translated RET8 block for RET0 when
  # switching from the terrain callback ABI to an actor virtual callback ABI.
  for pop in [0,8]:
   self.u.mem_write(RETURN+pop*0x100,b'\xd9\x05'+struct.pack('<I',VALUES)+b'\xd8\x05'+struct.pack('<I',VALUES+4)+(b'\xc2'+struct.pack('<H',pop) if pop else b'\xc3'))
  self.write_settings()
 def write_env(self):
  super().write_env();self.w(self.ACTOR+8,self.c['actor_flags']|int(self.env['destructed']))
 def write_settings(self):
  d=self.c['definition'];self.w(self.DATA+0x2a4,ABILITY if d['death_ability'] else 0)
  self.w(self.DATA+0x27c,d['rot_time']);self.w(self.DATA+0x280,d['sink_time'])
  self.w(self.ACTOR+0x78,MODIFIER if self.c['modifier_present'] else 0)
 def event(self,event):
  if self.in_setup:return
  self.events.append({'event':event,'body':self.state()})
  name=event if isinstance(event,str) else next(iter(event))
  if self.mutations and self.mutations[0]['on']==name:
   m=self.mutations.pop(0)
   for key,value in m.get('body',{}).items():self.w(self.BODY+OFFSETS[key],value)
   self.env.update(m.get('environment',{}));self.c['definition'].update(m.get('definition',{}))
   self.c.update(m.get('settings',{}));self.write_env();self.write_settings()
 def float_return(self,pair,pop):
  self.w(VALUES,pair[0]);self.w(VALUES+4,pair[1])
  self.u.reg_write(UC_X86_REG_EIP,RETURN+pop*0x100)
 def hook(self,u,pc,size,user):
  if 0x400000<=pc<0xc00000:self.visited[pc]=size
  if pc==CAPABILITY:self.event('BodyVirtual38');self.ret(value=int(self.c['retained']));return
  if pc==0x4f6760:
   assert self.arg()==self.ACTOR;self.event({'RemoveActor':{'argument':self.arg(1)}});self.ret(8);return
  if pc==0x4c8650:
   assert self.arg()==ABILITY
   self.event({'BeginAbility':{'definition':1}})
   u.mem_write(u.reg_read(UC_X86_REG_ECX),bytes(48));self.ret(4,value=u.reg_read(UC_X86_REG_ECX));return
  if pc==ALLEGIANCE:self.event('Allegiance');self.ret(value=OWNER if self.c['owner_present'] else 0);return
  if pc==0x4c9eb0:
   context=self.arg();assert self.arg(1)==MODIFIER
   values=[self.r(context+i*4) for i in range(12)]
   assert values[0]==2 and values[2]==self.ACTOR and values[3] in [0,OWNER]
   assert all(values[i]==0 for i in [1,4,5,6,7,10,11])
   request={'actor':self.env['actor_id'],'allegiance':1 if values[3] else None,'position':values[8:11],'kind':values[0]}
   self.event({'InvokeAbility':{'ability':1,'modifier':1,'request':request}});self.ret(8);return
  if pc==0x4ce080:self.event({'ReleaseAbility':{'ability':1}});self.ret();return
  if pc==0x4f0e50:
   position=[self.r(self.arg()+i*4) for i in range(3)]
   self.event({'TerrainHeight':{'flags':self.arg(1),'position':position}})
   pair=self.c['heights'][self.height_index];self.height_index+=1;self.float_return(pair,8);return
  if pc==DIMENSION:
   self.event({'Dimension':{'index':self.dimension_index}})
   pair=self.c['dimensions'][self.dimension_index];self.dimension_index+=1;self.float_return(pair,0);return
  if pc==0x50acc0:self.event({'ScheduleDecay':{'delay':self.arg(),'argument':self.arg(1)}});self.ret(8);return
  if pc==0x4e1510:
   assert u.reg_read(UC_X86_REG_ECX)==OWNER and self.arg()==self.ACTOR
   self.event({'RemoveFromAllegiance':{'allegiance':1}});self.ret(4);return
  super().hook(u,pc,size,user)
 def run(self):
  self.call(0x562380)
  assert not self.mutations,self.mutations
  return {'body':self.state(),'environment':self.env,'events':self.events}

def base(name):
 return {'name':name,'precision':'Double53','body':initial(0),'environment':environment(),
  'retained':True,'definition':{'death_ability':None,'rot_time':bits(2),'sink_time':bits(3)},'actor_flags':0,
  'modifier_present':True,'owner_present':True,'heights':[[bits(5),bits(2**-25)],[bits(2),bits(2**-27)]],
  'dimensions':[[bits(4),0],[bits(4),0],[bits(4),0]],'mutations':[]}
def cases():
 rows=[]
 def add(name,edit=None):
  c=base(name)
  if edit:edit(c)
  rows.append(c)
 add('ordinary');add('removed_immediately',lambda c:c.update(retained=False))
 add('no_owner',lambda c:c.update(owner_present=False))
 add('ability',lambda c:c['definition'].update(death_ability=1))
 add('ability_no_modifier',lambda c:(c['definition'].update(death_ability=1),c.update(modifier_present=False)))
 add('ability_no_owner',lambda c:(c['definition'].update(death_ability=1),c.update(owner_present=False)))
 for rot,sink in [(9,3),(10,0),(-2,1),(.1,.2),(0,0),(0,-1),(100000,100000)]:add(f'duration_{rot}_{sink}',lambda c,r=rot,s=sink:c['definition'].update(rot_time=bits(r),sink_time=bits(s)))
 for a,b,d in [(5,2,4),(2,2,4),(1,2,4),(5,2,0),(5,2,-1),(.1,.03,.17)]:add(f'terrain_{a}_{b}_{d}',lambda c,a=a,b=b,d=d:c.update(actor_flags=1<<21,heights=[[bits(a),0],[bits(b),0]],dimensions=[[bits(d),0]]*3))
 add('terrain_zero_sink',lambda c:(c.update(actor_flags=1<<21),c['definition'].update(sink_time=0)))
 add('terrain_negative_sink',lambda c:(c.update(actor_flags=1<<21),c['definition'].update(sink_time=bits(-1))))
 add('terrain_dimension_changes',lambda c:c.update(actor_flags=1<<21,dimensions=[[bits(4),0],[bits(5),bits(2**-24)],[bits(6),bits(2**-23)]]))
 add('ability_release_mutates_time',lambda c:(c['definition'].update(death_ability=1),c.update(mutations=[{'on':'ReleaseAbility','environment':{'world_time':bits(99)}}])))
 add('ability_changes_definition',lambda c:(c['definition'].update(death_ability=1),c.update(mutations=[{'on':'InvokeAbility','definition':{'sink_time':bits(7),'rot_time':bits(1)}}])))
 add('height_changes_actor_position',lambda c:c.update(actor_flags=1<<21,mutations=[{'on':'TerrainHeight','environment':{'owner_position':[bits(20),bits(30),bits(40)]}}]))
 add('dimension_changes_sink',lambda c:c.update(actor_flags=1<<21,mutations=[{'on':'Dimension','body':{'sink_duration':bits(2)}}]))
 add('schedule_removes_owner',lambda c:c.update(mutations=[{'on':'ScheduleDecay','settings':{'owner_present':False}}]))
 add('schedule_adds_owner',lambda c:c.update(owner_present=False,mutations=[{'on':'ScheduleDecay','settings':{'owner_present':True}}]))
 return rows
def main():
 rows=[];visited={}
 for pc in ['Single24','Double53','Extended64']:
  for c in cases():
   c=copy.deepcopy(c);c['precision']=pc
   try:m=Machine(c);out=m.run();visited.update(m.visited)
   except Exception as exc:raise RuntimeError(f'{c["name"]} {pc}') from exc
   rows.append({'input':c,'output':out})
 spans=[]
 for a,n in sorted(visited.items()):
  if spans and spans[-1][1]>=a:spans[-1][1]=max(spans[-1][1],a+n)
  else:spans.append([a,a+n])
 windows=[]
 for a,z in spans:
  raw=IMAGE.at(a,z-a);assert raw==ACCEPTED.at(a,z-a)
  windows.append({'start':hex(a),'end':hex(z),'sha256':hashlib.sha256(raw).hexdigest()})
 report={'schema':1,'passed':True,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
  'original_sha256':pe.ORIGINAL_SHA,'accepted_sha256':pe.ACCEPTED_SHA,'cases':rows,'native_windows':windows,
  'seams':['Body virtual38, ability constructor/execution/release, allegiance queries, world height, actor dimension, timer and ownership removal are explicit leaves.',
  'Full562380 control/arithmetic, action-context zero constructor4C8300 and property getter529C30 execute unchanged.',
  'Height/dimension leaves return a real x87FADD of two supplied binary32 numbers; actual terrain/world/ability implementation is not claimed.'],
  'window_scope':'Visited original byte pins include intercepted seam-entry addresses; execution behind those entries is not claimed.'}
 path=HERE/'dying-native-oracle.json';path.write_text(json.dumps(report,indent=2)+'\n')
 print(json.dumps({'cases':len(rows),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
