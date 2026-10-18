"""Original typed current Var setters40E0A0/40E110/4B3190/4B32B0.
Explicit laboratory Vars test flags0/2/20 and raw/current word semantics;
no process-current capture or source/WorldRule admission is inferred.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-net-integration'));from native_launch_assignment import Machine as Base
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn.x86_const import UC_X86_REG_EIP,UC_X86_REG_FPCW
class Machine(Base):
 def __init__(self,image,pc):
  self.intercepted=set();super().__init__(False,image);self.mode='channels';self.u.reg_write(UC_X86_REG_FPCW,h.PRECISION[pc])
 def hook(self,u,at,n,z):
  super().hook(u,at,n,z)
  if u.reg_read(UC_X86_REG_EIP)!=at:self.intercepted.add(at)
 def run(self,kind,flags,v):
  p=self.alloc(0x2c);self.w(p,0x3f000000);self.w(p+4,0x3f000000);self.w(p+8,0x3f000000);self.w(p+12,0x3f000000)
  self.w(p+0x24,kind);self.w(p+0x28,flags)
  self.w(p+0x14,0xbf800000 if kind==2 else 0xfffffffe);self.w(p+0x18,0x3f800000 if kind==2 else 2)
  self.call({1:0x4b3190,2:0x40e0a0,3:0x40e110,5:0x4b32b0}[kind],v if kind==5 else[v],this=p,callee_pop=12 if kind==5 else 4)
  return [self.rb(p)]if kind==1 else[self.r(p+i*4)for i in(1,2,3)]if kind==5 else[self.r(p)]
def main():
 rows=[];pins={};seams=set()
 cases=[]
 for kind in(1,2,3):
  for flags in(0,2,32):
   for v in(0,1,0xffffffff,0x80000000,0x3f800000,0xbf800000,0x40000000):
    cases.append((kind,flags,v))
 for flags in(0,2):cases.extend((5,flags,v)for v in([0,0x3f800000,0xbf800000],[0x40000000,0xc0000000,0x3f000000]))
 for pc in h.PRECISION:
  a=Machine('original',pc);b=Machine('v14',pc)
  for kind,flags,v in cases:
   x=a.run(kind,flags,v);y=b.run(kind,flags,v);assert x==y
   rows.append(dict(precision=pc,kind=kind,flags=flags,input=v,output=x))
  pins.update(a.visited);pins.update(b.visited);seams|=a.intercepted|b.intercepted
  print(pc,'completed',len(cases),'paired setter calls',flush=True)
 for a,n in pins.items():
  if a not in seams and 0x401000<=a<0x9ef000:assert h.P.range(a,n)==h.V.range(a,n)
 doc=dict(schema=1,scope=__doc__,rows=rows,executed=[dict(address=hex(a),bytes=h.P.range(a,n).hex())for a,n in sorted(pins.items())if a not in seams and 0x401000<=a<0x9ef000],intercepted=[hex(a)for a in sorted(seams)])
 p=HERE/'native-setters.json';raw=(json.dumps(doc,indent=1)+'\n').encode()
 if '--write'in sys.argv:p.write_bytes(raw)
 else:assert p.read_bytes()==raw
 print(json.dumps(dict(rows=len(rows),machines=2*len(rows),pins=len(doc['executed']),sha256=hashlib.sha256(raw).hexdigest())))
if __name__=='__main__':main()

