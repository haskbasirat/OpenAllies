"""Actual AA700E30/6FE920/6FDF80 bounded reset/projection.

Required modes/pose/visual-height children are explicit transports. FSIN/FPTAN
are transported from THIS CPU's isolated x87, never Unicorn binary64. Camera
prefills are laboratory inputs, not a completed7016A0 constructor. Required
dependency cutoffs precede the corresponding native instruction/service.
"""
from pathlib import Path
import sys,json,hashlib,itertools,struct,ctypes
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'))
from native_machine import Machine as Base,original,modified,PRECISIONS,f32
sys.path.insert(0,str(ROOT/'research/skirmish-20261016-settlement-completion'))
from hardware_trig import HardwareTrig
from unicorn import UC_HOOK_MEM_READ
from unicorn.x86_const import UC_X86_REG_EIP
SHA=lambda b:hashlib.sha256(b).hexdigest()
class Cutoff(Exception):pass
class Hardware(HardwareTrig):
 def __init__(self):
  super().__init__();code=(b'\x48\x0f\xae\x81\x00\x02\x00\x00'+b'\xdb\xe3'+b'\xd9\x29'+b'\xd9\x79\x02'+b'\xdb\x69\x10'+b'\xd9\xf2\xdd\xd8'+b'\xdb\x79\x20'+b'\xdd\x79\x30'+b'\x48\x0f\xae\x89\x00\x02\x00\x00'+b'\xc3')
  address=self.k.VirtualAlloc(None,4096,0x3000,4);assert address;ctypes.memmove(address,code,len(code));old=ctypes.c_ulong();assert self.k.VirtualProtect(address,4096,0x20,ctypes.byref(old));assert self.k.FlushInstructionCache(self.k.GetCurrentProcess(),address,len(code));self.code_pages.append(address);self.calls['fptan']=ctypes.CFUNCTYPE(None,ctypes.c_void_p)(address);self.instructions['fptan']={'bytes':code.hex(),'sha256':SHA(code)}
class Machine(Base):
 def __init__(self,image,pc,hardware):
  super().__init__(pc);self.hardware=hardware;self.trace=[];self.actual={};self.trans=[];self.fail=None
  if image=='v14':
   old=(original.image_size+4095)&~4095;new=(modified.image_size+4095)&~4095
   if new>old:self.u.mem_map(original.base+old,new-old)
   for vs,rva,raw,off in modified.sections:self.u.mem_write(modified.base+rva,modified.b[off:off+raw])
 def event(self,name,value=None):
  self.trace.append([name,value]);
  if self.fail==len(self.trace):raise Cutoff()
 def _hook(self,u,pc,size,data):
  if pc in(0x6fe958,0x70102a):
   name='fptan'if pc==0x6fe958 else'fsin';v=self.st0_live();raw=struct.pack('<QH',v['extended_significand'],v['extended_sign_exponent']);out=self.hardware.trig(name,PRECISIONS[self.precision],raw);assert out['status_word']&(1|4|8|0x400)==0
   self.trans.append(dict(pc=hex(pc),opcode=bytes(u.mem_read(pc,size)).hex(),op=name,input80=raw.hex(),output80=out['raw80']))
   p=self.alloc(16);u.mem_write(p,bytes.fromhex(out['raw80']));code=b'\xdd\xd8'+b'\xdb\x2d'+struct.pack('<I',p)+(b'\xd9\xe8'if name=='fptan'else b'')+b'\x68'+struct.pack('<I',pc+size)+b'\xc3';stub=self.alloc(len(code));u.mem_write(stub,code);u.reg_write(UC_X86_REG_EIP,stub);return
  super()._hook(u,pc,size,data)
  if pc not in self.seams and u.reg_read(UC_X86_REG_EIP)==pc and original.base<=pc<original.base+original.image_size:self.actual[pc]=bytes(u.mem_read(pc,size)).hex()

