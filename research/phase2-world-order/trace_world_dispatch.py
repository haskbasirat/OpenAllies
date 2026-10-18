"""Execute native world dispatch/list mutation with explicit leaf tracer seams.

No live process, game launch, native patch or original-file mutation.
"""
from pathlib import Path
import hashlib, json, struct, sys

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
GAME=ROOT.parent
sys.path.insert(0,str(ROOT/'research/engine-baseline'))
from audit_baseline import PE
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE
from unicorn.x86_const import *

SOURCE=GAME/'AA.exe'
NATIVE=PE(SOURCE)
assert hashlib.sha256(NATIVE.b).hexdigest()=='5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26'
ACCEPTED=PE(GAME/'reconstruction/native-game/runtime-expanded-v14/AA.exe')
RANGES={
 'world_update':(0x4f9970,0x4f9c06),'phase_transition':(0x4f65b0,0x4f675b),
 'remove_node':(0x50c510,0x50c58b),'remove_value':(0x4f1430,0x4f14a5),
 'registry_unlink':(0x50c1b0,0x50c1bf),'availability_leaf':(0x4f4c40,0x4f4d79),
 'world_constructor':(0x4f7d00,0x4f8150),'world_init':(0x4f27e0,0x4f2db0),
 'periodic_dispatch':(0x50b330,0x50b39e),'deferred_dispatch':(0x50b020,0x50b117),
 'periodic_constructor':(0x50aba0,0x50abb9),'deferred_constructor':(0x50b3a0,0x50b3e0),
 'periodic_initialize':(0x50b580,0x50b690),'deferred_initialize':(0x50b690,0x50b790),
 'deferred_slot_thunks':(0x4f05b0,0x4f05c5),
}

def bits(f):return struct.unpack('<I',struct.pack('<f',f))[0]
def digest(b):return hashlib.sha256(b).hexdigest()

