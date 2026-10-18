from pathlib import Path
import sys
sys.path.insert(0,'research/phase7-definitions');from probe_lists import Machine as Base
from unicorn.x86_const import *
class Machine(Base):
 def hook(self,u,pc,n,z):
  if pc in [0x404f60,0x404ed0,0x404ea0]:self.ret()
  elif pc==0x5b3b10:
   out=self.arg();self.w(out,self.string(list(map(ord,'fixture'))));self.ret(out,pop=4)
  else:super().hook(u,pc,n,z)
 def make(self,value):
  node=self.node('SVars');parser=self.alloc(0x80);token=self.alloc(0x30);self.w(parser,node);self.w(parser+0x40,self.string(list(map(ord,'UnitSupplyEnabled'))));self.w(parser+0x38,5);self.w(token+0x18,self.string(list(map(ord,value))));self.w(token+0x10,7)
  self.invoke(0x5ba010,parser,[token]);return node
m=Machine();settings=m.alloc(0x80)
try:
 m.invoke(0x4b2ce0,settings,[])
 for value in ['false','true','false']:
  node=m.make(value);m.invoke(0x4b3c10,settings,[node]);count=m.r(settings+12);array=m.r(settings+8);var=m.r(array);print(value,'count',count,'var',hex(var),'bits',m.r(var),'type',m.r(var+36),'flags',m.r(var+40))
except Exception as e:print('ERROR',type(e).__name__,str(e),*[hex(x)for x in m.recent]);raise
