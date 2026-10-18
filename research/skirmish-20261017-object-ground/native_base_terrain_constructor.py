"""49C760 proper constructor prefix, ending after both BaseTerrain arrays.
ThingData base constructor and empty CString factory are explicit services;
real shape constructors, common SEH/register prologue and ObjectData scalar/
pointer/count/capacity stores execute.49C813 is not an executed instruction.
"""
from pathlib import Path
import sys,json,hashlib
P=Path(__file__).resolve().parent;R=P.parents[1]
sys.path.insert(0,str(R/'research/phase8-ai'));import native_machine as nm
class Cut(Exception):pass
def run(poison,image,pc):
 m=nm.Machine(pc)
 if image=='v14':
  old=(nm.original.image_size+4095)&~4095;new=(nm.modified.image_size+4095)&~4095
  if new>old:m.u.mem_map(nm.original.base+old,new-old)
  for _,rva,raw,off in nm.modified.sections:m.u.mem_write(nm.modified.base+rva,nm.modified.b[off:off+raw])
 data=m.alloc(0x700);m.u.mem_write(data,bytes([poison])*0x700)
 factory=m.alloc(16);vtable=m.alloc(32);m.w(factory,vtable)
 m.w(vtable+0xc,m.stub(lambda mm:mm.ret(m.alloc(0x20))))
 m.seam(0x4bbe70,lambda mm:mm.ret(data,8));m.seam(0x856370,lambda mm:mm.ret(factory))
 m.seam(0x49c813,lambda mm:(_ for _ in ()).throw(Cut()))
 try:m.call(0x49c760,data,[0,0])
 except Cut:pass
 fields={hex(o):m.r(data+o)for o in [0x8c,0x90,0x94,0x98,0x9c,0xa0,0xa4]}
 assert fields=={'0x8c':0,'0x90':0,'0x94':0,'0x98':1,'0x9c':0,'0xa0':0,'0xa4':1}
 assert not m.null_accesses;m.verify_v14();return fields,m.visited
def main():
 rows=[];pins={}
 for poison in [0,0x55,0xaa]:
  for pc in nm.PRECISIONS:
   a,pa=run(poison,'original',pc);b,pb=run(poison,'v14',pc);assert a==b;rows.append(dict(poison=poison,pc=pc,fields=a));pins.update(pa);pins.update(pb)
 out={**nm.header(__file__,__doc__),'rows':rows,'native_executions':len(rows)*2,'executed_pins':nm.instructions(pins)}
 path=P/'native-base-terrain-constructor.json';encoded=json.dumps(out,indent=2)+'\n'
 if '--write'in sys.argv:
  with path.open('x')as f:f.write(encoded)
 else:assert json.loads(path.read_text())==out
 print(dict(rows=len(rows),runs=len(rows)*2,pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest()))
if __name__=='__main__':main()