class Machine:
 def __init__(self,tick=0,phase=0):
  self.u=Uc(UC_ARCH_X86,UC_MODE_32)
  self.u.mem_map(0,0x1000)
  self.u.mem_map(NATIVE.base,(NATIVE.image_size+0xfff)&~0xfff)
  for vs,r,size,off in NATIVE.sections:self.u.mem_write(NATIVE.base+r,NATIVE.b[off:off+size])
  self.u.mem_map(0x2000000,0x200000);self.u.mem_map(0x3000000,0x20000)
  self.world=0x2000000;self.next=0x2010000;self.stack=0x3008000;self.stop=0x300f000
  self.events=[];self.actors={};self.names={};self.actions={};self.node_names={};self.queue_hooks={}
  self.write(0xb36534,self.world)
  svar=self.alloc(0x300);fog=self.alloc(4);self.write(0xb39ab0,svar);self.write(svar+0x1bc,fog)
  registry=self.alloc(0x20);self.write(self.world+0x130,registry)
  self.write(self.world+0xdc,tick);self.write(self.world+0xd8,phase)
  self.write(self.world+0xe0,bits(max(0,tick-1)*.0625));self.write(self.world+0xe4,bits(tick*.0625));self.write(self.world+0xe8,bits(.0625))
  self.write(self.world+0x13c,self.alloc(0x50))
  self.aux=self.alloc(4);self.write(self.aux,7);self.write(self.world+4,self.aux)
  self.actor_vtable=self.alloc(0x200)
  for slot in [0x8c,0x90,0xb0,0xb4,0xbc,0xc0,0xe0,0xf0]:self.write(self.actor_vtable+slot,0x21f0000+slot)
  for q,off in enumerate([0x1ac,0x1c0,0x1d4,0x1e8,0x208]):
   table=self.alloc(0x10);self.write(table+8,0x21e0000+q*0x10);self.write(self.world+off,table)
  self.u.reg_write(UC_X86_REG_FPCW,0x037f)
  self.u.hook_add(UC_HOOK_CODE,self.hook)
 def alloc(self,n):
  p=self.next;self.next+=(n+15)&~15;assert self.next<0x21d0000
  self.u.mem_write(p,b'\0'*n);return p
 def write(self,a,v):self.u.mem_write(a,struct.pack('<I',v&0xffffffff))
 def read(self,a):return struct.unpack('<I',self.u.mem_read(a,4))[0]
 def arg(self,i=0):return self.read(self.u.reg_read(UC_X86_REG_ESP)+4+4*i)
 def ret(self,bytes=0,value=0):
  sp=self.u.reg_read(UC_X86_REG_ESP);pc=self.read(sp)
  self.u.reg_write(UC_X86_REG_EAX,value);self.u.reg_write(UC_X86_REG_ESP,sp+4+bytes);self.u.reg_write(UC_X86_REG_EIP,pc)
 def event(self,kind,**kw):
  self.events.append({'event':kind,'phase':self.read(self.world+0xd8),'tick':self.read(self.world+0xdc),**kw})
 def trigger(self,key):
  action=self.actions.pop(key,None)
  if action:action()
 def actor(self,name,retain=(False,),classification=None,flags=0):
  p=self.alloc(0x140);self.write(p,self.actor_vtable);self.write(p+8,flags)
  self.names[p]=name;self.actors[p]={'retain':list(retain),'classification':classification}
  return p
 def payload(self,name):
  p=self.alloc(0x80);self.names[p]=name
  component=self.alloc(0x20);self.names[component]=name;self.write(p+0x60,component)
  return p
 def append(self,off,p,front=False):
  node=self.alloc(8);self.write(node,p);self.node_names[node]=f'{off:03x}:{self.names.get(p,hex(p))}'
  head=self.read(self.world+off);tail=self.read(self.world+off+4)
  if front:self.write(node+4,head);self.write(self.world+off,node)
  elif tail:self.write(tail+4,node)
  else:self.write(self.world+off,node)
  if not tail or not front:self.write(self.world+off+4,node)
  return node
 def list_names(self,off):
  out=[];p=self.read(self.world+off)
  while p:
   assert len(out)<100
   out.append(self.names.get(self.read(p),hex(self.read(p))));p=self.read(p+4)
  return out
 def hook(self,u,pc,size,user):
  ecx=u.reg_read(UC_X86_REG_ECX)
  if pc in (0x41c0e0,0x41c110):self.ret(4);return
  if pc==0x77d9d0:self.event('fog_callback',enabled=self.arg(1)&255);self.ret(12);return
  if pc==0x4f65b0:self.event('request_phase',requested=self.arg());return
  if pc==0x4f96d0:self.event('first_tick_callback',aux=self.read(self.aux));self.ret();return
  if pc==0x5147c0:
   name=self.names[ecx];self.event('deferred_payload',name=name);self.trigger('payload:'+name);self.ret();return
  if 0x21e0000<=pc<0x21e0050 and (pc-0x21e0000)%0x10==0:
   q=(pc-0x21e0000)//0x10;self.event('queue_update',queue=q);self.trigger('queue:'+str(q));self.ret();return
  if pc==0x54c280:self.event('manager_13c_update');self.trigger('manager');self.ret();return
  if pc==0x561c20:
   name=self.names[ecx];self.event('pending_100_component',name=name);self.trigger('component:'+name);self.ret();return
  if pc==0x4f4c40:self.event('availability_leaf',name=self.names[self.arg()]);return
  if pc==0x50c1b0:self.event('registry_unlink',name=self.names[self.arg()]);return
  if pc==0x50c510:self.event('retained_unlink',name=self.names[self.read(self.arg())]);return
  if pc==0x4f1430:self.event('classification_remove',list_offset=hex(ecx-self.world),name=self.names[self.read(self.arg())]);return
  if pc==0x6f7430:self.event('free_list_node',node=self.node_names.get(self.arg(),'native_new'));self.ret();return
  if pc==0x6f7810:self.ret(value=self.alloc(self.arg()));return
  if 0x21f0000<=pc<0x21f0100:
   slot=pc-0x21f0000;actor=self.actors[ecx];name=self.names[ecx]
   if slot in (0xb0,0xb4):self.ret();return
   if slot==0x8c:
    plan=actor['retain'];value=plan.pop(0) if len(plan)>1 else plan[0]
    self.event('actor_8c',name=name,result=value);self.ret(value=int(value));return
   if slot in (0xbc,0xc0):
    value=actor['classification']==slot;self.event(f'actor_{slot:02x}',name=name,result=value);self.ret(value=int(value));return
   self.event(f'actor_{slot:02x}',name=name,**({'argument':self.arg()} if slot in(0xe0,0xf0) else {}))
   self.trigger(f'actor:{name}:{slot:02x}');self.ret(4 if slot in(0xe0,0xf0) else 0);return
 def run(self,address=0x4f9970,args=()):
  self.write(self.stack,self.stop)
  for i,a in enumerate(args):self.write(self.stack+4+i*4,a)
  self.u.reg_write(UC_X86_REG_ECX,self.world);self.u.reg_write(UC_X86_REG_ESP,self.stack)
  registers={UC_X86_REG_EBX:0xabc00001,UC_X86_REG_ESI:0xabc00002,UC_X86_REG_EDI:0xabc00003,UC_X86_REG_EBP:0xabc00004}
  for r,v in registers.items():self.u.reg_write(r,v)
  self.u.emu_start(address,self.stop,count=200000)
  assert self.u.reg_read(UC_X86_REG_ESP)==self.stack+4+4*len(args)
  for r,v in registers.items():assert self.u.reg_read(r)==v
  return {'events':self.events,'final':{'phase':self.read(self.world+0xd8),'tick':self.read(self.world+0xdc),'begin_bits':self.read(self.world+0xe0),'end_bits':self.read(self.world+0xe4),'aux':self.read(self.aux),
    'lists':{hex(off):self.list_names(off) for off in [0x100,0x108,0x110,0x178,0x188,0x1a4]},
    'actor_flags':{self.names[a]:self.read(a+8) for a in self.actors}}}

