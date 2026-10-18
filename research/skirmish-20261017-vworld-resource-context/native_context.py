"""Real5AE0A0/5ACA50 and actual constant Map/Film/Resource/Ni class getters.

Only CString assignment is a transport seam; CRT96CB83 and bucket traversal
4245A0 execute. Eligible Ni unload stops BEFORE5ABFF0, not successful unload.
Descriptor+8 parent stores execute40977F..4097A8 from static-CRT immediate
inputs (pinned below), not a claimed complete CRT class registration. Biome
source getter is an explicit TGI transport at5B54D0; actual+98 store executes.
"""
from pathlib import Path
import hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn.x86_const import *
from unicorn import UC_HOOK_MEM_READ
class Required(Exception):pass
VT={'Map':0xa44008,'Film':0xa44130,'Resource':0xa467e0,'Ni':0xa45c98}
PARENTS=[(0xb38e24,0xb38e24,0x9c7fe0),(0xb3d7dc,0xb38e24,0x9d0340),(0xb3d7f0,0xb3d7dc,0x9d0370),(0xb3d968,0xb38e24,0x9d0680),(0xb3d990,0xb3d968,0x9d06e0)]
class Machine(h.Machine):
 def __init__(self,pc,image):
  super().__init__(pc)
  if image is not h.P:
   original_end=(h.P.image_size+4095)&~4095;image_end=(image.image_size+4095)&~4095
   if image_end>original_end:self.u.mem_map(image.base+original_end,image_end-original_end)
   self.u.mem_write(image.base,image.b[:4096])
   for vs,r,raw,off in image.sections:
    if raw:self.u.mem_write(image.base+r,image.b[off:off+min(raw,max(vs,raw))])
  self.events=[];self.prefixes={};self.guards=[];self.running=False;self.classified=[]
  self.u.hook_add(UC_HOOK_MEM_READ,self.read_guard)
  for descriptor,parent,crt in PARENTS:
   #These exact descriptor/parent immediates are independently pinned below.
   self.w32(h.STACK+0x28,parent);self.w32(h.STACK+0x2c,0xb38e08);self.w32(h.STACK+0x30,0)
   self.u.reg_write(UC_X86_REG_ESP,h.STACK);self.u.reg_write(UC_X86_REG_ESI,descriptor)
   self.u.emu_start(0x40977f,0x4097a8,count=100)
   assert self.r32(descriptor+8)==(0 if descriptor==parent else parent)
  self.seams[0x405b80]=self.assign
  self.seams[0x5abff0]=lambda _:self.required('Ni unload5ABFF0')
 def _hook(self,u,pc,size,user):
  if getattr(self,'running',False) and pc==0x5aca8b:
   self.classified.append(next(kind for address,kind in self.guards if address==self.ecx()))
  super()._hook(u,pc,size,user)
 def required(self,name):raise Required(name)
 def text(self,units):
  data=self.alloc(16+2*(len(units)+1));self.w32(data,0,len(units),len(units),1)
  self.u.mem_write(data+16,struct.pack('<'+'H'*(len(units)+1),*units,0));self.prefixes[data+16]=list(units);return data+16
 def assign(self,_):
  source=self.arg();dest=self.ecx();self.w32(dest,self.r32(source));self.events.append('assign_counted_string');self.ret(4,dest)
 def read_guard(self,u,access,a,size,value,_):
  if self.running:
   for p,kind in self.guards:
    if kind in ('Map','Film','Resource') and p+4<=a<p+12:raise AssertionError('non-Ni flags/count were read')
 def run_context(self,case):
  manager=self.alloc(0x130);arg=self.alloc(4)
  self.w32(manager+0x80,self.text(case['current']));self.w32(arg,self.text(case['next']))
  buckets=self.alloc(5*4);self.w32(manager+0x60,buckets,5,len(case['resources']))
  #Deliberately nontrivial bucket/chain layout; raw linked nodes are real
  #native hash iterator fields. Class pointer constants are actual vtables.
  for i,item in reversed(list(enumerate(case['resources']))):
   p=self.alloc(0xb4);self.w32(p,VT[item['class']],item['count'],item['flags']);self.guards.append((p,item['class']))
   bucket=(i*2+1)%5;node=self.alloc(8);self.w32(node,p,self.r32(buckets+bucket*4));self.w32(buckets+bucket*4,node)
  if case.get('stop_invalidation'):
   self.seams[0x5aca50]=lambda _:self.required('invalidation5ACA50')
  self.running=True;stop=None
  try:self.call(0x5ae0a0,manager,[arg])
  except Required as e:stop=str(e)
  self.running=False
  return {'current':self.prefixes[self.r32(manager+0x80)],'events':self.events,'stop':stop,'examined':self.classified}
 def constructor(self):
  manager=self.alloc(0x130);allocator=self.alloc(4);vt=self.alloc(16);self.w32(allocator,vt)
  stub=h.STUBS+0x100;self.w32(vt+12,stub)
  def string_manager(_):
   assert self.arg()==2;self.ret(value=allocator)
  self.seams[0x856370]=string_manager
  self.seams[stub]=lambda _:self.ret(value=self.text([])-16)
  self.u.reg_write(UC_X86_REG_ESP,h.STACK);self.u.reg_write(UC_X86_REG_ESI,manager)
  self.u.emu_start(0x5b04ea,0x5b0509,count=100)
  return {'current':self.prefixes[self.r32(manager+0x80)]}
 def source(self,units):
  data=self.alloc(0xc0);node=self.alloc(4);value=self.alloc(4);self.w32(value,self.text(units))
  def getter(_):
   assert self.arg()==0xa1ffec
   assert h.P.range(0xa1ffec,46).decode('utf16').split('\0')[0]=='texture_subfolder_name'
   self.events.append('typed_source_texture_subfolder_name');self.ret(4,value)
  self.seams[0x5b54d0]=getter
  self.u.reg_write(UC_X86_REG_ESP,h.STACK);self.u.reg_write(UC_X86_REG_EBP,data);self.u.reg_write(UC_X86_REG_ESI,node)
  self.u.emu_start(0x4b8eac,0x4b8ec4,count=1000)
  return {'stored':self.prefixes[self.r32(data+0x98)],'events':self.events}
