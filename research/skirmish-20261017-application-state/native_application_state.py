"""Application40CE10 exact field/callback order, both executable images/allPC.
Diagnostic404F60 and actual table-node OnLeave+18/OnEnter+14 are explicit
controlled child seams; stops preserve prefix but do not emulate SEH/unwind.
Constructor pair proof executes the two actual4178F0/4178F6 stores with explicit
ESI/current allocation and EBX0 precondition, pinning its constructor producer.
"""
from pathlib import Path
import sys,json,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as n
from unicorn.x86_const import UC_X86_REG_ESI,UC_X86_REG_EBX
class Stop(Exception):pass

def machine(image,pc):
 m=n.Machine(pc)
 if image=='v14':
  old=(n.original.image_size+4095)&~4095;new=(n.modified.image_size+4095)&~4095
  if new>old:m.u.mem_map(n.original.base+old,new-old)
  for vs,rva,raw,off in n.modified.sections:m.u.mem_write(n.modified.base+rva,n.modified.b[off:off+raw])
 return m

def constructor(image,pc,poison):
 m=machine(image,pc);app=m.alloc(0x120);m.u.mem_write(app,bytes([poison])*0x120)
 def initialize(mm):mm.u.reg_write(UC_X86_REG_ESI,app);mm.u.reg_write(UC_X86_REG_EBX,0)
 m.observe(0x4178f0,initialize)
 def tail(mm):raise Stop('end-pair-fragment')
 m.seam(0x4178fc,tail)
 try:m.call(0x4178f0)
 except Stop:pass
 return dict(current=m.r(app+0x10c),previous=m.r(app+0x110)),m.visited

def run(c,image,pc):
 m=machine(image,pc);app=m.alloc(0x120);m.w(app+0x10c,c['current']);m.w(app+0x110,c['previous'])
 events=[]
 def fields():return [m.r(app+0x10c),m.r(app+0x110)]
 def mutate(label):
  v=c['mutate'].get(label)
  if v is not None:m.w(app+0x10c,v[0]);m.w(app+0x110,v[1])
 def diagnostic(mm):
  events.append(['diagnostic',mm.arg(6),mm.arg(7),*fields()]);mutate('diagnostic')
  if c['stop']=='diagnostic':raise Stop('diagnostic')
  mm.ret(0)
 m.seam(0x404f60,diagnostic)
 def leave(index,mm):
  events.append(['leave',index,*fields()]);mutate('leave')
  if c['stop']=='leave':raise Stop('leave')
  mm.ret(0)
 def enter(index,mm):
  events.append(['enter',index,mm.arg(1),mm.arg(2),*fields()]);mutate('enter')
  if c['stop']=='enter':raise Stop('enter')
  mm.ret(0,8)
 for i in range(1,9):
  if i in c['null_nodes']:continue
  node=m.alloc(4);vt=m.alloc(0x1c);m.w(node,vt)
  m.w(vt+0x18,m.stub(lambda mm,index=i:leave(index,mm)))
  m.w(vt+0x14,m.stub(lambda mm,index=i:enter(index,mm)))
  m.w(app+0xcc+4*i,node)
 m.observe(0x40ce53,lambda mm:events.append(['node',m.r(app+0x10c)]))
 m.observe(0x40ce77,lambda mm:events.append(['node',m.r(app+0x10c)]))
 status='ok'
 try:m.call(0x40ce10,app,[c['requested'],c['arg8'],c['arg_c']])
 except Stop as e:status='required_'+str(e)
 except AssertionError:
  if not m.null_accesses:raise
  address=m.null_accesses[-1][0]
  assert address in [0x40ce5a,0x40ce84],m.null_accesses
  status='null_leave'if address==0x40ce5a else'null_enter'
 return dict(current=m.r(app+0x10c),previous=m.r(app+0x110),events=events,status=status),m.visited

def cases():
 for current,previous,requested,args in itertools.product([0,2,6],[0,7],[0,2,6],[(0,0),(0x11223344,0xdeadbeef)]):
  yield dict(current=current,previous=previous,requested=requested,arg8=args[0],arg_c=args[1],mutate={},stop=None,null_nodes=[])
 base=dict(current=2,previous=7,requested=6,arg8=0xaabbccdd,arg_c=0x7f001234,mutate={},stop=None,null_nodes=[])
 for mutate in [{'diagnostic':[3,9]},{'diagnostic':[0,9]},{'diagnostic':[6,9]},{'leave':[4,11]},{'enter':[5,12]},
                {'diagnostic':[3,9],'leave':[4,11],'enter':[5,12]}]:
  for stop in [None,'diagnostic','leave','enter']:yield dict(base,mutate=mutate,stop=stop)
 for c in [dict(base,null_nodes=[2]),dict(base,null_nodes=[6]),dict(base,current=6,null_nodes=[6]),
           dict(base,current=0,requested=0,null_nodes=[2,6]),dict(base,current=0xffffffff,requested=0),
           dict(base,current=6,requested=6,mutate={'enter':[4,9]},stop='enter')]:yield c

def main():
 rows=[];ctors=[];pins={}
 for pc in n.PRECISIONS:
  for poison in [0,0xa5,0xff]:
   a,pa=constructor('original',pc,poison);b,pb=constructor('v14',pc,poison);assert a==b==dict(current=0,previous=0)
   ctors.append(dict(precision=pc,poison=poison,output=a));pins.update(pa);pins.update(pb)
  for c in cases():
   a,pa=run(c,'original',pc);b,pb=run(c,'v14',pc);assert a==b,(c,a,b)
   rows.append(dict(precision=pc,input=c,output=a));pins.update(pa);pins.update(pb)
 for va,raw in pins.items():assert n.original.range(va,len(raw))==n.modified.range(va,len(raw))==raw
 #Instruction producer is pinned separately; fragment precondition is not
 #misrepresented as completion of the constructor's managers/thread children.
 from capstone import Cs,CS_ARCH_X86,CS_MODE_32
 code=list(Cs(CS_ARCH_X86,CS_MODE_32).disasm(n.original.range(0x4177e0,0x120),0x4177e0))
 zeros=[i for i in code if i.mnemonic=='xor'and i.op_str=='ebx, ebx'];assert len(zeros)==1
 producer=[dict(va=hex(i.address),bytes=i.bytes.hex(),instruction=i.mnemonic+' '+i.op_str)for i in zeros]
 for i in zeros:assert n.modified.range(i.address,len(i.bytes))==i.bytes
 doc=dict(schema=1,scope=__doc__,original_sha256=n.sha(n.original.b),v14_sha256=n.sha(n.modified.b),rows=rows,
  constructor_rows=ctors,constructor_ebx_zero_producer=producer,native_executions=2*(len(rows)+len(ctors)),instructions=n.instructions(pins),
  faulting_instruction_pins=['0x40ce5a','0x40ce84'])
 p=HERE/'native-application-state.json'
 if '--write'in sys.argv:
  with p.open('x')as f:json.dump(doc,f,indent=2);f.write('\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps(dict(rows=len(rows),constructor_rows=len(ctors),native_executions=doc['native_executions'],pins=len(pins),sha256=n.sha(p.read_bytes()))))
if __name__=='__main__':main()
