"""Original565FA0 Construction ctor and566BC0 Load with full565F20/5657B0/45F4D0/5655C0 empty-predecessor cost path. No-modifier raw cost copies, vector dimensions and noneditor reset execute. Saved arithmetic NaN inputs are excluded pending hardware-backed proof (Unicorn does not model signaling-NaN quieting reliably); raw memcpy definition costs preserve arbitrary bits. Allocation/I-O/string storage and editor stream-mode are explicit seams."""
from pathlib import Path
import sys,json,struct,hashlib,itertools
ROOT=Path(__file__).resolve().parents[2];sys.path.insert(0,str(ROOT/'research/phase6-kingdoms'))
from native_kingdoms import Machine as Base,P,V,VISITED
from unicorn.x86_const import *
W=lambda *v:struct.pack('<'+'I'*len(v),*v)
class Machine(Base):
 def __init__(self,payload,costs,editor):
  super().__init__(payload,28);self.component=self.alloc(0x40);self.editor=editor;self.definition=self.read(self.object+4)
  self.write(self.definition+0x174,512|8192 if costs is not None else 8192);self.write(self.definition+0x4d0,0)
  values=[] if costs is None else costs;self.write(self.definition+0x324,self.alloc(len(values)*4));self.write(self.definition+0x328,len(values))
  for i,v in enumerate(values):self.write(self.read(self.definition+0x324)+i*4,v)
  data=self.alloc(0x400);self.write(0xb36530,data);self.write(data+0x38,len(values));context=self.alloc(0x100);self.write(context+0xa8,1);self.write(self.world+0x28,context)
 def hook(self,u,pc,size,arg):
  if pc in (0x45f180,0x4849f0,0x50b250,0x4c2e60):
   raw=P.range(pc,size);assert raw==V.range(pc,size);VISITED[pc]=raw.hex();return
  if pc==0x59e520:self.ret(value=int(self.editor));return
  super().hook(u,pc,size,arg)
 def call(self,pc,arg):
  self.write(self.stack,self.stop);self.write(self.stack+4,arg);self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.reg_write(UC_X86_REG_ECX,self.component);self.u.emu_start(pc,self.stop,count=200000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.stop and self.u.reg_read(UC_X86_REG_ESP)==self.stack+8
 def snapshot(self):
  arrays=[]
  for o in [0x10,0x1c]:arrays.append({'values':[self.read(self.read(self.component+o)+i*4)for i in range(self.read(self.component+o+4))],'capacity':self.read(self.component+o+8)})
  return {'owner_matches':self.read(self.component+4)==self.object,'head':self.read(self.component+0xc),'arrays':arrays,'definition28':self.read(self.component+0x28)}
rows=[];inputs=[0,0x80000000,0x3f800000,0xbf800000,0x7f800000,0xff800000,0x7fa01234,0xff801234,0x7fc05678]
saved_inputs=[0,0x80000000,0x3f800000,0xbf800000,0x7f800000,0xff800000,0x7f7fffff,1,0x80000001]
for n,version,editor,savedcount,upversion in itertools.product([None,0,1,7,8,9,17],[0,1],[False,True],[0,3,20,0xffffffff],[0,1]):
 costs=None if n is None else[inputs[i%len(inputs)]for i in range(n)];saved=[]if savedcount==0xffffffff else[saved_inputs[(i+4)%len(saved_inputs)]for i in range(savedcount)]
 payload=W(upversion,*([0]if upversion else []),version,savedcount,*saved)
 if version==1:payload+=W(savedcount,*saved)+b'\0\0'
 else:payload+=W(0,0)
 m=Machine(payload,costs,editor);m.call(0x565fa0,m.object);before=m.snapshot();m.call(0x566bc0,m.world);assert m.r.at==len(payload);after=m.snapshot()
 rows.append({'costs':costs,'version':version,'editor':editor,'upversion':upversion,'payload_hex':payload.hex(),'before':before,'after':after})
p=Path(__file__).with_name('native-construction.json');p.write_text(json.dumps({'scope':__doc__,'original_sha256':hashlib.sha256(P.b).hexdigest(),'v14_sha256':hashlib.sha256(V.b).hexdigest(),'cases':rows,'instructions':[{'va':hex(a),'bytes':b}for a,b in sorted(VISITED.items())]},indent=2)+'\n');print({'cases':len(rows),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()})
