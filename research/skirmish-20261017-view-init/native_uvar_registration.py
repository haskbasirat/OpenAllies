"""Execute41F9D0 through all213 actual UVar constructor/registration calls.
Stop at the213th captured registration, before the later snapshot manager.
Captured4B3470 is a
registration transport seam, not a second source/manager implementation.
Actual typed Var constructors execute; inherited CString/heap/diagnostic seams
are excluded from executed instruction pins. No source/profile load claimed.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-net-integration'))
from native_launch_assignment import Machine as Base
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn.x86_const import UC_X86_REG_EIP,UC_X86_REG_FPCW
class Complete(Exception):pass

class Machine(Base):
 def __init__(self,image,pc):
  self.intercepted=set();super().__init__(False,image);self.mode='channels'
  self.u.reg_write(UC_X86_REG_FPCW,h.PRECISION[pc]);self.rows=[]
  self.seams[0x4b3470]=self.register
 def hook(self,u,pc,size,user):
  super().hook(u,pc,size,user)
  if u.reg_read(UC_X86_REG_EIP)!=pc:self.intercepted.add(pc)
 def register(self):
  index=self.arg(0);p=self.arg(1)
  assert index==len(self.rows),(index,len(self.rows))
  kind=self.r(p+0x24)
  self.rows.append({'index':index,'name':self.rs(p+0x1c),'kind':kind,
   'flags':self.r(p+0x28),
   'bool_current':self.rb(p)if kind==1 else None,
   'bool_default':self.rb(p+1)if kind==1 else None})
  if index==212:
   self.intercepted.add(0x4b3470);raise Complete()
  self.ret(8,p)
 def run(self):
  try:self.call(0x41f9d0,[],callee_pop=0)
  except Complete:pass
  assert len(self.rows)==213,len(self.rows)
  return self.rows

def main():
 rows=[];visited={};seams=set();canonical=None
 for image,pc in itertools.product(('original','v14'),h.PRECISION):
  m=Machine(image,pc);output=m.run();seams|=m.intercepted
  projected=output
  if canonical is None:canonical=projected
  else:assert canonical==projected
  rows.append({'image':image,'precision':pc,'registrations':projected})
  visited.update(m.visited)
 pins=[]
 for a,n in sorted(visited.items()):
  if a in seams or not(0x401000<=a<0x9ef000):continue
  b=h.P.range(a,n);assert b==h.V.range(a,n);pins.append({'va':hex(a),'bytes':b.hex()})
 doc={'schema':1,'scope':__doc__,'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,
  'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
  'seams_not_executed':[hex(a)for a in sorted(seams)],'executed_instructions':pins,'cases':rows}
 p=HERE/'native-uvar-registration.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'runs':len(rows),'registrations_per_run':len(canonical),'pins':len(pins),
  'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