OFFSETS=[0x138,0x13c,0x140,0x144,0x148,0x14c,0x150,0x208,0x20c,0x210,0x214,0x218,0x22c,0x230,0x240,0x244,0x248,0x24c,0x250,0x254,0x2b8,0x2bc,0x2c0,0x2c4]
AVARS={0x1c:f32(1),0x20:f32(512),0x24:f32(.1),0x28:f32(1.5),0x2c:f32(1),0x30:f32(.5),0x34:f32(1.2),0x38:f32(1.2),0x3c:f32(.7),0x40:f32(2.1),0x44:f32(1),0x48:f32(-45),0x4c:0,0x50:f32(40),0x54:f32(10),0x58:f32(100),0x5c:f32(15)}
def run(image,pc,case,hw):
 m=Machine(image,pc,hw);m.fail=case.get('fail');camera=m.alloc(0x394);m.u.mem_write(camera,b'\xa5'*0x394)
 for off,bits in[(0x21c,f32(-1)),(0x220,f32(1)),(0x224,f32(1)),(0x228,f32(-1)),(0x230,f32(.75))]:m.w(camera+off,bits)
 m.wb(camera+0x33c,case['tracking']);m.w(camera+0x174,0);m.w(camera+0x218,case.get('fov',f32(15)))
 variables=dict(AVARS);variables.update({int(k):v for k,v in case.get('avars',{}).items()});variables[0x5c]=case.get('fov',f32(15));table=m.alloc(0x100);m.w(0xb39990,table);addresses={}
 for off,word in variables.items():a=m.alloc(4);m.w(a,word);m.w(table+off,a);addresses[a]=off
 dims=case['dimensions'];gmap=0
 if dims is not None:gmap=m.alloc(0x80);m.w(gmap+0x74,dims[0]);m.w(gmap+0x78,dims[1])
 m.w(0xb36518,gmap);uvars=m.alloc(0xb0);m.w(0xb39940,uvars)
 for off,v in zip((0xa8,0xac),case['fallback']):a=m.alloc(4);m.w(a,v);m.w(uvars+off,a)
 seeds=case['seeds']
 for a,v in zip((0xb43cf8,0xb43cfc,0xb43d00),seeds):m.w(a,v)
 m.modes=list(case['modes']);m.devs=list(case['devs']);m.pose=None
 def mode(x):v=m.modes.pop(0);x.event('mode',v);x.ret(int(v))
 def dev(x):v=m.devs.pop(0);x.event('developer',v);x.ret(int(v))
 m.seam(0x40cee0,mode);m.seam(0x6fe9c0,dev)
 def visual(x):xy=[x.r(x.arg(1)),x.r(x.arg(1)+4)];x.event('visual',[xy,x.arg(2)]);x.ret_st0_f32(case['height'],8)
 m.seam(0x43d050,visual)
 def pose(x):m.pose=[x.arg(i)for i in range(1,7)];x.event('pose',m.pose);x.ret(0,24)
 m.seam(0x6ff220,pose)
 def mem(u,access,address,size,value,data):
  if size==4 and address in addresses:m.event('avar',[addresses[address],m.r(address)])
  if size==4 and address in(0xb43cf8,0xb43cfc,0xb43d00):m.event('seed',[address,m.r(address)])
 m.u.hook_add(UC_HOOK_MEM_READ,mem)
 def dimensions(x):x.event('dimensions',dims)
 m.observe(0x700f76,dimensions);m.observe(0x700f87,lambda x:x.event('fallback',case['fallback']))
 # Actual873D90 executes with genuine laboratory count0; observation marks
 # its required integration boundary, not a fabricated full874B40 owner.
 m.observe(0x873d90,lambda x:x.event('dirty'))
 error=None
 try:m.call(0x6fe920 if case.get('op')=='projection' else 0x700e30,camera)
 except Cutoff:error='required dependency cutoff'
 output={hex(off):m.r(camera+off)for off in OFFSETS};return dict(fields=output,trace=m.trace,pose=m.pose,error=error),m.actual,m.trans
def cases():
 base=dict(tracking=0,dimensions=[256,128],fallback=[300,500],height=f32(17.5),modes=[False,False,False],devs=[False,False],seeds=[0,0,0])
 for mode in('gameplay','developer','metagame','changing'):
  for tracking,dimensions in[(0,None),(0,[255,129]),(1,None),(1,[512,256])]:
   c=dict(base,tracking=tracking,dimensions=dimensions)
   if mode=='developer':c.update(devs=[True,True])
   elif mode=='metagame':c.update(modes=[True,True,True],devs=[])
   elif mode=='changing':c.update(modes=[True,False,True],devs=[True])
   yield c
 for fov in[5,15,40,90,120,179]:yield dict(base,fov=f32(fov),seeds=[f32(2),f32(-3),f32(4)])
 for zoom in[-2,.5,1,9]:yield dict(base,avars={0x44:f32(zoom)})
 for near,far in[(.01,10),(.01,100000),(5,20),(-2,512)]:yield dict(base,avars={0x1c:f32(near),0x20:f32(far)})
 # Each reached input callback/read boundary is stopped before its effect.
 for i in range(1,24):yield dict(base,fail=i)
def main():
 hw=Hardware();rows=[];pins={};trans=[];inputs=list(cases())
 try:
  for image,pc in itertools.product(('original','v14'),PRECISIONS):
   for i,c in enumerate(inputs):
    out,visited,transport=run(image,pc,c,hw);pins.update(visited);trans.extend(transport);rows.append(dict(image=image,precision=pc,case=i,input=c,output=out))
  for a,b in pins.items():assert original.range(a,len(bytes.fromhex(b))).hex()==b==modified.range(a,len(bytes.fromhex(b))).hex(),hex(a)
  doc=dict(schema=1,scope=__doc__,producer_sha256=SHA(Path(__file__).read_bytes()),rows=rows,instructions=[dict(va=hex(a),bytes=b)for a,b in sorted(pins.items())],hardware_shims=hw.instructions,hardware_transports=trans)
  path=HERE/'native-camera-reset.json'
  if'--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
  else:assert json.loads(path.read_text())==doc
  print(json.dumps(dict(rows=len(rows),input_cases=len(inputs),executed_pins=len(pins),hardware_transports=len(trans),sha256=SHA(path.read_bytes()))))
 finally:hw.close()
if __name__=='__main__':main()
