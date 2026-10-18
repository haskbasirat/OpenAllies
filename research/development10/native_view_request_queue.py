"""Actual43D030 ->45F240 FIFO append at VWorld208/20C.
Heap allocation is inherited transport, not pointer/allocator parity. Views
are laboratory opaque allocation identities; repeated entries are retained.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE/'creator-labels'))
from native_creator_labels import Machine,PE0,PE1,PC
def main():
 rows=[];pins={}
 for image in ['original','v14']:
  m=Machine(image)
  for precision in PC:
   for order in [[],[1],[1,2],[1,2,1,3],[3,3,3]]:
    m.fpu(precision);m.actual={};m.record_actual=True;world=m.alloc(0x300);m.w(world+0x208,0);m.w(world+0x20c,0)
    views={i:m.alloc(0x80)for i in [1,2,3]};reverse={v:k for k,v in views.items()}
    for value in order:m.call(0x43d030,[views[value]],this=world,callee_pop=4)
    out=[];node=m.r(world+0x208);seen=set();tail=0
    while node:assert node not in seen;seen.add(node);out.append(reverse[m.r(node)]);tail=node;node=m.r(node+4)
    assert tail==m.r(world+0x20c)and out==order
    rows.append(dict(image=image,precision=precision,input=order,output=out,head_present=bool(m.r(world+0x208)),tail_present=bool(m.r(world+0x20c))));pins.update(m.actual)
 instructions=[]
 for va,size in sorted(pins.items()):
  if PE0.base<=va<PE0.base+PE0.image_size:
   raw=PE0.range(va,size);assert raw==PE1.range(va,size);instructions.append(dict(va=hex(va),bytes=raw.hex()))
 doc=dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,instructions=instructions)
 out=HERE/'native-view-request-queue.json'
 if '--write'in sys.argv:out.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(out.read_text())==doc
 print(json.dumps(dict(rows=len(rows),pins=len(instructions),sha256=hashlib.sha256(out.read_bytes()).hexdigest())))
if __name__=='__main__':main()
