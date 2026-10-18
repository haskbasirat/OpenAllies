"""Whole native category initializer/descriptor constructors; registry is a sink."""
from pathlib import Path
import json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase5-actors'))
from original_creation import Machine as Base,P,V,H
from unicorn.x86_const import UC_X86_REG_ECX
class Machine(Base):
 def hook(self,u,pc,n,z):
  if pc==0x401b00:
   self.w(u.reg_read(UC_X86_REG_ECX),self.string([ord(c)for c in self.text(self.arg())]));self.ret(pop=4)
  elif pc==0x4915f0:self.ret(0,pop=4)
  elif pc==0x491740:
   p=self.arg();self.rows.append({'name':self.text(self.r(p+8)),'registry_index':self.r(p+0xc),'native_kind':self.r(p+0x18),'runtime_class':self.r(p+0x1c),'view_type':self.r(p+0x20)});self.ret(pop=8)
  else:super().hook(u,pc,n,z)
def main():
 m=Machine(0x27f);m.rows=[];owner,manager,entries=0x225a000,0x225b000,0x225c000
 m.w(owner+0x3b4,manager);m.w(manager+0x24,entries);m.w(manager+0x28,0);m.w(manager+0x2c,100);m.w(manager+0x34,64)
 m.invoke(0x476700,owner,[])
 spans=[]
 for a,n in sorted(m.visited.items()):
  if not P.base<=a<P.base+P.image_size:continue
  if spans and spans[-1][1]>=a:spans[-1][1]=max(spans[-1][1],a+n)
  else:spans.append([a,a+n])
 pins=[]
 for a,z in spans:
  raw=P.range(a,z-a);assert raw==V.range(a,z-a);pins.append({'start':hex(a),'end':hex(z),'sha256':H(raw)})
 raw=P.range(0x4bca00,1576);assert raw==V.range(0x4bca00,1576)
 pins.append({'start':'0x4bca00','end':hex(0x4bca00+1576),'sha256':H(raw),'evidence':'Static factory dispatch selects descriptor+18 definition kind, not instantiated runtime class+1C.'})
 p=HERE/'categories-native.json';p.write_text(json.dumps({'schema':1,'passed':True,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':H(P.b),'v14_sha256':H(V.b),'categories':m.rows,'native_windows':pins,'scope':'Whole476700,actual474920 descriptor constructor,actual4B4E60 ordinal assignment and766DE0 ordered arrayappend. Successfulallocation andliteralCStringconstruction leaves; duplicate lookup reportsabsent for43unique names; hashbucketinsertion491740 isobservationsink. Runtimeclass+1C/nativekind+18/viewtype+20/index+C proven forfresh builtins; userdata overrides notinferred.'},indent=2)+'\n');print(len(m.rows),H(p.read_bytes()))
if __name__=='__main__':main()
