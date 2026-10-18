"""Fresh actual 5065D0 typed SVar construction. Registration is an explicit
transport, never Windows-captured current values. Typed constructors/CString
execute; manager order/count is transported to support final auto-index Add.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-net-integration'))
from native_launch_assignment import Machine as Base
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn.x86_const import UC_X86_REG_EIP,UC_X86_REG_FPCW,UC_X86_REG_ECX
class Machine(Base):
 def __init__(self,image,pc):
  self.intercepted=set();super().__init__(False,image);self.mode='channels';self.rows=[]
  self.u.reg_write(UC_X86_REG_FPCW,h.PRECISION[pc]);self.seams[0x4b3470]=self.register
 def hook(self,u,pc,n,z):
  super().hook(u,pc,n,z)
  if u.reg_read(UC_X86_REG_EIP)!=pc:self.intercepted.add(pc)
 def register(self):
  index=self.arg(0);v=self.arg(1);kind=self.r(v+0x24)
  assert index==len(self.rows),(index,len(self.rows),hex(self.u.reg_read(UC_X86_REG_ECX)),self.rows[-2:])
  row=dict(index=index,name=self.rs(v+0x1c),kind=kind,flags=self.r(v+0x28),
   words=[self.r(v+i*4)for i in range(7)],string=self.rs(v+0x10)if kind==4 else None)
  if kind==1:row['words'][0]&=255
  #CString slots are allocation identities, not numeric source values.
  row['words'][4]=None
  self.rows.append(row);self.w(0xb39aa8+0xc,len(self.rows));self.ret(8,v)
 def run(self):
  self.call(0x4b2ce0,[],this=0xb39aa8,callee_pop=0)
  self.call(0x5065d0,[],this=0xb39aa8,callee_pop=0)
  return self.rows

def main():
 cases=[];pins={};seams=set();canonical=None
 for image in('original','v14'):
  for pc in h.PRECISION:
   m=Machine(image,pc);rows=m.run()
   if canonical is None:canonical=rows
   else:assert canonical==rows,(image,pc)
   cases.append(dict(image=image,precision=pc,registrations=rows));pins.update(m.visited);seams|=m.intercepted
 for a,n in pins.items():
  if a not in seams and 0x401000<=a<0x9ef000:assert h.P.range(a,n)==h.V.range(a,n)
 output=dict(schema=1,scope=__doc__,cases=cases,
  executed=[dict(address=hex(a),bytes=h.P.range(a,n).hex())for a,n in sorted(pins.items())if a not in seams and 0x401000<=a<0x9ef000],
  intercepted=[hex(a)for a in sorted(seams)])
 raw=(json.dumps(output,indent=1)+'\n').encode();p=HERE/'native-constructors.json'
 if '--write'in sys.argv:p.write_bytes(raw)
 else:assert p.read_bytes()==raw
 print(json.dumps(dict(rows=len(canonical),machines=len(cases),pins=len(output['executed']),sha256=hashlib.sha256(raw).hexdigest())))
if __name__=='__main__':main()


