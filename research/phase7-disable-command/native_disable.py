"""Original52D910 through actual52A810 mode2 state replacement.
State virtual/entry/scheduler and final actor4EEBE0 are required fixture leaves;
this validates command routing and mutation order, not a fake facing/world.
"""
from pathlib import Path
import copy,hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase7-mind'))
import native_lifecycle as life
from unicorn.x86_const import *
old_sp,old_sid=life.sp,life.sid
life.sp=lambda i:life.mp(1)+0x68 if i==4 else old_sp(i)
life.sid=lambda p:4 if p==life.mp(1)+0x68 else old_sid(p)
bits=lambda f:struct.unpack('<I',struct.pack('<f',f))[0]
class Machine(life.LifecycleMachine):
 def __init__(self,c):
  super().__init__(c);self.actors={i:0x4600000+(i-100)*0x1000 for i in range(101,105)};self.ids={p:i for i,p in self.actors.items()}
  for key,m in c['arena']['minds'].items():self.w(life.mp(int(key))+4,self.actors[m['actor']])
  for i,a in c['actors'].items():
   actor=self.actors[int(i)];comp=actor+0x300;self.w(actor+0x80,comp if a['element'] else 0);self.w(comp+0x10,self.actors.get(a['leader'],0));self.w(actor+0x2c,a['facing'][0]);self.w(actor+0x30,a['facing'][1])
 def snapshot(self):
  out=super().snapshot()
  if hasattr(self,'actors'):
   for key,m in out['minds'].items():m['actor']=self.ids[self.r(life.mp(int(key))+4)]
  return out
 def event(self,name,ident,extra=[]):
  super().event(name,ident,extra)
  if name=='changed' and self.c['change_actor']:self.w(life.mp(1)+4,self.actors[102])
 def hook(self,u,pc,size,arg):
  if pc==0x4eebe0:
   self.visited[pc]=bytes(u.mem_read(pc,size));p=self.r(u.reg_read(UC_X86_REG_ESP)+4);actor=self.ids[u.reg_read(UC_X86_REG_ECX)];self.event('face',actor,[self.r(p),self.r(p+4)]);self.ret(pop=4);return
  super().hook(u,pc,size,arg)
 def run(self):
  self.reset(0);self.u.reg_write(UC_X86_REG_ECX,life.mp(1));self.w(self.STACK+4,0)
  regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
  for i,r in enumerate(regs):self.u.reg_write(r,0x11111111*(i+1))
  self.u.emu_start(0x52d910,self.STOP,count=100000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+8
  assert [self.u.reg_read(r)for r in regs]==[0x11111111*(i+1)for i in range(4)];assert not self.c['mutations']
  return{'accepted':bool(self.u.reg_read(UC_X86_REG_EAX)&255),'arena':self.snapshot(),'events':self.events}
def cases():
 rows=[]
 for stack in [[],[None],[1],[1,2,3]]:
  for allow in [False,True]:
   for branch in ['absent','empty','leader']:
    for change in [False,True]:
     c=copy.deepcopy(life.cases()[0]);c.update(name=f'{stack}_{allow}_{branch}_{change}',operation='activate',mode=2,can_activate=allow,change_actor=change);c['arena']['minds']['1']['stack']=stack
     c['actors']={str(i):{'element':i in [101,102],'leader':103 if i==101 else(104 if i==102 else None),'facing':[bits((i-100)*.317),bits((100-i)*.717)]}for i in range(101,105)}
     if branch=='absent':c['actors']['101']['element']=False
     if branch=='empty':c['actors']['101']['leader']=None
     rows.append(c)
 base=copy.deepcopy(rows[-1])
 for event,changes in [('can',{'states':{'4':{'parent':2}}}),('leave',{'stacks':{'1':[None,2]}}),('schedule',{'stacks':{'1':[5]}}),('enter',{'stacks':{'1':[2]}})]:
  c=copy.deepcopy(base);c['name']='mutation_'+event;c['mutations']=[{'event':event,**changes}];rows.append(c)
 return rows
def main():
 rows=[];visited={}
 for c in cases():m=Machine(c);rows.append({'input':c,'output':m.run()});visited.update(m.visited)
 for a,raw in visited.items():assert raw==life.original.range(a,len(raw))==life.modified.range(a,len(raw))
 doc={'schema':1,'passed':True,'original_sha256':life.sha(life.original.b),'accepted_sha256':life.sha(life.modified.b),'producer_sha256':life.sha(Path(__file__).read_bytes()),'method':__doc__,'cases':rows,'instructions':[{'address':hex(a),'bytes':raw.hex()}for a,raw in sorted(visited.items())]}
 (HERE/'native-disable-fixtures.json').write_text(json.dumps(doc,indent=2)+'\n',encoding='utf8');print(json.dumps({'passed':True,'cases':len(rows)}))
if __name__=='__main__':main()
