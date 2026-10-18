"""Actual56EC90 front dispatch, stopped at its genuine zone/attached children.
No child successful return, allocation, solver, zone or readiness is supplied.
Structure branches stop before4D69A0; ordinary branches before56EB40; unknown
Actor138 continuation stops before its actual Data430 read. Real529C30 executes.
"""
from pathlib import Path
import sys,json,itertools,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn import UC_HOOK_MEM_READ
from unicorn.x86_const import *
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
class Machine(h.Machine):
 def __init__(self,pc,image,c):
  super().__init__(pc)
  if image is not h.P:
   old=(h.P.image_size+4095)&~4095;new=(image.image_size+4095)&~4095
   if new>old:self.u.mem_map(image.base+old,new-old)
   self.u.mem_write(image.base,image.b[:4096])
   for vs,r,raw,off in image.sections:
    if raw:self.u.mem_write(image.base+r,image.b[off:off+min(raw,max(vs,raw))])
  self.actor=self.alloc(0x180);self.d=self.alloc(0x18);self.data=self.alloc(0x600)
  self.w32(self.actor+4,self.data);self.w32(self.actor+8,c['flags']);self.w32(self.actor+0x100,0x200000 if c['structure'] else 0)
  self.w32(self.actor+0x138,c['word138']);self.w32(self.actor+0x148,0,0,9)
  self.w32(self.d,0xa3c568,self.actor,0xa3c540,0,0,c['radius']);self.w32(self.data+0x1d0,c['remote'])
  self.reads=[];self.stop=None
  self.names={self.actor+0x100:'structure',self.actor+0x138:'word138',self.actor+8:'flags',self.d+0x14:'radius',self.data+0x1d0:'remote_range'}
  self.u.hook_add(UC_HOOK_MEM_READ,self.read)
 def read(self,u,access,address,size,value,_):
  if address in self.names:self.reads.append(self.names[address])
 def _hook(self,u,pc,size,_):
  if pc in (0x56eb40,0x4d69a0,0x56ed4d):
   sp=u.reg_read(UC_X86_REG_ESP)
   if pc==0x56eb40:
    assert self.r32(sp+4)==6 and u.reg_read(UC_X86_REG_ECX)==self.d
    self.stop=dict(kind='zone',enabled=bool(self.r32(sp+8)&255),radius=self.r32(sp+12),entry=hex(pc))
   elif pc==0x4d69a0:
    assert self.r32(sp+4)==6 and u.reg_read(UC_X86_REG_ECX)==self.actor
    self.stop=dict(kind='zone',enabled=True,radius=self.r32(sp+12),entry=hex(pc))
   else:self.stop=dict(kind='required_attached',entry=hex(pc))
   u.emu_stop();return
  super()._hook(u,pc,size,_)
 def run(self):
  try:self.call(0x56ec90,self.d)
  except AssertionError:
   if self.stop is None:raise
  assert not self.seams,self.seams
  return dict(reads=self.reads,result=self.stop or dict(kind='unchanged'))
def main():
 rows=[]
 for structure,word138,flags,radius,pc in itertools.product([False,True],[0,1],[0,1],[0,0x80000000,0x3f800000,0xbf800000],h.PRECISION):
  c=dict(structure=structure,word138=word138,flags=flags,radius=radius,remote=radius)
  values=[Machine(pc,image,c).run()for image in(h.P,h.V)];assert values[0]==values[1]
  rows.append(dict(precision=pc,input=c,native=values[0]))
 pins={hex(a):b for a,b in sorted(h.VISITED.items())}
 for a,b in pins.items():assert h.P.range(int(a,16),len(bytes.fromhex(b)))==h.V.range(int(a,16),len(bytes.fromhex(b)))==bytes.fromhex(b)
 doc=dict(schema=1,scope=__doc__,producer_sha256=H(Path(__file__)),rows=rows,executed_pins=pins)
 path=HERE/'native-after-fields.json';data=(json.dumps(doc,indent=1)+'\n').encode()
 if '--write'in sys.argv:path.write_bytes(data)
 else:assert path.read_bytes()==data
 print(json.dumps(dict(paired_rows=len(rows),executions=2*len(rows),executed_pins=len(pins),sha256=hashlib.sha256(data).hexdigest())))
if __name__=='__main__':main()
