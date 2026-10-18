"""Original Structure/Settlement attachment graph predicates and parent read."""
from pathlib import Path
import sys,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(HERE));from actor_states import Machine as Base,P,V,H
from unicorn.x86_const import *
class Machine(Base):
 def __init__(self,c):
  super().__init__();self.c=c;self.structure=0x20e0000;self.parent=0x20e1000;self.settlement=0x20e2000;self.replacement=0x20e3000
  self.w(self.thing,0x20e4000);self.w(0x20e4000+0x158,0x4ce980 if c['virtual']=='ordinary' else 0x301f500)
  self.w(self.thing+0x94,self.structure if c['structure'] else 0);self.w(self.structure+4,self.thing);self.w(self.structure+0x10,self.parent if c['parent'] else 0);self.w(self.parent+0x98,self.settlement);self.w(self.settlement+0x14,self.thing if c['matches'] else self.parent);self.w(self.replacement+0x10,0x20e5000)
 def hook(self,u,pc,size,user):
  if pc==0x301f500:
   if self.c['replace']:self.w(self.thing+0x94,self.replacement)
   self.ret(1 if self.c['virtual']=='true' else 0)
  else:super().hook(u,pc,size,user)
 def run(self):
  self.invoke(0x4ce980,self.thing);attached=self.u.reg_read(UC_X86_REG_EAX)
  self.invoke(0x4ce9a0,self.thing);p=self.u.reg_read(UC_X86_REG_EAX)
  return {'attached':bool(attached),'parent':0 if p==0 else 2 if p==self.parent else 3}
def main():
 rows=[];visited={}
 for s in [False,True]:
  for p in [False,True]:
   for match in [False,True]:
    for v in ['ordinary','false','true']:
     if v=='true' and not s:continue
     for replace in ([False,True]if v=='true' else [False]):
      c={'structure':s,'parent':p,'matches':match,'virtual':v,'replace':replace};m=Machine(c);rows.append({'input':c,'output':m.run()});visited.update(m.observed)
 for a,b in visited.items():assert b==P.range(a,len(b))==V.range(a,len(b))
 out={'schema':1,'cases':rows,'original_sha256':H(P.b),'v14_sha256':H(V.b),'producer_sha256':H(Path(__file__).read_bytes()),'instructions':[{'va':hex(a),'bytes':b.hex()}for a,b in sorted(visited.items())],'scope':'Full4CE980/588990 and4CE9A0. Exact valid graph records; optional derived virtual overrides are explicit callbacks which can replace Structure before the parent read. Native null-dereference domains excluded from execution and rejected by Rust.'}
 p=HERE/'native-attachment.json';p.write_text(json.dumps(out,indent=2));print(json.dumps({'cases':len(rows),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
