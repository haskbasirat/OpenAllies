"""Original5818D0 periodic property callback through actual Body heal/damage.

Expiry virtual methods, component removal feedback, derived-stat recomputation,
outer/ancestor graph and final actor effects are explicit fixture leaves. Native
5031E0/5034F0/5032D0 removal,581690 flags and5016xx getter loops execute unchanged.
Opaque temporary heal-record pointers are normalized to token1 in event output.
"""
from pathlib import Path
import copy,hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase6-combat'))
import original_damage as body
from unicorn.x86_const import *
bits=body.bits
PROPERTIES=body.MODIFIER-0x14
ARRAY=0x20b0000
EFFECTS=0x20b1000
DEFINITIONS=0x20b5000
VTABLE=0x20b8000
HEAL_ARRAY=0x20b8100
DAMAGE_ARRAY=0x20b8200
HEAL_DATA=0x20b8500
DAMAGE_DATA=0x20b9000
EXPIRE=0x20be000
REMOVED=0x20be010
PARENT_ACTOR=0x20bd000
ANCESTOR_ACTOR=0x20bd400

class Machine(body.Machine):
 def __init__(self,c):
  self.periodic=copy.deepcopy(c);self.calls=[];self.expiry_events=[];self.effect_addresses={}
  super().__init__(c['body_input'])
  s=c['state'];self.w(PROPERTIES-4,self.ACTOR);self.w(PROPERTIES+8,ARRAY);self.w(PROPERTIES+12,len(s['effects']))
  self.w(PROPERTIES-8+0xd8,s['modifier_flags'])
  self.w(VTABLE,EXPIRE);self.w(VTABLE+12,REMOVED)
  for i,effect in enumerate(s['effects']):
   p=EFFECTS+i*0x80;self.effect_addresses[p]=effect['handle'];d=DEFINITIONS+effect['property']*0x40
   self.w(ARRAY+i*4,p);self.w(p,VTABLE);self.w(p+12,d);self.w(p+0x10,int(effect['disabled']));self.w(p+0x14,effect['source_id']);self.w(d+0x24,effect['property_flags'])
  self.w(self.ACTOR+0x60,self.BODY if c['body_present'] else 0)
  self.w(PARENT_ACTOR+0x100,c['outer_flags']);self.w(ANCESTOR_ACTOR+0x100,c['ancestor_flags'])
  self.set_healing(s['healing']);self.set_damage(s['damage'])
 def set_healing(self,entries):
  self.w(PROPERTIES+0xbc,len(entries));self.w(body.MODIFIER+0xa4,HEAL_ARRAY)
  for i,value in enumerate(entries):self.w(HEAL_ARRAY+i*4,HEAL_DATA+i*0x20);self.w(HEAL_DATA+i*0x20+12,value)
 def set_damage(self,entries):
  self.w(PROPERTIES+0xb0,len(entries));self.w(body.MODIFIER+0x98,DAMAGE_ARRAY)
  for i,value in enumerate(entries):
   p=DAMAGE_DATA+i*0x40;self.w(DAMAGE_ARRAY+i*4,p);self.w(p,p+0x20)
   self.w(p+4,value['source_id']);self.w(p+8,value['allegiance_id']);self.w(p+0x2c,value['damage_type']);self.w(p+0x30,value['amount'])
 def props(self):
  effects=[]
  for i in range(self.r(PROPERTIES+12)):
   p=self.r(self.r(PROPERTIES+8)+i*4);d=self.r(p+12)
   effects.append({'handle':self.effect_addresses[p],'property':(d-DEFINITIONS)//0x40,'source_id':self.r(p+0x14),
                   'disabled':bool(self.r(p+0x10)&255),'property_flags':self.r(d+0x24)})
  healing=[self.r(self.r(self.r(body.MODIFIER+0xa4)+i*4)+12) for i in range(self.r(PROPERTIES+0xbc))]
  damages=[]
  for i in range(self.r(PROPERTIES+0xb0)):
   p=self.r(self.r(body.MODIFIER+0x98)+i*4);d=self.r(p)
   damages.append({'source_id':self.r(p+4),'allegiance_id':self.r(p+8),'damage_type':self.r(d+12),'amount':self.r(d+16)})
  return {'owner':3,'effects':effects,'modifier_flags':self.r(PROPERTIES-8+0xd8),'healing':healing,'damage':damages}
 def effect_event(self,name,p):
  self.expiry_events.append({'name':name,'handle':self.effect_addresses[p]})
 def hook(self,u,pc,size,user):
  this=u.reg_read(UC_X86_REG_ECX)
  if pc==EXPIRE:
   handle=self.effect_addresses[this];self.effect_event('expired',this)
   self.ret(value=int(handle in self.periodic['expired']));return
  if pc==REMOVED:self.effect_event('effect_removed',this);self.ret(4);return
  if pc==0x4ceb00:self.effect_event('actor_effect_removed',self.arg());self.ret(4);return
  if pc==0x502c60:
   self.effect_event('recompute_modifier',self.arg())
   if self.periodic['recompute_healing'] is not None:self.set_healing(self.periodic['recompute_healing'])
   if self.periodic['recompute_damage'] is not None:self.set_damage(self.periodic['recompute_damage'])
   self.ret(12);return
  if pc==0x6f7430:
   if self.arg() in self.effect_addresses:self.effect_event('release_effect',self.arg())
   self.ret();return
  if pc==0x4ced40:
   self.expiry_events.append({'name':'outer_actor'});self.ret(value=PARENT_ACTOR if self.periodic['outer_present'] else 0);return
  if pc==0x4ceda0 and this==PARENT_ACTOR:self.ret(value=ANCESTOR_ACTOR if self.periodic['ancestor_present'] else 0);return
  if pc==0x562ba0:
   self.calls.append({'kind':'heal','amount':self.r(self.arg()+4)})
  if pc==0x5625c0 and this==self.BODY:
   record=body.info(self.words(self.arg()))
   request={'record':record,'apply_type_modifier':bool(self.arg(1)&255),'weapon_id':None,'weapon_definition':None,'propagate_to_neighbors':bool(self.arg(3)&255)}
   self.calls.append({'kind':'damage','request':request})
   self.c['request']=request;self.c['source_present']=record['source_id']!=0;self.c['allegiance_present']=record['allegiance_id']!=0
  super().hook(u,pc,size,user)
 def run(self):
  self.w(self.STACK,self.STOP);self.w(self.STACK+4,0);self.w(self.STACK+8,0)
  self.u.reg_write(UC_X86_REG_ECX,PROPERTIES);self.u.reg_write(UC_X86_REG_ESP,self.STACK)
  regs={register:0x77cc0000+i for i,register in enumerate([UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP])}
  for register,value in regs.items():self.u.reg_write(register,value)
  try:self.u.emu_start(0x5818d0,self.STOP,count=300000)
  except Exception as exc:raise RuntimeError(hex(self.u.reg_read(UC_X86_REG_EIP))) from exc
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+12
  for register,value in regs.items():assert self.u.reg_read(register)==value
  for row in self.events:
   if isinstance(row['event'],dict):
    for key in ['HealingNotification','Recovered']:
     if key in row['event'] and row['event'][key]['token']!=0:row['event'][key]['token']=1
  return {'state':self.props(),'calls':self.calls,'expiry_events':self.expiry_events,'body':self.state(),'environment':self.env,'events':self.events}

