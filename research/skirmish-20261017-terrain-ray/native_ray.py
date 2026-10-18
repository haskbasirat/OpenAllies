"""Whole71D840 terrain walker and reached real plane/triangle helpers.

Laboratory TerrainMap dimensions/blocks and vertex arrays are explicit input
projections, not native ctor/resource-load receipts or current mission values.
All reached query bodies execute in independently mapped original/v14 images
at each PC. No successful child, geometry, math or allocator seams are used.
Defined domain: finite inputs and nonzero normalized rayZ, positive aligned
dimensions/cells. Native parallel-plane local-stack undefined values excluded.
Point and optional secondary-point writes are retained independently of AL.
"""
from pathlib import Path
import hashlib,itertools,json,random,sys
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE));import probe_native_ray as probe
h=probe.h;P,V=h.P,h.V
def cases():
 rng=random.Random(0x71d840)
 out=[]
 def add(name,**kw):out.append(dict(name=name,**kw))
 eyes=[[8,8,50],[1.5,2.75,20],[13,3,30],[-4,8,20],[17,17,40],[8,8,-10]]
 directions=[[0,0,-1],[.02,.04,-1],[.1,-.03,-1],[-.16,.24,-1],[-.4,-.2,-1],[.25,.5,1]]
 for pattern in ['flat','slope','alternating','random']:
  heights=[0.0]*81 if pattern=='flat'else[x*.125+y*.25 for y in range(9)for x in range(9)]if pattern=='slope'else[float((x+y)%5)for y in range(9)for x in range(9)]if pattern=='alternating'else[rng.randrange(-20,30)/8 for _ in range(81)]
  for i,(eye,direction)in enumerate(itertools.product(eyes,directions)):
   add(f'{pattern}-{i}',eye=eye,direction=direction,heights=heights,mesh=True,loaded=True,secondary=True,border=0)
 for mesh,loaded,secondary,border in itertools.product([False,True],[False,True],[False,True],[0,1]):
  add(f'flags-{int(mesh)}{int(loaded)}{int(secondary)}-{border}',eye=[6,6,20],direction=[.06,.03,-1],heights=[float((x+y)%5)for y in range(9)for x in range(9)],mesh=mesh,loaded=loaded,secondary=secondary,border=border)
 return out
def main():
 h.VISITED.clear();rows=[]
 for pc,c in itertools.product(h.PRECISION,cases()):
  outputs=[]
  for image in [P,V]:
   h.P=image;args={k:v for k,v in c.items()if k!='name'}
   outputs.append(probe.run(pc=pc,**args))
  assert outputs[0]==outputs[1],(pc,c,outputs)
  rows.append(dict(precision=pc,input=c,native=outputs[0]))
 h.P=P
 pins={hex(a):v for a,v in sorted(h.VISITED.items())}
 for a,v in pins.items():assert P.range(int(a,16),len(bytes.fromhex(v)))==V.range(int(a,16),len(bytes.fromhex(v)))
 doc=dict(schema=1,scope=__doc__,rows=rows,executed_pins=pins,
  producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),probe_sha256=hashlib.sha256(Path(probe.__file__).read_bytes()).hexdigest(),harness_sha256=hashlib.sha256(Path(h.__file__).read_bytes()).hexdigest())
 data=(json.dumps(doc,indent=1)+'\n').encode();path=HERE/'native-ray-v3.json'
 if '--write'in sys.argv:
  with path.open('xb')as f:f.write(data)
 else:assert path.read_bytes()==data
 print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,executed_pins=len(pins),sha256=hashlib.sha256(data).hexdigest())))
if __name__=='__main__':main()
