"""Original full7A0AA0 SAI ctor and fresh-world5A4280 load prefix.

7A0AA0,857D60,857910,857840,79E130,85D8A0,7A5BF0,856DF0 and4B2E60
execute original instructions. Leaves: allocator/CString storage, OS event
creation,59ABA0 supplied actual-clock input, typed GoalPriorityCSV Var lookup.
The original Sim ctor's76F2C5..76F2D7 field stores project its null world slot;
the complete Sim/thread/network constructor is explicitly NOT executed.
Then59E6F0/5B17F0 construct the task;5A4280 executes76AB80 empty-world branch,
4F7910 diagnostic clear and the chosen5A3* wrapper until5A2B90 MapResource::Load.
Locks are recorded successful OS synchronization leaves; no task completion,
GWorld parsing, StartMission, activation or old-world teardown is executed.
"""
from pathlib import Path
import sys,struct,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1];GAME=ROOT.parent
sys.path.insert(0,str(GAME/'reconstruction/tools/python-packages'))
sys.path.insert(0,str(ROOT/'research/phase6-mapheader'))
from native_header import Machine as Base,P,V
from unicorn import UC_HOOK_MEM_READ,UC_HOOK_MEM_WRITE
from unicorn.x86_const import *
sha=lambda b:hashlib.sha256(b).hexdigest()
class Machine(Base):
 def __init__(self,clock,pc):
  super().__init__(b'');self.clock=clock;self.visited={};self.events=[];self.map_mode=None;self.phase='constructor'
  self.u.reg_write(UC_X86_REG_FPCW,{24:0x7f,53:0x27f,64:0x37f}[pc])
  self.u.hook_add(UC_HOOK_MEM_READ|UC_HOOK_MEM_WRITE,self.null_guard,begin=0,end=0xfff)
  self.waiter_manager=self.alloc(0x20);self.write(0xb43b5c,self.waiter_manager);self.write(0xb364d4,0)
  self.write(0xb36408,0x2e01000);self.u.mem_write(0x2e01000,b'\xc3')
  self.variable=self.alloc(0x40);self.write(self.variable+0x24,4);self.write(self.variable+0x10,self.empty)
  self.cell=self.alloc(4);self.write(self.cell,self.variable)
  self.write(0xb40d78,0);self.u.mem_write(0xb40d70,b'\0')
  context=self.alloc(0x100);table=self.alloc(0x20);self.write(context+0x24,table);self.write(table+4,0x2e01010);self.write(table+8,0x2e01020);self.write(0xb36524,context)
  self.u.mem_write(0x2e01010,b'\xc3');self.u.mem_write(0x2e01020,b'\xc3')
  self.write(0xb3cc88,self.empty);self.write(0xb3cc8c,0);self.write(0xb3cc90,0)
 def null_guard(self,u,access,address,size,value,data):
  assert bytes(u.mem_read(u.reg_read(UC_X86_REG_EIP),1))==b'\x64',f'non-FS null at{u.reg_read(UC_X86_REG_EIP):x}'
 def text(self,p):
  b=bytearray()
  while (unit:=bytes(self.u.mem_read(p,2)))!=b'\0\0':b.extend(unit);p+=2;assert len(b)<4096
  return b.decode('utf-16-le')
 def call(self,address,this,args=()):
  self.write(self.stack,self.stop)
  for i,arg in enumerate(args):self.write(self.stack+4+4*i,arg)
  self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.reg_write(UC_X86_REG_ECX,this)
  self.u.emu_start(address,self.stop,count=1000000)
  assert self.map_mode is not None or self.u.reg_read(UC_X86_REG_EIP)==self.stop,hex(self.u.reg_read(UC_X86_REG_EIP))
 def hook(self,u,pc,size,data):
  if P.base<=pc<P.base+P.image_size:
   raw=bytes(u.mem_read(pc,size));assert raw==P.range(pc,size)==V.range(pc,size);self.visited[pc]=raw
  if pc==0x2e01000:
   assert [self.arg(i) for i in range(4)]==[0,0,0,0]
   self.events.append('create_auto_reset_nonsignaled_event');self.ret(16,0x12340000);return
  if pc in (0x2e01010,0x2e01020):self.events.append('lock' if pc==0x2e01010 else 'unlock');self.ret();return
  if pc==0x59aba0:self.events.append('application_clock');self.ret(value=self.clock);return
  if pc==0x405370:
   assert self.text(self.read(self.arg()))=='GoalPriorityCSV';self.events.append('GoalPriorityCSV_typed_lookup');self.ret(4,self.cell);return
  if pc==0x4b2c60:raise AssertionError('empty source unexpectedly opened debug file')
  if pc==0x404f60:self.ret();return
  if pc in [0x76ab80,0x4f7910,0x5a3e20,0x5a3eb0,0x5a3f20]:self.events.append(f'{pc:08x}')
  if pc==0x4f8690:raise AssertionError('fresh constructor null-world branch must not Uninit')
  if pc==0x5a28b0:self.ret(value=self.map_resource);return
  if pc==0x5a2b90:
   self.map_mode=self.arg(1);self.events.append('map_resource_load');u.emu_stop();return
  super().hook(u,pc,size,data)
 def run(self,mode):
  sai=self.alloc(0xa0);self.u.mem_write(sai,b'\xa5'*0xa0);self.call(0x7a0aa0,sai)
  assert self.read(0xb36528)==sai and self.read(0xb40d4c)==sai and self.read(0xb40d48)==0xb36528
  assert self.u.mem_read(sai+0x20,1)==b'\0'
  seed=self.read(self.read(sai+0x2c));expected=((self.clock<<16)|self.clock)&0xffffffff;expected^=0x8e4b12a5
  assert seed==(expected or 0x8e4b12a5)
  ctor=dict(present=True,active=False,seed=seed,waiter_count=self.read(self.waiter_manager+8),signal_handle=self.read(sai+0x18),words={f'{o:x}':self.read(sai+o) for o in [0x28,0x30,0x34,0x38,0x3c,0x40,0x44,0x4c,0x50,0x54,0x58,0x64,0x68,0x6c,0x70,0x78,0x7c,0x80,0x84,0x88,0x8c,0x94,0x98]},events=list(self.events))
  sim=self.alloc(0x50);self.u.mem_write(sim,b'\xa5'*0x50);self.write(0xb36554,sim)
  self.u.reg_write(UC_X86_REG_ESI,sim);self.u.reg_write(UC_X86_REG_EBX,0);self.u.reg_write(UC_X86_REG_ESP,self.stack)
  self.u.emu_start(0x76f2c5,0x76f2d7,count=1000);assert self.read(sim+0x3c)==0
  source=self.alloc(0x60);self.map_resource=self.alloc(0x90);self.write(self.map_resource+0x38,self.empty);self.write(source+0x50,self.map_resource)
  task=self.alloc(0x30);self.call(0x59e6f0,task,[source,mode]);self.events=[];self.phase='task';self.call(0x5a4280,task)
  assert self.map_mode=={1:2,2:4}.get(mode,1)
  assert self.u.mem_read(sai+0x20,1)==b'\0' and self.read(0xb36528)==sai and self.read(sim+0x3c)==0
  assert self.events[:4]==['lock','0076ab80','lock','unlock'],self.events
  return dict(clock=self.clock,task_mode=mode,constructor=ctor,load_mode=self.map_mode,events=self.events,observer_present=True,observer_active=False,old_world_still_null=True,task_state=self.read(task+0x14))
def main():
 rows=[];visited={}
 for pc in [24,53,64]:
  for clock in [0,1,65535,65536,0x8e4b12a5,0xffffffff]:
   for mode in [0,1,2]:
    m=Machine(clock,pc);rows.append(dict(precision=pc,**m.run(mode)));visited.update(m.visited)
 out=dict(scope=__doc__,original_sha256=sha(P.b),v14_sha256=sha(V.b),producer_sha256=sha(Path(__file__).read_bytes()),cases=rows,passed=len(rows),instructions=[dict(address=f'{a:08x}',bytes=b.hex()) for a,b in sorted(visited.items())])
 (HERE/'native-lifecycle.json').write_text(json.dumps(out,indent=2)+'\n');print('passed',len(rows))
if __name__=='__main__':main()
