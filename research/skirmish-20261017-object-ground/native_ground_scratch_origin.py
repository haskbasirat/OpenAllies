"""Native4D7230 final FX wrapper ->4F9C10 PostCreate ->4EB500 residue.

Actual prologue/tail,42A590/429080(empty or paused),4CF300/4EBA40/4D1E10,
and4EB500 descriptor stores execute. InitExisting's already-completed middle,
base admission, multi predicate and height sampler are explicit transports.
551140 is a capture cutoff, not successful brush execution. Initial scratch
poison varies; only the FX-gate path can mint the observed Actor-Z equality.
"""
from pathlib import Path
import sys,json,struct,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/development10/creator-labels'))
from native_creator_labels import Machine as Base,PE0,PE1,PC
from unicorn import UC_HOOK_MEM_WRITE
from unicorn.x86_const import *

class Machine(Base):
 def __init__(self,image):
  self.ready=False;super().__init__(image);self.actual={};self.record_actual=True;self.ready=True
 def hook(self,u,pc,size,user):
  if self.ready:
   if pc==0x4ebe50:self.ret(4,1);return
   if pc==0x4d1fd0:self.ret(value=1);return
   if pc==0x4d7265:u.reg_write(UC_X86_REG_EIP,0x4d7532);return
   if pc==0x4ee700:self.ret(value=1);return
   if pc==0x4f0e50:u.reg_write(UC_X86_REG_EIP,self.height_stub);return
   if pc==0x551140:
    p=self.arg(4);self.captured=dict(descriptor=[self.r(p+4*i)for i in range(9)],source_matches=p+0x1c==self.scratch)
    u.emu_stop();return
   if pc==0x4f9f99:u.emu_stop();return
  super().hook(u,pc,size,user)

def run(m,case):
 actor=m.alloc(0x180);data=m.alloc(0x200);params=m.alloc(0x54);world=m.alloc(0x300);resource=m.alloc(0xac);svars=m.alloc(0x20);slope=m.alloc(4);pause=m.alloc(4)
 m.w(actor,0xa273f0);m.w(actor+4,data);m.w(actor+8,2760074496);m.w(actor+0x100,(1<<21)if case['suppress_fx']else 0)
 for i,word in enumerate([1109393408,1129840640,case['z']]):m.w(actor+0x20+4*i,word)
 m.w(actor+0x2c,0);m.w(actor+0x30,0xbf800000)
 m.w(data+0xe4,1 if case['list_present']else 0)
 m.w(params,0xa25b28);m.w(params+0x24,1688);m.w(params+0x3c,1689 if case['different_id']else 1688);m.w(params+0x38,5)
 m.w(0xb36534,world);m.w(world+0x28,resource);m.w(resource+0xa8,0);m.w(world+0x34,m.alloc(0x60));m.w(world+0x130,pause);m.w(pause,1 if case['paused']else 0)
 m.w(0xb39ab0,svars);m.w(svars+0x18,slope);m.w(slope,0x3f800000)
 height=m.alloc(4);m.w(height,0x40d00000);m.height_stub=m.alloc(16);m.u.mem_write(m.height_stub,b'\xd9\x05'+struct.pack('<I',height)+b'\xc2\x08\x00')
 common=m.stack-0x400;m.scratch=common-68;m.w(m.scratch,case['poison']);m.captured=None;writes=[]
 def changed(u,access,address,size,value,user):
  if address<=m.scratch<address+size or m.scratch<=address<m.scratch+4:writes.append(dict(pc=hex(u.reg_read(UC_X86_REG_EIP)),bytes=size,value=value&0xffffffff))
 hook=m.u.hook_add(UC_HOOK_MEM_WRITE,changed)
 m.u.reg_write(UC_X86_REG_ESP,common);m.u.reg_write(UC_X86_REG_ESI,actor);m.u.reg_write(UC_X86_REG_EDI,params);m.u.reg_write(UC_X86_REG_EBP,world)
 try:m.u.emu_start(0x4f9f7d,0x4f9f99,count=100000)
 except Exception as error:raise RuntimeError((case,hex(m.u.reg_read(UC_X86_REG_EIP)),[hex(a)for a in list(m.actual)[-12:]]))from error
 finally:m.u.hook_del(hook)
 assert m.captured is not None,'actual deformation descriptor not reached'
 return dict(capture=m.captured,writes=writes,scratch=m.r(m.scratch))

def main():
 rows=[];pins={}
 for image in ['original','v14']:
  m=Machine(image)
  for pc in PC:
   m.fpu(pc)
   for z,poison,suppress,different in itertools.product([0,0x80000000,0x3f800000,1086590007,0x7fc01234],[0xdeadbeef,0x12345678],[False,True],[False,True]):
    case=dict(z=z,poison=poison,suppress_fx=suppress,different_id=different,list_present=False,paused=False)
    out=run(m,case)
    if not suppress and not different:assert out['scratch']==z and out['writes'][-1]['pc']=='0x42a5d3'and out['capture']['source_matches']
    rows.append(dict(image=image,precision=pc,input=case,output=out))
   for z in [0x80000000,1086590007]:
    case=dict(z=z,poison=0xdeadbeef,suppress_fx=False,different_id=False,list_present=True,paused=True);out=run(m,case);assert out['scratch']==z
    rows.append(dict(image=image,precision=pc,input=case,output=out))
  pins.update(m.actual)
 a=[{k:v for k,v in row.items()if k!='image'}for row in rows if row['image']=='original'];b=[{k:v for k,v in row.items()if k!='image'}for row in rows if row['image']=='v14'];assert a==b
 instructions=[]
 for at,size in sorted(pins.items()):
  if PE0.base<=at<PE0.base+PE0.image_size:
   raw=PE0.range(at,size);assert raw==PE1.range(at,size);instructions.append(dict(va=hex(at),bytes=raw.hex()))
 data=[]
 for address in [0xa273fc,0xa274d8,0xa274dc,0xa25b28]:
  raw=PE0.range(address,4);assert raw==PE1.range(address,4);data.append(dict(va=hex(address),bytes=raw.hex()))
 doc=dict(scope=__doc__,rows=rows,instructions=instructions,dispatch=data,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),images=dict(original=hashlib.sha256(PE0.b).hexdigest(),v14=hashlib.sha256(PE1.b).hexdigest()))
 path=HERE/'native-ground-scratch-origin.json'
 if '--create'in sys.argv:
  with path.open('x')as out:json.dump(doc,out,indent=2);out.write('\n')
 else:assert json.loads(path.read_bytes())==doc
 print(json.dumps(dict(rows=len(rows),pins=len(instructions),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