def only(row,event):return [r for r in row['events'] if r['event']==event]

def cases():
 out=[]
 for tick in [0,7]:
  m=Machine(tick);r=m.run();r['name']='first_tick' if tick==0 else 'subsequent_tick'
  assert [e['requested'] for e in only(r,'request_phase')]==[1,4,5,0]
  assert [e['queue'] for e in only(r,'queue_update')]==[0,1,2,3,4]
  assert [e['phase'] for e in only(r,'queue_update')]==[1,1,1,1,5]
  assert len(only(r,'first_tick_callback'))==int(tick==0)
  assert r['final']['tick']==tick+1 and r['final']['end_bits']==bits((tick+1)*.0625)
  out.append(r)
 m=Machine(7);a=m.payload('A');b=m.payload('B');c=m.payload('C')
 m.append(0x1a4,a);m.append(0x1a4,b);m.actions['payload:A']=lambda:m.append(0x1a4,c)
 r=m.run();r['name']='deferred_tail_append_during_callback'
 assert [e['name'] for e in only(r,'deferred_payload')]==['A','B','C'];assert not r['final']['lists']['0x1a4'];out.append(r)
 m=Machine(7);p=m.payload('P');q=m.payload('Q');a=m.actor('A',classification=0xbc);b=m.actor('B',(True,False),0xc0,0x20)
 m.append(0x188,a);m.append(0x178,b)
 m.actions['queue:0']=lambda:(m.append(0x100,p),m.append(0x108,a))
 m.actions['manager']=lambda:(m.append(0x100,q),m.append(0x108,b))
 r=m.run();r['name']='phase_boundaries_drain_then_retained_final_scan'
 assert [e['phase'] for e in only(r,'pending_100_component')]==[2,2]
 assert [e['name'] for e in only(r,'actor_e0')]==['A','B'];assert r['final']['actor_flags']['B']==0xa1
 assert not any(r['final']['lists'][x] for x in ['0x108','0x110','0x178','0x188']);out.append(r)
 m=Machine(7);a=m.actor('A',(True,));b=m.actor('B',(True,));m.append(0x108,a);m.append(0x108,b)
 r=m.run();r['name']='retained_insertions_are_front_order'
 assert r['final']['lists']['0x110']==['B','A'];assert [e['name'] for e in only(r,'actor_90')]==['A','B'];out.append(r)
 m=Machine(7);a=m.actor('A');b=m.actor('B',(True,));c=m.actor('C')
 for p in [a,b,c]:m.append(0x110,p)
 r=m.run();r['name']='retained_scan_preserves_next_before_deleting_current'
 assert [e['name'] for e in only(r,'actor_8c')]==['A','B','C'];assert r['final']['lists']['0x110']==['B'];out.append(r)
 m=Machine(7);a=m.actor('A');b=m.actor('B',(True,));d=m.actor('D')
 m.append(0x110,a);m.append(0x110,b);m.actions['actor:A:e0']=lambda:m.append(0x110,d,front=True)
 r=m.run();r['name']='new_head_during_retained_cleanup_waits'
 assert [e['name'] for e in only(r,'actor_8c')]==['A','B'];assert r['final']['lists']['0x110']==['D','B'];out.append(r)
 m=Machine(7);a=m.actor('A');b=m.actor('B');m.append(0x108,a);m.actions['actor:A:90']=lambda:m.append(0x108,b)
 r=m.run();r['name']='removal_callback_addition_drained_same_transition'
 assert [e['name'] for e in only(r,'actor_e0')]==['A','B'];out.append(r)
 for old,new in [(0,0),(1,0),(4,5),(5,0)]:
  m=Machine(7,old);p=m.payload('P');a=m.actor('A');m.append(0x100,p);m.append(0x108,a)
  r=m.run(0x4f65b0,(new,));r['name']=f'transition_{old}_to_{new}'
  assert bool(only(r,'pending_100_component'))==(old in[1,4] and old!=new)
  assert bool(only(r,'actor_e0'))==(old!=new);assert r['final']['phase']==new;out.append(r)
 return out

