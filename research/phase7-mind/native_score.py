"""Original52ED30 utility arithmetic, with movement/player/effect lookup leaves.

Native local/shared definition loads, every health branch and x87 arithmetic run
unchanged. No synthetic utility result is supplied. The player multiplier501650
and movement admission4C8A40 are explicit live dependencies.
"""
from pathlib import Path
import copy,hashlib,json,struct,sys,random
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE))
from native_decision import DecisionMachine,cases as decision_cases,mp,DECISION,ACTOR,ABILITY,RETURN,VALUE,ptr,ident,f64,sha
from native_constructors import original,modified
from unicorn.x86_const import *
LOCAL,SHARED,ACTDEF,BODY,EFFECT,PLAYER,OUTPUT=0x4650000,0x4660000,0x4680000,0x46a0000,0x46b0000,0x46c0000,0x46d0000
bits=lambda f:struct.unpack('<I',struct.pack('<f',f))[0]
class ScoreMachine(DecisionMachine):
 def __init__(self,c):
  super().__init__(c);self.write_score()
 def write_score(self):
  for k,local in self.c['locals'].items():
   key=int(k);p=ptr(LOCAL,key);self.w(ptr(ABILITY,key)+8,p);self.w(p,ptr(SHARED,local['shared_definition']));self.w(p+0x20,local['priority'])
   for i,v in enumerate(local['factors']):self.w(p+0x24+i*4,v)
   for i,v in enumerate(local['bonuses']):self.w(p+0x30+i*4,v)
  for k,d in self.c['shared'].items():p=ptr(SHARED,int(k));self.w(p+0x54,d['flags54']);self.w(p+0x64,d['word64_bits'])
  for k,a in self.c['actors'].items():
   actor=ptr(ACTOR,int(k));definition=ACTDEF+int(k)*0x1000;self.w(actor+4,definition);self.u.mem_write(definition+0x214,bytes([a['player_weight']]))
   for i,v in enumerate(a['bonuses']):self.w(definition+0x204+i*4,v)
   self.w(actor+0x60,ptr(BODY,int(k)));self.w(ptr(BODY,int(k))+0x10,a['health']['current_bits']);self.w(ptr(BODY,int(k))+0x14,a['health']['maximum_bits'])
  for key,value in self.c['effects'].items():
   if value is not None:self.w(EFFECT+int(key)*0x100+0x14,value)
  self.w(0xb3d2e4,1);self.w(0xb3d2e0,VALUE+8);self.w(VALUE+8,self.c['health_bonus'])
 def score_event(self,name,args):
  self.c['decision']=self.snapshot_decision();self.events.append([name,*args,copy.deepcopy(self.c['decision'])]);mutations=self.c['mutations']
  if mutations and mutations[0]['event']==name:
   m=mutations.pop(0);self.c['decision'].update(m.get('decision',{}))
   if 'mind_actor' in m:self.c['arena']['minds']['1']['actor']=m['mind_actor']
   for k,changes in m.get('locals',{}).items():self.c['locals'][k].update(changes)
   self.write_world();self.write_score()
 def hook(self,u,pc,size,data):
  if original.base<=pc<original.base+original.image_size:self.visited[pc]=bytes(u.mem_read(pc,size))
  cx=u.reg_read(UC_X86_REG_ECX);s=u.reg_read(UC_X86_REG_ESP)
  if pc==0x52ed30:return
  if pc==0x4c8a40:self.score_event('movement',[ident(ABILITY,cx),ident(ACTOR,self.r(s+4))]);self.ret(int(self.c['movement_wait']),4)
  elif pc==0x4be8e0:
   kind=self.r(s+4);value=self.c['effects'].get(str(kind));self.ret(EFFECT+kind*0x100 if value is not None else 0,4)
  elif pc==0x4cf110:
   key=ident(ACTOR,cx);self.ret(0 if key is None or self.c['actors'][str(key)]['player'] is None else ptr(PLAYER,self.c['actors'][str(key)]['player']))
  elif pc==0x501650:
   self.score_event('player',[ident(PLAYER,cx),self.r(s+4)]);self.u.mem_write(VALUE,struct.pack('<Q',self.c['player_factor_bits']));u.reg_write(UC_X86_REG_EIP,RETURN)
  else:super().hook(u,pc,size,data)
 def run_score(self):
  self.reset(0);self.u.reg_write(UC_X86_REG_ECX,mp(1));self.w(self.STACK+4,DECISION);regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
  self.u.reg_write(UC_X86_REG_FPCW,{'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[self.c['precision']])
  for i,r in enumerate(regs):self.u.reg_write(r,0x11111111*(i+1))
  self.u.emu_start(0x52ed30,self.STOP,count=25000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+8
  assert [self.u.reg_read(r)for r in regs]==[0x11111111*(i+1)for i in range(4)]
  self.u.mem_write(self.STOP,b'\xd9\x15'+struct.pack('<I',OUTPUT)+b'\xdb\x3d'+struct.pack('<I',OUTPUT+8));self.u.emu_start(self.STOP,self.STOP+12)
  sig,exp=struct.unpack('<QH',self.u.mem_read(OUTPUT+8,10));assert not self.c['mutations']
  return {'stored':self.r(OUTPUT),'extended_significand':sig,'extended_sign_exponent':exp,'events':self.events,'decision':self.snapshot_decision()}
def cases():
 base=copy.deepcopy(next(decision_cases()));base.pop('precision');base['decision']['target']=3;base['decision']['status']=0;base.update(locals={'1':{'shared_definition':1,'priority':bits(.3333333),'factors':list(map(bits,[2.7,-1.8,.71])),'bonuses':list(map(bits,[1.3,.81,-.4]))}},shared={'1':{'flags54':0,'word64_bits':0}},effects={'3':bits(25),'4':bits(5)},health_bonus=bits(1.25),movement_wait=False,player_factor_bits=f64(1.7))
 for a in base['actors'].values():a.update(bonuses=list(map(bits,[.11,.22,.33])),player_weight=False,player=8,health={'current_bits':bits(80),'maximum_bits':bits(100)})
 rows=[]
 def add(name,change):c=copy.deepcopy(base);c['name']=name;change(c);rows.append(c)
 for status in [0,1,2,3,4,0xffffffff]:
  for wait in [False,True]:
   for heal in [False,True]:add(f'status{status}_wait{wait}_heal{heal}',lambda c,s=status,w=wait,h=heal:(c['decision'].update(status=s),c.update(movement_wait=w),c['shared']['1'].update(flags54=64 if h else 0)))
 for missing in [-10,0,10,25,100]:
  for amount in [None,-10,10,25,100]:add(f'healing_{missing}_{amount}',lambda c,m=missing,a=amount:(c['shared']['1'].update(flags54=64),c['actors']['3']['health'].update(current_bits=bits(100-m)),c['effects'].update({'3':None if a is None else bits(a)})))
 for scalar in [-1,0,1]:
  for enabled in [False,True]:
   for player in [None,8]:add(f'player_{scalar}_{enabled}_{player}',lambda c,s=scalar,e=enabled,v=player:(c['shared']['1'].update(word64_bits=bits(s)),c['actors']['1'].update(player_weight=e),c['actors']['3'].update(player=v)))
 add('movement_callback_changes_cached_fields',lambda c:(c['decision'].update(status=2),c.update(mutations=[{'event':'movement','mind_actor':2,'decision':{'status':1},'locals':{'1':{'priority':bits(5)}}}]),c['actors']['2'].update(bonuses=list(map(bits,[7,8,9])))))
 add('movement_keeps_cached_ability',lambda c:(c['decision'].update(status=2),c['locals'].update({'2':{**copy.deepcopy(c['locals']['1']),'priority':bits(500)}}),c.update(mutations=[{'event':'movement','decision':{'ability':2}}])))
 add('movement_keeps_cached_shared_definition',lambda c:(c['decision'].update(status=2),c['shared']['1'].update(flags54=64),c['shared'].update({'2':{'flags54':0,'word64_bits':0}}),c.update(mutations=[{'event':'movement','locals':{'1':{'shared_definition':2}}}])))
 add('player_extended_product',lambda c:(c['shared']['1'].update(word64_bits=bits(1)),c['actors']['1'].update(player_weight=True),c.update(player_factor_bits=f64(1+2**-40))))
 rng=random.Random(0x52ed30)
 for i in range(24):
  values=[bits(rng.uniform(-100,100))for _ in range(7)];add(f'fractional{i}',lambda c,v=values,s=i%3:(c['decision'].update(status=s),c['locals']['1'].update(priority=v[0],factors=v[1:4],bonuses=v[4:7])))
 for p in ['Single24','Double53','Extended64']:
  for c in rows:yield {**copy.deepcopy(c),'name':c['name']+'_'+p,'precision':p}
def main():
 rows=[];visited={}
 for c in cases():
  try:m=ScoreMachine(c);result=m.run_score()
  except Exception:print(c['name']);raise
  rows.append({'input':c,'output':result});visited.update(m.visited)
 for a,b in visited.items():assert b==original.range(a,len(b))==modified.range(a,len(b)),hex(a)
 d={'schema':1,'method':__doc__,'original_sha256':sha(original.b),'v14_sha256':sha(modified.b),'producer_sha256':sha(Path(__file__).read_bytes()),'instructions':[{'va':hex(a),'bytes':b.hex()}for a,b in sorted(visited.items())],'cases':rows}
 p=HERE/'native-score.json';p.write_text(json.dumps(d,indent=2),encoding='utf-8');print(json.dumps({'cases':len(rows),'sha256':sha(p.read_bytes())}))
if __name__=='__main__':main()
