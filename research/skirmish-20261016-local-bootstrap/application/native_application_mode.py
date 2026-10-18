"""Native GameAppState ctor651810, getter40D960 and setter651840.
794B70/79A340 post effects are explicit successful observational seams;
manager pointers/fields are independent probes, not a native app startup.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parent))
from native_bootstrap import BootstrapMachine,PE0,PE1,PC,proof_ranges
from unicorn.x86_const import *
class AppMachine(BootstrapMachine):
 def hook(self,u,pc,size,user):
  if getattr(self,'app_ready',False) and pc in (0x794b70,0x79a340):
   self.app_events.append({'call':f'{pc:08X}','mode_before':self.r(self.state+0xc),'changed_before':self.rb(self.state+8)})
   self.ret();return
  super().hook(u,pc,size,user)
def main():
 cases=[];ctors=[];visited={}
 for image in ('original','v14'):
  m=AppMachine(image);m.app_ready=True;m.app_events=[]
  s=m.alloc(0x14);m.state=s;m.u.mem_write(s,b'\xa5'*0x14)
  m.call(0x651810,this=s)
  app=m.alloc(0x114);m.w(app+0xfc,s)
  result=m.call(0x40d960,this=app)
  assert result==s
  ctor={'mode':m.r(s+0xc),'changed':m.rb(s+8),'flag10':m.rb(s+0x10),'flag11':m.rb(s+0x11),'base4':m.r(s+4),'getter_identity':result==s}
  assert ctor=={'mode':1,'changed':0,'flag10':0,'flag11':0,'base4':0,'getter_identity':True}
  ctors.append({'image':image,**ctor})
  mgr_a=m.alloc(8);mgr_b=m.alloc(0x18)
  for p in PC:
   for a in (None,0,1):
    for b in (None,0,1):
     for mode in (0,1,2,4,0xffffffff):
      m.fpu(p);m.w(0xb40d00,0 if a is None else mgr_a);m.w(mgr_a+4,a or 0)
      m.w(0xb40d28,0 if b is None else mgr_b);m.w(mgr_b+0x14,b or 0)
      m.b(s+8,0);m.w(s+0xc,0xdeadbeef);m.app_events=[]
      m.call(0x651840,[mode],this=s,callee_pop=4)
      assert m.r(s+0xc)==mode and m.rb(s+8)==1
      assert [e['call']for e in m.app_events]==(['00794B70']if a else[])+(['0079A340']if b else[])
      assert all(e['mode_before']==mode and e['changed_before']==0 for e in m.app_events)
      cases.append({'image':image,'precision':p,'manager_a_field4':a,'manager_b_field14':b,'mode':mode,'changed':m.rb(s+8),'events':m.app_events.copy()})
  visited.update(m.visited)
 a=[{k:v for k,v in r.items()if k!='image'}for r in cases if r['image']=='original'];b=[{k:v for k,v in r.items()if k!='image'}for r in cases if r['image']=='v14'];assert a==b
 out={'passed':True,'scope':'GameAppState constructor651810/base40CA60, actual40D960 identity getter, and651840 setter with posted-event observational seams; not full Application Init/UI/state transition startup','producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'original_sha256':hashlib.sha256(PE0.b).hexdigest(),'v14_sha256':hashlib.sha256(PE1.b).hexdigest(),'constructors':ctors,'cases':cases,'executed_windows':proof_ranges(visited)}
 (HERE/'native-application-mode.json').write_text(json.dumps(out,indent=2)+'\n');print('passed',len(cases),'mode rows')
if __name__=='__main__':main()
