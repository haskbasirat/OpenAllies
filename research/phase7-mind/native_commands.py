"""Execute original Mind command dispatch/queue/process/drain with real refcount instructions.

Subtype handlers, scheduler, state deferral, group notification and actor fallback
are explicit required leaves. Original command copy4D9CE0 executes (parameter
object absent in this scope); destructors/allocators are ownership-only seams.
"""
from pathlib import Path
import copy,hashlib,json,sys
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE))
from native_lifecycle import LifecycleMachine,cases as lifecycle_cases,mp,sp,sid,VT,SVT,UPVT,SCHEDULE
from native_constructors import original,modified
from unicorn.x86_const import *
COMMAND,DEFINITION,ACTOR=0x4500000,0x4600000,0x4650000
CVT,DTOR,DEFERS=0x4303000,0x4700200,0x4700300
HANDLER=0x4710000
SLOTS=[0xc0,0xc4,0xc8,0xd0,0xcc,0xbc,0xd4,0xd8,0xdc,0xe0,0xe4,0xe8,0xec]
sha=lambda b:hashlib.sha256(b).hexdigest()
class CommandMachine(LifecycleMachine):
 def __init__(self,c):
  super().__init__(c);self.commands={int(k):COMMAND+int(k)*0x100 for k in c['commands']};self.next_id=100;self.original_dtor=self.r(0xa28450);self.handled=0
  self.w(CVT,DTOR);self.w(SVT+8,DEFERS);self.w(mp(1)+4,ACTOR);self.w(ACTOR+8,c['actor_flags']);self.w(ACTOR+0x14,123)
  for slot in SLOTS:self.w(VT+slot,HANDLER+slot*16)
  for key,value in c['commands'].items():
   ident=int(key);p=self.commands[ident];definition=DEFINITION+ident*0x100
   self.w(p,CVT);self.w(p+8,definition);self.w(definition+0x18,value['component_slot']);self.w(definition+0xc,value['mind_type']);self.w(definition+0x2c,value['behavior_flags']);self.u.mem_write(p+0x3c,bytes([value['trailing']]))
   self.w(p+0xc,0x40);self.w(p+0x18,0x3f800001);self.w(p+0x1c,0xc0200000);self.w(p+0x20,0x3e800000);self.w(p+0x24,0xbf800000);self.w(p+0x28,0x40000000)
   refs=value['external']+int(c['arena']['minds']['1']['current_command']==ident)+c['arena']['minds']['1']['pending_commands'].count(ident)
   self.w(p+4,refs)
  self.w(mp(1)+0x20,self.pointer(c['arena']['minds']['1']['current_command']));self.write_pending(c['arena']['minds']['1']['pending_commands'])
  self.reset(0);self.u.emu_start(0x9cf5d0,self.STOP,count=1000)
  for i,slot in enumerate(SLOTS):assert self.r(0xb3d178+i*16+4)==0
  self.w(0xb3d170,c['depth'])
 def pointer(self,ident):return 0 if ident is None or ident==0 else self.commands[ident]
 def identity(self,p):return None if p==0 else next(k for k,v in self.commands.items() if v==p)
 def write_pending(self,values):
  head=tail=0
  for ident in reversed(values):
   node=self.alloc(8);self.w(node,self.pointer(ident));self.w(node+4,head);head=node
   if not tail:tail=node
  self.w(mp(1)+0x18,head);self.w(mp(1)+0x1c,tail)
 def state(self):
  result={'current':self.identity(self.r(mp(1)+0x20)),'pending':[],'stack':[],'refs':{str(k):self.r(v+4)for k,v in self.commands.items()},'depth':self.r(0xb3d170)}
  node=self.r(mp(1)+0x18)
  while node:result['pending'].append(self.identity(self.r(node)));node=self.r(node+4)
  node=self.r(mp(1)+0x10)
  while node:result['stack'].append(sid(self.r(node)));node=self.r(node+4)
  return result
 def drop(self,p):
  self.w(p+4,self.r(p+4)-1)
  if not self.r(p+4):self.events.append(['destroy',self.identity(p)])
 def change_current(self,ident):
  old=self.r(mp(1)+0x20);new=self.pointer(ident)
  if old!=new:
   if new:self.w(new+4,self.r(new+4)+1)
   if old:self.drop(old)
   self.w(mp(1)+0x20,new)
 def event_command(self,name,args):
  self.events.append([name,*args,self.state()]);changes=self.c['mutations']
  if changes and changes[0]['event']==name:
   m=changes.pop(0)
   if 'current' in m:self.change_current(m['current'])
   if 'stack' in m:
    self.c['arena']['minds']['1']['stack']=m['stack'];self.write_arena()
   if 'append' in m:
    pending=self.state()['pending']
    for ident in m['append']:p=self.pointer(ident);self.w(p+4,self.r(p+4)+1)
    self.write_pending(pending+m['append'])
   if 'original_type' in m:self.w(DEFINITION+0x100+0xc,m['original_type'])
 def hook(self,u,pc,size,data):
  if original.base<=pc<original.base+original.image_size:self.visited[pc]=bytes(u.mem_read(pc,size))
  cx=u.reg_read(UC_X86_REG_ECX);s=u.reg_read(UC_X86_REG_ESP)
  if pc in [DTOR,getattr(self,'original_dtor',0)]:self.events.append(['destroy',self.identity(cx)]);self.ret(pop=4)
  elif HANDLER<=pc<HANDLER+0x1000:
   slot=(pc-HANDLER)//16;ident=self.identity(self.r(s+4));self.event_command('handler',[slot,ident]);answers=self.c['accepted'];answer=answers[min(self.handled,len(answers)-1)];self.handled+=1;self.ret(int(answer),4)
  elif pc==SCHEDULE:self.event_command('schedule',[self.r(s+4),self.r(s+8)]);self.ret(pop=8)
  elif pc==DEFERS:self.event_command('defers',[sid(cx)]);self.ret(int(self.c['defers']))
  elif pc==0x4d9ce0:
   ident=self.next_id;self.next_id+=1;self.commands[ident]=cx;self.events.append(['clone',self.identity(self.r(s+4)),ident])
  elif pc==0x77dc50:self.ret(pop=8)
  elif pc in [0x4d9a40,0x404f60]:self.ret()
  elif pc==0x4d9b60:self.event_command('stop',[101]);self.ret(int(self.c['stop_result']),4)
  elif pc==0x4d9e20:
   # Fallback type6 stack command construction belongs to the actor dispatcher.
   self.w(cx+8,DEFINITION+0x100);self.ret(cx,4)
  elif pc==0x4ca2d0:self.ret()
  elif pc==0x44dea0:self.ret(int(self.c['group']))
  elif pc==0x4d1900:self.ret(0x4660000)
  elif pc==0x4de7a0:self.event_command('group',[101]);self.ret()
  else:super().hook(u,pc,size,data)
 def run_command(self):
  self.reset(0);self.u.reg_write(UC_X86_REG_ECX,mp(1));self.w(0x4670000,self.pointer(1));self.w(self.STACK+4,self.pointer(1) if self.c['operation']=='process' else 0x4670000)
  regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
  for i,r in enumerate(regs):self.u.reg_write(r,0x11111111*(i+1))
  entry={'dispatch':0x52a060,'enqueue':0x52c070,'clear':0x52c0d0,'discard':0x52a8d0,'process':0x52c320,'resume':0x52bf30}[self.c['operation']]
  self.u.emu_start(entry,self.STOP,count=50000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP
  assert self.u.reg_read(UC_X86_REG_ESP)==self.STACK+(8 if self.c['operation'] in ['dispatch','enqueue','process'] else 4)
  assert [self.u.reg_read(r)for r in regs]==[0x11111111*(i+1)for i in range(4)]
  assert not self.c['mutations'],self.c['name']
  return {'accepted':bool(self.u.reg_read(UC_X86_REG_EAX)&255)if self.c['operation']in['dispatch','discard','process','resume']else None,'state':self.state(),'events':self.events}

def cases():
 arena=copy.deepcopy(lifecycle_cases()[0]['arena']);arena['minds']['1'].update(current_command=2,pending_commands=[3,4],stack=[1,2,3])
 base={'name':'base','operation':'dispatch','arena':arena,'commands':{str(i):{'component_slot':4,'mind_type':5,'behavior_flags':16,'trailing':False,'external':1}for i in range(1,6)},'accepted':[True],'depth':5,'defers':False,'stop_result':True,'actor_flags':0,'group':False,'mutations':[]}
 rows=[]
 def add(name,change):c=copy.deepcopy(base);c['name']=name;change(c);rows.append(c)
 for kind in range(13):
  for accepted in [False,True]:add(f'dispatch_type{kind}_{accepted}',lambda c,k=kind,a=accepted:(c['commands']['1'].update(mind_type=k),c.update(accepted=[a])))
 for current in [None,1,2]:
  for accepted in [False,True]:
   for replacement in [None,1,2,5]:add(f'handler_replaces_{current}_{accepted}_{replacement}',lambda c,p=current,a=accepted,r=replacement:(c['arena']['minds']['1'].update(current_command=p),c.update(accepted=[a],mutations=[{'event':'handler','current':r}])))
 for slot in [0,3,5,31]:add(f'foreign_component{slot}',lambda c,s=slot:c['commands']['1'].update(component_slot=s))
 add('depth_wrap',lambda c:c.update(depth=0xffffffff))
 for operation in ['enqueue','clear','discard','resume']:
  for queue in [[],[1],[3,4,3]]:
   for current in [None,2]:add(f'{operation}_{queue}_{current}',lambda c,o=operation,q=queue,r=current:(c.update(operation=o),c['arena']['minds']['1'].update(pending_commands=q,current_command=r)))
 for flags in [0,15,16,0xffffffff]:
  for trailing in [False,True]:
   for accepted in [False,True]:
    for defers in [False,True]:add(f'process_{flags}_{trailing}_{accepted}_{defers}',lambda c,f=flags,t=trailing,a=accepted,d=defers:(c.update(operation='process',accepted=[a],defers=d),c['commands']['1'].update(behavior_flags=f,trailing=t)))
 for operation in ['clear','discard','process','resume']:
  add(f'{operation}_destroys_owned_only',lambda c,o=operation:(c.update(operation=o,accepted=[False]),[v.update(external=0)for k,v in c['commands'].items()if k in['2','3','4']]))
 for stack in [[],[None],[1],[1,2,3]]:
  add(f'process_reject_stack{stack}',lambda c,s=stack:(c.update(operation='process',accepted=[False],defers=True),c['arena']['minds']['1'].update(stack=s)))
 for answers in [[False],[False,True],[False,False,True]]:
  for group in [False,True]:add(f'resume_answers{answers}_group{group}',lambda c,a=answers,g=group:(c.update(operation='resume',accepted=a,group=g),c['arena']['minds']['1'].update(pending_commands=[3,4,1])))
 add('resume_actor_flag6',lambda c:c.update(operation='resume',actor_flags=64))
 add('resume_handler_appends',lambda c:c.update(operation='resume',accepted=[False,False,True],mutations=[{'event':'handler','append':[1]}]))
 add('process_handler_changes_original_type',lambda c:c.update(operation='process',accepted=[False],mutations=[{'event':'handler','original_type':6}]))
 add('process_defers_replaces_head',lambda c:c.update(operation='process',accepted=[False],defers=True,mutations=[{'event':'defers','stack':[3,2]}]))
 add('process_stop_rejected',lambda c:c.update(operation='process',accepted=[False],stop_result=False))
 return rows

def main():
 rows=[];visited={}
 for c in cases():
  try:m=CommandMachine(c);result=m.run_command()
  except Exception:print(c['name']);raise
  rows.append({'input':c,'output':result});visited.update(m.visited)
 for a,b in visited.items():assert b==original.range(a,len(b))==modified.range(a,len(b)),hex(a)
 doc={'schema':1,'original_sha256':sha(original.b),'v14_sha256':sha(modified.b),'producer_sha256':sha(Path(__file__).read_bytes()),'method':__doc__,'instructions':[{'va':hex(a),'bytes':b.hex()}for a,b in sorted(visited.items())],'cases':rows}
 p=HERE/'native-commands.json';p.write_text(json.dumps(doc,indent=2),encoding='utf-8');print(json.dumps({'cases':len(rows),'sha256':sha(p.read_bytes())}))
if __name__=='__main__':main()
