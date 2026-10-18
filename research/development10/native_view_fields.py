"""Actual VThing45C1E0/VObject455050 constructor and common Init45B530.

Raw supplied global/source/Thing words are laboratory owners, not a claim that
the application seed producer or full VActor/Organization Init is bound. Init
virtual8C, Occupy42FD10, Data virtual14 and request43D030 are named service
leaves, including explicit pre-call cutoffs. GPU upload is not an Init leaf.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(HERE/'creator-labels'))
from native_creator_labels import Machine,PE0,PE1,PC
from unicorn import UC_HOOK_MEM_READ
from unicorn.x86_const import UC_X86_REG_EIP
VIRTUAL=0x7700c00;PRE=VIRTUAL+16
class Stop(Exception):pass
class Fields(Machine):
 def __init__(self,image):
  self.fields_ready=False;super().__init__(image);self.u.hook_add(UC_HOOK_MEM_READ,self.read)
 def read(self,u,access,address,size,value,user):
  if self.fields_ready:
   if address<=self.uvar<address+size:self.events.append('read-uvar108')
   if address<=0xb39930<address+size:self.events.append('read-global-request-mode')
   if address<=self.world+0xac<address+size:self.events.append('read-vworld-ac')
 def hook(self,u,pc,size,user):
  if self.fields_ready and pc in [VIRTUAL,0x42fd10,PRE,0x43d030]:
   name={VIRTUAL:'virtual8C',0x42fd10:'occupy',PRE:'source-pre-request',0x43d030:'request-queue'}[pc]
   self.events.append(name)
   if self.cutoff==name:raise Stop()
   if pc==PRE:
    assert self.ecx()==self.data and self.arg()==1 and self.arg(1)==0
    self.ret(8);return
   if pc==0x42fd10:
    assert self.ecx()==self.grid and self.arg()==self.view
    self.ret(4);return
   if pc==0x43d030:
    assert self.ecx()==self.world and self.arg()==self.view
    self.ret(4);return
   assert self.ecx()==self.view,(name,hex(self.ecx()),hex(self.view))
   self.ret();return
  super().hook(u,pc,size,user)

COMMON=[4,8,12,0x18,0x1c,*range(0x20,0x78,4)]
OBJECT=[*range(0x78,0xbc,4)]
def snapshot(m,offsets):return {hex(o):m.r(m.view+o)for o in offsets}
def run(m,precision,c):
 m.fields_ready=False;m.fpu(precision);m.actual={};m.record_actual=True
 m.view=m.alloc(0x220);m.u.mem_write(m.view,b'\xa5'*0x220)
 m.data=m.alloc(0x600);thing=m.alloc(0x40);m.world=m.alloc(0x400)
 sourcevt=m.alloc(0x150);m.w(sourcevt+0x14,PRE);m.w(m.data,sourcevt);m.w(m.data+0x1c,c['source_flags'])
 m.grid=m.alloc(0x110);m.w(m.world+0x94,m.grid)
 m.w(0xb36534,m.world);m.w(0xb36538,m.world);m.w(m.world+0xe0,c['time']);m.b(m.world+0xac,c['ac'])
 for a,v in zip([0xb43cf8,0xb43cfc,0xb43d00],c['seed']):m.w(a,v)
 game=m.alloc(0x100);vector=m.alloc(16);m.w(0xb39990,game);m.w(game+0x64,vector)
 for i,v in enumerate(c['default_facing']):m.w(vector+4+4*i,v)
 m.w(thing+8,c['thing_flags'])
 for off,v in zip([0x20,0x24,0x28],c['position']):m.w(thing+off,v)
 for off,v in zip([0x2c,0x30,0x34],c['facing']):m.w(thing+off,v)
 m.call(0x455050 if c['object'] else 0x45c1e0,[m.data,thing if c['thing'] else 0],this=m.view,callee_pop=8)
 ctor=snapshot(m,COMMON+(OBJECT if c['object']else []))
 assert m.r(m.view+0x10)==m.data and m.r(m.view+0x14)==(thing if c['thing']else 0)
 vt=m.alloc(0x150);m.w(vt+0x8c,VIRTUAL);m.w(m.view,vt)
 m.w(m.view+0x54,c['initial_flags'])
 manager=m.alloc(0x200);m.uvar=m.alloc(4);m.w(0xb39940,manager);m.w(manager+0x1b0,m.uvar);m.b(m.uvar,c['uvar']);m.b(0xb39930,c['mode'])
 m.events=[];m.cutoff=c.get('cutoff');m.fields_ready=True;error=None;result=None
 try:result=m.call(0x45b530,this=m.view)&0xff
 except Stop:error=m.events[-1]
 return dict(constructor=ctor,after=snapshot(m,COMMON),events=m.events,error=error,result=result),m.actual.copy()

def main():
 base=dict(object=True,thing=True,source_flags=0,thing_flags=0,initial_flags=0x1000000,seed=[0,0x80000000,0x7fc12345],
  default_facing=[0x3f800000,0,0x80000000],time=0x12345678,position=[0x41200000,0xc1200000,0x7fc00001],
  facing=[0,0xbf800000,0],uvar=0,mode=0,ac=0)
 cases=[]
 for obj in [False,True]:
  for thing in [False,True]:
   for source in [0,1<<21]:
    for flags in [0,1<<12]:cases.append(dict(base,object=obj,thing=thing,source_flags=source,thing_flags=flags))
 for uvar in [0,1,128]:
  for mode in [0,1]:
   for ac in [0,1]:cases.append(dict(base,uvar=uvar,mode=mode,ac=ac))
 cases += [dict(base,initial_flags=(1<<4)|0x1000000),dict(base,initial_flags=0xffffffff)]
 cases += [dict(base,uvar=1,cutoff=x)for x in ['virtual8C','occupy','source-pre-request','request-queue']]
 rows=[];pins={}
 for image in ['original','v14']:
  m=Fields(image)
  for precision in PC:
   for c in cases:
    output,actual=run(m,precision,c);pins.update(actual);rows.append(dict(image=image,precision=precision,input=c,output=output))
 assert [{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original']==[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14']
 instructions=[]
 for va,size in sorted(pins.items()):
  if PE0.base<=va<PE0.base+PE0.image_size:
   raw=PE0.range(va,size);assert raw==PE1.range(va,size);instructions.append(dict(va=hex(va),bytes=raw.hex()))
 doc=dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,instructions=instructions)
 out=HERE/'native-view-fields.json'
 if '--write'in sys.argv:out.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(out.read_text())==doc
 print(json.dumps(dict(rows=len(rows),pins=len(instructions),sha256=hashlib.sha256(out.read_bytes()).hexdigest())))
if __name__=='__main__':main()