def main():
 evidence={};cs=Cs(CS_ARCH_X86,CS_MODE_32)
 for name,(start,end) in RANGES.items():
  data=NATIVE.range(start,end-start);assert data==ACCEPTED.range(start,end-start),name
  evidence[name]={'start':hex(start),'end_exclusive':hex(end),'sha256':digest(data),'unchanged_in_v14':True}
  (HERE/(name+'.asm.txt')).write_text('\n'.join(f'{i.address:08x} {i.bytes.hex():24} {i.mnemonic} {i.op_str}' for i in cs.disasm(data,start))+'\n')
 result={'passed':True,'schema':1,'source_sha256':digest(NATIVE.b),'accepted_sha256':digest(ACCEPTED.b),'producer_sha256':digest(Path(__file__).read_bytes()),
 'method':'Actual original world_update, phase_transition and list removal instructions; explicit leaf/virtual tracers in isolated Unicorn. No native process.',
 'fpu_control_word':'0x037f','ranges':evidence,'cases':cases(),
 'executed_helpers':['4F65B0 phase transition','50C510 list-node removal','4F1430 list-value removal','4F4C40 availability leaf with B0/B4 both false','50C1B0 registry-unlink with empty registry'],
 'seams':['Timing/Lua callbacks, first-tick callback, deferred payload5147C0, all5 embedded queue Update virtuals, manager54C280, component561C20, allocator/free, actorvirtual booleans/callbacks. Leaf actions are explicit controlled fixture mutations, not gameplay implementations.'],
 'limits':['No periodic/deferred queue internals or leaf gameplay are executed by these outer-dispatch cases.','Fixture actors expose controlled virtual-slot responses; real class meanings and all producers remain to be proven.','This does not establish command cadence, native save/network protocol, full world phases or safe arbitrary parallelism.']}
 result['case_count']=len(result['cases']);target=HERE/'world-dispatch-oracle.json';target.write_text(json.dumps(result,indent=2)+'\n')
 print(json.dumps({'passed':True,'cases':result['case_count'],'sha256':digest(target.read_bytes())}))

if __name__=='__main__':main()
