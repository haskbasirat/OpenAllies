"""Original56B720 Denizen constructor and56C320 Load with an explicitly empty definition Inventory. Original list construction, excess saved occupant discard, versioned bytes/34 word and handle-head insertion execute; command objects remain outside this bounded path."""
from pathlib import Path
import sys,json,struct,hashlib,itertools
ROOT=Path(__file__).resolve().parents[2];sys.path.insert(0,str(ROOT/'research/phase6-kingdoms'))
from native_kingdoms import Machine as Base,P,V,VISITED
from unicorn.x86_const import *
W=lambda *v:struct.pack('<'+'I'*len(v),*v)
class Machine(Base):
 def __init__(self):
  super().__init__(b'',28);self.component=self.alloc(0x100);self.u.mem_write(self.component,b'\xcd'*0x100);self.write(self.read(self.object+4)+0x35c,0)
 def hook(self,u,pc,size,arg):
  if pc==0x50b250:
   raw=P.range(pc,size);assert raw==V.range(pc,size);VISITED[pc]=raw.hex();return
  super().hook(u,pc,size,arg)
 def call(self,entry,arg):
  self.write(self.stack,self.stop);self.write(self.stack+4,arg);self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.reg_write(UC_X86_REG_ECX,self.component);self.u.emu_start(entry,self.stop,count=100000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.stop and self.u.reg_read(UC_X86_REG_ESP)==self.stack+8
 def snapshot(self):
  handles=[];p=self.read(self.component+0x2c)
  while p:handles.append(self.read(p));p=self.read(p+4)
  return {'words':{hex(o):self.read(self.component+o)for o in [0x10,0x14,0x24,0x28,0x34,0x38,0x3c,0x8c,0x90]},'bytes':[self.u.mem_read(self.component+o,1)[0]for o in [0x20,0x21]],'handles':handles,'owner_matches':self.read(self.component+4)==self.object,'group_owner_matches':self.read(self.component+0x88)==self.component,'groups_zero':all(self.read(self.component+o)==0 for o in range(0x48,0x88,4)),'occupant_list_empty':self.read(self.component+0x18)==self.read(self.component+0x1c)==0,'head':self.read(self.component+0xc),'command_list_empty':self.read(self.component+0x40)==self.read(self.component+0x44)==0}
rows=[]
for version,uv,n,hn in itertools.product(range(5),[0,1],[0,3,0xffffffff],[0,3,0xffffffff]):
 payload=W(uv,*([0xffffffff]if uv else []),version)
 if version>=1:payload+=bytes([255])
 if version>=2:payload+=bytes([17])
 if version>=3:payload+=W(n,*([v for i in range(n)for v in [0x12340000+i,0x7fa01234]]if n!=0xffffffff else []))
 if version>=4:payload+=W(0x80000000)
 if version>=3:payload+=W(hn,*([7,0,7]if hn==3 else []))+W(0xffffffff)
 m=Machine();m.call(0x56b720,m.object);before=m.snapshot();after=[]
 for repeat in range(2):m.r.b=payload;m.r.at=0;m.call(0x56c320,m.world);assert m.r.at==len(payload);after.append(m.snapshot())
 rows.append({'version':version,'updatable_version':uv,'payload_hex':payload.hex(),'before':before,'after':after})
p=Path(__file__).with_name('native-denizen.json');p.write_text(json.dumps({'scope':__doc__,'original_sha256':hashlib.sha256(P.b).hexdigest(),'v14_sha256':hashlib.sha256(V.b).hexdigest(),'cases':rows,'instructions':[{'va':hex(a),'bytes':b}for a,b in sorted(VISITED.items())]},indent=2)+'\n');print({'constructors':len(rows),'loads':len(rows)*2,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()})
