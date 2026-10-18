"""Original4E8EF8..4E93A0 Kingdom saved suffix with real insertion606750 rehash and4CDC70 Resource load. Empty named states; lookup/I-O/allocator seams only. Definition pointers are controlled synthetic native pointers; portable Rust tokens must not hash as pointers."""
from pathlib import Path
import sys,json,struct,hashlib,random
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'research/phase6-kingdoms'))
from native_kingdoms import Machine as Base,P,V,VISITED
from unicorn.x86_const import *
W=lambda *v:struct.pack('<'+'I'*len(v),*v)
S=lambda v:struct.pack('<H',len(v))+v.encode('utf-16le')
class Machine(Base):
 def __init__(self):
  super().__init__(b'',28)
  self.definitions={f'd{i}':0x2700000+i*40 for i in range(7)}
  for key,p in self.definitions.items():self.write(p+0x10,self.empty)
  self.write(self.object+0x35c,self.alloc(40));self.write(self.object+0x360,10);self.write(self.object+0x364,0);self.write(self.object+0x368,1)
  self.write(self.object+0x36c,0);self.write(self.object+0x370,0);nation=self.alloc(0x50);self.write(self.object+0x234,nation);self.write(nation+0x40,16);self.write(nation+0x10,self.empty)
  self.write(self.object+0x238,0x01010001);self.write(self.object+0x314,0)
  data=self.alloc(0x400);self.write(0xb36530,data);self.write(data+0x38,3)
  self.resources_registry=self.alloc(8);self.write(0xb3a660,self.resources_registry);self.resources={}
  for i in range(3):p=self.alloc(16);self.write(p+12,i);self.resources['r'+str(i)]=p
  self.write(self.object+0x258,self.alloc(32));self.write(self.object+0x25c,3);self.write(self.object+0x260,8)
 def hook(self,u,pc,size,arg):
  if pc in (0x4cdbb0,0x45f180,0x4849f0):
   raw=P.range(pc,size);assert raw==V.range(pc,size);VISITED[pc]=raw.hex();return
  if pc in (0x4bdba0,0x48d500):
   p=self.arg();key=bytes(u.mem_read(p,self.read(p-12)*2)).decode('utf16')
   self.ret(8 if pc==0x4bdba0 else 4,value=(self.definitions if pc==0x4bdba0 else self.resources).get(key,0));return
  super().hook(u,pc,size,arg)
 def run(self,version,refs,indices,bytes_,resources,empty_count=0):
  payload=(W(empty_count)if version>=12 else b'')
  if version>=15:payload+=W(len(refs))+b''.join(S(k)+W(v)for k,v in refs)
  if version>=16:payload+=W(len(indices),*indices)
  payload+=bytes(bytes_[:max(0,min(3,version-16))])
  if version>=20:payload+=W(len(resources))+b''.join(S(k)+W(v)for k,v in resources)
  self.r.b=payload;self.r.at=0
  self.write(self.stack+0x14,version);self.write(self.stack+0x40,self.world)
  self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.reg_write(UC_X86_REG_EBP,self.object);self.u.reg_write(UC_X86_REG_EDI,self.world);self.u.reg_write(UC_X86_REG_EBX,0)
  self.u.emu_start(0x4e8ef8,0x4e93a0,count=1000000)
  assert self.u.reg_read(UC_X86_REG_EIP)==0x4e93a0 and self.u.reg_read(UC_X86_REG_ESP)==self.stack and self.r.at==len(payload)
  pointers={v:k for k,v in self.definitions.items()};chains={k:[] for k in self.definitions};buckets=self.read(self.object+0x360);table=self.read(self.object+0x35c)
  for i in range(buckets):
   p=self.read(table+i*4)
   while p:chains[pointers[self.read(p)]].append(self.read(p+4));p=self.read(p+8)
  p=self.read(self.object+0x36c);tech=[]
  while p:tech.append(self.read(p));p=self.read(p+4)
  result={'buckets':buckets,'count':self.read(self.object+0x364),'lookup':{k:(v[0]if v else None)for k,v in chains.items()},'chains':chains,'indices':tech,'byte238':self.u.mem_read(self.object+0x238,1)[0],'byte239':self.u.mem_read(self.object+0x239,1)[0],'byte314':self.u.mem_read(self.object+0x314,1)[0],'resources':[self.read(self.read(self.object+0x258)+i*4)for i in range(3)]}
  return {'version':version,'refs':refs,'indices':indices,'source_bytes':bytes_,'resources':resources,'empty_named_count':empty_count,'native':result,'consumed':len(payload)}
rows=[]
for version in range(0,21):
 m=Machine();rows.append([m.run(version,[('d2',10),('d2',20),('',123),('missing',999),('d0',30)],[3,15,16,0xffffffff],[7,8,9],[('r2',0x7fc01234),('r0',0xbf800000)])])
rng=random.Random(0x606750)
for count in [19,20,21,22,45,46,47,48,93,94,95,200]:
 m=Machine();steps=[]
 for n in [count,17,0,53]:
  refs=[('d'+str(rng.randrange(7)),rng.getrandbits(32))for _ in range(n)]
  steps.append(m.run(20,refs,[],[1,0,1],[('r1',0xffffffff)],0xffffffff))
 rows.append(steps)
p=Path(__file__).with_name('native-kingdom-suffix.json');p.write_text(json.dumps({'scope':__doc__,'original_sha256':hashlib.sha256(P.b).hexdigest(),'v14_sha256':hashlib.sha256(V.b).hexdigest(),'cases':rows,'instructions':[{'va':hex(a),'bytes':b}for a,b in sorted(VISITED.items())]},indent=2)+'\n');print({'sequences':len(rows),'loads':sum(map(len,rows)),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()})
