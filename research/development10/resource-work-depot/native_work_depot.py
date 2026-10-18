"""Directory59E060 constructor,5ABE50 work selection and fresh Film
5AAAE0->5AA660->5AA260->5ACF60 registry transfer. Successful allocators,
CString pool and lock leaves reuse the existing named transports. Typed
fixture managers use small valid hash capacities, not observed startup
budgets or a full ResourceManager initializer. No filesystem IO is claimed.
Class59CB00/59C360/59C040 execute against an explicit empty class cache with
zero-byte budgets and positive weights; no eviction is reached. RegistryID17
is a named opaque identity transport.4094F0 converts the actual CRT literal
KKC_FilmResource; its type-manager registration is not executed. CRT%s
formatting and current Int-variable lookup are explicit laboratory leaves.
"""
from pathlib import Path
import hashlib,itertools,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/development10/world-ready-7757f0'))
from native_application_film_string import Machine as Base
from native_application_film_string import base
from unicorn.x86_const import UC_X86_REG_ECX,UC_X86_REG_EIP
from native_bootstrap import LOCK_VT
class Machine(Base):
 def __init__(self,image):
  self.watch=False;super().__init__(image);self.w(self.vtable,0x7700b00);self.u.mem_write(0x7700b00,b'\xc3')
 def hook(self,u,pc,size,user):
  if self.watch and pc==0x401330:
   self.actual[pc]=size;return
  if self.watch and pc==0x7700b00:
   capacity=self.arg();assert self.arg(1)==2
   p=self.alloc(16+2*(capacity+1));self.w(p,self.manager);self.w(p+4,0);self.w(p+8,capacity);self.w(p+12,1)
   self.ret(8,p);return
  if self.watch and pc==0x405d60 and self.text(self.arg(1))=='ResourceCache%sRAMWeight':
   self.cs(self.arg(),'ResourceCache'+self.text(self.arg(2))+'RAMWeight');self.ret(value=self.arg());return
  if self.watch and pc==0x405370 and getattr(self,'cache_probe',False):
   name=self.text(self.r(self.arg()));assert name=='ResourceCacheFilmRAMWeight',repr(name)
   self.lookups+=1;self.ret(4,self.weightptr if self.weight is not None else 0);return
  if self.watch and pc==0x404ea0:
   raise AssertionError(('unexpected native assertion',hex(pc)))
  super().hook(u,pc,size,user)
 def directory(self,path,writable):
  p=self.alloc(0x24);self.u.mem_write(p,b'\xa5'*0x24)
  a=self.alloc(4);b=self.alloc(4);c=self.alloc(4)
  self.cs(a,path);self.cs(b,path);self.cs(c,'')
  self.actual.clear();self.call(0x59e060,[a,b,c,writable&0xffffffff,0],this=p,callee_pop=20)
  return p,dict(path4=self.rs(p+4),path8=self.rs(p+8),prefix=self.rs(p+12),
   writable=self.r(p+16),priority=self.r(p+20),dirty=self.rb(p+24),flags=self.r(p+28))
 def make_manager(self,depots):
  mgr=self.alloc(0x160);v=self.alloc(max(4,4*len(depots)))
  for i,p in enumerate(depots):self.w(v+4*i,p)
  self.w(mgr+0x84,v);self.w(mgr+0x88,len(depots));self.w(mgr+0x90,0xabcdef01)
  self.w(mgr+0x20,LOCK_VT);self.w(0xb36540,mgr)
  for off,cap in((0x60,11),(0x70,7)):
   self.w(mgr+off,self.alloc(cap*4));self.w(mgr+off+4,cap);self.w(mgr+off+8,0);self.b(mgr+off+12,0)
  self.w(0xb43b70,0)
  return mgr
