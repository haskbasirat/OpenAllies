"""Whole766960(false) caller, with named profile lookup and79FC40 controller
creation seams; actual778F20 queue writer remains native. Source/controller
producer proof lives separately in load_sai_players; no invented AI result.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE.parent))
from native_bootstrap import BootstrapMachine,PE0,PE1,PC,proof_ranges
from unicorn.x86_const import *
class SaiMachine(BootstrapMachine):
 def hook(self,u,pc,size,user):
  if getattr(self,'sai_ready',False):
   if pc==0x48d500:
    self.trace.append(['find',self.text(self.arg())]);self.ret(4,self.profile if self.known else 0);return
   if pc==0x79fc40:
    assert self.arg()==self.kingdom and self.arg(2)==self.player
    self.trace.append(['add',self.arg(1)==self.profile]);self.ret(12,0xffffffff if self.fail_slot else 0);return
   if pc==0x778f20:
    self.trace.append(['queue',self.text(self.arg()),self.text(self.arg(1)),self.text(self.arg(2)),self.arg(3)])
   if pc==0x404f60:
    self.trace.append(['diagnostic']);self.ret();return
  super().hook(u,pc,size,user)
def main():
 rows=[];visited={}
 probes=[(False,True,True,False,1,False),(True,False,True,False,1,False),
 (True,True,True,True,1,False),(True,True,True,False,1,False),
 (True,True,False,False,1,False),(True,True,True,False,4,False),(True,True,True,False,4,True)]
 for image in ('original','v14'):
  for p in PC:
   m=SaiMachine(image);m.fpu(p);m.begin(1,list(range(16)),0)
   m.player=m.alloc(0x54);m.cs(m.player+0x14,'ChosenProfile')
   m.kingdom=m.alloc(0x240);m.cs(m.kingdom+0x18,'KingdomThing')
   m.profile=m.alloc(0x18);m.cs(m.profile+8,'ActualProfileIds');m.cs(m.profile+0x10,'ActualProfileName')
   controller=m.alloc(0x20);m.w(controller+0x18,m.profile)
   slots=m.alloc(4);m.w(slots,controller);sai=m.alloc(0x64);m.w(sai+0x60,slots);m.w(0xb36528,sai)
   manager=m.alloc(0x568);m.w(manager+0x564,m.alloc(16));m.w(0xb36530,manager)
   app=m.alloc(0x14);m.w(0xb39690+0xfc,app)
   nation=m.alloc(0x18);m.cs(nation+0x10,'ActualNationName')
   m.sai_ready=True
   for world,kingdom,known,fail,mode,has_nation in probes:
    m.trace=[];m.known=known;m.fail_slot=fail
    m.w(0xb36534,m.alloc(4)if world else 0);m.w(m.player+0x28,m.kingdom if kingdom else 0)
    m.w(app+0xc,mode);m.w(m.kingdom+0x234,nation if has_nation else 0)
    queue=m.alloc(4*12)
    for i in range(12):m.cs(queue+i*4,'')
    m.w(m.S+4,queue);m.w(m.S+8,0);m.w(m.S+12,4)
    m.call(0x766960,[0],this=m.player,callee_pop=4)
    count=m.r(m.S+8);out=[[m.rs(m.r(m.S+4)+i*12+j*4)for j in range(3)]for i in range(count)]
    expected=[]if not world or not kingdom or fail else [[[ord(c)for c in text]for text in ['KingdomThing','ActualNationName'if mode==4 and has_nation else'ActualProfileName','ActualProfileName']]]
    assert out==expected,(image,p,probes,out,expected)
    rows.append({'image':image,'precision':p,'world':world,'kingdom':kingdom,'known':known,'fail_slot':fail,'mode':mode,'nation':has_nation,'trace':m.trace.copy(),'queued':out})
   visited.update(m.visited)
 a=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original'];b=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14'];assert a==b
 out={'passed':True,'scope':'Whole766960(false), actual778F20(false)/778730;48D500 Find/79FC40 construction/diagnostic successful named seams',
  'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'original_sha256':hashlib.sha256(PE0.b).hexdigest(),'v14_sha256':hashlib.sha256(PE1.b).hexdigest(),'cases':rows,'executed_windows':proof_ranges(visited)}
 (HERE/'native-create-sai.json').write_text(json.dumps(out,indent=2)+'\n');print('passed',len(rows),'CreateSAI rows')
if __name__=='__main__':main()
