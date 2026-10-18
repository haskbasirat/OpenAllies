"""Execute original hit4CA400 through original Body5625C0 and health561E50.

Native first-match damage modifiers, offensive preference, armor arithmetic,
minimum damage and Body sequencing run. Graph, effect, ownership and capture
leaves reuse the independently reviewed Body oracle's explicit callbacks.
"""
from pathlib import Path
import copy, hashlib, json, random, struct, sys

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase6-combat'))
import original_damage as body
from unicorn.x86_const import *
from capstone import Cs, CS_ARCH_X86, CS_MODE_32

bits=body.bits
RULE=0x20ab000
CONTEXT=0x20ab100
OFFENSE=0x20ab200
ATTACK_MODIFIER=0x20ab300
ATTACK_ORG=0x20ab400
ATTACK_LEADER=0x20ab500
ATTACK_LEADER_DATA=0x20ab600
ACTOR_ELIGIBLE=0x20ae100

class Machine(body.Machine):
 def __init__(self,c):
  self.weapon_case=copy.deepcopy(c)
  self.deliveries=[]
  super().__init__(c['body_input'])
  hit=c['hit'];weapon=hit['weapon'];rule=hit['rule']
  self.w(RULE+4,rule['kind']);self.w(RULE+0x14,rule['damage_type']);self.w(RULE+0x18,rule['amount'])
  self.w(self.ACTOR+0x60,self.BODY if c['target_has_body'] else 0)
  self.w(0x2091000+0xb4,ACTOR_ELIGIBLE)
  self.w(self.ACTOR+0x78,body.MODIFIER-0x1c if self.c['modifier_present'] else 0)
  self.w(CONTEXT,hit['event_kind'])
  self.w(CONTEXT+8,body.SOURCE if hit['source_actor'] is not None else 0)
  self.w(CONTEXT+12,body.ALLEGIANCE if hit['allegiance_actor'] is not None else 0)
  self.w(CONTEXT+0x1c,self.ACTOR if hit['target_actor'] is not None else 0)
  self.w(body.SOURCE+0x74,OFFENSE if c['source_offensive_component'] else 0)
  self.w(OFFENSE+4,body.SOURCE)
  self.w(body.SOURCE+0x78,ATTACK_MODIFIER-0x1c if c['source_modifier'] else 0)
  self.w(body.SOURCE+0x80,ATTACK_ORG if c['source_organization'] else 0)
  self.w(ATTACK_ORG+0x10,ATTACK_LEADER if c['source_leader'] else 0)
  self.w(ATTACK_LEADER+0x7c,ATTACK_LEADER_DATA)
  self.w(ATTACK_LEADER_DATA+0x84,c['leader_factor'])
  self.w(ATTACK_MODIFIER+0x58,c['offensive_primary']);self.w(ATTACK_MODIFIER+0x5c,c['offensive_secondary'])
  self.w(self.CACHE+0xa4,0x20abe00);self.w(0x20abe00,c['minimum'])
  if weapon:
   self.w(body.WEAPON+0x20,weapon['amount']);self.w(body.WEAPON_DATA+0x54,weapon['flags'])
 def hook(self,u,pc,size,user):
  if pc==ACTOR_ELIGIBLE:self.ret(value=int(self.weapon_case['target_eligible']));return
  if pc==0x4cf110:
   self.executed[pc]=size
   return # execute actual actor+78 getter for both attacker and defender
  if pc==0x5625c0 and u.reg_read(UC_X86_REG_ECX)==self.BODY:
   record=body.info(self.words(self.arg()))
   request={'record':record,'apply_type_modifier':bool(self.arg(1)&255),
            'weapon_id':1 if self.arg(2) else None,
            'weapon_definition':self.weapon_case['hit']['weapon']['propagation'] if self.arg(2) else None,
            'propagate_to_neighbors':bool(self.arg(3)&255)}
   self.deliveries.append(request)
   self.c['request']=request
  super().hook(u,pc,size,user)
 def run(self):
  hit=self.weapon_case['hit']
  self.w(self.STACK,self.STOP)
  for i,v in enumerate([body.WEAPON if hit['weapon'] else 0,CONTEXT,hit['scale']]):self.w(self.STACK+4+4*i,v)
  self.u.reg_write(UC_X86_REG_ECX,RULE);self.u.reg_write(UC_X86_REG_ESP,self.STACK)
  preserved={register:0x88990000+i for i,register in enumerate([UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP])}
  for register,value in preserved.items():self.u.reg_write(register,value)
  try:self.u.emu_start(0x4ca400,self.STOP,count=300000)
  except Exception as exc:raise RuntimeError(hex(self.u.reg_read(UC_X86_REG_EIP))) from exc
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP
  assert self.u.reg_read(UC_X86_REG_ESP)==self.STACK+16
  for register,value in preserved.items():assert self.u.reg_read(register)==value
  assert not self.mutations
  return {'dispatched':bool(self.u.reg_read(UC_X86_REG_EAX)&255),'requests':self.deliveries,
          'body':self.state(),'environment':self.env,'events':self.events}

