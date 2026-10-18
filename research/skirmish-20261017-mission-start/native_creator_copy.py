"""Native Creator clone and resource+88 replacement slice, original/v14/all PCs.
Executes514550/514790 and5145E0 including actual outer Team/Kingdom/rule
vector copy loops. Native allocation/base-constructor state, capacity resize,
CString assignment and opaque Kingdom record assignment are explicit transport
seams. This proves outer value copies and retained source+50 identity, NOT the
full nested Kingdom copier, source defaults, scenario selection or whole5A2B90.
Also runs exact5A2FCF..3019 and ctor5A2831..285F with explicit valid frame and
actual class-getter/clone bodies; destructor is a transport seam. No PE patches.
"""
from pathlib import Path
import hashlib,itertools,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h
from unicorn.x86_const import *
ORIGINAL=h.P
class Machine(h.Machine):
 def __init__(self,image,pc):
  h.P=ORIGINAL if image=='original' else h.V;super().__init__(pc)
  self.seam_entries=set();self.clones=[];self.events=[]
  def seam(a,f):self.seams[a]=f;self.seam_entries.add(a)
  seam(0x6f7810,lambda m:m.ret(value=m.alloc(m.arg())))
  seam(0x513eb0,self.base_ctor)
  for address,size in [(0x50f6b0,12),(0x4a4ce0,108),(0x41cd80,12)]:
   seam(address,lambda m,s=size:self.resize(m,s))
  seam(0x405b80,self.string_copy)
  seam(0x4a4c00,self.kingdom_copy)
  seam(0x514410,self.destroy)
 def base_ctor(self,m):
  p=m.ecx();m.w32(p,0xa3214c)
  for off in (8,0x14,0x2c):m.w32(p+off,0,0,0)
  m.w32(p+0x24,0,0);m.w32(p+0x38,0,0,0,0,0,0)
  m.ret(pop=4,value=p)
 def resize(self,m,size):
  p=m.ecx();n=m.arg();m.w32(p,m.alloc(max(1,n)*size),0,n);m.ret(pop=4)
 def string_copy(self,m):m.w32(m.ecx(),m.r32(m.arg()));m.ret(pop=4,value=m.ecx())
 def kingdom_copy(self,m):m.u.mem_write(m.ecx(),bytes(m.u.mem_read(m.arg(),108)));m.ret(pop=4,value=m.ecx())
 def destroy(self,m):self.events.append('destroy_old88');m.ret(pop=4)
 def vector(self,p,off,rows):
  n=len(rows);data=self.alloc(max(1,n)*len(rows[0])*4 if n else 4)
  for i,row in enumerate(rows):self.w32(data+i*len(row)*4,*row)
  self.w32(p+off,data,n,n)
 def source(self,kind,n):
  p=self.alloc(0x54);self.w32(p,0xa3220c if kind=='Load' else 0xa3214c)
  teams=[[100+i*3+j for j in range(3)] for i in range(n)]
  kingdoms=[[1000+i*27+j for j in range(27)] for i in range(n)]
  rules=[[2000+i*3+j for j in range(3)] for i in range(n)]
  for off,rows in ((8,teams),(0x14,kingdoms),(0x2c,rules)):self.vector(p,off,rows)
  scalars={str(off):0x31000000+off for off in (0x24,0x28,0x38,0x3c,0x40,0x44,0x48,0x4c)}
  for off,value in scalars.items():self.w32(p+int(off),value)
  link=self.alloc(0xb4) if kind=='Load' else 0
  self.w32(p+0x50,link)
  return p,{'kind':kind,'teams':teams,'kingdoms':kingdoms,'rules':rules,'scalars':scalars}
 def projection(self,p,source):
  rows={}
  for name,off,size in [('teams',8,3),('kingdoms',0x14,27),('rules',0x2c,3)]:
   n=self.r32(p+off+4);data=self.r32(p+off)
   rows[name]=[[self.r32(data+i*size*4+j*4) for j in range(size)]for i in range(n)]
   rows[name+'_outer_independent']=data!=self.r32(source+off)
  rows['scalars']={str(off):self.r32(p+off) for off in (0x24,0x28,0x38,0x3c,0x40,0x44,0x48,0x4c)}
  rows['kind']='Load' if self.r32(p)==0xa3220c else 'Base'
  rows['same_source50']=self.r32(p+0x50)==self.r32(source+0x50) if rows['kind']=='Load' else None
  rows['different_allocation']=p!=source
  return rows
 def clone(self,kind,n):
  p,inputs=self.source(kind,n);clone=self.call(0x514790 if kind=='Load' else 0x514550,p,[0]);self.clones.append(clone)
  return inputs,self.projection(clone,p)
 def replacement(self,mode,oldkind):
  old,_=self.source(oldkind,1);selected,_=self.source('Load',2)
  resource=self.alloc(0xb4);self.w32(resource+0x88,old)
  frame=self.alloc(0x80)+0x40;self.w32(frame+8,selected,mode)
  self.u.reg_write(UC_X86_REG_EBP,frame);self.u.reg_write(UC_X86_REG_ESI,resource);self.u.reg_write(UC_X86_REG_ESP,h.STACK)
  self.u.emu_start(0x5a2fcf,0x5a3019,count=10000)
  assert self.u.reg_read(UC_X86_REG_EIP)==0x5a3019
  result=self.r32(resource+0x88);changed=result!=old
  return {'mode':mode,'old_kind':oldkind,'changed':changed,'events':self.events.copy(),
   'clone':self.projection(result,selected) if changed else None}
 def ctor88(self):
  selected,_=self.source('Load',2);resource=self.alloc(0xb4)
  #5A2831's stack offset is the ctor's original selected Creator argument.
  self.w32(h.STACK+0x24,selected);self.u.reg_write(UC_X86_REG_ESP,h.STACK)
  self.u.reg_write(UC_X86_REG_ESI,resource);self.u.reg_write(UC_X86_REG_EBX,0)
  self.u.emu_start(0x5a2831,0x5a285f,count=10000)
  assert self.u.reg_read(UC_X86_REG_EIP)==0x5a285f
  return self.projection(self.r32(resource+0x88),selected)
