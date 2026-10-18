"""Whole5625C0 and561E50 instructions, with explicit mutable world leaves.

Linked/neighbor bodies are traced required dispatch boundaries, not dropped.
No native process or original file is modified. Definition registry identities,
capture eligibility and actual component/world effects remain fixture inputs.
"""
from pathlib import Path
import copy, hashlib, json, struct, sys
HERE=Path(__file__).resolve().parent; ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-body'))
from original_body import Machine as Base, IMAGE, ACCEPTED, initial, environment, OFFSETS, bits, pe
from unicorn.x86_const import *

INFO=0x20a0000; LINK_ACTOR=0x20a1000; LINK_BODY=0x20a1100
SOURCE=0x20a2000; ALLEGIANCE=0x20a2200; SOURCE_DATA=0x20a2400
WEAPON=0x20a3000; WEAPON_DATA=0x20a3400
WALL_LINK=0x20a4000; WALL_ACTOR=0x20a4400; WALL_DEF=0x20a4800; WALL_SEG=0x20a4c00
NEIGHBOR_F=0x20a5000; NEIGHBOR_B=0x20a5400; NEIGHBOR_FB=0x20a5800; NEIGHBOR_BB=0x20a5c00
ORG=0x20a6000; LEADER=0x20a6400; LEADER_ORG=0x20a6800; MIND=0x20a6c00
OUTER=0x20a7000; OUTER_SEG=0x20a7400; PARENT=0x20a7800; PARENT88=0x20a7c00; OWN88=0x20a8000
MODIFIER=0x20a8800; FACTORS=0x20a8c00; VT=0x20af000
HOSTILE=0x20ae000; OUTER_CALL=0x20ae010; MIND_CALL=0x20ae020

def info(words):
 return dict(zip(['source_id','allegiance_id','damage_type','amount','time','event_kind','reason'],words))
def record_words(record):return [record[k] for k in ['source_id','allegiance_id','damage_type','amount','time','event_kind','reason']]
def variant(name,**args):return {name:args} if args else name

