"""Whole4F8AE0/4F6280 with original RNG, WorldRules, Team/Kingdom fields,
and4F0DB0/4EFC50/4EFBE0 slot traversal. Typed logger leaves/CString allocator
are explicit transports. Positive/error logger prefixes retain the real RNG.
"""
from pathlib import Path
import sys,json,hashlib,random
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parent/'skirmish-20261017-launch-settings'))
from native_launch_dump import DumpMachine,Stop,h

class Machine(DumpMachine):
 def execute(self,c):
  for row in c['rules']['rules']:self.w(self.def_ptr('WORLD_RULES',row['rule'])+0x20,row['kind'])
  world=self.alloc(0x400);self.w(0xb36534,world)
  for offset,value in c['words'].items():self.w(world+int(offset,16),value)
  self.w(world+0xd4,c['rng']);self.build_wc(world+0xc,c['rules'])
  objects={}
  for row in c['things']:
   p=self.alloc(0x40);objects[row['id']]=p
   self.w(p+0x14,row['id']);self.w(p+0x1c,row['created'])
   for i,value in enumerate(row['position']):self.w(p+0x20+4*i,value)
  for offset,key in [(0x16c,'teams'),(0x148,'kingdoms')]:
   values=c[key];a=self.alloc(max(1,len(values))*4)
   self.w(world+offset,a);self.w(world+offset+4,len(values))
   for i,value in enumerate(values):self.w(a+4*i,objects[value]if value else 0)
  registry=self.alloc(0x60010);self.w(0xb364c0,registry)
  self.w(registry+0x6000c,max([row['id']&65535 for row in c['things']]+[0]))
  for row in c['things']:self.w(registry+0x20004+4*(row['id']&65535),objects[row['id']])
  if c['weather'] is not None:
   weather=self.alloc(0x20);self.w(weather+0xc,c['weather']);self.w(world+0x290,weather)
  label=self.string('g_gworld');stopped=False
  try:self.call(0x4f8ae0,[label,world])
  except Stop:stopped=True
  return dict(events=self.events,stopped=stopped,rng=self.r(world+0xd4))

def cases():
 out=[]
 for i,seed in enumerate([1,0x7fffffff,0x80000000,0xffffffff]):
  things=[dict(id=(gen<<16)|slot,created=0x3f800000+n*0x10000,position=[0x40000000+n*0x10000,0x40800000+n*0x10000,0x3f000000+n*0x10000])for n,(gen,slot)in enumerate([(2,9),(3,2),(0,20)])]
  out.append(dict(rng=seed,words={f'{a:x}':v for a,v in [(0xdc,17),(0xe0,0x3f000000),(0xe4,0x3f800000),(0xe8,0x3d800000),(0x140,8),(0x184,6),(0x194,5),(0x198,4),(0x44,2),(0x48,1),(0x275,i&1),(0x278,0x3f800000),(0x158,2),(0x25c,3)]},things=things if i else[],teams=[things[0]['id'],None]if i else[],kingdoms=[None,things[1]['id'],things[2]['id']]if i else[],weather=None if i%2==0 else 19,
   rules=dict(kind=2,condition=0,rules=[dict(rule=j,kind=3 if i==3 and j==0 else j,enabled=bool(j&1),value=v)for j,v in enumerate([0x100 if i==2 else 1,0xfffffffe,0x3faaaaab])],changes=123)))
 return out

MACHINES={}
def run(c,image,pc,stop):
 #Definition/CString transports are reused; every reached World, RNG, GTID
 #registry, vector and Thing has new explicit storage on every execution.
 #Exclude the inherited constructor startup from this function's pin census.
 key=(image,pc)
 if key not in MACHINES:MACHINES[key]=Machine(image,pc)
 m=MACHINES[key];m.cutoff=stop;m.events=[];m.visited={};m.intercepted=set()
 result=m.execute(c);return result,dict(m.visited),set(m.intercepted)

def main():
 rows=[];visited={};seams=set()
 for pc in h.PRECISION:
  for c in cases():
   full,_,_=run(c,'original',pc,None)
   for stop in [None,0,1,2,7,len(full['events'])//2,len(full['events'])-1]:
    a,pa,sa=run(c,'original',pc,stop);b,pb,sb=run(c,'v14',pc,stop);assert a==b
    if stop is not None:assert a['events']==full['events'][:stop]and a['stopped']
    visited.update(pa);visited.update(pb);seams|=sa|sb;rows.append(dict(precision=pc,input=c,stop=stop,output=a))
 pins=[]
 for va,n in sorted(visited.items()):
  if va in seams or not 0x401000<=va<0x9ef000:continue
  raw=h.P.range(va,n);assert raw==h.V.range(va,n);pins.append(dict(va=hex(va),bytes=raw.hex()))
 assert any(0x85d840<=int(p['va'],16)<0x85d870 for p in pins),'real RNG never executed'
 doc=dict(schema=2,scope=__doc__,original_sha256=h.ORIGINAL_SHA,v14_sha256=h.V14_SHA,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,native_runs=len(rows)*2,additional_prefix_reference_runs=len(h.PRECISION)*len(cases()),instructions=pins,seams=[hex(a)for a in sorted(seams)])
 path=HERE/'native-initial-world-sync.json';data=(json.dumps(doc,indent=1)+'\n').encode()
 if '--write'in sys.argv:path.write_bytes(data)
 else:assert path.read_bytes()==data
 print(json.dumps(dict(rows=len(rows),native_runs=len(rows)*2,pins=len(pins),sha256=hashlib.sha256(data).hexdigest())))
if __name__=='__main__':main()
