"""Original54C280→5411D0→54BDC0→4DB300 control and cleanup.

546AF0 is explicitly replaced by ordered bool/mutation scripts; no opcode or
tutorial execution support is claimed by these sequencing fixtures.
"""
from pathlib import Path
import copy,hashlib,itertools,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-body'));from original_body import IMAGE,ACCEPTED,pe,bits
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE
from unicorn.x86_const import *
BASE,MANAGER,WORLD,APPLICATION,STACK,STOP=[0x4000000+i*0x1000 for i in range(6)]
class Machine:
 def __init__(self,c):
  self.c=copy.deepcopy(c);self.u=Uc(UC_ARCH_X86,UC_MODE_32);IMAGE.load(self.u);self.u.mem_map(0,4096);self.u.mem_map(BASE,0x1000000);self.bump=BASE+0x10000
  self.visited={};self.calls=[];self.diagnostics=[];self.evaluated=[];self.fired=[];self.releases=[];self.nodes={};self.leaves={};self.steps={};self.triggerptr=[];self.current=None
  self.u.hook_add(UC_HOOK_CODE,self.hook);self.w(0xb36520,MANAGER);self.w(0xb36534,WORLD);self.w(0xb36550,APPLICATION);self.w(WORLD+0xe0,c['time_bits']);self.w(MANAGER+0x2c,BASE+0x8000)
  self.w(MANAGER+0x10,c['context_word_10'])
  for ti,t in enumerate(c['triggers']):
   p=self.alloc(0x38);self.triggerptr.append(p);self.w(p,t['mode']);self.w(p+4,t['activation_bits']);self.w(p+8,t['id']);self.u.mem_write(p+0x34,bytes([t['enabled'],t['repeat']]))
   for offset,label in [(0x14,'events'),(0x1c,'conditions'),(0x24,'actions')]:
    ptrs=[]
    for e in t[label]:
     q=self.alloc(20);self.leaves[q]=(ti,e);ptrs.append(q)
    self.w(p+offset,self.link(ptrs,'element'))
  self.w(MANAGER+4,self.link(self.triggerptr,'trigger'))
  for subject in reversed(c['subjects']):self.add_subject(subject)
  for event in reversed(c['deferred']):self.add_event(event)
 def alloc(self,n):p=self.bump;self.bump+=(n+15)&~15;return p
 def w(self,a,v):self.u.mem_write(a,struct.pack('<I',v&0xffffffff))
 def r(self,a):return struct.unpack('<I',self.u.mem_read(a,4))[0]
 def byte(self,a):return self.u.mem_read(a,1)[0]
 def link(self,values,kind):
  head=0
  for value in reversed(values):p=self.alloc(8);self.w(p,value);self.w(p+4,head);self.nodes[p]=(kind,value);head=p
  return head
 def list_values(self,head):
  rows=[];seen=set()
  while head:
   assert head not in seen;seen.add(head);rows.append(self.r(head));head=self.r(head+4)
  return rows
 def add_subject(self,value):
  if value in self.list_values(self.r(MANAGER+0x14)):return
  p=self.alloc(8);self.w(p,value);self.w(p+4,self.r(MANAGER+0x14));self.nodes[p]=('subject',value);self.w(MANAGER+0x14,p)
  if self.r(MANAGER+0x18)==0:self.w(MANAGER+0x18,p)
  self.w(MANAGER+0x1c,MANAGER+0x14);self.w(MANAGER+0x20,p)
 def add_event(self,event):
  if event is None:q=0
  else:q=self.alloc(0x20);self.nodes[q]=('payload',event);self.w(q,event)
  p=self.alloc(8);self.w(p,q);self.w(p+4,self.r(MANAGER+0x50));self.nodes[p]=('event',event);self.w(MANAGER+0x50,p)
  if self.r(MANAGER+0x54)==0:self.w(MANAGER+0x54,p)
 def ret(self,pop=0,value=0):
  sp=self.u.reg_read(UC_X86_REG_ESP);self.u.reg_write(UC_X86_REG_EIP,self.r(sp));self.u.reg_write(UC_X86_REG_ESP,sp+4+pop);self.u.reg_write(UC_X86_REG_EAX,value)
 def mutate(self,m):
  if 'trigger' in m:
   p=self.triggerptr[m['trigger']]
   for key,offset in [('mode',0),('activation_bits',4)]:
    if key in m:self.w(p+offset,m[key])
   for key,offset in [('enabled',0x34),('repeat',0x35)]:
    if key in m:self.u.mem_write(p+offset,bytes([m[key]]))
  if 'subject' in m:self.add_subject(m['subject'])
  if 'event' in m:self.add_event(m['event'])
  if 'context_word_10' in m:self.w(MANAGER+0x10,m['context_word_10'])
  if 'time_bits' in m:self.w(WORLD+0xe0,m['time_bits'])
 def hook(self,u,a,n,_):
  if 0x400000<=a<0xc00000:self.visited[a]=n
  if a==0x5411d0:
   self.current=self.triggerptr.index(u.reg_read(UC_X86_REG_ECX));self.evaluated.append(self.c['triggers'][self.current]['id'])
  elif a==0x54c2a1:
   if u.reg_read(UC_X86_REG_EAX)&255:self.fired.append(self.c['triggers'][self.current]['id'])
  elif a==0x546af0:
   ti,e=self.leaves[u.reg_read(UC_X86_REG_ECX)];cursor=self.r(MANAGER+0x20);self.calls.append({'trigger':ti,'element':e['id'],'subject':self.r(cursor) if cursor else None,'context_word_10':self.r(MANAGER+0x10)})
   k=(ti,e['id']);step=self.steps.get(k,0);self.steps[k]=step+1
   instructions=e.get('mutations',[])
   for mutation in instructions:
    if mutation.get('call',0)==step:self.mutate(mutation)
   values=e['returns'];self.ret(4,int(values[min(step,len(values)-1)]))
  elif a==0x76ac90:self.ret(value=self.c['application_mode'])
  elif a==0x5412cb:
   ti,e=self.leaves[u.reg_read(UC_X86_REG_ESI)];self.diagnostics.append({'trigger':ti,'element':e['id']});u.reg_write(UC_X86_REG_EIP,0x54139e)
  elif a==0x6f7430:
   p=self.r(u.reg_read(UC_X86_REG_ESP)+4);kind,value=self.nodes[p]
   self.releases.append({ {'subject':'SubjectNode','event':'EventNode','payload':'EventPayload'}[kind]:value})
   self.ret()
 def run(self):
  self.w(STACK,STOP);self.u.reg_write(UC_X86_REG_ESP,STACK);self.u.reg_write(UC_X86_REG_ECX,MANAGER)
  regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
  for i,r in enumerate(regs):self.u.reg_write(r,0x11111111*(i+1))
  self.u.emu_start(0x54c280,STOP,count=100000);assert self.u.reg_read(UC_X86_REG_EIP)==STOP and self.u.reg_read(UC_X86_REG_ESP)==STACK+4
  assert [self.u.reg_read(r) for r in regs]==[0x11111111*(i+1) for i in range(4)]
  state=[]
  for p in self.triggerptr:state.append({'id':self.r(p+8),'mode':self.r(p),'enabled':self.byte(p+0x34),'repeat':self.byte(p+0x35),'activation_bits':self.r(p+4)})
  return {'triggers':state,'calls':self.calls,'diagnostics':self.diagnostics,'trace':{'evaluated':self.evaluated,'fired':self.fired,'releases':self.releases},'subjects':self.list_values(self.r(MANAGER+0x14)),'context_word_10':self.r(MANAGER+0x10),'cursor_bound_to_subject_list':bool(self.r(MANAGER+0x1c)),'cursor':None if self.r(MANAGER+0x20)==0 else self.r(self.r(MANAGER+0x20)),'deferred':[self.nodes[p][1] if p else None for p in self.list_values(self.r(MANAGER+0x50))],'time_bits':self.r(WORLD+0xe0),'abi_preserved':True}