def effect(handle,property=1,source=0x10017,disabled=False,flags=1):
 return {'handle':handle,'property':property,'source_id':source,'disabled':disabled,'property_flags':flags}
def damage(amount,kind=7):return {'source_id':0x10017,'allegiance_id':0x10018,'damage_type':kind,'amount':bits(amount)}
def base(name):
 c=body.base(name);c['modifier_present']=True
 return {'name':name,'body_input':c,'state':{'owner':3,'effects':[],'modifier_flags':0,'healing':[bits(5)],'damage':[damage(10)]},
         'expired':[],'body_present':True,'outer_present':False,'outer_flags':0,'ancestor_present':False,'ancestor_flags':0,
         'recompute_healing':None,'recompute_damage':None}
def cases():
 rows=[]
 def add(name,edit=None):
  c=base(name)
  if edit:edit(c)
  rows.append(c)
 add('heal_then_damage')
 add('multiple_entries',lambda c:c['state'].update(healing=[bits(5),bits(30),bits(10)],damage=[damage(10),damage(4),damage(7)]))
 add('no_body',lambda c:c.update(body_present=False))
 add('no_entries',lambda c:c['state'].update(healing=[],damage=[]))
 add('nonpositive_heals',lambda c:c['state'].update(healing=[bits(-1),0,bits(3)]))
 add('nonpositive_damage',lambda c:c['state'].update(damage=[damage(-1),damage(0),damage(3)]))
 add('full_health',lambda c:c['body_input']['body'].update(health=bits(100)))
 for flags in [0x100000,0x40000000,0x40100000,0x200000]:add(f'owner_flags_{flags}',lambda c,f=flags:c['body_input']['environment'].update(actor_flags=f))
 add('outer_blocks',lambda c:c.update(outer_present=True,outer_flags=0x100000))
 add('ancestor_blocks',lambda c:c.update(outer_present=True,ancestor_present=True,ancestor_flags=0x40000000))
 add('ancestor_unrelated_flags',lambda c:c.update(outer_present=True,ancestor_present=True,ancestor_flags=0x100000))
 add('expired_single',lambda c:(c['state'].update(effects=[effect(1)],modifier_flags=1),c.update(expired=[1])))
 add('expired_skips_shifted_next',lambda c:(c['state'].update(effects=[effect(1),effect(2,2,flags=2),effect(3,3,flags=4)],modifier_flags=7),c.update(expired=[1,2,3])))
 add('duplicate_reenables',lambda c:(c['state'].update(effects=[effect(1),effect(2,disabled=True)],modifier_flags=1),c.update(expired=[1])))
 add('disabled_expired',lambda c:(c['state'].update(effects=[effect(1,disabled=True)],modifier_flags=0),c.update(expired=[1])))
 add('zero_flags_no_recompute',lambda c:(c['state'].update(effects=[effect(1,flags=0)],modifier_flags=123),c.update(expired=[1])))
 add('stale_sources_match_null',lambda c:(c['state'].update(effects=[effect(1,source=0x20017),effect(2,source=0x30017)],modifier_flags=1),c.update(expired=[2])))
 add('removal_rebuilds_lists',lambda c:(c['state'].update(effects=[effect(1)],modifier_flags=1),c.update(expired=[1],recompute_healing=[bits(9),bits(13)],recompute_damage=[damage(1),damage(3)])))
 add('lethal_periodic',lambda c:c['state'].update(healing=[],damage=[damage(1000)]))
 add('zero_sources',lambda c:c['state'].update(damage=[{'source_id':0,'allegiance_id':0,'damage_type':7,'amount':bits(12)}]))
 add('health_effects',lambda c:c['body_input']['environment'].update(health_rules=[{'kind':2,'threshold':bits(.001)}]))
 for amount in [.1,.33333334,1.7,17.77,100000,1e-8]:add(f'fractional_{amount}',lambda c,a=amount:c['state'].update(healing=[bits(a)],damage=[damage(a*1.7)]))
 return rows
