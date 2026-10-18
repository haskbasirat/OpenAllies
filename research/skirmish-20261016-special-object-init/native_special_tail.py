"""Full Area/Camera/Doodad Init tails original/v14 at all PCs.

Object4EBE50 success/failure is a named base seam. Native5904A0 RNG arithmetic,
85D840 stream,50ACC0 owner-list insertion,58F960 and50B250 unlink all execute.
Deferred queue placement is an explicit callback recording owner/delay/data and
returning an allocated bucket node; queue arithmetic has its separate proof.
Missing source/queue/legacy services stop before the reached unavailable read.
"""
from pathlib import Path
import sys,json,hashlib,copy,random
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'))
import native_machine as nm
ORIGINAL=nm.original
ENTRY={6:0x4db200,7:0x4dbcf0,9:0x4dc730}
VT={6:0xa28a00,7:0xa28b88,9:0xa28e90}
def cases():
 out=[]
 base=dict(name='base',class_id=6,base_success=True,version=27,area_head=False,
   counters=[0,0],rng=0x8e4b12a5,components=[None,None],fail=None)
 def add(name,**kw):
  c=copy.deepcopy(base);c.update(kw);c['name']=name;out.append(c)
 for cls in (6,7,9):
  add('base_false_'+str(cls),class_id=cls,base_success=False,counters=[5,9])
  add('base_true_'+str(cls),class_id=cls,counters=[0xffffffff,0xffffffff])
 add('area_legacy_requires',version=21)
 add('area_legacy_existing',version=21,area_head=True)
 add('area_unsigned_version',version=0xffffffff)
 rng=random.Random(0x4dc730)
 for sample in range(24):
  mask=sample%4;components=[]
  for slot in range(2):
   components.append(dict(period=[0,0x3a83126f,0x3f000000,0x4144cccd][sample%4],
      state=[0,1,2,6][(sample+slot)%4],pending=rng.randrange(3)) if mask&(1<<slot) else None)
  add('doodad_'+str(sample),class_id=9,components=components,rng=[0,1,0xffffffff,0x8e4b12a5][sample%4])
 add('doodad_missing_definition',class_id=9,components=[dict(period=0x3f000000,state=0,pending=2),None],fail='definition:0')
 add('doodad_missing_queue',class_id=9,components=[dict(period=0x3f000000,state=0,pending=2),None],fail='schedule:0')
 add('doodad_second_queue_failure',class_id=9,components=[dict(period=0x3f000000,state=0,pending=0),dict(period=0x3f800000,state=0,pending=1)],fail='schedule:1')
 for i,period in enumerate([0,1,0x00800000,0xbf000000,0x7f7fffff,0x3f800001]):
  add('doodad_period_edge_'+str(i),class_id=9,components=[dict(period=period,state=0,pending=0),dict(period=period,state=1,pending=2)],rng=[0,1,0xffffffff][i%3])
 return out
