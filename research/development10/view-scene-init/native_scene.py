"""Native43F948..43FCA4 scene slice, actual Ni constructors/attachment/update.

The caller supplies a valid empty VWorld field projection, explicit current
NiPoint3 seed words, current camera root (real867690) and a7010D0 camera service
seam. OS thread/critical-section imports are allocator transports; the actual
8D4B60 pool allocator and its linked-node initialization execute. No AD write,
terrain/resource barrier or complete VWorld initialization is claimed.
"""
from pathlib import Path
import sys,json,itertools
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'))
import native_machine as h
from unicorn.x86_const import *
ORIGINAL,V14=h.P,h.V

class Machine(h.Machine):
 def __init__(self,pc,c):
  super().__init__(b'',pc);self.c=c;self.events=[];self.outcome='running'
  self.view=self.alloc(0x300);self.camera=self.alloc(0x300)
  self.parent=self.alloc(0x100,0xa5);self.call(0x867690,self.parent,[0])
  for i,w in enumerate(c['parent']):self.w(self.parent+0x6c+4*i,w)
  self.w(self.camera+0x164,self.parent)
  pointer=self.alloc(4);self.w(pointer,self.camera);self.w(0xb40048,pointer)
  for i,w in enumerate(c['seed']):self.w(0xb43cf8+4*i,w)
  # Valid preexisting pool freelist, not a seam over its allocate/clear body.
  head=0
  for i in range(48):
   p=self.alloc(12,0xa5);self.w(p,head);head=p
  self.w(0xb45f1c,head)
  for slot,pop,value in[(0x9f01e4,0,7),(0x9f01f0,4,0),(0x9f01f4,4,0)]:
   self.w(slot,self.stub(lambda m,pop=pop,value=value:m.ret(pop,value)))
  self.seam(0x7010d0,self.camera_reset)
  self.probes[0x43fca4]=lambda m:m.stop('complete')
  self.probes[0x43fb66]=lambda m:m.event('camera_attach_complete')
  self.probes[0x43fbc3]=lambda m:m.event('root_updates_complete')
  self.probes[0x43fbee]=lambda m:m.event('light_direction_complete')
  self.probes[0x43fc53]=lambda m:m.event('effect_stored')
  if c['existing_effect']:
   p=self.alloc(0x190,0xa5);self.call(0x86b940,p,[]);self.w(self.view+0x118,p)
   self.w(p+0x160,5);self.w(p+0xc4,0)
 def stop(self,name):self.outcome=name;raise h.Throw()
 def event(self,name):
  if self.c['stop']==name:self.stop('error:'+name)
  self.events.append(name)
 def camera_reset(self,m):
  assert m.u.reg_read(UC_X86_REG_ECX)==self.camera
  self.event('camera_reset');self.ret(0)
 def words(self,p,o,n):return[self.r(p+o+4*i)for i in range(n)]
 def snapshot(self):
  root,wire,tex,context,effect=[self.r(self.view+o)for o in[0xf0,0x100,0x104,0x108,0x118]]
  light=self.r(self.r(context+4)+8)if context else 0
  desc=self.r(self.r(tex+0x2c))if tex else 0
  return dict(outcome=self.outcome,events=self.events,
   present=[bool(p)for p in[root,wire,tex,context,effect]],
   root_local=self.words(root,0x38,13)if root else None,
   root_world=self.words(root,0x6c,13)if root else None,
   root_parent=self.r(root+0x24)==self.parent if root else False,
   root_children=self.r(root+0xd4)if root else 0,
   root_properties=self.r(root+0xac)if root else 0,
   root_effects=self.r(root+0xe8)if root else 0,
   wire_flags=int.from_bytes(self.u.mem_read(wire+0x20,2),'little')if wire else None,
   texturing=[self.r(tex+0x20),self.r(tex+0x30),self.r(tex+0x34),self.r(desc+0xc)if desc else None]if tex else None,
   context=[self.r(context),self.r(self.r(context+4)+4)]if context else None,
   light_local=self.words(light,0x38,13)if light else None,
   light_world=self.words(light,0x6c,13)if light else None,
   light_direction=self.words(light,0x120,3)if light else None,
   light_values=self.words(light,0xf4,10)if light else None,
   planar_direction=self.words(0xb285cc,0,2),
   effect=[self.r(effect+0x154),self.r(effect+0x15c),self.r(effect+0x160),self.r(effect+0x164),self.b(effect+0xc4)]if effect else None,
   effect_world=self.words(effect,0x6c,13)if effect else None,
   initialized=self.b(self.view+0xad))
 def run(self):
  self.u.reg_write(UC_X86_REG_ESI,self.view)
  self.u.reg_write(UC_X86_REG_EBP,0);self.u.reg_write(UC_X86_REG_EBX,0xffffffff)
  self.call(0x43f948,0,[])
  assert not self.null_accesses,self.null_accesses
  assert self.outcome=='complete'or self.outcome.startswith('error:'),self.outcome
  return self.snapshot()

