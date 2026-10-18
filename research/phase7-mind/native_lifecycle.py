"""Original Mind activation/stack lifecycle with required virtual state leaves."""
from pathlib import Path
import copy,hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine,original,modified
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
MIND,STATE,META,VT,SVT,UPVT=0x4000000,0x4100000,0x4200000,0x4300000,0x4301000,0x4302000
CAN,ENTER,LEAVE,SCHEDULE,DEFAULT,CHANGED=range(0x4700000,0x4700060,0x10)
mp=lambda i:MIND+i*0x1000
sp=lambda i:0 if i is None else STATE+i*0x1000
sid=lambda p:None if p==0 else (p-STATE)//0x1000
sha=lambda b:hashlib.sha256(b).hexdigest()

class LifecycleMachine(Machine):
 def __init__(self,c):
  super().__init__();self.c=copy.deepcopy(c);self.u.mem_map(0,4096);self.u.mem_map(0x4000000,0x800000);self.heap=0x4400000;self.events=[];self.visited={}
  self.w(VT+0x1a0,DEFAULT);self.w(VT+0x1a4,CHANGED);self.w(SVT+4,CAN);self.w(SVT+0x18,ENTER);self.w(SVT+0x1c,LEAVE);self.w(UPVT+4,SCHEDULE)
  for i in [1,2]:self.w(mp(i),VT);self.w(mp(i)+4,100+i);self.w(mp(i)+8,UPVT)
  self.write_arena();self.u.hook_add(UC_HOOK_CODE,self.hook)
 def alloc(self,n):p=self.heap;self.heap+=(n+15)&~15;return p
 def write_arena(self):
  for key,state in self.c['arena']['states'].items():
   i=int(key);p=sp(i);self.w(p,SVT);self.w(p+4,mp(state['parent']));self.w(p+8,META+i*16);self.w(p+12,state['status']);self.u.mem_write(META+i*16+8,bytes([state['discard_on_suspend']]))
  for key,mind in self.c['arena']['minds'].items():
   p=mp(int(key));head=tail=0
   for ident in reversed(mind['stack']):
    node=self.alloc(8);self.w(node,sp(ident));self.w(node+4,head);head=node
    if not tail:tail=node
   self.w(p+0x10,head);self.w(p+0x14,tail)
 def snapshot(self):
  arena=copy.deepcopy(self.c['arena'])
  for key,state in arena['states'].items():
   p=sp(int(key));state['status']=self.r(p+12);state['parent']=(self.r(p+4)-MIND)//0x1000;state['discard_on_suspend']=bool(self.u.mem_read(self.r(p+8)+8,1)[0])
  for key,mind in arena['minds'].items():
   values=[];node=self.r(mp(int(key))+0x10)
   while node:values.append(sid(self.r(node)));node=self.r(node+4);assert len(values)<100
   mind['stack']=values
  return arena
 def ret(self,value=0,pop=0):
  p=self.u.reg_read(UC_X86_REG_ESP);self.u.reg_write(UC_X86_REG_EAX,value);self.u.reg_write(UC_X86_REG_EIP,self.r(p));self.u.reg_write(UC_X86_REG_ESP,p+4+pop)
 def event(self,name,ident,extra=[]):
  self.c['arena']=self.snapshot();self.events.append([name,ident,*extra,copy.deepcopy(self.c['arena'])])
  mutations=self.c['mutations']
  if mutations and mutations[0]['event']==name:
   change=mutations.pop(0)
   for key,value in change.get('states',{}).items():self.c['arena']['states'][key].update(value)
   for key,value in change.get('stacks',{}).items():self.c['arena']['minds'][key]['stack']=value
   self.write_arena()
 def hook(self,u,pc,size,_):
  if original.base<=pc<original.base+original.image_size:self.visited[pc]=bytes(u.mem_read(pc,size))
  cx=u.reg_read(UC_X86_REG_ECX);stack=u.reg_read(UC_X86_REG_ESP)
  if pc==0x6f7810:self.ret(self.alloc(self.r(stack+4)))
  elif pc==0x6f7430:self.ret()
  elif pc==CAN:self.event('can',sid(cx));self.ret(int(self.c['can_activate']))
  elif pc==ENTER:self.event('enter',sid(cx));self.ret()
  elif pc==LEAVE:self.event('leave',sid(cx));self.ret()
  elif pc==SCHEDULE:self.event('schedule',(cx-8-MIND)//0x1000,[self.r(stack+4),self.r(stack+8)]);self.ret(pop=8)
  elif pc==DEFAULT:self.event('default',(cx-MIND)//0x1000);self.ret()
  elif pc==CHANGED:self.event('changed',(cx-MIND)//0x1000);self.ret()
 def run(self):
  self.reset(0);op=self.c['operation'];regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
  for i,r in enumerate(regs):self.u.reg_write(r,0x11111111*(i+1))
  self.u.reg_write(UC_X86_REG_ECX,sp(4) if op=='activate' else mp(1));self.w(self.STACK+4,self.c['mode'] if op=='activate' else sp(4))
  address={'activate':0x52a810,'push':0x52a290,'replace_all':0x52a420,'replace_top':0x52a470,'clear':0x52a3c0}[op]
  self.u.emu_start(address,self.STOP,count=20000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP
  assert self.u.reg_read(UC_X86_REG_ESP)==self.STACK+(4 if op=='clear' else 8)
  assert [self.u.reg_read(r)for r in regs]==[0x11111111*(i+1)for i in range(4)]
  assert not self.c['mutations']
  return {'accepted':bool(self.u.reg_read(UC_X86_REG_EAX)&255) if op=='activate' else None,'arena':self.snapshot(),'events':self.events}

def cases():
 mind={'actor':101,'stack':[1,2,3],'pending_commands':[],'current_command':None,'words_24_30':[0]*4}
 base={'name':'base','operation':'activate','mode':3,'can_activate':True,'arena':{'minds':{'1':mind,'2':{**mind,'actor':102,'stack':[]}},'states':{str(i):{'parent':1,'discard_on_suspend':False,'status':10+i}for i in range(1,6)}},'mutations':[]}
 rows=[]
 def add(name,change):
  c=copy.deepcopy(base);c['name']=name;change(c);rows.append(c)
 for mode in [0,1,2,3,4,0xffffffff]:
  for allow in [False,True]:
   for stack in [[],[None],[1],[1,2,3]]:add(f'mode{mode}_allow{allow}_stack{stack}',lambda c,m=mode,a=allow,s=stack:(c.update(mode=m,can_activate=a),c['arena']['minds']['1'].update(stack=s)))
 for operation in ['push','replace_all','replace_top','clear']:
  for discard in [False,True]:
   add(f'{operation}_discard{discard}',lambda c,op=operation,d=discard:(c.update(operation=op),c['arena']['states']['1'].update(discard_on_suspend=d),c['arena']['states']['2'].update(discard_on_suspend=d)))
 for operation in ['push','replace_all','replace_top','clear']:
  for stack in [[],[3,2],[None,2]]:
   add(f'{operation}_leave_replaces_{stack}',lambda c,op=operation,s=stack:c.update(operation=op,mutations=[{'event':'leave','stacks':{'1':s}}]))
 add('eligibility_changes_parent',lambda c:c.update(mutations=[{'event':'can','states':{'4':{'parent':2}}}]))
 add('eligibility_changes_stack',lambda c:c.update(mutations=[{'event':'can','stacks':{'1':[2,3]}}]))
 add('schedule_changes_head',lambda c:c.update(mutations=[{'event':'schedule','stacks':{'1':[5,3]}}]))
 add('enter_changes_stack',lambda c:c.update(mutations=[{'event':'enter','stacks':{'1':[2]}}]))
 add('leave_marks_discard',lambda c:c.update(mutations=[{'event':'leave','states':{'1':{'discard_on_suspend':True}}}]))
 add('default_installs_state',lambda c:c.update(operation='clear',mutations=[{'event':'default','stacks':{'1':[5]}}]))
 return rows

def main():
 rows=[];visited={}
 for c in cases():
  m=LifecycleMachine(c);rows.append({'input':c,'output':m.run()});visited.update(m.visited)
 for a,raw in visited.items():assert raw==original.range(a,len(raw))==modified.range(a,len(raw))
 doc={'schema':1,'original_sha256':sha(original.b),'v14_sha256':sha(modified.b),'producer_sha256':sha(Path(__file__).read_bytes()),'method':'Original52A810/52A290/52A420/52A470/52A3C0 execute with state entry/exit, eligibility, scheduling, default reset and changed callbacks supplied. Allocation/free are memory-only seams. Callback mutations alter actual state graph before native resumes.','instructions':[{'va':hex(a),'bytes':raw.hex()}for a,raw in sorted(visited.items())],'cases':rows}
 path=HERE/'native-lifecycle.json';path.write_text(json.dumps(doc,indent=2),encoding='utf-8');print(json.dumps({'cases':len(rows),'sha256':sha(path.read_bytes())}))
if __name__=='__main__':main()
