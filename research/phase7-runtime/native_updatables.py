"""Execute original table construction, fresh Updatable registration and removal."""
from pathlib import Path
import sys,struct,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase6-queues'))
import native_queues as q
from unicorn.x86_const import *
class Machine(q.Machine):
 def __init__(self,count):
  super().__init__({'counts':[1]*5,'seed':123,'precision':53,'clock':{'begin_bits':0,'end_bits':0,'step_bits':q.bits(.0625)}})
  self.data=b'';self.at=0;self.failure=False
  vt=self.alloc(32);self.write(vt,0x23f0100);self.write(vt+12,0x23f0110)
  self.write(self.world+0x3c,vt);self.table=self.alloc(12)
  self.call(0x50b470,self.table,[count,self.world+0x284]);assert self.read(self.world+0x284)==self.table
 def hook(self,u,pc,size,arg):
  if pc==0x23f0100:
   dest,n=self.arg(),self.arg(1);out=self.data[self.at:self.at+n];assert len(out)==n
   u.mem_write(dest,out);self.at+=n;self.ret(8);return
  if pc==0x23f0110:self.ret(value=0);return
  if pc==0x50bd42:
   self.failure=True;u.emu_stop();return
  super().hook(u,pc,size,arg)
 def load(self,version,id,owner):
  self.at=0;self.failure=False;self.data=struct.pack('<I',version)+(struct.pack('<i',id) if version else b'')
  self.write(self.stack,self.stop);self.write(self.stack+4,self.world)
  u=self.u;u.reg_write(UC_X86_REG_ESP,self.stack);u.reg_write(UC_X86_REG_ECX,self.owner(owner))
  u.emu_start(0x50bcd0,self.stop,count=20000)
  assert self.at==len(self.data);return not self.failure
 def slots(self):
  n,p=self.read(self.table),self.read(self.table+4)
  return [self.names.get(self.read(p+4*i)) for i in range(n)]
def main():
 cases=[]
 for count in [0,1,2,7,16,275]:
  for variant in range(3):
   m=Machine(count);ops=[]
   def load(version,id,owner):
    ok=m.load(version,id,owner);ops.append({'operation':'load','version':version,'id':id,'owner':owner,'ok':ok,'slots':m.slots()})
   def remove(owner):
    ok=bool(m.call(0x50b120,m.table,[m.owner(owner)])&255);ops.append({'operation':'remove','owner':owner,'ok':ok,'slots':m.slots()})
   selected=list(dict.fromkeys([0,count//2,count-1])) if count else []
   if variant==0:
    scenario='first-middle-last'
    for version,id in [(0,0),(1,-1),(1,-9)]:load(version,id,10)
    for index,id in enumerate(selected):load(1,id,10+index)
    if count:load(1,selected[0],99)
    for owner in list(range(10,10+len(selected)))+[99]:remove(owner)
   elif variant==1:
    scenario='reverse-order-remove-and-rebind'
    for index,id in enumerate(reversed(selected)):load(1,id,20+index)
    if count:
     remove(20);load(1,selected[-1],29);load(1,selected[0],99)
    else:
     #Strictly greater than count reaches native diagnostic without executing
     #the separate id==count out-of-allocation bug.
     load(1,1,29)
    load(0,0,29);load(1,-1,29)
    for owner in [29]+list(reversed(range(20,20+len(selected)))):remove(owner)
   else:
    scenario='shared-pointer-first-match-removal'
    remove(30)
    rotated=selected[1:]+selected[:1]
    for id in rotated:load(1,id,30)
    load(1,-2147483648,30);load(0,0,30)
    for _ in range(len(selected)+1):remove(30)
   cases.append({'count':count,'scenario':scenario,'ops':ops})
 out={'passed':True,'cases':cases,'original_sha256':hashlib.sha256(q.ORIGINAL.b).hexdigest(),'v14_sha256':hashlib.sha256(q.ACCEPTED.b).hexdigest(),'visited':[{'va':hex(a),'bytes':b}for a,b in sorted(q.VISITED.items())],
 'scope':'Original50B470,8589B0,50BCD0,50B250 empty-owner branch and50B120 execute. Stream I/O supplies fixturebytes; duplicateerror stops at original error boundary before formatting/throw. No liveevents at entry, as on fresh load constructors. Malformedid==count native out-of-allocation access is intentionally rejected rather than reproduced in Rust.'}
 p=HERE/'native-updatables.json';p.write_text(json.dumps(out,indent=2)+'\n');print('passed',len(cases),hashlib.sha256(p.read_bytes()).hexdigest())
if __name__=='__main__':main()