def main():
 fixtures=[];visited={}
 for pc in ['Single24','Double53','Extended64']:
  for c in cases():
   c=copy.deepcopy(c);c['body_input']['precision']=pc
   try:
    m=Machine(c);out=m.run();visited.update(m.executed)
   except Exception as exc:raise RuntimeError(f'{c["name"]} {pc}') from exc
   fixtures.append({'input':c,'output':out})
 windows=[]
 for address,size in sorted(visited.items()):
  raw=body.IMAGE.at(address,size);assert raw==body.ACCEPTED.at(address,size)
  windows.append({'address':hex(address),'bytes':raw.hex()})
 report={'schema':1,'passed':True,'original_sha256':body.pe.ORIGINAL_SHA,'accepted_sha256':body.pe.ACCEPTED_SHA,
         'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
         'oracle_dependencies':{p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest() for p in ['research/phase6-combat/original_damage.py','research/phase4-body/original_body.py','research/phase3-terrain/terrain_uv_oracle.py']},
         'method':__doc__,'cases':fixtures,'visited_windows':windows,
         'scope':'Actual periodic expiry, removal, flags, getter loops, healing and damage instructions; required graph, virtual feedback, derived stat rebuild, allocator and existing Body leaves are fixture callbacks. Visited windows include intercepted leaf entry bytes. Opaque nonzero stack heal-record event tokens map to 1.'}
 (HERE/'native-periodic-fixtures.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
 print(json.dumps({'cases':len(fixtures),'sha256':hashlib.sha256((HERE/'native-periodic-fixtures.json').read_bytes()).hexdigest()}))
if __name__=='__main__':main()
