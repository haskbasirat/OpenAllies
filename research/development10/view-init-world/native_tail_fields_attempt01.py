"""Native Terrain130 sizing,42D0D0/F0 global alias and86C490 field constructors.
Allocator/free are explicit Rust-backend transports. These bounded leaves do
not prove a full terrain renderer, VWorld or AD/Loading completion.
"""
from pathlib import Path
import sys,json,itertools,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'));import native_machine as h
from unicorn.x86_const import *
ORIGINAL,V14=h.P,h.V
def slots(pc,count):
 m=h.Machine(b'',pc);view=m.alloc(0x300);data=m.alloc(0x700);old=m.alloc(12,0xa5)
 m.w(0xb36530,data);m.w(data+0x634,count)
 m.w(view+0x130,old);m.w(view+0x134,0);m.w(view+0x138,3)
 def done(m):m.outcome={'kind':'bounded-slots'};raise h.Throw()
 m.probes[0x43fcf1]=done;m.u.reg_write(UC_X86_REG_ESI,view);m.call(0x43fca4,0,[])
 assert not m.null_accesses
 p=m.r(view+0x130);capacity=m.r(view+0x138)
 return dict(count=m.r(view+0x134),capacity=capacity,words=[m.r(p+4*i)for i in range(capacity)])
def helper(pc,sequence):
 m=h.Machine(b'',pc);owners={};names={0:None};rows=[]
 for operation in sequence:
  kind,name=operation.split(':')
  if kind=='new':
   p=m.alloc(4,0xa5);owners[name]=p;names[p]=name;m.call(0x42d0d0,p,[])
  else:m.call(0x42d0f0,owners[name],[])
  rows.append(dict(operation=operation,root=names[m.r(0xb39d8c)],alias=m.r(0xb39d90)==0xb39d8c,current=names[m.r(0xb39d94)],vtable=hex(m.r(owners[name]))))
 assert not m.null_accesses
 return rows
def auxiliary(pc,fill):
 m=h.Machine(b'',pc);p=m.alloc(0x38,fill);assert m.call(0x86c490,p,[])==p
 assert not m.null_accesses
 return list(m.u.mem_read(p,0x38))
def main():
 h.VISITED.clear();rows=[]
 scenarios=[('slots',n)for n in[0,1,2,3,4,5,6,7,20]]
 scenarios += [('helper',s)for s in[['new:A'],['new:A','drop:A'],['new:A','new:B','drop:A','drop:B'],['new:A','new:B','drop:B','drop:A']]]
 scenarios += [('auxiliary',fill)for fill in[0,0xa5,0xff]]
 for pc,(kind,value)in itertools.product(h.CW,scenarios):
  output=[]
  for image in[ORIGINAL,V14]:
   h.P=image;output.append(globals()[kind](pc,value))
  assert output[0]==output[1];rows.append(dict(precision=pc,kind=kind,input=value,output=output[0]))
 h.P=ORIGINAL
 for a in[0xb39d8c,0xb39d90,0xb39d94]:assert ORIGINAL.range(a,4)==V14.range(a,4)==b'\0'*4
 excluded={0x6f7810,0x6f7430,0x43fcf1}
 pins={hex(a):v for a,v in sorted(h.VISITED.items())if a not in excluded}
 doc=dict(schema=1,scope=__doc__,producer_sha256=h.H(Path(__file__).read_bytes()),rows=rows,pins=pins,
  fresh_pe_words=[dict(address=hex(a),raw='00000000')for a in[0xb39d8c,0xb39d90,0xb39d94]],
  domain='valid typed nonnegative source registry count, successful allocation; Rust poisoned-lock/panic and native allocator failure behavior excluded')
 data=(json.dumps(doc,indent=1)+'\n').encode();p=HERE/'native-tail-fields.json'
 if '--write'in sys.argv:p.write_bytes(data)
 else:assert p.read_bytes()==data
 print(json.dumps(dict(rows=len(rows),executions=2*len(rows),pins=len(pins),sha256=h.H(data))))
if __name__=='__main__':main()
