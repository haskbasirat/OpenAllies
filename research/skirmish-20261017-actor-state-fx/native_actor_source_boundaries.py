"""Actor event enum, completed Enter-list validation and Enter->Leave copy callsite.
Bounded native instruction islands execute with explicit frame/register input;
the copy routine is an observed required service, not a claimed Leave compiler.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
from unicorn.x86_const import UC_X86_REG_EBP,UC_X86_REG_EDI,UC_X86_REG_EIP,UC_X86_REG_ESP
class Boundary(Exception):pass
def machine(image,pc):
 m=nm.Machine(pc)
 if image=='v14':
  old=(nm.original.image_size+0xfff)&~0xfff;new=(nm.modified.image_size+0xfff)&~0xfff
  if new>old:m.u.mem_map(nm.original.base+old,new-old)
  for vs,rva,raw,off in nm.modified.sections:m.u.mem_write(nm.modified.base+rva,nm.modified.b[off:off+raw])
 return m
def run(c,image,pc):
 m=machine(image,pc);events=[]
 if c['kind']=='enum':
  def text(p):
   b=b''
   while bytes(m.u.mem_read(p,2))!=b'\0\0':b+=bytes(m.u.mem_read(p,2));p+=2
   return b.decode('utf-16-le')
  def insert(mm):
   assert mm.arg(3)==0;events.append([mm.arg(1),text(mm.arg(2))]);mm.ret(pop=12)
  m.seam(0x4976b0,insert);m.seam(0x6f7810,lambda mm:mm.ret(m.alloc(mm.arg(1))))
  m.call(0x4985e0,m.alloc(36),[1]);assert events==[[0,'WhenEnter'],[1,'WhenLeave']]
  output=dict(values=events)
 elif c['kind']=='copy':
  data=m.alloc(0x500);m.observe(0x464ff2,lambda mm:mm.u.reg_write(UC_X86_REG_EBP,data))
  def copy(mm):
   assert [mm.arg(i) for i in [1,2,3]]==[data+0x194,data+0x19c,0]
   events.append(dict(source='ActorData+194 WhenEnter',destination='ActorData+19C WhenLeave',flag=0));mm.ret()
  m.seam(0x42b8e0,copy);m.observe(0x465007,lambda mm:(_ for _ in ()).throw(Boundary()))
  try:m.call(0x464ff2,0,[]);raise AssertionError('missing cutoff')
  except Boundary:pass
  assert m.u.reg_read(UC_X86_REG_EIP)==0x465007
  output=dict(events=events,stack_delta=-12)
 else:
  data=m.alloc(0x500);m.w(data+8,0x11223344);head=0
  for state in reversed(c['states']):
   node=m.alloc(0x40);m.w(node+0x34,state);head=m.array([node,head])
  pair=m.array([head,0]);m.observe(0x464fc1,lambda mm:(mm.u.reg_write(UC_X86_REG_EBP,data),mm.u.reg_write(UC_X86_REG_EDI,pair)))
  def diagnostic(mm):
   assert mm.arg(1)==0xa09760 and mm.arg(2)==0x11223344
   events.append('requires concrete state42');raise Boundary()
  m.seam(0x410c40,diagnostic);m.observe(0x464fe7,lambda mm:(_ for _ in ()).throw(Boundary()))
  try:m.call(0x464fc1,0,[]);raise AssertionError('missing cutoff')
  except Boundary:pass
  output=dict(rejected=42 in c['states'],events=events,stop=hex(m.u.reg_read(UC_X86_REG_EIP)))
  assert output['rejected']==bool(events)
 assert not m.null_accesses
 return output,m.visited
def main():
 cases=[dict(kind='enum'),dict(kind='copy')]+[dict(kind='validate',states=s)for s in [[],[0],[41],[0,41],[42],[0,42],[42,0],[1,42,2]]]
 rows=[];pins={}
 for pc,c in itertools.product(nm.PRECISIONS,cases):
  a,pa=run(c,'original',pc);b,pb=run(c,'v14',pc);assert a==b;pins.update(pa);pins.update(pb);rows.append(dict(precision=pc,input=c,output=a))
 for va,raw in pins.items():assert nm.original.range(va,len(raw))==nm.modified.range(va,len(raw))==raw
 # Fresh local reference-map initialization precedes the Enter compiler and
 # is not evidence of sharing ObjectData's earlier View reference context.
 slices=[(0x464f18,0x81),(0x464f99,0x28),(0x464fc1,0x46),(0x45eca0,0x14)]
 raw=[]
 for va,length in slices:
  a=nm.original.range(va,length);assert a==nm.modified.range(va,length);raw.append(dict(va=hex(va),bytes=a.hex()))
 doc=dict(schema=1,scope=__doc__,rows=rows,native_executions=len(rows)*2,instructions=[dict(va=hex(v),bytes=b.hex())for v,b in sorted(pins.items())],static_slices=raw)
 path=HERE/'native-actor-source-boundaries.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=len(rows)*2,pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
