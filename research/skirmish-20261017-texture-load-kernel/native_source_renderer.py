"""Actual87BA60 Source renderer gate plus real87BBF0 discard/refcount prefix.
GRenderer/Source/policy fields are explicit controlled fixture inputs. Renderer
virtual100 and Pixel destructor virtual0 are named children, not GPU proof.
Native re-reads post-renderer Source2C and lazily short-circuits discard gates.
"""
from pathlib import Path
import sys,json,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as n
from unicorn import UC_HOOK_MEM_READ,UC_HOOK_MEM_WRITE
class Required(Exception):pass
def run(c,image,pc):
 m=n.Machine(pc)
 if image=='v14':
  old=(n.original.image_size+0xfff)&~0xfff;new=(n.modified.image_size+0xfff)&~0xfff
  if new>old:m.u.mem_map(n.original.base+old,new-old)
  for vs,rva,raw,off in n.modified.sections:m.u.mem_write(n.modified.base+rva,n.modified.b[off:off+raw])
 source=m.alloc(0x50);pixel=m.alloc(8);renderer=m.alloc(4);rvt=m.alloc(0x104);pvt=m.alloc(4);events=[]
 m.w(source,0xab8fe8);m.w(source+0x2c,1 if c['data']else 0);m.wb(source+0x44,c['discard']);m.w(source+0x40,pixel if c['pixels']else 0);m.w(pixel,pvt);m.w(pixel+4,c['refs'])
 m.w(renderer,rvt);m.w(0xb442c0,renderer if c['renderer']else 0);m.wb(0xb43f20,c['policy'])
 def child(mm):
  assert mm.arg(1)==source;events.append(['create'])
  if c['fail']=='create':raise Required('create')
  if c['install'] is not None:mm.w(source+0x2c,int(c['install']))
  mm.ret(int(c['result']),4)
 m.w(rvt+0x100,m.stub(child))
 def destroy(mm):
  assert mm.ecx()==pixel and mm.arg(1)==1;events.append(['destroy'])
  if c['fail']=='destroy':raise Required('destroy')
  mm.ret(0,4)
 m.w(pvt,m.stub(destroy))
 def read(u,access,a,size,value,_):
  if a in [0xb442c0,0xb43f20,source+0x2c,source+0x44,source+0x40]:
   kind={0xb442c0:'renderer',0xb43f20:'policy',source+0x2c:'data',source+0x44:'discard',source+0x40:'pixels'}[a]
   raw=m.rb(a)if size==1 else m.r(a);events.append(['read',kind,raw if kind in ['policy','discard']else int(raw!=0)])
 def write(u,access,a,size,value,_):
  if a==pixel+4:events.append(['refs',value])
  if a==source+0x40:events.append(['clear',value])
 m.u.hook_add(UC_HOOK_MEM_READ,read);m.u.hook_add(UC_HOOK_MEM_WRITE,write)
 stopped=None;result=None
 try:result=m.call(0x87ba60,source)&255
 except Required as e:stopped=str(e)
 return dict(events=events,result=result,stopped=stopped,pixel_refs=m.r(pixel+4),pixels=m.r(source+0x40)!=0,data=m.r(source+0x2c)!=0),m.visited
def main():
 cases=[dict(renderer=r,result=result,policy=policy,data=data,discard=discard,pixels=pixels,refs=2,install=None,fail=None)
  for r,result,policy,data,discard,pixels in itertools.product([False,True],[False,True],[0,1,2],[False,True],[0,1,2],[False,True])]
 for install in [False,True]:cases.append(dict(renderer=True,result=True,policy=1,data=not install,discard=1,pixels=True,refs=2,install=install,fail=None))
 for fail in ['create','destroy']:cases.append(dict(renderer=True,result=True,policy=1,data=True,discard=1,pixels=True,refs=1,install=None,fail=fail))
 rows=[];pins={}
 for pc,c in itertools.product(n.PRECISIONS,cases):
  a,pa=run(c,'original',pc);b,pb=run(c,'v14',pc);assert a==b,(pc,c,a,b);pins.update(pa);pins.update(pb);rows.append(dict(precision=pc,input=c,output=a))
 for va,raw in pins.items():assert n.original.range(va,len(raw))==n.modified.range(va,len(raw))==raw
 doc=dict(schema=1,scope=__doc__,original_sha256=n.sha(n.original.b),v14_sha256=n.sha(n.modified.b),rows=rows,native_executions=len(rows)*2,instructions=n.instructions(pins))
 path=HERE/'native-source-renderer.json'
 if '--write'in sys.argv:
  with path.open('x')as f:json.dump(doc,f,indent=2);f.write('\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,pins=len(pins),sha256=n.sha(path.read_bytes()))))
if __name__=='__main__':main()
