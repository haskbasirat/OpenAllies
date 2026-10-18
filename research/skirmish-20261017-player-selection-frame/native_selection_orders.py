"""Actual accepted5F4450/5F4490/5F44C0/5F44F0 Process order and76A530
B0 reader over actual runtime class vtables. Selection child entries are named
recorded required boundaries/transport; existing selection kernel proof remains
separate. No UI picking, input producer, selected command or battle claim.
"""
from pathlib import Path
import sys,json,itertools
ROOT=Path(__file__).resolve().parents[2];HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'))
import native_machine as h
from unicorn.x86_const import UC_X86_REG_ECX
P,V=h.P,h.V
TABLES={1:0xa2a680,2:0xa2a2a0,3:0xa273f0,4:0xa299d8,5:0xa29780,6:0xa28a00,7:0xa28b88,9:0xa28e90}
ENTRY=dict(Set=0x5f4450,Add=0x5f4490,Remove=0x5f44c0,Clear=0x5f44f0)

def process(c,pc):
 m=h.Machine(b'',pc);order=m.alloc(8);admin=m.alloc(0x288);a=m.alloc(0x54);b=m.alloc(0x54);va=m.alloc(4);vb=m.alloc(4);targets=[0,m.alloc(0x400),m.alloc(0x400)]
 m.w(order+4,targets[c['target']]);m.w(0xb36558,admin);m.w(admin+0x284,a);m.w(a+0x50,va);m.w(b+0x50,vb);events=[]
 def read(mm):events.append(['current',1 if mm.r(admin+0x284)==a else 2])
 m.probes[0x74b3b0]=read
 def child(kind,pop):
  def call(mm):
   view=1 if mm.u.reg_read(UC_X86_REG_ECX)==va else 2
   event=[kind,view]+([]if kind=='clear'else[targets.index(mm.arg())]);events.append(event)
   if c.get('stop')==kind:m.outcome=dict(kind='required',child=kind);raise h.Throw()
   if kind=='clear':
    if 'after_target'in c:mm.w(order+4,targets[c['after_target']])
    if c.get('after_player'):mm.w(admin+0x284,b)
   mm.ret(pop)
  return call
 for addr,kind,pop in [(0x4ed0d0,'clear',0),(0x4ecff0,'add',4),(0x4ed080,'remove',4)]:m.seam(addr,child(kind,pop))
 m.call(ENTRY[c['operation']],order,[])
 assert not m.null_accesses
 return dict(status=m.outcome['child']if m.outcome else'complete',events=events,target=targets.index(m.r(order+4)),player=1 if m.r(admin+0x284)==a else 2)

def reader(c,pc):
 m=h.Machine(b'',pc);dest=m.alloc(4,0xa5);registry=m.alloc(0x60008);obj=m.alloc(0x400,0xa5)
 m.w(obj,TABLES[c['class']]);m.w(registry+0x20004+4*c['slot'],obj if c['present'] else 0);m.w(0xb364c0,registry if c['registry'] else 0)
 def read_bits(mm):assert mm.arg(2)==16;mm.w(mm.arg(1),c['slot']);mm.ret()
 m.seam(0x40d5d0,read_bits)
 #76A530 is cdecl: caller argument words remain above return and callee RET0.
 m.w(h.STACK+4,42);m.w(h.STACK+8,dest);m.call(0x76a530,0,[])
 assert not m.null_accesses
 return dict(found=m.r(dest)==obj)

def cases():
 out=[]
 for op in ['Add','Remove']:
  out.extend([dict(operation=op,target=0),dict(operation=op,target=1),dict(operation=op,target=1,stop=op.lower())])
 out.extend([dict(operation='Clear',target=1),dict(operation='Clear',target=1,stop='clear'),dict(operation='Set',target=0),dict(operation='Set',target=1),dict(operation='Set',target=1,stop='clear'),dict(operation='Set',target=1,stop='add'),dict(operation='Set',target=1,after_target=2),dict(operation='Set',target=1,after_target=0),dict(operation='Set',target=1,after_player=True)])
 return out

def main():
 h.VISITED.clear();rows=[];readers=[]
 for pc,c in itertools.product(h.CW,cases()):
  pair=[]
  for pe in[P,V]:h.P=pe;pair.append(process(c,pc))
  assert pair[0]==pair[1];rows.append(dict(precision=pc,input=c,native=pair[0]))
 for pc,cls,(reg,present)in itertools.product(h.CW,TABLES,[(True,True),(True,False),(False,True)]):
  c=dict(class_=cls,slot=37,registry=reg,present=present);c['class']=c.pop('class_');pair=[]
  for pe in[P,V]:h.P=pe;pair.append(reader(c,pc))
  assert pair[0]==pair[1];assert pair[0]['found']==(reg and present and cls not in[1,5]);readers.append(dict(precision=pc,input=c,native=pair[0]))
 h.P=P
 seams={0x4ed0d0,0x4ecff0,0x4ed080,0x40d5d0};pins={hex(a):b for a,b in sorted(h.VISITED.items())if a not in seams}
 tables=[]
 for cls,t in TABLES.items():
  raw=P.range(t+0xb0,4);assert raw==V.range(t+0xb0,4);tables.append(dict(class_=cls,vtable=hex(t),slot=hex(t+0xb0),bytes=raw.hex(),target=hex(int.from_bytes(raw,'little'))))
 out=dict(schema=1,scope=__doc__,rows=rows,readers=readers,executions=2*(len(rows)+len(readers)),pins=pins,vtables=tables,seams=['40D5D0 reads explicit16-bit slot; bitstream transport excluded','4ED0D0/4ECFF0/4ED080 selection children record entered arguments and may stop before body, not completed children'],producer_sha256=h.H(Path(__file__).read_bytes()),machine_sha256=h.H(Path(h.__file__).read_bytes()))
 path=HERE/'native-selection-orders.json'
 if '--write'in sys.argv:path.write_text(json.dumps(out,indent=2)+'\n')
 else:assert out==json.loads(path.read_text())
 print(json.dumps(dict(process_rows=len(rows),reader_rows=len(readers),executions=out['executions'],pins=len(pins),sha256=h.H(path.read_bytes()))))
if __name__=='__main__':main()

