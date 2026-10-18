"""Original486870/486980 Bone Data+C producer, successful unique-ID domain.
The empty Find, string transport/factory and hash are explicit seams. Actual
allocation, constructor+C=-1, index store,93D4E0 append and47B400 run unchanged.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE));import native_fx_graph as graph
nm=graph.nm;ORIGINAL=nm.original
def run(index,image,pc):
 m=nm.Machine(pc)
 if image=='v14':
  old=(ORIGINAL.image_size+0xfff)&~0xfff;new=(nm.modified.image_size+0xfff)&~0xfff
  if new>old:m.u.mem_map(ORIGINAL.base+old,new-old)
  for vs,rva,raw,off in nm.modified.sections:m.u.mem_write(nm.modified.base+rva,nm.modified.b[off:off+raw])
 manager=m.alloc(0x50);m.w(manager+8,0);storage=m.alloc((index+2)*4);m.w(manager+0x24,storage);m.w(manager+0x28,index);m.w(manager+0x2c,index+2)
 m.w(manager+0x34,31);m.wb(manager+0x3c,0)
 text=m.alloc(64);m.w(text+12,1000000);m.u.mem_write(text+16,'head\0'.encode('utf-16-le'));ids=m.array([text+16])
 factory=m.alloc(16);vt=m.alloc(16);empty=m.alloc(32);m.w(empty+12,1000000);m.w(factory,vt)
 m.w(vt+12,m.stub(lambda mm:mm.ret(empty)))
 m.seam(0x856370,lambda mm:mm.ret(factory))
 m.seam(0x4012c0,lambda mm:mm.ret(mm.arg(1)))
 m.seam(0x404850,lambda mm:mm.ret(mm.ecx(),4))
 m.seam(0x4915f0,lambda mm:mm.ret(0,pop=4))
 m.seam(0x40d720,lambda mm:mm.ret(0,pop=4))
 m.seam(0x6f7810,lambda mm:mm.ret(m.alloc(mm.arg(1))))
 #Fresh486650/47D880 manager proves count0/mode0. The unused one-element
 #string reserve lifecycle is transport; it never supplies Data+C values.
 fresh=m.alloc(0x50)
 m.seam(0x96d18d,lambda mm:mm.ret(pop=20))
 m.call(0x486650,fresh,[17,0]);assert m.r(fresh+8)==0 and m.r(fresh+0x28)==0 and m.r(fresh)==0xa0d3b4
 p=m.call(0x486870,manager,[ids]);assert m.r(p+8)==text+16
 assert m.r(storage+index*4)==p and m.r(manager+0x28)==index+1
 assert m.r(m.r(m.r(manager+0x30)))==p and m.r(manager+0x38)==1
 return dict(ordinal=m.r(p+12),count=m.r(manager+0x28),appended_same_allocation=True,registered_same_allocation=True),m.visited
def main():
 rows=[];pins={}
 for pc,index in itertools.product(nm.PRECISIONS,[0,1,3,16,64,1000]):
  a,pa=run(index,'original',pc);b,pb=run(index,'v14',pc);assert a==b;pins.update(pa);pins.update(pb);rows.append(dict(precision=pc,index=index,output=a))
 for va,raw in pins.items():assert ORIGINAL.range(va,len(raw))==nm.modified.range(va,len(raw))==raw
 doc=dict(schema=1,scope=__doc__,rows=rows,native_executions=len(rows)*2,instructions=[dict(va=hex(va),bytes=raw.hex()) for va,raw in sorted(pins.items())])
 path=HERE/'native-bone-ordinal.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=doc['native_executions'],pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
