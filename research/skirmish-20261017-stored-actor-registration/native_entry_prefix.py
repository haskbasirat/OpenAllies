"""Actual515620 zone-leaf prefix and first required owner boundaries.
Real flags and Actor+B8 Detection pointer are read; positive slot5/4/3/City
owner getters and positive discovery body stop BEFORE their first instruction.
No positive component/city/discovery success, allocator or game-world proof.
"""
from pathlib import Path
import hashlib,itertools,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
P,V=h.P,h.V;H=lambda b:hashlib.sha256(b).hexdigest()
BOUNDARIES={0x515647:'slot5',0x51566c:'slot4',0x515697:'slot3',0x5156d3:'city',0x51571c:'discovery'}
class Stop(Exception):pass
def run(flags,detection,pc,image):
 h.P=image;m=h.Machine(pc);leaf=m.alloc(20);actor=m.alloc(0x400)
 m.w32(leaf,0xa323e8,1,0xdeadbeef,flags,0);m.w32(actor,0xa273f0);m.w32(actor+0xb8,0x1234 if detection else 0)
 outcome='complete'
 def stop(label):
  def reached(machine):
   nonlocal outcome
   outcome=label;raise Stop()
  return reached
 for address,label in BOUNDARIES.items():m.seams[address]=stop(label)
 try:
  m.call(0x515620,leaf,[actor]);assert m.final_esp==h.STACK+8 and m.fpu_top()==0
 except Stop:pass
 assert m.r32(actor+0xb8)==(0x1234 if detection else 0)
 return outcome
def main():
 h.VISITED.clear();rows=[]
 for pc,flags,detection in itertools.product(h.PRECISION,[0,1,2,4,8,16,32,64,256,12,24,40,48,264,272,288,0x13c,0xffff],[False,True]):
  a=run(flags,detection,pc,P);b=run(flags,detection,pc,V);assert a==b
  expected='slot5' if flags&32 else 'slot4' if flags&16 else 'slot3' if flags&8 else 'city' if flags&256 else 'discovery' if flags&4 and detection else 'complete'
  assert a==expected,(flags,detection,a,expected)
  rows.append(dict(precision=pc,flags=flags,detection=detection,outcome=a))
 h.P=P
 pins={hex(a):v for a,v in sorted(h.VISITED.items()) if a not in BOUNDARIES}
 stopped={hex(a):h.VISITED[a] for a in BOUNDARIES if a in h.VISITED}
 for a,v in {**pins,**stopped}.items():assert P.range(int(a,16),len(bytes.fromhex(v)))==V.range(int(a,16),len(bytes.fromhex(v)))
 doc=dict(schema=1,scope=__doc__,producer_sha256=H(Path(__file__).read_bytes()),machine_sha256=H(Path(h.__file__).read_bytes()),rows=rows,executed_pins=pins,stopped_boundary_pins=stopped)
 data=(json.dumps(doc,indent=1)+'\n').encode();p=HERE/'native-entry-prefix.json'
 if '--write' in sys.argv:
  with p.open('xb') as f:f.write(data)
 else:assert p.read_bytes()==data
 print(json.dumps(dict(rows=len(rows),executions=2*len(rows),executed_pins=len(pins),stopped_pins=len(stopped),sha256=H(data))))
if __name__=='__main__':main()