class Machine(Base):
 def __init__(self,c):
  self.c=copy.deepcopy(c);self.local_record=None;self.executed={};self.in_setup=True
  super().__init__(c['body'],c['environment'],{'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[c['precision']],[])
  self.in_setup=False;self.mutations=copy.deepcopy(c.get('mutations',[]))
  self.w(self.ACTOR+0x60,self.BODY)
  self.w(0x2091000+0x140,HOSTILE);self.w(0x2091000+0x100,OUTER_CALL)
  self.w(MIND,VT);self.w(VT+0x120,MIND_CALL)
  self.w(SOURCE+4,SOURCE_DATA);self.w(SOURCE+0x14,c['request']['record']['source_id'])
  self.w(ALLEGIANCE+0x14,c['request']['record']['allegiance_id'])
  self.w(LINK_ACTOR+0x60,LINK_BODY);self.w(self.ACTOR+0x168,0x10009 if c['environment']['has_linked_body'] else 0)
  self.register(0x10009,LINK_ACTOR)
  self.register(c['request']['record']['source_id'],SOURCE if c['source_present'] else 0)
  self.register(c['request']['record']['allegiance_id'],ALLEGIANCE if c['allegiance_present'] else 0)
  for i,value in enumerate(c['source_position']):self.w(SOURCE+0x20+i*4,value)
  self.w(WEAPON+4,WEAPON_DATA if c['request']['weapon_definition'] else 0)
  if c['request']['weapon_definition']:
   self.w(WEAPON_DATA+0x38,c['request']['weapon_definition']['definition_word38'])
   self.w(WEAPON_DATA+0x4c,c['request']['weapon_definition']['definition_word4c'])
  self.w(WALL_LINK+0x10,WALL_ACTOR if c['wall_present'] else 0)
  self.w(WALL_ACTOR+4,WALL_DEF);self.w(WALL_ACTOR+0x98,WALL_SEG if c['segments_present'] else 0)
  self.w(WALL_DEF+0x4ac,c['transfer_factor'])
  self.w(WALL_SEG+0x24,FACTORS+0x800);self.w(FACTORS+0x800,NEIGHBOR_F);self.w(FACTORS+0x804,NEIGHBOR_B)
  self.w(NEIGHBOR_F+0x60,NEIGHBOR_FB);self.w(NEIGHBOR_B+0x60,NEIGHBOR_BB)
  self.w(LEADER+0x7c,LEADER_ORG);self.w(OUTER+0x98,OUTER_SEG)
  self.w(PARENT+0x88,PARENT88 if c['ancestor_component'] else 0)
  self.w(MODIFIER+0x74,FACTORS);self.w(MODIFIER+0x78,len(c['factors']))
  self.w(MODIFIER+0x50,c['normalization_factor'])
  for i,(kind,factor) in enumerate(c['factors']):
   ptr=FACTORS+0x100+i*8;self.w(FACTORS+i*4,ptr);self.w(ptr,kind);self.w(ptr+4,factor)
  for i,value in enumerate(record_words(c['request']['record'])):self.w(INFO+i*4,value)
  self.write_view()
 def register(self,id,ptr):
  slot=id&65535;self.u.mem_write(self.REGISTRY+4+slot*2,struct.pack('<H',id>>16));self.w(self.REGISTRY+0x20004+slot*4,ptr)
 def words(self,p):return [self.r(p+i*4) for i in range(7)]
 def write_view(self):
  v=self.c['view'];self.w(self.ACTOR+0x90,WALL_LINK if v['wall_link'] else 0)
  self.w(self.ACTOR+0x80,ORG if v['organization'] else 0);self.w(ORG+0x10,LEADER if v['organization_leader'] else 0)
  self.w(self.ACTOR+0x70,MIND if v['mind'] else 0);self.w(self.ACTOR+0x88,OWN88 if v['component88'] else 0)
 def event(self,event):
  if getattr(self,'in_setup',False):return
  self.events.append({'event':event,'body':self.state()})
  key=event if isinstance(event,str) else next(iter(event))
  if key=='Action':key=event['Action']['kind'] if isinstance(event['Action']['kind'],str) else next(iter(event['Action']['kind']))
  if self.mutations and self.mutations[0]['on']==key:
   m=self.mutations.pop(0)
   for k,v in m.get('body',{}).items():
    if k=='last_damage':
     for i,value in enumerate(v):self.w(self.BODY+0x28+4*i,value)
    else:self.w(self.BODY+OFFSETS[k],v)
   self.env.update(m.get('environment',{}));self.write_env()
   self.c['view'].update(m.get('view',{}));self.write_view()
   for k,v in m.get('settings',{}).items():self.c[k]=v
   if 'record' in m:
    assert self.local_record
    for key,value in m['record'].items():self.w(self.local_record+4*list(info([0]*7)).index(key),value)
 def action(self,kind,record=None):
  self.event({'Action':{'kind':kind,'record':info(self.words(record or self.local_record))}})
 def hook(self,u,pc,size,user):
  if 0x400000<=pc<0xc00000:
   self.executed[pc]=size
  ecx=u.reg_read(UC_X86_REG_ECX)
  if pc==0x5625c0 and ecx!=self.BODY:
   target='LinkedBody' if ecx==LINK_BODY else {'NeighborBody':1 if ecx==NEIGHBOR_FB else 2}
   self.event({'PropagateDamage':{'target':target,'record':info(self.words(self.arg())),
     'apply_type_modifier':bool(self.arg(1)&255),'weapon':bool(self.arg(2)), 'propagate_to_neighbors':bool(self.arg(3)&255)}})
   self.ret(16);return
  if pc==0x561e50 and ecx!=self.BODY:
   self.event({'LinkedHealth':{'requested':self.arg(),'token':self.arg(1),'notify':bool(self.arg(2)), 'source':self.arg(3)}})
   self.ret(16);return
  if pc==0x562843:self.local_record=u.reg_read(UC_X86_REG_ESP)+0x18
  if pc==HOSTILE:
   assert self.arg()==(ALLEGIANCE if self.c['allegiance_present'] else 0)
   self.event('Hostile');self.ret(4,int(self.c['hostile']));return
  if pc==OUTER_CALL:self.event('OuterActor');self.ret(value=OUTER if self.c['outer_present'] else 0);return
  if pc==0x4cf110:self.ret(value=MODIFIER if self.c['modifier_present'] else 0);return
  if pc==0x585450:self.ret(4,self.c['wall_index']);return
  if pc==0x585480:self.ret(12,(0 if self.arg(1) else 1) if self.c['neighbors'][0 if self.arg(1) else 1] else 0xffffffff);return
  if pc==0x4d6660:self.action('ResetRecovery');self.ret();return
  if pc==0x579490:self.action(variant('OrganizationElementDamaged',leader=1));self.ret(8);return
  if pc==0x584cd0:self.action(variant('OuterActorDamaged',component98=1));self.ret(8);return
  if pc==0x501770:self.action(variant('ModifierOnAttack',modifier=1,source=1));self.ret(8);return
  if pc==MIND_CALL:self.action(variant('MindDamaged',mind=1));self.ret(4);return
  if pc==0x50d0b0:self.action('PresentationDamaged');self.ret(8);return
  if pc==0x4d6190:self.action(variant('SourceType3',source=1,weapon=self.c['request']['weapon_id']));self.ret(8);return
  if pc==0x570f70:self.action(variant('OrganizationDamaged',organization=1));self.ret(4);return
  if pc==0x4ceda0:self.ret(value=PARENT if self.c['ancestor_present'] else 0);return
  if pc==0x56ae70:self.action(variant('Component88Damaged',component=2 if ecx==PARENT88 else 1));self.ret(4);return
  if pc==0x4d1120:self.event('CaptureEligible');self.ret(value=int(self.c['capture_eligible']));return
  if pc==0x4d5230:self.event('AttemptCapture');self.ret(8,int(self.c['capture_success']));return
  if pc==0x453ed0:
   self.w(0x20af800,self.c['capture_factor']);u.mem_write(0x20af900,b'\xd9\x05'+struct.pack('<I',0x20af800)+b'\xc2\x04\x00');u.reg_write(UC_X86_REG_EIP,0x20af900);return
  if pc==0x584e70:self.action(variant('WallMemberKilled',component98=1));self.ret(4);return
  if pc==0x4f1a90:self.action('QueueDeath');self.ret(4);return
  if pc==0x404f60:self.ret();return
  super().hook(u,pc,size,user)
 def run(self):
  r=self.c['request'];self.call(0x5625c0,[INFO,int(r['apply_type_modifier']),WEAPON if r['weapon_id'] is not None else 0,int(r['propagate_to_neighbors'])],16)
  assert not self.mutations,self.mutations
  return {'body':self.state(),'environment':self.env,'events':self.events}

def base(name):
 return {'name':name,'precision':'Double53','body':initial(),'environment':environment(),
  'request':{'record':{'source_id':0x10017,'allegiance_id':0x10018,'damage_type':7,'amount':bits(25),'time':bits(12),'event_kind':1,'reason':10},'apply_type_modifier':False,'weapon_id':None,'weapon_definition':None,'propagate_to_neighbors':True},
  'view':dict(wall_link=False,organization=False,organization_leader=False,mind=False,component88=False),
  'source_present':True,'allegiance_present':True,'source_position':[bits(7),bits(16),bits(1)],
  'wall_present':True,'segments_present':True,'transfer_factor':bits(.25),'wall_index':0,'neighbors':[True,True],
  'hostile':True,'modifier_present':False,'factors':[[7,bits(.5)]], 'outer_present':False,'normalization_factor':bits(1),
  'ancestor_present':False,'ancestor_component':False,'capture_eligible':False,'capture_success':False,'capture_factor':bits(.3),'mutations':[]}
def cases():
 rows=[]
 def add(name,edit=None):
  c=base(name)
  if edit:edit(c)
  rows.append(c)
 for amount in [-1,0,1,25,75,100,100000,.0001]:add('amount_'+str(amount),lambda c,v=amount:c['request']['record'].update(amount=bits(v)))
 add('destructed',lambda c:c['environment'].update(destructed=True))
 add('friendly',lambda c:c.update(hostile=False))
 add('allegiance_stale',lambda c:c.update(allegiance_present=False,hostile=False))
 add('source_stale',lambda c:c.update(source_present=False))
 add('all_components',lambda c:(c['view'].update(organization=True,organization_leader=True,mind=True,component88=True),c.update(outer_present=True,modifier_present=True)))
 for factor in [0,.1,.5,1,2,-1]:add('modifier_'+str(factor),lambda c,v=factor:(c['request'].update(apply_type_modifier=True),c.update(modifier_present=True,factors=[[7,bits(v)]])))
 add('modifier_first_match',lambda c:(c['request'].update(apply_type_modifier=True),c.update(modifier_present=True,factors=[[7,bits(.5)],[7,bits(3)]])))
 add('modifier_no_match',lambda c:(c['request'].update(apply_type_modifier=True),c.update(modifier_present=True,factors=[[8,bits(3)]])))
 add('recovery_suppressed',lambda c:c['environment'].update(actor_flags=0x100000))
 add('no_recovery',lambda c:c['environment'].update(has_recovery_component=False))
 for kind in [0,1,2,3,4,10]:add('event_kind_'+str(kind),lambda c,v=kind:(c['request']['record'].update(event_kind=v),c['view'].update(mind=True),c.update(modifier_present=True)))
 for parent,component in [(False,False),(True,False),(True,True)]:add(f'ancestor_{parent}_{component}',lambda c,p=parent,v=component:(c.update(ancestor_present=p,ancestor_component=v),c['view'].update(component88=True)))
 for hostile,capture,success,flag,destructible in [(True,True,True,0,True),(True,True,False,0,True),(False,True,False,0,True),(True,True,False,0x40000000,True),(True,False,False,0,False),(True,False,False,0,True)]:
  add(f'lethal_{hostile}_{capture}_{success}_{flag}_{destructible}',lambda c,h=hostile,ca=capture,s=success,f=flag,d=destructible:(c['request']['record'].update(amount=bits(100)),c.update(hostile=h,capture_eligible=ca,capture_success=s),c['environment'].update(actor_flags=f,destructible=d)))
 add('linked',lambda c:c['environment'].update(has_linked_body=True))
 add('linked_zero_before_gate',lambda c:(c['environment'].update(has_linked_body=True),c['request']['record'].update(amount=0)))
 for factor,index,neighbors in [(.25,0,[True,True]),(.25,-1,[True,True]),(.25,0,[False,True]),(0,0,[True,True]),(-.5,0,[True,True])]:add(f'wall_{factor}_{index}_{neighbors}',lambda c,f=factor,i=index,n=neighbors:(c['view'].update(wall_link=True),c.update(transfer_factor=bits(f),wall_index=i,neighbors=n)))
 add('wall_absent',lambda c:(c['view'].update(wall_link=True),c.update(wall_present=False)))
 add('wall_segments_absent',lambda c:(c['view'].update(wall_link=True),c.update(segments_present=False)))
 add('wall_input_flag_false',lambda c:(c['view'].update(wall_link=True),c['request'].update(propagate_to_neighbors=False)))
 add('wall_lethal',lambda c:(c['view'].update(wall_link=True),c['request']['record'].update(amount=bits(100))))
 for x,y in [(0,0),(1,0),(0,1),(-1,-1)]:add(f'weapon_propagation_{x}_{y}',lambda c,x=x,y=y:(c['view'].update(wall_link=True),c['request'].update(weapon_id=1,weapon_definition={'definition_word38':bits(x),'definition_word4c':bits(y)})))
 for kind in range(4):add(f'health_fx_{kind}',lambda c,k=kind:(c['environment'].update(health_rules=[dict(kind=k,threshold=bits(.5 if k<2 else .1))]),c['request']['record'].update(amount=bits(50))))
 add('health_fx_near_source',lambda c:(c['environment'].update(health_rules=[dict(kind=0,threshold=bits(.5))]),c.update(source_position=c['environment']['owner_position'][:]),c['request']['record'].update(amount=bits(50))))
 add('health_fx_stale_source',lambda c:(c['environment'].update(health_rules=[dict(kind=0,threshold=bits(.5))]),c.update(source_present=False),c['request']['record'].update(amount=bits(50))))
 for kind in [1,3]:add(f'health_fx_gain_{kind}',lambda c,k=kind:(c['body'].update(health=bits(25)),c['environment'].update(health_rules=[dict(kind=k,threshold=bits(.5 if k==1 else .1))]),c['request'].update(apply_type_modifier=True),c['request']['record'].update(amount=bits(50)),c.update(modifier_present=True,factors=[[7,bits(-1)]])))
 add('health_fx_xy_zero_z_retained',lambda c:(c['environment'].update(health_rules=[dict(kind=0,threshold=bits(.5))]),c.update(source_position=[bits(10),bits(20),bits(-7)]),c['request']['record'].update(amount=bits(50))))
 for factor in [0.33333334,1.5,3.25]:add(f'health_fx_derived_extended_{factor}',lambda c,f=factor:(c['environment'].update(actor_flags=0x20000000,derived_normalization_maximum=bits(137.125),health_rules=[dict(kind=2,threshold=bits(.001))]),c.update(modifier_present=True,normalization_factor=bits(f))))
 add('callback_mutates_health',lambda c:c.update(mutations=[{'on':'RefreshActorStance','body':{'health':bits(1)}}]))
 add('callback_kills_actor',lambda c:c.update(mutations=[{'on':'PresentationDamaged','body':{'health':0}}]))
 add('callback_changes_kind',lambda c:c.update(mutations=[{'on':'PresentationDamaged','record':{'event_kind':3}}]))
 add('callback_adds_org',lambda c:c.update(mutations=[{'on':'PresentationDamaged','view':{'organization':True}}]))
 add('callback_first_neighbor_removes_second',lambda c:(c['view'].update(wall_link=True),c.update(mutations=[{'on':'PropagateDamage','settings':{'neighbors':[True,False]}}])))
 add('linked_callback_changes_health',lambda c:(c['environment'].update(has_linked_body=True),c.update(mutations=[{'on':'LinkedHealth','body':{'health':bits(40)}}])))
 add('callback_capture_reject_heals',lambda c:(c['request']['record'].update(amount=bits(100)),c.update(mutations=[{'on':'CaptureEligible','body':{'health':bits(12)}}])))
 add('callback_hostile_changes_maximum',lambda c:c.update(mutations=[{'on':'Hostile','body':{'maximum':bits(40)}}]))
 return rows
def main():
 fixtures=[];visited={}
 for pc in ['Single24','Double53','Extended64']:
  for c in cases():
   c=copy.deepcopy(c);c['precision']=pc
   try:
    m=Machine(c);result=m.run();visited.update(m.executed)
   except Exception as exc:raise RuntimeError(f'{c["name"]} {pc}') from exc
   fixtures.append({'input':c,'output':result})
 spans=[]
 for a,n in sorted(visited.items()):
  if spans and spans[-1][1]>=a:spans[-1][1]=max(spans[-1][1],a+n)
  else:spans.append([a,a+n])
 windows=[]
 for a,z in spans:
  raw=IMAGE.at(a,z-a);assert raw==ACCEPTED.at(a,z-a)
  windows.append({'start':hex(a),'end':hex(z),'sha256':hashlib.sha256(raw).hexdigest()})
 report={'schema':1,'passed':True,'original_sha256':pe.ORIGINAL_SHA,'accepted_sha256':pe.ACCEPTED_SHA,
  'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'cases':fixtures,'native_windows':windows,
  'window_scope':'Visited native entry/instruction byte pins include intercepted seam entry addresses; no execution behind those seams is claimed.',
  'seams':['Linked/neighbor body dispatch, virtual hostility/outer actor, graph lookups, capture eligibility/attempt, component events and world death queues are explicit world leaves.','Original5625C0 arithmetic, native561E50 setter and directional FX,501650 first-match damage factor,45D430/4C8100 weapon predicates execute.','Diagnostics are ignored; valid source objects are provided in friendly-fire log cases.','All three PC modes are explicit; no process-wide default or graph integration is inferred.']}
 path=HERE/'damage-native-oracle.json';path.write_text(json.dumps(report,indent=2)+'\n')
 print(json.dumps({'cases':len(fixtures),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
