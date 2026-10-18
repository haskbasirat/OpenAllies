"""Whole4E3C70,4F1430,4D0900,4E3AB0 with actual Data vf24 leaves.
Only heap allocation/free transport is replaced. Real original/v14 vtables
select6A71F0(ThingData) or462E50(Object/ActorData byte34); no predicate seam.
Synthetic generation/pointer storage models the exact valid/stale/null domain.
"""
from pathlib import Path
import sys,json,itertools,struct,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
H=lambda b:hashlib.sha256(b).hexdigest()
class Machine(h.Machine):
 def run(self,c):
  k=self.alloc(0x340);a=self.alloc(0x200);target=self.alloc(0x200);data=self.alloc(0x40);table=self.alloc(0x60008)
  self.w32(0xb364c0,table)
  current=0x10001;required=0x20002
  self.w32(a+0x14,current);self.w32(target+0x14,required)
  self.w32(a+4,data);self.w32(target+4,data)
  self.w32(data,0xa20770 if c['thing']else 0xa05540);self.u.mem_write(data+0x34,bytes([c['predicate']]))
  for id,p in[(current,a),(required,target)]:
   self.u.mem_write(table+4+(id&65535)*2,struct.pack('<H',id>>16));self.w32(table+0x20004+(id&65535)*4,p)
  ref={'null':0,'self':current,'target':required,'stale':0x30002,'absent':required}[c['reference']]
  if c['reference']=='absent':self.w32(table+0x20004+2*4,0)
  self.w32(a+0x158,ref);self.w32(a+0xd4,self.alloc(4)if c['city']else 0)
  head=0;tail=0
  for id in reversed(c['list']):
   node=self.alloc(8);self.w32(node,id,head);head=node
   if tail==0:tail=node
  self.w32(k+0x30c,head,tail);freed=[]
  self.seams[0x6f7810]=lambda m:m.ret(value=m.alloc(m.arg()))
  self.seams[0x6f7430]=lambda m:(freed.append(m.arg()),m.ret())
  self.call(0x4e3c70,k,[a,int(c['add'])])
  values=[];nodes=[];p=self.r32(k+0x30c)
  while p:
   assert p not in nodes;nodes.append(p);values.append(self.r32(p));p=self.r32(p+4)
  assert self.r32(k+0x310)==(nodes[-1]if nodes else 0)
  count=self.call(0x4e3ab0,k,[])
  return dict(values=values,count=count,freed=len(freed),head_null=not nodes,tail_null=self.r32(k+0x310)==0)
def main():
 h.VISITED.clear();rows=[]
 lists=[[],[0x10001],[0x20002],[0x10003],[0x10001,0x20002,0x10001]]
 for pc,ref,thing,predicate,city,add,prior in itertools.product(h.PRECISION,['null','self','target','stale','absent'],[False,True],[0,1], [False,True],[False,True],lists):
  c=dict(reference=ref,thing=thing,predicate=predicate,city=city,add=add,list=prior)
  h.P=ORIGINAL;a=Machine(pc).run(c)
  h.P=V14;b=Machine(pc).run(c);assert a==b,(pc,c,a,b)
  rows.append(dict(precision=pc,input=c,output=a))
 h.P=ORIGINAL
 excluded={0x6f7810,0x6f7430}
 pins={hex(a):raw for a,raw in sorted(h.VISITED.items())if a not in excluded}
 for a,b in pins.items():assert ORIGINAL.range(int(a,16),len(bytes.fromhex(b)))==V14.range(int(a,16),len(bytes.fromhex(b)))
 doc=dict(schema=1,scope=__doc__,producer_sha256=H(Path(__file__).read_bytes()),rows=rows,pins=pins)
 data=(json.dumps(doc,indent=1)+'\n').encode();path=HERE/'native-kingdom-ownership.json'
 if '--write'in sys.argv:path.write_bytes(data)
 else:assert path.read_bytes()==data
 print(json.dumps(dict(pairs=len(rows),executions=len(rows)*2,pins=len(pins),sha256=H(data))))
ORIGINAL=h.P;V14=h.V
if __name__=='__main__':main()
