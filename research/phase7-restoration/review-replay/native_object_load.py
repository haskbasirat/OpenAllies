"""Run original GObject Load with original Thing field reader and variant RNG.
Only profiler/no-input logging, host CString storage and streamI/O use seams.
Nested source payloads are zero-count, as all225851 original map placements are.
"""
from pathlib import Path
import sys,struct,json,random,hashlib
ROOT=Path(__file__).resolve().parents[3]
sys.path.insert(0,str(ROOT/'research/phase6-kingdoms'))
from native_teams import Machine as Base,P,V,VISITED
from unicorn.x86_const import *
W=lambda *v:struct.pack('<'+'I'*len(v),*v)
B=lambda v:struct.unpack('<I',struct.pack('<f',v))[0]
class Machine(Base):
 def __init__(self,payload,minimum,maximum,weights,total,enabled,seed,pc):
  super().__init__(payload);self.u.reg_write(UC_X86_REG_FPCW,pc)
  self.write(0xb36534,self.world);self.write(self.world+0xd4,seed)
  definition=self.read(self.object+4);self.write(definition+0x28,minimum);self.write(definition+0x2c,maximum)
  self.write(definition+0x108,int(enabled));self.write(definition+0x110,len(weights));self.write(definition+0x128,total)
  entries=self.alloc(len(weights)*4);self.write(definition+0x10c,entries)
  for i,weight in enumerate(weights):entry=self.alloc(0x30);self.write(entry+0x2c,weight);self.write(entries+i*4,entry)
  self.write(self.object+0x40,0xffffffff)
 def hook(self,u,pc,size,arg):
  if pc==0x77dc50:self.ret(8);return
  if pc==0x4f02b0:self.ret(value=self.world+0xd4);return
  super().hook(u,pc,size,arg)
 def run(self):
  self.write(self.stack,self.stop);self.write(self.stack+4,self.world);self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.reg_write(UC_X86_REG_ECX,self.object)
  self.u.emu_start(0x4ec7a0,self.stop,count=100000);assert self.u.reg_read(UC_X86_REG_EIP)==self.stop
  return {'consumed':self.r.at,'scale_bits':self.read(self.object+0x3c),'variant':self.read(self.object+0x40),'word50':self.read(self.object+0x50),'word54':self.read(self.object+0x54),'rng_state':self.read(self.world+0xd4)}
def main():
 rng=random.Random(0x4ec7a0);rows=[]
 for pc,cw in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
  for version in range(6):
   for sample in range(8):
    minimum=B(rng.uniform(.001,2));maximum=B(rng.uniform(2,4));multiplier=B(rng.uniform(.01,100));seed=rng.getrandbits(32)
    weights=[B(1),B(2),B(3)];total=B(6);enabled=bool(sample&1);variant=[-1,-2,0,2,3,99,1,-1][sample]
    thing=W(2)+b'\0\0'+W(12,*[B(v) for v in (1,2,3,0,1,0)],0)
    payload=thing+W(version,multiplier)+(W(variant&0xffffffff) if version>=1 else b'')+(W(0x12345678) if version>=2 else b'')+(struct.pack('<H',65535) if version>=3 else b'')+(W(0) if version>=4 else b'')
    m=Machine(payload+b'FOLLOWING',minimum,maximum,weights,total,enabled,seed,cw);native=m.run();assert native['consumed']==len(payload)
    rows.append({'precision':pc,'version':version,'minimum':minimum,'maximum':maximum,'multiplier':multiplier,'variant':variant,'weights':weights,'total':total,'enabled':enabled,'rng_before':seed,'native':native})
 out={'schema':1,'scope':__doc__,'original_sha256':hashlib.sha256(P.b).hexdigest(),'v14_sha256':hashlib.sha256(V.b).hexdigest(),'cases':rows,'instructions':[{'va':hex(a),'bytes':b}for a,b in sorted(VISITED.items())]}
 p=Path(__file__).with_name('native-object-load.json');p.write_text(json.dumps(out,indent=2)+'\n');print(json.dumps({'cases':len(rows),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}))

_original_run = Machine.run
_abi_calls = 0
def _review_run(self,*args,**kwargs):
 global _abi_calls
 regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
 for i,r in enumerate(regs): self.u.reg_write(r,0x11111111*(i+1))
 result=_original_run(self,*args,**kwargs)
 assert self.u.reg_read(UC_X86_REG_ESP)==self.stack+8
 assert [self.u.reg_read(r)for r in regs]==[0x11111111*(i+1)for i in range(4)]
 _abi_calls+=1
 return result
Machine.run=_review_run

if __name__=='__main__':
 main()
 print({'independent_abi_calls':_abi_calls})
