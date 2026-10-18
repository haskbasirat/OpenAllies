"""Unchanged MainMenu/Loading constructors and callbacks, both images/allPC.
Actual enter runs to5C0440 entry (not an executed show backend). Primitive
selectors include genuine null fixture pointers, never an application-absence claim.
"""
from pathlib import Path
import sys,json,itertools,hashlib,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn import UC_HOOK_MEM_READ,UC_HOOK_MEM_WRITE
H=lambda b:hashlib.sha256(b).hexdigest()
ORIGINAL=h.P;V14=h.V
class Machine(h.Machine):
 def run(self,inp):
  obj=self.alloc(0x40);self.u.mem_write(obj,b'\xa5'*0x40)
  op=inp['op']
  if op in ['ctor_main','ctor_loading','leave_main','leave_loading','finish_main','finish_loading']:
   self.w32(0xb39748,inp.get('splash',0))
   if op.startswith('ctor_'):self.call(0x6e7ff0 if op=='ctor_main'else 0x682de0,obj)
   else:
    self.w8(obj+8,inp.get('changed',165));self.w8(obj+0x15,inp.get('flag15',165))
    self.call({'leave_main':0x4b2830,'leave_loading':0x40caa0,'finish_main':0x943800,'finish_loading':0x682db0}[op],obj,[inp.get('argument',0)] if op.startswith('finish_')else[])
   return dict(bytes=list(self.u.mem_read(obj,0x18)))
  self.w32(obj+4,inp.get('current',0));self.w32(obj+0x10,inp.get('kind',0))
  self.w32(0xb39748,inp.get('splash',0));array=self.alloc(0x30);self.w32(array+8,inp.get('table2',0));self.w32(0xb39758,array if inp.get('array',1) else 0)
  trace=[];show=[]
  def read(u,access,a,n,v,_):
   if a==obj+4:trace.append('menu')
   elif a==obj+0x10:trace.append('kind')
   elif a==0xb39748:trace.append('splash')
   elif a==0xb39758:trace.append('menu2')
  def write(u,access,a,n,v,_):
   if a==obj+4:trace.append(['store',v])
  self.u.hook_add(UC_HOOK_MEM_READ,read);self.u.hook_add(UC_HOOK_MEM_WRITE,write)
  def stop(m):show.extend([m.ecx(),m.arg(0)]);trace.append(['show',m.ecx(),m.arg(0)]);m.u.emu_stop()
  self.seams[0x5c0440]=stop
  op=inp['op']
  try:
   if op=='main':self.call(0x40ca80,obj,[inp['explicit'],0x76543210])
   elif op=='loading':self.call(0x682e70,obj,[inp.get('explicit',3),0x76543210])
   else:raise ValueError(op)
  except AssertionError:
   assert self.u.reg_read(h.UC_X86_REG_EIP)==0x5c0440 and show
  return dict(current=self.r32(obj+4),trace=trace,show=show)
def main():
 h.VISITED.clear();rows=[]
 inputs=[dict(op='main',current=c,explicit=e)for c,e in itertools.product([0,1,2],[0,1,3])]
 inputs += [dict(op='loading',kind=k,current=c,splash=s,table2=t,array=a,explicit=99)for k,c,s,t,a in itertools.product([0,1,2,0xffffffff],[0,3],[0,1],[0,2],[0,1])]
 inputs += [dict(op='ctor_main',splash=s)for s in [0,1]]+[dict(op='ctor_loading')]
 inputs += [dict(op='leave_main')]+[dict(op='leave_loading',changed=c)for c in [0,1,128,255]]
 inputs += [dict(op='finish_main',argument=a)for a in [0,1,128,255]]+[dict(op='finish_loading',argument=a,flag15=f)for a,f in itertools.product([0,1,128,255],[0,1,128])]
 for pc,inp in itertools.product(h.PRECISION,inputs):
  h.P=ORIGINAL;a=Machine(pc).run(inp);h.P=V14;b=Machine(pc).run(inp);assert a==b
  rows.append(dict(pc=pc,input=inp,output=a))
 h.P=ORIGINAL
 pins={hex(a):v for a,v in sorted(h.VISITED.items())if a!=0x5c0440}
 for a,v in pins.items():assert ORIGINAL.range(int(a,16),len(bytes.fromhex(v)))==V14.range(int(a,16),len(bytes.fromhex(v)))
 tables={}
 for vt in [0xa7edd0,0xa7f0d0,0xa64d98]:
  target=ORIGINAL.u32(vt+0xe8);assert target==0x4b2830 and V14.u32(vt+0xe8)==target and ORIGINAL.range(target,1)==V14.range(target,1)==b'\xc3';tables[hex(vt+0xe8)]=hex(target)
 data=(json.dumps(dict(schema=1,scope=__doc__,producer_sha256=H(Path(__file__).read_bytes()),rows=rows,pins=pins,stopped_entries=['0x5c0440'],interface_e8_tables=tables),indent=1)+'\n').encode()
 p=HERE/'native-menu-callbacks.json'
 if '--write'in sys.argv:p.write_bytes(data)
 else:assert p.read_bytes()==data
 print(json.dumps(dict(rows=len(rows),native_runs=2*len(rows),executed_pins=len(pins),sha256=H(data))))
if __name__=='__main__':main()
