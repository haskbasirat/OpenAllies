"""85FA70 MainThread+48 current OS-ID capture; base85F9B0 is a named seam.
Actual singleton publication, current-ID store and current-handle store execute.
This does not claim a resource worker has started or its thread ID is fixed.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
ORIGINAL=nm.original
def run(identity,handle,image,pc):
 m=nm.Machine(pc)
 if image=='v14':
  old=(ORIGINAL.image_size+0xfff)&~0xfff;new=(nm.modified.image_size+0xfff)&~0xfff
  if new>old:m.u.mem_map(ORIGINAL.base+old,new-old)
  for vs,rva,raw,off in nm.modified.sections:m.u.mem_write(nm.modified.base+rva,nm.modified.b[off:off+raw])
 owner=m.alloc(0xb0);m.u.mem_write(owner,b'\xcd'*0xb0);events=[]
 m.seam(0x85f9b0,lambda mm:(events.append('base'),mm.ret(owner,pop=4)))
 def current(mm):
  assert m.r(0xb364d4)==owner and m.r(owner+0x48)==0xcdcdcdcd
  events.append('current_id');mm.ret(identity)
 def current_handle(mm):
  assert m.r(owner+0x48)==identity;events.append('current_handle');mm.ret(handle)
 m.w(0x9f01e4,m.stub(current));m.w(0x9f00ec,m.stub(current_handle))
 assert m.call(0x85fa70,owner,[0x12345678])==owner
 return dict(identity=m.r(owner+0x48),handle=m.r(owner+0x44),singleton_is_same_owner=m.r(0xb364d4)==owner,events=events),m.visited
def main():
 rows=[];pins={}
 for pc,identity,handle in itertools.product(nm.PRECISIONS,[1,10708,44416,0xfeed1234],[0xfffffffe,0x13572468]):
  a,pa=run(identity,handle,'original',pc);b,pb=run(identity,handle,'v14',pc);assert a==b;pins.update(pa);pins.update(pb)
  rows.append(dict(precision=pc,input=dict(identity=identity,handle=handle),output=a))
 for va,raw in pins.items():assert ORIGINAL.range(va,len(raw))==nm.modified.range(va,len(raw))==raw
 doc=dict(schema=1,scope=__doc__,rows=rows,native_executions=len(rows)*2,instructions=[dict(va=hex(va),bytes=raw.hex()) for va,raw in sorted(pins.items())])
 path=HERE/'native-execution-context.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=doc['native_executions'],pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
