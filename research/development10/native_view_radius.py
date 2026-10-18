"""Actual45B670 radius setter and the five concrete view virtual30 slots.

The shared943800 ret4 executes. Positive4306C0 is an explicit BEFORE-entry
required-service cutoff, not a successful grid update. Both images/all3PC.
Supplied scalar owners are laboratory data, not actual source model radii.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE/'creator-labels'))
from native_creator_labels import Machine,PE0,PE1,PC
class Required(Exception):pass
class Radius(Machine):
 def hook(self,u,pc,size,user):
  if getattr(self,'armed',False)and pc==0x4306c0:
   self.actual.pop(pc,None)
   self.child=[self.arg(i)for i in range(5)]
   assert self.ecx()==self.grid
   raise Required()
  super().hook(u,pc,size,user)
def main():
 rows=[];pins={};slots=[]
 for name,vt in [('VThing',0xa02d40),('VObject',0xa02798),('VActor',0xa02270),('Organization',0xa019f8),('Camera',0xa01ca0)]:
  raw=PE0.range(vt+0x30,4);assert raw==PE1.range(vt+0x30,4)and int.from_bytes(raw,'little')==0x943800
  slots.append(dict(name=name,va=hex(vt+0x30),bytes=raw.hex()))
 assert PE0.range(0x943800,3)==PE1.range(0x943800,3)==bytes.fromhex('c20400')
 values=[0,0x80000000,0x3f800000,0x40200000,0xc0400000,0x7f800000,0xff800000,0x7fc12345]
 for image in ['original','v14']:
  m=Radius(image)
  for precision in PC:
   for old in values:
    for new in values:
     for occupied in [False,True]:
      m.armed=False;m.fpu(precision);view=m.alloc(0x100);m.w(view,0xa02798)
      m.w(view+0x74,old);m.w(view+0x54,0x1000020 if occupied else 0x1000000)
      m.w(view+0x38,0x41280000);m.w(view+0x3c,0xc0780000)
      world=m.alloc(0x100);m.grid=m.alloc(0x100);m.w(world+0x94,m.grid);m.w(0xb36538,world)
      m.actual={};m.record_actual=True;m.child=None;m.armed=True;error=None
      try:m.call(0x45b670,[new],this=view,callee_pop=4)
      except Required:error='4306C0'
      pins.update(m.actual)
      child=None if m.child is None else [*m.child[1:]]
      if m.child is not None:assert m.child[0]==view
      rows.append(dict(image=image,precision=precision,old=old,new=new,occupied=occupied,
       output=dict(radius=m.r(view+0x74),flags=m.r(view+0x54),child=child,error=error)))
 assert [{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original']==[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14']
 instructions=[]
 for va,size in sorted(pins.items()):
  if PE0.base<=va<PE0.base+PE0.image_size:
   raw=PE0.range(va,size);assert raw==PE1.range(va,size);instructions.append(dict(va=hex(va),bytes=raw.hex()))
 doc=dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,instructions=instructions,slots=slots)
 out=HERE/'native-view-radius.json'
 if '--write'in sys.argv:out.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(out.read_text())==doc
 print(json.dumps(dict(rows=len(rows),pins=len(instructions),sha256=hashlib.sha256(out.read_bytes()).hexdigest())))
if __name__=='__main__':main()