def produce():
 constructors=[];selection=[];location=[];classes=[];pins={}
 configs=[[],[0],[1],[-1],[0,1],[1,0],[1,2],[2,1],[1,1],[0,-1,2,0]]
 for image in('original','v14'):
  m=Machine(image)
  for pc in base.PC:
   m.fpu(pc);m.watch=True
   for writable in(-1,0,1):
    p,result=m.directory('Userdata/',writable);pins.update(m.actual)
    assert result['path4']==result['path8']==list('Userdata/'.encode('utf-16-le')[::2])
    assert result['prefix']==[]and result['priority']==0 and result['dirty']==1 and result['flags']==0
    constructors.append(dict(image=image,precision=pc,writable=writable,result=result))
   for cfg in configs:
    depots=[m.directory('Userdata/',v)[0]for v in cfg];mgr=m.make_manager(depots)
    m.actual.clear();m.call(0x5abe50,this=mgr);pins.update(m.actual)
    expected=next((i for i in reversed(range(len(cfg)))if cfg[i]>0),None)
    actual=depots.index(m.r(mgr+0x90))if m.r(mgr+0x90)in depots else None
    assert actual==expected
    selection.append(dict(image=image,precision=pc,writable=cfg,work=actual,priorities=[m.r(p+20)for p in depots]))
   work=m.directory('Userdata/',1)[0];mgr=m.make_manager([work]);m.w(mgr+0x90,work)
   for old_flags in(0x18,0x10,0x8):
    resource=m.alloc(0xb4);m.w(resource,0xa44130);m.w(resource+8,old_flags)
    m.w(resource+0x24,m.alloc(8));m.w(resource+0x28,0);m.w(resource+0x2c,2);m.cs(resource+0x38,'films/test.RFM')
    #Reset both name hashes between valid fresh-allocation scenarios.
    for off,cap in((0x60,11),(0x70,7)):
     m.w(mgr+off,m.alloc(cap*4));m.w(mgr+off+8,0)
    m.actual.clear();m.call(0x5aaae0,[work],this=resource,callee_pop=4);pins.update(m.actual)
    flags=m.r(resource+8);assert flags==(old_flags&~0x16)
    active=m.r(mgr+0x68);inactive=m.r(mgr+0x78)
    assert(active,inactive)==((1,0)if old_flags&0x10 else(0,0))
    location.append(dict(image=image,precision=pc,flags_before=old_flags,flags_after=flags,
     requests=m.r(resource+4),depots=m.r(resource+0x28),work_is_tail=m.r(m.r(resource+0x24))==work,
     active_count=active,inactive_count=inactive))
   #4094F0 is the native ASCII->CString child of the real CRT9D0370 class
   #registration.17 is an explicit opaque registry-ID transport, not an
   #observed process class number or a completed9D0370 registration claim.
   m.actual.clear();m.call(0x4094f0,[0xa44d54],this=0xb3d7f4,callee_pop=4);pins.update(m.actual);m.w(0xb3d7f0,17)
   assert m.rs(0xb3d7f4)==[ord(c)for c in'KKC_FilmResource']
   for weight in(None,3,11):
    cache=m.alloc(0x48);m.w(cache+0x20,m.alloc(23*4));m.w(cache+0x24,23);m.b(cache+0x2c,1)
    m.weight=weight;m.weightptr=m.alloc(4);iv=m.alloc(4);m.w(iv,weight or 0);m.w(m.weightptr,iv)
    resource=m.alloc(0xb4);m.w(resource,0xa44130);m.cache_probe=True;m.lookups=0
    m.actual.clear()
    try:value=m.call(0x59cb00,[resource],this=cache,callee_pop=4)
    except Exception as e:raise AssertionError((image,pc,weight,hex(m.u.reg_read(UC_X86_REG_EIP)),m.lookups,[hex(v)for v in list(m.actual)[-24:]]))from e
    pins.update(m.actual)
    first_lookups=m.lookups;actual=weight if weight is not None else 10
    assert m.r(value+0x14)==actual and m.r(value+0x24)==10
    before=[m.r(value+4*i)for i in range(13)]
    m.weight=999;m.actual.clear();again=m.call(0x59cb00,[resource],this=cache,callee_pop=4);pins.update(m.actual)
    assert again==value and m.lookups==first_lookups and[m.r(value+4*i)for i in range(13)]==before
    classes.append(dict(image=image,precision=pc,weight=weight,name=m.rs(value),class_id=m.r(value+4),
     ram_weight=m.r(value+0x14),vram_weight=m.r(value+0x24),ram_total=m.r(cache+0x38),vram_total=m.r(cache+0x44),
     count=m.r(cache+0x28),lookups=first_lookups,cached_lookups=m.lookups-first_lookups))
    m.cache_probe=False
 return dict(passed=True,scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
  original_sha256=hashlib.sha256(base.PE0.b).hexdigest(),v14_sha256=hashlib.sha256(base.PE1.b).hexdigest(),
  constructors=constructors,selection=selection,location=location,classes=classes,executed_windows=base.proof_ranges(pins))
if __name__=='__main__':
 result=produce();p=HERE/'native-work-depot.json'
 if'--write'in sys.argv:p.write_text(json.dumps(result,indent=2)+'\n')
 else:assert result==json.loads(p.read_text())
 print('passed',len(result['constructors']),len(result['selection']),len(result['location']),len(result['classes']))
