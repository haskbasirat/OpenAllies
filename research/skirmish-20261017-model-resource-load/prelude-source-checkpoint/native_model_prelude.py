"""Actual unprocessed87AA10 hidden-tree/property/effect/bound preludes.

The tree is an explicit native-layout fixture, not a NIF loader. Actual RTTI,
8C72D0/8C7190 hidden-node mutation, empty-controller/skin checks, own string
keep markers, property-state/effect propagation and root virtual38 execute.
Current-thread/critical-section calls and allocator/string storage are declared
transports; default NiProperty globals are explicit inputs, not startup claims.
The footprint caller fragment87AC5B..87ACAB executes with878790 intercepted to
record the exact height operand and already-zeroed output prefix, including NaN.
"""
from pathlib import Path
import sys,json,struct,hashlib,itertools
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE))
import native_model_base_texture as h
from unicorn.x86_const import *
from unicorn import UC_HOOK_MEM_READ

def imported(pe,name):
 nt=struct.unpack_from('<I',pe.b,0x3c)[0];rva=struct.unpack_from('<I',pe.b,nt+24+104)[0];rows=[]
 while True:
  original,stamp,chain,dll,first=struct.unpack('<5I',pe.range(pe.base+rva,20))
  if not any([original,stamp,chain,dll,first]):break
  i=0
  while True:
   value=struct.unpack('<I',pe.range(pe.base+(original or first)+4*i,4))[0]
   if value==0:break
   if not value&0x80000000:
    at=pe.base+value+2;raw=bytearray()
    while pe.range(at,1)!=b'\0':raw.extend(pe.range(at,1));at+=1
    if raw==name.encode():rows.append(pe.base+first+4*i)
   i+=1
  rva+=20
 assert len(rows)==1,(name,rows)
 return rows[0]
def f(value):return struct.unpack('<I',struct.pack('<f',value))[0]
I=[f(1),0,0,0,f(1),0,0,0,f(1),0,0,0,f(1)]

class Machine(h.Machine):
 def __init__(self,image):
  super().__init__(image);self.prelude_ready=False;self.events=[];self.data_reads={};self.pe=h.PE0 if image=='original'else h.PE1
  nt=struct.unpack_from('<I',self.pe.b,0x3c)[0];count=struct.unpack_from('<H',self.pe.b,nt+6)[0];table=nt+24+struct.unpack_from('<H',self.pe.b,nt+20)[0];self.rdata=[]
  for i in range(count):
   at=table+i*40;name=self.pe.b[at:at+8].split(b'\0')[0];r,size=struct.unpack_from('<II',self.pe.b,at+12)
   if name==b'.rdata':self.rdata.append((self.pe.base+r,self.pe.base+r+size))
  assert self.rdata
  self.read_hook=self.u.hook_add(UC_HOOK_MEM_READ,self.read)
  self.os_stubs={}
  for i,name in enumerate(['GetCurrentThreadId','EnterCriticalSection','LeaveCriticalSection']):
   at=self.alloc(16);self.os_stubs[at]=name
   self.w(imported(h.PE0,name),at)
  for descriptor in [0xb43ce8,0xb45f6c]:self.init_type(descriptor)
 def read(self,u,access,address,size,value,user):
  if not any(start<=address and address+size<=end for start,end in self.rdata):return
  actual=bytes(u.mem_read(address,size));source=self.pe.range(address,size)
  #Patched imported function pointers are declared hostcall transports, not
  #immutable source-data pins. All matching original-image reads are retained.
  if actual==source:self.data_reads[(address,size)]=actual.hex()
 def hook(self,u,pc,size,user):
  if pc in getattr(self,'os_stubs',{}):
   name=self.os_stubs[pc];self.events.append(name)
   if name=='GetCurrentThreadId':self.ret(value=0x1234)
   else:self.ret(4)
   return
  if getattr(self,'prelude_ready',False)and pc==0x878790:
   self.events.append(dict(footprint_height=self.arg(3),prefix=[self.r(self.arg(1)),self.r(self.arg(2)),self.r(self.arg(2)+4)]));self.ret();return
  if getattr(self,'prelude_ready',False)and pc==0x87acab:
   u.emu_stop();return
  super().hook(u,pc,size,user)