def base(name):
 c=body.base(name)
 c['request']['weapon_id']=1;c['request']['weapon_definition']={'definition_word38':0,'definition_word4c':0}
 c['modifier_present']=True
 return {'name':name,'body_input':c,'hit':{
   'rule':{'kind':4,'damage_type':7,'amount':bits(999)},
   'weapon':{'id':1,'amount':bits(30),'flags':0,'propagation':c['request']['weapon_definition']},
   'event_kind':1,'source_actor':1,'allegiance_actor':2,'target_actor':3,'scale':bits(1),'time':c['environment']['world_time']},
   'target_has_body':True,'target_eligible':True,'source_offensive_component':True,
   'source_modifier':True,'source_organization':False,'source_leader':False,
   'leader_factor':bits(1.5),'offensive_primary':bits(1.2),'offensive_secondary':bits(.9),'minimum':bits(1)}

def cases():
 result=[]
 def add(name,edit=None):
  c=base(name)
  if edit:edit(c)
  h=c['hit'];bi=c['body_input']
  bi['request']['record']['source_id']=0x10017 if h['source_actor'] is not None else 0
  bi['request']['record']['allegiance_id']=0x10018 if h['allegiance_actor'] is not None else 0
  bi['source_present']=h['source_actor'] is not None
  bi['allegiance_present']=h['allegiance_actor'] is not None
  if h['weapon'] is None:bi['request']['weapon_id']=None;bi['request']['weapon_definition']=None
  result.append(c)
 add('ordinary_hit')
 add('target_absent',lambda c:c['hit'].update(target_actor=None))
 add('target_not_actor',lambda c:c.update(target_eligible=False))
 add('target_without_body',lambda c:c.update(target_has_body=False))
 add('source_absent',lambda c:c['hit'].update(source_actor=None))
 add('allegiance_absent',lambda c:c['hit'].update(allegiance_actor=None))
 add('no_offense_component',lambda c:c.update(source_offensive_component=False))
 add('no_offense_modifier',lambda c:c.update(source_modifier=False))
 add('defender_no_modifier',lambda c:c['body_input'].update(modifier_present=False))
 for kind in [0,1,3,5,99]:
  add(f'nonweapon_kind_{kind}',lambda c,k=kind:(c['hit']['rule'].update(kind=k,amount=bits(12.25)),c['hit'].update(scale=bits(500))))
 add('nonweapon_without_weapon',lambda c:(c['hit']['rule'].update(kind=3,amount=bits(12.25)),c['hit'].update(weapon=None)))
 for flags in [0,1,2,3,0xffffffff]:add(f'weapon_flags_{flags}',lambda c,f=flags:c['hit']['weapon'].update(flags=f))
 for amount in [-100,-0.0,0,1,8,1000]:add(f'weapon_amount_{amount}',lambda c,v=amount:c['hit']['weapon'].update(amount=bits(v)))
 for defense in [-5,0,1,29,30,31,1000]:add(f'defense_{defense}',lambda c,v=defense:c['body_input']['body'].update(defense=bits(v)))
 for factor in [-3,0,.1,1,2]:add(f'defender_factor_{factor}',lambda c,v=factor:c['body_input'].update(factors=[[7,bits(v)]]))
 add('defender_first_match',lambda c:c['body_input'].update(factors=[[7,bits(.3)],[7,bits(9)]]))
 add('defender_missing_type',lambda c:c['body_input'].update(factors=[[9,bits(.3)]]))
 for primary,secondary in [(0,0),(-2,-1),(0,.7),(1.5,9),(-1,2)]:add(f'offense_preference_{primary}_{secondary}',lambda c,p=primary,s=secondary:c.update(offensive_primary=bits(p),offensive_secondary=bits(s)))
 for factor in [-1,0,.25,1,3]:add(f'leader_factor_{factor}',lambda c,f=factor:c.update(source_organization=True,source_leader=True,leader_factor=bits(f)))
 add('organization_without_leader',lambda c:c.update(source_organization=True,source_leader=False))
 for minimum in [-5,0,.3,10]:add(f'minimum_{minimum}',lambda c,m=minimum:(c.update(minimum=bits(m)),c['hit']['weapon'].update(amount=bits(1))))
 for scale in [-1,0,.25,1.5,3.7]:add(f'scale_{scale}',lambda c,s=scale:c['hit'].update(scale=bits(s)))
 add('lethal_damage',lambda c:c['hit']['weapon'].update(amount=bits(1000)))
 add('nonweapon_negative_amount',lambda c:c['hit']['rule'].update(kind=1,amount=bits(-4)))
 add('nonweapon_negative_factor',lambda c:(c['hit']['rule'].update(kind=1,amount=bits(20)),c['body_input'].update(factors=[[7,bits(-1)]])))
 add('wall_propagation',lambda c:c['body_input']['view'].update(wall_link=True))
 add('wall_weapon_suppresses_propagation',lambda c:(c['body_input']['view'].update(wall_link=True),c['hit']['weapon']['propagation'].update(definition_word38=bits(1))))
 add('source_health_fx',lambda c:c['body_input']['environment'].update(health_rules=[{'kind':2,'threshold':bits(.01)}]))
 rng=random.Random(0x4ca400)
 for i in range(96):
  add(f'fractional_{i}',lambda c:(c['hit']['weapon'].update(amount=bits(rng.uniform(.001,250))),c['hit'].update(scale=bits(rng.uniform(.001,3))),c['body_input']['body'].update(defense=bits(rng.uniform(.001,80))),c.update(offensive_primary=bits(rng.uniform(.001,4))),c['body_input'].update(factors=[[7,bits(rng.uniform(.001,3))]])))
 return result

