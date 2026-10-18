"""Native5B0400 current-map hash constructor slice, full5AA890 base Resource
constructor and cached59F570 lookup/open-close protocol. Successful allocator,
StringManager and depot stream callbacks are named transport seams. No whole
ResourceManager init/worker/scheduler/depot-list constructor is certified.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE.parent/'skirmish-20261016-local-bootstrap'))
from native_bootstrap import BootstrapMachine,PE0,PE1,PC,proof_ranges
from unicorn.x86_const import *
class RequiredWrongType(Exception):pass
class Machine(BootstrapMachine):
 def __init__(self,image):
  super().__init__(image);self.w(0x9f01e8,0x7700900);self.w(self.vtable,0x7700b00)
 def hook(self,u,pc,size,user):
  if self.bootstrap_ready and pc==0x4252a0:raise RequiredWrongType()
  if self.bootstrap_ready and pc==0x401330:
   #Bypass inherited zero-fill Resize seam. Native copy-on-write must preserve
   #the complete path while859E50 replaces a slash in a shared CString.
   self.visited[pc]=size;return
  if self.bootstrap_ready and pc==0x7700b00:
   capacity=self.arg();assert self.arg(1)==2
   p=self.alloc(16+2*(capacity+1));self.w(p,self.manager);self.w(p+4,0);self.w(p+8,capacity);self.w(p+12,1)
   self.ret(8,p);return
  if self.bootstrap_ready and pc in(0x7700a00,0x7700a10,0x7700a20):
   if pc==0x7700a00:self.protocol.append('open_as_file');self.ret(4,self.file)
   elif pc==0x7700a10:self.protocol.append('close');self.ret()
   else:self.protocol.append('destroy');self.ret(4)
   return
  if self.bootstrap_ready and pc==0x7700900:
   p=self.arg();value=(self.r(p)+1)&0xffffffff;self.w(p,value);self.ret(4,value);return
  if self.bootstrap_ready and pc==0x404ea0:raise AssertionError('unexpected native error callback')
  super().hook(u,pc,size,user)
def main():
 rows=[];lookups=[];wrong=[];visited={}
 for image in('original','v14'):
  m=Machine(image)
  for pc,old in itertools.product(PC,(0,0xa5,0xff)):
   m.fpu(pc);manager=m.alloc(0x130);m.u.mem_write(manager,bytes([old])*0x130)
   m.u.reg_write(UC_X86_REG_ESP,m.stack);m.u.reg_write(UC_X86_REG_ESI,manager);m.u.reg_write(UC_X86_REG_EBX,0)
   m.u.emu_start(0x5b0479,0x5b04ea,count=100000)
   assert m.r(manager+0x68)==0 and m.r(manager+0x78)==0
   assert bytes(m.u.mem_read(m.r(manager+0x60),2009*4))==bytes(2009*4)
   assert bytes(m.u.mem_read(m.r(manager+0x70),317*4))==bytes(317*4)
   resource=m.alloc(0x3c);m.u.mem_write(resource,bytes([old])*0x3c);ids=m.alloc(4);m.cs(ids,'Maps/Actual.RMP')
   m.call(0x5aa890,[ids],this=resource,callee_pop=4)
   assert m.r(resource+0x14)==0 and m.r(resource+8)==0x10 and m.rs(resource+0x38)==[ord(c)for c in'Maps/Actual.RMP']
   rows.append({'image':image,'precision':pc,'old_byte':old,'counts':[m.r(manager+0x68),m.r(manager+0x78)],
    'bucket_counts':[m.r(manager+0x64),m.r(manager+0x74)],'refcount':m.r(resource+0x14),
    'base_flags':m.r(resource+8),'ids':m.rs(resource+0x38)})
   m.w(0xb36540,manager);m.w(resource,0xa44008)
   depot=m.alloc(4);vt=m.alloc(0x20);m.w(depot,vt);m.w(vt+0x1c,0x7700a00)
   m.w(m.r(resource+0x24),depot);m.w(resource+0x28,1)
   m.file=m.alloc(4);filevt=m.alloc(0x2c);m.w(m.file,filevt);m.w(filevt+0x28,0x7700a10);m.w(filevt+0x10,0x7700a20)
   hash=m.call(0x405030,[ids]);m.call(0x5ac1c0,[resource,hash%2009],this=manager+0x60,callee_pop=8)
   assert m.call(0x5ac160,[ids],this=manager+0x60,callee_pop=4)!=0,('inserted cache Find failed',hex(hash),m.r(manager+0x64),m.rs(resource+0x38))
   m.seams.pop(0x59f570,None)
   for initial in(0,0xffffffff):
    for key in('Maps/Actual.RMP','maps/actual.rmp','Maps\\Actual.RMP'):
     m.w(resource+0x14,initial);m.protocol=[];query=m.alloc(4);m.cs(query,key)
     print(image,pc,old,initial,key,flush=True)
     result=m.call(0x59f570,[query])
     assert result==resource and m.r(resource+0x14)==((initial+1)&0xffffffff)
     assert m.protocol==['open_as_file','close','destroy']
     lookups.append({'image':image,'precision':pc,'initial':initial,'key':key,'count':m.r(resource+0x14),'same_resource':True,'protocol':m.protocol})
   m.w(resource,0xa467e0);m.w(resource+0x14,0);m.protocol=[];query=m.alloc(4);m.cs(query,'Maps/Actual.RMP')
   try:m.call(0x59f570,[query]);raise AssertionError('wrong native class returned success')
   except RequiredWrongType:pass
   assert m.r(resource+0x14)==1 and m.protocol==[]
   wrong.append({'image':image,'precision':pc,'count_before':0,'count_after':1,'boundary':0x4252a0,'probe_not_opened':True})
   visited.update(m.visited)
 out={'passed':True,'scope':__doc__,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
  'original_sha256':hashlib.sha256(PE0.b).hexdigest(),'v14_sha256':hashlib.sha256(PE1.b).hexdigest(),
  'constructors':rows,'cached_lookups':lookups,'wrong_class':wrong,'executed_windows':proof_ranges(visited)}
 (HERE/'native-cache.json').write_text(json.dumps(out,indent=2)+'\n');print('passed',len(rows),'native cache/base constructor rows')
if __name__=='__main__':main()
