"""Execute native31-slot factory dispatch, observing actual ctor choice."""
from pathlib import Path
import sys,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(HERE));from actor_states import Machine as Base,P,V,H
from unicorn.x86_const import *
TABLE=json.loads((HERE/'component-dispatch.json').read_text())['rows'];CTORS={int(r['constructor'],16):r for r in TABLE}
class Machine(Base):
 def __init__(self,slot,mask):
  super().__init__();self.slot=slot;self.mask=mask;self.calls=[];self.allocated=[];self.w(self.thing+4,self.definition);self.w(self.definition+0x174,mask)
 def hook(self,u,pc,size,user):
  if P.base<=pc<P.base+P.image_size:self.observed[pc]=bytes(u.mem_read(pc,size))
  if pc==0x6f7810:self.allocated.append(self.arg())
  if pc in CTORS:
   self.calls.append(hex(pc));self.ret(u.reg_read(UC_X86_REG_ECX),4)
  else:super().hook(u,pc,size,user)
 def run(self):
  self.invoke(0x4cf480,self.thing,self.slot)
  return {'constructors':self.calls,'allocation_sizes':self.allocated,'installed':bool(self.r(self.thing+0x60+4*self.slot))}
def main():
 cases=[];visited={}
 for slot,mask in [(s,0x7fffffff)for s in range(31)]+[(4,((n&1)<<7)|((n&2)<<22)|((n&4)<<6))for n in range(8)]:
  m=Machine(slot,mask);cases.append({'slot':slot,'mask':mask,'output':m.run()});visited.update(m.observed)
 for a,b in visited.items():assert b==P.range(a,len(b))==V.range(a,len(b))
 out={'schema':1,'cases':cases,'original_sha256':H(P.b),'v14_sha256':H(V.b),'producer_sha256':H(Path(__file__).read_bytes()),'instructions':[{'va':hex(a),'bytes':b.hex()}for a,b in sorted(visited.items())],'scope':'Entire4CF480 dispatch with successfulallocation; actualconstructorentries recorded then returnthis. Proves31slot sizes/selection/installation and Mind precedence, not leafconstructor behavior for unsupported nonground components.'}
 p=HERE/'native-factory-dispatch.json';p.write_text(json.dumps(out,indent=2));print(json.dumps({'cases':len(cases),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
