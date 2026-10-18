"""Original ExperienceComponent constructor/rank setter/promotion/reset execution.

Rank table selection, XP arithmetic/cap/thresholds, actual classifiers and score
arithmetic execute unchanged. Modifier/rank effects, strings, allegiance and
experience propagation are explicit synchronous mutable leaves.
"""
from pathlib import Path
import copy,hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-body'))
from original_body import IMAGE,ACCEPTED,pe,bits
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE
from unicorn.x86_const import *
ACTOR,XP,DATA,ELEMENT,ELEMDEF,RANKS,STATS,OWNER,VT,STRING,STUB,STACK,STOP=[0x2000000+i*0x1000 for i in range(13)]
rank=lambda x:0 if x is None else 0x2020000+x*0x100
rankid=lambda p:None if p==0 else (p-0x2020000)//0x100
modifier=lambda x:0 if x is None else 0x2040000+x*16
mid=lambda p:None if p==0 else (p-0x2040000)//16

class Machine:
 def __init__(self,c):
  self.c=copy.deepcopy(c);self.s=self.c['state'];self.events=[];self.visited={};self.allegiance_calls=0
  self.u=Uc(UC_ARCH_X86,UC_MODE_32);IMAGE.load(self.u);self.u.mem_map(0,4096);self.u.mem_map(0x2000000,0x80000)
  self.u.reg_write(UC_X86_REG_FPCW,{'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[c['precision']]);self.u.hook_add(UC_HOOK_CODE,self.hook)
  self.w(XP+4,ACTOR);self.w(ACTOR,VT);self.w(ACTOR+4,DATA);self.w(VT+0x24,STUB);self.w(VT+0x108,STUB+16);self.w(OWNER+0x32c,STATS if c['has_stats'] else 0)
  self.w(STRING-4,100000);self.w(0xb39d8c,0x2070000)
  for i,v in enumerate(c['stats']):self.w(STATS+0x28+4*i,v)
  self.write_state()
 def w(self,a,v):self.u.mem_write(a,struct.pack('<I',v&0xffffffff))
 def r(self,a):return struct.unpack('<I',self.u.mem_read(a,4))[0]
 def arg(self,i=0):return self.r(self.u.reg_read(UC_X86_REG_ESP)+4+4*i)
 def ret(self,pop=0,value=0):
  sp=self.u.reg_read(UC_X86_REG_ESP);self.u.reg_write(UC_X86_REG_EIP,self.r(sp));self.u.reg_write(UC_X86_REG_ESP,sp+4+pop);self.u.reg_write(UC_X86_REG_EAX,value)
 def write_state(self):
  c=self.s['component'];a=self.s['actor'];t=self.s['ranks'];self.w(XP+8,c['experience']);self.w(XP+12,rank(c['current_rank']))
  for k,off in [('regiment',0x7c),('element',0x80),('experience_component',0xa4),('componentd8',0xd8)]:self.w(ACTOR+off,ELEMENT if a[k] else 0)
  self.w(ELEMENT+0x14,ELEMDEF if a['element_experience_cap'] is not None else 0)
  if a['element_experience_cap'] is not None:self.w(ELEMDEF+0x18,a['element_experience_cap'])
  self.w(DATA+0x4f8,RANKS);self.w(DATA+0x4fc,len(t['entries']))
  for i,k in enumerate(t['entries']):self.w(RANKS+i*4,rank(k))
  for k,v in t['definitions'].items():
   address=rank(int(k));self.w(address,v['threshold']);self.w(address+4,v['index']);self.w(address+8,modifier(v['modifier']))
 def read_state(self):
  self.s['component']['experience']=self.r(XP+8);self.s['component']['current_rank']=rankid(self.r(XP+12));return copy.deepcopy(self.s)
 def event(self,event):
  self.events.append({'event':event,'state':self.read_state()});name=event if isinstance(event,str) else next(iter(event))
  mutations=self.c['mutations']
  if mutations and mutations[0]['on']==name:
   m=mutations.pop(0)
   for group in ['component','actor'] :self.s[group].update(m.get(group,{}))
   if 'ranks' in m:self.s['ranks'].update(m['ranks'])
   self.c.update(m.get('world',{}));self.write_state()
 def hook(self,u,a,n,_):
  if 0x400000<=a<0xc00000:self.visited[a]=n
  if a==0x4cef70:self.event({'RemoveModifier':{'modifier':mid(self.arg())}});self.ret(8);return
  if a==0x4cef10:self.event({'AddModifier':{'modifier':mid(self.arg()),'kind':self.arg(1)}});self.ret(12,int(self.c['modifier_accepted']));return
  if a==0x4cebb0:self.event({'RankChanged':{'old':rankid(self.arg()),'new':rankid(self.arg(1))}});self.ret(8);return
  if a==STUB:self.event('ActorLabel');self.w(self.arg(),STRING);self.ret(4,self.arg());return
  if a==STUB+16:
   self.event('Allegiance');items=self.c['allegiances'];value=items[min(self.allegiance_calls,len(items)-1)];self.allegiance_calls+=1;self.ret(value=OWNER if value is not None else 0);return
  if a==0x42d490:self.event({'Notification':{'kind':self.arg(1),'allegiance':1 if self.arg(2) else None,'label':'actor'}});self.ret();return
  if a==0x4ceb70:self.event({'PropagateExperience':{'amount':self.arg()}});self.ret(4);return
 def run(self):
  self.w(STACK,STOP);self.u.reg_write(UC_X86_REG_ESP,STACK);self.u.reg_write(UC_X86_REG_ECX,XP)
  args={'new':[ACTOR],'set':[rank(self.c['requested'])],'apply':[self.c['amount'],self.c['notify']],'reset':[self.c['amount'],self.c['notify']]}[self.c['method']]
  for i,v in enumerate(args):self.w(STACK+4+i*4,v)
  regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
  for i,r in enumerate(regs):self.u.reg_write(r,0x11111111*(i+1))
  try:self.u.emu_start({'new':0x571690,'set':0x5716b0,'apply':0x571a60,'reset':0x571c50}[self.c['method']],STOP,count=100000)
  except Exception as e:raise RuntimeError(self.c['name']+' at '+hex(self.u.reg_read(UC_X86_REG_EIP))) from e
  assert self.u.reg_read(UC_X86_REG_EIP)==STOP and self.u.reg_read(UC_X86_REG_ESP)==STACK+4+len(args)*4
  assert [self.u.reg_read(r) for r in regs]==[0x11111111*(i+1) for i in range(4)]
  assert not self.c['mutations']
  return {'state':self.read_state(),'events':self.events,'stats':[self.r(STATS+0x28+i*4) for i in range(48)],'abi_preserved':True}

def base(name):
 return {'name':name,'method':'apply','requested':2,'amount':bits(11),'notify':True,'state':{
  'component':{'owner':1,'experience':bits(0),'current_rank':1},
  'ranks':{'entries':[1,2,3,4],'definitions':{str(i+1):{'threshold':bits(i*10),'index':i,'modifier':i+1} for i in range(4)}},
  'actor':{'regiment':True,'element':False,'experience_component':True,'componentd8':False,'element_experience_cap':None}},
 'has_stats':True,'stats':[0]*48,'allegiances':[1],'modifier_accepted':True,'mutations':[]}

def cases():
 rows=[]
 def add(name,fn=None):
  c=base(name)
  if fn:fn(c)
  rows.append(c)
 add('constructor',lambda c:c.update(method='new'))
 for rank_value in [None,1,2,4]:add('set_'+str(rank_value),lambda c,k=rank_value:c.update(method='set',requested=k))
 add('set_initial',lambda c:(c.update(method='set'),c['state']['component'].update(current_rank=None)))
 add('reset',lambda c:(c.update(method='reset'),c['state']['component'].update(experience=bits(100))))
 for amount in [-100,-1,0,.1,9.999,10,11,20,30,100,16777216]:add('amount_'+str(amount),lambda c,a=amount:c.update(amount=bits(a)))
 for cap in [-1,0,5,10,15,100]:add('cap_'+str(cap),lambda c,a=cap:(c.update(amount=bits(40)),c['state']['actor'].update(element=True,element_experience_cap=bits(a))))
 add('absent_element_ignores_cap',lambda c:c['state']['actor'].update(element_experience_cap=bits(1)))
 add('silent',lambda c:c.update(notify=False));add('no_allegiance',lambda c:c.update(allegiances=[None]));add('no_stats',lambda c:c.update(has_stats=False));add('declined_modifier',lambda c:c.update(modifier_accepted=False))
 for regiment,element,expc,d8 in [(False,False,True,False),(False,True,True,False),(False,True,False,False),(True,True,True,False),(True,False,True,True)]:
  add(f'class_{regiment}_{element}_{expc}_{d8}',lambda c,r=regiment,e=element,x=expc,d=d8:c['state']['actor'].update(regiment=r,element=e,experience_component=x,componentd8=d))
 add('null_next',lambda c:c['state']['ranks']['entries'].__setitem__(1,None))
 add('native_index_selects_next',lambda c:c['state']['ranks']['definitions']['1'].update(index=1))
 add('old_rank_final',lambda c:c['state']['component'].update(current_rank=4))
 add('zero_modifiers',lambda c:[r.update(modifier=None) for r in c['state']['ranks']['definitions'].values()])
 add('remove_changes_xp',lambda c:c.update(mutations=[{'on':'RemoveModifier','component':{'experience':bits(35)}}]))
 add('add_changes_rank',lambda c:c.update(mutations=[{'on':'AddModifier','component':{'current_rank':3}}]))
 add('rank_changes_xp',lambda c:(c.update(amount=bits(40)),c.update(mutations=[{'on':'RankChanged','component':{'experience':bits(5)}}])))
 add('label_changes_class',lambda c:c.update(mutations=[{'on':'ActorLabel','actor':{'regiment':False,'element':True}}]))
 add('propagate_changes_xp',lambda c:c.update(mutations=[{'on':'PropagateExperience','component':{'experience':bits(100)}}]))
 add('allegiance_changes_stats_class',lambda c:(c.update(notify=False),c.update(mutations=[{'on':'Allegiance','actor':{'regiment':False,'element':True}}])))
 add('set_callback_changes_rank',lambda c:(c.update(method='set'),c.update(mutations=[{'on':'AddModifier','component':{'current_rank':4}}])))
 add('set_remove_mutates_rank',lambda c:(c.update(method='set',requested=2),c.update(mutations=[{'on':'RemoveModifier','component':{'current_rank':4}}])))
 add('nonascending_thresholds',lambda c:(c['state']['ranks']['definitions']['2'].update(threshold=bits(10)),c['state']['ranks']['definitions']['3'].update(threshold=bits(5))))
 for p in ['Single24','Double53','Extended64']:
  for row in rows:yield {**copy.deepcopy(row),'name':row['name']+'_'+p,'precision':p}
def main():
 rows=[];visited={}
 for c in cases():
  m=Machine(c);rows.append({'input':c,'output':m.run()});visited.update(m.visited)
 windows=[]
 for a,n in sorted(visited.items()):
  raw=IMAGE.at(a,n);assert raw==ACCEPTED.at(a,n);windows.append({'address':hex(a),'bytes':raw.hex()})
 doc={'schema':1,'original_sha256':pe.ORIGINAL_SHA,'v14_sha256':pe.ACCEPTED_SHA,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'cases':rows,'visited_windows':windows,'seams':['modifier add/remove','rank-changed effects','actor strings','allegiance virtual','presentation notification','member/allegiance experience propagation']}
 path=HERE/'native-promotion-fixtures.json';path.write_text(json.dumps(doc,indent=2)+'\n');print(json.dumps({'cases':len(rows),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
