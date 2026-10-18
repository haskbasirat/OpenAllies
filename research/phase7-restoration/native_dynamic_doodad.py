"""Original DynamicDoodad saved ctor4DC6F0; full4DCD10 with prior separately-proven ObjectLoad seam. Actual two-slot4DCBF0 dispatch,491ED0 lookup,590160/58FE10 constructors,5903E0/58FD60 Load and50BCD0 publication execute."""
from pathlib import Path
import sys,json,struct,hashlib,itertools
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'research/phase6-kingdoms'))
from native_kingdoms import Machine as Base,P,V,VISITED
from unicorn.x86_const import *
W=lambda *v:struct.pack('<'+'I'*len(v),*v)
class Machine(Base):
 def __init__(self,payload,definition_mask,world_mode):
  super().__init__(payload,28);self.definition=self.read(self.object+4);self.write(self.definition+0x170,definition_mask)
  for slot in range(2):self.write(self.definition+0x174+slot*4,self.alloc(0x100) if definition_mask&(1<<slot) else 0);self.write(self.object+0x60+slot*4,0)
  self.write(self.world+0x48,world_mode);self.failed=None;self.calls=[]
 def hook(self,u,pc,size,arg):
  if pc==0x4ec7a0:self.ret(4);return # Object payload separate whole-function proof.
  if pc==0x50b250:
   raw=P.range(pc,size);assert raw==V.range(pc,size);VISITED[pc]=raw.hex();return
  if pc==0x858ad0:self.failed='missing-definition';u.emu_stop();return
  super().hook(u,pc,size,arg)
 def run(self):
  self.write(self.stack,self.stop);self.write(self.stack+4,self.world);self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.reg_write(UC_X86_REG_ECX,self.object)
  self.u.emu_start(0x4dcd10,self.stop,count=100000)
  if self.failed is None:assert self.u.reg_read(UC_X86_REG_EIP)==self.stop and self.u.reg_read(UC_X86_REG_ESP)==self.stack+8 and self.r.at==len(self.r.b)
  components=[]
  for slot in range(2):
   p=self.read(self.object+0x60+slot*4)
   components.append(None if not p else {'owner_matches':self.read(p+4)==self.object,'definition_matches':self.read(p+8)==self.read(self.definition+0x174+slot*4)==self.read(p+0x14),'head':self.read(p+0x10),'words':[self.read(p+o)for o in ([0x18]if slot==0 else range(0x18,0x30,4))]})
  table=self.read(self.world+0x284);ids=self.read(table+4);registrations=[]
  for identity in range(3):
   p=self.read(ids+identity*4)
   if p:registrations.append([identity,next(slot for slot in range(2)if p==self.read(self.object+0x60+slot*4)+0xc)])
  return {'components':components,'registrations':registrations,'failed':self.failed,'consumed':self.r.at}
rows=[]
for definition_mask,saved_mask,defaults,mode,uv,bv in itertools.product(range(4),range(4),range(4),[0,1],[0,1],[0,1]):
 payload=W(0,saved_mask,defaults)
 for slot in range(2):
  if saved_mask&(1<<slot):
   if slot==1:payload+=W(bv)
   payload+=W(0,uv)+(W(slot+1)if uv else b'')+W(0)
   if slot==1:payload+=W(2,0x80000000,0x7fc01234,0x3f800000,0xbf800000)+(W(0x40000000)if bv else b'')
 m=Machine(payload,definition_mask,mode);native=m.run();rows.append({'definition_mask':definition_mask,'saved_mask':saved_mask,'defaults':defaults,'world_mode':mode,'updatable_version':uv,'breakable_version':bv,'payload_hex':payload.hex(),'native':native})
# Exact derived ctor is checked separately with actual native base constructors.
sys.path.insert(0,str(ROOT/'research/phase5-actors'))
from original_creation import Machine as CtorMachine
ctors=[]
for saved in [1,0x12340007,0xffffffff]:
 m=CtorMachine(0x27f);m.u.mem_write(m.obj,b'\xcd'*0x80);m.call(0x4efb70,[saved]);m.invoke(0x4dc6f0,m.obj,[0,m.definition,saved]);ctors.append({'id':saved,'vtable':m.r(m.obj),'slots':[m.r(m.obj+0x60),m.r(m.obj+0x64)]})
 for a,n in m.visited.items():
  if P.base<=a<P.base+P.image_size:raw=P.range(a,n);assert raw==V.range(a,n);VISITED[a]=raw.hex()
p=Path(__file__).with_name('native-dynamic-doodad.json');p.write_text(json.dumps({'scope':__doc__,'original_sha256':hashlib.sha256(P.b).hexdigest(),'v14_sha256':hashlib.sha256(V.b).hexdigest(),'constructors':ctors,'cases':rows,'instructions':[{'va':hex(a),'bytes':b}for a,b in sorted(VISITED.items())]},indent=2)+'\n');print({'cases':len(rows),'constructors':len(ctors),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()})
