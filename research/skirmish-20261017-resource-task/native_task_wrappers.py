"""Native5B0270/5B1910/5B1A10/5B2180 texture task wrappers.

Real queue operations and Texture3C5AA6F0 execute. Explicit transports are
Win32 mutex/atomic/TLS, heap, diagnostics/timers/current-thread name, and
Texture4C decoder RETURN OR REQUIRED STOP. Successful decoder transport
certifies only the task coordinator contract; it is not texture payload,
GPU/backend, C++ exception-unwind, active waiter or resource-readiness proof.
"""
from pathlib import Path
import argparse,hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'))
from native_machine import Machine,original,modified
from unicorn import UC_HOOK_MEM_WRITE
sha=lambda b:hashlib.sha256(b).hexdigest()

def cases():
 base=dict(entry='request',mode=1,flags=0x200000,queued=False,state=0,timing=False,clear_cache=False,stop=None)
 out=[]
 def add(name,**change):out.append(dict(base,name=name,**change))
 for mode in [0,1,2,3,4,0xffffffff,0x80000000]:add('request-mode'+str(mode),mode=mode)
 add('cache-required',flags=0x200020)
 add('cache-cleared-after-lock',flags=0x200020,clear_cache=True)
 add('cache-cleared-sync',mode=4,flags=0x200020,clear_cache=True)
 add('sync-required-thread',mode=4,stop='thread')
 add('sync-required-decoder',mode=4,stop='load')
 for timing in [False,True]:
  add('block-queued-'+str(timing),entry='block',queued=True,timing=timing)
  for state in [2,3]:add('block-complete-%d-%s'%(state,timing),entry='block',state=state,timing=timing)
 add('block-wait-required',entry='block')
 add('block-running-required',entry='block',state=1)
 add('block-decoder-required',entry='block',queued=True,stop='load')
 add('block-thread-required',entry='block',queued=True,stop='thread')
 return out

