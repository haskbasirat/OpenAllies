"""Native51E3A0 parent coordinator, successful/failing explicit child contracts.

The real guard, command-clearNULL leaf,4D11A0 generation lookup, source+310
read and branch order execute. resume52BF30, Join51E1D0, activation52A810,
Denizen56C210 and scheduling50AD80 are explicitly named fixture children, not
claimed native effects or initialized game actors. Errors stop at real entry.
"""
from pathlib import Path
import json,sys,struct,itertools,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
from unicorn.x86_const import UC_X86_REG_EIP
ctors=json.loads((ROOT/'research/phase7-mind/native-constructors.json').read_text())
raw=bytes.fromhex(next(c for c in ctors['cases']if c['input']['kind']=='regiment')['output']['initialized_hex'])
class Probe(nm.Machine):
 def __init__(self,pc,image,c):
  super().__init__(pc)
  if image=='v14':
   old=(nm.original.image_size+0xfff)&~0xfff;new=(nm.modified.image_size+0xfff)&~0xfff
   if new>old:self.u.mem_map(nm.original.base+old,new-old)
   for vs,r,size,off in nm.modified.sections:
    if size:self.u.mem_write(nm.modified.base+r,nm.modified.b[off:off+size])
  self.c=c;self.trace=[];self.cut=None
  self.mind=self.alloc(len(raw));self.u.mem_write(self.mind,raw);self.actor=self.alloc(0x200);self.data=self.alloc(0x580)
  self.home=self.alloc(0x200);self.registry=self.alloc(0x40008);self.denizen=self.alloc(0x50)
  self.w(self.mind+4,self.actor);self.w(self.actor+4,self.data);self.w(self.actor+0x70,self.mind)
  self.w(self.mind+0x20,0);self.w(self.mind+0x28,0x01000001 if c['home']!='none' else 0)
  self.w(0xb364c0,self.registry);self.u.mem_write(self.registry+6,struct.pack('<H',0x0100));self.w(self.registry+0x20008,self.home)
  self.w(self.home+8,int(c['home']=='removed'));self.w(self.home+0x88,self.denizen);self.wb(self.data+0x310,int(c['rampages']))
  self.wb(0xb3cf58,int(c['guard']))
  self.observe(0x52a660,lambda m:self.trace.append(['clear']))
  self.observe(0x51e3a8,lambda m:self.trace.append(['guard_read']))
  self.observe(0x51e3b7,lambda m:self.trace.append(['guard_store',True]))
  for a in(0x51e3fc,0x51e420,0x51e442):self.observe(a,lambda m:self.trace.append(['guard_store',False]))
  self.observe(0x4d11a0,lambda m:self.trace.append(['home']))
  self.observe(0x51e3d6,lambda m:self.trace.append(['home_removed']))
  self.observe(0x51e40b,lambda m:self.trace.append(['rampages']))
  self.seam(0x52bf30,lambda m:self.child('resume',[],int(c['resume']),0))
  self.seam(0x51e1d0,lambda m:self.child('join',[self.arg(1),self.arg(2)==self.home,self.arg(3)],0,12))
  self.seam(0x56c210,lambda m:self.child('denizen',[self.arg(1)==self.actor],0,4))
  self.seam(0x52a810,lambda m:self.child('activate',[self.ecx()-self.mind,self.arg(1)],int(c['activate_result']),4))
  self.seam(0x50ad80,lambda m:self.child('schedule',[self.arg(1),self.arg(2)],0,8))
 def child(self,name,args,result,pop):
  self.trace.append([name,*args])
  if self.c['fail']==name:self.cut=name;self.u.emu_stop();return
  self.ret(result,pop)
 def run(self):
  try:self.call(0x51e3a0,self.mind)
  except AssertionError:
   if not self.cut:raise
  return {'trace':self.trace,'guard':bool(self.rb(0xb3cf58)),'current_command':self.r(self.mind+0x20),'failure':self.cut}
def main():
 cases=[]
 for guard,resume,home,rampages,accepted,fail in itertools.product((False,True),(False,True),('none','live','removed'),(False,True),(False,True),(None,'resume','join','denizen','activate','schedule')):
  cases.append(dict(guard=guard,resume=resume,home=home,rampages=rampages,activate_result=accepted,fail=fail))
 rows=[];pins={}
 for pc in nm.PRECISIONS:
  for c in cases:
   pair=[]
   for image in('original','v14'):
    m=Probe(pc,image,c);pair.append(m.run());pins.update(m.visited)
   assert pair[0]==pair[1]
   rows.append({'precision':pc,'input':c,'output':pair[0]})
 for addr,data in pins.items():assert nm.original.range(addr,len(data))==nm.modified.range(addr,len(data))==data
 result={**nm.header(__file__,__doc__),'scope':__doc__,'rows':rows,'instructions':nm.instructions(pins),
  'constructor_fixture_sha256':hashlib.sha256((ROOT/'research/phase7-mind/native-constructors.json').read_bytes()).hexdigest(),
  'harness_sha256':hashlib.sha256((ROOT/'research/phase8-ai/native_machine.py').read_bytes()).hexdigest()}
 path=HERE/'native-regiment-reset.json'
 if '--write'in sys.argv:path.write_text(json.dumps(result,indent=1))
 else:assert json.loads(path.read_text())==result
 print(json.dumps({'rows':len(rows),'native_runs':2*len(rows),'executed_pins':len(pins),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
