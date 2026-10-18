"""Original Ability recharge/XYsign and resource cost dense-vector operations."""
from pathlib import Path
import sys,json,struct
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE))
from native_ground_definitions import Machine as Base,P,V,H,bits
from unicorn.x86_const import *
class Machine(Base):
 def hook(self,u,pc,n,z):
  if pc==0x5b5130:
   key=self.arg();amount=self.values[key];p=0x225f000;self.w(p,amount)
   u.mem_write(0x301e000,b'\xd9\x05'+struct.pack('<I',p)+b'\xc2\x04\x00');u.reg_write(UC_X86_REG_EIP,0x301e000)
  elif pc==0x47a360:
   self.enums.append([self.arg(),self.arg(1)]);self.ret(pop=12)
  else:super().hook(u,pc,n,z)
 def recharge(self,length,reload,minimum,cw):
  self.u.reg_write(UC_X86_REG_FPCW,cw);p=self.alloc(0x78);definition=self.alloc(0x80);self.w(p,definition);self.w(definition+0x20,minimum);self.w(p+4,reload)
  if length is not None:
   arr=self.alloc(4);vec=self.alloc(12);ptr=self.alloc(4);anim=self.alloc(0x20);self.w(p+0x40,1);self.w(p+0x3c,arr);self.w(arr,vec);self.w(vec+4,1);self.w(vec,ptr);self.w(ptr,anim);self.w(anim+0x10,length)
  self.run(0x4bff32,0x4bffca,[(UC_X86_REG_ESI,p)]);return self.r(p+0x18)
 def cost(self,kinds,amounts):
  manager=self.alloc(0x30);resources=self.alloc(4*len(kinds));self.w(manager+0x24,resources);self.w(manager+0x28,len(kinds));self.w(0xb3a660,manager);self.values={}
  for i,(kind,amount)in enumerate(zip(kinds,amounts)):
   resource=self.alloc(0x20);self.w(resource+0x18,kind);name=self.string([65+i]);self.w(resource+8,name);self.w(resources+4*i,resource);self.values[name]=amount
  vector=self.alloc(12);values=self.alloc(4*(len(kinds)+1));self.w(vector,values);self.w(vector+8,len(kinds)+1)
  self.w(self.stack,self.stop);self.w(self.stack+4,vector);self.w(self.stack+8,0x2240000);self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.emu_start(0x473cb0,self.stop,count=100000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.stop
  return [self.r(values+4*i)for i in range(self.r(vector+4))]
def main():
 recharge=[];costs=[];visited={}
 for name,cw in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
  for length,reload,minimum in [(None,1.,0.),(0.,0.,0.),(1.,2.,3.),(1.,2.,4.),(.1,.2,.3),(1.,2**-24,1.),(16777216.,1.,16777216.),(2.,-1.,3.)]:
   m=Machine();inp=[None if length is None else bits(length),bits(reload),bits(minimum)];out=m.recharge(*inp,cw);recharge.append({'precision':name,'input':inp,'output_bits':out});visited.update(m.visited)
 for kinds in [[],[0],[1],[2],[0,1,1,1,2,1,2],[1,0,2,0],[0,0,0]]:
  amounts=[bits(i+.25)for i in range(len(kinds))];m=Machine();output=m.cost(kinds,amounts);assert output==[v if k==0 else 0 for k,v in zip(kinds,amounts)];costs.append({'kinds':kinds,'amounts':amounts,'output_bits':output});visited.update(m.visited)
 m=Machine();m.enums=[];m.run(0x48a3f6,0x48a422,[(UC_X86_REG_EBX,0),(UC_X86_REG_ESI,0x2240000)]);visited.update(m.visited)
 enums=[]
 for value,p in m.enums:enums.append([value,P.range(p,80).decode('utf-16-le').split('\0')[0]])
 spans=[]
 for a,n in sorted(visited.items()):
  if not P.base<=a<P.base+P.image_size:continue
  if spans and spans[-1][1]>=a:spans[-1][1]=max(spans[-1][1],a+n)
  else:spans.append([a,a+n])
 spans += [[0x9fdc48,0x9fdc4c],[0x4bf990,0x4bfa6a],[0x4becb0,0x4bee20],[0x4bf390,0x4bf990]]
 pins=[]
 for a,b in spans:
  raw=P.range(a,b-a);assert raw==V.range(a,b-a);pins.append({'start':hex(a),'end':hex(b),'sha256':H(raw)})
 doc={'passed':True,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':H(P.b),'v14_sha256':H(V.b),'recharge_cases':recharge,'cost_cases':costs,'resource_type_enums':enums,'spawn_xy_multiplier_bits':struct.unpack('<I',P.range(0x9fdc48,4))[0],'ranges':pins,'seams':['Recharge original complete arithmetic branch4BFF32..4BFFCA executes with prepared native clip-list shape.','Cost loader original473CB0 and45F180 execute; typed GetFixed supplies binary32 through a fixture fld/ret leaf. Registry order/type slots supplied explicitly.','Resource enum actual callsites execute; insertion leaf records key/value. AbilityData XY multiplier and complete definition loader/ctor are statically pinned; no full4BF990 runtime construction claim.']}
 (HERE/'native-ability-arithmetic.json').write_text(json.dumps(doc,indent=2));print('passed',len(recharge),len(costs),enums)
if __name__=='__main__':main()
