"""Run original kill-credit, classifications, score and experience arithmetic.

Registry/actor virtuals, player iteration, strings, derived actor value, rewards
and world notifications are explicit seams. Classification, score increments,
relative-score comparison and experience formula execute native instructions.
"""
from pathlib import Path
import copy,hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-body'))
from original_body import IMAGE,ACCEPTED,bits,pe
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE
from unicorn.x86_const import *

ptr=lambda k:0 if k is None else 0x2000000+k*0x1000
key=lambda p:None if p==0 else (p-0x2000000)//0x1000
WORLD,CACHE,SESSION,SETTINGS,VT,STUB,STACK,STOP,STRINGS,PLAYERS,PLIST,ITER,REGISTRY,RETURN,VALUE=[0x2200000+i*0x1000 for i in range(15)]
OFFSETS={'component7c':0x7c,'organization':0x80,'component88':0x88,'component90':0x90,'component94':0x94,'component98':0x98,'component9c':0x9c,'componenta4':0xa4,'componentc0':0xc0,'componentc4':0xc4,'componentd8':0xd8}
VIRTUALS={'SourceAccepts':0x16c,'Hostile':0x140,'Outer':0x100,'Virtual20':0x20,'Visible':0x70,'VictimLabel':0x24,'Allegiance':0x108}

