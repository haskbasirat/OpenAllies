"""Original model-path branch/directory and ordinary effective-aspect evidence.
String storage/format/copy allocator leaves are fixture services. No game process.
The aspect fragment starts at46A9AF with caller-owned string refs/registers;
this is not a claim of complete AspectData/source loading or resource acquisition.
"""
from pathlib import Path
import json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine,original,modified,sha
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
RANGES={'normal_assignment':(0x49e323,0x49e35f),'folder_assignment':(0x48d684,0x48d6ae),'folder':(0x85a550,0x85a653),'folder_search':(0x859940,0x8599a0),'aspect_prefix':(0x46a930,0x46aa46),'relative_resource':(0x4763c0,0x476496),'ordinary_aspect':(0x4eaaa0,0x4eab16),'normal_index':(0x49b480,0x49b487),'state_index':(0x45e5e0,0x45e5f6),'state_dispatch':(0x4d0700,0x4d0720)}
for lo,hi in RANGES.values():assert original.range(lo,hi-lo)==modified.range(lo,hi-lo)
class Strings(Machine):
 def __init__(self):
  super().__init__();self.u.mem_map(0,4096);self.reset(0);self.next=self.STATE+0x3000
  self.w(self.STATE+0x1000,self.STATE+0x1100);self.w(self.STATE+0x110c,self.STATE+0x1200)
  self.u.hook_add(UC_HOOK_CODE,self.hook)
 def text(self,p):
  out=bytearray()
  for i in range(4096):
   c=bytes(self.u.mem_read(p+i*2,2))
   if c==b'\0\0':return out.decode('utf-16-le')
   out+=c
  raise ValueError('unterminated')
 def alloc(self,s):
  base=self.next;raw=s.encode('utf-16-le');self.next+=(len(raw)+31)&~15
  self.w(base+4,len(s));self.w(base+12,1000);self.u.mem_write(base+16,raw+b'\0\0');return base+16
 def ref(self,p,s):self.w(p,self.alloc(s))
 def ret(self,pop=0,value=None):
  sp=self.u.reg_read(UC_X86_REG_ESP)
  if value is not None:self.u.reg_write(UC_X86_REG_EAX,value)
  self.u.reg_write(UC_X86_REG_EIP,self.r(sp));self.u.reg_write(UC_X86_REG_ESP,sp+4+pop)
 def hook(self,u,pc,size,_):
  sp=u.reg_read(UC_X86_REG_ESP);cx=u.reg_read(UC_X86_REG_ECX)
  if pc==0x96c26d:self.ret(value=len(self.text(self.r(sp+4))))
  elif pc==0x96de44:
   p=self.r(sp+4);at=self.text(p).rfind(chr(self.r(sp+8)));self.ret(value=0 if at<0 else p+at*2)
  elif pc==0x4098f0:
   dst=self.r(sp+4);self.ref(dst,self.text(self.r(cx))[:self.r(sp+8)]);self.ret(8,dst)
  elif pc==0x4012c0:self.ret(value=self.r(sp+4))
  elif pc==0x401b00:self.ref(cx,self.text(self.r(sp+4)));self.ret(4,cx)
  elif pc==0x405b80:self.w(cx,self.r(self.r(sp+4)));self.ret(4,cx)
  elif pc==0x856370:self.ret(value=self.STATE+0x1000)
  elif pc==self.STATE+0x1200:self.ret(value=self.alloc('')-16)
  elif pc==0x4108c0:
   dst=self.r(sp+4);self.ref(dst,self.text(self.r(self.r(sp+8)))+self.text(self.r(self.r(sp+12))));self.ret(value=dst)
  elif pc==0x405d60:
   dst=self.r(sp+4);fmt=self.text(self.r(sp+8));args=[]
   for i in range(fmt.count('%s')):args.append(self.text(self.r(sp+12+i*4)))
   self.ref(dst,fmt%tuple(args));self.ret(value=dst)
  elif pc==0x424c10:
   self.ref(cx,self.text(self.r(cx)).lstrip(chr(self.r(sp+4))));self.ret(4,cx)

