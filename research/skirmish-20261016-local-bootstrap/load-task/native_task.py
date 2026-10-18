"""Full59E6F0/5B17F0 task constructor and76BE80 publication/74DAF0.
5AED10 resource-manager enqueue is a named transport seam inspecting the actual
published task, not a fabricated Run/completion. Status2/3 leaf stores are
separately executed on the actual allocation; they do not prove a loaded map.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parent))
from native_bootstrap import BootstrapMachine,PE0,PE1,PC,proof_ranges,NODE,NETNODE,LOCK_VT
from unicorn.x86_const import *
class Machine(BootstrapMachine):
 def __init__(self,image):
  super().__init__(image);self.w(0x9f01e8,0x7700c00);self.w(0x9f01ec,0x7700c10);self.enqueues=[]
 def hook(self,u,pc,size,user):
  if self.bootstrap_ready and pc in(0x7700c00,0x7700c10):
   address=self.arg();value=(self.r(address)+(1 if pc==0x7700c00 else -1))&0xffffffff
   self.w(address,value);self.ret(4,value);return
  if self.bootstrap_ready and pc==0x5aed10:
   task=self.arg();assert task==self.r(self.S+0x124)
   self.enqueues.append({'task_is_session124':True,'references4':self.r(task+4),'status14':self.r(task+0x14),
    'type8':self.r(task+8),'creator_alias1c':self.r(task+0x1c)==self.creator,'kind20':self.r(task+0x20)})
   self.ret(4);return
  super().hook(u,pc,size,user)
def main():
 rows=[];strings=[];visited={}
 for image in('original','v14'):
  m=Machine(image)
  for pc,kind,old in itertools.product(PC,(0,1,2),(0xa5,0xff)):
   m.fpu(pc);m.begin(7,list(range(16)),0);m.w(m.S+0x124,0);m.w(NETNODE+8,LOCK_VT)
   launch=m.alloc(0x34);m.u.mem_write(launch,bytes([old])*0x34)
   m.u.reg_write(UC_X86_REG_ESI,launch);m.u.reg_write(UC_X86_REG_ESP,m.stack)
   m.u.emu_start(0x76e6c5,0x76e6e1,count=100000)
   assert m.rs(launch+0x30)==[]
   strings.append({'image':image,'precision':pc,'old_byte':old,'launch_text30':m.rs(launch+0x30)})
   m.creator=m.alloc(0x54);source=m.alloc(0xc0);m.w(m.creator+0x50,source)
   p=m.alloc(0x28);m.u.mem_write(p,bytes([old])*0x28)
   m.call(0x59e6f0,[m.creator,kind],this=p,callee_pop=8)
   fields=[m.r(p+off)for off in(4,8,0xc,0x10,0x14,0x18,0x20)]
   assert fields==[0,2,0,0,0,0,kind] and m.r(p+0x1c)==m.creator
   assert m.r(p+0x24)==(source if kind==1 else 0)
   m.w(NODE+0x34,0xdeadbeef);m.b(NODE+0x8c,old)
   link=m.alloc(8);m.w(link,NODE);m.w(link+4,0);m.w(NODE+0x2ac,link);m.enqueues=[]
   m.call(0x76be80,[m.creator,kind],this=m.S,callee_pop=8)
   task=m.r(m.S+0x124);assert task and m.r(task+4)==1
   assert m.rb(NODE+0x8c)==1 and m.r(NODE+0x34)==1
   assert len(m.enqueues)==1 and m.enqueues[0]['status14']==0
   m.b(NODE+0x6c,old);m.call(0x74dbc0,[0],this=NODE,callee_pop=4)
   assert m.rb(NODE+0x6c)==0
   statuses=[]
   for status in(2,3):
    m.call(0x5b18c0,[status],this=task,callee_pop=4);statuses.append(m.r(task+0x14));assert m.r(task+0x14)==status
   rows.append({'image':image,'precision':pc,'kind20':kind,'old_byte':old,'constructor_fields':fields,
    'constructor_source24_is_selected':kind==1,'enqueue':m.enqueues[0],
    'after_begin_task':{'node8c':m.rb(NODE+0x8c),'peer34':m.r(NODE+0x34)},'after_launch_reset_version6c':m.rb(NODE+0x6c),'separate_status_stores':statuses})
   visited.update(m.visited)
 out={'passed':True,'scope':__doc__,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
  'original_sha256':hashlib.sha256(PE0.b).hexdigest(),'v14_sha256':hashlib.sha256(PE1.b).hexdigest(),
  'cases':rows,'launch_string_constructor':strings,'executed_windows':proof_ranges(visited)}
 (HERE/'native-task.json').write_text(json.dumps(out,indent=2)+'\n');print('passed',len(rows),'native task constructor/publication rows')
if __name__=='__main__':main()
