"""Native FXC497A40, AT497E20 and HealthTrigger42BF70 registrations.
Insertion/string reserve allocation are explicit services; actual registering
callers and health enum header constructor execute in both images/all PC.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE));import native_fx_graph as graph
nm=graph.nm;ORIGINAL=nm.original
def run(kind,image,pc):
 m=nm.Machine(pc)
 if image=='v14':
  old=(ORIGINAL.image_size+0xfff)&~0xfff;new=(nm.modified.image_size+0xfff)&~0xfff
  if new>old:m.u.mem_map(ORIGINAL.base+old,new-old)
  for vs,rva,raw,off in nm.modified.sections:m.u.mem_write(nm.modified.base+rva,nm.modified.b[off:off+raw])
 values=[]
 def text(p):
  b=b''
  while bytes(m.u.mem_read(p,2))!=b'\0\0':b+=bytes(m.u.mem_read(p,2));p+=2
  return b.decode('utf-16-le')
 def insert(mm):
  assert mm.arg(3)==0;values.append([mm.arg(1),text(mm.arg(2))]);mm.ret(pop=12)
 m.seam(0x4976b0,insert);m.seam(0x42bb30,insert);m.seam(0x6f7810,lambda mm:mm.ret(m.alloc(mm.arg(1))))
 m.call({'FXC':0x497a40,'AT':0x497e20,'HealthTrigger':0x42bf70}[kind],m.alloc(36),[1] if kind=='HealthTrigger' else [])
 assert [v[0] for v in values]==list(range(len(values)))
 assert [v[1] for v in values]=={'FXC':graph.FXC,'AT':graph.AT,'HealthTrigger':graph.HEALTH}[kind]
 return values,m.visited
def main():
 rows=[];pins={}
 for pc,kind in itertools.product(nm.PRECISIONS,['FXC','AT','HealthTrigger']):
  a,pa=run(kind,'original',pc);b,pb=run(kind,'v14',pc);assert a==b;pins.update(pa);pins.update(pb);rows.append(dict(precision=pc,domain=kind,values=a))
 for va,raw in pins.items():assert ORIGINAL.range(va,len(raw))==nm.modified.range(va,len(raw))==raw
 doc=dict(schema=1,scope=__doc__,rows=rows,native_executions=len(rows)*2,instructions=[dict(va=hex(va),bytes=raw.hex()) for va,raw in sorted(pins.items())])
 path=HERE/'native-fx-enums.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=doc['native_executions'],pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