class Machine:
 def __init__(self,c):
  self.c=copy.deepcopy(c);self.s=self.c['state'];self.events=[];self.mutations=self.c['mutations'];self.visited={};self.player_index=0;self.next_allocation=0x2300000
  self.u=Uc(UC_ARCH_X86,UC_MODE_32);IMAGE.load(self.u);self.u.mem_map(0,4096);self.u.mem_map(0x2000000,0x600000)
  self.u.reg_write(UC_X86_REG_FPCW,{'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[c['precision']])
  for k in range(1,9):self.w(ptr(k),VT);self.w(ptr(k)+4,ptr(k)+0x500)
  self.virtual_pc={}
  for n,(name,offset) in enumerate(VIRTUALS.items()):self.w(VT+offset,STUB+n*0x10);self.virtual_pc[STUB+n*0x10]=name
  self.w(0xb36534,WORLD);self.w(0xb39ab0,CACHE);self.w(0xb36550,SESSION);self.w(0xb364c0,REGISTRY)
  self.w(0xb3cb8c,1);self.w(0xb3cb88,SETTINGS+4);self.w(0xb36520,0x222f000);self.w(0xb39d8c,0x222e000)
  for n,(field,offset) in enumerate([('experience_factor',0x168),('experience_minimum',0x160),('experience_maximum',0x164),('zero_value_fallback',0x178)]):self.w(CACHE+offset,SETTINGS+0x10+n*4)
  for k in [2,3]:self.u.mem_write(REGISTRY+4+k*2,struct.pack('<H',1 if str(k) in c['registry'] else 2));self.w(REGISTRY+0x20004+k*4,ptr(c['registry'].get(str(k))))
  self.w(PLIST,PLAYERS);self.w(PLIST+4,len(c['players']))
  for i,k in enumerate(c['players']):self.w(PLAYERS+i*4,ptr(k))
  for i,k in enumerate([101,102]):
   for j,v in enumerate(c['stats'][str(k)]):self.w(ptr(k)+0x28+j*4,v)
  for k in [103,104]:
   r=c['relations'][str(k)];base=ptr(k);self.w(base+0x80,base+0x500);self.w(base+0x84,1)
   nodes=list(r['values'].items());self.w(base+0x500,base+0x600 if nodes else 0)
   for i,(other,value) in enumerate(nodes):
    node=base+0x600+i*12;self.w(node,ptr(int(other)));self.w(node+4,value);self.w(node+8,node+12 if i+1<len(nodes) else 0)
   self.w(base+0xe8,r['maximum']);self.w(base+0xc0,ptr(r['leader']))
  # Stable native floating return code; callback changes only operands.
  self.u.mem_write(RETURN,b'\xd9\x05'+struct.pack('<I',VALUE)+b'\xd8\x05'+struct.pack('<I',VALUE+4)+b'\xc2\x04\x00')
  for n in range(2):self.w(STRINGS+0x100+n*0x100-4,100000)
  self.write_state();self.u.hook_add(UC_HOOK_CODE,self.hook)
 def w(self,a,v):self.u.mem_write(a,struct.pack('<I',v&0xffffffff))
 def r(self,a):return struct.unpack('<I',self.u.mem_read(a,4))[0]
 def arg(self,i=0):return self.r(self.u.reg_read(UC_X86_REG_ESP)+4+i*4)
 def ret(self,pop=0,value=0):
  sp=self.u.reg_read(UC_X86_REG_ESP);self.u.reg_write(UC_X86_REG_EIP,self.r(sp));self.u.reg_write(UC_X86_REG_ESP,sp+4+pop);self.u.reg_write(UC_X86_REG_EAX,value)
 def write_state(self):
  v=self.s['victim'];s=self.s['settings'];actor=ptr(1);data=actor+0x500;self.w(actor+8,v['flags']);self.w(actor+0xe8,ptr(v['owner']))
  for name,offset in OFFSETS.items():self.w(actor+offset,actor+0x800+offset if v[name] else 0)
  self.w(actor+0x890,v['organization_leader'] and ptr(v['organization_leader']) or 0)
  self.w(actor+0x60,actor+0x700 if v['body_damage'] is not None else 0)
  for where,record in [(actor+0x728,v['body_damage']),(actor+0x90+0x87c,v['alternate_damage'])]:
   if record is not None:
    for i,value in enumerate(record):self.w(where+i*4,value)
  self.w(data+0x2c8,data+0x600);self.w(data+0x600,int(v['definition2c8_nonempty']))
  self.w(data+0x43c,int(v['definition43c_nonempty']));self.w(data+0x4e0,int(v['definition4e0_nonzero']));self.w(data+0x4f4,v['definition4f4'])
  self.w(WORLD+0xe0,s['world_time']);self.w(SESSION+0xe4,s['session_mode']);self.w(SETTINGS+4,s['kill_credit_timeout']);self.w(0xb36528,0x222d000 if s['campaign_observer'] else 0)
  for n,name in enumerate(['experience_factor','experience_minimum','experience_maximum','zero_value_fallback']):self.w(SETTINGS+0x10+n*4,s[name])
  self.w(ptr(5)+8,self.c['outer_flags'])
  for owner,handles in self.c['handles'].items():self.w(ptr(int(owner))+0x32c,ptr(handles['ordinary']));self.w(ptr(int(owner))+0x340,ptr(handles['relations']))
  f=self.c['source_fields'];self.w(ptr(2)+0x7c,ptr(2)+0x800 if f['component7c'] else 0);self.w(ptr(2)+0x80,ptr(2)+0x900 if f['organization_leader'] is not None else 0);self.w(ptr(2)+0x910,ptr(f['organization_leader']))
 def read_state(self):
  self.s['victim']['flags']=self.r(ptr(1)+8)
  return copy.deepcopy(self.s)
 def event(self,e):
  self.events.append({'event':e,'state':self.read_state()})
  name=e if isinstance(e,str) else next(iter(e))
  if self.mutations and self.mutations[0]['on']==name:
   m=self.mutations.pop(0);self.s['victim'].update(m.get('victim',{}));self.s['settings'].update(m.get('settings',{}));self.c.update(m.get('world',{}));self.write_state()
 def hook(self,u,a,size,_):
  if 0x400000<=a<0xc00000:self.visited[a]=size
  if a in self.virtual_pc:
   name=self.virtual_pc[a]
   if name=='SourceAccepts':self.event({'SourceAccepts':{'source':key(u.reg_read(UC_X86_REG_ECX))}});self.ret(4,int(self.c['source_accepts']))
   elif name=='Hostile':other=key(self.arg());self.event({'Hostile':{'other':other}});self.ret(4,int(self.c['hostile'].get(str(other),False)))
   elif name=='Outer':self.event('Outer');self.ret(value=ptr(self.c['outer']))
   elif name=='Virtual20':arg=bool(self.arg());self.event({'Virtual20':{'argument':arg}});self.ret(4,int(self.c['virtual20'][str(arg).lower()]))
   elif name=='Visible':player=key(self.arg());self.event({'Visible':{'player':player,'argument':bool(self.arg(1))}});self.ret(8,int(self.c['visible'].get(str(player),False)))
   elif name=='VictimLabel':self.event('VictimLabel');self.w(self.arg(),STRINGS+0x100);self.ret(4,self.arg())
   elif name=='Allegiance':self.event('Allegiance');self.ret(value=ptr(self.c['allegiance']))
   return
  if a==0x4e9560:
   self.event({'OwnerLabel':{'owner':key(u.reg_read(UC_X86_REG_ECX))}});self.w(self.arg(),STRINGS+0x200);self.ret(8,self.arg());return
  if a==0x42d490:
   if self.arg(1)==0x11:self.event({'PlayerNotification':{'player':key(self.arg(2)),'owner_label':'owner','victim_label':'victim'}})
   else:assert self.arg(1)==0x16;self.event({'SpecialNotification':{'allegiance':key(self.arg(2)),'victim_label':'victim'}})
   self.ret();return
  if a==0x4a5be0:
   self.event('BeginPlayers');self.player_index=0;self.w(self.arg(),PLIST);self.w(self.arg()+4,0);self.ret(4,self.arg());return
  if a==0x4f06b0:
   self.event('NextPlayer');self.player_index+=1;self.w(u.reg_read(UC_X86_REG_ECX)+4,self.player_index);self.ret(4);return
  if a==0x58e870:self.event({'Stat':{'component':key(u.reg_read(UC_X86_REG_ECX)),'index':self.arg()}});return
  if a==0x58d950:self.event({'Relation':{'component':key(u.reg_read(UC_X86_REG_ECX)),'index':self.arg(),'other':key(self.arg(1))}});return
  if a==0x6f7810:
   assert self.arg()==12
   allocated=self.next_allocation;self.next_allocation+=0x100;self.ret(value=allocated);return
  if a==0x4d01e0:
   actor=key(u.reg_read(UC_X86_REG_ECX));self.event({'ActorValue':{'actor':actor}});pair=self.c['values'][str(actor)];self.w(VALUE,pair[0]);self.w(VALUE+4,pair[1]);u.reg_write(UC_X86_REG_EIP,RETURN);return
  if a in [0x7a06a0,0x4ce770,0x4ceb40,0x4d10f0,0x4d3e20,0x54be10]:
   if a==0x7a06a0:e='CampaignDeath';pop=4
   elif a==0x54be10:e='DeathTrigger';pop=4
   elif a==0x4ce770:e={'AwardExperience':{'source':key(u.reg_read(UC_X86_REG_ECX)),'amount':self.arg(),'argument1':bool(self.arg(1)),'argument2':bool(self.arg(2))}};pop=12
   else:e={{0x4ceb40:'OrganizationReward',0x4d10f0:'Component7cReward',0x4d3e20:'Componentc4Reward'}[a]:{'source':key(u.reg_read(UC_X86_REG_ECX))}};pop=4
   self.event(e);self.ret(pop);return
  if a==0x404f90:self.ret();return # Diagnostics only.
 def run(self):
  self.w(STACK,STOP);self.u.reg_write(UC_X86_REG_ESP,STACK);self.u.reg_write(UC_X86_REG_ECX,ptr(1))
  regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
  for i,r in enumerate(regs):self.u.reg_write(r,0x11111111*(i+1))
  try:self.u.emu_start(0x4d57c0,STOP,count=200000)
  except Exception as e:raise RuntimeError(self.c['name']+' at '+hex(self.u.reg_read(UC_X86_REG_EIP))) from e
  assert self.u.reg_read(UC_X86_REG_EIP)==STOP and self.u.reg_read(UC_X86_REG_ESP)==STACK+4
  assert [self.u.reg_read(r) for r in regs]==[0x11111111*(i+1) for i in range(4)]
  assert not self.mutations,self.mutations
  stats={str(k):[self.r(ptr(k)+0x28+i*4) for i in range(48)] for k in [101,102]};relations={}
  for k in [103,104]:
   relation={'values':{},'maximum':self.r(ptr(k)+0xe8),'leader':key(self.r(ptr(k)+0xc0))};node=self.r(ptr(k)+0x500)
   while node:relation['values'][str(key(self.r(node)))]=self.r(node+4);node=self.r(node+8)
   relations[str(k)]=relation
  return {'source':key(self.u.reg_read(UC_X86_REG_EAX)),'state':self.read_state(),'events':self.events,'stats':stats,'relations':relations,'abi_preserved':True}

def base(name):
 v={'identity':1,'flags':0,'owner':4,'body_damage':[0x10002,0x10003,5,bits(10),bits(9),1,4],'alternate_damage':None,
    **{k:False for k in OFFSETS},'organization_leader':None,'definition2c8_nonempty':False,'definition43c_nonempty':False,'definition4e0_nonzero':False,'definition4f4':bits(0)}
 s={'session_mode':0,'world_time':bits(10),'kill_credit_timeout':bits(3),'experience_factor':bits(10),'experience_minimum':bits(.5),'experience_maximum':bits(100),'zero_value_fallback':bits(2),'campaign_observer':True}
 return {'name':name,'precision':'Double53','state':{'victim':v,'settings':s},'registry':{'2':2,'3':3},'source_accepts':True,'hostile':{'3':True,'7':True,'8':False},'visible':{'7':True,'8':True},'outer':None,'outer_flags':0,'virtual20':{'true':True,'false':True},'players':[7,8],'allegiance':4,'handles':{'4':{'ordinary':101,'relations':103},'3':{'ordinary':102,'relations':104}},'source_fields':{'component7c':False,'organization_leader':6},'values':{'1':[bits(10),bits(2**-24)],'2':[bits(5),bits(2**-24)],'6':[bits(3),bits(2**-25)]},'stats':{str(k):[bits(0)]*48 for k in [101,102]},'relations':{'103':{'values':{'3':bits(0)},'maximum':bits(0),'leader':None},'104':{'values':{'4':bits(0)},'maximum':bits(0),'leader':None}},'mutations':[]}

def cases():
 rows=[]
 def add(name,edit=None):
  c=base(name)
  if edit:edit(c)
  rows.append(c)
 add('ordinary');add('already_credited',lambda c:c['state']['victim'].update(flags=16));add('session3',lambda c:c['state']['settings'].update(session_mode=3));add('no_record',lambda c:c['state']['victim'].update(body_damage=None))
 add('alternate_record',lambda c:c['state']['victim'].update(alternate_damage=c['state']['victim']['body_damage'],body_damage=None,component7c=True))
 for t in [6,7,8,10,20]:add(f'time_{t}',lambda c,t=t:c['state']['victim']['body_damage'].__setitem__(4,bits(t)))
 add('invalid_source',lambda c:c['registry'].pop('2'));add('invalid_allegiance',lambda c:c['registry'].pop('3'));add('source_rejects',lambda c:c.update(source_accepts=False));add('friendly_allegiance',lambda c:c['hostile'].update({'3':False}))
 for comp in OFFSETS:add('only_'+comp,lambda c,comp=comp:c['state']['victim'].update({comp:True}))
 add('org_leader',lambda c:c['state']['victim'].update(organization=True,organization_leader=6))
 add('all_classes',lambda c:c['state']['victim'].update(component7c=True,organization=True,componenta4=True,componentc4=True,component9c=True,definition2c8_nonempty=True,definition43c_nonempty=True))
 for number in [1,2,3,4]:add(f'class_{number}',lambda c,n=number:c['state']['victim'].update(component9c=True,definition43c_nonempty=n==1,definition4e0_nonzero=n==2,component88=n==3))
 add('attached',lambda c:(c.update(outer=5,outer_flags=1),c['state']['victim'].update(component94=True,component9c=True)))
 add('outer_alive',lambda c:(c.update(outer=5),c['state']['victim'].update(component94=True,component9c=True)))
 add('no_owner',lambda c:c['state']['victim'].update(owner=None));add('no_stats',lambda c:c.update(handles={'4':{'ordinary':None,'relations':None},'3':{'ordinary':None,'relations':None}}))
 add('v20_false',lambda c:(c.update(virtual20={'true':False,'false':False}),c['state']['victim'].update(organization=True,component7c=True)))
 add('no_campaign',lambda c:c['state']['settings'].update(campaign_observer=False));add('no_players',lambda c:c.update(players=[]));add('hidden_player',lambda c:c['visible'].update({'7':False}))
 for value in [0,-1,.1,2,10000]:add(f'victim_value_{value}',lambda c,value=value:c['values'].update({'1':[bits(value),0]}))
 add('zero_sentinel',lambda c:(c['values'].update({'1':[0,0]}),c['state']['victim'].update(definition4f4=bits(-1))))
 for value in [0,-1,.1,2,10000]:add(f'source_value_{value}',lambda c,value=value:c['values'].update({'2':[bits(value),0]}))
 add('zero_factor',lambda c:c['state']['settings'].update(experience_factor=0))
 add('callback_flags',lambda c:c.update(mutations=[{'on':'SourceAccepts','victim':{'component7c':True,'organization':True,'componentc4':True}}]))
 add('callback_factor_cache',lambda c:c.update(mutations=[{'on':'ActorValue','settings':{'experience_factor':bits(1000),'experience_minimum':bits(21)}}]))
 add('callback_reward_fields',lambda c:c.update(mutations=[{'on':'AwardExperience','victim':{'organization':True,'component7c':True,'componentc4':True}}]))
 add('callback_owner',lambda c:c.update(mutations=[{'on':'VictimLabel','victim':{'owner':3}}]))
 add('callback_classification',lambda c:c.update(mutations=[{'on':'Virtual20','victim':{'organization':True,'componenta4':True,'definition2c8_nonempty':True}}]))
 add('timeout_extended_boundary',lambda c:(c['state']['settings'].update(world_time=bits(1),kill_credit_timeout=bits(1)),c['state']['victim']['body_damage'].__setitem__(4,bits(-2**-25))))
 add('cached_allegiance',lambda c:c.update(mutations=[{'on':'SourceAccepts','victim':{'body_damage':[0,0,0,0,0,0,0]}}]))
 add('relation_insert',lambda c:(c['state']['victim'].update(organization=True),c['relations']['103'].update(values={}),c['relations']['104'].update(values={})))
 add('relation_extended_maximum',lambda c:(c['state']['victim'].update(organization=True),c['relations']['103'].update(values={'3':bits(16777216)},maximum=bits(16777216),leader=4)))
 add('ordinary_stat_rounding',lambda c:(c['state']['victim'].update(component7c=True),c['stats']['101'].__setitem__(5,bits(16777216))))
 add('callback_reward_removes_fields',lambda c:(c['state']['victim'].update(organization=True,component7c=True,componentc4=True),c.update(mutations=[{'on':'AwardExperience','victim':{'organization':False,'component7c':False,'componentc4':False}}])))
 for p in ['Single24','Double53','Extended64']:
  for row in rows:yield {**copy.deepcopy(row),'precision':p,'name':row['name']+'_'+p}

def main():
 rows=[];visited={}
 for c in cases():
  m=Machine(c);rows.append({'input':c,'output':m.run()});visited.update(m.visited)
 windows=[]
 for a,n in sorted(visited.items()):
  raw=IMAGE.at(a,n);assert raw==ACCEPTED.at(a,n);windows.append({'address':hex(a),'bytes':raw.hex()})
 doc={'schema':1,'original_sha256':pe.ORIGINAL_SHA,'v14_sha256':pe.ACCEPTED_SHA,'oracle_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'case_count':len(rows),'seams':['actor registry fixture','actor virtuals','player iteration','strings','derived actor value','reward application and world notifications','map node allocator','diagnostic logging'],'visited_byte_window_scope':'Includes hook-entry bytes; not evidence of execution behind intercepted seams.','visited_windows':windows,'fixtures':rows}
 path=HERE/'credit-native-oracle.json';path.write_text(json.dumps(doc,indent=2)+'\n');print(json.dumps({'cases':len(rows),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}))
if __name__=='__main__':main()

