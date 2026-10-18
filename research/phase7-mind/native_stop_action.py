"""Original52D660 plus actual529C30/5767C0/4DB0F0; cancel/default are world seams."""
from pathlib import Path
import copy,hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(HERE));from native_lifecycle import LifecycleMachine,cases,mp,sp
from native_constructors import original,modified
from unicorn.x86_const import *
ap=lambda n:0 if n is None else 0x4500000+(n-100)*0x100
cp=lambda n:0 if n is None else 0x4600000+(n-200)*0x100
aid=lambda n:None if n==0 else (n-0x4500000)//0x100+100
cid=lambda n:None if n==0 else (n-0x4600000)//0x100+200
sha=lambda b:hashlib.sha256(b).hexdigest()
class StopMachine(LifecycleMachine):
 def __init__(self,c):
  super().__init__(c);self.write_world()
 def write_world(self):
  for key,m in self.c['arena']['minds'].items():self.w(mp(int(key))+4,ap(m['actor']))
  for key,a in self.c['actors'].items():self.w(ap(int(key))+0x6c,cp(a['mover']));self.w(ap(int(key))+0xe4,cp(a['active_component']))
  for key,owner in self.c['owners'].items():self.w(cp(int(key))+4,ap(owner))
 def mutate(self,name):
  changes=self.c['mutations']
  if changes and changes[0]['event']==name:
   m=changes.pop(0)
   for key,a in m.get('actors',{}).items():self.c['actors'][key].update(a)
   for key,s in m.get('states',{}).items():self.c['arena']['states'][key].update(s)
   self.write_arena();self.write_world()
 def hook(self,u,pc,size,data):
  if original.base<=pc<original.base+original.image_size:self.visited[pc]=bytes(u.mem_read(pc,size))
  cx=u.reg_read(UC_X86_REG_ECX)
  if pc==0x5767c0:self.events.append(['stop',cid(cx),False,False])
  elif pc==0x50b250:self.events.append(['cancel',cid(cx-8)]);self.mutate('cancel');self.ret()
  elif pc==0x4cfc80:self.events.append(['default',aid(cx)]);self.mutate('default');self.ret()
  else:super().hook(u,pc,size,data)
 def run_stop(self):
  self.reset(0);self.u.reg_write(UC_X86_REG_ECX,sp(4));self.u.emu_start(0x52d660,self.STOP,count=3000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+4
  assert not self.c['mutations'];return {'events':self.events,'arena':self.snapshot(),'actors':self.c['actors']}
def main():
 base={'arena':cases()[0]['arena'],'actors':{'101':{'mover':201,'active_component':201},'102':{'mover':202,'active_component':None}},'owners':{'201':101,'202':102},'mutations':[]}
 specs=[]
 def add(name,fn):c=copy.deepcopy(base);c['name']=name;fn(c);specs.append(c)
 for mover in [None,201]:
  for active in [None,201,202]:add(f'mover{mover}_active{active}',lambda c,m=mover,a=active:c['actors']['101'].update(mover=m,active_component=a))
 add('cancel_removes_mover',lambda c:c.update(mutations=[{'event':'cancel','actors':{'101':{'mover':None}}}]))
 add('cancel_deactivates_mover',lambda c:c.update(mutations=[{'event':'cancel','actors':{'101':{'active_component':None}}}]))
 add('cancel_switches_mover',lambda c:c.update(mutations=[{'event':'cancel','actors':{'101':{'mover':202}}}]))
 add('cancel_changes_mind_parent',lambda c:c.update(mutations=[{'event':'cancel','states':{'4':{'parent':2}}}]))
 add('mover_owner_differs',lambda c:c['owners'].update({'201':102}))
 rows=[];visited={}
 for c in specs:
  m=StopMachine(c);rows.append({'input':c,'output':m.run_stop()});visited.update(m.visited)
 for a,b in visited.items():assert b==original.range(a,len(b))==modified.range(a,len(b))
 doc={'schema':1,'original_sha256':sha(original.b),'v14_sha256':sha(modified.b),'producer_sha256':sha(Path(__file__).read_bytes()),'method':__doc__,'instructions':[{'va':hex(a),'bytes':b.hex()}for a,b in sorted(visited.items())],'cases':rows}
 p=HERE/'native-stop-action.json';p.write_text(json.dumps(doc,indent=2),encoding='utf-8');print(json.dumps({'cases':len(rows),'sha256':sha(p.read_bytes())}))
if __name__=='__main__':main()
