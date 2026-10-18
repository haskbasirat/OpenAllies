"""Execute original Body health routines, with explicit synchronous host seams.

No gameplay process/GUI. No guessed damage formula. Native setters, clamps,
health-condition predicates, arithmetic, heal and death dispatch run unchanged.
Host leaves record events and can perform specified fixture mutations.
"""
from pathlib import Path
import csv,hashlib,importlib.util,json,struct,sys
HERE=Path(__file__).resolve().parent; ROOT=HERE.parents[1]; GAME=ROOT.parent
spec=importlib.util.spec_from_file_location('pe',ROOT/'research/phase3-terrain/terrain_uv_oracle.py')
pe=importlib.util.module_from_spec(spec);spec.loader.exec_module(pe)
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE
from unicorn.x86_const import *
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
IMAGE=pe.Pe(GAME/'AA.exe',pe.ORIGINAL_SHA)
ACCEPTED=pe.Pe(GAME/'reconstruction/native-game/runtime-expanded-v14/AA.exe',pe.ACCEPTED_SHA)
sha=lambda b:hashlib.sha256(b).hexdigest()
bits=lambda v:struct.unpack('<I',struct.pack('<f',v))[0]
FUNCTIONS=[0x561960,0x561d40,0x5617e0,0x561e50,0x562200,0x562ba0,0x561c20,0x529c30,0x45d730,0x45d760]
OFFSETS={'health':0x10,'maximum':0x14,'defense':0x18,'dying_started':0x1c,'last_heal_time':0x20,'sink_duration':0x24}


def initial(health=75,maximum=100):
 return dict(health=bits(health),maximum=bits(maximum),defense=bits(8),dying_started=bits(-1),
             last_heal_time=bits(-999),sink_duration=0,last_damage=[0,0,0,0,0,0,10])


def environment(**kwargs):
 e=dict(actor_id=777,has_linked_body=False,destructed=False,destructible=True,
        restore_disabled_ratio=bits(.25),actor_flags=0,derived_normalization_maximum=bits(200),
        health_rules=[],owner_position=[bits(10),bits(20),bits(3)],has_recovery_component=True,world_time=bits(12.5))
 e.update(kwargs);return e


