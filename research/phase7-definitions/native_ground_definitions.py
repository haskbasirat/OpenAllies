"""Native layout slot multiplicity and Detection finalization fragments."""
from pathlib import Path
import sys,json,struct,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase5-actors'))
from original_creation import Machine as Base,P,V,H
from unicorn.x86_const import *
class Machine(Base):
 def __init__(self):super().__init__(0x27f)
 def run(self,pc,end,registers):
  self.u.reg_write(UC_X86_REG_ESP,self.stack)
  for reg,value in registers:self.u.reg_write(reg,value)
  self.u.emu_start(pc,end,count=100000);assert self.u.reg_read(UC_X86_REG_EIP)==end
 def hook(self,u,pc,n,z):
  if pc==0x404ed0:self.ret()
  else:super().hook(u,pc,n,z)
def bits(n):return struct.unpack('<I',struct.pack('<f',n))[0]
def main():
 layouts=[];detections=[];visited={}
 for counts in [(0,),(1,),(3,1,1,1,2,1),(2,3),(1,0,4)]:
  for primary in [None,len(counts)-1]:
   for morale in [False,True]:
    m=Machine();layout=m.alloc(128);array=m.alloc(400);m.w(layout+0x28,array);m.w(layout+0x30,100);m.w(layout+0x18,0xffffffff)
    for i,count in enumerate(counts):
     slot=m.alloc(16);m.w(slot+8,count);m.w(layout+0x20,i+1);m.w(m.stack+0x10,layout)
     m.run(0x4c3794,0x4c3826,[(UC_X86_REG_EAX,int(morale)),(UC_X86_REG_ESI,slot),(UC_X86_REG_EBP,layout)])
     m.run(0x4c383b,0x4c385e,[(UC_X86_REG_EAX,int(primary==i)),(UC_X86_REG_EBP,layout)])
    count=m.r(layout+0x2c);words=[m.r(array+i*4)for i in range(count)]
    assert words==[i for i,n in enumerate(counts)for _ in range(n)]
    layouts.append({'counts':counts,'primary':primary,'morale_line':morale,'expanded_groups':words,'stored_count':count,'stored_primary':m.r(layout+0x18),'stored_morale_count':m.r(layout+0x4c)});visited.update(m.visited)
 for active in [False,True]:
  for values in itertools.product([0.,3.,7.],repeat=5):
   m=Machine();p=m.definition;m.w(p+0x174,(1<<22)if active else 0)
   words=[bits(v)for v in values]
   for offset,value in zip([0x1cc,0x2fc,0x420,0x364,0x368],words):m.w(p+offset,value)
   m.run(0x468b16,0x468bd9,[(UC_X86_REG_EBP,p)])
   actual=m.r(p+0x1cc);assert actual==bits(max(values)if active else values[0])
   detections.append({'active':active,'input_bits':words,'output_bits':actual});visited.update(m.visited)
 spans=[]
 for a,n in sorted(visited.items()):
  if not P.base<=a<P.base+P.image_size:continue
  if spans and spans[-1][1]>=a:spans[-1][1]=max(spans[-1][1],a+n)
  else:spans.append([a,a+n])
 spans += [[0x464bd0,0x468ce0],[0x4c3610,0x4c3884]]
 pins=[]
 for a,b in spans:
  raw=P.range(a,b-a);assert raw==V.range(a,b-a);pins.append({'start':hex(a),'end':hex(b),'sha256':H(raw)})
 doc={'schema':1,'passed':True,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':H(P.b),'v14_sha256':H(V.b),'layout_cases':layouts,'detection_cases':detections,'ranges':pins,'seams':['Layout fragment executes original multiplicity, expanded slot mapping, morale count and primary-slot writes; successful preallocated capacity avoids allocator. Typed field inputs are supplied from separately proven declaration resolver.','Detection whole finalizer and original four-way max execute; diagnostic logger is a no-op. Finite source fields only.','Full actor loader is statically pinned for source field/store identity, not represented as a complete execution. Formation loading, property registry and runtime layout construction are separate.']}
 (HERE/'native-ground-definitions.json').write_text(json.dumps(doc,indent=2));print('passed',len(layouts),len(detections))
if __name__=='__main__':main()

