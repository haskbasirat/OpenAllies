"""Original4D8290 component load/default/remove branches. Factories, virtual Loads and deletes are observation seams; stop at native invalid-definition diagnostic."""
from pathlib import Path
import sys,json,hashlib,struct
ROOT=Path(__file__).resolve().parents[3]
sys.path.insert(0,str(ROOT/'research/phase6-actor-payloads'))
from native_actor_payload import TailMachine,P,V
from unicorn.x86_const import *
class Machine(TailMachine):
 def __init__(self):
  super().__init__(b'');self.events=[];self.rejected=False
  self.w(self.cvtable+0x2c,0x301f700);self.w(self.cvtable,0x301f800)
 def hook(self,u,pc,size,data):
  if pc==0x4cf480:
   slot=self.arg();self.events.append('construct');self.w(self.thing+0x60+slot*4,self.component);self.ret(pop=4)
  elif pc==0x301f700:self.events.append('load');self.ret(pop=4)
  elif pc==0x301f800:self.events.append('delete');self.ret(pop=4)
  elif pc==0x858a00:self.rejected=True;u.emu_stop()
  else:super().hook(u,pc,size,data)
cases=[]
for slot in range(31):
 for saved in [False,True]:
  for default in [False,True]:
   for defined in [False,True]:
    m=Machine();bit=1<<slot
    m.w(m.definition+0x174,bit if defined else 0)
    sm,dm=0x2089000,0x2089004;m.w(sm,bit if saved else 0);m.w(dm,bit if default else 0)
    for i,v in enumerate([m.stop,m.world,slot,sm,dm]):m.w(m.stack+i*4,v)
    m.u.reg_write(UC_X86_REG_ESP,m.stack);m.u.reg_write(UC_X86_REG_ECX,m.thing)
    m.u.emu_start(0x4d8290,m.stop,count=10000)
    if not m.rejected:assert m.u.reg_read(UC_X86_REG_EIP)==m.stop and m.u.reg_read(UC_X86_REG_ESP)==m.stack+20
    cases.append({'slot':slot,'saved':saved,'default':default,'defined':defined,'native_events':m.events,'native_present':m.r(m.thing+0x60+slot*4)!=0,'native_definition_diagnostic':m.rejected})
raw=P.range(0x4d8290,0x1bf);assert raw==V.range(0x4d8290,len(raw))
out={'scope':__doc__,'original_sha256':hashlib.sha256(P.b).hexdigest(),'source_range_sha256':hashlib.sha256(raw).hexdigest(),'cases':cases}
p=Path(__file__).with_name('native-component-actions.json');p.write_text(json.dumps(out,indent=2)+'\n');print({'cases':len(cases),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()})