def cases():
 def row(cur,nxt,res=(),**kw):return dict(current=cur,next=nxt,resources=list(res),**kw)
 def res(kind,flags=0xffffffff,count=0xffffffff):return {'class':kind,'flags':flags,'count':count}
 for cur,nxt in [([],[]),([],[65]),([65],[65]),([65],[97]),([65,0,88],[65,0,89]),([65,0,88],[66,0,89])]:
  yield row(cur,nxt)
  yield row(cur,nxt,[res('Map'),res('Film'),res('Resource'),res('Map',0,0)])
 for flags,count in [(0,0),(1,0),(32,1),(32,0),(33,0),(32,0xffffffff),(32,0x80000000),(32,0x7fffffff)]:
  yield row([],[84],[res('Map'),res('Ni',flags,count),res('Film')])
 yield row([],[88],[res('Map')],stop_invalidation=True)
 yield row([88],[88],[res('Ni',32,0)],stop_invalidation=True)
def main():
 rows=[];sources=[];constructors=[]
 for pc in h.PRECISION:
  a=Machine(pc,h.P).constructor();b=Machine(pc,h.V).constructor();assert a==b
  constructors.append(dict(precision=pc,output=a))
 for case in cases():
  for pc in h.PRECISION:
   outputs=[]
   for image in [h.P,h.V]:outputs.append(Machine(pc,image).run_context(case))
   assert outputs[0]==outputs[1];rows.append(dict(input=case,precision=pc,output=outputs[0]))
 for value in [[],list(map(ord,'Arctic')),list(map(ord,'Tropical')),list(map(ord,'Desert'))]:
  for pc in h.PRECISION:
   a=Machine(pc,h.P).source(value);b=Machine(pc,h.V).source(value);assert a==b
   sources.append(dict(input=value,precision=pc,output=a))
 static=[]
 for _,_,crt in PARENTS:
  raw=h.P.range(crt,42);assert raw==h.V.range(crt,42);static.append(dict(address=hex(crt),bytes=raw.hex(),sha256=hashlib.sha256(raw).hexdigest()))
 executed=[dict(address=hex(a),bytes=b)for a,b in sorted(h.VISITED.items())if a not in (0x405b80,0x5abff0,0x5aca50,0x5b54d0,0x856370)]
 #5ACA50 executes in ordinary rows and is also a stopped boundary in one.
 if 0x5aca50 in h.VISITED:executed.append(dict(address=hex(0x5aca50),bytes=h.VISITED[0x5aca50]))
 out=dict(schema=1,scope=__doc__,original_sha256=h.ORIGINAL_SHA,v14_sha256=h.V14_SHA,cases=rows,source_rows=sources,constructor_rows=constructors,descriptor_inputs=[dict(descriptor=hex(a),parent=hex(b),crt=hex(c))for a,b,c in PARENTS],static_crt=static,executed_address_byte_pins=sorted(executed,key=lambda r:int(r['address'],16)),intercepted_entries=['0x405b80','0x5abff0','0x5b54d0','0x856370'],sometimes_stopped_entry='0x5aca50')
 path=HERE/'native-context.json';raw=(json.dumps(out,indent=1)+'\n').encode()
 if '--write'in sys.argv:path.write_bytes(raw)
 else:assert path.read_bytes()==raw
 print(json.dumps(dict(rows=len(rows),source_rows=len(sources),machines=(len(rows)+len(sources)+len(constructors))*2,executed_pins=len(executed),sha256=hashlib.sha256(raw).hexdigest())))
if __name__=='__main__':main()
