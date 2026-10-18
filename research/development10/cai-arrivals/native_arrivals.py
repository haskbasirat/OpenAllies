"""Whole524B80 and4D0900 actual arrival list writes, required virtual/522890
children explicit. Caller mutates current list/mode/GTID before the native tail.
Success allocation only; cutoffs are service boundaries, not C++ unwinding.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn import UC_HOOK_MEM_WRITE,UC_HOOK_MEM_READ,UC_HOOK_CODE
OLD=h.P;V14=h.V
class Cutoff(Exception):pass
class M(h.Machine):
 def list(self):
  head=self.r32(self.mind+0x2bc);tail=self.r32(self.mind+0x2c0);values=[];last=0
  while head:
   assert head not in self.seen;self.seen.add(head);values.append(self.r32(head));last=head;head=self.r32(head+4)
  assert last==tail,(values,hex(last),hex(tail))
  return values
 def run(self,c):
  self.mind=self.alloc(0x300);member=self.alloc(0x100);mmind=self.alloc(0x50);vt=self.alloc(0x50);phase=self.alloc(0xf0)
  self.w32(0xb36550,phase);self.w32(phase+0xe4,c['mode']);self.w32(self.mind+0x24,0x89abcdef);self.w32(self.mind+0x28,0x10001)
  reg=self.alloc(0x60010);self.w32(0xb364c0,reg);self.u.mem_write(reg+6,b'\x01\x00');self.w32(reg+0x20008,0)
  self.w32(member+0x70,mmind);self.w32(mmind,vt);self.w32(vt+0x3c,h.STUBS+0x100);self.w32(vt+0x40,h.STUBS+0x200);self.w32(member+0x14,c['gtid'])
  head=tail=0
  for g in reversed(c['initial']):
   n=self.alloc(8);self.w32(n,g,head);head=n
   if not tail:tail=n
  self.w32(self.mind+0x2bc,head,tail);self.trace=[];self.writes=[];self.allocs=[]
  def puthead(g):
   n=self.alloc(8);self.w32(n,g,self.r32(self.mind+0x2bc));self.w32(self.mind+0x2bc,n)
   if not self.r32(self.mind+0x2c0):self.w32(self.mind+0x2c0,n)
  def first(m):assert m.ecx()==mmind;m.trace.append(['3c',m.arg(0)]);m.ret(4)
  def second(m):assert m.ecx()==mmind;m.trace.append(['40',m.arg(0)]);m.ret(4)
  def helper(m):
   assert m.ecx()==self.mind and m.arg(0)==member and m.arg(1)==0;m.trace.append(['522890'])
   if c['fail']=='helper':raise Cutoff()
   if c['mutation']=='front':puthead(0x76543210)
   if c['mutation']=='mode3':m.w32(phase+0xe4,3)
   if c['mutation']=='gtid':m.w32(member+0x14,c['gtid']^0xffffffff)
   m.ret(8)
  def alloc(m):
   assert m.arg(0)==8;m.trace.append(['allocate',8])
   if c['fail']=='allocate':raise Cutoff()
   p=m.alloc(8);m.allocs.append(p);m.ret(value=p)
  def write(u,access,a,size,v,_):
   if a in [self.mind+0x2bc,self.mind+0x2c0]:self.writes.append(['head'if a==self.mind+0x2bc else'tail',int(v!=0)])
  self.u.hook_add(UC_HOOK_MEM_WRITE,write)
  self.seams[h.STUBS+0x100]=first;self.seams[h.STUBS+0x200]=second;self.seams[0x522890]=helper;self.seams[0x6f7810]=alloc
  err=None
  try:self.call(0x524b80,self.mind,[member])
  except Cutoff:err=c['fail']
  self.seen=set();values=self.list()
  return dict(values=values,head=values[0]if values else None,tail=values[-1]if values else None,trace=self.trace,writes=self.writes,error=err)
class Drain(M):
 def run(self,c):
  self.mind=self.alloc(0x300);state=self.mind+0x34;self.w32(state+4,self.mind);self.w32(self.mind+4,self.alloc(0x100))
  reg=self.alloc(0x60010);self.w32(0xb364c0,reg);self.trace=[];self.writes=[];self.objects={};self.freed=[];head=tail=0
  for i,g in enumerate(c['values']):
   index=g&0xffff;generation=g>>16
   if index!=0 and i%3!=1:
    obj=self.alloc(0x100);member=self.alloc(0x20);self.w32(obj+0x80,member);self.w32(member+0x10,self.r32(self.mind+4)if i%3==0 else 0)
    self.u.mem_write(reg+4+2*index,generation.to_bytes(2,'little'));self.w32(reg+0x20004+4*index,obj);self.objects[obj]=g
  for g in reversed(c['values']):
   n=self.alloc(8);self.w32(n,g,head);head=n
   if not tail:tail=n
  self.w32(self.mind+0x2bc,head,tail)
  def free(m):
   node=m.arg(0);self.trace.append(['free',m.r32(node)]);self.freed.append(m.r32(node));m.ret()
  def org(m):assert m.ecx()==state;m.ret(value=self.r32(self.mind+4))
  def arrived(m):
   assert m.ecx()==self.mind and m.arg(1)==0;g=self.objects[m.arg(0)];self.trace.append(['arrived',g])
   if c['fail_after']==g:raise Cutoff()
   m.ret(8)
  def resolve(u,access,a,size,v,_):
   if reg+4<=a<reg+0x20004 and size==2:
    node=self.r32(self.mind+0x2bc);self.trace.append(['resolve',self.r32(node)])
  def write(u,access,a,size,v,_):
   if a in[self.mind+0x2bc,self.mind+0x2c0]:self.trace.append(['head'if a==self.mind+0x2bc else'tail',int(v!=0)])
  def stop(u,a,size,_):
   if a==0x524fe7:raise Cutoff()
  self.u.hook_add(UC_HOOK_MEM_READ,resolve);self.u.hook_add(UC_HOOK_MEM_WRITE,write);self.u.hook_add(UC_HOOK_CODE,stop)
  self.seams[0x6f7430]=free;self.seams[0x529c30]=org;self.seams[0x522890]=arrived
  self.u.reg_write(h.UC_X86_REG_ESP,h.STACK);self.u.reg_write(h.UC_X86_REG_ESI,state);self.u.reg_write(h.UC_X86_REG_EBX,0)
  try:self.u.emu_start(0x524f53,h.STOP,count=10000)
  except Cutoff:pass
  self.seen=set();values=self.list()
  return dict(values=values,trace=self.trace,freed=self.freed)
def main():
 h.VISITED.clear();rows=[];drain=[]
 cases=[dict(initial=v,mode=m,gtid=g,mutation=t,fail=None)for v,m,g,t in itertools.product([[],[7],[0,7,7]],[0,2,3,0xffffffff],[0,1689,0xffffffff],[None,'front','mode3','gtid'])]
 cases += [dict(initial=v,mode=2,gtid=1689,mutation='front',fail=f)for v,f in itertools.product([[],[7,9]],['helper','allocate'])]
 for pc,c in itertools.product(h.PRECISION,cases):
  h.P=OLD;a=M(pc).run(c);h.P=V14;b=M(pc).run(c);assert a==b,(pc,c,a,b);rows.append(dict(pc=pc,input=c,output=a))
 for pc,v,f in itertools.product(h.PRECISION,[[],[0],[0x10001],[0x10001,0x20002,0x30003],[0x10001,0x10001,0x30003]],[None,0x10001]):
  c=dict(values=v,fail_after=f);h.P=OLD;a=Drain(pc).run(c);h.P=V14;b=Drain(pc).run(c);assert a==b,(pc,c,a,b);drain.append(dict(pc=pc,input=c,output=a))
 h.P=OLD;seams=[0x522890,0x6f7810,0x6f7430,0x529c30,0x524fe7]
 pins={hex(a):v for a,v in sorted(h.VISITED.items())if a not in seams}
 doc=dict(schema=1,scope=__doc__,rows=rows,drain_rows=drain,pins=pins,excluded_entries=[hex(a)for a in seams],producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
 raw=(json.dumps(doc,indent=2)+'\n').encode();p=HERE/'native-arrivals.json'
 if '--write'in sys.argv:p.write_bytes(raw)
 else:assert p.read_bytes()==raw
 print(json.dumps(dict(push_rows=len(rows),drain_rows=len(drain),runs=2*(len(rows)+len(drain)),executed_pins=len(pins),sha256=hashlib.sha256(raw).hexdigest())))
if __name__=='__main__':main()
