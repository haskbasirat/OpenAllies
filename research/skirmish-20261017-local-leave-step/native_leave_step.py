"""Actual752A60/7514F0 vote-list constructors, fresh770500/74DA10 and null775DB0.
History constructors/reset run natively; CString provider/copy, allocation,
deallocation and diagnostics are explicit successful transport services.
"""
from pathlib import Path
import json,sys,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
from unicorn.x86_const import UC_X86_REG_ESI,UC_X86_REG_EBX,UC_X86_REG_EIP
class Cutoff(Exception):pass
def machine(image,pc):
 m=nm.Machine(pc)
 if image=='v14':
  old=(nm.original.image_size+0xfff)&~0xfff;new=(nm.modified.image_size+0xfff)&~0xfff
  if new>old:m.u.mem_map(nm.original.base+old,new-old)
  for vs,rva,raw,off in nm.modified.sections:m.u.mem_write(nm.modified.base+rva,nm.modified.b[off:off+raw])
 pool=m.alloc(0x20);vt=m.alloc(0x20);m.w(pool,vt)
 empty=m.alloc(18);m.w(empty+12,1000000)
 m.w(vt+12,m.stub(lambda mm:mm.ret(empty)))
 m.seam(0x856370,lambda mm:mm.ret(pool))
 m.seam(0x405b80,lambda mm:(m.w(mm.ecx(),empty+16),mm.ret(mm.ecx(),4)))
 m.seam(0x6f7810,lambda mm:mm.ret(m.alloc(mm.arg(1))))
 m.seam(0x6f7430,lambda mm:mm.ret())
 m.seam(0x404f60,lambda mm:mm.ret())
 return m
def vote_list(m,peer,count,links):
 head=0;tail=0
 for value in reversed(range(count)):
  node=m.array([0x1000+value,head]);links[node]=(peer,value);head=node
  if tail==0:tail=node
 m.w(peer+0x64,head);m.w(peer+0x68,tail)
def values(m,p):
 out=[];head=m.r(p+0x64);last=0
 while head:out.append(m.r(head));last=head;head=m.r(head+4)
 assert m.r(p+0x68)==last
 return out
def run(c,image,pc):
 m=machine(image,pc);events=[]
 if c['kind']=='task':
  s=m.alloc(0x180);m.u.mem_write(s,b'\xcd'*0x180)
  m.observe(0x76f998,lambda mm:(mm.u.reg_write(UC_X86_REG_ESI,s),mm.u.reg_write(UC_X86_REG_EBX,0)))
  m.observe(0x76f99e,lambda mm:(_ for _ in ()).throw(Cutoff()))
  try:m.call(0x76f998);raise AssertionError('missing ctor-store cutoff')
  except Cutoff:pass
  assert m.r(s+0x124)==0
  m.call(0x775db0,s,[c['block']]);output=dict(task_null=m.r(s+0x124)==0,side_effects=events)
 else:
  admin=m.alloc(0x310);m.u.mem_write(admin,b'\xcd'*0x310);m.call(0x752a60,admin)
  assert values(m,admin)==[]
  if c['kind']=='constructor':output=dict(head=m.r(admin+0x64),tail=m.r(admin+0x68),order_version=m.rb(admin+0x6c))
  else:
   peers=[admin]+[m.alloc(0x310)for _ in c['counts'][1:]];links={}
   for i,(p,count) in enumerate(zip(peers,c['counts'])):
    if i:m.u.mem_write(p,b'\xcd'*0x310);m.call(0x752a60,p)
    vote_list(m,p,count,links)
   def release(mm):
    p,value=links[mm.arg(1)];events.append(dict(peer=peers.index(p),value=value,remaining=values(m,p)));mm.ret()
   m.seam(0x6f7430,release)
   if c['kind']=='reset':
    assert len(peers)==1;m.call(0x7514f0,admin)
   else:
    pair_head=0;pair_tail=0
    for p in reversed(peers):
     link=m.array([p,pair_head]);pair_head=link
     if pair_tail==0:pair_tail=link
    m.w(admin+0x2ac,pair_head);m.w(admin+0x2b0,pair_tail);m.w(0xb36558,admin)
    s=m.alloc(0x180);m.u.mem_write(s,b'\xcd'*0x180);m.w(s+0xe4,c['state']);m.w(s+0x54,2);m.w(s+0x58,2)
    m.call(0x770500,s)
    assert m.r(admin+0x2ac)==pair_head and m.r(admin+0x2b0)==pair_tail
   output=dict(votes=[values(m,p)for p in peers],retired=events,peer_list_unchanged=True)
 assert not m.null_accesses
 return output,m.visited
def main():
 cases=[dict(kind='constructor')]+[dict(kind='task',block=b)for b in [0,1,255]]
 cases +=[dict(kind='reset',counts=[n])for n in [0,1,3]]
 cases +=[dict(kind='step',state=s,counts=v)for s,v in itertools.product([0,3,6,0xffffffff],[[0],[1],[3],[2,0,1]])]
 rows=[];pins={}
 for pc,c in itertools.product(nm.PRECISIONS,cases):
  a,pa=run(c,'original',pc);b,pb=run(c,'v14',pc);assert a==b;pins.update(pa);pins.update(pb);rows.append(dict(precision=pc,input=c,output=a))
 for va,raw in pins.items():assert nm.original.range(va,len(raw))==nm.modified.range(va,len(raw))==raw
 #Only EBX writer before the ctor124 store is XOR EBX,EBX76F8C7;
 #intervening callees have the original callee-save EBX contract.
 static=nm.original.range(0x76f8c7,0xd7);assert static==nm.modified.range(0x76f8c7,len(static))
 doc=dict(schema=1,scope=__doc__,rows=rows,native_executions=2*len(rows),static_ctor_prefix=dict(va='0x76f8c7',bytes=static.hex()),instructions=[dict(va=hex(v),bytes=b.hex(),observed_cutoff=v==0x76f99e)for v,b in sorted(pins.items())])
 path=HERE/'native-leave-step.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=2*len(rows),pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
