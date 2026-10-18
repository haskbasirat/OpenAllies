"""Actual5AED10/5AE160 +5ADDD0 queue tail pop/status/thread/virtual dispatch.
View Task actual5B3550->40D970->5B18C0 executes. Resource vf1C body is
an explicit named child; allocator/free/atomic imports/synchronization/log/TLS
thread-map are host transports. Stops before unrelated5AE022 depot timing.
No decoder/GPU/worker-thread bootstrap or natural whole-view claim.
"""
from pathlib import Path
import sys,json,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as n
class Required(Exception):pass
class Done(Exception):pass
def cases():
 out=[]
 def add(name,tasks,**kw):out.append(dict(name=name,tasks=tasks,sequence=0,stop=None,**kw))
 for priorities in [[1,1,0],[0,0,0],[-1,0,1],[3,0,2,0],[0x7fffffff,0,0x80000000]]:
  add('order_'+str(priorities),[dict(kind='view'if i==len(priorities)-1 else'resource',priority=p&0xffffffff)for i,p in enumerate(priorities)])
 add('view_only',[dict(kind='view',priority=0)])
 add('empty',[])
 for stop in ['thread','diagnostic','resource','loading']:
  c=dict(name='required_'+stop,tasks=[dict(kind='resource',priority=1),dict(kind='view',priority=0)],sequence=0,stop=stop);out.append(c)
 for bits in [0,1,0x5a,0xff]:
  out.append(dict(name='flags_'+str(bits),tasks=[dict(kind='view',priority=0)],sequence=0,stop=None,ac=bits,loading17=bits))
 out.append(dict(name='sequence_wrap',tasks=[dict(kind='resource',priority=1),dict(kind='view',priority=0)],sequence=0xffffffff,stop=None))
 out.append(dict(name='cancel_at_diagnostic',tasks=[dict(kind='view',priority=0)],sequence=0,stop=None,cancel=True))
 return out
def run(c,image,pc):
 m=n.Machine(pc)
 old=(n.original.image_size+4095)&~4095;new=(image.image_size+4095)&~4095
 if new>old:m.u.mem_map(image.base+old,new-old)
 for _,rva,size,off in image.sections:m.u.mem_write(image.base+rva,image.b[off:off+size])
 manager=m.alloc(0x140);queue=manager+0xfc;view=m.alloc(0x210);loading=m.alloc(0x20)
 m.w(0xb3da08,c['sequence']);m.w(0xb36538,view);m.w(0xb39690+0xe4,loading)
 m.wb(view+0xac,c.get('ac',1));m.wb(view+0xad,0x5a);m.wb(loading+0x17,c.get('loading17',0))
 vt=m.alloc(16);m.w(vt+4,m.stub(lambda mm:mm.ret()));m.w(vt+8,m.stub(lambda mm:mm.ret()));m.w(manager+0x20,vt);m.w(manager+4,vt);m.w(queue,vt)
 events=[];tasks=[];ids={};current=[None]
 def event(e):
  events.append(e)
  if c['stop']==e[0]:raise Required(e[0])
 for a in [0x857e10,0x857de0,0x8573a0]:m.seam(a,lambda mm:mm.ret())
 def malloc(mm):
  size=mm.arg(1);assert size==12;mm.ret(mm.alloc(size))
 m.seam(0x6f7810,malloc);m.seam(0x6f7430,lambda mm:mm.ret())
 for address,delta in [(0x9f01e8,1),(0x9f01ec,-1)]:
  def atomic(mm,delta=delta):
   at=mm.arg(1);value=(mm.r(at)+delta)&0xffffffff;mm.w(at,value);mm.ret(value,4)
  m.w(address,m.stub(atomic))
 m.wb(0xb43bbc,0)
 def thread(mm):event(['thread',current[0]]);mm.ret(17)
 m.w(0x9f01e4,m.stub(thread));m.seam(0x40e930,lambda mm:mm.ret(0x12340000,4))
 def diagnostic(mm):
  event(['diagnostic',current[0]])
  if c.get('cancel'):mm.w(tasks[current[0]]+0x14,4)
  mm.ret()
 m.seam(0x404f90,diagnostic)
 def resource(mm):event(['resource',ids[mm.ecx()]]);mm.w(mm.ecx()+0x14,2);mm.ret()
 m.seam(0x5b1a10,resource)
 def loading_guard(mm):event(['loading',current[0]])
 m.observe(0x40d970,loading_guard)
 def deleting(mm):event(['delete',ids[mm.ecx()]])
 m.observe(0x5c38a0,deleting)
 def popped(mm):current[0]=ids[mm.u.reg_read(n.UC_X86_REG_ESI)]
 m.observe(0x5aded3,popped)
 m.observe(0x5adf8d,lambda mm:events.append(['body',current[0]]))
 m.observe(0x5adf9b,lambda mm:events.append(['after',current[0],mm.r(tasks[current[0]]+0x14)]))
 m.observe(0x5ae022,lambda mm:(_ for _ in ()).throw(Done()))
 for i,spec in enumerate(c['tasks']):
  task=m.alloc(0x20);m.u.mem_write(task,b'\xcd'*0x20);ids[task]=i;tasks.append(task)
  if spec['kind']=='resource':
   resource=m.alloc(0x50);m.call(0x5b1850,task,[resource,spec['priority']]);m.w(task+4,1)
  else:m.call(0x5b17f0,task,[spec['priority']]);m.w(task,0xa4bba4)
  m.call(0x5aed10,manager,[task])
 def order():
  result=[];node=m.r(queue+0x24)
  while node:result.append(ids[m.r(node)]);node=m.r(node+8);assert len(result)<100
  return result
 enqueued=order();events.clear();terminal='returned'
 try:m.call(0x5addd0,0,[manager,0])
 except Done:pass
 except Required as e:terminal=str(e)
 output=dict(enqueued=enqueued,remaining=order(),sequence=m.r(0xb3da08),events=events,terminal=terminal,
  ac=m.rb(view+0xac),ad=m.rb(view+0xad),loading17=m.rb(loading+0x17),
  tasks=[dict(refs=m.r(t+4),priority=m.r(t+8),sequence=m.r(t+0xc),queued=m.r(t+0x10)!=0,state=m.r(t+0x14),thread=m.r(t+0x18))for t in tasks])
 return output,m.visited
def main():
 rows=[];pins={}
 for pc,c in itertools.product(n.PRECISIONS,cases()):
  a,pa=run(c,n.original,pc);b,pb=run(c,n.modified,pc);assert a==b,(pc,c,a,b);pins.update(pa);pins.update(pb);rows.append(dict(precision=pc,input=c,output=a))
 for a,b in pins.items():assert n.original.range(a,len(b))==n.modified.range(a,len(b))==b
 doc=dict(schema=1,scope=__doc__,original_sha256=n.sha(n.original.b),v14_sha256=n.sha(n.modified.b),rows=rows,instructions=n.instructions(pins),native_executions=2*len(rows))
 p=HERE/'native-view-task-queue-final.json'
 if '--write'in sys.argv:
  with p.open('x')as f:json.dump(doc,f,indent=2);f.write('\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps(dict(rows=len(rows),executions=2*len(rows),pins=len(pins),sha256=n.sha(p.read_bytes()))))
if __name__=='__main__':main()