def main():
 fixtures=[];visited={}
 for precision in ['Single24','Double53','Extended64']:
  for case in cases():
   case=copy.deepcopy(case);case['body_input']['precision']=precision
   try:
    machine=Machine(case);out=machine.run();visited.update(machine.executed)
   except Exception as exc:raise RuntimeError(f'{case["name"]} {precision}') from exc
   fixtures.append({'input':case,'output':out})
 windows=[]
 for address,size in sorted(visited.items()):
  raw=body.IMAGE.at(address,size);assert raw==body.ACCEPTED.at(address,size)
  windows.append({'address':hex(address),'bytes':raw.hex()})
 report={'schema':1,'passed':True,'original_sha256':body.pe.ORIGINAL_SHA,'accepted_sha256':body.pe.ACCEPTED_SHA,
  'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
  'body_oracle_sha256':hashlib.sha256((ROOT/'research/phase6-combat/original_damage.py').read_bytes()).hexdigest(),
  'oracle_dependencies':{p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest() for p in ['research/phase6-combat/original_damage.py','research/phase4-body/original_body.py','research/phase3-terrain/terrain_uv_oracle.py']},
  'method':__doc__,'cases':fixtures,'visited_windows':windows,
  'scope':'Actual4CA400,4C8330,4CF110,57DBF0,501530,501650,5625C0,561E50; graph/virtual/presentation/capture leaves remain explicit fixture callbacks. Visited windows include intercepted leaf entry bytes.'}
 (HERE/'native-weapon-fixtures.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
 print(json.dumps({'cases':len(fixtures),'sha256':hashlib.sha256((HERE/'native-weapon-fixtures.json').read_bytes()).hexdigest()}))

if __name__=='__main__':main()
