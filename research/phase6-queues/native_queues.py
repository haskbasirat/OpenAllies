"""Execute unchanged original queue operations with required callback seams.

No live native process, original mutation, or gameplay scheduling inference.
"""
from pathlib import Path
import hashlib,json,struct,sys,random
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/engine-baseline'))
from audit_baseline import PE
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE
from unicorn.x86_const import *
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
ORIGINAL=PE(ROOT.parent/'AA.exe')
ACCEPTED=PE(ROOT.parent/'reconstruction/native-game/runtime-expanded-v14/AA.exe')
assert hashlib.sha256(ORIGINAL.b).hexdigest()=='5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26'
assert hashlib.sha256(ACCEPTED.b).hexdigest()=='3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1'
VISITED={}
def bits(f):return struct.unpack('<I',struct.pack('<f',f))[0]

class Machine:
 def __init__(self,case):
  self.case=case;self.u=Uc(UC_ARCH_X86,UC_MODE_32)
  self.u.mem_map(0,0x1000)
  self.u.mem_map(ORIGINAL.base,(ORIGINAL.image_size+4095)&~4095)
  for vs,r,size,off in ORIGINAL.sections:self.u.mem_write(ORIGINAL.base+r,ORIGINAL.b[off:off+size])
  self.u.mem_map(0x2000000,0x400000);self.u.mem_map(0x3000000,0x20000)
  self.world=0x2000000;self.next=0x2010000;self.stack=0x3008000;self.stop=0x300f000
  self.events=[];self.callback_actions={int(k):list(v) for k,v in case.get('callback_actions',{}).items()}
  self.write(0xb36534,self.world);self.write(self.world+0xd4,case['seed'])
  self.owners={};self.names={};self.current_queue=None
  self.owner_vtable=self.alloc(0x30)
  for off in (8,12,16): self.write(self.owner_vtable+off,0x23f0000)
  self.write(self.owner_vtable+20,0x23f0010)
  self.queues=[]
  for i,count in enumerate(case['counts']):
   q=self.world+[0x1ac,0x1c0,0x1d4,0x1e8,0x208][i];self.queues.append(q)
   self.write(q,0xa30be0 if i<3 else 0xa30c24);self.write(q+4,i)
   heads=self.alloc(count*4)
   if i<3:self.write(q+12,count);self.write(q+16,heads)
   else:
    self.write(q+12,heads);self.write(q+16,count);self.write(q+20,count);self.write(q+24,0x23f0020)
    for k in range(count):self.write(heads+4*k,self.alloc(4))
  self.clock(case['clock'])
  self.u.reg_write(UC_X86_REG_FPCW,{24:0x7f,53:0x27f,64:0x37f}[case['precision']])
  self.u.hook_add(UC_HOOK_CODE,self.hook)
 def alloc(self,size):
  p=self.next;self.next+=(size+15)&~15
  assert self.next<0x23e0000
  self.u.mem_write(p,b'\0'*size);return p
 def write(self,a,v):self.u.mem_write(a,struct.pack('<I',v&0xffffffff))
 def read(self,a):return struct.unpack('<I',self.u.mem_read(a,4))[0]
 def arg(self,i=0):return self.read(self.u.reg_read(UC_X86_REG_ESP)+4+4*i)
 def ret(self,n=0,value=0):
  sp=self.u.reg_read(UC_X86_REG_ESP)
  self.u.reg_write(UC_X86_REG_EAX,value);self.u.reg_write(UC_X86_REG_EIP,self.read(sp));self.u.reg_write(UC_X86_REG_ESP,sp+4+n)
 def owner(self,id):
  if id not in self.owners:
   p=self.alloc(0x20);self.write(p,self.owner_vtable);self.owners[id]=p;self.names[p]=id
  return self.owners[id]
 def clock(self,clock):
  for off,name in ((0xe0,'begin_bits'),(0xe4,'end_bits'),(0xe8,'step_bits')):self.write(self.world+off,clock[name])
 def hook(self,u,pc,size,_):
  if ORIGINAL.base<=pc<ORIGINAL.base+ORIGINAL.image_size:
   try:
    raw=ORIGINAL.range(pc,size);assert raw==ACCEPTED.range(pc,size),hex(pc);VISITED[pc]=raw.hex()
   except ValueError:pass
  ecx=u.reg_read(UC_X86_REG_ECX)
  if pc==0x6f7810:self.ret(value=self.alloc(self.arg()));return
  if pc==0x6f7430:self.ret();return
  if pc==0x77dc50:self.ret(8);return
  if pc==0x77da70:self.ret(12);return
  if pc==0x404f90:self.ret();return #native assertion logger; following cap still executes
  if pc==0x23f0000:self.ret(value=1);return
  if pc in (0x23f0010,0x23f0020):
   periodic=pc==0x23f0010
   event={'queue':self.current_queue,'owner':self.names[ecx], 'data':self.arg(1 if periodic else 0),
          'elapsed_bits':self.arg() if periodic else None}
   self.events.append(event)
   actions=self.callback_actions.pop(event['data'],[])
   #Run nested native actions with separate stack; retain the callback frame.
   registers=[UC_X86_REG_EAX,UC_X86_REG_EBX,UC_X86_REG_ECX,UC_X86_REG_EDX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP,UC_X86_REG_ESP,UC_X86_REG_EIP,UC_X86_REG_EFLAGS]
   saved={r:u.reg_read(r) for r in registers}
   for action in actions:self.action(action,nested=True)
   for r,v in saved.items():u.reg_write(r,v)
   self.ret(8 if periodic else 4);return
 def call(self,va,ecx,args=(),nested=False):
  sp=self.stack-(0x2000 if nested else 0)
  self.write(sp,self.stop)
  for i,value in enumerate(args):self.write(sp+4+4*i,value)
  self.u.reg_write(UC_X86_REG_ECX,ecx);self.u.reg_write(UC_X86_REG_ESP,sp)
  self.u.emu_start(va,self.stop,count=5000000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.stop,hex(self.u.reg_read(UC_X86_REG_EIP))
  assert self.u.reg_read(UC_X86_REG_ESP)==sp+4+4*len(args),(hex(va),hex(self.u.reg_read(UC_X86_REG_ESP)),hex(sp))
  return self.u.reg_read(UC_X86_REG_EAX)
 def action(self,op,nested=False):
  typ=op['type']
  if typ=='clock':self.clock(op['clock'])
  elif typ=='cancel':self.call(0x50b250,self.owner(op['owner']),nested=nested)
  elif typ=='update':
   self.current_queue=op['queue'];self.call(0x50b330 if op['queue']<3 else 0x50b020,self.queues[op['queue']],nested=nested)
  elif typ=='add':
   q=op['queue'];owner=self.owner(op['owner'])
   if q<3:
    #The third periodic queue has no identical public owner wrapper in this
    #window. Execute its real insertion then link owner list like50AC00.
    if q<2:self.call([0x50ac00,0x50ac60][q],owner,[op['data']],nested=nested)
    else:
     node=self.call(0x50b2c0,self.queues[q],[owner,op['data']],nested=nested)
     head=self.read(owner+4);self.write(node+12,owner+4);self.write(node+16,head)
     if head:self.write(head+12,node+16)
     self.write(owner+4,node)
   else:
    self.call(0x50acc0 if q==3 else 0x50ad80,owner,[op['delay_bits'],op['data']],nested=nested)
  elif typ=='at':
   assert op['queue']==3
   self.call(0x50ad20,self.owner(op['owner']),[op['when_bits'],op['data']],nested=nested)
  else:raise ValueError(op)
 def snapshot(self):
  snapshots=[]
  for id,q in enumerate(self.queues):
   count=self.read(q+(12 if id<3 else 16));heads=self.read(q+(16 if id<3 else 12));buckets=[]
   for i in range(count):
    p=self.read(heads+4*i)
    if id>=3:p=self.read(p)
    bucket=[]
    while p:
     bucket.append([self.names[self.read(p)],self.read(p+20)]);p=self.read(p+8)
     assert len(bucket)<10000
    buckets.append(bucket)
   snapshots.append({'id':id,'kind':'Periodic' if id<3 else 'Deferred','current':self.read(q+8),
                     'last_end_bits':0 if id<3 else self.read(q+28),'buckets':buckets})
  return snapshots

def add(q,owner,data,delay=0.0):return {'type':'add','queue':q,'owner':owner,'data':data,'delay_bits':bits(delay)}
def update(q):return {'type':'update','queue':q}
def cancel(owner):return {'type':'cancel','owner':owner}
def run():
 base={'counts':[1,3,7,3,4],'seed':0x12345678,'clock':{'begin_bits':0,'end_bits':bits(.0625),'step_bits':bits(.0625)}}
 specs=[]
 specs.append(('periodic_order',[add(0,i,i) for i in (1,2,3)]+[update(0),update(0)],{}))
 specs.append(('periodic_remove_current',[add(0,i,i) for i in (1,2,3,4)]+[update(0),update(0)],{4:[cancel(4)]}))
 specs.append(('periodic_remove_successor',[add(0,i,i) for i in (1,2,3)]+[update(0)],{3:[cancel(2)]}))
 specs.append(('periodic_prepend',[add(0,1,1),add(0,2,2),update(0),update(0)],{2:[add(0,3,3)]}))
 specs.append(('deferred_reentrant',[add(3,1,1),add(3,2,2),update(3)],{2:[add(3,3,3)]}))
 specs.append(('cross_queue_cancel',[add(0,1,10),add(1,1,11),add(3,1,12),add(4,1,13),cancel(1)]+[update(i) for i in range(5)],{}))
 specs.append(('deferred_remove_owner',[add(3,1,1),add(3,1,2),update(3)],{2:[cancel(1)]}))
 specs.append(('deferred_every_thousand',[add(3,(i%8)+1,i) for i in range(2005)]+[update(3)],{}))
 specs.append(('deferred_growth_wrapped',[update(3),update(3),add(3,1,1,.125),add(3,2,2,.5),add(3,3,3,1.0)]+[update(3) for _ in range(20)],{}))
 specs.append(('relative_rounding',[add(3,i+1,100+i,d) for i,d in enumerate((-1.0,-0.0,0.0,1e-7,.0624,.0625,.0626,.2,10.1,100.0))]+[update(3) for _ in range(163)],{}))
 specs.append(('absolute_origin',[{'type':'clock','clock':{'begin_bits':bits(1.0),'end_bits':bits(1.0625),'step_bits':bits(.0625)}},update(3),{'type':'at','queue':3,'owner':1,'data':1,'when_bits':bits(1.0625)},{'type':'at','queue':3,'owner':2,'data':2,'when_bits':bits(1.0)},update(3)],{}))
 rnd=random.Random(123)
 for n in range(12):
  ops=[]
  for i in range(50):
   q=rnd.randrange(5)
   if i%9==0:ops.append(cancel(rnd.randrange(1,7)))
   elif i%3==0:ops.append(update(q))
   else:ops.append(add(q,rnd.randrange(1,7),1000+n*100+i,rnd.choice([0,.031,.125,.5,1,2])))
  specs.append((f'mixed_{n}',ops,{}))
 cases=[]
 for pc in (24,53,64):
  for name,ops,actions in specs:
   case={**base,'name':f'{name}_pc{pc}','precision':pc,'operations':ops,'callback_actions':actions}
   m=Machine(case)
   for op in ops:m.action(op)
   case.update(expected=m.snapshot(),events=m.events,rng_state=m.read(m.world+0xd4));cases.append(case)
 periods=[]
 for pc in (24,53,64):
  for step in (.0625,.1,.3,1e-3,7.25):
   for period in (-1,0,.0001,.05,.1,.3,1,5,10,16.00001):
    m=Machine({**base,'precision':pc,'clock':{**base['clock'],'step_bits':bits(step)}})
    value=m.call(0x4f0720,m.world,[bits(period)])
    periods.append({'precision':pc,'step_bits':bits(step),'period_bits':bits(period),'ticks':value if value<0x80000000 else value-0x100000000})
 footprint=[{'va':hex(va),'bytes':raw} for va,raw in sorted(VISITED.items())]
 out={'schema':1,'cases':cases,'periods':periods,'visited_native_instructions_and_seam_entries':footprint,
      'scope':'Native queue insertion/cancellation/dispatch/time arithmetic; allocation, log and actor callbacks explicit seams. Third periodic owner linking reconstructed fixture setup. No world/admin or actor behavior claim.'}
 (HERE/'native-queues.json').write_text(json.dumps(out,indent=2)+'\n')
 print(json.dumps({'passed':True,'sequence_cases':len(cases),'period_cases':len(periods),'visited':len(footprint)}))
if __name__=='__main__':run()