class Machine:
 BODY=0x2001000;ACTOR=0x2002000;DATA=0x2003000;HEAL=0x2004000;HEALER=0x2004100
 REGISTRY=0x2010000;WORLD=0x2080000;CACHE=0x2081000;THRESHOLD=0x2082000
 STACK=0x3008000;STOP=0x300f000;VIRTUAL=0x2090000
 def __init__(self,state,env,cw,mutations):
  self.u=Uc(UC_ARCH_X86,UC_MODE_32);IMAGE.load(self.u);self.u.mem_map(0,4096)
  self.u.mem_map(0x2000000,0x100000);self.u.mem_map(0x3000000,0x10000)
  self.u.reg_write(UC_X86_REG_FPCW,cw);self.events=[];self.mutations=list(mutations);self.env=dict(env)
  self.w(self.BODY+4,self.ACTOR);self.w(self.ACTOR+4,self.DATA);self.w(self.ACTOR+0x14,env['actor_id'])
  self.w(self.ACTOR,0x2091000);self.w(0x2091000+0x2c,self.VIRTUAL)
  self.w(0xb364c0,self.REGISTRY);self.w(0xb36534,self.WORLD);self.w(self.WORLD+0x130,0x2083000)
  self.w(0xb39ab0,self.CACHE);self.w(self.CACHE+0x188,self.THRESHOLD)
  self.w(self.DATA+0x284,env['derived_normalization_maximum']);self.w(self.DATA+0x288,state['defense'])
  for key,offset in OFFSETS.items():self.w(self.BODY+offset,state[key])
  for i,v in enumerate(state['last_damage']):self.w(self.BODY+0x28+4*i,v)
  self.write_env();self.nodes={};self.first_match=None
  rules=env['health_rules'];self.w(self.DATA+0x29c,0x2085000 if rules else 0)
  for i,rule in enumerate(rules):
   node=0x2085000+i*0x10;data=0x2086000+i*0x80;self.nodes[node]=i
   self.w(node,data);self.w(node+4,node+0x10 if i+1<len(rules) else 0)
   self.w(data+0x2c,rule['threshold']);self.w(data+0x30,rule['kind'])
  self.u.hook_add(UC_HOOK_CODE,self.hook)
 def w(self,a,v):self.u.mem_write(a,struct.pack('<I',v&0xffffffff))
 def r(self,a):return struct.unpack('<I',self.u.mem_read(a,4))[0]
 def arg(self,i=0):return self.r(self.u.reg_read(UC_X86_REG_ESP)+4+4*i)
 def ret(self,n=0,value=0):
  sp=self.u.reg_read(UC_X86_REG_ESP);self.u.reg_write(UC_X86_REG_EIP,self.r(sp));self.u.reg_write(UC_X86_REG_ESP,sp+4+n);self.u.reg_write(UC_X86_REG_EAX,value)
 def state(self):
  return {**{k:self.r(self.BODY+o) for k,o in OFFSETS.items()},'last_damage':[self.r(self.BODY+0x28+4*i) for i in range(7)]}
 def write_env(self):
  e=self.env;self.u.mem_write(self.DATA+0x291,bytes([e['destructible']]))
  self.w(self.ACTOR+8,int(e['destructed']));self.w(self.ACTOR+0x100,e['actor_flags'])
  self.w(self.ACTOR+0x9c,0x2084000 if e['has_recovery_component'] else 0)
  self.w(self.WORLD+0xe0,e['world_time']);self.w(self.THRESHOLD,e['restore_disabled_ratio'])
  for i,v in enumerate(e['owner_position']):self.w(self.ACTOR+0x20+4*i,v)
 def event(self,event):
  self.events.append({'event':event,'body':self.state()})
  name=event if isinstance(event,str) else next(iter(event))
  if self.mutations and self.mutations[0]['on']==name:
   mutation=self.mutations.pop(0)
   for key,v in mutation.get('body',{}).items():self.w(self.BODY+OFFSETS[key],v)
   self.env.update(mutation.get('environment',{}));self.write_env()
 def hook(self,u,pc,size,user):
  if pc==0x4db0c0:
   self.w(self.u.reg_read(UC_X86_REG_ECX)+4,self.arg());self.ret(4,value=self.BODY)
  elif pc==0x4d0050:self.event('RefreshActorStance');self.ret()
  elif pc==0x4d64f0:self.event({'ActorState':{'state':self.arg(),'enabled':bool(self.arg(1))}});self.ret(8)
  elif pc==0x566ac0:self.event({'Recovered':{'token':self.arg()}});self.ret(4)
  elif pc==0x4cf110:self.ret(value=0) # No-kingdom derived getter; actual45D730 executes.
  elif pc==0x562061:self.first_match=self.nodes[self.u.reg_read(UC_X86_REG_ECX)]
  elif pc==0x429290:
   assert self.arg()==self.ACTOR
   event={'first_matching_rule':self.first_match,'old_ratio':self.arg(2),'new_ratio':self.arg(3),
          'position':[self.r(self.arg(1)+i*4) for i in range(3)],'offset':[self.r(self.arg(4)+i*4) for i in range(3)]}
   self.event({'HealthEffects':event});self.ret(20)
  elif pc==0x50c380:
   p=self.arg(1);assert self.arg()==self.ACTOR
   source=self.r(p);source_id=self.r(source+0x14) if source else 0
   self.event({'HealingNotification':{'source_id':source_id,'amount':self.r(p+4),'token':p}});self.ret(8)
  elif pc==self.VIRTUAL:self.event({'ActorVirtual2c':{'argument':self.arg()}});self.ret(4)
  elif pc==0x4d7960:self.event('BeginDeath');self.ret()
 def call(self,pc,args=(),pop=0):
  self.w(self.STACK,self.STOP)
  for i,v in enumerate(args):self.w(self.STACK+4+4*i,v)
  self.u.reg_write(UC_X86_REG_ESP,self.STACK);self.u.reg_write(UC_X86_REG_ECX,self.BODY)
  regs={UC_X86_REG_EBX:0xabab0001,UC_X86_REG_ESI:0xabab0002,UC_X86_REG_EDI:0xabab0003,UC_X86_REG_EBP:0xabab0004}
  for reg,v in regs.items():self.u.reg_write(reg,v)
  try:self.u.emu_start(pc,self.STOP,count=100000)
  except Exception as exc:raise RuntimeError(hex(self.u.reg_read(UC_X86_REG_EIP))) from exc
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP
  assert self.u.reg_read(UC_X86_REG_ESP)==self.STACK+4+pop
  for reg,v in regs.items():assert self.u.reg_read(reg)==v,(hex(pc),reg)


