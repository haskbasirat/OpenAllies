from pathlib import Path
import sys,struct
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'research/phase5-actors'))
from original_creation import Machine as Base
from unicorn.x86_const import *
class Machine(Base):
 def __init__(self):
  super().__init__(0x27f);self.w(0x2255064,self.r(0xb35444))
  manager=self.alloc(16);vt=self.alloc(16);self.w(0xb39bc8,manager);self.w(manager+4,vt);self.w(vt+4,0x301f330);self.w(vt+8,0x301f340)
  self.w(self.poolvt,0x301f350)
  self.w(self.poolvt+8,0x301f360)
 def hook(self,u,pc,n,z):
  if pc==0x9738a9:self.ret(0x2255000)
  elif pc==0x6f7430:self.ret()
  elif pc==0x85c680:self.ret(self.alloc(0x100))
  elif pc in(0x301f330,0x301f340):self.ret()
  elif pc==0x301f350:self.ret(self.string([0]*self.arg())-16,pop=8)
  elif pc==0x301f360:
   old,count=self.arg(),self.arg(1);length=self.r(old+4);new=self.string([0]*count)-16
   u.mem_write(new+16,bytes(u.mem_read(old+16,min(count,length)*2)));self.w(new+4,min(count,length));self.ret(new,pop=12)
  elif pc==0x42bc40:
   name=self.arg(1);raw=bytearray()
   while (unit:=bytes(u.mem_read(name,2)))!=b'\0\0':raw.extend(unit);name+=2
   text=raw.decode('utf-16-le')
   out=self.arg();self.w(out,self.string([ord(c)for c in f'?{self.arg(2):07d}_{text}']));self.ret(out,pop=12)
  else:super().hook(u,pc,n,z)
 def node(self,name,flags=0,parent=0,ordinal=0,base=0):
  p=self.alloc(0x58);s=self.string([ord(c)for c in name]);empty=self.string([])
  for o,v in [(0,s),(4,empty),(0x10,parent),(0x14,ordinal),(0x18,base),(0x1c,p),(0x20,flags),(0x28,5),(0x30,1),(0x3c,5),(0x44,5),(0x4c,1),(0x54,empty)]:self.w(p+o,v)
  return p
 def name(self,node):
  p=self.r(node);n=self.r(p-12);return bytes(self.u.mem_read(p,n*2)).decode('utf-16-le')
 def add(self,parent,node,prepend=0):self.invoke(0x5b5c80,parent,[node,prepend])
 def snapshot(self,parent):
  return [(self.name(self.r(self.r(parent+0x34)+i*4)),self.r(self.r(self.r(parent+0x34)+i*4)+0x14),self.r(self.r(self.r(parent+0x34)+i*4)+0x20))for i in range(self.r(parent+0x38))]
if __name__=='__main__':
 m=Machine();root=m.node('Root',1)
 for i,flag in enumerate([4,16,16]):
  try:m.add(root,m.node('Ability',flag,ordinal=i));print('added',i,m.snapshot(root))
  except Exception:print('trace',*[hex(p)for p in m.recent]);raise
 derived=m.node('Derived',1,base=root);m.add(derived,m.node('Ability',16,ordinal=3))
 print('beforecopy',m.snapshot(derived))
 try:m.invoke(0x5b57c0,root,[derived,0]);print('aftercopy',m.snapshot(derived))
 except Exception:print('trace',*[hex(p)for p in m.recent]);raise
 actual=m.node('Thing',0,base=derived)
 try:m.invoke(0x5b57c0,derived,[actual,0]);print('actual',m.snapshot(actual))
 except Exception:print('trace',*[hex(p)for p in m.recent]);raise
