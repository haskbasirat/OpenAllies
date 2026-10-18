"""Bounded native ASYNC ownership/start/LoadGame-dispatch proof.

Constructor fragments retain caller-owned Application/ResourceManager allocations;
unrelated constructors, locks, allocators and OS services are explicitly named
seams. Thread creation returns a laboratory handle/ID: no emulated row claims an
actual Windows thread was created. Run dispatch stops at the real LoadGame Run
entry rather than replacing its loading implementation with successful output.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
from unicorn.x86_const import *
ORIGINAL=nm.original
def machine(image,pc):
 m=nm.Machine(pc)
 if image=='v14':
  old=(ORIGINAL.image_size+0xfff)&~0xfff;new=(nm.modified.image_size+0xfff)&~0xfff
  if new>old:m.u.mem_map(ORIGINAL.base+old,new-old)
  for vs,rva,raw,off in nm.modified.sections:m.u.mem_write(nm.modified.base+rva,nm.modified.b[off:off+raw])
 return m
def fragment(m,start,end,registers={},args=()):
 sp=nm.STACK-0x100
 m.w(sp,nm.STOP)
 for i,value in enumerate(args):m.w(sp+4+4*i,value)
 m.u.reg_write(UC_X86_REG_ESP,sp)
 for r,v in registers.items():m.u.reg_write(r,v)
 m.u.emu_start(start,end,count=100000)
 assert not m.null_accesses,m.null_accesses
 assert m.u.reg_read(UC_X86_REG_EIP)==end,hex(m.u.reg_read(UC_X86_REG_EIP))
def source_owners(image,pc):
 m=machine(image,pc);events=[];app=0xb39690;manager=m.alloc(0x140)
 m.seam(0x4177e0,lambda mm:(events.append(['application_ctor',hex(mm.ecx())]),mm.ret(mm.ecx())))
 m.seam(0x96cab0,lambda mm:(events.append(['atexit',hex(mm.arg(1))]),mm.ret()))
 m.call(0x9c8560)
 def string(mm):
  text=bytes(m.u.mem_read(mm.arg(1),24)).decode('utf-16le').split('\0')[0]
  p=m.alloc(0x50);m.w(p+0xc,3);m.w(mm.ecx(),p+0x10)
  events.append(['name',text]);mm.ret(mm.ecx(),pop=4)
 m.seam(0x401b00,string)
 m.seam(0x85f9b0,lambda mm:(events.append(['thread_ctor',hex(mm.ecx()-app)]),mm.ret(mm.ecx(),pop=4)))
 fragment(m,0x41795e,0x4179fb,{UC_X86_REG_ESI:app,UC_X86_REG_EBX:0})
 thunk=manager+0xeb
 m.w(0x9f0228,m.stub(lambda mm:mm.ret(0xffffffff)))
 def flush(mm):
  assert [mm.arg(i) for i in range(1,4)]==[0xffffffff,thunk,14]
  events.append(['flush_resource_thunk',14]);mm.ret(1,pop=12)
 m.w(0x9f0224,m.stub(flush))
 fragment(m,0x5b0576,0x5b05a9,{UC_X86_REG_ESI:manager,UC_X86_REG_EAX:thunk})
 assert m.rb(thunk)==0xb8 and m.r(thunk+1)==manager and m.r(thunk+5)==0x50240487 and m.rb(thunk+9)==0xe9
 destination=(thunk+14+m.r(thunk+10))&0xffffffff
 assert destination==0x5addd0
 events.append(['resource_thunk_target',hex(destination)])
 m.w(manager+0x118,0x13572468)
 def register(mm):
  assert mm.ecx()==0xb39888+0x68
  assert [mm.arg(1),mm.arg(2)]==[0x13572468,3]
  events.append(['async_signal_registration',3]);mm.ret(pop=8)
 m.seam(0x85e880,register)
 fragment(m,0x5abe08,0x5abe21,{UC_X86_REG_ESI:manager})
 return dict(events=events,async_owner=hex(app+0x1f8),thunk_target=hex(destination)),m.visited
def start_worker(image,pc,tls,hold,handle=0x24681357,identity=44416):
 m=machine(image,pc);app=0xb39690;child=app+0x1f8;parent=m.alloc(0xb0);manager=m.alloc(0x140);events=[]
 m.w(child,0xab5d58);m.w(child+0x44,0);m.w(child+0x48,0);m.w(child+0x64,0);m.w(app+0x134,manager)
 pvt=m.alloc(0x50);m.w(parent,pvt)
 def current(mm):events.append('get_current_id');mm.ret(10708)
 def lookup(mm):
  assert m.r(mm.arg(1))==10708;events.append('lookup_parent');mm.ret(parent,pop=4)
 def tlsget(mm):events.append('tls_parent');mm.ret(parent,pop=4)
 m.wb(0xb43bbc,tls);m.w(0xb43bc0,19);m.w(0x9f01e4,m.stub(current));m.w(0x9f020c,m.stub(tlsget));m.seam(0x40e930,lookup)
 def begin(mm):
  values=[mm.arg(i) for i in range(1,7)]
  assert values[:5]==[0,0,child+0x54,0,4]
  assert m.r(child+0xa4)==manager+0xeb and m.r(child+0x4c)==parent
  events.append('create_suspended');m.w(values[5],identity);mm.ret(handle)
 m.seam(0x9713ef,begin)
 def parent_add(mm):
  assert mm.ecx()==parent and mm.arg(1)==child
  assert [m.r(child+o) for o in [0x44,0x48,0x64]]==[handle,identity,2]
  events.append('register_with_parent');mm.ret(pop=4)
 m.w(pvt+0x34,m.stub(parent_add))
 m.seam(0x85dba0,lambda mm:(events.append('on_created'),mm.ret()))
 def resume(mm):
  assert events[-2:]==['register_with_parent','on_created'];events.append('resume');mm.ret()
 m.seam(0x85f0a0,resume)
 def priority(mm):
  assert mm.arg(1)==handle and mm.arg(2)==m.r(0xab5864)
  events.append('priority_1');mm.ret(1,pop=8)
 m.w(0x9f00e8,m.stub(priority))
 if hold:
  m.call(0x85e7b0,child,[manager+0xeb,hold])
 else:fragment(m,0x419ba4,0x419bc8,{UC_X86_REG_ESI:app})
 assert ('resume'in events)==(not hold)
 return dict(tls=bool(tls),hold=bool(hold),events=events,thread_id=m.r(child+0x48),handle=m.r(child+0x44),state=m.r(child+0x64),callback_is_manager_thunk=m.r(child+0xa4)==manager+0xeb),m.visited
def worker_loop(image,pc):
 m=machine(image,pc);child=m.alloc(0xb0);manager=m.alloc(0x140);logger=m.alloc(0x80);vt=m.alloc(0x80);events=[]
 m.w(child,0xab5d58);m.w(child+0x64,1);m.w(child+0x88,0);m.w(child+0x90,0);m.w(child+0xa4,manager+0xeb);m.w(child+0x48,44416)
 # The exact constructor-produced thunk (same bytes proven in source_owners).
 thunk=manager+0xeb
 import struct
 m.u.mem_write(thunk,b'\xb8'+struct.pack('<I',manager)+b'\x87\x04\x24\x50\xe9'+struct.pack('<I',(0x5addd0-thunk-14)&0xffffffff))
 m.w(0xb43b74,logger);m.w(logger,vt)
 m.w(vt+0x1c,m.stub(lambda mm:(events.append('thread_started_diagnostic'),mm.ret())))
 m.w(vt+0x6c,m.stub(lambda mm:(events.append('before_callback'),mm.ret(pop=4))))
 m.w(0x9f0228,m.stub(lambda mm:mm.ret(0xffffffff)))
 m.w(0x9f0224,m.stub(lambda mm:mm.ret(1,pop=12)))
 # Execute actual base constructor's entry-thunk producer. Unrelated base fields
 # are admitted caller state here, not a claim of full Thread construction.
 fragment(m,0x85f953,0x85f989,{UC_X86_REG_ESI:child})
 assert m.r(child+0x55)==child
 assert (child+0x54+14+m.r(child+0x5e))&0xffffffff==0x85f700
 m.wb(0xb43bbc,1);m.w(0xb43bc0,19)
 def tls_set(mm):
  assert [mm.arg(1),mm.arg(2)]==[19,child];events.append('publish_worker_tls');mm.ret(1,pop=8)
 m.w(0x9f00e0,m.stub(tls_set))
 def wait(mm):
  assert [mm.arg(1),mm.arg(2)]==[0,0];events.append('wait_signal');mm.ret(0,pop=12)
 m.seam(0x85ebc0,wait)
 fragment(m,child+0x54,0x5addd0,args=[0])
 assert m.arg(1)==manager
 events.append('reached_resource_update')
 return dict(events=events,entry='Thread85F700',callback='ResourceManager5ADDD0'),m.visited
def loadgame_dispatch(image,pc,tls):
 m=machine(image,pc);manager=m.alloc(0x140);task=m.alloc(0x50);node=m.alloc(12);sim=m.alloc(0x40);worker=m.alloc(0xb0);events=[]
 m.w(task,0xa43794);m.w(task+4,1);m.w(task+0x10,0x87654321);m.w(task+0x14,0);m.w(task+0x18,0)
 # Native LoadGame vtable predicates are executed, not callback answers.
 assert m.r(0xa43794+0x10)==0x6a71d0 and m.r(0xa43794+0x14)==0x6a71f0 and m.r(0xa43794+0x1c)==0x5a4280
 m.w(node,task);m.w(node+4,0);m.w(node+8,0);m.w(manager+0x120,node);m.w(manager+0x124,node);m.w(0xb36524,sim)
 def lock_obj(address,label):
  vt=m.alloc(16);m.w(address,vt)
  m.w(vt+4,m.stub(lambda mm:(events.append(label+'_lock'),mm.ret())))
  m.w(vt+8,m.stub(lambda mm:(events.append(label+'_unlock'),mm.ret())))
 lock_obj(manager+0x20,'queue');lock_obj(sim+8,'sim8');lock_obj(sim+0x24,'sim24')
 m.seam(0x857e10,lambda mm:(events.append('queue_guard_lock'),mm.ret()))
 m.seam(0x857de0,lambda mm:(events.append('queue_guard_unlock'),mm.ret()))
 def increment(mm):p=mm.arg(1);m.w(p,m.r(p)+1);mm.ret(m.r(p),pop=4)
 def decrement(mm):p=mm.arg(1);m.w(p,m.r(p)-1);mm.ret(m.r(p),pop=4)
 m.w(0x9f01e8,m.stub(increment));m.w(0x9f01ec,m.stub(decrement))
 m.seam(0x6f7430,lambda mm:(events.append('free_queue_node'),mm.ret()))
 m.wb(0xb43bbc,tls);m.w(0xb43bc0,19)
 m.w(0x9f020c,m.stub(lambda mm:(events.append('current_worker_tls'),mm.ret(worker,pop=4))))
 m.w(0x9f01e4,m.stub(lambda mm:(events.append('current_worker_id'),mm.ret(44416))))
 m.seam(0x40e930,lambda mm:(events.append('current_worker_lookup'),mm.ret(worker,pop=4)))
 m.seam(0x59e5c0,lambda mm:(events.append('task_name'),mm.ret(0xabcdef)))
 m.seam(0x404f90,lambda mm:(events.append('diagnostic'),mm.ret()))
 # Actual LoadGame blocking selector and5B1600 lock/wait order. The manager's
 # separate waiter registration and actual waiting implementation are named
 # seams, not an emulated claim that another OS thread completed the task.
 m.w(0xb36540,manager)
 def register_wait(mm):
  assert mm.ecx()==manager and mm.arg(1)==task
  events.append('register_task_waiter');mm.ret(0x12345678,pop=4)
 m.seam(0x5ad2e0,register_wait)
 def release_recursive(mm):
  assert mm.ecx()==sim+0x24;events.append('release_recursive_sim24');mm.ret(3)
 m.seam(0x8573c0,release_recursive)
 def task_wait(mm):
  assert mm.ecx()==manager and mm.arg(1)==0x12345678
  events.append('wait_task_completion');mm.ret(pop=4)
 m.seam(0x5ad3d0,task_wait)
 def restore_recursive(mm):
  assert mm.ecx()==sim+0x24 and mm.arg(1)==3
  events.append('restore_recursive_sim24');mm.ret(pop=4)
 m.seam(0x8573f0,restore_recursive)
 m.call(0x5b2180,task)
 fragment(m,0x5addd0,0x5a4280,args=[manager])
 assert m.ecx()==task and m.r(task+0x14)==1 and m.r(task+0x18)==worker
 assert m.r(task+0x10)==0 and m.r(manager+0x120)==m.r(manager+0x124)==0
 events.append('reached_loadgame_run_5a4280')
 return dict(tls=bool(tls),events=events,task_state=m.r(task+0x14),worker_owner_matches=m.r(task+0x18)==worker,queue_empty=m.r(manager+0x120)==0),m.visited
def main():
 rows=[];pins={}
 for pc in nm.PRECISIONS:
  cases=[('source_owners',source_owners,()),('worker_loop',worker_loop,())]
  cases += [('start_worker',start_worker,(tls,hold)) for tls,hold in itertools.product([0,1],[0,1])]
  cases += [('loadgame_dispatch',loadgame_dispatch,(tls,)) for tls in[0,1]]
  for name,fn,args in cases:
   a,pa=fn('original',pc,*args);b,pb=fn('v14',pc,*args);assert a==b,(name,a,b)
   pins.update(pa);pins.update(pb);rows.append(dict(precision=pc,operation=name,input=list(args),output=a))
 for va,raw in pins.items():assert ORIGINAL.range(va,len(raw))==nm.modified.range(va,len(raw))==raw,hex(va)
 refs=[(0xa43794,0x20),(0xab5d58,0x38),(0x9f73fc,12),(0x9f7408,8)]
 data=[]
 for va,size in refs:
  raw=ORIGINAL.range(va,size);assert raw==nm.modified.range(va,size);data.append(dict(va=hex(va),bytes=raw.hex()))
 doc=dict(schema=1,scope=__doc__,rows=rows,native_executions=len(rows)*2,instructions=[dict(va=hex(va),bytes=raw.hex())for va,raw in sorted(pins.items())],static_source_references=data)
 path=HERE/'native-async-worker.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=doc['native_executions'],pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
