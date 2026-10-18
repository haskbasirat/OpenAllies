"""Original Area4DBA40/Camera4DC1F0 saved readers with prior GObjectLoad seam. Area shape46DB70, Updatable publication, prepend order and versioned localization execute. Camera v0 transform conversion remains outside this bounded restoration."""
from pathlib import Path
import sys,json,struct,hashlib,itertools,random
ROOT=Path(__file__).resolve().parents[2];sys.path.insert(0,str(ROOT/'research/phase6-kingdoms'))
from native_kingdoms import Machine as Base,P,V,VISITED
W=lambda *v:struct.pack('<'+'I'*len(v),*v)
S=lambda text:struct.pack('<H',len(text))+text.encode('utf-16le',errors='surrogatepass')
from unicorn.x86_const import *
class Machine(Base):
 def __init__(self,payload,world_version):
  super().__init__(payload,world_version)
  for o in [0x64,0x80]:self.write(self.object+o,self.empty)
  self.write(self.object+0x60,0xa289d8)
  for o in [0x64,0x68,0x6c,0x70,0x74,0x78,0x7c,0x84,0x88]:self.write(self.object+o,0)
 def hook(self,u,pc,size,arg):
  if pc==0x4ec7a0:self.ret(4);return
  if pc==0x50b250:
   raw=P.range(pc,size);assert raw==V.range(pc,size);VISITED[pc]=raw.hex();return
  super().hook(u,pc,size,arg)
 def run(self,kind):
  if kind=='Camera':self.write(self.object+0x64,self.empty);self.write(self.object+0x60,0)
  self.write(self.stack,self.stop);self.write(self.stack+4,self.world);self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.reg_write(UC_X86_REG_ECX,self.object)
  self.u.emu_start(0x4dba40 if kind=='Area' else 0x4dc1f0,self.stop,count=100000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.stop and self.u.reg_read(UC_X86_REG_ESP)==self.stack+8 and self.r.at==len(self.r.b)
  o=0x80 if kind=='Area' else 0x64;p=self.read(self.object+o);n=self.read(p-12);label=list(struct.unpack('<'+'H'*n,self.u.mem_read(p,2*n)))
  if kind=='Camera':return {'word60':self.read(self.object+0x60),'label':label,'consumed':self.r.at}
  entries=[];p=self.read(self.object+0x84)
  while p:record=self.read(p);entries.append([self.read(record),self.u.mem_read(record+4,1)[0]]);p=self.read(p+4)
  table=self.read(self.world+0x284);registered=self.read(self.read(table+4))==self.object+0x60
  return {'shape':[self.read(self.object+o)for o in range(0x68,0x80,4)],'label':label,'entries':entries,'registered_id0':registered,'consumed':self.r.at}
rows=[];rng=random.Random(0x4dba40)
for kind in ['Area','Camera']:
 for version,world_version,uv,text in itertools.product(range(3)if kind=='Area'else[1,2],[21,22,28],[0,1],['','label','\ud800']):
  entries=[[0,0],[0x12340007,255],[0x12340007,2]];shape=[rng.getrandbits(32)for _ in range(6)];word=0x7fc01234
  payload=(W(uv,*([0]if uv else []))if kind=='Area'and world_version>=22 else b'')+W(version)+S(text)
  if kind=='Area':payload+=W(*shape)+(W(len(entries))+b''.join(W(v)+bytes([b])for v,b in entries)if version>=1 else b'')
  else:payload+=W(word)
  m=Machine(payload,world_version);rows.append({'kind':kind,'version':version,'world_version':world_version,'updatable_version':uv,'payload_hex':payload.hex(),'native':m.run(kind)})
sys.path.insert(0,str(ROOT/'research/phase5-actors'));from original_creation import Machine as CtorMachine
ctors=[]
for kind,entry,start,end in [('Area',0x4db6f0,0x60,0x8c),('Camera',0x4dbf40,0x60,0x68)]:
 for saved in [1,0x12340007,0xffffffff]:
  m=CtorMachine(0x27f);m.u.mem_write(m.obj,b'\xcd'*0x100);m.call(0x4efb70,[saved]);m.invoke(entry,m.obj,[0,m.definition,saved]);words={hex(o):m.r(m.obj+o)for o in range(start,end,4)};label_offset=0x80 if kind=='Area' else 0x64;p=m.r(m.obj+label_offset);words.pop(hex(label_offset));ctors.append({'kind':kind,'saved_id':saved,'words':words,'label_length':m.r(p-12)})
  for a,n in m.visited.items():
   if P.base<=a<P.base+P.image_size:raw=P.range(a,n);assert raw==V.range(a,n);VISITED[a]=raw.hex()
p=Path(__file__).with_name('native-map-objects.json');p.write_text(json.dumps({'scope':__doc__,'original_sha256':hashlib.sha256(P.b).hexdigest(),'v14_sha256':hashlib.sha256(V.b).hexdigest(),'constructors':ctors,'cases':rows,'instructions':[{'va':hex(a),'bytes':b}for a,b in sorted(VISITED.items())]},indent=2)+'\n');print({'cases':len(rows),'constructors':len(ctors),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()})
