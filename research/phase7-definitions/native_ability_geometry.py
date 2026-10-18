from pathlib import Path
import sys,json,struct
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE))
from native_ability_arithmetic import Machine as Base,P,V,H,bits
from unicorn.x86_const import *
class Machine(Base):
 def hook(self,u,pc,n,z):
  if pc==0x410c40:self.failed=True;u.emu_stop()
  else:super().hook(u,pc,n,z)
 def geometry(self,area,minimum,factor,impulse,cw):
  self.u.reg_write(UC_X86_REG_FPCW,cw);p=self.alloc(0x9c);self.w(p+0x38,area);self.values={0xa21d8c:minimum,0xa21cd0:factor,0xa21c28:impulse};self.failed=False
  self.u.reg_write(UC_X86_REG_ESI,p);self.u.reg_write(UC_X86_REG_EDI,0);self.u.reg_write(UC_X86_REG_EBP,0x2240000);self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.emu_start(0x4bf54c,0x4bf67f,count=10000)
  return None if self.failed else[self.r(p+o)for o in [0x58,0x5c,0x60]]
rows=[];visited={}
for name,cw in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
 for values in [[bits(0),bits(1),bits(1),0],[bits(2),bits(1),bits(.5),0],[bits(1),bits(1),bits(1),0],[bits(2),0,0,0],[bits(2),0x38d1b717,bits(1),0x38d1b717],[bits(2),bits(2.00005),bits(1.00005),bits(1)],[bits(2),bits(-1),bits(1),0],[bits(2),bits(3),bits(1),0],[bits(2),bits(1),bits(-.1),0],[bits(2),bits(1),bits(1.1),0],[bits(2),bits(1),bits(1),bits(-.1)]]:
  m=Machine();out=m.geometry(*values,cw);rows.append({'precision':name,'input_bits':values,'output_bits':out});visited.update(m.visited)
spans=[]
for a,n in sorted(visited.items()):
 if not P.base<=a<P.base+P.image_size:continue
 if spans and spans[-1][1]>=a:spans[-1][1]=max(spans[-1][1],a+n)
 else:spans.append([a,a+n])
pins=[]
for a,b in spans:
 raw=P.range(a,b-a);assert raw==V.range(a,b-a);pins.append({'start':hex(a),'end':hex(b),'sha256':H(raw)})
d={'passed':True,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':H(P.b),'v14_sha256':H(V.b),'cases':rows,'ranges':pins,'seams':['Original4BF54C..4BF67F executes; typed GetFixed field getter returns supplied f32, prepared zero ctor fields represent separately pinned4BEC B0 defaults.','Invalid cases stop at original410C40 diagnostic; full application exception/display handling is not executed.']};(HERE/'native-ability-geometry.json').write_text(json.dumps(d,indent=2));print('passed',len(rows))