def run(c,image,pc):
 m=Machine(pc)
 old=(original.image_size+4095)&~4095;new=(image.image_size+4095)&~4095
 if new>old:m.u.mem_map(image.base+old,new-old)
 for _,rva,size,offset in image.sections:m.u.mem_write(image.base+rva,image.b[offset:offset+size])
 manager=m.alloc(0x140);queue=manager+0xfc;resource=m.alloc(0x80);task=m.alloc(0x20)
 m.w(0xb36540,manager);m.w(0xb3da08,0)
 events=[];stores=[];terminal=['returned'];active=[False]
 vt=m.alloc(16)
 def lock(_):
  if active[0] and c['clear_cache'] and m.ecx()==manager+0x20 and m.r(resource+8)&0x20:m.w(resource+8,m.r(resource+8)&~0x20)
  m.ret()
 m.w(vt+4,m.stub(lock));m.w(vt+8,m.stub(lambda _:m.ret()))
 for offset in [4,0x20,0x3c,0xfc]:m.w(manager+offset,vt)
 def allocate(_):m.ret(m.alloc(m.arg(1)))
 m.seam(0x6f7810,allocate);m.seam(0x6f7430,lambda _:m.ret())
 for address in [0x857e10,0x857de0,0x8573a0]:m.seam(address,lambda _:m.ret())
 def atomic(delta):
  def call(_):
   p=m.arg(1);value=(m.r(p)+delta)&0xffffffff;m.w(p,value);m.ret(value,4)
  return call
 m.w(0x9f01e8,m.stub(atomic(1)));m.w(0x9f01ec,m.stub(atomic(-1)))
 m.w(resource,0xa454a8);m.w(resource+8,c['flags']);m.w(resource+0x20,task)
 name=m.array([0x00410041,0]);m.w(resource+0x38,name)
 #Explicit laboratory non-Map RTTI chain for actual Texture GetClass leaf.
 descriptor=m.call(0x5a6180,resource);m.w(descriptor+8,0)
 m.call(0x5b1850,task,[resource,1]);m.w(task+4,1)
 def stop(reason):terminal[0]=reason+'_required';m.u.emu_stop()
 def logger(_):
  if active[0]:
   line=m.arg(3)
   label={0x61a:'request_immediate',0x61f:'request_queue',0x24b:'loaded',0x636:'removing',0xc7:'diag_immediate',0xd2:'diag_completed',0xe1:'diag_waited'}.get(line)
   if label:events.append(label)
  m.ret()
 m.seam(0x404f90,logger);m.seam(0x404f60,logger)
 if c['queued']:m.call(0x5aed10,manager,[task])
 m.w(task+0x14,c['state'])
 thread=m.alloc(0x60);m.w(thread+0x50,name)
 m.wb(0xb43bbc,1);m.w(0xb43bc0,17)
 def tls(_):
  events.append('thread')
  if c['stop']=='thread':stop('thread');return
  m.ret(thread,4)
 m.w(0x9f020c,m.stub(tls))
 def decoder(_):
  events.append('load')
  if c['stop']=='load':stop('load');return
  m.ret()
 m.seam(0x5a8cd0,decoder)
 m.observe(0x5b1a10,lambda _:events.append('enter_load'))
 m.observe(0x5aa6f0,lambda _:events.append('after'))
 m.observe(0x5b1adf,lambda _:events.append('leave_load'))
 m.observe(0x5b02ab,lambda _:events.append('enter_request'))
 for a in [0x5b02df,0x5b0303]:m.observe(a,lambda _:events.append('leave_request'))
 def cache(_):events.append('cache');stop('cache')
 m.seam(0x59bfa0,cache)
 for address,event in [(0x59aa70,'timer_reset'),(0x59ab60,'timer_start')]:
  def timing(_,label=event):events.append(label);m.ret()
  m.seam(address,timing)
 def enabled(_):events.append('timing_enabled');m.ret(int(c['timing']))
 m.seam(0x40eaa0,enabled);m.seam(0x59aba0,lambda _:m.ret(7));m.seam(0x40f770,lambda _:m.ret(thread))
 def wait(_):events.append('wait');stop('wait')
 m.seam(0x5b1fd0,wait)
 def writes(u,access,address,size,value,data):
  if active[0] and address==resource+8:stores.append(value)
 m.u.hook_add(UC_HOOK_MEM_WRITE,writes)
 active[0]=True
 try:
  if c['entry']=='request':m.call(0x5b0270,manager,[resource,c['mode']])
  else:m.call(0x5b2180,task)
 except AssertionError:
  if terminal[0]=='returned':raise
 out=dict(terminal=terminal[0],events=events,flag_stores=stores,flags=m.r(resource+8),task=dict(refs=m.r(task+4),priority=m.r(task+8),sequence=m.r(task+0xc),state=m.r(task+0x14),thread_present=m.r(task+0x18)!=0,queued=m.r(task+0x10)!=0),queue_contains=m.r(queue+0x24)!=0)
 return out,m.visited

def main():
 p=argparse.ArgumentParser();p.add_argument('--create',action='store_true');a=p.parse_args()
 rows=[];pins={}
 for pc in ['Single24','Double53','Extended64']:
  for case in cases():
   pair=[]
   for image in [original,modified]:
    out,visited=run(case,image,pc);pair.append(out)
    for address,raw in visited.items():
     assert raw==original.range(address,len(raw))==modified.range(address,len(raw)),hex(address)
     pins[address]=raw
   assert pair[0]==pair[1],(case,pc)
   rows.append(dict(precision=pc,input=case,native=pair[0]))
 out=dict(scope=__doc__,original_sha256=sha(original.b),v14_sha256=sha(modified.b),rows=rows,pins=[dict(address=hex(a),bytes=b.hex())for a,b in sorted(pins.items())])
 path=HERE/'native-task-wrappers.json'
 if a.create:
  with path.open('x',encoding='utf-8')as f:json.dump(out,f,indent=2);f.write('\n')
 else:assert out==json.loads(path.read_text(encoding='utf-8'))
 print(json.dumps(dict(paired_rows=len(rows),separate_executions=len(rows)*2,pins=len(pins),fixture_sha256=sha(path.read_bytes()))))
if __name__=='__main__':main()
