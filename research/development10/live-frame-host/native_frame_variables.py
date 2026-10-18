"""Native constructors for the four current frame variables only.

The UVar constructor prefix executes through all213 registration calls,
stopping at the last registration before the later snapshot-manager work;
registration itself is a transport seam. The AVar slice starts at
40371A and stops at its registration, before unwinding the absent caller.
No source/profile load or current UI mutation is claimed by this fixture.
"""
from pathlib import Path
import sys,json,itertools,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/skirmish-20261017-view-init'))
import native_uvar_registration as base
from unicorn.x86_const import UC_X86_REG_ESI
class Done(Exception):pass
class Machine(base.Machine):
 def register(self):
  index=self.arg(0);p=self.arg(1);kind=self.r(p+0x24)
  row={'index':index,'name':self.rs(p+0x1c),'kind':kind,'flags':self.r(p+0x28),
   'current':self.rb(p)if kind==1 else self.r(p)}
  self.rows.append(row)
  if self.avar or index==212:raise Done()
  self.ret(8,p)
 def run(self,avar):
  self.avar=avar
  if avar:self.u.reg_write(UC_X86_REG_ESI,self.alloc(0x100))
  try:self.call(0x40371a if avar else 0x41f9d0,[],callee_pop=0)
  except Done:pass
  return self.rows if avar else [r for r in self.rows if r['index']in(0x72,0x73,0xd4)]
def main():
 rows=[];visited={};seams=set()
 for image,pc,avar in itertools.product(('original','v14'),base.h.PRECISION,(False,True)):
  m=Machine(image,pc);out=m.run(avar);visited.update(m.visited);seams|=m.intercepted|{0x4b3470}
  rows.append({'image':image,'precision':pc,'class':'AVar'if avar else'UVar','values':out})
 pins=[]
 for va,size in sorted(visited.items()):
  if va in seams or not(0x401000<=va<0x9ef000):continue
  raw=base.h.P.range(va,size);assert raw==base.h.V.range(va,size)
  pins.append({'va':hex(va),'bytes':raw.hex()})
 doc={'schema':1,'scope':__doc__,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
  'original_sha256':base.h.ORIGINAL_SHA,'v14_sha256':base.h.V14_SHA,
  'seams_not_executed':[hex(x)for x in sorted(seams)],'pins':pins,'cases':rows}
 p=HERE/'native-frame-variables.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'runs':len(rows),'pins':len(pins),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