def make_tree(m,rows):
 pointers=[m.alloc(0x120)for _ in rows]
 for i,c in enumerate(rows):
  p=pointers[i];kind=c.get('kind','node');descriptor={'node':0xb43cd8,'shape':0xb43ce8}[kind]
  m.w(p,h.type_vtable(h.PE0,descriptor));m.w(p+4,100);m.w(p+0xc,m.narrow(c['name']));m.w(p+0x20,c.get('flags',0x10))
  for j,v in enumerate(I):m.w(p+0x38+j*4,v);m.w(p+0x6c+j*4,v)
  for j,v in enumerate([f(i+1),f(i*2),f(-i),f(i+1)]):m.w(p+0x28+4*j,v)
  parent=c.get('parent');m.w(p+0x24,0 if parent is None else pointers[parent])
  children=c.get('children',[]);array=m.alloc(max(1,len(children))*4);m.w(p+0xc8,array);m.w(p+0xcc,len(children));m.w(p+0xd0,len(children));m.w(p+0xd4,len(children))
  for j,child in enumerate(children):m.w(array+j*4,0 if child is None else pointers[child])
  if c.get('keep')is not None:
   extra=m.alloc(0x14);m.w(extra,h.type_vtable(h.PE0,0xb45f6c));m.w(extra+4,100);m.w(extra+0x10,m.narrow(c['keep']));array=m.alloc(4);m.w(array,extra);m.w(p+0x14,array);m.w(p+0x18,1)
 return pointers

def snapshot(m,pointers):
 names={p:i for i,p in enumerate(pointers)}
 def logical(p):
  assert p==0 or p in names,hex(p)
  return None if not p else names[p]
 return [dict(parent=logical(m.r(p+0x24)),children=[logical(m.r(m.r(p+0xc8)+4*j))for j in range(m.r(p+0xd0))],flags=m.r(p+0x20),references=m.r(p+4),sphere=[m.r(p+0x28+4*j)for j in range(4)])for p in pointers]

def hidden_cases():
 yield [dict(name='Root',flags=0x11)]
 for name,flags,keep in itertools.product(['ordinary','SelectionBox','selectionbox','Sg'],[0x10,0x11],[None,'fiducial','sgoKeep','NiOptimizeKeep','NiBoneLOD#','SGOKEEP','ordinary']):
  yield [dict(name='Root',children=[1,None]),dict(name=name,kind='shape',flags=flags,keep=keep,parent=0)]
 for keeper in ['ordinary','SelectionBox']:
  yield [dict(name='Root',children=[1,3]),dict(name='hidden parent',flags=0x11,parent=0,children=[2]),dict(name=keeper,kind='shape',flags=0x11,parent=1),dict(name='visible sibling',kind='shape',parent=0)]

def run_hidden(m,case):
 points=make_tree(m,case);owner=m.alloc(8);m.w(owner,0xab7bdc);before=snapshot(m,points)
 try:m.call(0x8c72d0,[points[0]],this=owner,callee_pop=4)
 except Exception as error:raise RuntimeError(('hidden',case,'EIP',hex(m.u.reg_read(UC_X86_REG_EIP)),'tail',[hex(x)for x in list(m.actual)[-12:]]))from error
 assert m.r(owner+4)==points[0]
 return dict(before=before,after=snapshot(m,points))

