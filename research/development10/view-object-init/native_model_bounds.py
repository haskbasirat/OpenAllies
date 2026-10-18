"""Actual878630 geometry radial/Z and878790 low-height footprint arithmetic.

Explicit scalar/vertex/world-transform inputs; one virtual vertex-count and
type-query transport stands in for the already supplied geometry/Data type.
No NIF loading, NiScene traversal, skin transforms, tags or source selection
is certified. Skin pointers are explicit NULL, hidden flag is an explicit
input. All floating arithmetic executes original/v14 instructions at all3PC.
"""
from pathlib import Path
import sys,json,hashlib,struct
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parent/'creator-labels'))
from native_creator_labels import Machine,PE0,PE1,PC
def f(v):return struct.unpack('<I',struct.pack('<f',v))[0]
IDENTITY=[f(1),0,0,0,f(1),0,0,0,f(1),0,0,0,f(1)]
def run(m,pc,transform,vertices,hidden):
 m.fpu(pc);m.actual={};m.record_actual=True
 g=m.alloc(0x110);data=m.alloc(0x40);vt=m.alloc(0x50);dvt=m.alloc(0x50)
 stub=m.alloc(0x20);m.u.mem_write(stub,bytes.fromhex('668b410cc3'));m.w(dvt+0x38,stub)
 m.w(data,dvt);m.w(data+0xc,len(vertices));array=m.alloc(max(1,len(vertices))*12)
 for i,v in enumerate(vertices):
  for j,w in enumerate(v):m.w(array+i*12+j*4,w)
 m.w(data+0x20,array);m.w(g+0xcc,data);m.w(g+0xd0,0);m.w(g+0x20,int(hidden))
 m.w(g,vt);m.u.mem_write(stub+8,b'\xb8'+struct.pack('<I',0xb43cf0)+b'\xc3');m.w(vt+4,stub+8)
 for i,w in enumerate(transform):m.w(g+0x6c+i*4,w)
 radial=m.alloc(4);low=m.alloc(4);high=m.alloc(4)
 m.w(radial,0);m.w(low,0x7f7fffff);m.w(high,0xff7fffff)
 m.call(0x878630,[g,radial,low,high])
 footprint=m.alloc(12)
 for i in range(3):m.w(footprint+4*i,0)
 m.call(0x878790,[g,footprint,footprint+4,m.r(low)])
 return dict(radial=m.r(radial),min_z=m.r(low),max_z=m.r(high),
             footprint=[m.r(footprint+i*4)for i in range(3)]),m.actual
def main():
 rows=[];pins={}
 transforms=[IDENTITY,IDENTITY[:9]+[f(1.25),f(-2.5),f(0.125),f(0.75)],
             [0,f(-1),0,f(1),0,0,0,0,f(1),f(-0.2),f(0.3),f(-0.01),f(1.25)],
             [f(.6),f(-.8),0,f(.8),f(.6),0,0,0,f(1),f(.01),f(.02),f(.03),0x3f800001]]
 vectors=[[],[[0,0,0]],[[f(3),f(4),f(-.1)],[f(-5),f(2),f(.2)],[f(1),f(-2),f(3)]],
          [[0x3f800001,0x3f7fffff,0x80000000],[f(1e-15),f(-1e-15),f(1e-9)],[f(127.1),f(-129.3),f(-.001)]]]
 for image in ['original','v14']:
  m=Machine(image)
  for pc in PC:
   for transform in transforms:
    for vertices in vectors:
     for hidden in [False,True]:
      output,actual=run(m,pc,transform,vertices,hidden);pins.update(actual)
      rows.append(dict(image=image,precision=pc,input=dict(transform=transform,vertices=vertices,hidden=hidden),output=output))
 a=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original']
 b=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14'];assert a==b
 instructions=[]
 for va,size in sorted(pins.items()):
  if PE0.base<=va<PE0.base+PE0.image_size:
   raw=PE0.range(va,size);assert raw==PE1.range(va,size)
   instructions.append(dict(va=hex(va),bytes=raw.hex()))
 doc=dict(schema=1,scope=__doc__,threshold=int.from_bytes(PE0.range(0x9fe0c0,4),'little'),producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
          rows=rows,instructions=instructions)
 out=HERE/'native-model-bounds.json'
 if '--write'in sys.argv:out.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(out.read_text())==doc
 print(json.dumps(dict(rows=len(rows),pins=len(instructions),sha256=hashlib.sha256(out.read_bytes()).hexdigest())))
if __name__=='__main__':main()
