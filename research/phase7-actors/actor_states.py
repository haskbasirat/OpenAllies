"""Original actor state registration, masks, state selection and mutation."""
from pathlib import Path
import sys,struct,json,hashlib,random
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase6-mapworld'))
from native_load_oracle import Machine as Base,P,V,H,W
from unicorn.x86_const import *

class Machine(Base):
 def __init__(self):
  super().__init__(b'');self.heap=0x20a0000;self.observed={};self.records=[];self.events=[];self.names={}
  self.stringheap=0x20c0000;self.catalog=0x2090000;self.array=0x2091000;self.w(0xb36530,self.catalog);self.w(self.catalog+0x2e4,self.array)
  self.w(0x209f000,0x209f010);self.w(0x209f01c,0x301f400);self.w(0x209f10c,99999)
 def text(self,p):
  data=bytearray()
  while True:
   b=bytes(self.u.mem_read(p,2));p+=2
   if b==b'\0\0':break
   data+=b
  return data.decode('utf-16le')
 def hook(self,u,pc,size,user):
  if P.base<=pc<P.base+P.image_size:self.observed[pc]=bytes(u.mem_read(pc,size))
  if pc==0x6f7810:
   p=self.heap;self.heap+=(self.arg()+15)&~15;self.ret(p)
  elif pc==0x401b00:
   s=self.text(self.arg());p=self.stringheap;self.stringheap+=0x100
   self.w(p+12,99999);self.w(p+4,len(s));u.mem_write(p+16,s.encode('utf-16le')+b'\0\0');self.names[p+16]=s
   self.w(u.reg_read(UC_X86_REG_ECX),p+16);self.ret(p+16,4)
  elif pc==0x4012c0:self.ret(self.arg())
  elif pc==0x856370:self.ret(0x209f000)
  elif pc==0x301f400:self.ret(0x209f100)
  elif pc==0x4b4e60:
   p=self.arg();index=self.r(p+12);assert index==len(self.records)
   self.records.append({'index':index,'name':self.names[self.r(p+8)],'group':self.r(p+0x2c),'exclusive':bool(u.mem_read(p+0x30,1)[0]),'address':p})
   self.w(self.array+index*4,p);self.w(self.catalog+0x2e8,len(self.records));self.ret(p,4)
  elif pc in (0x4d4a90,0x4d16b0,0x4d1350):
   if pc==0x4d4a90:self.events.append(['enter',self.r(self.arg()+12),None if not self.arg(1) else self.r(self.arg(1)+12)])
   elif pc==0x4d16b0:self.events.append(['leave',self.r(self.arg()+12)])
   else:self.events.append(['rebuild'])
   self.ret(pop={0x4d4a90:8,0x4d16b0:4,0x4d1350:0}[pc])
  else:super().hook(u,pc,size,user)
 def invoke(self,entry,this,*args):
  self.w(self.stack,self.stop)
  for i,x in enumerate(args):self.w(self.stack+4+4*i,x)
  self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.reg_write(UC_X86_REG_ECX,this);self.u.emu_start(entry,self.stop,count=100000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.stop,(hex(self.u.reg_read(UC_X86_REG_EIP)),hex(entry))
  assert self.u.reg_read(UC_X86_REG_ESP)==self.stack+4+4*len(args)
 def register(self):
  self.invoke(0x477fc0,self.catalog)
  return {'records':[{k:v for k,v in x.items() if k!='address'}for x in self.records],'masks':list(struct.unpack('<31Q',self.u.mem_read(0xb3ac40,31*8)))}
 def set_state(self,bits,flags,index,enable):
  self.events=[];self.w(self.thing+0x100,bits&0xffffffff);self.w(self.thing+0x104,bits>>32);self.w(self.thing+8,flags)
  self.invoke(0x4d64f0,self.thing,index,int(enable))
  return {'bits':self.r(self.thing+0x100)|(self.r(self.thing+0x104)<<32),'flags':self.r(self.thing+8),'events':self.events}
 def group(self,bits,group):
  self.w(self.thing+0x100,bits&0xffffffff);self.w(self.thing+0x104,bits>>32);self.invoke(0x4d49b0,self.thing,group)
  p=self.u.reg_read(UC_X86_REG_EAX);return self.r(p+12) if p else None

def main():
 m=Machine();registration=m.register();rng=random.Random(0x4d64f0);rows=[];groups=[]
 for index in range(42):
  for bits in [0,(1<<42)-1,1<<index,rng.getrandbits(42)]:
   for enabled in [False,True]:
    for flags in [0x100,0x80000100]:
     c={'bits':bits,'flags':flags,'index':index,'enable':enabled};rows.append({'input':c,'output':m.set_state(**c)})
 for group in range(31):
  for bits in [0,(1<<42)-1]+[rng.getrandbits(42) for _ in range(8)]:groups.append({'bits':bits,'group':group,'selected':m.group(bits,group)})
 for a,b in m.observed.items():assert b==P.range(a,len(b))==V.range(a,len(b))
 out={'schema':1,'original_sha256':H(P.b),'v14_sha256':H(V.b),'producer_sha256':H(Path(__file__).read_bytes()),'registration':registration,'state_cases':rows,'group_cases':groups,'instructions':[{'va':hex(a),'bytes':b.hex()}for a,b in sorted(m.observed.items())],'scope':'Full477FC0 registration plus474270 descriptors/473490 masks, full4D64F0 and4D49B0. CString/heap and registry transport are seams; enter/leave/ability-rebuild callbacks are observed required runtime boundaries, not simulated gameplay effects.'}
 p=HERE/'native-actor-states.json';p.write_text(json.dumps(out,indent=2));print(json.dumps({'states':len(registration['records']),'state_cases':len(rows),'group_cases':len(groups),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
