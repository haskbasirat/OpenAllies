"""Same Application+84 constructor/time32 seed and UI+D4 reseed slices.

Runs actual41788C..9C,419898..AD,5CAFA7..D5 and5C8669..8D plus85D840/70/80
on each PE at all three x87 precisions. CRT_time32 is an explicit supplied
signed laboratory input, not a captured OS result. Prior global draws are
explicit test inputs, not a claim about native startup order. Every stream
keeps its same word; the UI slice stops before the following camera allocation.
Whole Application/UI Init, other consumers and indirect alias closure are
excluded. No native process, source data, profile or save is changed.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parent/'creator-labels'))
from native_creator_labels import Machine,PE0,PE1,PC
from unicorn.x86_const import UC_X86_REG_ESI,UC_X86_REG_EBX,UC_X86_REG_ECX,UC_X86_REG_ESP
APP=0xb39690
GLOBAL=APP+0x84

class RandomMachine(Machine):
 def hook(self,u,pc,size,user):
  if getattr(self,'time_ready',False)and pc==0x96d1ef:
   assert self.arg()==0
   self.time_calls+=1
   self.ret(value=self.time32&0xffffffff)
   return
  super().hook(u,pc,size,user)

def window(m,start,end,this):
 m.u.reg_write(UC_X86_REG_ESI,this)
 m.u.reg_write(UC_X86_REG_ECX,this)
 m.u.reg_write(UC_X86_REG_EBX,0)
 m.u.reg_write(UC_X86_REG_ESP,m.stack)
 m.u.emu_start(start,end,count=1000)
 assert m.u.reg_read(UC_X86_REG_ESP)==m.stack

def run(m,pc,time32,prior):
 m.fpu(pc);m.actual={};m.record_actual=True;m.time_ready=False
 m.w(GLOBAL,0xa5a5a5a5)
 window(m,0x41788c,0x41789c,APP)
 ctor=m.r(GLOBAL);assert ctor==0x8e4b12a5
 m.time32=time32;m.time_calls=0;m.time_ready=True
 window(m,0x419898,0x4198ad,APP)
 assert m.time_calls==1
 m.time_ready=False
 seeded=m.r(GLOBAL)
 preceding=[]
 for _ in range(prior):
  value=m.call(0x85d840,[GLOBAL]);preceding.append(dict(value=value,state=m.r(GLOBAL)))
 ui=m.alloc(0x200);m.u.mem_write(ui,b'\xa5'*0x200)
 window(m,0x5cafa7,0x5cafd5,ui)
 ui_ctor=m.r(ui+0xd4);assert ui_ctor==ctor
 window(m,0x5c8669,0x5c868d,ui)
 global_after=m.r(GLOBAL);ui_seed=m.r(ui+0xd4)
 values=[]
 for _ in range(12):
  value=m.call(0x85d840,[ui+0xd4]);values.append(dict(value=value,state=m.r(ui+0xd4)))
 assert m.r(GLOBAL)==global_after
 return dict(application_constructor=ctor,application_seeded=seeded,preceding=preceding,
             global_after=global_after,ui_constructor=ui_ctor,ui_seeded=ui_seed,values=values),m.actual

def main():
 rows=[];pins={}
 for image in ['original','v14']:
  m=RandomMachine(image)
  for pc in PC:
   for time32 in [0,1,-1,-2147483648,2147483647,1792195200]:
    for prior in [0,2,32]:
     output,actual=run(m,pc,time32,prior);pins.update(actual)
     rows.append(dict(image=image,precision=pc,input=dict(time32=time32,prior_global_draws=prior),output=output))
 a=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original']
 b=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14'];assert a==b
 instructions=[]
 for va,size in sorted(pins.items()):
  if PE0.base<=va<PE0.base+PE0.image_size:
   raw=PE0.range(va,size);assert raw==PE1.range(va,size)
   instructions.append(dict(va=hex(va),bytes=raw.hex()))
 doc=dict(schema=1,scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
          rows=rows,instructions=instructions)
 out=HERE/'native-application-random.json'
 if '--write'in sys.argv:out.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(out.read_text())==doc
 print(json.dumps(dict(rows=len(rows),pins=len(instructions),sha256=hashlib.sha256(out.read_bytes()).hexdigest())))
if __name__=='__main__':main()
