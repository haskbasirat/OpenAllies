"""Original4FBA50 replacement body and4FC4A0 stable drain order.
Removal4F53D0,creation4F9C10/4FA590,property4CEF10,diagnostics/UI andCString
storage are named laboratory children. Their successful answers are explicit
inputs, not real world destruction/creation parity. Reached child cutoffs keep
the full earlier native prefix. Original/v14 execute atPC24/53/64.
"""
from pathlib import Path
import sys,json,struct,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/development10/creator-labels'))
from native_creator_labels import Machine,PE0,PE1,PC
from unicorn.x86_const import UC_X86_REG_EIP
from unicorn import UC_HOOK_MEM_READ
class Cutoff(Exception):pass
class Replacement(Machine):
 def __init__(self,image):
  self.replacement_ready=False;super().__init__(image)
  self.u.hook_add(UC_HOOK_MEM_READ,self.read)
 def event(self,name,**fields):
  self.trace.append(dict(call=name,**fields))
  if self.cutoff==name:raise Cutoff()
 def read(self,u,access,address,size,value,user):
  if self.replacement_ready and address==self.old_definition+0x16d:
   self.event('announce',value=self.announce)
 def hook(self,u,pc,size,user):
  if self.replacement_ready:
   if pc==self.vb4:
    assert self.ecx()==self.old;self.event('b4',value=self.b4);self.ret(value=int(self.b4));return
   if pc==self.v160:
    assert self.ecx()==self.old and self.arg()==0;self.event('virtual160',argument=0);self.ret(4);return
   if pc==0x4f53d0:
    assert self.ecx()==self.world and self.arg()==self.old and self.arg(1)==0
    self.event('remove',source=self.source,argument=0);self.ret(8);return
   if pc in [0x4f9c10,0x4fa590]:
    assert self.ecx()==self.world and self.arg()==self.params
    self.event('create_actor'if pc==0x4fa590 else'create_object',requested=self.r(self.params+0x24))
    self.ret(4,self.created if self.success else 0);return
   if pc==0x4cef10:
    assert self.ecx()==self.created and self.arg(1)==2 and self.arg(2)==self.created
    self.event('property',definition=self.properties_by_ptr[self.arg()],kind=2,source='new_actor')
    self.ret(12);return
   if pc in [0x404f90,0x404f30]:
    self.event('created_log'if pc==0x404f90 else'failure_log')
    self.ret();return
   if pc==0x4b9f40:
    self.event('failure_text');self.cs(self.arg(),'failure %s at %g,%g');self.ret(8,self.arg());return
   if pc==0x405d60:
    self.event('failure_format');self.cs(self.arg(),'explicit laboratory failure text');self.ret(value=self.arg());return
   if pc==0x4e4bc0:
    self.event('failure_ui');self.ret();return
  super().hook(u,pc,size,user)
def run(m,pc,c):
 m.fpu(pc);m.actual={};m.record_actual=True;m.replacement_ready=True;m.trace=[];m.cutoff=c.get('cutoff')
 m.world=m.alloc(0x300);m.w(0xb36534,m.world)
 m.source=c.get('source',0x10007);m.old=m.alloc(0x200);vt=m.alloc(0x170);m.w(m.old,vt)
 m.vb4=m.alloc(16);m.v160=m.alloc(16);m.u.mem_write(m.vb4,b'\xc3');m.u.mem_write(m.v160,b'\xc3');m.w(vt+0xb4,m.vb4);m.w(vt+0x160,m.v160)
 m.old_definition=m.alloc(0x600);category=m.alloc(0x30);m.w(m.old+4,m.old_definition);m.w(m.old_definition+0x18,category);m.w(category+0xc,c.get('category',31))
 m.announce=c.get('announce',0);m.b(m.old_definition+0x16d,m.announce);m.b4=c.get('b4',True)
 registry=m.alloc(0x60010);m.w(0xb364c0,registry);slot=m.source&0xffff
 live_slot=slot if m.source else 7
 generation=((m.source>>16)+int(c.get('stale',False)))&0xffff if m.source else 1
 m.u.mem_write(registry+4+live_slot*2,struct.pack('<H',generation))
 m.w(registry+0x20004+live_slot*4,0 if c.get('missing',False)else m.old)
 m.w(m.old+0x14,(generation<<16)|live_slot)
 assert m.r(registry+0x20004)==0
 if not c.get('missing',False):assert m.r(m.old+0x14)==((generation<<16)|live_slot)
 request=m.alloc(0x90);m.b(request,int(c.get('actor',True)));m.w(request+0x80,m.source)
 m.params=request+(4 if c.get('actor',True)else 0x58)
 d=m.alloc(0x600);m.w(m.params+4,d);m.cs(d+8,'replacement');m.cs(d+0x10,'Replacement')
 m.w(m.params+8,0x3f800000);m.w(m.params+0xc,0xc0000000);m.w(m.params+0x24,m.source)
 m.properties_by_ptr={0:None};properties=[]
 for value in c.get('properties',[11,None,11]):
  if value is None:properties.append(0)
  else:
   ptr=m.alloc(4);m.properties_by_ptr[ptr]=value;properties.append(ptr)
 array=m.alloc(max(4,4*len(properties)))
 for i,p in enumerate(properties):m.w(array+4*i,p)
 m.w(request+0x84,array);m.w(request+0x88,len(properties))
 m.created=m.alloc(0x200);m.success=c.get('success',True)
 error=None;returned=None
 try:returned=m.call(0x4fba50,[request],callee_pop=4)&255
 except Cutoff:error=m.cutoff
 return dict(returned=returned,error=error,trace=m.trace,source_still_in_registry=m.r(registry+0x20004+slot*4)==m.old,properties_unchanged=[m.properties_by_ptr[m.r(array+4*i)]for i in range(len(properties))]),m.actual.copy()
def main():
 cases=[{},dict(actor=False),dict(source=0),dict(stale=True),dict(missing=True),dict(b4=False),dict(announce=1),dict(announce=128),dict(category=0),dict(category=32),dict(category=33),dict(category=34),dict(category=35),dict(category=36),dict(properties=[]),dict(success=False),dict(actor=False,success=False)]
 cases += [dict(cutoff=step)for step in ['b4','announce','remove','create_actor','property','created_log']]
 cases += [dict(announce=1,cutoff='virtual160')]
 cases += [dict(success=False,cutoff=step)for step in ['failure_log','failure_text','failure_format','failure_ui']]
 rows=[];pins={}
 for image in ['original','v14']:
  m=Replacement(image)
  for pc in PC:
   for case in cases:
    output,visited=run(m,pc,case);pins.update(visited);rows.append(dict(image=image,precision=pc,input=case,output=output))
 a=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original'];b=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14'];assert a==b
 instructions=[]
 for va,size in sorted(pins.items()):
  if PE0.base<=va<PE0.base+PE0.image_size:
   raw=PE0.range(va,size);assert raw==PE1.range(va,size);instructions.append(dict(va=hex(va),bytes=raw.hex()))
 doc=dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,instructions=instructions)
 p=HERE/'native-replacement-drain.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps(dict(rows=len(rows),executed_pins=len(instructions),sha256=hashlib.sha256(p.read_bytes()).hexdigest())))
if __name__=='__main__':main()