def run_properties(m):
 #The global defaults are explicit NULL input to this laboratory, not a BSS or
 #application-startup assertion. Actual own MaterialProperty overrides follow.
 for at in [0xb43f3c,0xb442f8,0xb44304,0xb43f48,0xb44334,0xb44310,0xb4431c,0xb442fc,0xb44300,0xb44308,0xb45f18]:m.w(at,0)
 m.w(0xb43e30,0);m.w(0xb43e34,0)
 case=[dict(name='root',children=[1,None,2]),dict(name='child',kind='shape',parent=0),dict(name='empty node',parent=0)]
 points=make_tree(m,case);m.events=[]
 m.call(0x86eab0,this=points[0]);m.call(0x86f090,this=points[0]);m.call(m.r(m.r(points[0])+0x38),this=points[0])
 states=[m.r(p+0xc4)for p in points];assert states[1],states
 fields=[m.r(states[1]+4*j)for j in range(2,14)]
 return dict(state_present=[bool(s)for s in states],state_defaults=fields,state_references=m.r(states[1]+4),lock_thread=m.r(0xb43e30),lock_depth=m.r(0xb43e34),effect_global=m.r(0xb45f18),events=m.events,snapshots=snapshot(m,points))

def run_footprint(m,radial,low):
 metadata=m.alloc(0x28);m.w(metadata+8,radial);m.w(metadata+0x10,low)
 for i in range(3):m.w(metadata+0x14+i*4,0x12345678+i)
 stack=m.stack-0x400;m.w(stack+0x14,0xaaaaaaaa);m.events=[];m.prelude_ready=True
 m.u.reg_write(UC_X86_REG_ESP,stack);m.u.reg_write(UC_X86_REG_EDI,metadata);m.u.reg_write(UC_X86_REG_EBX,0);m.u.reg_write(UC_X86_REG_EBP,m.alloc(4))
 m.u.emu_start(0x87ac5b,0x87acab,count=1000);m.prelude_ready=False
 return dict(events=m.events,footprint=[m.r(metadata+0x14+i*4)for i in range(3)])

def main():
 rows=[];pins={};data={}
 for image in ['original','v14']:
  m=Machine(image)
  for pc in h.PC:
   m.fpu(pc)
   for case in hidden_cases():rows.append(dict(image=image,precision=pc,kind='hidden',input=case,output=run_hidden(m,case)))
   rows.append(dict(image=image,precision=pc,kind='properties',input=dict(global_defaults='explicit NULL',effects='empty'),output=run_properties(m)))
   for radial,low in itertools.product([0,0x80000000,f(-1),f(1),0x7fc01234,0x7f800000],[0,0x80000000,f(-1),f(1),0x7fc01234]):
    rows.append(dict(image=image,precision=pc,kind='footprint_caller',input=dict(radial=radial,min_z=low),output=run_footprint(m,radial,low)))
  pins.update(m.actual)
  for key,value in m.data_reads.items():
   if key in data:assert data[key]==value
   else:data[key]=value
 a=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original'];b=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14'];assert a==b
 instructions=[]
 for at,size in sorted(pins.items()):
  if h.PE0.base<=at<h.PE0.base+h.PE0.image_size:
   raw=h.PE0.range(at,size);assert raw==h.PE1.range(at,size);instructions.append(dict(va=hex(at),bytes=raw.hex()))
 data_pins=[]
 for(address,size),value in sorted(data.items()):
  raw=h.PE0.range(address,size);assert raw==h.PE1.range(address,size)and raw.hex()==value;data_pins.append(dict(va=hex(address),bytes=value))
 out=dict(scope=__doc__,rows=rows,instructions=instructions,immutable_data_reads=data_pins,images=dict(original=hashlib.sha256(h.PE0.b).hexdigest(),v14=hashlib.sha256(h.PE1.b).hexdigest()),transports=['Inherited allocator/CString/free laboratory transports','Win32 GetCurrentThreadId explicit0x1234;Enter/LeaveCriticalSection declared hostcalls','878790 intercepted only for caller fragment arguments/prefix'],producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
 path=HERE/'native-model-prelude-v2.json'
 if '--create'in sys.argv:
  with path.open('x')as output:json.dump(out,output,indent=2);output.write('\n')
 else:assert json.loads(path.read_bytes())==out
 print(json.dumps(dict(rows=len(rows),pins=len(instructions),immutable_data_pins=len(data_pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
