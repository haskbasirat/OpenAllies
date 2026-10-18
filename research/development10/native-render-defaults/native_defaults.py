"""Actual8C3ED0 guard+12default constructors through cutoff8C3F20, then
895550/895380 shared reference retention. Only allocator/free transport and
post-prefix continuation are seams; no ctor/selection/value callback seams.
Ni bytefields/pointer identity are normalized; no full Ni runtime ABI claim.
"""
from pathlib import Path
import sys,json,itertools,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn import UC_HOOK_MEM_WRITE
SLOTS=[0xb43f3c,0xb442f8,0xb44304,0xb43f48,0xb44334,0xb44310,0xb4431c,0xb442ec,0xb43dfc,0xb43f1c,0xb43e08,0xb44328]
SIZES=[0x24,0x24,0x34,0x64,0x24,0x24,0x24,0x40,0x48,0x2c,0x24,0x28]
CALLS=[0x87d060,0x8898a0,0x889c70,0x87d500,0x88a7d0,0x889ef0,0x88a110,0x889550,0x86d690,0x87b7d0,0x86e7b0,0x88a3c0]
H=lambda b:hashlib.sha256(b).hexdigest()
class Machine(h.Machine):
 def run(self,c):
  written=set();calls=[];allocations=[]
  def writes(u,access,a,size,value,context):written.update(range(a,a+size))
  self.u.hook_add(UC_HOOK_MEM_WRITE,writes)
  self.u.hook_add(h.UC_HOOK_CODE,lambda u,a,size,c:calls.append(a)if a in CALLS else None)
  def allocate(m):
   n=m.arg();p=m.alloc(n);m.u.mem_write(p,b'\xa5'*n);allocations.append(n);m.ret(value=p)
  self.seams[0x6f7810]=allocate;self.seams[0x6f7430]=lambda m:m.ret()
  self.seams[0x8c3f20]=lambda m:m.ret() #stop BEFORE nextNi init/type-registration
  self.u.mem_write(0xb45f54,b'\0');self.w32(0xb45f14,0,);self.w32(0xb323c0,c['counter'])
  self.w32(0xb43e68,*c['fog'])
  for a in SLOTS:self.w32(a,0)
  for _ in range(c['repeat']):self.call(0x8c3ed0)
  pointers=[self.r32(a)for a in SLOTS];assert all(pointers)
  sets=[]
  for _ in range(c['sets']):
   p=self.alloc(0x38);self.call(0x895550,p);sets.append([self.r32(p+8+4*i)for i in range(12)])
  assert all(s==pointers for s in sets)
  props=[]
  for slot,(p,size)in enumerate(zip(pointers,SIZES)):
   opaque={0,4}|({0x28,0x2c,0x30}if slot==8 else set())
   full={str(off):self.r32(p+off)for off in range(8,size,4)if off not in opaque and all(p+off+i in written for i in range(4))}
   partial={str(off):self.u.mem_read(p+off,1)[0]for off in range(8,size)if (off//4)*4 not in opaque and p+off in written and str((off//4)*4)not in full}
   props.append(dict(fields=full,bytes=partial,refs=self.r32(p+4)))
  assert calls==CALLS
  return dict(guard=self.u.mem_read(0xb45f54,1)[0],calls=calls,material_next=self.r32(0xb323c0),
   ni_objects=self.r32(0xb45f14),properties=props,all_sets_share_current=True,allocations=allocations)
def main():
 h.VISITED.clear();rows=[]
 for pc,counter,fog,repeat,sets in itertools.product(h.PRECISION,[1],[[0,0,0],[0x3f800000,0x80000000,0x7fc00001]],[1,2],[1,2]):
  c=dict(counter=counter,fog=fog,repeat=repeat,sets=sets)
  h.P=ORIGINAL;a=Machine(pc).run(c);h.P=V14;b=Machine(pc).run(c);assert a==b
  rows.append(dict(precision=pc,input=c,output=a))
 h.P=ORIGINAL
 excluded={0x6f7810,0x6f7430,0x8c3f20}
 pins={hex(a):raw for a,raw in sorted(h.VISITED.items())if a not in excluded}
 for a,b in pins.items():assert ORIGINAL.range(int(a,16),len(bytes.fromhex(b)))==V14.range(int(a,16),len(bytes.fromhex(b)))
 doc=dict(schema=1,scope=__doc__,producer_sha256=H(Path(__file__).read_bytes()),rows=rows,pins=pins)
 data=(json.dumps(doc,indent=1)+'\n').encode();p=HERE/'native-defaults.json'
 if '--write'in sys.argv:p.write_bytes(data)
 else:assert p.read_bytes()==data
 print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,pins=len(pins),sha256=H(data))))
ORIGINAL=h.P;V14=h.V
if __name__=='__main__':main()
