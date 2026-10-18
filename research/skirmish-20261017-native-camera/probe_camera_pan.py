"""Exploratory unchanged7016A0 execution. NOT a frozen proof or runtime ctor.

Heap/presentation parameter fields are explicit lab inputs; no native malloc
zeroing guarantee is inferred. Failures are retained as diagnostics.
"""
from pathlib import Path
import sys,json,struct
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE));import native_camera_pose as P;R=P.R
from unicorn.x86_const import UC_X86_REG_EIP
from unicorn import UC_HOOK_MEM_WRITE
def probe(indexword=0,renderlists=0,publish_ui_reset=False,image="original",precision="Single24",details=False,millis=0,direction=(0,0)):

 hw=R.Hardware();m=P.Machine(image,precision,hw)
 try:
  m.seams={};m.observers={};m.actual={};m.trace=[];m.pose=None;writes=[]
  def written(u,access,address,size,value,_):writes.append((address,size,value,u.reg_read(UC_X86_REG_EIP)))
  m.u.hook_add(UC_HOOK_MEM_WRITE,written)
  def alloc(x):
   size=x.arg(1);p=x.alloc(size)
   if size==0x394:x.camera_allocation=p;x.w(p+0x204,indexword)
   x.ret(p)
  #Only allocation is a transport. Native constructor bodies execute.
  m.seam(0x6f7810,alloc)
  m.seam(0x6f7430,lambda x:x.ret())
  m.w(0x9f01e4,m.stub(lambda x:x.ret(17)))
  m.w(0x9f01f0,m.stub(lambda x:x.ret(0,4)))
  m.w(0x9f01f4,m.stub(lambda x:x.ret(0,4)))
  renderer=m.alloc(0x300);device=m.alloc(0x100);m.w(renderer+0x40,device);m.w(renderer+0x24c,2);m.w(0xb36510,renderer)
  table=m.alloc(0x400);m.w(0xb39990,table)
  for off,word in R.AVARS.items():p=m.alloc(4);m.w(p,word);m.w(table+off,p)
  color=m.alloc(0x14)
  for i,v in enumerate([0,R.f32(.2),R.f32(.4),R.f32(.6)]):m.w(color+4*i,v)
  m.w(table+0xc0,color)
  uvars=m.alloc(0x400);m.w(0xb39940,uvars)
  for off,value in[(0xa8,256),(0xac,128),(0xb4,0),(0xf8,0),(0x288,0)]:p=m.alloc(4);m.w(p,value);m.w(uvars+off,p)
  for off,word in [(0x28,R.f32(10)),(0x18,R.f32(2))]:
   var=m.alloc(4);m.w(var,word);m.w(uvars+off,var)
  #ATL CString storage transport; all actual CString method bodies run.
  def data(chars,size=2):
   d=m.alloc(16+(chars+1)*size);m.w(d,mgr);m.w(d+4,0);m.w(d+8,chars);m.w(d+12,1);return d
  mgr=m.alloc(4);vt=m.alloc(20);m.w(mgr,vt)
  def mgr_alloc(x):x.ret(data(x.arg(1),x.arg(2)),8)
  def mgr_free(x):x.ret(0,4)
  def mgr_realloc(x):
   old=x.arg(1);n=x.arg(2);size=x.arg(3);d=data(n,size);length=x.r(old+4);x.u.mem_write(d+16,bytes(x.u.mem_read(old+16,(length+1)*size)));x.w(d+4,length);x.ret(d,12)
  nil=data(0)
  for i,fn in enumerate((mgr_alloc,mgr_free,mgr_realloc,lambda x:x.ret(nil),lambda x:x.ret(mgr))):m.w(vt+4*i,m.stub(fn))
  m.seam(0x856370,lambda x:x.ret(mgr))
  def text(p):
   out=[]
   for i in range(1000):
    w=bytes(m.u.mem_read(p+2*i,2))
    if w==b'\0\0':return bytes().join(out).decode('utf-16le')
    out.append(w)
   raise AssertionError('unterminated label')
  #Actual labels are ASCII; conversion uses this Windows host's real API.
  import ctypes
  kernel=ctypes.WinDLL('kernel32',use_last_error=True)
  m.w(0xb279e8,m.stub(lambda x:x.ret(1252)))
  def mbwc(x):
   src=x.arg(3);length=x.arg(4);dest=x.arg(5);capacity=x.arg(6);raw=bytearray()
   for j in range(1000):
    byte=x.rb(src+j)
    if length==0xffffffff and byte==0:raw.append(0);break
    raw.append(byte)
    if length!=0xffffffff and len(raw)==length:break
   assert all(c<128 for c in raw),'non-ASCII laboratory label'
   if dest:
    output=(ctypes.c_uint16*capacity)();count=kernel.MultiByteToWideChar(x.arg(1),x.arg(2),ctypes.c_char_p(bytes(raw)),ctypes.c_int(length if length!=0xffffffff else -1),output,capacity);x.u.mem_write(dest,bytes(output))
   else:count=kernel.MultiByteToWideChar(x.arg(1),x.arg(2),ctypes.c_char_p(bytes(raw)),ctypes.c_int(length if length!=0xffffffff else -1),None,0)
   x.ret(count,24)
  m.w(0xb3647c,m.stub(mbwc))
  vars={};var_reads=[]
  def lookup(x):
   name=text(x.arg(1));var_reads.append([name,x.arg(2)])
   if name not in vars:
    var=x.alloc(0x2c);x.w(var+0x24,1);x.w(var+0x28,0);x.w(var,1);vars[name]=var
   x.ret(vars[name],8)
  m.seam(0x4b2c30,lookup)
  terrain_reads=[]
  def terrain(x):
   at=x.u.reg_read(UC_X86_REG_EIP)
   if at==0x43d050:terrain_reads.append([hex(at),[x.r(x.arg(1)),x.r(x.arg(1)+4),x.arg(2)]])
   else:terrain_reads.append([hex(at),[x.arg(1),x.arg(2)]])
   x.ret_st0_f32(R.f32(12.5),8)
  m.seam(0x43d050,terrain);m.seam(0x550210,terrain)
  m.w(0xb36518,0);camera=m.alloc(0x394);m.w(camera+0x204,indexword);m.w(0xb4a998,1)
  if publish_ui_reset:
   m.seam(0x96cab0,lambda x:x.ret());m.call(0x9e3500)
   result=m.call(0x702380,0,(renderlists,),pop=0);camera=m.camera_allocation;assert result==camera and m.r(camera+0x204)==0
   m.call(0x700e30,camera)
  else:result=m.call(0x7016a0,camera,(renderlists,));assert result==camera
  target=[]
  def capture(x):target.extend([x.arg(1),x.arg(2)]);x.ret(0,8)
  m.seam(0x6ff130,capture);vec=m.alloc(8)
  for i,w in enumerate(direction):m.w(vec+4*i,w)
  m.call(0x700500,camera,(millis,vec))
  root=m.r(camera+0x164);shape=m.r(camera+0x370)
  lists=m.r(camera+0x374);descriptors=[]
  if lists:
   for i in range(m.r(lists+4)):
    desc=m.r(m.r(lists)+i*4);pairs=[];node=m.r(desc+0x10)
    while node:pairs.append([m.r(node),m.r(node+4)]);node=m.r(node+8)
    descriptors.append(dict(label=text(m.r(desc+8)),sort=m.r(desc+12),varname=next((name for name,p in vars.items()if p==m.r(desc+0x1c)),None),pairs=pairs))
  def known(base,size):
   masks=[0]*(size//4)
   for address,count,value,at in writes:
    for a in range(max(address,base),min(address+count,base+size)):
     index=(a-base)//4;masks[index]|=0xff<<(8*((a-base)&3))
   return masks
  return dict(pan_target=target,basis=[m.r(camera+0x114),m.r(camera+0x118)],center=[m.r(camera+0x24c),m.r(camera+0x250)],camera_known=known(camera,0x394),root_known=known(root,0x100),shape_known=known(shape,0x2c),instructions={hex(a):b for a,b in sorted(m.actual.items())} if details else None,hardware_transports=m.trans if details else None,passed=True,terrain_reads=terrain_reads,descriptors=descriptors,var_reads=var_reads,camera_words=[m.r(camera+i*4)for i in range(0x394//4)],root_words=[m.r(root+i*4)for i in range(0x100//4)],shape_words=[m.r(shape+i*4)for i in range(0x2c//4)],pins=len(m.actual),heap_allocated=m.heap-0x20000000)
 except Exception as e:
  return dict(passed=False,error=repr(e),pc=hex(m.u.reg_read(UC_X86_REG_EIP)),trace=m.trace[-12:],last_pins=list(map(hex,m.actual))[-12:])
 finally:hw.close()
if __name__=='__main__':
 result=probe();(HERE/'constructor-probe-01.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({k:v for k,v in result.items()if k not in('camera_words','root_words','shape_words')}));assert result['passed']
