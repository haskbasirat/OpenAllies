"""564BE0/5644F0 City layout request, clear/prepend and real RNG selection.

Real564BE0,5644F0,763B90,45DBD0,4F02B0/85D840,5642A0/588AB0 and
4D0900 execute. CString pool/allocation, SyncA/SyncS log sink, Actor108 and
nested4FA030/4F9C10 creation are explicit transports. Nested creation captures
the full actual Params54 block; it does not certify new actors or objects.
Actor108 deliberately mutates current facing to test the later reread.
"""
from pathlib import Path
import sys,json,hashlib,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-conversion'))
import native_machine as h
from unicorn.x86_const import UC_X86_REG_ECX

def f(v):return struct.unpack('<I',struct.pack('<f',v))[0]
def run(pc,c):
 m=h.Machine(pc);events=[];world=m.alloc(0x300);actor=m.alloc(0x180);data=m.alloc(0x600);city=m.alloc(0x24);params=m.alloc(0x54)
 m.w(0xb36534,world);m.w(world+0xd4,c['seed']);m.w(actor+4,data);m.w(actor+0x14,0x10007);m.w(actor+0xd4,city);m.w(city+4,actor)
 for off,word in [(0x20,c['position'][0]),(0x24,c['position'][1]),(0x2c,c['facing'][0]),(0x30,c['facing'][1])]:m.w(actor+off,word)
 old=[]
 for value in [81,82]:
  n=m.alloc(8);m.w(n,value);m.w(n+4,0)
  if old:m.w(old[-1]+4,n)
  old.append(n)
 m.w(city+0x10,old[0]);m.w(city+0x14,old[-1]);m.w(params+0x34,c['argument'])
 defs=[m.definition('actor_target',3,'ActorData'),m.definition('object_target',2,'ObjectData')]
 layouts=m.alloc(c['layouts']*4);m.w(data+0x5a8,layouts);m.w(data+0x5ac,c['layouts'])
 for index in range(c['layouts']):
  layout=m.alloc(12);rows=m.alloc(c['rows']*20);m.w(layout,rows);m.w(layout+4,c['rows']);m.w(layouts+index*4,layout)
  for j in range(c['rows']):
   for k,value in enumerate([defs[j%2],f((j+1)*.125+index),f(-.25-index),f(.6),f(-.8)]):m.w(rows+j*20+k*4,value)
 def seed(mm):events.append(['seed',mm.arg(1)]);mm.ret(12)
 m.seam(0x77da70,seed)
 owners=[0]
 def owner(mm):
  events.append(['owner']);owners[0]+=1
  if c['mutate']:
   m.w(actor+0x2c,c['after_facing'][0]);m.w(actor+0x30,c['after_facing'][1])
  mm.ret(0,0x10003)
 m.w(actor,m.vtable({0x108:owner}))
 made=[]
 def create(mm,kind):
  p=mm.arg();raw=[mm.r(p+4*i)for i in range(21)]
  events.append(['create',kind,defs.index(raw[1]),raw[2:7],raw[8:21]])
  assert mm.cstring_units(raw[7])==[]
  if c['fail']==len(made):mm.outcome={'kind':'required_create'};raise h.Stop()
  out=0
  if c['created']:
   out=mm.alloc(0x180);mm.w(out+0x14,0x10010+len(made));settlement=mm.alloc(0x38)
   mm.w(out+0x94,settlement if c['settlement']else 0)
   made.append((out,settlement))
  else:made.append((0,0))
  mm.ret(4,out)
 m.seam(0x4fa030,lambda mm:create(mm,'actor'));m.seam(0x4f9c10,lambda mm:create(mm,'object'))
 if c['component']:m.call(0x564be0,city,[params,c['flag']],8)
 else:m.call(0x5644f0,0,[actor,c['argument'],0],0)
 values=[];node=m.r(city+0x10)
 while node:values.append(m.r(node));node=m.r(node+4)
 return dict(events=events,seed=m.r(world+0xd4),values=values,settlement_city=[m.r(s+0x34)if s else None for _,s in made],error=None if m.outcome is None else m.outcome['kind'])

def main():
 rows=[];original=h.P
 cases=[]
 for i in range(48):cases.append(dict(component=i%3!=0,flag=[0,1,256][i%3],argument=[0xffffffff,0,1][i%3]if i%2 else 0xffffffff,layouts=2 if i%3==2 else 1,rows=1 if i%2 else 3,
  seed=[0,1,0x12345678,0xffffffff][i%4],position=[f(1000.125),f(-.33)],facing=[f(.6),f(-.8)],mutate=i%4==0,after_facing=[f(-1),0],created=i%5!=0,settlement=i%2==0,fail=1 if i%7==0 else -1))
 try:
  for image,pe in [('original',original),('v14',h.V)]:
   h.P=pe
   for pc in h.CW:
    for c in cases:rows.append(dict(image=image,precision=pc,input=c,output=run(pc,c)))
 finally:h.P=original
 a=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original'];b=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14'];assert a==b
 doc=dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,observed_pins=[dict(va=hex(a),bytes=b)for a,b in sorted(h.VISITED.items())])
 p=HERE/'native-fresh-city.json'
 if '--create'in sys.argv:
  with p.open('x')as out:json.dump(doc,out,indent=2);out.write('\n')
 else:assert json.loads(p.read_bytes())==doc
 print(json.dumps(dict(rows=len(rows),pins=len(h.VISITED),sha256=hashlib.sha256(p.read_bytes()).hexdigest())))
if __name__=='__main__':main()
