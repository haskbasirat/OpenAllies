"""Native TGI ->4C05F0 deferred City rows ->475230/4C05D0 binding.

Source parser, point getters, deferred linked list, source row allocation and
layout append/growth execute. Canonical4BDBE0 definition lookup is an explicit
IDS-to-owned-fixture-pointer transport; heap/CString/line/CRT/lock transports
are inherited from the whole parser harness. This is not a full GameData load.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/skirmish-20261017-member-deployment'))
from native_point2_decimal_prefix import Machine as Base,P,V,PCS
from unicorn.x86_const import UC_X86_REG_EIP
class Machine(Base):
 def hook(self,u,pc,size,user):
  if getattr(self,'lookup_ready',False)and pc==0x4bdbe0:
   ids=self.text(self.arg());assert self.arg(1)==0x2b
   self.lookups.append(ids);self.ret(self.targets[ids],pop=8);self.excluded.add(pc);return
  super().hook(u,pc,size,user)
def run(c,pe,pc):
 lines=['[Layout]','{']
 for i in range(c['count']):lines +=['[Object]','{',f'string object_ids = Target{i}',f'point2 offset = {i}.125, -{i}.25',f'point2 orientation = 0.6, -0.8','}']
 lines+=['}'];src=('\n'.join(lines)+'\n').encode()
 m=Machine(src,pe,pc);parsed=m.parse();assert not parsed['error'],parsed
 node=m.node_ptrs['Layout'];layout=m.alloc(12);owner=m.alloc(4);m.w(owner,m.string(list(map(ord,'ActualCity'))))
 old=m.r(0xb36530);game=m.alloc(0x900);m.u.mem_write(game,bytes(m.u.mem_read(old,0x600)));m.w(0xb36530,game);m.w(game+0x824,0)
 m.targets={f'Target{i}':m.alloc(0x40)for i in range(c['count'])};m.lookups=[];m.lookup_ready=True
 m.invoke(0x4c0470,layout,[]);m.invoke(0x4c05f0,layout,[node,owner]);assert m.r(layout+4)==0
 deferred=[];p=m.r(game+0x824)
 while p:deferred.append(m.cstr(m.r(p+4)));p=m.r(p+8)
 m.invoke(0x475230,game+0x824,[]);assert m.r(game+0x824)==0
 values=[]
 for i in range(m.r(layout+4)):
  p=m.r(layout)+20*i;target=next(k for k,v in m.targets.items()if v==m.r(p))
  values.append(dict(definition_ids=target,offset=[m.r(p+4),m.r(p+8)],orientation=[m.r(p+12),m.r(p+16)]))
 pins={a:bytes(m.u.mem_read(a,n))for a,n in m.visited.items()if P.base<=a<P.base+P.image_size and a not in m.excluded}
 return dict(deferred=deferred,lookup=m.lookups,rows=values),pins
def main():
 rows=[];pins={}
 for image,pe in [('original',P),('v14',V)]:
  for pc,cw in PCS.items():
   for count in [1,2,5]:
    c=dict(count=count);out,actual=run(c,pe,cw);pins.update(actual)
    assert [r['definition_ids']for r in out['rows']]==[f'Target{i}'for i in reversed(range(count))]
    rows.append(dict(image=image,precision=pc,input=c,output=out))
 a=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original'];b=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14'];assert a==b
 for va,raw in pins.items():assert P.range(va,len(raw))==V.range(va,len(raw))==raw
 doc=dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,instructions=[dict(va=hex(va),bytes=raw.hex())for va,raw in sorted(pins.items())])
 p=HERE/'native-city-layout-source.json'
 if '--create'in sys.argv:
  with p.open('x')as out:json.dump(doc,out,indent=2);out.write('\n')
 else:assert json.loads(p.read_bytes())==doc
 print(json.dumps(dict(rows=len(rows),pins=len(pins),sha256=hashlib.sha256(p.read_bytes()).hexdigest())))
if __name__=='__main__':main()
