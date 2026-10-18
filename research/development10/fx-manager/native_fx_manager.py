"""Original50CE00 header and50C800 submission/list order with explicit seams.
Constructor unused-capacity element lifecycle is a named seam; no element read.
No FXC graph is compiled and no renderer/audio success is asserted.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
ORIGINAL=h.P

def constructor(precision,fill,image):
 h.P=ORIGINAL if image=='original' else h.V;m=h.Machine(precision)
 manager=m.alloc(32);m.u.mem_write(manager,bytes([fill])*32);calls=[]
 def allocate(mm):
  assert mm.arg()==0x134;calls.append('allocate_capacity');mm.ret(0,mm.alloc(0x134))
 def unused_elements(mm):
  assert [mm.arg(i) for i in range(1,5)]==[0x4c,4,0x50ca40,0x50caa0]
  calls.append('unused_capacity_element_lifecycle');mm.ret(20)
 m.seams.update({0x6f7810:allocate,0x96d18d:unused_elements})
 assert m.call(0x50ce00,manager)==manager
 assert calls==['allocate_capacity','unused_capacity_element_lifecycle']
 return dict(muted=m.r8(manager),view=m.r32(manager+4),head=m.r32(manager+8),tail=m.r32(manager+12),
             active_count=m.r32(manager+0x14),enabled=m.r8(manager+0x1c),capacity=m.r32(manager+0x18),
             unused_storage_present=m.r32(manager+0x10)!=0,calls=calls)

def run(c,image):
 h.P=ORIGINAL if image=='original' else h.V;m=h.Machine(c['precision'])
 manager=m.alloc(32);view=m.alloc(4);m.w8(manager,int(c['muted']));m.w32(manager+4,view if c['view'] else 0)
 prior=[]
 for value in [11,0,12]:
  node=m.alloc(8);m.w32(node,value,0)
  if prior:m.w32(prior[-1]+4,node)
  prior.append(node)
 m.w32(manager+8,prior[0],prior[-1]);calls=[]
 def allocate(mm):assert mm.arg()==8;mm.ret(0,mm.alloc(8))
 def free(mm):mm.ret(0)
 def destroy(mm):calls.append(['destroy',mm.ecx()]);mm.ret(0)
 def deliver(mm):calls.append(['deliver',mm.arg()]);mm.ret(4)
 m.seams.update({0x6f7810:allocate,0x6f7430:free,0x428350:destroy,0x43b870:deliver})
 for value,flag in c['submissions']:m.call(0x50c800,manager,[value,flag])
 pending=[];node=m.r32(manager+8);seen=set();tail=0
 while node:
  assert node not in seen;seen.add(node);pending.append(m.r32(node));tail=node;node=m.r32(node+4)
 assert m.r32(manager+12)==tail
 return dict(calls=calls,pending=pending)

def main():
 rows=[]
 patterns=[[[21,0],[0,1],[22,255]],[[21,1],[22,1],[0,0]],[[0,0],[21,0],[22,0]],
           [[21,0],[22,1]],[[21,128]],[[0,255],[0,0]]]
 for precision,muted,view,submissions in itertools.product(h.PRECISION,[False,True],[False,True],patterns):
  c=dict(precision=precision,muted=muted,view=view,submissions=submissions)
  a=run(c,'original');b=run(c,'v14');assert a==b;rows.append(dict(input=c,output=a))
 constructors=[]
 for precision,fill in itertools.product(h.PRECISION,[0,0xa5,0xff]):
  a=constructor(precision,fill,'original');b=constructor(precision,fill,'v14');assert a==b
  constructors.append(dict(precision=precision,fill=fill,output=a))
 out=dict(schema=1,scope=__doc__,rows=rows,constructors=constructors,native_executions=(len(rows)+len(constructors))*2,
  original_sha256=hashlib.sha256(ORIGINAL.b).hexdigest(),v14_sha256=hashlib.sha256(h.V.b).hexdigest(),
  instructions=[dict(address=hex(a),bytes=b) for a,b in sorted(h.VISITED.items())])
 path=HERE/'native-fx-manager.json'
 if '--write' in sys.argv:path.write_text(json.dumps(out,indent=1)+'\n')
 else:assert json.loads(path.read_text())==out
 print(json.dumps(dict(rows=len(rows),constructors=len(constructors),executions=(len(rows)+len(constructors))*2,pins=len(h.VISITED),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
