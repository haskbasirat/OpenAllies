"""Original TGI type classifier and fixed/float assignment, with native CRT parsing.

Only TLS allocation/string-pool allocation/field insertion are external leaves.
Literal conversion, percentage multiplication and field constructors execute.
"""
from pathlib import Path
import json,struct,sys,random
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase5-actors'))
from original_creation import Machine as Base,P,V,H
from unicorn.x86_const import *

class Machine(Base):
 def __init__(self,pc):
  super().__init__(pc);self.w(0x2255064,self.r(0xb35444));self.value=None
 def hook(self,u,pc,n,z):
  if pc==0x9738a9:self.ret(0x2255000)
  elif pc==0x85c680:self.ret(self.alloc(256))
  elif pc==0x5b3f00:self.value=self.r(self.arg()+16);self.ret(pop=4)
  else:super().hook(u,pc,n,z)
 def scalar(self,text,kind):
  s=self.string([ord(c)for c in text]);self.w(self.record,s)
  self.w(self.stack,self.stop);self.w(self.stack+4,self.record);self.u.reg_write(UC_X86_REG_ESP,self.stack)
  self.u.emu_start(0x5bd6e0,self.stop,count=100000);classification=self.u.reg_read(UC_X86_REG_EAX)&255
  parser,token,node=0x2256000,0x2257000,0x2258000
  self.w(parser,node);self.w(parser+0x40,self.string([ord(c)for c in 'field']));self.w(parser+0x38,3 if kind=='float' else 4)
  self.w(token+0x18,s);self.w(token+0x10,7)
  self.value=None
  if classification in [1,2,3]:self.invoke(0x5bb9f0 if kind=='float' else 0x5b9e20,parser,[token])
  return {'text':text,'kind':kind,'classification':classification,'value_bits':self.value}

def main():
 texts=['0','-0','+0','1','-1','200','3500','2.5','0.1','1.45','50%','0.1%','100%','1f','2d','1.25k','-0.1k','1e3','false','true','$health * 2','1 + 2','"3"','.5','-.5','3.','+3.25','16777217','16777219','0.00000000000001','99999999999999']
 rng=random.Random(0x5b9e20)
 texts += [f'{rng.randint(-100000,100000)}.{rng.randrange(1000000):06d}'+('%' if i%3==0 else '') for i in range(100)]
 rows=[];visited={}
 for name,pc in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
  m=Machine(pc)
  for kind in ['fixed','float']:
   for text in texts:rows.append({'precision':name,**m.scalar(text,kind)})
  visited.update(m.visited)
 spans=[]
 for a,n in sorted(visited.items()):
  if not P.base<=a<P.base+P.image_size:continue
  if spans and spans[-1][1]>=a:spans[-1][1]=max(spans[-1][1],a+n)
  else:spans.append([a,a+n])
 pins=[]
 for a,z in spans:
  raw=P.range(a,z-a);assert raw==V.range(a,z-a);pins.append({'start':hex(a),'end':hex(z),'sha256':H(raw)})
 doc={'schema':1,'passed':True,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':H(P.b),'v14_sha256':H(V.b),'cases':rows,'native_windows':pins,'percent_constant_bits':struct.unpack('<I',P.range(0xa03078,4))[0],'seams':['C locale thread data supplied to original CRT through __getptd; conversion/classification instructions execute unchanged.','String-pool retain/allocation and successful fixed-size field allocation are fixture leaves.','Original field constructor executes; insertion captures field+10 rawbits instead of requiring complete parser registry.','Invalid classifications are recorded without invoking exception-throwing assignment. No native general template loader claimed.']}
 p=HERE/'typed-scalar-native.json';p.write_text(json.dumps(doc,indent=2)+'\n');print(json.dumps({'cases':len(rows),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