def folders():
 rows=[]
 for path in ['Units/USA/usa_infantry.tgi','Templates/template_actor.tgi','root.tgi','Units\\GBR\\test.tgi']:
  m=Strings();m.ref(m.STATE+0x800,path);m.w(m.STACK+4,m.STATE+0x808);m.w(m.STACK+8,m.STATE+0x800)
  m.u.emu_start(0x85a550,m.STOP,count=10000)
  out=m.text(m.r(m.STATE+0x808));rows.append(dict(path=path,folder=out))
 return rows

def paths():
 rows=[]
 for folder in ['', 'Units/USA','Doodads/Temperate']:
  for resource in ['USAInfantry/USAInfantry','Depot/Depot']:
   for aspect in ['', 'Kneel','/Shared/Infantry']:
    for fmt in ['%s.nif','/Shared/Fixed.nif']:
     m=Strings();aspect_ptr=m.STATE+0x500;data=m.STATE+0x100
     m.ref(data+4,folder);m.w(0xb36530,data);m.ref(aspect_ptr+4,aspect);m.ref(m.STATE+0x700,resource)
     m.ref(m.STACK+0x10,fmt);m.ref(m.STACK+0x34,'');m.u.reg_write(UC_X86_REG_EDI,aspect_ptr+4);m.u.reg_write(UC_X86_REG_ESI,aspect_ptr);m.u.reg_write(UC_X86_REG_EBP,m.STATE+0x700)
     #Fragment starts after temporary empty-string allocation at46A9A8..AB.
     m.u.emu_start(0x46a9af,0x46aa2c,count=10000)
     out=m.text(m.r(m.STACK+0x34));rows.append(dict(folder=folder,resource_prefix=resource,aspect_prefix=aspect,model=fmt,result=out))
 return rows

def selection():
 rows=[]
 for is_actor in [0,1]:
  for variations in [0,1]:
   for supports_variant in [0,1]:
    for variant in [-1,0,2]:
     for state in [None,1]:
      m=Machine();m.reset(0);obj=m.STATE;data=obj+0x500;vtable=obj+0x900;dv=obj+0xa00
      m.w(obj,vtable);m.w(obj+4,data);m.w(obj+0x40,variant);m.w(obj+0x100,0);m.w(data,dv);m.u.mem_write(data+0x108,bytes([variations]));m.w(data+0x118,4)
      m.w(vtable+0xb4,obj+0xb00);m.u.mem_write(obj+0xb00,b'\xb8'+struct.pack('<I',is_actor)+b'\xc3')
      m.w(dv+0x40,obj+0xb10);m.u.mem_write(obj+0xb10,b'\xb8'+struct.pack('<I',supports_variant)+b'\xc3')
      m.w(obj+0x15c,0);m.w(data+0x188,obj+0xc00);m.w(obj+0xc00,obj+0xc10);m.w(obj+0xc18,obj+0xc20);m.w(obj+0xc28,1)
      def hook(u,pc,size,_):
       if pc==0x4cf100:
        sp=u.reg_read(UC_X86_REG_ESP);u.reg_write(UC_X86_REG_EAX,int(state is not None));u.reg_write(UC_X86_REG_EIP,m.r(sp));u.reg_write(UC_X86_REG_ESP,sp+4)
      m.u.hook_add(UC_HOOK_CODE,hook);m.u.emu_start(0x4eaaa0,m.STOP,count=10000)
      rows.append(dict(is_actor=bool(is_actor),variations=bool(variations),supports_variant=bool(supports_variant),variant=variant,state_aspect=state,normal_index=4,result=m.u.reg_read(UC_X86_REG_EAX)))
 return rows
if __name__=='__main__':
 out={'format':'aa-native-live-model-v1','original_sha256':sha(original.b),'windows':{name:{'start':hex(lo),'end':hex(hi),'sha256':sha(original.range(lo,hi-lo))}for name,(lo,hi)in RANGES.items()},'seams':['String allocation/copy/format/substr/CRT length leaves supplied; original path branch and directory search execute','46A9AF..46AA2C fragment stops before85A030 parent-path normalization; supported real-source model paths have no parent components','Ordinary4EAAA0 with construction flag clear; actor type and state-presence callbacks supplied; actual45E5E0 state index lookup executes'],'folders':folders(),'paths':paths(),'selection':selection()}
 (HERE/'native-model-binding.json').write_text(json.dumps(out,indent=2)+'\n');print({k:len(out[k])for k in ['folders','paths','selection']})
