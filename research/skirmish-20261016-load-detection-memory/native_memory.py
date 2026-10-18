"""Actual4EBC70 traversal +4EA3E0/4EB9F0/4ECA80 partial create original/v14/allPC.

Refresh eligibility and create/delete/clear virtual service answers are explicit
test inputs. Refresh itself, null-hole iterator and shared fog queries execute.
Separate allocation/record constructor and create-prefix rows execute the real
child functions; the unavailable virtual24 name stops AFTER timestamp store.
"""
from pathlib import Path
import sys,json,hashlib,random,copy
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'))
import native_machine as nm
ORIGINAL=nm.original
def cases():
 rng=random.Random(0x4ebc70);out=[]
 for i in range(72):
  count=i%5;k=[]
  for n in range(count):
   k.append(None if (i+n)%7==0 else dict(id=n+1,index=n,nonmajor=int((i+n)%11==0),shared=[1,0x10001,0x30003,0xffffffff][(i+n)%4]))
  slots=[rng.choice([False,False,True]) for _ in range(count)] if i%3==0 else None
  out.append(dict(name='refresh_'+str(i),eligible=i%4!=0,word=[0,1,0xffff,0xffff0000,0xffffffff,0x20002][i%6],
   flags=[0,1,128,0x20000000][i%4],kingdoms=k,slots=slots,fail='create' if i%19==0 else ('delete' if i%23==0 else None)))
 out.append(dict(name='clear_last',eligible=True,word=0,flags=0,kingdoms=[dict(id=1,index=0,nonmajor=0,shared=0x10001)],slots=[True],fail=None))
 out.append(dict(name='required_create',eligible=True,word=1,flags=0,kingdoms=[dict(id=1,index=0,nonmajor=0,shared=0x10001)],slots=None,fail='create'))
 actual=HERE/'actual-first-object-entry.json'
 if actual.exists():out.append(json.loads(actual.read_text())['input'])
 return out
def refresh(c,image,pc):
 nm.original=ORIGINAL if image=='original' else nm.modified;m=nm.Machine(pc)
 obj=m.alloc(0x100);vt=m.alloc(0x160);m.w(obj,vt);m.w(obj+8,c['flags']);m.w(obj+0x54,c['word'])
 world=m.alloc(0x300);m.w(0xb36534,world);kingdoms=[];by_ptr={}
 for row in c['kingdoms']:
  if row is None:kingdoms.append(0);continue
  k=m.alloc(0x300);kingdoms.append(k);by_ptr[k]=row;m.w(k+0x18c,row['index']);m.w(k+0x190,row['nonmajor']);m.w(k+0x228,row['shared'])
 m.w(world+0x154,m.array(kingdoms));m.w(world+0x158,len(kingdoms))
 slots=list(c['slots']) if c['slots'] is not None else None
 allocation=m.array([0x7777 if x else 0 for x in slots]) if slots is not None else 0
 m.w(obj+0x58,len(slots) if slots is not None else 0);m.w(obj+0x5c,allocation)
 trace=[];stop=None
 def halt(label):
  nonlocal stop
  stop=label;m.u.emu_stop()
 def keeps(mm):trace.append(['keeps',m.r(obj+0x58)>0]);mm.ret(int(m.r(obj+0x58)>0))
 def eligible(mm):trace.append(['eligible',c['eligible']]);mm.ret(int(c['eligible']))
 m.w(vt+0x8c,m.stub(keeps));m.seam(0x4eb490,eligible)
 def create(mm):
  nonlocal allocation
  row=by_ptr[mm.arg(1)];trace.append(['create',row['id']])
  if c['fail']=='create':halt('create');return
  if not m.r(obj+0x58):
   allocation=m.array([0]*len(kingdoms));m.w(obj+0x5c,allocation);m.w(obj+0x58,len(kingdoms))
  m.w(allocation+4*row['index'],0x7777);mm.ret(pop=4)
 def delete(mm):
  row=by_ptr[mm.arg(1)];trace.append(['delete',row['id']])
  if c['fail']=='delete':halt('delete');return
  m.w(allocation+4*row['index'],0);mm.ret(pop=4)
 def clear(mm):
  trace.append(['clear']);m.w(obj+0x58,0);m.w(obj+0x5c,0);mm.ret()
 m.w(vt+0x148,m.stub(create));m.w(vt+0x14c,m.stub(delete));m.seam(0x4ebb80,clear)
 try:m.call(0x4ebc70,obj)
 except AssertionError:
  if stop is None:raise
 actual_slots=[bool(m.r(m.r(obj+0x5c)+4*i)) for i in range(m.r(obj+0x58))] if m.r(obj+0x5c) else None
 return dict(trace=trace,stop=stop,count=m.r(obj+0x58),slots=actual_slots),m.visited
