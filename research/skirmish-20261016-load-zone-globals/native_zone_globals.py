"""Actual53F840/5402A0 constructors and50C4C0 presentation gate.
Real move-solver heap constructor/8092 element constructors execute. The only
returning seam is successful allocation; positive43BA80 presentation stops at
its actual entry.50CE00 header proof is reused explicitly, including its named
unused-capacity lifecycle seam; no full presentation manager is claimed.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn import UC_HOOK_MEM_READ
from unicorn.x86_const import UC_X86_REG_EIP
H=lambda b:hashlib.sha256(b).hexdigest()
class Machine(h.Machine):
 def __init__(self,pc,image):
  super().__init__(pc)
  if image is not h.P:
   old=(h.P.image_size+0xfff)&~0xfff;new=(image.image_size+0xfff)&~0xfff
   if new>old:self.u.mem_map(image.base+old,new-old)
   self.u.mem_write(image.base,image.b[:0x1000])
   for vs,r,raw,off in image.sections:
    if raw:self.u.mem_write(image.base+r,image.b[off:off+min(raw,max(vs,raw))])
  self.allocations=[];self.seams[0x6f7810]=lambda m:m.allocation()
 def allocation(self):
  n=self.arg();assert n==0x1f9c*0x14,n;self.allocations.append(n);p=self.alloc(n);self.u.mem_write(p,b'\xa5'*n);self.ret(value=p)
def constructors(pc,image,fill):
 m=Machine(pc,image);world=m.alloc(0x300);map_=m.alloc(0x60);release=m.alloc(0x38);move=m.alloc(0x58)
 m.u.mem_write(release,bytes([fill])*0x38);m.u.mem_write(move,bytes([fill])*0x58);m.w32(0xb36534,world);m.w32(world+0x30,map_)
 assert m.call(0x53f840,release)==release;assert m.call(0x5402a0,move,count=2000000)==move
 assert m.r32(release)==0xa36048 and m.r32(release+4)==map_ and m.r32(release+8)==release+0x34 and m.r32(release+0x34)==0xa3602c
 assert m.r32(move)==0xa36954 and m.r32(move+8)==move+0x44 and m.r32(move+0x44)==0xa3668c
 assert m.r32(move+0x4c)==0 and m.r32(move+0x50)==0x1f9c and m.r32(move+0x54)==0x1f9c
 p=m.r32(move+0x48);assert p
 for i in range(0x1f9c):assert m.r32(p+i*20,2)==[0,0]
 return {'release_class':hex(m.r32(release)),'release_map_same':True,'release_neighbor_offset':m.r32(release+8)-release,'release_neighbor_class':hex(m.r32(release+0x34)),
  'move_class':hex(m.r32(move)),'move_heap_offset':m.r32(move+8)-move,'move_heap_class':hex(m.r32(move+0x44)),'heap_count':m.r32(move+0x4c),'native_capacity':m.r32(move+0x50),'native_limit':m.r32(move+0x54),'all_unused_elements_constructed':True,'allocation_sizes':m.allocations}
class Present(Machine):
 def __init__(self,pc,image,phase,view):
  super().__init__(pc,image);self.world=self.alloc(0x300);self.manager=self.alloc(32);self.view=self.alloc(4)
  self.w32(0xb36534,self.world);self.w32(self.world+0xd8,phase);self.w32(self.manager+4,self.view if view else 0)
  self.reads=[];self.stopped=None;self.u.hook_add(UC_HOOK_MEM_READ,self.read,begin=self.manager+4,end=self.manager+7)
 def read(self,u,access,address,size,value,_):self.reads.append([address-self.manager,size])
 def _hook(self,u,pc,size,_):
  if pc==0x43ba80:
   h.VISITED[pc]=bytes(u.mem_read(pc,size)).hex();assert h.P.range(pc,size)==h.V.range(pc,size)
   assert self.ecx()==self.view;self.stopped=[self.arg(i) for i in range(5)]
   # Native constructs flags as a WORD in a pushed register slot; upper16
   # are not the child's u16 parameter and retain unrelated register bits.
   self.stopped[-1]&=0xffff
   u.emu_stop();return
  super()._hook(u,pc,size,_)
 def run(self):
  args=[7,0x42c80000,0x42cc0000,0x41800000,0x40]
  try:self.call(0x50c4c0,self.manager,args)
  except AssertionError:
   if self.stopped is None:raise
  assert self.u.reg_read(UC_X86_REG_EIP)==h.STOP or self.stopped==args
  return {'outcome':'required43BA80' if self.stopped else 'complete','view_reads':self.reads,'presentation_child_args':self.stopped}
def main():
 ctors=[];present=[]
 for pc,fill in itertools.product(h.PRECISION,[0,0xa5,0xff]):
  x=[constructors(pc,image,fill) for image in (h.P,h.V)];assert x[0]==x[1];ctors.append({'precision':pc,'poison':fill,'native':x[0]})
 for pc,phase,view in itertools.product(h.PRECISION,[0,1,6,0xffffffff],[False,True]):
  x=[Present(pc,image,phase,view).run() for image in (h.P,h.V)];assert x[0]==x[1];present.append({'precision':pc,'phase':phase,'view':view,'native':x[0]})
 ranges=[(0x4f2854,0x1d),(0x4f2ae5,0x17),(0x4f2b14,0x1a)]
 pins=[]
 for va,n in ranges:assert h.P.range(va,n)==h.V.range(va,n);pins.append({'va':hex(va),'bytes':h.P.range(va,n).hex()})
 doc={'schema':1,'scope':__doc__,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,'constructors':ctors,'presentation_rows':present,'world_init_callsite_pins':pins,
  'reused_fx_header_proof_sha256':H((ROOT/'research/development10/fx-manager/native-fx-manager.json').read_bytes()),'native_machines':2*(len(ctors)+len(present)),
  'address_byte_pins':[{'va':hex(a),'bytes':b,'kind':'allocator_entry' if a==0x6f7810 else 'required_entry' if a==0x43ba80 else 'executed'} for a,b in sorted(h.VISITED.items())]}
 p=HERE/'native-zone-globals.json'
 if '--write' in sys.argv:p.write_text(json.dumps(doc,indent=1)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'constructors':len(ctors),'presentations':len(present),'native_machines':doc['native_machines'],'all_pins':len(doc['address_byte_pins']),'executed_pins':sum(p['kind']=='executed' for p in doc['address_byte_pins']),'fixture_sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