def main():
 identity=[h.f32(float(i in[0,4,8]))for i in range(9)]+[0,0,0,h.f32(1)]
 rotated=[0,h.f32(-1),0,h.f32(1),0,0,0,0,h.f32(1),h.f32(17),h.f32(-9),h.f32(3),h.f32(1)]
 oblique=[h.f32(v)for v in[.36,-.8,.48,.8,0,-.6,.48,.6,.64,11.25,-5.5,2.125,1.25]]
 cases=[]
 for parent,seed,existing in itertools.product([identity,rotated,oblique],[[0,0,0],[h.f32(.125),h.f32(-.25),h.f32(.5)]],[False,True]):
  cases.append(dict(parent=parent,seed=seed,existing_effect=existing,stop=None))
 baseline=dict(parent=identity,seed=[0,0,0],existing_effect=False,stop=None)
 for stop in['camera_reset','camera_attach_complete','root_updates_complete','light_direction_complete','effect_stored']:
  cases.append(dict(baseline,stop=stop))
 h.VISITED.clear();rows=[]
 for pc,c in itertools.product(h.CW,cases):
  h.P=ORIGINAL;a=Machine(pc,c).run();h.P=V14;b=Machine(pc,c).run();assert a==b,(pc,c)
  rows.append(dict(precision=pc,input=c,output=a))
 numeric=[]
 #Finite transform compositions include cancellation/summation cases in
 #addition to actual fresh-scene identity/local-seed transforms above.
 matrices=[identity,rotated,oblique]
 matrices.append([h.f32(v)for v in[16777216.,1.,-16777216.,.33333334,-.2,.75,-.7,.125,1.,1.0000001,-.99999994,.03125,.875]])
 for pc,parent,local in itertools.product(h.CW,matrices,matrices):
  pair=[]
  for image in[ORIGINAL,V14]:
   h.P=image;m=h.Machine(b'',pc);a=m.alloc(52);c=m.alloc(52);out=m.alloc(52)
   for i,w in enumerate(parent):m.w(a+4*i,w)
   for i,w in enumerate(local):m.w(c+4*i,w)
   result=m.call(0x45cab0,a,[out,c]);assert result==out and not m.null_accesses
   pair.append([m.r(out+4*i)for i in range(13)])
  assert pair[0]==pair[1];numeric.append(dict(kind='transform',precision=pc,parent=parent,local=local,result=pair[0]))
 directions=[[0,0,0],[0x80000000,0,0],[h.f32(1e-4),0,0],[h.f32(1e-4)+1,0,0],[h.f32(1e-4)-1,0,0],
  [h.f32(3),h.f32(4),0],[h.f32(-3),h.f32(4),0],[h.f32(.36),h.f32(.8),h.f32(.48)]]
 for pc,direction in itertools.product(h.CW,directions):
  pair=[]
  for image in[ORIGINAL,V14]:
   h.P=image;m=h.Machine(b'',pc);m.w(h.STACK,h.STOP)
   for i,w in enumerate(direction):m.w(h.STACK+4+4*i,w)
   m.u.reg_write(UC_X86_REG_ESP,h.STACK);m.u.emu_start(0x717460,h.STOP,count=10000)
   assert m.u.reg_read(UC_X86_REG_ESP)==h.STACK+4
   pair.append([m.r(0xb285cc+4*i)for i in range(2)])
  assert pair[0]==pair[1];numeric.append(dict(kind='planar',precision=pc,direction=direction,result=pair[0]))
 ctors=[]
 for pc,fill,null_array in itertools.product(h.CW,[0,0xa5,0xff],[False,True]):
  pair=[]
  for image in[ORIGINAL,V14]:
   h.P=image;m=h.Machine(b'',pc);view=m.alloc(0x300,fill)
   for va,pop in[(0x5f1d00,8),(0x424b90,0),(0x42e730,4)]:m.seam(va,lambda m,pop=pop:m.ret(pop))
   if null_array:m.seam(0x6f7810,lambda m:m.ret(0,0))
   def finished(m):m.outcome={'kind':'bounded-constructor-fields'};raise h.Throw()
   m.probes[0x442d99]=finished;m.call(0x442c00,view,[0x12345678])
   assert not m.null_accesses
   values=[m.r(view+o)for o in[0xf0,0x100,0x104,0x108,0x118]];assert values==[0]*5
   pair.append(values)
  assert pair[0]==pair[1];ctors.append(dict(precision=pc,fill=fill,null_unrelated_array=null_array,result=pair[0]))
 h.P=ORIGINAL
 excluded={0x6f7810,0x6f7430,0x7010d0,0x43fca4,0x43fb66,0x43fbc3,0x43fbee,0x43fc53,0x5f1d00,0x424b90,0x42e730,0x442d99}
 pins={hex(a):v for a,v in sorted(h.VISITED.items())if a not in excluded}
 doc=dict(schema=2,scope=__doc__,producer_sha256=h.H(Path(__file__).read_bytes()),rows=rows,numeric=numeric,constructor_fields=ctors,pins=pins,
  service_seams=['7010D0 current-camera reset; does not modify supplied parent world transform','constructor field projection only:5F1D00/424B90/42E730 required base children are explicit seams'],
  transports=['6F7810 allocate A5','6F7430 free without reuse','GetCurrentThreadId=7','Enter/LeaveCriticalSection'],
  observed_cutoffs=[hex(a)for a in sorted(excluded)if a>=0x43f948 and a<=0x43fca4])
 data=(json.dumps(doc,indent=1)+'\n').encode();path=HERE/'native-scene.json'
 if '--write'in sys.argv:path.write_bytes(data)
 else:assert path.read_bytes()==data
 print(json.dumps(dict(rows=len(rows),numeric=len(numeric),constructors=len(ctors),executions=2*(len(rows)+len(numeric)+len(ctors)),pins=len(pins),sha256=h.H(data))))
if __name__=='__main__':main()