def main():
 rows=[]
 specs=[]
 def add(name,op,state,env,**args):specs.append(dict(name=name,operation=op,state=state,environment=env,**args))
 for value in [-50,0,25,75,100,500]:add(f'clamp_{value}','set',initial(),environment(),requested=bits(value),notify=True,token=17)
 add('no_notify','set',initial(),environment(),requested=bits(50),notify=False,token=0)
 for old,new in [(25,0),(0,24),(0,25),(0,26),(50,25)]:
  add(f'disabled_{old}_{new}','set',initial(old),environment(destructible=False),requested=bits(new),notify=True,token=11)
 for kind in range(5):
  for old,new in [(75,25),(50,25),(25,50),(25,75),(100,75),(100,74.999),(25,50.001)]:
   add(f'fx_{kind}_{old}_{new}','set',initial(old),environment(health_rules=[dict(kind=kind,threshold=bits(.5 if kind<2 else .25))]),requested=bits(new),notify=True,token=19)
 add('fx_first_match','set',initial(75),environment(health_rules=[dict(kind=0,threshold=bits(.5)),dict(kind=2,threshold=bits(.1))]),requested=bits(25),notify=True,token=19)
 add('fx_suppressed','set',initial(75),environment(actor_flags=0x200000,health_rules=[dict(kind=0,threshold=bits(.5))]),requested=bits(25),notify=True,token=19)
 add('fx_derived_max','set',initial(100),environment(actor_flags=0x20000000,health_rules=[dict(kind=0,threshold=bits(.4))]),requested=bits(60),notify=True,token=19)
 add('no_recovery_component','set',initial(),environment(has_recovery_component=False),requested=bits(100),notify=True,token=19)
 for amount in [-1,0,10,25,50]:add(f'heal_{amount}','heal',initial(),environment(),amount=bits(amount),source_id=22,token=Machine.HEAL,converts_to_damage=False)
 add('heal_full','heal',initial(100),environment(),amount=bits(10),source_id=0,token=Machine.HEAL,converts_to_damage=False)
 add('heal_destructed','heal',initial(),environment(destructed=True),amount=bits(10),source_id=22,token=Machine.HEAL,converts_to_damage=False)
 for health,maximum in [(100,50),(25,200),(-2,0),(0,0.00001),(0.0001,0.0001)]:
  add(f'maximum_{health}_{maximum}','maximum',initial(),environment(),requested=bits(health),maximum=bits(maximum))
 for health,dead in [(0,False),(-1,False),(1,False),(0,True)]:add(f'death_{health}_{dead}','death',initial(health),environment(destructed=dead))
 for health,old_max,new_max,keep in [(75,100,200,True),(75,100,40,True),(75,100,40,False),(0,100,200,True),(0,100,.25,True),(.1,100,.25,False),(1,3,7,True),(2,7,99999,True)]:
  add(f'recalculate_{health}_{old_max}_{new_max}_{keep}','recalculate',initial(health,old_max),environment(),maximum=bits(new_max),preserve_ratio=keep)
 for start,now,rot,sink in [(-1,10,2,3),(0,1,2,3),(0,2,2,3),(0,5,2,3),(0,8,2,3),(0,5,2,0),(0,5,2,.0001),(1,7,1,3),(100000,100000.0625,.01,.3)]:
  state=initial();state['dying_started']=bits(start);state['sink_duration']=bits(sink)
  add(f'sink_{start}_{now}_{rot}_{sink}','sink',state,environment(),now=bits(now),rot_time=bits(rot))
 add('callback_changes_health','set',initial(),environment(destructible=False),requested=bits(80),notify=True,token=44,mutations=[dict(on='RefreshActorStance',body={'health':bits(0)})])
 add('callback_changes_max','set',initial(),environment(),requested=bits(80),notify=True,token=44,mutations=[dict(on='RefreshActorStance',body={'maximum':bits(80)})])
 add('effect_changes_health','set',initial(75),environment(health_rules=[dict(kind=0,threshold=bits(.5))]),requested=bits(25),notify=True,token=19,mutations=[dict(on='HealthEffects',body={'health':bits(100)})])
 add('death_callback_marks_destructed','death',initial(0),environment(),mutations=[dict(on='ActorVirtual2c',environment={'destructed':True})])
 add('heal_callback_and_clock','heal',initial(),environment(),amount=bits(20),source_id=22,token=Machine.HEAL,converts_to_damage=False,mutations=[dict(on='HealingNotification',body={'health':bits(40)},environment={'world_time':bits(20)}),dict(on='RefreshActorStance',environment={'world_time':bits(30)})])
 for name,cw in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
  for spec in specs:
   m=Machine(spec['state'],spec['environment'],cw,spec.get('mutations',[]));op=spec['operation']
   if op=='set':m.call(0x561e50,[spec['requested'],spec['token'],int(spec['notify']),0],16)
   elif op=='maximum':m.call(0x562200,[spec['requested'],spec['maximum']],8)
   elif op=='death':m.call(0x561c20)
   elif op=='recalculate':m.w(m.DATA+0x284,spec['maximum']);m.call(0x561d40,[0,0,int(spec['preserve_ratio'])],12)
   elif op=='sink':
    m.w(m.DATA+0x27c,spec['rot_time']);m.call(0x5617e0,[spec['now']],4)
    code=b'\xd9\x1d'+struct.pack('<I',0x2007000);m.u.mem_write(0x2092000,code);m.u.emu_start(0x2092000,0x2092000+len(code),count=2)
   else:
    m.w(m.HEAL,m.HEALER if spec['source_id'] else 0);m.w(m.HEALER+0x14,spec['source_id']);m.w(m.HEAL+4,spec['amount']);m.call(0x562ba0,[m.HEAL],4)
   assert not m.mutations
   row={**spec,'precision':name,'result':m.state(),'events':m.events}
   if op=='sink':row['fraction']=m.r(0x2007000)
   rows.append(row)
  m=Machine(initial(),environment(),cw,[]);m.call(0x561960,[m.ACTOR],4)
  rows.append(dict(name='constructor',operation='constructor',precision=name,state=initial(),environment=environment(),result=m.state(),events=m.events))
 functions={int(r['address'],16):int(r['bodyBytes']) for r in csv.DictReader((GAME/'reconstruction/research/native-aa/functions.tsv').open(),delimiter='\t')}
 ranges={};cs=Cs(CS_ARCH_X86,CS_MODE_32)
 for va in FUNCTIONS:
  size=functions[va];data=IMAGE.at(va,size);assert data==ACCEPTED.at(va,size)
  ranges[hex(va)]=dict(size=size,sha256=sha(data));(HERE/f'{va:08x}.asm.txt').write_text('\n'.join(f'{i.address:08x}: {i.mnemonic} {i.op_str}' for i in cs.disasm(data,va))+'\n')
 report=dict(schema=1,passed=True,producer_sha256=sha(Path(__file__).read_bytes()),original_sha256=pe.ORIGINAL_SHA,accepted_sha256=pe.ACCEPTED_SHA,ranges=ranges,cases=rows,
             seams=['Actor base-constructor stores owner only; derived health/defense getters run original with null kingdom.',
                    'Actor stance/state, effect-chain, recovery, heal notification and death leaves are synchronous recorded host callbacks with explicit fixture mutations.',
                    'No valid linked Body and no attacker-direction or damage-converting healer. No weapon/armor/capture/kill-credit/death-ability behavior invented.'])
 p=HERE/'body-native-oracle.json';p.write_text(json.dumps(report,indent=2)+'\n');print(len(rows),sha(p.read_bytes()))


if __name__=='__main__':main()
