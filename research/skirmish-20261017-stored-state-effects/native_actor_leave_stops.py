"""Unchanged42B8E0/42B3B0/42AF90: ActorLeave's recursive StopVFX synthesis.
Successful heap and debug logger are explicit services; real node constructors,
vtable predicates, child traversal, copied fields, prepends/backlinks execute.
VFXData+24 is an explicit typed input, not a source-loading completion claim.
"""
from pathlib import Path
import sys,json,itertools,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
def node(kind='create',attached=True,duration=False,permanent=True,trigger=0,state=12,children=None,chain=None):
 return dict(kind=kind,attached=attached,duration=duration,permanent=permanent,trigger=trigger,state=state,immediate=children or [],chain=chain or [])
def cases():
 out=[]
 for kind,attached,duration,permanent,trigger in itertools.product(('create','play','stop'),(False,True),(False,True),(False,True),(0,1,2,3,0xffffffff)):
  out.append(dict(nodes=[node(kind,attached,duration,permanent,trigger)],roots=[0],leave=[]))
 out.extend([
  dict(nodes=[node(children=[1,2],chain=[3]),node(state=8),node(trigger=1,state=21),node(trigger=2,state=24)],roots=[0,2],leave=[1]),
  dict(nodes=[node(children=[1]),node(children=[2]),node(trigger=1)],roots=[0],leave=[]),
 ])
 return out
def run(c,image,pc):
 m=nm.Machine(pc)
 if image=='v14':
  old=(nm.original.image_size+4095)&~4095;new=(nm.modified.image_size+4095)&~4095
  if new>old:m.u.mem_map(nm.original.base+old,new-old)
  for vs,r,size,off in nm.modified.sections:
   if size:m.u.mem_write(nm.modified.base+r,nm.modified.b[off:off+size])
 allocated=[];trace=[]
 def allocate(mm):
  p=m.alloc(0x6c);m.u.mem_write(p,b'\xcd'*0x6c);allocated.append(p);mm.ret(p)
 m.seam(0x85c680,allocate)
 m.seam(0x6f7810,lambda mm:mm.ret(m.alloc(mm.arg(1))))
 m.seam(0x404f90,lambda mm:mm.ret())
 inputs=[]
 for i,n in enumerate(c['nodes']):
  p=m.alloc(0x6c);m.u.mem_write(p,b'\xcd'*0x6c);inputs.append(p)
  m.call({'create':0x42aef0,'play':0x42add0,'stop':0x42af90}[n['kind']],p)
  for off,value in [(0x18,0x100+i),(0x1c,0x200+i),(0x24,3+i),(0x28,0x3e800000),(0x2c,0xbf800000),(0x30,n['trigger']),(0x34,n['state'])]:m.w(p+off,value)
  m.w(p+0x20,0)
  if n['kind']=='create':
   vfx=m.alloc(0x40);m.w(vfx+0x24,int(n['permanent']));m.w(p+0x3c,vfx);m.wb(p+0x64,int(n['duration']));m.wb(p+0x65,int(n['attached']))
 def linked(pair,values):
  head=0;tail=0
  for value in reversed(values):
   item=m.alloc(8);m.w(item,value);m.w(item+4,head);head=item
   if not tail:tail=item
  m.w(pair,head);m.w(pair+4,tail)
 for p,n in zip(inputs,c['nodes']):
  linked(p+8,[inputs[i]for i in n['immediate']]);linked(p+16,[inputs[i]for i in n['chain']])
 source=m.alloc(8);destination=m.alloc(8);linked(source,[inputs[i]for i in c['roots']]);linked(destination,[inputs[i]for i in c['leave']])
 m.call(0x42b8e0,0,[source,destination,0],pop=0)
 all_nodes=inputs+allocated
 def index(p):return all_nodes.index(p)if p in all_nodes else None
 order=[];cursor=m.r(destination);tail=0
 while cursor:order.append(index(m.r(cursor)));tail=cursor;cursor=m.r(cursor+4)
 assert m.r(destination+4)==tail
 result=dict(roots=order,generated=[] ,backlinks=[index(m.r(p+0x68))if n['kind']=='create'else None for p,n in zip(inputs,c['nodes'])])
 for p in allocated:
  result['generated'].append(dict(words={str(o):m.r(p+o)for o in(0x18,0x1c,0x24,0x28,0x2c,0x30,0x34)},target=index(m.r(p+0x3c)),terrain=m.r(p+0x20),flag=m.rb(p+0x38),immediate=m.r(p+8),chain=m.r(p+16)))
 return result,m.visited
def main():
 rows=[];pins={}
 for pc in nm.PRECISIONS:
  for c in cases():
   pair=[]
   for image in('original','v14'):
    output,visited=run(c,image,pc);pair.append(output);pins.update(visited)
   assert pair[0]==pair[1];rows.append(dict(precision=pc,input=c,output=pair[0]))
 for a,b in pins.items():assert b==nm.original.range(a,len(b))==nm.modified.range(a,len(b))
 doc={**nm.header(__file__,__doc__),'rows':rows,'instructions':nm.instructions(pins),'harness_sha256':hashlib.sha256((ROOT/'research/phase8-ai/native_machine.py').read_bytes()).hexdigest()}
 p=HERE/'native-actor-leave-stops.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=1))
 else:assert json.loads(p.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=2*len(rows),pins=len(pins),sha256=hashlib.sha256(p.read_bytes()).hexdigest())))
if __name__=='__main__':main()
