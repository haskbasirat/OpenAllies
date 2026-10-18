"""Actual complete native reset→pose→NiCamera math, separate laboratory.

Runs real9E3520 process matrix constructor and actual6FF220/6FE3B0/6FE470/
6FE050/8742D0/culling/viewprojection children. Mode/dev/terrain queries remain
explicit service transports. Camera/NiNode prefills are inputs, NOT a full
7016A0 constructor claim. Watches the separate Camera+164 root allocation.
"""
from pathlib import Path
import json,sys,struct,itertools,hashlib
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE));import native_camera_reset as R
from unicorn import UC_HOOK_MEM_WRITE
from unicorn.x86_const import UC_X86_REG_EIP
class Machine(R.Machine):
 last=None
 def __init__(self,image,pc,hw):
  super().__init__(image,pc,hw);Machine.last=self;self.camera=None;self.root_writes=[];self.root=None;self.trans_pc=set()
  self.world=self.alloc(0x300);self.w(0xb36534,self.world);self.w(self.world+0x34,self.alloc(4));self.wb(self.world+0x275,0)
  session=self.alloc(0x110);self.w(0xb36550,session);self.w(session+0xe4,2)
  self.w(0xb4a998,1);self.call(0x9e3520);self.matrix=[self.r(0xb40054+4*i)for i in range(9)];self.actual={}
 def seam(self,pc,fn):
  if pc==0x6ff220:
   def pose_entry(m):
    m.camera=m.ecx();m.pose=[m.arg(i)for i in range(1,7)];m.event('pose',m.pose)
    c=m.camera;m.w(c,0xa84850);m.w(c+0x24,0);m.w(c+0xb0,0);m.w(c+0x68,0x3f800000)
    #Actual base ctor has these null/one fields; this oracle deliberately
    #prefills them without claiming execution of that unrelated constructor.
    uvar=m.alloc(0x300);m.u.mem_write(uvar,bytes(m.u.mem_read(m.r(0xb39940),0xb0)));m.w(0xb39940,uvar);v=m.alloc(4);m.wb(v,0);m.w(uvar+0xb4,v)
    m.root=m.alloc(0x100);m.u.mem_write(m.root,b'\xa5'*0x100);m.w(c+0x164,m.root)
    def watch(u,access,a,size,value,data):m.root_writes.append(dict(pc=hex(u.reg_read(UC_X86_REG_EIP)),offset=a-m.root,size=size,value=value))
    m.u.hook_add(UC_HOOK_MEM_WRITE,watch,begin=m.root,end=m.root+0xff)
   self.observe(pc,pose_entry)
   def terrain(m):m.event('map_height',[m.arg(1),m.arg(2)]);m.ret_st0_f32(R.f32(12.5),8)
   super().seam(0x550210,terrain)
  else:super().seam(pc,fn)
 def _hook(self,u,pc,size,data):
  if R.original.base<=pc<R.original.base+R.original.image_size:
   op=bytes(u.mem_read(pc,size))
   if op in(b'\xd9\xfe',b'\xd9\xff',b'\xd9\xf2'):
    name={b'\xd9\xfe':'fsin',b'\xd9\xff':'fcos',b'\xd9\xf2':'fptan'}[op];v=self.st0_live();raw=struct.pack('<QH',v['extended_significand'],v['extended_sign_exponent']);out=self.hardware.trig(name,R.PRECISIONS[self.precision],raw);assert out['status_word']&(1|4|8|0x400)==0
    self.trans.append(dict(pc=hex(pc),opcode=op.hex(),op=name,input80=raw.hex(),output80=out['raw80']));self.trans_pc.add(pc)
    p=self.alloc(16);u.mem_write(p,bytes.fromhex(out['raw80']));code=b'\xdd\xd8'+b'\xdb\x2d'+struct.pack('<I',p)+(b'\xd9\xe8'if name=='fptan'else b'')+b'\x68'+struct.pack('<I',pc+size)+b'\xc3';stub=self.alloc(len(code));u.mem_write(stub,code);u.reg_write(UC_X86_REG_EIP,stub);return
  super()._hook(u,pc,size,data)
def main():
 R.Machine=Machine;hw=R.Hardware();rows=[];pins={};trans=[];inputs=[]
 for case in list(R.cases())[:30]:
  c=dict(case);c['modes']=c['modes']+[c['modes'][-1]]*24;c['devs']=c['devs']+([c['devs'][-1]]if c['devs']else[False])*24;inputs.append(c)
 try:
  for image,pc in itertools.product(('original','v14'),R.PRECISIONS):
   for i,c in enumerate(inputs):
    output,visited,transport=R.run(image,pc,c,hw);m=Machine.last;assert not m.root_writes,m.root_writes
    assert m.camera is not None
    output['full_words']=[m.r(m.camera+4*j)for j in range(0x394//4)];output['matrix_b40054']=m.matrix;output['root_164_writes']=m.root_writes
    rows.append(dict(image=image,precision=pc,case=i,input=c,output=output));pins.update(visited);trans.extend(transport)
  for a,b in pins.items():assert R.original.range(a,len(bytes.fromhex(b))).hex()==R.modified.range(a,len(bytes.fromhex(b))).hex()==b,hex(a)
  doc=dict(schema=1,scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,instructions={hex(a):b for a,b in sorted(pins.items())},hardware_transports=trans)
  path=HERE/'native-camera-pose.json'
  if'--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
  else:assert json.loads(path.read_text())==doc
  print(json.dumps(dict(rows=len(rows),pins=len(pins),hardware_transports=len(trans),camera_root_writes=0,sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
 finally:hw.close()
if __name__=='__main__':main()