def cases():
 def leaf(id,value=True,mutations=None):return {'id':id,'returns':[value] if isinstance(value,bool) else value,'mutations':mutations or []}
 def trigger(id=1,events=None,conditions=None,actions=None,mode=1,enabled=1,repeat=0):return {'id':id,'mode':mode,'enabled':enabled,'repeat':repeat,'activation_bits':bits(1),'events':events or [],'conditions':conditions or [],'actions':actions or []}
 base={'time_bits':bits(10),'application_mode':6,'context_word_10':17,'subjects':[],'deferred':[]};rows=[]
 def add(name,triggers,**kwargs):rows.append({'name':name,**copy.deepcopy(base),'triggers':triggers,**kwargs})
 add('empty',[]);add('disabled',[trigger(enabled=0)],subjects=[30,20],deferred=[1,None,2]);add('unconditional',[trigger(actions=[leaf(1)])])
 for mode in [0,1,2]:
  for e1,e2,c1,c2 in itertools.product([False,True],repeat=4):add(f'bool_{mode}_{e1}_{e2}_{c1}_{c2}',[trigger(events=[leaf(1,e1),leaf(2,e2)],conditions=[leaf(3,c1),leaf(4,c2)],actions=[leaf(5)],mode=mode)])
 add('actions_false_continue',[trigger(actions=[leaf(1,False),leaf(2)])]);add('action_editor_diagnostic',[trigger(actions=[leaf(1,False),leaf(2,False)])],application_mode=7)
 add('repeat_subjects',[trigger(actions=[leaf(1)],repeat=1)],subjects=[30,20,10]);add('single_subject_stop',[trigger(actions=[leaf(1)])],subjects=[30,20,10])
 add('condition_subject_loop',[trigger(conditions=[leaf(1,[False,True])],actions=[leaf(2)])],subjects=[30,20,10])
 add('event_add_dedup_subjects',[trigger(events=[leaf(1,True,[{'subject':20},{'subject':30},{'subject':20}])],actions=[leaf(2)],repeat=1)])
 add('action_add_subject',[trigger(actions=[leaf(1,True,[{'subject':40}])],repeat=1)],subjects=[30,20])
 add('condition_changes_mode',[trigger(conditions=[leaf(1,True,[{'trigger':0,'mode':0}]),leaf(2,False)],actions=[leaf(3)])])
 add('action_changes_repeat_time',[trigger(actions=[leaf(1,True,[{'trigger':0,'repeat':1},{'time_bits':bits(12.5)}])])],subjects=[3,2])
 add('action_stops_repeat',[trigger(actions=[leaf(1,True,[{'trigger':0,'repeat':0}])],repeat=1)],subjects=[3,2])
 add('enable_later',[trigger(actions=[leaf(1,True,[{'trigger':1,'enabled':1}])]),trigger(id=2,enabled=0,actions=[leaf(2)])])
 add('disable_later',[trigger(actions=[leaf(1,True,[{'trigger':1,'enabled':0}])]),trigger(id=2,actions=[leaf(2)])])
 add('queue_from_action',[trigger(actions=[leaf(1,True,[{'event':3},{'event':4},{'context_word_10':99}])])],deferred=[1,2])
 add('null_drains_one_then_stops',[trigger()],deferred=[1,None,2]);add('head_null_stops',[],deferred=[None,2]);add('no_null_drains',[],deferred=[1,2,3])
 return rows
def main():
 rows=[];visited={}
 for c in cases():
  m=Machine(c);rows.append({'input':c,'output':m.run()});visited.update(m.visited)
 windows=[]
 for a,n in sorted(visited.items()):
  raw=IMAGE.at(a,n);assert raw==ACCEPTED.at(a,n);windows.append({'address':hex(a),'bytes':raw.hex()})
 doc={'schema':1,'original_sha256':pe.ORIGINAL_SHA,'v14_sha256':pe.ACCEPTED_SHA,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'cases':rows,'visited_windows':windows,'scope':'Actual54C280/5411D0/54BDC0/4DB300 ordering. Immutable trigger/element lists during callbacks. No native546AF0 opcode implementation claimed.','seams':['546AF0 bool/mutation leaf scripts','application mode getter','mode7 localized failure reporting branch, captured and skipped','heap free observation'],'floating_point_scope':'No floating arithmetic in these routines; world time is copied as raw binary32.'}
 path=HERE/'native-manager-fixtures.json';path.write_text(json.dumps(doc,indent=2)+'\n');print(json.dumps({'cases':len(rows),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
