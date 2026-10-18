"""Actual86EAB0 propagation over the real initialized8C3ED0 defaults.

The bounded twelve-property producer from native_model_defaults executes first.
Native Node/Geometry property traversal, state-copy/alias and enum replacement
then execute over explicit native-layout source trees. Source material objects
use real87D280 constructors. OS thread/lock and allocator calls are declared
transports; no NiStream loader, GPU readiness or arbitrary later writer claim.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE));import native_model_defaults as d
sys.path.insert(0,str(HERE.parents[1]/'skirmish-20261017-model-resource-load'))
import native_model_prelude as p

class Machine(d.Machine):
 def __init__(self,image):
  super().__init__(image);self.os_stubs={};self.events=[]
  for i,name in enumerate(['GetCurrentThreadId','EnterCriticalSection','LeaveCriticalSection']):
   at=self.alloc(16);self.os_stubs[at]=name;self.w(p.imported(d.PE0,name),at)
  for descriptor in [0xb43ce8,0xb45f6c]:self.init_type(descriptor)
 def hook(self,u,pc,size,user):
  if pc in getattr(self,'os_stubs',{}):
   name=self.os_stubs[pc];self.events.append(name)
   if name=='GetCurrentThreadId':self.ret(value=0x1234)
   else:self.ret(4)
   return
  super().hook(u,pc,size,user)

def main():
 rows=[];pins={}
 for image in ['original','v14']:
  for precision in ['Single','Double','Extended']:
   m=Machine(image);m.fpu(precision);m.allocations={};m.mask={};m.actual={};m.record_actual=True
   m.nonorder.discard(0x9e7850);m.call(0x9e7850);m.armed=True
   try:m.call(0x8c3ed0)
   except d.Stop:pass
   else:raise AssertionError('default prefix missing boundary')
   m.armed=False;defaults=[m.r(g)for g in d.GLOBALS];assert all(defaults)
   for override in ['none','root','child','both','shared']:
    case=[dict(name='root',children=[1,None,2]),dict(name='first',kind='shape',parent=0),dict(name='branch',parent=0,children=[3,4]),dict(name='grandchild',kind='shape',parent=2),dict(name='second',kind='shape',parent=2)]
    points=p.make_tree(m,case);source=[]
    def add(node,which):
     if which==len(source):
      pointer=m.alloc(100);m.call(0x87d280,this=pointer);m.w(pointer+4,100);source.append(pointer)
     entry=m.alloc(12);m.w(entry+8,source[which]);m.w(points[node]+0xa4,entry);m.w(points[node]+0xa8,entry);m.w(points[node]+0xac,1)
    if override in ['root','both','shared']:add(0,0)
    if override=='child':add(3,0)
    if override=='both':add(3,1)
    if override=='shared':add(3,0)
    m.call(0x86eab0,this=points[0]);m.call(0x86f090,this=points[0])
    owners={pointer:f'd{index}'for index,pointer in enumerate(defaults)};owners.update({pointer:f's{index}'for index,pointer in enumerate(source)})
    states=[m.r(point+0xc4)for point in points];assert states[0]==states[2]==0
    logical=[]
    for state in states:
     logical.append(None if state==0 else[owners[m.r(state+8+4*i)]for i in range(12)])
    aliases=[[a,b]for a in range(len(states))for b in range(a+1,len(states))if states[a]and states[a]==states[b]]
    assert all(logical[i]and len(logical[i])==12 for i in [1,3,4]);assert m.r(0xb45f18)==0
    rows.append(dict(image=image,precision=precision,override=override,states=logical,aliases=aliases,effects=[m.r(point+0xc8)if False else bool(m.r(point+0xd4))for point in []]))
   pins.update(m.actual)
 assert[{k:v for k,v in r.items()if k!='image'}for r in rows[:15]]==[{k:v for k,v in r.items()if k!='image'}for r in rows[15:]]
 instructions=[]
 for address,size in sorted(pins.items()):
  if d.PE0.base<=address<d.PE0.base+d.PE0.image_size:
   raw=d.PE0.range(address,size);assert raw==d.PE1.range(address,size);instructions.append(dict(va=hex(address),bytes=raw.hex()))
 doc=dict(scope=__doc__,rows=rows,instructions=instructions,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
 path=HERE/'native-model-property-state.json'
 if '--create'in sys.argv:path.open('x').write(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_bytes())==doc
 print(json.dumps(dict(rows=len(rows),pins=len(instructions),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
