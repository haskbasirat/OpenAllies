"""Original fixed-SVar registration and setter for MoraleRetreatPenalty.

Original constructor401710, Settings Add4B3470, and typed setter4B2F80 run.
Literal binary32 setter arguments/string-pool/allocation are explicit inputs. No whole
source directory or game session is executed. Numeric cases do not reuse the
earlier Bool-only source-order claim as numeric evidence.
"""
from pathlib import Path
import hashlib,json,sys,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2];GAME=ROOT.parent
sys.path.insert(0,str(GAME/'reconstruction/tools/python-packages'))
sys.path.insert(0,str(ROOT/'research/phase7-definitions'))
from probe_lists import Machine as Base
from original_creation import P,V
from unicorn import UC_HOOK_MEM_READ,UC_HOOK_MEM_WRITE
from unicorn.x86_const import *
sha=lambda raw:hashlib.sha256(raw).hexdigest()
class Machine(Base):
 def __init__(self,pc):
  super().__init__();self.u.reg_write(UC_X86_REG_FPCW,pc)
  self.u.hook_add(UC_HOOK_MEM_READ|UC_HOOK_MEM_WRITE,self.null_guard,begin=0,end=0xfff)
 def null_guard(self,u,access,address,size,value,user):
  raw=bytes(u.mem_read(u.reg_read(UC_X86_REG_EIP),4));assert 0x64 in raw[:2],f'non-FS null access at{u.reg_read(UC_X86_REG_EIP):x}'
 def hook(self,u,pc,n,z):
  if pc==0x404f60:self.ret()
  elif pc in [0x404ed0,0x404ea0]:raise RuntimeError(f'unexpected native setting warning/error {pc:x}')
  elif pc==0x5b3b10:
   out=self.arg();self.w(out,self.string(list(map(ord,'typed-svar-fixture'))));self.ret(out,pop=4)
  else:super().hook(u,pc,n,z)
def main():
 rows=[];visited={}
 for precision,pc in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
  for values in [('5','0'),('5','-1.5','2.25')]:
   m=Machine(pc);settings=m.alloc(0x80);m.invoke(0x4b2ce0,settings,[]);out=[]
   # Native pre-registration as type2/flags0, rather than allowing the first
   # source to choose the variable's type.
   var=m.alloc(0x2c);m.invoke(0x401710,var,[m.string(list(map(ord,'MoraleRetreatPenalty'))),0x40a00000,0]);m.invoke(0x4b3470,settings,[0,var])
   for value in values:
    v=m.r(m.r(settings+8));bits=struct.unpack('<I',struct.pack('<f',float(value)))[0];m.invoke(0x4b2f80,v,[bits]);out.append(dict(value=value,input_bits=bits,bits=m.r(v),kind=m.r(v+0x24),flags=m.r(v+0x28),count=m.r(settings+12)))
   assert all(v['kind']==2 and v['flags']==0 and v['count']==1 for v in out)
   rows.append(dict(precision=precision,values=values,states=out));visited.update(m.visited)
 pins=[]
 for a,n in sorted(visited.items()):
  if P.base<=a<P.base+P.image_size:
   raw=P.range(a,n);assert raw==V.range(a,n);pins.append(dict(address=f'{a:08x}',bytes=raw.hex()))
 report=dict(scope=__doc__,source_sha256=sha(P.b),v14_sha256=sha(V.b),cases=rows,passed=len(rows),instruction_pins=pins,producer_sha256=sha(Path(__file__).read_bytes()),seams=['Existing probe_lists allocator/string pool.','Literal values enter as binary32 setter arguments; Settings LoadData4B3C10 dispatch is static evidence, not executed here.','Null page guard permits only instructions with FS prefix; no null-data dereference allowed.'])
 (HERE/'native-morale-settings.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8');print('passed',len(rows))
if __name__=='__main__':main()