def main():
 cases=[];seams=set()
 for image,pc,kind,n in itertools.product(('original','v14'),h.PRECISION,('Base','Load'),(0,1,3)):
  m=Machine(image,pc);inputs,output=m.clone(kind,n);seams|=m.seam_entries
  cases.append({'image':image,'precision':pc,'input':inputs,'output':output})
 replacements=[];constructors=[]
 for image,pc in itertools.product(('original','v14'),h.PRECISION):
  m=Machine(image,pc);constructors.append({'image':image,'precision':pc,'output':m.ctor88()});seams|=m.seam_entries
  for mode,kind in itertools.product((1,2,4),('Base','Load')):
   m=Machine(image,pc);output=m.replacement(mode,kind);seams|=m.seam_entries
   replacements.append({'image':image,'precision':pc,'output':output})
 executed=[{'va':hex(a),'bytes':b}for a,b in sorted(h.VISITED.items())if a not in seams]
 static=[{'va':hex(a),'bytes':ORIGINAL.range(a,n).hex()}for a,n in ((0xa3214c,0x30),(0xa3220c,0x30),(0xb3ce40,12))]
 for pin in static:assert ORIGINAL.range(int(pin['va'],16),len(bytes.fromhex(pin['bytes'])))==h.V.range(int(pin['va'],16),len(bytes.fromhex(pin['bytes'])))
 doc={'schema':1,'scope':__doc__,'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,
  'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'seams_not_executed':[hex(a)for a in sorted(seams)],
  'executed_instructions':executed,'separate_static_metadata':static,'cases':cases,'replacement_cases':replacements,'constructor_cases':constructors}
 path=HERE/'native-creator-copy.json'
 if '--write' in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps({'clone_cases':len(cases),'replacements':len(replacements),'constructors':len(constructors),'executed_pins':len(executed),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
