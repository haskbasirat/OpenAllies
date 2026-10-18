"""4FD8C0 actual ordered property scan stops BEFORE required404F60 diagnostic.

Original/v14 separate fresh machines at all x87 precisions. Successful seams
are allocator/delete, CString manager/empty/free transport and ordinary owner
virtual108. Queue constructors, native parameter constructors,503080 filtering
and all property scan/control instructions execute. The diagnostic BODY never
executes; its failure/unwinding is not claimed. This certifies the required stop
point precedes later property reads, required-object lookup and queue append.
"""
from pathlib import Path
import hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-metagame'));import mg_machine as h
from unicorn import UC_HOOK_MEM_READ
ORIGINAL=h.P;V14=h.V
class Required(Exception):pass

def run(case,image,cw):
 h.P=ORIGINAL if image=='original'else V14;h.VISITED.clear();m=h.Machine(cw)
 successful=set();reads=[];warned=None
 def seam(va,fn):
  def call(mm):successful.add(va);fn(mm)
  m.seams[va]=call
 def stub(fn):
  p=m.alloc(16);m.u.mem_write(p,b'\xc3');seam(p,fn);return p
 manager=m.alloc(8);vt=m.alloc(32);m.w32(manager,vt)
 def empty(mm):
  p=mm.alloc(18);mm.w32(p,manager);mm.w32(p+12,1);mm.ret(0,p)
 m.w32(vt+12,stub(empty));m.w32(vt+16,stub(lambda mm:mm.ret(0,manager)))
 m.w32(vt+4,stub(lambda mm:mm.ret(4)))
 seam(0x856370,lambda mm:mm.ret(0,manager))
 seam(0x6f7810,lambda mm:mm.ret(0,mm.alloc(max(4,mm.arg(0)))))
 seam(0x6f7430,lambda mm:mm.ret())
 queue=m.alloc(24)
 for off in [0,12]:m.call(0x4f5160,[8,1],queue+off)
 source=m.alloc(0x200);svt=m.alloc(0x110);m.w32(source,svt)
 current=m.alloc(0x600);replacement=m.alloc(0x600)
 m.w32(source+4,current);m.w32(source+0x14,7);m.w32(replacement+0x4e0,current)
 m.w32(svt+0x108,stub(lambda mm:mm.ret(0,777)))
 component=m.alloc(32);m.w32(source+0x78,component);instances=[]
 for index,entry in enumerate(case['properties']):
  p=m.alloc(32);definition=m.alloc(0x20);m.w32(definition+8,900+index)
  m.w32(p+8,entry['kind']);m.w32(p+12,definition);m.w32(p+0x14,entry['source']);instances.append(p)
 m.w32(component+0x10,m.array(instances));m.w32(component+0x14,len(instances))
 def read(u,access,address,size,value,_):
  for index,p in enumerate(instances):
   if p+8<=address<p+0x18:reads.append([index,address-p,size])
  if address==replacement+0x4e0:reads.append(['required_object',0x4e0,size])
 m.u.hook_add(UC_HOOK_MEM_READ,read)
 def stop(mm):
  nonlocal warned
  warned=[mm.arg(5),mm.arg(6)];raise Required('404F60 actual diagnostic required')
 m.seams[0x404f60]=stop;stopped=None
 try:m.call(0x4fd8c0,[source,replacement],queue)
 except RuntimeError as e:
  if not isinstance(e.__cause__,Required):raise
  stopped=str(e.__cause__)
 assert stopped and warned
 out=dict(stopped=stopped,warning=warned,reads=reads,queue_counts=[m.r32(queue+4),m.r32(queue+16)])
 assert out['queue_counts']==[0,0]
 assert all(r[0]!='required_object'for r in reads)
 assert all(isinstance(r[0],int)and r[0]<=case['warning_index']for r in reads)
 return out,h.VISITED.copy(),{s for s in successful if 0x400000<=s<0x1000000}

def main():
 rows=[];pins={};seams=set()
 cases=[dict(properties=[dict(kind=1,source=99),dict(kind=2,source=7)],warning_index=0),
  dict(properties=[dict(kind=2,source=0),dict(kind=2,source=7),dict(kind=3,source=88),dict(kind=1,source=0)],warning_index=2),
  dict(properties=[dict(kind=4,source=88),dict(kind=0,source=99),dict(kind=1,source=88),dict(kind=2,source=0)],warning_index=2)]
 for pc,cw in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
  for case in cases:
   a,pa,sa=run(case,'original',cw);b,pb,sb=run(case,'v14',cw);assert a==b
   rows.append(dict(input=case,precision=pc,output=a));pins.update(pa);pins.update(pb);seams|=sa|sb
 executed=[]
 for va,raw in sorted(pins.items()):
  if va in seams or va==0x404f60:continue
  assert ORIGINAL.range(va,len(bytes.fromhex(raw)))==V14.range(va,len(bytes.fromhex(raw)))==bytes.fromhex(raw)
  executed.append(dict(va=hex(va),bytes=raw))
 doc=dict(schema=1,scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
  original_sha256=h.ORIGINAL_SHA256,v14_sha256=h.V14_SHA256,cases=rows,separate_machine_runs=2*len(rows),
  executed_instructions=executed,successful_in_image_transport_seams=[hex(s)for s in sorted(seams)],
  required_boundary=dict(va='0x404f60',bytes=ORIGINAL.range(0x404f60,6).hex()))
 path=HERE/'native-warning-prefix.json'
 if '--write'in sys.argv:
  with path.open('x',encoding='utf-8')as f:f.write(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text(encoding='utf-8'))==doc
 print(json.dumps(dict(rows=len(rows),machine_runs=2*len(rows),executed_pins=len(executed),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