def run(c,image,pc):
 nm.original=ORIGINAL if image=='original' else nm.modified;m=nm.Machine(pc)
 obj=m.alloc(0x100);world=m.alloc(0x300);m.w(0xb36534,world);m.w(obj,VT[c['class_id']])
 m.w(world+0x2c,c['version']);m.w(world+0x180,c['counters'][0]);m.w(world+0x190,c['counters'][1]);m.w(world+0xd4,c['rng'])
 m.w(obj+0x64,0x11111111 if c['area_head'] and c['class_id']==6 else 0)
 trace=[];nodes={};components={};reverse={};serial=0;freed=[];stop=None
 def halt(label):
  nonlocal stop
  stop=label;m.u.emu_stop()
 def new_node(slot):
  nonlocal serial
  serial+=1;node=m.alloc(24);bucket=m.alloc(4);m.w(bucket,node);m.w(node+4,bucket);nodes[node]=(serial,slot)
  return node
 def link(slot,node):
  updatable=components[slot]+12;head=m.r(updatable+4);m.w(node+12,updatable+4);m.w(node+16,head)
  if head:m.w(head+12,node+16)
  m.w(updatable+4,node)
 for slot,row in enumerate(c['components']):
  if row is None:continue
  component=m.alloc(0x30);data=m.alloc(0x50);components[slot]=component;reverse[component+12]=slot
  m.w(obj+0x60+4*slot,component);m.w(component,0xa40e04 if slot==0 else 0xa40dbc)
  m.w(component+4,obj);m.w(component+8,data);m.w(component+12,0xa40ddc if slot==0 else 0xa40d94)
  m.w(component+0x18,row['state']);m.w(data+0x10,row['period'])
  for _ in range(row['pending']):link(slot,new_node(slot))
 def base(mm):
  assert mm.ecx()==obj and mm.arg(1)==0;trace.append(dict(call='object',value=c['base_success']));mm.ret(int(c['base_success']),pop=4)
 m.seam(0x4ebe50,base)
 def legacy(mm):trace.append(dict(call='legacy'));halt('legacy')
 m.seam(0x50ac00,legacy)
 m.seam(0x77da70,lambda mm:mm.ret(pop=12)) # thiscall logger consumes3args; no RNG answer seam.
 def period(mm):
  slot=next(s for s,p in components.items() if p==mm.u.reg_read(nm.UC_X86_REG_ESI))
  trace.append(dict(call='period',slot=slot,bits=c['components'][slot]['period']))
  if c['fail']=='definition:'+str(slot):halt('definition:'+str(slot))
 m.observe(0x5904b5,period)
 vtable=m.alloc(16);m.w(world+0x1e8,vtable) # embedded deferred queue, not pointer-to-object.
 def scheduled(mm):
  assert mm.ecx()==world+0x1e8
  slot=reverse[mm.arg(1)];delay=mm.arg(2);assert mm.arg(3)==0
  trace.append(dict(call='schedule',slot=slot,delay=delay,data=0))
  node=new_node(slot);mm.ret(node,pop=12)
 m.w(vtable+12,m.stub(scheduled))
 def schedule_entry(mm):
  slot=reverse[mm.ecx()]
  if c['fail']=='schedule:'+str(slot):
   trace.append(dict(call='schedule_required',slot=slot,delay=mm.arg(1)));halt('schedule:'+str(slot))
 m.observe(0x50acc0,schedule_entry)
 def clear(mm):trace.append(dict(call='clear',slot=reverse[mm.ecx()]))
 m.observe(0x50b250,clear)
 def free(mm):
  node=mm.arg(1)
  if node in nodes:freed.append(nodes[node][0])
  mm.ret()
 m.seam(0x6f7430,free)
 try:returned=bool(m.call(ENTRY[c['class_id']],obj,[0])&255)
 except AssertionError:
  if stop is None:raise
  returned=None
 pending=[]
 for slot in range(2):
  values=[];node=m.r(components[slot]+16) if slot in components else 0
  while node:values.append(nodes[node][0]);node=m.r(node+16)
  pending.append(values)
 output=dict(returned=returned,stop=stop,trace=trace,counters=[m.r(world+0x180),m.r(world+0x190)],
   rng=m.r(world+0xd4),pending=pending,freed=freed)
 return output,m.visited
def main():
 rows=[];pins={}
 for c in cases():
  for pc in nm.PRECISIONS:
   a,pa=run(c,'original',pc);b,pb=run(c,'v14',pc);assert a==b
   rows.append(dict(input=c,precision=pc,output=a));pins.update(pa);pins.update(pb)
 for va,raw in pins.items():assert ORIGINAL.range(va,len(raw))==nm.modified.range(va,len(raw))==raw
 doc=dict(schema=1,scope=__doc__,paired_cases=len(rows),native_executions=len(rows)*2,cases=rows,
  instructions=[dict(va=hex(va),bytes=raw.hex()) for va,raw in sorted(pins.items())])
 path=HERE/'native-special-tail.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=doc['native_executions'],pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
