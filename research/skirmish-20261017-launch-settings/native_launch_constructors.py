"""Actual514000/514050/5141A0/944BC0 constructors with common513EB0.
Successful allocation domain; retained current variables, typed Biome Find,
CString/allocator/logging transports inherited and named. No retagged Load.
Native RMC arrays retain capacity1 but count0; capacity is not a game cap.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/skirmish-20261017-mission-start'))
from native_creator_constructor import Machine,h

def run(image,pc,inputs):
 m=Machine(image,pc,inputs);rows=[]
 for kind,address,size in [('Blank',0x514000,0x54),('Load',0x5141f0,0x54),('Random',0x514050,0x78),('Preset',0x5141a0,0x54),('Metagame',0x944bc0,0x5c)]:
  p=m.alloc(size);m.u.mem_write(p,b'\xcd'*size);m.call(address,[0],this=p,callee_pop=4)
  base=m.current(p);base.pop('kind_load');base.pop('source50_null')
  base['closed30']=[m.rb(m.r(p+0x14)+i*0x6c+0x30)for i in range(m.r(p+0x18))]
  tail={}
  if kind in ['Blank','Metagame']:tail['doodify']=m.rb(p+0x50)
  if kind in ['Load','Random','Preset']:tail['null50']=m.r(p+0x50)==0
  if kind=='Random':
   tail.update(total=m.r(p+0x54),terrain_count=m.r(p+0x5c),terrain_capacity=m.r(p+0x60),terrain_nonnull=m.r(p+0x58)!=0,
    object_count=m.r(p+0x68),object_capacity=m.r(p+0x6c),object_nonnull=m.r(p+0x64)!=0,heroes=m.r(p+0x70),text74=m.rs(p+0x74))
  if kind=='Metagame':tail['words54_58']=[m.r(p+0x54),m.r(p+0x58)]
  rows.append(dict(kind=kind,vtable=m.r(p),base=base,tail=tail))
 return rows,m.visited,m.intercepted

def main():
 inputs=[dict(width=w,height=ht,independent=n,resource_count=r)for w,ht,n,r in [(192,192,0,3),(16777217,-16777217,1,0),(-2147483648,2147483647,-1,3),(1,256,2,3)]]
 rows=[];visited={};seams=set()
 for pc,inp in itertools.product(h.PRECISION,inputs):
  a,pa,sa=run('original',pc,inp);b,pb,sb=run('v14',pc,inp);assert a==b
  visited.update(pa);visited.update(pb);seams|=sa|sb;rows.append(dict(precision=pc,input=inp,output=a))
 pins=[]
 for va,size in sorted(visited.items()):
  if va in seams or not 0x401000<=va<0x9ef000:continue
  raw=h.P.range(va,size);assert raw==h.V.range(va,size);pins.append(dict(va=hex(va),bytes=raw.hex()))
 doc=dict(schema=1,scope=__doc__,original_sha256=h.ORIGINAL_SHA,v14_sha256=h.V14_SHA,
  producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,executions=len(rows)*2*5,
  instructions=pins,seams=[hex(a)for a in sorted(seams)])
 path=HERE/'native-launch-constructors.json';encoded=(json.dumps(doc,indent=2)+'\n').encode()
 if '--write'in sys.argv:path.write_bytes(encoded)
 else:assert path.read_bytes()==encoded
 print(json.dumps(dict(rows=len(rows),constructors=len(rows)*10,pins=len(pins),sha256=hashlib.sha256(encoded).hexdigest())))
if __name__=='__main__':main()
