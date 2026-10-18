"""Execute original owned-path/event/ActionGo instructions with explicit seams.

Only allocation, UAI/world callbacks and the one wide RNG draw are fixture
boundaries. Native FSQRT, all x87 stores and 45D400 normalization execute.
"""
from pathlib import Path
import copy, hashlib, json, random, struct, sys
ROOT=Path(__file__).resolve().parents[2]; GAME=ROOT.parent; OUT=Path(__file__).parent
sys.path.insert(0,str(ROOT/'research/phase3-terrain'))
from terrain_uv_oracle import Pe, ORIGINAL_SHA, ACCEPTED_SHA
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32, UC_HOOK_CODE
from unicorn.x86_const import *
UAI,ACTOR,MOVER,ORG,LEADER,VT,WORLD,CACHE,VALUES,SOURCE,POOL=[0x2000000+n*0x1000 for n in range(1,12)]
STATE=UAI+0x7c; PATH=STATE+0x58; STUB=0x2100000; STOP=0x2200000
STATE_FIELDS={'target_id':(0x10,1),'destination_bits':(0x14,2),'auxiliary_bits':(0x1c,2),'requested_speed_bits':(0x24,1),'phase':(0x2c,1),'mode':(0x30,1),'waypoint_offset_bits':(0x54,1)}
def bits(x):return struct.unpack('<I',struct.pack('<f',x))[0]
def node(x=10.,y=2.,dx=1.,dy=0.,kind=1,tail=(0,0,0)):return {'words':[bits(x),bits(y),bits(dx),bits(dy),kind,*tail]}
def path(nodes,dirty=False,word18=0x12345678):return {'nodes':nodes,'kind1_count':sum(n['words'][4]==1 for n in nodes),'kind2_count':sum(n['words'][4]==2 for n in nodes),'dirty':dirty,'word18':word18}
class Machine:
 def __init__(self,pe,c):
  self.c=c; self.u=Uc(UC_ARCH_X86,UC_MODE_32); pe.load(self.u); self.u.mem_map(0,0x1000);self.u.mem_map(0x2000000,0x400000);self.u.mem_map(0x3000000,0x10000)
  self.u.reg_write(UC_X86_REG_FPCW,{'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[c['precision']]);self.events=[];self.next_node=0x2300000;self.freed=[];self.allocated=[]
  self.put(UAI,VT);self.put(UAI+4,ACTOR);self.put(STATE,VT);self.put(STATE+4,UAI);self.put(UAI+8,VT+0x200);self.put(UAI+0x48+4,UAI)
  self.put(UAI+0x10,VALUES+0x200 if c['special_state_active'] else 0);self.put(VALUES+0x200,UAI+0x48)
  self.put(VT+4,STUB);self.put(VT+0x160,STUB+0x10);self.put(VT+0xa4,STUB+0x20);self.put(VT+0x204,STUB+0x30)
  a=c['actor'];self.put(ACTOR+8,a['flags']);self.put(ACTOR+0x20,*a['position_bits']);self.put(ACTOR+0x6c,MOVER if a['has_mover'] else 0);self.put(ACTOR+0x80,ORG)
  self.put(ORG+0x10,LEADER if a['has_leader'] else 0);self.byte(ORG+0x1c,a['organization_special']);self.byte(MOVER+0x54,a['mover_instant_turn'])
  self.put(MOVER+0x24,a['mover_requested_speed_bits']);self.put(MOVER+0x28,a['mover_maximum_speed_bits']);self.put(MOVER+0x50,a['mover_speed_factor_bits'])
  self.put(0xb36534,WORLD);self.put(0xb39ab0,CACHE);self.put(CACHE+0x190,VALUES);self.put(VALUES,c['group_speed_minimum_bits']);self.put(CACHE+0x48,VALUES+4);self.put(VALUES+4,c['catchup_speed_factor_bits'])
  self.byte(0xb3d43c,False);self.put(0xb27ae0,POOL)
  for name,(off,count) in STATE_FIELDS.items():
   v=c['state'][name];self.put(STATE+off,*(v if count>1 else [v]))
  self.write_path(PATH,c['state']['path']);self.write_path(SOURCE,c['source'])
  self.u.hook_add(UC_HOOK_CODE,self.hook)
 def put(self,a,*v):self.u.mem_write(a,struct.pack('<'+'I'*len(v),*(x&0xffffffff for x in v)))
 def byte(self,a,v):self.u.mem_write(a,bytes([bool(v)]))
 def ints(self,a,n=1):return list(struct.unpack('<'+'I'*n,self.u.mem_read(a,n*4)))
 def args(self,n=1):return self.ints(self.u.reg_read(UC_X86_REG_ESP)+4,n)
 def ret(self,pop=0,value=1):
  s=self.u.reg_read(UC_X86_REG_ESP);self.u.reg_write(UC_X86_REG_EIP,self.ints(s)[0]);self.u.reg_write(UC_X86_REG_ESP,s+4+pop);self.u.reg_write(UC_X86_REG_EAX,value)
 def event(self,name,data=()):
  self.events.append({'name':name,'bits':list(data)})
  changes=self.c.get('mutations',{}).get(name,{})
  for key,value in changes.get('state',{}).items():
   off,count=STATE_FIELDS[key];self.put(STATE+off,*(value if count>1 else [value]))
  for key,value in changes.get('actor',{}).items():
   addr={'mover_requested_speed_bits':MOVER+0x24,'mover_speed_factor_bits':MOVER+0x50,'mover_maximum_speed_bits':MOVER+0x28}[key];self.put(addr,value)
  if 'first_kind' in changes:self.put(self.ints(PATH+4)[0]+0x10,changes['first_kind'])
 def alloc(self):
  p=self.next_node;self.next_node+=0x100;return p
 def write_path(self,a,p):
  ptrs=[self.alloc() for _ in p['nodes']]
  self.put(a,0,ptrs[0] if ptrs else 0,ptrs[-1] if ptrs else 0,p['kind1_count'],p['kind2_count']);self.byte(a+0x14,p['dirty']);self.put(a+0x18,p['word18'])
  for i,(ptr,n) in enumerate(zip(ptrs,p['nodes'])):self.put(ptr,*n['words'],ptrs[i-1] if i else 0,ptrs[i+1] if i+1<len(ptrs) else 0)
 def read_path(self,a):
  first,last,k1,k2=self.ints(a+4,4);nodes=[];p=first;prev=0
  while p:
   assert len(nodes)<1000 and self.ints(p+0x20)[0]==prev
   nodes.append({'words':self.ints(p,8)});prev=p;p=self.ints(p+0x24)[0]
  assert prev==last
  signed=lambda x:x if x<0x80000000 else x-0x100000000
  return {'nodes':nodes,'kind1_count':signed(k1),'kind2_count':signed(k2),'dirty':bool(self.u.mem_read(a+0x14,1)[0]),'word18':self.ints(a+0x18)[0]}
 def hook(self,u,a,size,_):
  if a==STOP:u.emu_stop();return
  ecx=u.reg_read(UC_X86_REG_ECX)
  if a==0x85c680:
   p=self.alloc();self.allocated.append(p);self.ret(value=p);return
  if a==0x85c2b0:
   p=self.args()[0];self.freed.append(self.ints(p,8));self.ret(4);return
  if a in (0x77dc50,0x77d930,0x77da70):self.ret({0x77dc50:8,0x77d930:12,0x77da70:12}[a]);return
  if a==0x52a000:self.event('SpecialStateActive') # real tiny state predicate executes
  if a==STUB:self.event('CanActivate');self.ret(value=int(self.c['can_activate']));return
  if a==0x52a810:
   self.event('Activate',self.args());self.ret(4,int(self.c['activate_result']));return
  if a==STUB+0x10:self.event('NodeReached');self.ret();return
  if a==STUB+0x20:self.event('OrganizationReady');self.ret(value=int(self.c['organization_ready']));return
  if a==STUB+0x30:self.event('Schedule',self.args(2));self.ret(8);return
  if a==0x52e6a0:self.event('SingleNodePlacement',self.args(2));self.ret(8,int(self.c['single_node_placement']));return
  if a==0x4cede0:
   name='LeaderIsMoving' if ecx==LEADER else 'ActorIsMoving';self.event(name);self.ret(value=int(self.c['leader_is_moving' if ecx==LEADER else 'actor_is_moving']));return
  if a==0x52ff00:self.event('FormationReady');self.ret(value=int(self.c['formation_ready']));return
  if a==0x4ce6c0:self.ret(value=VALUES+0x100);return
  if a==0x44dea0:self.event('ActorOrLeaderHandleValid');self.ret(value=int(self.c['actor_or_leader_handle_valid']));return
  if a==0x52d660:self.event('StopAction');self.ret();return
  if a==0x5773c0:
   self.event('BeginMover',self.args(6));self.ret(24,int(self.c['begin_result']));return
  if a==0x85d840:
   self.event('RandomDraw');self.ret(value=self.c['random_u32']);return
 def run(self):
  op=self.c['operation'];obj=STATE
  if op=='copy':entry,args,obj=0x52d0f0,[SOURCE],PATH
  elif op=='self_copy':entry,args,obj=0x52d0f0,[PATH],PATH
  elif op=='clear':entry,args,obj=0x5189f0,[],PATH+4
  elif op=='prune':entry,args,obj=0x52ce00,[],PATH
  elif op=='activate':
   q=self.c['activation'];entry,args=0x52df60,[q['activation_mode'],*q['destination_bits'],q['movement_mode'],q['requested_speed_bits'],*q['auxiliary_bits']]
  elif op in ('move_event','formation_event'):entry,args,obj=(0x532170 if op=='move_event' else 0x532310),[SOURCE,self.c['speed_bits'],0xdeadbeef,0x11223344],UAI
  else:entry,args=0x5300a0,[self.c['speed_bits']]
  stack=0x3008000;self.put(stack,STOP,*args);self.u.reg_write(UC_X86_REG_ESP,stack);self.u.reg_write(UC_X86_REG_ECX,obj)
  regs=[(UC_X86_REG_EBX,0x11223344),(UC_X86_REG_ESI,0x33445566),(UC_X86_REG_EDI,0x55667788),(UC_X86_REG_EBP,0x778899aa)]
  for r,v in regs:self.u.reg_write(r,v)
  self.u.emu_start(entry,STOP,count=200000)
  assert self.u.reg_read(UC_X86_REG_EIP)==STOP and self.u.reg_read(UC_X86_REG_ESP)==stack+4+len(args)*4
  assert all(self.u.reg_read(r)==v for r,v in regs)
  state={}
  for name,(off,count) in STATE_FIELDS.items():
   v=self.ints(STATE+off,count);state[name]=v if count>1 else v[0]
  state['path']=self.read_path(PATH)
  actor=copy.deepcopy(self.c['actor']);actor['mover_instant_turn']=bool(self.u.mem_read(MOVER+0x54,1)[0]);actor['mover_requested_speed_bits']=self.ints(MOVER+0x24)[0];actor['mover_speed_factor_bits']=self.ints(MOVER+0x50)[0];actor['mover_maximum_speed_bits']=self.ints(MOVER+0x28)[0]
  # Confirm native ownership, including free order, independently of the Rust
  # representation. The source chain must remain byte/payload-identical.
  expected_free=[];expected_alloc=0
  if op in ('copy','clear'):
   expected_free=[n['words'] for n in self.c['state']['path']['nodes']]
   expected_alloc=len(self.c['source']['nodes']) if op=='copy' else 0
  if op in ('move_event','formation_event') and self.u.reg_read(UC_X86_REG_EAX)&255:
   expected_free=[n['words'] for n in self.c['state']['path']['nodes']];expected_alloc=len(self.c['source']['nodes'])
   prune=self.c['source']
  elif op=='prune':prune=self.c['state']['path']
  else:prune=None
  if prune:
   remaining=prune['kind2_count']
   for n in reversed(prune['nodes']):
    if remaining<=0:break
    expected_free.append(n['words'])
    if n['words'][4]==2:remaining-=1
  assert self.freed==expected_free and len(self.allocated)==expected_alloc
  assert self.read_path(SOURCE)==self.c['source']
  return {'state':state,'actor':actor,'events':self.events,'freed_payloads':self.freed,'allocated_count':len(self.allocated),'native_success':bool(self.u.reg_read(UC_X86_REG_EAX)&255),'abi_preserved':True,'source_after':self.read_path(SOURCE)}

def cases():
 base={'name':'action_default','operation':'action','precision':'Double53',
  'state':{'target_id':0x10007,'destination_bits':[bits(9),bits(8)],'auxiliary_bits':[bits(3),bits(4)],'requested_speed_bits':bits(3),'phase':2,'mode':0,'waypoint_offset_bits':bits(.125),'path':path([node()])},
  'actor':{'flags':0,'position_bits':[0,0],'mover_instant_turn':False,'mover_requested_speed_bits':bits(4),'mover_speed_factor_bits':bits(.7),'mover_maximum_speed_bits':bits(5),'has_mover':True,'organization_special':False,'has_leader':False},
  'source':path([node(11,4),node(12,5,kind=2,tail=(0x7fc12345,0x80000000,0xffffffff))]),
  'activation':{'activation_mode':2,'destination_bits':[bits(7),bits(8)],'movement_mode':3,'requested_speed_bits':bits(3.5),'auxiliary_bits':[bits(-1),bits(.25)]},
  'speed_bits':bits(2),'group_speed_minimum_bits':bits(1),'catchup_speed_factor_bits':bits(1.5),'random_u32':0x87654321,
  'special_state_active':False,'can_activate':True,'activate_result':True,'organization_ready':False,'single_node_placement':False,'actor_is_moving':False,'leader_is_moving':False,'formation_ready':True,'actor_or_leader_handle_valid':False,'begin_result':True}
 rows=[]
 def add(name,**parts):
  row=copy.deepcopy(base);row['name']=name
  for k,v in parts.items():
   if k in ('actor','state','activation'):row[k].update(v)
   else:row[k]=v
  rows.append(row)
 for op in ('copy','self_copy','clear','prune'):add(op,operation=op)
 add('copy_empty',operation='copy',source=path([]))
 add('clear_populated_metadata',operation='clear',state={'path':path([node(kind=1),node(kind=2)])})
 add('prune_mixed_tail',operation='prune',state={'path':path([node(kind=1),node(kind=2),node(kind=1),node(kind=0)])})
 add('prune_all',operation='prune',state={'path':path([node(kind=2),node(kind=1),node(kind=2)])})
 add('prune_negative_counter',operation='prune',state={'path':{**path([node(kind=2)]),'kind2_count':-1}})
 for op in ('move_event','formation_event'):
  add(op,operation=op)
  add(op+'_dead',operation=op,actor={'flags':1})
  add(op+'_flag29',operation=op,actor={'flags':1<<29})
  add(op+'_special_state',operation=op,special_state_active=True)
  add(op+'_cannot_activate',operation=op,can_activate=False)
  add(op+'_activation_false',operation=op,activate_result=False)
  add(op+'_all_pruned',operation=op,source=path([node(kind=2),node(kind=1)]))
 for mode in (0,1,2,3,0xffffffff):add('activate_'+str(mode),operation='activate',can_activate=False,activation={'activation_mode':mode})
 add('activate_ok',operation='activate');add('activate_callback_false',operation='activate',activate_result=False)
 add('action_default');add('action_empty',state={'path':path([])})
 add('action_close',state={'path':path([node(.00005,0)])})
 add('action_exact_epsilon',state={'path':path([node(struct.unpack('<f',struct.pack('<I',0x38d1b717))[0],0)])})
 add('action_projection_zero',state={'path':path([node(0,2)])})
 add('action_behind',state={'path':path([node(-1,0)])})
 add('action_kind0',state={'path':path([node(kind=0)])})
 add('action_single_override',single_node_placement=True)
 add('action_multi_no_single',single_node_placement=True,state={'path':path([node(),node(20,2)])})
 add('action_speed_fallback_state',speed_bits=0)
 add('action_speed_fallback_mover',speed_bits=0,actor_is_moving=True)
 add('action_speed_epsilon_kept',speed_bits=0x38d1b717)
 add('action_formation_reject',state={'mode':3},formation_ready=False)
 add('action_formation_factor',state={'mode':3})
 add('action_zone_stop',actor_or_leader_handle_valid=True,group_speed_minimum_bits=bits(4))
 add('action_zone_equal_go',actor_or_leader_handle_valid=True,group_speed_minimum_bits=bits(3))
 add('action_org_special',actor={'organization_special':True},organization_ready=True)
 add('action_org_no_leader',organization_ready=True)
 add('action_org_stationary_leader',actor={'has_leader':True},organization_ready=True)
 add('action_org_catchup',actor={'has_leader':True},organization_ready=True,leader_is_moving=True)
 add('action_mover_failure_still_draws',begin_result=False)
 add('event_activation_mutates_fields',operation='move_event',mutations={'Activate':{'state':{'target_id':99,'requested_speed_bits':bits(9)}}})
 add('event_canactivate_mutates_then_rejects',operation='formation_event',can_activate=False,mutations={'CanActivate':{'state':{'phase':88}}})
 add('action_placement_mutates_speed',speed_bits=0,mutations={'SingleNodePlacement':{'state':{'requested_speed_bits':bits(7)}}})
 add('action_formation_mutates_factor',state={'mode':3},mutations={'FormationReady':{'actor':{'mover_speed_factor_bits':bits(1.125)}}})
 add('action_actor_predicate_mutates_request',speed_bits=0,actor_is_moving=True,mutations={'ActorIsMoving':{'actor':{'mover_requested_speed_bits':bits(8.25)}}})
 add('action_zone_predicate_mutates_request',actor_or_leader_handle_valid=True,mutations={'ActorOrLeaderHandleValid':{'state':{'requested_speed_bits':bits(.25)}}})
 add('action_mover_mutates_kind_before_draw',mutations={'BeginMover':{'first_kind':0}})
 add('action_rng_mutates_kind_before_schedule',mutations={'RandomDraw':{'first_kind':0}})
 add('action_arrival_callback_mutates_phase',state={'path':path([node(0,0)])},mutations={'NodeReached':{'state':{'phase':7}}})
 for value in (0,1,0x7fffffff,0x80000000,0xffffffff):add('action_rng_'+str(value),random_u32=value)
 rng=random.Random(5300)
 for i in range(20):add(f'action_fractional_{i}',state={'path':path([node(3+rng.random()*8,3+rng.random()*8,rng.random(),rng.random(),i%3)]),'waypoint_offset_bits':bits(rng.random()*2)},random_u32=rng.getrandbits(32))
 return [{**copy.deepcopy(r),'name':r['name']+'_'+pc,'precision':pc} for pc in ('Single24','Double53','Extended64') for r in rows]
def main():
 pe=Pe(GAME/'AA.exe',ORIGINAL_SHA);v14=Pe(GAME/'reconstruction/native-game/runtime-expanded-v14/AA.exe',ACCEPTED_SHA)
 from inspect_native import RANGES
 for a,z in RANGES.values():assert pe.at(a,z-a)==v14.at(a,z-a)
 rows=[]
 for c in cases():
  try:out=Machine(pe,c).run()
  except Exception as e:raise RuntimeError(c['name']) from e
  rows.append({'input':c,'output':out})
 report={'schema':1,'passed':True,'original_sha256':ORIGINAL_SHA,'v14_sha256':ACCEPTED_SHA,'case_count':len(rows),'oracle_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'fixtures':rows}
 pathout=OUT/'native-navigation-fixtures.json';pathout.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'passed':True,'cases':len(rows),'sha256':hashlib.sha256(pathout.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
