"""Original Kingdom tactical58D560 and strategic58EB60 constructors; empty tactical58DBE0 source loads and all strategic58EE80 versions. Untouched words remain explicitly absent."""
from pathlib import Path
import sys,json,struct,hashlib,random
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'research/phase6-kingdoms'))
from native_kingdoms import Machine as Base,P,V,VISITED
from unicorn import UC_HOOK_MEM_WRITE
from unicorn.x86_const import *
W=lambda *v:struct.pack('<'+'I'*len(v),*v)
class Machine(Base):
 def __init__(self,payload):
  super().__init__(payload,28);self.u.mem_write(self.object,b'\xcd'*0x400);self.written=set();self.u.hook_add(UC_HOOK_MEM_WRITE,self.onwrite)
 def onwrite(self,u,access,a,n,value,arg):
  if self.object<=a<self.object+0x400:self.written.update(range(a-self.object,a-self.object+n))
 def hook(self,u,pc,size,arg):
  if pc==0x2e00000:
   a,n=self.arg(),self.arg(1)
   if self.object<=a<self.object+0x400:self.written.update(range(a-self.object,a-self.object+n))
  super().hook(u,pc,size,arg)
 def invoke(self,pc,args):
  self.write(self.stack,self.stop)
  for i,v in enumerate(args):self.write(self.stack+4+i*4,v)
  self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.reg_write(UC_X86_REG_ECX,self.object);self.u.emu_start(pc,self.stop,count=100000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.stop and self.u.reg_read(UC_X86_REG_ESP)==self.stack+4+len(args)*4
 def strategic(self):return [self.read(self.object+i*4)if all(i*4+j in self.written for j in range(4))else None for i in range(66)]
 def tactical(self):
  tables=[]
  for o in range(0,0xa0,16):
   p=self.read(self.object+o);count=self.read(self.object+o+4)
   tables.append({'buckets':count,'count':self.read(self.object+o+8),'owns':self.u.mem_read(self.object+o+12,1)[0],'zero_buckets':all(self.read(p+i*4)==0 for i in range(count))})
  return {'tables':tables,'counters':[[self.read(self.object+0xa0+i*20+j*4)for j in range(5)]for i in range(4)]}
rows=[]
for version in [None,0,1,2]:
 for count in ([0]if version is None else [0,0xffffffff]):
  groups=4+int(version==2);payload=b''if version is None else W(version,*([count]*(groups*(2 if version else 1))))
  m=Machine(payload);m.invoke(0x58d560,[]);before=m.tactical()
  if version is not None:m.invoke(0x58dbe0,[m.world])
  assert m.r.at==len(payload);rows.append({'kind':'Tactical','version':version,'empty_count_word':count,'before':before,'after':m.tactical(),'consumed':m.r.at})
rng=random.Random(0x58eb60)
for initialize in [False,True]:
 for version in [None,0,1]:
  for sample in range(3):
   words=[] if version is None else [rng.getrandbits(32)for _ in range(60 if version==0 else 66)];payload=b''if version is None else W(version,*words)
   m=Machine(payload);m.invoke(0x58eb60,[int(initialize)]);before=m.strategic()
   if version is not None:m.invoke(0x58ee80,[m.world])
   assert m.r.at==len(payload);rows.append({'kind':'Strategic','initialize':initialize,'version':version,'source_words':words,'before':before,'after':m.strategic(),'consumed':m.r.at})
out={'scope':__doc__,'original_sha256':hashlib.sha256(P.b).hexdigest(),'v14_sha256':hashlib.sha256(V.b).hexdigest(),'cases':rows,'instructions':[{'va':hex(a),'bytes':b}for a,b in sorted(VISITED.items())]}
p=Path(__file__).with_name('native-kingdom-ai.json');p.write_text(json.dumps(out,indent=2)+'\n');print({'cases':len(rows),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()})
