"""Ground InitCreate, native supply predicate and actual periodic queue/RNG."""
from pathlib import Path
import sys,json,struct,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase6-queues'))
from native_queues import Machine as Base,ORIGINAL as P,ACCEPTED as V,VISITED,bits
from unicorn.x86_const import *
import hashlib
H=lambda b:hashlib.sha256(b).hexdigest()
INIT={'Body':0x562310,'Character':0x438ac0,'Mover':0x438ac0,'Experience':0x438ac0,'Properties':0x563fd0,'Organization':0x563fd0,'Element':0x563fd0,'Detection':0x563fd0,'Economy':0x56f250}
class Machine(Base):
 def __init__(self,c):
  super().__init__({'counts':[1,c['buckets'],1,1,1],'seed':c['seed'],'clock':{'begin_bits':0,'end_bits':bits(.0625),'step_bits':bits(.0625)},'precision':c['precision']})
  self.c=c;self.health_changes=[];self.comp=self.alloc(0x100);self.actor=self.alloc(0x200);self.definition=self.alloc(0x800);self.record=self.alloc(0x60);self.flag=self.alloc(4)
  self.write(self.comp+4,self.actor);self.write(self.comp+8,0xa3b0c0);self.write(self.actor+4,self.definition);self.write(self.actor+0x100,c['state_bits']);self.write(self.actor+0x7c,0x1234 if c['organization'] else 0)
  self.write(self.definition+0x2a8,c['damage_rate']);self.write(self.definition+0x348,int(c['supply_required']));self.write(self.flag,int(c['unit_supply']));self.write(0xb3d658,1);self.write(0xb3d654,self.flag)
  self.write(self.record+0x40,c['word40']);self.write(self.record+0x48,c['word48']);self.owners[99]=self.comp+8;self.names[self.comp+8]=99
  if c['existing']:self.action({'type':'add','queue':1,'owner':99,'data':0})
 def hook(self,u,pc,size,data):
  if pc==0x561e50:
   VISITED[pc]=P.range(pc,size).hex();self.health_changes.append([self.arg(i)for i in range(4)]);self.ret(16);return
  super().hook(u,pc,size,data)
 def run(self):
  result=self.call(INIT[self.c['kind']],self.comp,[self.record if self.c['record'] else 0])&255
  return {'result':result,'health_changes':self.health_changes,'queue':self.snapshot()[1],'rng_state':self.read(self.world+0xd4)}
def main():
 cases=[]
 for pc in [24,53,64]:
  for kind in INIT:
   domain=range(32) if kind=='Body' else range(4)
   for n in domain:
    c={'kind':kind,'precision':pc,'seed':[0,0x12345678,0xdeadbeef][n%3],'buckets':[1,3,7][n%3],
    'state_bits':(1<<21) if n&1 else 0,'organization':bool(n&2),'unit_supply':bool(n&4),'damage_rate':bits([0,1,-2,.03125][n%4]),
    'record':n%5!=0,'word40':int(n%7==0),'word48':int(n%11==0),'supply_required':bool(n&1),'existing':bool(n&8)}
    cases.append({'input':c,'output':Machine(c).run()})
 # Dedicated supply-positive gates and duplicate guard coverage.
 for record,word40,word48,existing,state in itertools.product([False,True],[0,1],[0,1],[False,True],[0,1<<21]):
  c={'kind':'Body','precision':53,'seed':987654321,'buckets':7,'state_bits':state,'organization':False,'unit_supply':True,'damage_rate':bits(1.5),'record':record,'word40':word40,'word48':word48,'supply_required':True,'existing':existing};cases.append({'input':c,'output':Machine(c).run()})
 for a,b in VISITED.items():assert P.range(a,len(bytes.fromhex(b)))==V.range(a,len(bytes.fromhex(b)))
 out={'schema':1,'original_sha256':H(P.b),'v14_sha256':H(V.b),'producer_sha256':H(Path(__file__).read_bytes()),'queue_dependency_sha256':H((ROOT/'research/phase6-queues/native_queues.py').read_bytes()),'cases':cases,'instructions':[{'va':hex(a),'bytes':b}for a,b in sorted(VISITED.items())],'scope':'Native InitCreate leaves and561B40 UnitSupplyEnable predicate, actual50AC60/50B220/50B2C0 queue linkage and RNG.561E50 health-change arguments are a seam; Rust calls already independently proven concrete Body setter. SVar reference is populated before call; no default assumed. Heap and diagnostic logging remain original queue oracle seams.'}
 p=HERE/'native-leaf-initializers.json';p.write_text(json.dumps(out,indent=2));print(json.dumps({'cases':len(cases),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
