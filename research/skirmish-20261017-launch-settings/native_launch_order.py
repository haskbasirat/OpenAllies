"""Actual76E540 constructor allocation/publication order to76B420 entry.
Header5A2210 is an explicit constructor service seam (its independent proof
is retained elsewhere); all five derived Creators/common and App40F7D0
execute. Required Reset76B420 is an observed cutoff, not completion.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/skirmish-20261017-mission-start'));from native_creator_constructor import Machine,h
class Cutoff(Exception):pass
class Ordered(Machine):
 def hook(self,u,pc,size,user):
  if getattr(self,'ready',False)and pc in [0x5a2210,0x514000,0x5141f0,0x514050,0x5141a0,0x944bc0,0x40f7d0]:self.order.append(hex(pc))
  super().hook(u,pc,size,user)
def run(image,pc):
 m=Ordered(image,pc,dict(width=192,height=192,independent=0,resource_count=3));m.ready=True;m.order=[]
 def header():m.ret(0,m.ecx())
 def stop():raise Cutoff()
 m.seams[0x5a2210]=header;m.seams[0x76b420]=stop;p=m.alloc(0x34)
 try:m.call(0x76e540,[],this=p)
 except Cutoff:pass
 else:raise AssertionError('required Reset must stop')
 pointers=[m.r(p+o)for o in [0x14,0x18,0x1c,0x20,0x24,0x28,0x2c]]
 assert len(set(pointers))==7 and all(pointers)
 assert m.order==['0x5a2210','0x514000','0x5141f0','0x514050','0x5141a0','0x944bc0','0x40f7d0']
 m.intercepted|={0x5a2210,0x76b420}
 return dict(order=m.order,distinct_nonnull_pointers=True,load_vtable=m.r(pointers[2]),text30=m.rs(p+0x30)),m.visited,m.intercepted
def main():
 rows=[];visited={};seams=set()
 for pc in h.PRECISION:
  a,pa,sa=run('original',pc);b,pb,sb=run('v14',pc);assert a==b;visited.update(pa);visited.update(pb);seams|=sa|sb;rows.append(dict(precision=pc,output=a))
 pins=[]
 for va,n in sorted(visited.items()):
  if va in seams or not 0x401000<=va<0x9ef000:continue
  raw=h.P.range(va,n);assert raw==h.V.range(va,n);pins.append(dict(va=hex(va),bytes=raw.hex()))
 data=(json.dumps(dict(scope=__doc__,rows=rows,executions=len(rows)*2,instructions=pins,seams=[hex(a)for a in sorted(seams)]),indent=1)+'\n').encode();path=HERE/'native-launch-order.json'
 if '--write'in sys.argv:path.write_bytes(data)
 else:assert path.read_bytes()==data
 print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,pins=len(pins),sha256=hashlib.sha256(data).hexdigest())))
if __name__=='__main__':main()
