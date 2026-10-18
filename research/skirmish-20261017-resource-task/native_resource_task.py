"""Real ResourceTask5B1850 fields and native queue/list/refcount operations.

Real5AED10/5AE160/5ADBE0/5B15B0,5B1F50/5B1ED0/5B1E70/5B1DF0,
5AED90/5AE0E0/5AD5B0 execute. Declared host transports: heap alloc/free,
atomic increment/decrement imports, synchronization lock/unlock/signal,
conditional logger. No task Run, decoder, GPU, waiter or ready substitute.
"""
from pathlib import Path
import argparse,hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'))
from native_machine import Machine,original,modified
sha=lambda b:hashlib.sha256(b).hexdigest()

def cases():
 out=[]
 def add(name,ops,sequence=0):out.append(dict(name=name,sequence=sequence,ops=ops))
 add('construct',[])
 add('equal-priority-sequences',[['enqueue',0],['enqueue',1],['enqueue',2],['enqueue',3]])
 add('mixed-priorities',[['priority',0,3],['priority',1,0],['priority',2,2],['priority',3,1],['enqueue',0],['enqueue',1],['enqueue',2],['enqueue',3]])
 add('signed-priorities',[['priority',0,0x80000000],['priority',1,0xffffffff],['priority',2,0x7fffffff],['enqueue',0],['enqueue',1],['enqueue',2],['enqueue',3]])
 add('reprioritize',[['enqueue',0],['enqueue',1],['enqueue',2],['priority',0,2],['priority',0,2],['priority',1,0],['priority',0,1]])
 add('remove',[['enqueue',0],['enqueue',1],['enqueue',2],['remove',1],['remove',1],['remove',2],['remove',0]])
 add('remove-empty',[['remove',0],['remove',1]])
 add('duplicate',[['enqueue',0],['enqueue',1],['enqueue',0],['remove',0],['priority',0,0],['remove',0],['remove',0]])
 add('duplicate-reprioritize',[['enqueue',0],['enqueue',0],['enqueue',1],['priority',0,2],['remove',0],['enqueue',0]])
 add('sequence-wrap',[['enqueue',0],['enqueue',1],['enqueue',2],['enqueue',3]],0xfffffffe)
 add('sequence-repeat-zero',[['enqueue',0],['enqueue',0],['enqueue',1],['enqueue',0]])
 add('all-roundtrip',[['enqueue',0],['enqueue',1],['priority',0,3],['remove',0],['enqueue',0],['remove',1],['enqueue',2],['priority',2,0xffffffff],['remove',2],['remove',0]])
 return out

def run(case,image,pc):
 m=Machine(pc)
 old=(original.image_size+4095)&~4095;new=(image.image_size+4095)&~4095
 if new>old:m.u.mem_map(image.base+old,new-old)
 for _,rva,size,offset in image.sections:m.u.mem_write(image.base+rva,image.b[offset:offset+size])
 manager=m.alloc(0x140);queue=manager+0xfc
 vt=m.alloc(16);lock=m.stub(lambda _:m.ret());unlock=m.stub(lambda _:m.ret())
 m.w(vt+4,lock);m.w(vt+8,unlock);m.w(manager+0x20,vt);m.w(queue,vt)
 m.w(0xb3da08,case['sequence'])
 calls=[]
 for address in [0x857e10,0x857de0,0x8573a0]:m.seam(address,lambda _:m.ret())
 def allocate(_):
  size=m.arg(1);assert size==12,size
  address=m.alloc(size);m.u.mem_write(address,b'\xa5'*size);m.ret(address)
 m.seam(0x6f7810,allocate);m.seam(0x6f7430,lambda _:m.ret())
 def atomic(delta):
  def apply(_):
   address=m.arg(1);value=(m.r(address)+delta)&0xffffffff;m.w(address,value);m.ret(value,4)
  return apply
 m.w(0x9f01e8,m.stub(atomic(1)));m.w(0x9f01ec,m.stub(atomic(-1)))
 m.seam(0x404f90,lambda _:m.ret())
 resources=[];tasks=[]
 for index in range(4):
  resource=m.alloc(0x50);task=m.alloc(0x20);m.u.mem_write(task,b'\xa5'*0x20)
  name=m.array([0x00410041+index,0]);m.w(resource+0x38,name)
  m.call(0x5b1850,task,[resource,m.r(0xb282f0)])
  assert m.r(task)==0xa47790
  #Actual separate caller InterlockedIncrement at5AA944; same import transport.
  m.w(task+4,(m.r(task+4)+1)&0xffffffff)
  tasks.append(task);resources.append(resource)
 def snapshot(result=None):
  ids={task:i for i,task in enumerate(tasks)};order=[];node=m.r(queue+0x24);prior=0
  while node:
   assert m.r(node+4)==prior
   order.append(ids[m.r(node)]);prior=node;node=m.r(node+8)
   assert len(order)<100
  assert m.r(queue+0x28)==prior
  return dict(order=order,sequence=m.r(0xb3da08),tasks=[dict(refs=m.r(t+4),priority=m.r(t+8),sequence=m.r(t+0xc),queued=m.r(t+0x10)!=0,state=m.r(t+0x14),thread=m.r(t+0x18),resource_index=resources.index(m.r(t+0x1c)))for t in tasks],removed=result)
 snapshots=[snapshot()]
 for op in case['ops']:
  task=tasks[op[1]];removed=None
  if op[0]=='enqueue':m.call(0x5aed10,manager,[task])
  elif op[0]=='priority':m.call(0x5b1f50,task,[op[2]])
  elif op[0]=='remove':removed=bool(m.call(0x5aed90,manager,[task])&0xff)
  else:raise AssertionError(op)
  snapshots.append(snapshot(removed))
 return snapshots,m.visited

def main():
 p=argparse.ArgumentParser();p.add_argument('--create',action='store_true');a=p.parse_args()
 assert original.range(0xb282f0,4)==modified.range(0xb282f0,4)==struct.pack('<I',1)
 rows=[];pins={}
 for pc in ['Single24','Double53','Extended64']:
  for case in cases():
   outputs=[]
   for image in [original,modified]:
    out,visited=run(case,image,pc);outputs.append(out)
    for address,raw in visited.items():
     assert raw==original.range(address,len(raw))==modified.range(address,len(raw)),hex(address)
     pins[address]=raw
   assert outputs[0]==outputs[1],(case,pc)
   rows.append(dict(precision=pc,input=case,native=outputs[0]))
 out=dict(scope=__doc__,original_sha256=sha(original.b),v14_sha256=sha(modified.b),default_priority=dict(address='0xb282f0',bytes=original.range(0xb282f0,4).hex()),rows=rows,pins=[dict(address=hex(x),bytes=b.hex())for x,b in sorted(pins.items())])
 path=HERE/'native-resource-task.json'
 if a.create:
  with path.open('x',encoding='utf-8')as f:json.dump(out,f,indent=2);f.write('\n')
 else:assert out==json.loads(path.read_text(encoding='utf-8'))
 print(json.dumps(dict(paired_rows=len(rows),separate_executions=len(rows)*2,pins=len(pins),fixture_sha256=sha(path.read_bytes()))))
if __name__=='__main__':main()