def construction(image,pc,kind,count):
 nm.original=ORIGINAL if image=='original' else nm.modified;m=nm.Machine(pc)
 obj=m.alloc(0x100);world=m.alloc(0x300);m.w(0xb36534,world);m.w(world+0x158,count);m.w(world+0xe0,0x41280000)
 vt=m.alloc(0x160);m.w(obj,vt);m.w(vt+0x8c,0x4ea3d0);allocations=[];stop=None
 def malloc(mm):
  size=mm.arg(1);p=m.alloc(size);allocations.append([p,size]);mm.ret(p)
 m.seam(0x6f7810,malloc)
 empty=m.alloc(18);allocator=m.alloc(16);avt=m.alloc(16);m.w(allocator,avt);m.w(avt+12,m.stub(lambda mm:mm.ret(empty)))
 m.seam(0x856370,lambda mm:mm.ret(allocator))
 if kind=='record':
  record=m.alloc(56);m.u.mem_write(record,b'\xcd'*56);m.call(0x4eb9f0,record)
  out=dict(words=[m.r(record+4*i) for i in range(14)],empty_name=m.r(record+4)==empty+16)
 elif kind=='slots':
  m.call(0x4ea3e0,obj);out=dict(count=m.r(obj+0x58),allocated=m.r(obj+0x5c)!=0,
   slots=[m.r(m.r(obj+0x5c)+4*i) for i in range(count)])
 elif kind=='create_prefix':
  kingdom=m.alloc(0x300);m.w(kingdom+0x190,0);m.w(kingdom+0x18c,0)
  m.w(obj+8,1<<29);m.w(vt+0x24,m.stub(lambda mm:halt('name')))
  m.seam(0x4eb490,lambda mm:mm.ret(1))
  def halt(label):
   nonlocal stop
   stop=label;m.u.emu_stop()
  try:m.call(0x4eca80,obj,[kingdom])
  except AssertionError:
   if stop is None:raise
  record=next(p for p,size in allocations if size==56)
  out=dict(stop=stop,count=m.r(obj+0x58),slots=[m.r(m.r(obj+0x5c)+4*i) for i in range(count)],
      record_words=[m.r(record+4*i) for i in range(14)],empty_name=m.r(record+4)==empty+16)
  # Definition/name pointer transport is represented by its constructor fact.
  out['record_words'][1]=None
 else:
  kingdom=m.alloc(0x300);m.w(kingdom+0x18c,0);m.w(kingdom+0x190,0);m.w(0xb36528,0)
  record=m.alloc(56);slots=m.array([record if kind=='delete_record' else 0]);m.w(obj+0x58,1);m.w(obj+0x5c,slots)
  freed=[]
  m.seam(0x671750,lambda mm:mm.ret()) # owned record's empty-string destructor transport.
  def free(mm):freed.append('record' if mm.arg(1)==record else 'slots');mm.ret()
  m.seam(0x6f7430,free)
  m.call(0x4eb970 if kind=='delete_record' else 0x4ebb80,obj,[kingdom] if kind=='delete_record' else [])
  out=dict(count=m.r(obj+0x58),allocated=m.r(obj+0x5c)!=0,slot=bool(m.r(slots)),freed=freed)
 if kind=='record':out['words'][1]=None
 return out,m.visited
def main():
 rows=[];pins={};static=[]
 for vt in (0xa2a2a0,0xa273f0,0xa299d8,0xa28a00,0xa28b88,0xa28e90):
  for slot in (0x8c,0x148,0x14c):
   va=vt+slot;raw=ORIGINAL.range(va,4);assert raw==nm.modified.range(va,4)
   assert int.from_bytes(raw,'little')=={0x8c:0x4ea3d0,0x148:0x4eca80,0x14c:0x4ce8c0 if vt in (0xa273f0,0xa299d8) else 0x4eb970}[slot]
   static.append(dict(va=hex(va),bytes=raw.hex()))
 for pc in nm.PRECISIONS:
  for c in cases():
   a,pa=refresh(c,'original',pc);b,pb=refresh(c,'v14',pc);assert a==b;pins.update(pa);pins.update(pb)
   rows.append(dict(operation='refresh',precision=pc,input=c,output=a))
  for kind,count in [('record',0)]+[('slots',n) for n in (0,1,3,16)]+[('create_prefix',3),('delete_record',1),('release_slots',1)]:
   a,pa=construction('original',pc,kind,count);b,pb=construction('v14',pc,kind,count);assert a==b;pins.update(pa);pins.update(pb)
   rows.append(dict(operation=kind,precision=pc,count=count,output=a))
 for va,raw in pins.items():assert ORIGINAL.range(va,len(raw))==nm.modified.range(va,len(raw))==raw
 doc=dict(schema=1,scope=__doc__,paired_rows=len(rows),native_executions=len(rows)*2,cases=rows,
  instructions=[dict(va=hex(va),bytes=raw.hex()) for va,raw in sorted(pins.items())],static=static)
 path=HERE/'native-memory.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=doc['native_executions'],pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
