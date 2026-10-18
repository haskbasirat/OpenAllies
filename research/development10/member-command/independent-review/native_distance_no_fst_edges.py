"""Independent529F10 finite edge cases that distinguish a final f32 spill.

Only the actual distance leaf executes, without callback transports. The
binary32-spilled comparator is deliberately rejected; it is not a native
implementation. The accepted576 coordinator rows remain unchanged.
"""
from pathlib import Path
import hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[3]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h

def main():
 original=h.P;rows=[]
 try:
  for label,image in [('original',original),('v14',h.V)]:
   h.P=image
   for pc in h.PRECISION:
    m=h.Machine(pc)
    for dy in [0.0001,0.00014,0.00015,0.00016]:
     dx=0x3effffff;y=h.f2b(dy)
     actual=m.call(0x529f10,args=(0,0,dx,y))&255
     assert m.fpu_top()==0
     squared=h.b2f(dx)**2+h.b2f(y)**2
     spilled=h.f2b(squared)
     rows.append(dict(image=label,precision=pc,position=[0,0],target=[dx,y],close=bool(actual),binary32_spilled_squared=spilled,spilled_close=h.b2f(spilled)<.25))
 finally:h.P=original
 a=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original'];b=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14'];assert a==b
 assert sum(r['close']and not r['spilled_close']for r in rows)==8
 doc=dict(scope=__doc__,rows=rows,original_sha256=h.ORIGINAL_SHA,v14_sha256=h.V14_SHA,
  producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),observed_pins=[dict(address=hex(a),bytes=b)for a,b in sorted(h.VISITED.items())])
 path=HERE/'distance-no-fst-edges.json';payload=(json.dumps(doc,indent=2)+'\n').encode()
 if '--write'in sys.argv:path.write_bytes(payload)
 else:assert path.read_bytes()==payload
 print(json.dumps(dict(result='PASS',rows=len(rows),pins=len(h.VISITED),sha256=hashlib.sha256(payload).hexdigest())))
if __name__=='__main__':main()
