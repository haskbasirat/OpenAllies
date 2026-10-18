from pathlib import Path
import sys,json,struct
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE))
from probe_lists import Machine as Base
from native_ground_definitions import P,V,H
from unicorn.x86_const import *
def bits(x):return struct.unpack('<I',struct.pack('<f',x))[0]
class Machine(Base):
 def __init__(self):
  super().__init__();self.values={};self.kinds={};self.enums=[];self.diagnostics=[];self.baseflags=0
  self.u.mem_write(0x301e000,b'\xd9\x05'+struct.pack('<I',0x225f000)+b'\xc2\x04\x00')
 def nameat(self,p):return P.range(p,160).decode('utf-16-le').split('\0')[0]
 def hook(self,u,pc,n,z):
  if pc==0x49dc40:self.w(u.reg_read(UC_X86_REG_ECX)+0x1c,self.baseflags);self.ret(pop=4)
  elif pc==0x5b54d0:
   node=u.reg_read(UC_X86_REG_ECX);name=self.nameat(self.arg());value=self.values[node][name];p=self.string(list(map(ord,value)));r=self.alloc(4);self.w(r,p);self.ret(r,pop=4)
  elif pc==0x5b5130:
   node=u.reg_read(UC_X86_REG_ECX);name=self.nameat(self.arg());self.w(0x225f000,self.values[node][name]);u.reg_write(UC_X86_REG_EIP,0x301e000)
  elif pc==0x5b5380:self.ret(self.values[u.reg_read(UC_X86_REG_ECX)][self.nameat(self.arg())],pop=4)
  elif pc==0x497220:
   pointer=self.r(self.arg());value=bytes(u.mem_read(pointer,self.r(pointer-12)*2)).decode('utf-16-le');kind=['DISTURBABLE','BREAKABLE'].index(value);p=self.alloc(8);self.w(p,p+4);self.w(p+4,kind);self.ret(p,pop=4)
  elif pc==0x42bf10:self.ret(pop=12)
  elif pc==0x49c0b0:self.ret(0x2251000,pop=4)
  elif pc==0x492360:self.enums.append({'kind':self.arg(),'name':self.nameat(self.arg(1))});self.ret(pop=12)
  elif pc==0x410c40:self.diagnostics.append(self.nameat(self.arg()));self.ret()
  else:super().hook(u,pc,n,z)
 def load(self,types,flags):
  self.baseflags=flags;base=self.node('Base',1);self.add(base,self.node('Component',4));node=self.node('Thing',base=base)
  for i,kind in enumerate(types):self.add(node,self.node('Component',16,ordinal=i))
  self.invoke(0x5b57c0,base,[node,0])
  for i in range(self.r(node+0x38)):
   child=self.r(self.r(node+0x34)+4*i);kind=types[i]
   self.values[child]={'type':['DISTURBABLE','BREAKABLE'][kind],'update_time':bits(.1+i),'affect_radius':bits(2+i),'recharge_time_min':bits(3+i),'recharge_time_max':bits(5+i),'break_impulse':bits(11+i),'toppled':1,'topple_duration':bits(.7),'bounce_duration':bits(.4),'bounce_percentage':bits(.25),'regrow':1,'regrow_sink_duration':bits(1.5),'regrow_dead_duration':bits(8),'regrow_grow_duration':bits(2),'trampled_aspect_ids':''}
  data=self.alloc(0x17c);self.invoke(0x4922e0,data,[node]);out=[]
  for kind in range(2):
   ptr=self.invoke(0x491ed0,data,[kind])
   offsets=[0x10,0x14,0x18,0x1c]if kind==0 else[0x10,0x14,0x18,0x1c,0x20,0x24,0x30,0x34,0x38,0x3c,0x40]
   out.append(None if not ptr else{hex(o):self.r(ptr+o)for o in offsets})
  return {'mask':self.r(data+0x170),'flags':self.r(data+0x1c),'components':out,'diagnostics':self.diagnostics}
rows=[];visited={}
for flags in [0,0xaaaaaaaa,0xffffffff]:
 for types in [[],[0],[1],[0,1],[1,0],[0,0],[1,1]]:
  m=Machine();out=m.load(types,flags);assert out['flags']==flags;assert out['mask']==sum(1<<x for x in set(types));rows.append({'types':types,'base_flags':flags,'output':out});visited.update(m.visited)
m=Machine();m.invoke(0x492530,0x2240000,[1]);enums=m.enums;visited.update(m.visited)
spans=[]
for a,n in sorted(visited.items()):
 if not P.base<=a<P.base+P.image_size:continue
 if spans and spans[-1][1]>=a:spans[-1][1]=max(spans[-1][1],a+n)
 else:spans.append([a,a+n])
spans += [[0x491ed0,0x491f30],[0x4920d0,0x4921ff],[0x4922e0,0x492357],[0x4c7da0,0x4c80f4]]
pins=[]
for a,b in spans:
 raw=P.range(a,b-a);assert raw==V.range(a,b-a);pins.append({'start':hex(a),'end':hex(b),'sha256':H(raw)})
d={'passed':True,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':H(P.b),'v14_sha256':H(V.b),'cases':rows,'enum':enums,'ranges':pins,'seams':['Base ObjectData49DC40 is a flag marker seam, independently proven base Thing flags apply; derived loader never alters those bits.','Typed string/fixed/bool field getters return supplied values; native component constructors, numeric stores, validation, slot overwrite and mask OR execute.','Type registry lookup is pre-resolved; actual enum constructor registration captures names/indices separately.','FX Action list loader42BF10 is empty opaque seam; optional aspect resolver is a supplied identity seam. No FX or model loading claim.']};(HERE/'native-doodads.json').write_text(json.dumps(d,indent=2));print('passed',len(rows),enums,H((HERE/'native-doodads.json').read_bytes()))
