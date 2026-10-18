"""Unchanged52D7C0 guard parent: named resume/activation fixture children.
No successful callback stands in for a completed reset or actual game actor.
NativeNULL52A660 executes on success; failure probes stop at real childentry.
"""
from pathlib import Path
import json,sys,struct,itertools,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
ctors=json.loads((ROOT/'research/phase7-mind/native-constructors.json').read_text())
raw=bytes.fromhex(next(c for c in ctors['cases']if c['input']['kind']=='element')['output']['initialized_hex'])
class Probe(nm.Machine):
 def __init__(self,pc,image,c):
  super().__init__(pc)
  if image=='v14':
   old=(nm.original.image_size+0xfff)&~0xfff;new=(nm.modified.image_size+0xfff)&~0xfff
   if new>old:self.u.mem_map(nm.original.base+old,new-old)
   for vs,r,size,off in nm.modified.sections:
    if size:self.u.mem_write(nm.modified.base+r,nm.modified.b[off:off+size])
  self.c=c;self.trace=[];self.cut=None
  self.mind=self.alloc(len(raw));self.u.mem_write(self.mind,raw);self.w(self.mind+0x20,0)
  self.wb(0xb3d2d4,int(c['guard']))
  self.observe(0x52d7c0,lambda m:self.trace.append(['guard_read']))
  self.observe(0x52d7cc,lambda m:self.trace.append(['guard_store',True]))
  self.observe(0x52d7dc,lambda m:self.trace.append(['guard_store',False]))
  self.observe(0x52d7f1,lambda m:self.trace.append(['guard_store',False]))
  self.seam(0x52bf30,lambda m:self.child('resume',[],int(c['resume']),0))
  self.seam(0x52a810,lambda m:self.child('activate',[self.ecx()-self.mind,self.arg(1)],int(c['activate_result']),4))
  self.observe(0x52a660,lambda m:self.clear())
 def child(self,name,args,result,pop):
  self.trace.append([name,*args])
  if self.c['fail']==name:self.cut=name;self.u.emu_stop();return
  self.ret(result,pop)
 def clear(self):
  self.trace.append(['clear'])
  if self.c['fail']=='clear':self.cut='clear';self.u.emu_stop()
 def run(self):
  try:self.call(0x52d7c0,self.mind)
  except AssertionError:
   if not self.cut:raise
  return {'trace':self.trace,'guard':bool(self.rb(0xb3d2d4)),'current_command':self.r(self.mind+0x20),'failure':self.cut}
def main():
 rows=[];pins={}
 for pc in nm.PRECISIONS:
  for guard,resume,accepted,fail in itertools.product((False,True),(False,True),(False,True),(None,'resume','activate','clear')):
   c=dict(guard=guard,resume=resume,activate_result=accepted,fail=fail);pair=[]
   for image in('original','v14'):
    m=Probe(pc,image,c);pair.append(m.run());pins.update(m.visited)
   assert pair[0]==pair[1];rows.append({'precision':pc,'input':c,'output':pair[0]})
 for addr,data in pins.items():assert nm.original.range(addr,len(data))==nm.modified.range(addr,len(data))==data
 result={**nm.header(__file__,__doc__),'rows':rows,'instructions':nm.instructions(pins),
  'constructor_fixture_sha256':hashlib.sha256((ROOT/'research/phase7-mind/native-constructors.json').read_bytes()).hexdigest(),
  'harness_sha256':hashlib.sha256((ROOT/'research/phase8-ai/native_machine.py').read_bytes()).hexdigest()}
 path=HERE/'native-element-reset.json'
 if '--write'in sys.argv:path.write_text(json.dumps(result,indent=1))
 else:assert json.loads(path.read_text())==result
 print(json.dumps({'rows':len(rows),'native_runs':2*len(rows),'executed_pins':len(pins),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
