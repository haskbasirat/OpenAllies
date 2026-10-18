"""Actual457A40 VObject Init selection, model store, radius and common dispatch.

Laboratory source/Thing/Aspect/Model scalar owners are explicit. Actual view
virtual104, runtime B4/F4, stance/default getters and45B670 execute.454F70
model resolution and45B530 common Init are named supplied child boundaries;
neither source resource loading nor common Init is certified by these rows.
Positive4306C0 and null-Thing random49B900 are BEFORE-entry required cutoffs.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parent/'creator-labels'))
from native_creator_labels import Machine,PE0,PE1,PC
class Stop(Exception):pass
class ObjectInit(Machine):
 def hook(self,u,pc,size,user):
  if getattr(self,'armed',False):
   if pc==0x454f70:
    assert self.ecx()==self.view
    aspect=self.arg();assert aspect in self.aspects
    self.events.append(['model',self.aspects.index(aspect)])
    if self.c['cutoff']=='model':raise Stop('454F70')
    self.ret(4,self.model);return
   if pc==0x45b530:
    assert self.ecx()==self.view
    self.events.append(['common'])
    if self.c['cutoff']=='common':raise Stop('45B530')
    self.ret(0,0xaabbcc00|self.c['result']);return
   if pc==0x4306c0:
    self.events.append(['grid',*[self.arg(i)for i in range(1,5)]])
    assert self.arg()==self.view
    raise Stop('4306C0')
   if pc==0x49b900:
    assert self.ecx()==self.data and self.arg()==self.app+0xd4
    self.events.append(['random'])
    raise Stop('49B900')
  super().hook(u,pc,size,user)

def run(m,precision,c):
 m.armed=False;m.fpu(precision);m.actual={};m.record_actual=True
 m.view=m.alloc(0xc0);m.w(m.view,0xa02798)
 m.data=m.alloc(0x600);m.w(m.view+0x10,m.data)
 m.w(m.view+0xb4,c['initial_index']);m.w(m.view+0x7c,0);m.w(m.view+0x74,0)
 m.w(m.view+0x54,0x1000020 if c['occupied']else 0x1000000)
 m.w(m.view+0x38,0x41280000);m.w(m.view+0x3c,0xc0780000)
 m.b(m.data+0x108,c['variations']);m.w(m.data+0x118,0)
 construction=m.alloc(4);m.w(construction,2);m.w(m.data+0x11c,construction);m.w(m.data+0x120,c['construction_count'])
 stance=m.alloc(8);entry=m.alloc(0xc);child=m.alloc(0xc);m.w(stance+4,entry);m.w(entry+8,child);m.w(child+8,3);m.w(m.data+0x188,stance)
 m.aspects=[];array=m.alloc(16)
 for i,radius in enumerate([0x3fa00000,0x40200000,0xc0500000,0x3dcccccd]):
  aspect=m.alloc(0x90);m.w(aspect+0x64,radius);m.aspects.append(aspect);m.w(array+4*i,aspect)
 m.w(m.data+0x10c,array)
 m.thing=0
 if c['thing']!='None':
  m.thing=m.alloc(0x350);m.w(m.thing,{'Object':0xa2a2a0,'Actor':0xa273f0,'Kingdom':0xa299d8,'Area':0xa28a00,'Camera':0xa28b88,'Doodad':0xa28e90}[c['thing']]);m.w(m.thing+4,m.data)
  m.w(m.thing+0x40,c['variant']);m.w(m.thing+0x3c,c['scale']);m.w(m.thing+0x100,c['state']);m.w(m.thing+0x15c,c['stance'])
 m.w(m.view+0x14,m.thing)
 m.model=m.alloc(0x100);m.w(m.model+8,c['model_flags']);m.w(m.model+0x64,c['model_radius'])
 world=m.alloc(0x110);grid=m.alloc(0x100);m.w(world+0x94,grid);m.w(0xb36538,world)
 m.app=m.alloc(0x200);m.w(0xb3653c,m.app)
 m.events=[];m.c=c;m.armed=True;error=None;result=None
 try:result=m.call(0x457a40,this=m.view)&0xff
 except Stop as e:error=str(e)
 return dict(index=m.r(m.view+0xb4),model=m.r(m.view+0x7c)==m.model,radius=m.r(m.view+0x74),flags=m.r(m.view+0x54),events=m.events,error=error,result=result),m.actual.copy()

def main():
 base=dict(thing='Actor',variations=0,initial_index=0xffffffff,variant=1,scale=0x3f800000,state=0,stance=0xffffffff,
  construction_count=0,model_flags=0,model_radius=0x40b00000,occupied=False,cutoff='',result=1)
 cases=[]
 for thing in ['None','Object','Actor','Kingdom','Area','Camera','Doodad']:
  for variations in [0,1,128]:
   for initial in [0xffffffff,1]:
    for ready in [0,4]:cases.append(dict(base,thing=thing,variations=variations,initial_index=initial,model_flags=ready))
 for state,count,stance in [(1<<21,1,1),(1<<21,0,1),(0,1,1),(1<<21,1,0xffffffff)]:
  for initial in [0xffffffff,1]:cases.append(dict(base,state=state,construction_count=count,stance=stance,initial_index=initial))
 for scale in [0,0x80000000,0x3dcccccd,0x3f800001,0xbf800000,0x41200000]:
  for ready in [0,4]:cases.append(dict(base,scale=scale,model_flags=ready,initial_index=2))
 for cutoff in ['model','common']:
  cases.append(dict(base,cutoff=cutoff))
 cases.extend([dict(base,occupied=True),dict(base,result=0)])
 rows=[];pins={}
 for image in ['original','v14']:
  m=ObjectInit(image)
  for precision in PC:
   for c in cases:
    output,actual=run(m,precision,c);pins.update(actual);rows.append(dict(image=image,precision=precision,input=c,output=output))
 assert [{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original']==[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14']
 instructions=[]
 for va,size in sorted(pins.items()):
  if PE0.base<=va<PE0.base+PE0.image_size:
   raw=PE0.range(va,size);assert raw==PE1.range(va,size);instructions.append(dict(va=hex(va),bytes=raw.hex()))
 slots=[]
 for vt in [0xa2a2a0,0xa273f0,0xa299d8,0xa28a00,0xa28b88,0xa28e90]:
  for offset in [0xb4,0xf4]:
   raw=PE0.range(vt+offset,4);assert raw==PE1.range(vt+offset,4);slots.append(dict(va=hex(vt+offset),bytes=raw.hex()))
 doc=dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,instructions=instructions,runtime_slots=slots)
 out=HERE/'native-object-init.json'
 if '--write'in sys.argv:out.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(out.read_text())==doc
 print(json.dumps(dict(rows=len(rows),pins=len(instructions),sha256=hashlib.sha256(out.read_bytes()).hexdigest())))
if __name__=='__main__':main()
