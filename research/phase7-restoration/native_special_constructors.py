"""Whole original Team/Kingdom saved-load constructors, including shared base constructors and derived defaults. Allocation leaves only; initialized storage is distinguished from untouched CD bytes."""
from pathlib import Path
import sys,json,struct
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'research/phase5-actors'))
from original_creation import Machine,P,V,H
from unicorn import UC_HOOK_MEM_WRITE
from unicorn.x86_const import *
class SpecialMachine(Machine):
 def __init__(self):
  super().__init__(0x27f);self.changed=set();self.u.mem_write(self.obj,b'\xcd'*0x400)
  self.u.hook_add(UC_HOOK_MEM_WRITE,self.write_hook)
 def hook(self,u,pc,size,user):
  if pc==0x6f7430:self.ret()
  else:super().hook(u,pc,size,user)
 def write_hook(self,u,access,a,n,value,user):
  if self.obj<=a<self.obj+0x400:self.changed.update(range(a-self.obj,a-self.obj+n))
rows=[];visited={}
for kind,entry,start,end in [('Team',0x4e67c0,0x38,0x6c),('Kingdom',0x4e5370,0x17c,0x374)]:
 for count in ([0] if kind=='Team' else [0,1,7,8,9,17]):
  for saved in [1,0x12340007,0xffffffff]:
   m=SpecialMachine();m.w(0xb36530,0x2250000);m.w(0x2250000+0x38,count);m.call(0x4efb70,[saved]);
   try:m.invoke(entry,m.obj,[0,m.definition,saved])
   except Exception:print(kind,count,saved,[hex(x)for x in m.recent]);raise
   words={str(o):m.r(m.obj+o) for o in range(start,end,4) if all(o+i in m.changed for i in range(4))}
   partial={str(o):m.u.mem_read(m.obj+o,1)[0] for o in range(start,end) if o in m.changed and str(o//4*4) not in words}
   # Addresses never become portable scalar words. Each owned allocation is
   # represented by its initialized bytes/count and growth metadata instead.
   pointer_fields={};strings={};vectors={}
   str_offsets=[0x3c,0x40,0x64] if kind=='Team' else [0x194,0x198,0x240,0x248,0x348]
   vec_offsets=[0x4c,0x58] if kind=='Team' else [0x1a0,0x1ac,0x1b8,0x1c4,0x1d0,0x1dc,0x214,0x24c,0x258,0x2c4,0x2d0,0x2dc,0x2e8,0x300,0x318,0x34c]
   for o in str_offsets:
    p=m.r(m.obj+o);n=m.r(p-12);strings[str(o)]=list(struct.unpack('<'+'H'*n,m.u.mem_read(p,n*2)));words.pop(str(o),None)
   for o in vec_offsets:
    p=m.r(m.obj+o);n=m.r(m.obj+o+4);cap=m.r(m.obj+o+8);vectors[str(o)]={'count':n,'growth':cap,'values':[m.r(p+i*4)for i in range(n)]};words.pop(str(o),None)
   if kind=='Kingdom':
    # Recruitment and supplemental zero-filled indexed containers.
    for o,n in [(0x204,57),(0x35c,10)]:
     p=m.r(m.obj+o);pointer_fields[str(o)]=[m.r(p+i*4)for i in range(n)];words.pop(str(o),None)
    words.pop(str(0x200),None) # Recruitment owner pointer, represented by saved ID.
    words.pop(str(0x180),None) # embedded Updatable vtable
    words.pop(str(0x1fc),None) # embedded Recruitment vtable
   rows.append({'kind':kind,'resource_count':count,'saved_id':saved,'words':words,'partial_bytes':partial,'strings':strings,'vectors':vectors,'zero_filled_indexed':pointer_fields,'registry':m.snapshot()});visited.update(m.visited)
pins=[]
for a,n in sorted(visited.items()):
 if P.base<=a<P.base+P.image_size:
  b=P.range(a,n);assert b==V.range(a,n);pins.append({'address':hex(a),'bytes':b.hex()})
p=Path(__file__).with_name('native-special-constructors.json');p.write_text(json.dumps({'scope':__doc__,'original_sha256':H(P.b),'v14_sha256':H(V.b),'cases':rows,'instructions':pins},indent=2)+'\n');print({'cases':len(rows),'sha256':H(p.read_bytes())})
