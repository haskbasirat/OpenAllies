"""Fresh concrete ordinary-Actor predicate and empty-listener registration paths.

Actor/Kingdom actual vtable dispatch. Activity uses real nested Element parents;
79E080 executes all three getters, including current GTID generation lookup.
Wall positive and matching-controller/positive category0 map lists are excluded.
Only ordinary registration's temporary new/free leaves are intercepted.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn.x86_const import UC_X86_REG_EAX
H=lambda b:hashlib.sha256(b).hexdigest();O=h.P;V=h.V
LEAVES={0x6f7810,0x6f7430}
def run(c,image):
 prior=h.P;h.P=O if image=='original'else V
 try:m=h.Machine(c['precision'])
 finally:h.P=prior
 h.VISITED.clear();a=m.alloc(0x300);m.w32(a,c['vtable']);m.w32(a+8,c.get('flags',0))
 op=c['op']
 if op=='mind':
  if c['word']is not None:
   mind=m.alloc(0x40);m.w32(a+0x70,mind);m.w32(mind+0x28,c['word'])
  m.u.reg_write(UC_X86_REG_EAX,0xfedcba98);got=m.call(0x4ceff0,a)&255
 elif op=='activity':
  for key,off in [('mover',0x6c),('mobile',0xac),('recruit',0xa0),('denizen',0x88)]:m.w32(a+off,int(c[key]))
  if c['parent']is not None:
   element=m.alloc(0x30);parent=m.alloc(0x300);m.w32(a+0x80,element);m.w32(element+0x10,parent)
   m.w32(parent,c['vtable']);m.w32(parent+8,0 if c['parent']else 4)
  got=m.call(0x4cf120,a,[0])&255
 elif op=='presence':
  registry=m.alloc(0x70000);m.w32(0xb364c0,registry)
  if c['structure']:
   structure=m.alloc(0x30);m.w32(a+0x94,structure);m.w32(structure+0x10,0xdead1111)
  if c['element']:
   element=m.alloc(0x30);m.w32(a+0x80,element);m.w32(element+0x10,0xdead2222)
  if c['mind']!='absent':
   mind=m.alloc(0x40);m.w32(a+0x70,mind);m.w32(mind+0x28,0x30001)
   m.u.mem_write(registry+6,(3 if c['mind']=='current'else 2).to_bytes(2,'little'));m.w32(registry+0x20008,0xdead3333)
  got=m.call(0x79e080,0,[a])
 else:
  manager=m.alloc(0xb0);data=m.alloc(0x500);category=m.alloc(0x20);m.w32(a+4,data);m.w32(data+0x18,category);m.w32(category+0xc,2)
  m.w32(a+8,1<<23);m.w32(a+0x84,1);m.w32(a+0x140,5);m.w32(a+0xe8,c['owner'])
  slots=m.alloc(4*len(c['slots']));m.w32(manager+0x60,slots,len(c['slots']))
  for i,owner in enumerate(c['slots']):
   if owner is not None:
    player=m.alloc(0x30);m.w32(player+8,owner);m.w32(slots+4*i,player)
  m.seams[0x6f7810]=lambda q:q.ret(value=q.alloc(q.arg()))
  m.seams[0x6f7430]=lambda q:q.ret()
  m.call(0x7a0390,manager,[a,c['category'],0]);got=m.r32(a+0x140)
 assert not m.events
 return got,dict(h.VISITED)
def cases():
 for pc,vt in itertools.product(h.PRECISION,[0xa273f0,0xa299d8]):
  base=dict(precision=pc,vtable=vt)
  for word in [None,0,0x30001]:yield base|dict(op='mind',word=word)
  for flags,mover,mobile,recruit,denizen,parent in itertools.product([0,4],[False,True],[False,True],[False,True],[False,True],[None,False,True]):
   yield base|dict(op='activity',flags=flags,mover=mover,mobile=mobile,recruit=recruit,denizen=denizen,parent=parent)
  for structure,element,mind in itertools.product([False,True],[False,True],['absent','current','stale']):yield base|dict(op='presence',structure=structure,element=element,mind=mind)
  for category,owner,slots in [(0,0,[]),(0,0,[None,999]),(1,782,[]),(1,782,[None,None]),(2,0,[]),(2,782,[None,999]),(2,782,[])]:
   yield base|dict(op='register',category=category,owner=owner,slots=slots)
def main():
 rows=[];pins={};leaves={}
 for c in cases():
  a,ap=run(c,'original');b,bp=run(c,'v14');assert a==b
  for va,raw in (ap|bp).items():(leaves if va in LEAVES else pins)[va]=raw
  rows.append(dict(input=c,output=a))
 data=[]
 for vt in [0xa273f0,0xa299d8]:
  for off,target in [(0x20,0x4cf120),(0x100,0x4ced60),(0x108,0x4d1e80)]:
   assert O.u32(vt+off)==V.u32(vt+off)==target;data.append(dict(va=hex(vt+off),bytes=O.range(vt+off,4).hex()))
 doc=dict(scope=__doc__,producer_sha256=H(Path(__file__).read_bytes()),original_sha256=H(O.b),v14_sha256=H(V.b),rows=rows,machines=2*len(rows),
  executed_pins=[dict(va=hex(a),bytes=b)for a,b in sorted(pins.items())],intercepted_entries=[dict(va=hex(a),bytes=b)for a,b in sorted(leaves.items())],data_pins=data,
  dependency=dict(path='research/phase8-movement/harness.py',sha256=H((ROOT/'research/phase8-movement/harness.py').read_bytes())))
 raw=(json.dumps(doc,indent=1)+'\n').encode();out=HERE/'native-actor-ordinary.json'
 if '--write'in sys.argv:out.write_bytes(raw)
 else:assert json.loads(out.read_text())==doc
 print(json.dumps(dict(rows=len(rows),machines=2*len(rows),executed_pins=len(pins),intercepted_entries=len(leaves),sha256=H(raw),immutable='--write'not in sys.argv)))
if __name__=='__main__':main()
