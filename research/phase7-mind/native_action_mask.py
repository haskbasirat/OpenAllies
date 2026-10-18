"""Original Element530510/Regiment521E10/Aircraft518720 action-mask contributors.
Real517FA0/51FF80 bit writes and all four Performer capability helpers execute.
Actor state/player/component predicates and Organization578140 remain live leaves.
"""
from pathlib import Path
import copy,hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE))
from native_lifecycle import LifecycleMachine,cases as lifecycle_cases,mp,VT
from native_constructors import original,modified
from unicorn.x86_const import *
ACTOR,COMP,PERF,ABILITY,DEF,LIST,PLAYER,CACHE,REG=0x4500000,0x4600000,0x4620000,0x4640000,0x4650000,0x4660000,0x4680000,0x46a0000,0x46b0000
ASTATE=0x4740000
ap=lambda i:0 if i is None else ACTOR+(i-100)*0x1000
cp=lambda i:0 if i is None else COMP+(i-200)*0x1000
pp=lambda i:0 if i is None else PERF+(i-300)*0x1000
pl=lambda i:0 if i is None else PLAYER+(i-400)*0x1000
aid=lambda p:(p-ACTOR)//0x1000+100
cid=lambda p:(p-COMP)//0x1000+200
sha=lambda b:hashlib.sha256(b).hexdigest()
bits=lambda f:struct.unpack('<I',struct.pack('<f',f))[0]
class MaskMachine(LifecycleMachine):
 def __init__(self,c):
  super().__init__(c);self.predicate_count=0;self.w(VT+0x34,ASTATE);self.w(0xb39ab0,CACHE);self.w(CACHE+0x108,CACHE+0x1000);self.w(CACHE+0xd8,CACHE+0x1004);self.w(CACHE+0x1000,int(c['setting108']));self.w(CACHE+0x1004,c['settingd8']);self.w(0xb364c0,REG)
  for k,v in c['actors'].items():self.w(ap(int(k))+0xf0,v['action_bits']&0xffffffff);self.w(ap(int(k))+0xf4,v['action_bits']>>32)
  self.write_fields();self.write_abilities()
 def write_fields(self):
  self.w(mp(1)+4,ap(self.c['arena']['minds']['1']['actor']))
  for k,a in self.c['actors'].items():
   p=ap(int(k));self.w(p,VT);self.w(p+8,a['flags']);self.w(p+0x80,cp(a['element']));self.w(p+0x7c,cp(a['organization']));self.w(p+0x74,pp(a['performer']));self.w(p+0x6c,int(a['mover']));self.w(p+0x9c,int(a['component9c']));self.w(p+0x108,a['word108']);self.w(p+0x118,a['word118']);self.w(p+0xac,cp(a['componentac']))
  for k,leader in self.c['elements'].items():self.w(cp(int(k))+0x10,ap(leader))
  for k,o in self.c['organizations'].items():self.w(cp(int(k))+0xd0,o['link']);self.w(cp(int(k))+0xb8,o['values'][0]);self.w(cp(int(k))+0x5c,o['values'][1])
  for k,flags in self.c['players'].items():self.w(pl(int(k)),flags)
  for raw,target in self.c['registry'].items():
   raw=int(raw);self.u.mem_write(REG+4+(raw&65535)*2,struct.pack('<H',raw>>16));self.w(REG+0x20004+(raw&65535)*4,ap(target))
  head=0
  for state in reversed(self.c['arena']['minds']['1']['stack']):
   node=self.alloc(8);value={4:mp(1)+0x20c,5:mp(1)+0x1cc,6:mp(1)+0x270}.get(state,0 if state is None else 0x4100000+state*0x1000);self.w(node,value);self.w(node+4,head);head=node
  self.w(mp(1)+0x10,head);self.w(mp(1)+0x1cc+4,mp(1));self.w(mp(1)+0x20c+4,mp(1));self.w(mp(1)+0x270+4,mp(1))
 def write_abilities(self):
  index=0
  for k,abilities in self.c['abilities'].items():
   array=LIST+(int(k)-300)*0x100;self.w(pp(int(k))+0x14,array);self.w(pp(int(k))+0x18,len(abilities))
   for i,a in enumerate(abilities):
    p=ABILITY+index*0x100;d=DEF+index*0x100;index+=1;self.w(array+i*4,p);self.w(p+4,d);self.u.mem_write(p+0x18,bytes([a['enabled']]));self.w(d+0x54,a['flags']);self.w(d+0x28,a['required_targets']);head=0
    for kind in reversed(a['effect_kinds']):node=self.alloc(8);effect=self.alloc(8);self.w(effect+4,kind);self.w(node,effect);self.w(node+4,head);head=node
    self.w(d+0x68,head)
 def event_mask(self,name,args):
  self.events.append([name,*args]);mutations=self.c['mutations']
  if mutations and mutations[0]['event']==name:
   m=mutations.pop(0)
   if 'owner' in m:self.c['arena']['minds']['1']['actor']=m['owner']
   if 'stack' in m:self.c['arena']['minds']['1']['stack']=m['stack']
   for k,v in m.get('actors',{}).items():self.c['actors'][k].update(v)
   self.write_fields()
 def hook(self,u,pc,size,data):
  if original.base<=pc<original.base+original.image_size:self.visited[pc]=bytes(u.mem_read(pc,size))
  cx=u.reg_read(UC_X86_REG_ECX);s=u.reg_read(UC_X86_REG_ESP)
  if pc==ASTATE:
   index=self.r(s+4);self.event_mask('state',[aid(cx),index]);self.ret(int(self.c['state_results'].get(str(index),False)),4)
  elif pc==0x4cf110:
   actor=aid(cx);self.event_mask('player',[actor]);self.ret(pl(self.c['actor_players'][str(actor)]))
  elif pc==0x578140:self.ret(self.c['organizations'][str(cid(cx))]['status'])
  elif pc==0x4d12c0:
   self.event_mask('predicate12',[aid(cx)]);values=self.c['predicate12'];value=values[min(self.predicate_count,len(values)-1)];self.predicate_count+=1;self.ret(int(value))
  elif pc in [0x575bb0,0x575c00]:self.event_mask('component',[cid(cx),pc]);assert self.r(s+4)&255==1;self.ret(int(self.c['ac'][str(pc)]),4)
  else:super().hook(u,pc,size,data)
 def run_mask(self):
  self.reset(0);self.u.reg_write(UC_X86_REG_FPCW,{'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[self.c['precision']]);self.u.reg_write(UC_X86_REG_ECX,mp(1));regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
  for i,r in enumerate(regs):self.u.reg_write(r,0x11111111*(i+1))
  self.u.emu_start({'element':0x530510,'regiment':0x521e10,'aircraft':0x518720,'exhausted':0x51d450,'constant_false':0x6a71f0}[self.c['kind']],self.STOP,count=30000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+4
  assert[self.u.reg_read(r)for r in regs]==[0x11111111*(i+1)for i in range(4)];assert not self.c['mutations'],self.c['name']
  return {'actions':{k:self.r(ap(int(k))+0xf0)|(self.r(ap(int(k))+0xf4)<<32)for k in self.c['actors']},'events':self.events,'predicate':bool(self.u.reg_read(UC_X86_REG_EAX)&255)if self.c['kind']in['exhausted','constant_false']else None}
def cases():
 a={'flags':0,'element':201,'organization':202,'performer':301,'mover':True,'component9c':False,'word108':1,'word118':0,'componentac':203,'action_bits':0xdeadbeef55555555}
 abilities=[{'enabled':True,'flags':0,'required_targets':128,'effect_kinds':[1,3,5]},{'enabled':True,'flags':8,'required_targets':0,'effect_kinds':[]}]
 base={'name':'base','kind':'element','arena':copy.deepcopy(lifecycle_cases()[0]['arena']),'actors':{'101':a,'102':copy.deepcopy(a)},'elements':{'201':102},'organizations':{'202':{'link':0x10002,'status':1,'values':[bits(5),bits(7)]}},'abilities':{'301':abilities,'302':[]},'players':{'401':0},'actor_players':{'101':401,'102':None},'registry':{'65538':None},'setting108':True,'settingd8':bits(2),'state_results':{'1':False,'7':False,'20':False},'predicate12':[False],'ac':{str(0x575bb0):True,str(0x575c00):False},'mutations':[]}
 rows=[]
 def add(name,change):c=copy.deepcopy(base);c['name']=name;change(c);rows.append(c)
 for flags in [0,1<<29]:
  for leader in [None,102]:
   for setting in [False,True]:add(f'element_flags{flags}_leader{leader}_setting{setting}',lambda c,f=flags,l=leader,s=setting:(c['actors']['101'].update(flags=f),c['elements'].update({'201':l}),c.update(setting108=s)))
 for field,value in [('mover',False),('performer',None),('component9c',True),('word108',0),('componentac',None)]:add(f'element_{field}_{value}',lambda c,f=field,v=value:c['actors']['101'].update({f:v}))
 for n,items in enumerate([[],[{**abilities[0],'enabled':False}],[{**abilities[0],'required_targets':0}],[{**abilities[0],'effect_kinds':[1,2]}],[abilities[1]],[abilities[0]]]):
  for state in [False,True]:add(f'abilities{n}_state1{state}',lambda c,it=items,s=state:(c['abilities'].update({'301':it}),c['state_results'].update({'1':s})))
 for flags in [0,0x800]:add(f'playerflags{flags}',lambda c,f=flags:c['players'].update({'401':f}))
 add('linked_target',lambda c:c['registry'].update({'65538':102}))
 for x,y in [(False,False),(False,True),(True,True)]:add(f'ac{x}_{y}',lambda c,x=x,y=y:c.update(ac={str(0x575bb0):x,str(0x575c00):y}))
 for stack in [[],[4],[5],[1,5]]:
  for status in [0,1,2]:add(f'regiment_stack{stack}_status{status}',lambda c,s=stack,t=status:(c.update(kind='regiment'),c['arena']['minds']['1'].update(stack=s),c['organizations']['202'].update(status=t)))
 for field,value in [('word118',1)]:add(f'regiment_{field}',lambda c,f=field,v=value:(c.update(kind='regiment'),c['actors']['101'].update({f:v})))
 for state in [False,True]:
  for predicate in [[False],[True],[False,True],[True,False]]:add(f'regiment_state7{state}_pred{predicate}',lambda c,s=state,v=predicate:(c.update(kind='regiment',predicate12=v),c['state_results'].update({'7':s}),c['arena']['minds']['1'].update(stack=[5])))
 for setting,baseval,current in [(2,5,7),(1,5,7),(1.000001,5,7),(1e10,-1e10,1),(1,0.0000001,2)]:add(f'regiment_values{setting}_{baseval}_{current}',lambda c,s=setting,b=baseval,v=current:(c.update(kind='regiment',settingd8=bits(s)),c['state_results'].update({'7':True}),c['arena']['minds']['1'].update(stack=[5]),c['organizations']['202'].update(values=[bits(b),bits(v)])))
 for initial in [0,0xffffffffffffffff,0x1234567887654321]:add(f'aircraft{initial}',lambda c,i=initial:(c.update(kind='aircraft'),c['actors']['101'].update(action_bits=i)))
 for stack in [[],[6],[5,6],[None,6],[4,5],[6,6]]:add(f'exhausted_stack{stack}',lambda c,s=stack:(c.update(kind='exhausted'),c['arena']['minds']['1'].update(stack=s)))
 add('constant_false',lambda c:c.update(kind='constant_false'))
 for targets in [256,0xffffffff]:add(f'repair_targets{targets}',lambda c,t=targets:c['abilities'].update({'301':[{**abilities[0],'required_targets':t}]}))
 add('element_state_changes_performer',lambda c:(c['state_results'].update({'1':True}),c.update(mutations=[{'event':'state','actors':{'101':{'performer':302}}}])))
 add('element_player_changes_owner',lambda c:c.update(mutations=[{'event':'player','owner':102}]))
 add('element_ac_changes_owner',lambda c:c.update(mutations=[{'event':'component','owner':102}]))
 add('regiment_player_changes_cached_fields',lambda c:(c.update(kind='regiment',mutations=[{'event':'player','stack':[5],'actors':{'101':{'word118':1}}}]),c['state_results'].update({'7':True})))
 for p in ['Single24','Double53','Extended64']:
  for c in rows:yield {**copy.deepcopy(c),'name':c['name']+'_'+p,'precision':p}
def main():
 rows=[];visited={}
 for c in cases():
  try:m=MaskMachine(c);result=m.run_mask()
  except Exception:print(c['name']);raise
  rows.append({'input':c,'output':result});visited.update(m.visited)
 for a,b in visited.items():assert b==original.range(a,len(b))==modified.range(a,len(b)),hex(a)
 d={'schema':1,'method':__doc__,'original_sha256':sha(original.b),'v14_sha256':sha(modified.b),'producer_sha256':sha(Path(__file__).read_bytes()),'instructions':[{'va':hex(a),'bytes':b.hex()}for a,b in sorted(visited.items())],'cases':rows}
 p=HERE/'native-action-mask.json';p.write_text(json.dumps(d,indent=2),encoding='utf-8');print(json.dumps({'cases':len(rows),'sha256':sha(p.read_bytes())}))
if __name__=='__main__':main()
