"""Bounded4E6379..4E63AE actual TechnologyProperty consumer in4E6180.
Reached structure branch, immutable source list and concrete actor/kingdom
registers/frame are explicit inputs. Real45ECA0 and traversal execute;4E3AD0
is a required observable service, with stopped-call prefixes retained.
"""
from pathlib import Path
import json,sys,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
from unicorn.x86_const import UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EIP
class Cutoff(Exception):pass
def run(c,image,pc):
 m=nm.Machine(pc)
 if image=='v14':
  old=(nm.original.image_size+0xfff)&~0xfff;new=(nm.modified.image_size+0xfff)&~0xfff
  if new>old:m.u.mem_map(nm.original.base+old,new-old)
  for vs,rva,raw,off in nm.modified.sections:m.u.mem_write(nm.modified.base+rva,nm.modified.b[off:off+raw])
 data=m.alloc(0x500);actor=m.alloc(0x200);kingdom=m.alloc(0x400);m.w(actor+4,data);head=0;tail=0
 for value in reversed(c['values']):
  link=m.array([value,head]);head=link
  if not tail:tail=link
 m.w(data+0x43c,head);m.w(data+0x440,tail);events=[]
 m.observe(0x4e6379,lambda mm:(mm.u.reg_write(UC_X86_REG_ESI,actor),mm.u.reg_write(UC_X86_REG_EDI,kingdom)))
 def add(mm):
  assert mm.ecx()==kingdom and mm.arg(2)==10 and mm.arg(3)==actor and mm.arg(4)==1
  events.append(dict(property=mm.arg(1),kind=10,current_actor=True,flag=1))
  if c['stop']==len(events)-1:raise Cutoff()
  mm.ret(pop=16)
 m.seam(0x4e3ad0,add);m.observe(0x4e63ae,lambda mm:(_ for _ in ()).throw(Cutoff()))
 try:m.call(0x4e6379,0,[]);raise AssertionError('no cutoff')
 except Cutoff:pass
 assert m.r(data+0x43c)==head and m.r(data+0x440)==tail
 return dict(events=events,completed=m.u.reg_read(UC_X86_REG_EIP)==0x4e63ae,source_head_and_tail_unchanged=True),m.visited
def main():
 rows=[];pins={}
 for pc,values,stop in itertools.product(nm.PRECISIONS,[[],[11],[33,22,11],[22,11,11]],[-1,0,1,2]):
  c=dict(values=values,stop=stop);a,pa=run(c,'original',pc);b,pb=run(c,'v14',pc);assert a==b;pins.update(pa);pins.update(pb);rows.append(dict(precision=pc,input=c,output=a))
 for va,raw in pins.items():assert nm.original.range(va,len(raw))==nm.modified.range(va,len(raw))==raw
 #4624E1 is the only EBX writer before43C/440 ctor stores, with callee-save
 #EBX obligations at calls. This is static instruction provenance, not a
 #claim that the complete ActorData constructor's services execute here.
 slices=[]
 for va,length in [(0x4624e1,0x662),(0x46797f,0x28)]:
  raw=nm.original.range(va,length);assert raw==nm.modified.range(va,length);slices.append(dict(va=hex(va),bytes=raw.hex()))
 doc=dict(schema=1,scope=__doc__,rows=rows,native_executions=len(rows)*2,instructions=[dict(va=hex(v),bytes=b.hex(),observed_cutoff=v==0x4e63ae)for v,b in sorted(pins.items())],static_slices=slices)
 path=HERE/'native-technology-use.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=len(rows)*2,pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
