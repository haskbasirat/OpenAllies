"""Full native4C8F20/4C8740/4C8A40/4C5840 with required actor virtual,
geometry and economic predicates supplied explicitly. Native modifier access,
effect iteration, target masks, recharge and branch order execute unchanged.
"""
from pathlib import Path
import copy,hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine as Base,original,modified
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
ACTOR,TARGET,KINGDOM,DEF,LOCAL,ABILITY,CONTEXT,VT,PROPS,TPROPS,PERF,BODY,EFFECTS,LIST,REG,WORLD,SVARS,VARIABLES=range(0x4000000,0x4012000,0x1000)
IS_ACTOR,IS_KINGDOM,ALLEGIANCE,FRIEND,ALLY,HEAL,PROPERTY=range(0x4020000,0x4020070,0x10)
handles={1:ACTOR,2:TARGET,3:KINGDOM};ids={v:k for k,v in handles.items()}
bits=lambda f:struct.unpack('<I',struct.pack('<f',f))[0]
class Machine(Base):
 def __init__(self,c):
  self.c=copy.deepcopy(c);super().__init__({'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[c['precision']]);self.u.mem_map(0,4096);self.u.mem_map(0x4000000,0x50000);self.events=[];self.executed={}
  self.w(0xb36534,WORLD);self.w(WORLD+0xe0,c['now']);self.w(0xb364c0,REG)
  self.w(0xb39ab0,SVARS);self.w(SVARS+0x1c0,VARIABLES);self.u.mem_write(VARIABLES,bytes([c['free_recharge']]))
  self.w(SVARS+0x1a4,VARIABLES+4);self.w(VARIABLES+4,c['minimum_range_setting'])
  for slot,callback in [(0xa8,IS_KINGDOM),(0xb4,IS_ACTOR),(0x108,ALLEGIANCE),(0x138,FRIEND),(0x140,ALLY)]:self.w(VT+slot,callback)
  for h,address in handles.items():
   v=c['actors'][str(h)];self.w(address,VT);self.w(address+4,address+0x600);self.w(address+0x600+0x1b8,v['class_flags']);self.w(address+8,v['flags8']);self.w(address+0x100,v['flags100']);self.w(address+0x104,v['flags104']);self.w(address+0x190,v['kingdom_kind']);self.put(address+0x20,*v['position'])
   self.w(address+0x7c,1 if v['has_regiment'] else 0);self.w(address+0x80,1 if v['has_organization'] else 0);self.w(address+0xa8,1 if v['has_summoner'] else 0)
   self.w(address+0x78,(PROPS if h==1 else TPROPS) if v['has_properties'] else 0)
   self.w(address+0x74,PERF if v['mana'] is not None else 0)
   if v['mana'] is not None:self.w(PERF+0x20,v['mana'])
   self.w(address+0x60,BODY if v['body'] is not None else 0)
   if v['body'] is not None:self.put(BODY+0x10,*v['body'])
   self.w(address+0x70,ACTOR+0x800 if v['has_mind'] else 0)
  self.w(ACTOR+0x800,VT+0x400);self.w(VT+0x400+0xa8,HEAL)
  self.w(PROPS+0x1c+0x64,c['recharge_factor']);self.w(PROPS+0x1c+0x28,c['projectile_delta']);self.w(PROPS+0x1c+0x2c,c['bombard_delta'])
  self.w(TPROPS+0x1c+0x68,TPROPS+0x300);self.w(TPROPS+0x1c+0x6c,len(c['immune_classes']))
  for i,kind in enumerate(c['immune_classes']):self.w(TPROPS+0x300+i*4,kind)
  d=c['definition'];self.w(ABILITY+4,DEF);self.w(ABILITY+8,LOCAL if c['local'] is not None else 0)
  for field,offset in [('last_use',0x10),('fuzzy_range',0x1c),('power',0x20)]:self.w(ABILITY+offset,c['ability'][field])
  self.u.mem_write(ABILITY+0x19,bytes([c['ability']['recharge_bypass']]))
  for field,offset in [('minimum_range',0x1c),('recharge_time',0x20),('range',0x24),('area',0x38),('mana_cost',0x40),('volley_area',0x4c),('flags',0x54)]:self.w(DEF+offset,d[field])
  self.put(DEF+0x28,d['targets']['required'],d['targets']['forbidden']);self.w(DEF+0x3c,1 if d['projectile'] else 0)
  if c['local'] is not None:
   for field,offset in [('recharge_time',0x18),('fuzzy_range',0x1c),('turret_index',0x54)]:self.w(LOCAL+offset,c['local'][field])
  self.w(VT+0x600+4,PROPERTY)
  for i,e in enumerate(c['effects']):
   p=EFFECTS+i*0x100;self.w(p,VT+0x600);self.w(p+4,e['kind']);self.w(p+0x1c,e['summon_filter']['required']);self.w(p+0x20,e['summon_filter']['forbidden'])
   self.w(p+0x80+0x64,e['immunity_class']);self.put(LIST+i*8,p,LIST+(i+1)*8 if i+1<len(c['effects']) else 0)
  self.w(DEF+0x68,LIST if c['effects'] else 0)
  self.listptr=LIST+0x400
  for i,id in enumerate(c['ability']['summoned_ids']):self.put(self.listptr+i*16,id,self.listptr+(i+1)*16 if i+1<len(c['ability']['summoned_ids']) else 0,self.listptr+(i-1)*16 if i else 0)
  self.w(ABILITY+0x24,self.listptr if c['ability']['summoned_ids'] else 0);self.w(ABILITY+0x28,self.listptr+(len(c['ability']['summoned_ids'])-1)*16 if c['ability']['summoned_ids'] else 0)
  for id in c['live_ids']:
   self.u.mem_write(REG+4+(id&0xffff)*2,struct.pack('<H',id>>16));self.w(REG+0x20004+(id&0xffff)*4,TARGET)
  self.w(KINGDOM+0x284,LIST+0x800 if c['owned'] else 0)
  for i,h in enumerate(c['owned']):self.put(LIST+0x800+i*8,handles[h],LIST+0x800+(i+1)*8 if i+1<len(c['owned']) else 0)
  ctx=c['context'];self.put(CONTEXT,ctx['event_kind'],ctx['phase'],handles.get(ctx['source'],0),handles.get(ctx['allegiance'],0),*ctx['source_position'],handles.get(ctx['target'],0),*ctx['target_position'],ctx['word2c'])
  self.u.hook_add(UC_HOOK_CODE,self.hook)
 def put(self,a,*w):self.u.mem_write(a,struct.pack('<'+'I'*len(w),*(v&0xffffffff for v in w)))
 def arg(self,i=0):return self.r(self.u.reg_read(UC_X86_REG_ESP)+4+4*i)
 def ret(self,value=0,pop=0):
  sp=self.u.reg_read(UC_X86_REG_ESP);self.u.reg_write(UC_X86_REG_EAX,value);self.u.reg_write(UC_X86_REG_EIP,self.r(sp));self.u.reg_write(UC_X86_REG_ESP,sp+4+pop)
 def hook(self,u,pc,size,_):
  if original.base<=pc<original.base+original.image_size:
   raw=original.range(pc,size);assert raw==modified.range(pc,size);self.executed[pc]=raw.hex()
  owner=ids.get(u.reg_read(UC_X86_REG_ECX))
  if pc==IS_ACTOR:self.ret(int(self.c['actors'][str(owner)]['is_actor']))
  elif pc==IS_KINGDOM:self.ret(int(self.c['actors'][str(owner)]['is_kingdom']))
  elif pc==ALLEGIANCE:self.ret(handles.get(self.c['actors'][str(owner)]['allegiance'],0))
  elif pc in [FRIEND,ALLY]:self.ret(int(self.c['friendly' if pc==FRIEND else 'allied']),4)
  elif pc==HEAL:self.ret(int(self.c['heal_override']))
  elif pc==PROPERTY:
   index=(u.reg_read(UC_X86_REG_ECX)-EFFECTS)//0x100;self.ret(EFFECTS+index*0x100+0x80 if self.c['effects'][index]['property'] else 0)
  elif pc in [0x4ee800,0x4ee7c0,0x46e290]:
   inverse=bool(self.arg(2 if pc==0x4ee800 else 3));self.events.append({'name':'range','range':self.arg(),'inverse':inverse});self.ret(int(self.c['min_pass' if inverse else 'max_pass']),12 if pc==0x4ee800 else 16)
  elif pc==0x4c76e0:self.events.append({'name':'turret'});self.ret(int(self.c['turret_pass']),12)
  elif pc==0x4e24b0:self.events.append({'name':'payment'});self.ret(int(self.c['payment']),4)
  elif pc==0x4d1120:self.ret(int(self.c['injured']))
  elif pc==0x4549c0:self.ret(handles.get(self.c['actors'][str(owner)]['leader'],0))
  elif pc==0x4c5810:self.ret(int(self.c['summoner_accepts']),4)
  elif pc==0x581280:self.ret(int(self.c['already_has_property']),8)
  elif pc==0x50c510:
   p=self.arg();n=self.r(p+4);prev=self.r(p+8)
   if prev:self.w(prev+4,n)
   else:self.w(ABILITY+0x24,n)
   if n:self.w(n+8,prev)
   else:self.w(ABILITY+0x28,prev)
   self.ret(pop=4)
 def run(self):
  self.reset(0);self.put(self.STACK+4,CONTEXT,int(self.c['check_range']));self.u.reg_write(UC_X86_REG_ECX,ABILITY)
  regs={r:0x77440000+i for i,r in enumerate([UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP])}
  for r,v in regs.items():self.u.reg_write(r,v)
  try:self.u.emu_start(0x4c8f20,self.STOP,count=100000)
  except Exception as exc:raise RuntimeError(hex(self.u.reg_read(UC_X86_REG_EIP))) from exc
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+12
  for r,v in regs.items():assert self.u.reg_read(r)==v
  w=struct.unpack('<12I',self.u.mem_read(CONTEXT,48));ctx={'event_kind':w[0],'phase':w[1],'source':ids.get(w[2]),'allegiance':ids.get(w[3]),'source_position':list(w[4:7]),'target':ids.get(w[7]),'target_position':list(w[8:11]),'word2c':w[11]}
  summoned=[];p=self.r(ABILITY+0x24)
  while p:summoned.append(self.r(p));p=self.r(p+4);assert len(summoned)<100
  return {'code':self.u.reg_read(UC_X86_REG_EAX),'context':ctx,'summoned':summoned,'events':self.events}

def actor(h):return {'flags8':0x4000,'flags100':0,'flags104':0,'position':list(map(bits,[h*3,h*7,1])),'has_regiment':False,'has_organization':False,'has_properties':False,'has_summoner':False,'has_mind':False,'mana':bits(20) if h==1 else None,'body':[bits(50),bits(100)] if h==2 else None,'kingdom_kind':0,'class_flags':0,'is_actor':h!=3,'is_kingdom':h==3,'allegiance':3,'leader':None}
def base(name):
 return {'name':name,'precision':'Double53','now':bits(10),'minimum_range_setting':bits(1),'free_recharge':False,
 'actors':{str(h):actor(h) for h in handles},'friendly':False,'allied':False,'heal_override':False,'injured':True,'summoner_accepts':False,
 'recharge_factor':bits(1),'projectile_delta':0,'bombard_delta':0,'effects':[],'already_has_property':False,'immune_classes':[],
 'ability':{'definition':10,'local_definition':11,'use_count':0,'last_use':bits(-100),'word14':0,'enabled':True,'recharge_bypass':False,'fuzzy_range':0,'power':bits(12),'summoned_ids':[],'word28':0},
 'definition':{'minimum_range':0,'recharge_time':bits(2),'range':bits(15),'targets':{'required':0,'forbidden':0},'area':0,'projectile':False,'mana_cost':bits(3),'volley_area':0,'flags':0,'effects':[]},
 'local':{'recharge_time':bits(3),'fuzzy_range':bits(.5),'turret_index':-1},
 'context':{'event_kind':1,'phase':0,'source':1,'allegiance':None,'source_position':[0,0,0],'target':2,'target_position':[bits(6),bits(14),bits(1)],'word2c':0},
 'check_range':True,'min_pass':True,'max_pass':True,'turret_pass':True,'payment':True,'live_ids':[],'owned':[]}
def effect(kind=4,prop=False,cl=7):return{'kind':kind,'property':prop,'immunity_class':cl,'summon_filter':{'required':0,'forbidden':0}}
def cases():
 rows=[]
 def add(name,edit=None):
  c=base(name)
  if edit:edit(c)
  c['definition']['effects']=list(range(100,100+len(c['effects'])))
  if c['local'] is None:c['ability']['local_definition']=None
  rows.append(c)
 add('allowed')
 add('no_source',lambda c:c['context'].update(source=None))
 add('source_is_allegiance',lambda c:c['context'].update(allegiance=1))
 add('not_actor',lambda c:c['actors']['2'].update(is_actor=False))
 add('not_targetable',lambda c:c['actors']['2'].update(flags8=0))
 add('dead_target',lambda c:c['actors']['2'].update(flags8=0x4001))
 for area in [-1,0,.1]:add('dead_area_'+str(area),lambda c,a=area:(c['actors']['2'].update(flags8=0x4001),c['definition'].update(area=bits(a))))
 add('dead_volley',lambda c:(c['actors']['2'].update(flags8=0x4001),c['definition'].update(volley_area=bits(1))))
 add('location_flag',lambda c:(c['context'].update(target=None),c['definition'].update(flags=0x20)))
 add('location_rejected',lambda c:c['context'].update(target=None))
 add('special_target_without_effect',lambda c:c['actors']['2'].update(flags104=0x80))
 add('special_target_type3',lambda c:(c['actors']['2'].update(flags104=0x80),c.update(effects=[effect(3)])))
 add('enchantment_missing_component',lambda c:c['definition'].update(flags=4))
 for f in [1,2,4,8,0x10,0x20,0x40,0x100,0x200,0x400,0x800,0x1000,0x2000,0x4000,0x8000]:add('required_filter_'+str(f),lambda c,v=f:c['definition']['targets'].update(required=v))
 add('friendly_filter',lambda c:(c.update(friendly=True),c['definition']['targets'].update(required=1)))
 add('allied_filter',lambda c:(c.update(allied=True),c['definition']['targets'].update(required=2)))
 add('same_source',lambda c:c['context'].update(target=1))
 add('low_mana',lambda c:c['actors']['1'].update(mana=bits(1)))
 for t in [-1,0,1]:add('low_mana_recharge_'+str(t),lambda c,v=t:(c['actors']['1'].update(mana=bits(1)),c['local'].update(recharge_time=bits(v))))
 add('min_reject',lambda c:(c['definition'].update(minimum_range=bits(3)),c.update(min_pass=False)))
 add('max_reject',lambda c:c.update(max_pass=False))
 add('range_ignored',lambda c:c.update(check_range=False,max_pass=False,min_pass=False))
 add('turret_reject',lambda c:(c['local'].update(turret_index=0),c.update(turret_pass=False)))
 add('payment_reject',lambda c:c.update(payment=False))
 add('recharging',lambda c:c['ability'].update(last_use=bits(9)))
 add('bypass_recharge',lambda c:(c['ability'].update(last_use=bits(9),recharge_bypass=True),c.update(free_recharge=True)))
 add('no_local_definition',lambda c:c.update(local=None))
 add('summon_exists',lambda c:(c['ability'].update(summoned_ids=[0x10002]),c.update(live_ids=[0x10002])))
 add('summon_stale_cleanup',lambda c:c['ability'].update(summoned_ids=[0x10002,0x20002]))
 add('summon_multiple_allowed',lambda c:(c['ability'].update(summoned_ids=[0x10002]),c.update(live_ids=[0x10002]),c['definition'].update(flags=0x100)))
 add('unique_summon',lambda c:(c['definition'].update(flags=0x200),c.update(effects=[effect(0)],owned=[2])))
 add('unique_dead_ignored',lambda c:(c['definition'].update(flags=0x200),c.update(effects=[effect(0)],owned=[1]),c['actors']['1'].update(flags8=1)))
 add('immune_all',lambda c:(c['definition'].update(flags=0x20000),c['actors']['2'].update(has_properties=True),c.update(effects=[effect(6,True)],immune_classes=[7])))
 add('immune_missing_property',lambda c:(c['definition'].update(flags=0x20000),c['actors']['2'].update(has_properties=True),c.update(effects=[effect(6,False)],immune_classes=[7])))
 add('duplicate_enchantment',lambda c:(c['definition'].update(flags=0x20004),c['actors']['2'].update(has_properties=True),c.update(effects=[effect(6,True)],already_has_property=True)))
 add('heal_full',lambda c:(c['definition'].update(flags=0x40),c['actors']['2'].update(body=[bits(100),bits(100)])))
 add('heal_override',lambda c:(c['definition'].update(flags=0x40),c['actors']['2'].update(body=[bits(100),bits(100)]),c['actors']['1'].update(has_mind=True),c.update(heal_override=True)))
 for phase in [1,2,3,4]:add('phase_'+str(phase),lambda c,p=phase:(c['context'].update(phase=p),c.update(payment=False,max_pass=False)))
 for factor in [.25,1.7,3.3333333]:add('modifier_'+str(factor),lambda c,f=factor:(c['actors']['1'].update(has_properties=True),c.update(recharge_factor=bits(f),projectile_delta=bits(-1.2),bombard_delta=bits(.31)),c['definition'].update(projectile=True,flags=8),c['ability'].update(fuzzy_range=bits(.713))))
 return [{**copy.deepcopy(c),'precision':p,'name':c['name']+'_'+p} for p in ['Single24','Double53','Extended64'] for c in rows]
def main():
 rows=[];visited={}
 for c in cases():
  try:m=Machine(c);result=m.run()
  except Exception as exc:raise RuntimeError(c['name']) from exc
  rows.append({'input':c,'output':result});visited.update(m.executed)
 report={'schema':1,'passed':True,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'original_sha256':hashlib.sha256(original.b).hexdigest(),'accepted_sha256':hashlib.sha256(modified.b).hexdigest(),'method':__doc__,'cases':rows,'visited_windows':[{'address':hex(a),'bytes':raw}for a,raw in sorted(visited.items())]}
 (HERE/'native-admission-fixtures.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8');print(json.dumps({'passed':True,'cases':len(rows)}))
if __name__=='__main__':main()
