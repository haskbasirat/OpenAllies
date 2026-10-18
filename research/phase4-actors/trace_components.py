"""Original actor component selection and lifecycle loops; explicit leaf seams.

No game process. Actual selection, bit tests, linked list, dependency validation
and lifecycle branches execute. Constructors and component virtual methods are
fixtures; complete spatial/owner/ability initialization is NOT claimed.
"""
from pathlib import Path
import hashlib,json,struct,sys
from trace_registry import P,V,H
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE
from unicorn.x86_const import *
HERE=Path(__file__).resolve().parent

class Machine:
 def __init__(self,case):
  self.case=case;self.events=[];self.visited={};self.end=None;self.heap=0x2200000
  self.u=Uc(UC_ARCH_X86,UC_MODE_32);self.u.mem_map(P.base,(P.image_size+4095)&~4095)
  for vs,r,size,off in P.sections:self.u.mem_write(P.base+r,P.b[off:off+size])
  self.u.mem_map(0,4096);self.u.mem_map(0x2000000,0x400000)
  self.actor=0x2000000;self.definition=0x2001000;self.stack=0x2300000;self.stop=0x23ff000
  self.w(self.actor,0x2002000);self.w(0x2002000+0x108,0x23ff100);self.w(self.actor+4,self.definition)
  self.w(self.definition+0x174,case.get('mask',0));self.w(self.actor+0x100,case.get('low',0));self.w(self.actor+0x104,case.get('high',0))
  for s in case.get('present',[]):self.install(s)
  self.u.hook_add(UC_HOOK_CODE,self.hook)
 def w(self,p,v):self.u.mem_write(p,struct.pack('<I',v&0xffffffff))
 def r(self,p):return struct.unpack('<I',self.u.mem_read(p,4))[0]
 def arg(self,n=0):return self.r(self.u.reg_read(UC_X86_REG_ESP)+4+n*4)
 def ret(self,value=0,pop=0):
  sp=self.u.reg_read(UC_X86_REG_ESP);self.u.reg_write(UC_X86_REG_EAX,value&0xffffffff);self.u.reg_write(UC_X86_REG_EIP,self.r(sp));self.u.reg_write(UC_X86_REG_ESP,sp+4+pop)
 def alloc(self,n):p=self.heap;self.heap+=(n+15)&~15;return p
 def install(self,s):
  ptr=0x2010000+s*0x100;vt=0x2020000+s*0x100;self.w(ptr,vt);self.w(ptr+4,s);self.w(self.actor+0x60+s*4,ptr)
  for off in [4,8,12,20,32]:self.w(vt+off,0x23fe000+off)
  return ptr
 def apply_mutation(self,kind,slot):
  for m in self.case.get('mutations',[]):
   if m['kind']==kind and m['slot']==slot:
    if 'low' in m:self.w(self.actor+0x100,m['low'])
    if 'high' in m:self.w(self.actor+0x104,m['high'])
    for s in m.get('add',[]):self.install(s)
    for s in m.get('remove',[]):self.w(self.actor+0x60+s*4,0)
 def hook(self,u,pc,size,user):
  self.visited[pc]=size;ecx=u.reg_read(UC_X86_REG_ECX)
  if pc==0x6f7810:self.ret(self.alloc(self.arg()))
  elif pc==0x4976b0:
   p=self.arg(1);b=bytearray()
   while bytes(self.u.mem_read(p,2))!=b'\0\0':b.extend(self.u.mem_read(p,2));p+=2
   self.events.append({'index':self.arg(),'name':b.decode('utf-16-le')});self.ret(pop=12)
  elif pc==0x6f7430:self.ret()
  elif pc==0x565fa0:
   self.events.append({'kind':'create','slot':15});p=self.install(15);self.apply_mutation('create',15);self.ret(p,4)
  elif pc==0x4cf480:
   s=self.arg();self.events.append({'kind':'create','slot':s})
   if s not in self.case.get('null_factory',[]):self.install(s)
   self.apply_mutation('create',s);self.ret(pop=4)
  elif pc==0x4d2100:
   assert self.arg()==29;self.events.append({'kind':'city_predicate'});self.ret(int(self.case.get('city',False)),4)
  elif pc==0x4b2830:self.events.append({'kind':'construction_callback'});self.ret()
  elif pc==0x23ff100:self.ret(0)
  elif pc==0x404ed0:self.events.append({'kind':'missing_requirement','slot':self.arg(6),'required':self.arg(7)});self.ret()
  elif 0x23fe000<=pc<0x23fe100:
   s=self.r(ecx+4);off=pc-0x23fe000
   kind={4:'init_create',8:'init_existing',12:'post_create',20:'post_existing',32:'requirement'}[off]
   event={'kind':kind,'slot':s}
   if off==4:event['record']=self.arg()
   if off==12:event.update(record=self.arg(),flag=self.arg(1))
   self.events.append(event);self.apply_mutation(kind,s)
   result=self.case.get('requirements',{}).get(str(s),-1) if off==32 else int(s!=self.case.get('fail_slot',-1))
   self.ret(result,4 if off==4 else 8 if off==12 else 0)
  elif pc==0x4ebe50:self.ret(int(self.case.get('base_ok',True)),4)
  elif pc==0x4d1fd0:self.ret(int(self.case.get('required_object_ok',True)))
  elif pc==0x4d72f3 and self.case['mode']=='existing':self.end=True;u.emu_stop()
  elif pc==0x4eba40:self.ret(pop=8)
  elif self.case['mode']=='create' and pc==self.loop_end:self.end=True;u.emu_stop()
  elif self.case['mode']=='create' and pc==self.loop_fail:self.end=False;u.emu_stop()
 def run(self):
  mode=self.case['mode'];args=[]
  if mode=='assemble':pc=0x4d21a0;args=[0]
  elif mode=='enum':pc=0x497fa0
  elif mode=='validate':pc=0x4cfb00
  elif mode=='existing':pc=0x4d7230;args=[0]
  elif mode=='post':pc=0x4cf300;args=[0x1234,1]
  elif mode=='create':
   # Enter exact isolated loop with its established saved registers. No claim
   # that the preceding owner/nested actor/stance construction has executed.
   pc=0x4d71b7;self.loop_end=0x4d71e2;self.loop_fail=0x4d722a
   self.w(self.stack+0x94,0x1234)
  else:raise ValueError(mode)
  self.w(self.stack,self.stop)
  for i,v in enumerate(args):self.w(self.stack+4+i*4,v)
  for reg,val in [(UC_X86_REG_ESP,self.stack),(UC_X86_REG_ECX,self.actor),(UC_X86_REG_EBX,0x10101010),(UC_X86_REG_EBP,0x20202020),(UC_X86_REG_ESI,0x30303030),(UC_X86_REG_EDI,0x40404040)]:self.u.reg_write(reg,val)
  if mode=='create':self.u.reg_write(UC_X86_REG_ESI,self.actor)
  self.u.emu_start(pc,self.stop,count=500000)
  if self.end is None:
   assert self.u.reg_read(UC_X86_REG_EIP)==self.stop,hex(self.u.reg_read(UC_X86_REG_EIP))
   assert self.u.reg_read(UC_X86_REG_ESP)==self.stack+4+len(args)*4
   for reg,val in [(UC_X86_REG_EBX,0x10101010),(UC_X86_REG_EBP,0x20202020),(UC_X86_REG_ESI,0x30303030),(UC_X86_REG_EDI,0x40404040)]:assert self.u.reg_read(reg)==val
  result=self.end if self.end is not None else (None if mode in ['post','enum'] else bool(self.u.reg_read(UC_X86_REG_EAX)&255))
  return {'input':self.case,'result':result,'events':self.events,'present':[s for s in range(31) if self.r(self.actor+0x60+s*4)],'low':self.r(self.actor+0x100),'high':self.r(self.actor+0x104)}

def main():
 cases=[]
 for low,high in [(0,0),(1<<21,0),(1<<25,0),(1<<26,0),(0,16),(1<<21|1<<25,0),(1<<21,16)]:
  for city in [False,True]:cases.append({'mode':'assemble','mask':0x7fffffff,'low':low,'high':high,'city':city})
 for slot in range(31):cases.append({'mode':'assemble','mask':1<<slot,'null_factory':[4]})
 cases.extend([
  {'mode':'assemble','mask':0x7fffffff,'present':[0,3,4,15,29],'city':True},
  {'mode':'assemble','mask':0x7fffffff,'null_factory':[0,2,4,29],'city':True},
  {'mode':'assemble','mask':0x7fffffff,'city':True,'mutations':[{'kind':'create','slot':15,'low':1<<21}]},
  {'mode':'assemble','mask':0x7fffffff,'low':1<<21,'city':True,'mutations':[{'kind':'create','slot':0,'low':0}]},
  {'mode':'assemble','mask':9,'requirements':{'3':1},'null_factory':[4]},
 ])
 for present,requires in [([],{}),([0,3,4],{}),([0,3,4],{'3':0,'4':3}),([0,3,4],{'0':1,'3':2}),([30],{'30':29})]:cases.append({'mode':'validate','present':present,'requirements':requires})
 for mode in ['existing','create']:
  for present in [[],[0,3,4,30],list(range(31))]:
   for fail in [-1,0,4,30]:cases.append({'mode':mode,'present':present,'fail_slot':fail})
 cases.extend([{'mode':'existing','present':[0,3,4],'base_ok':False},{'mode':'existing','present':[0,3,4],'required_object_ok':False},
  {'mode':'existing','present':[0,3,4],'mutations':[{'kind':'init_existing','slot':0,'add':[5],'remove':[3]},{'kind':'init_existing','slot':4,'add':[30]}]},
  {'mode':'create','present':[0,3,4],'mutations':[{'kind':'init_create','slot':0,'add':[2],'remove':[3]}]},
  {'mode':'post','present':list(range(31))},{'mode':'post','present':[0,3,4,30],'mutations':[{'kind':'post_create','slot':0,'add':[1],'remove':[3]}]}])
 m=Machine({'mode':'enum'});enum=m.run()['events'];assert len(enum)==31
 rows=[];visited=dict(m.visited)
 for c in cases:
  m=Machine(c);rows.append(m.run());visited.update(m.visited)
 spans=[]
 for a,n in sorted(visited.items()):
  if not P.base<=a<P.base+P.image_size:continue
  if spans and spans[-1][1]>=a:spans[-1][1]=max(a+n,spans[-1][1])
  else:spans.append([a,a+n])
 windows=[]
 for a,z in spans:
  b=P.range(a,z-a);assert b==V.range(a,z-a);windows.append({'start':hex(a),'end':hex(z),'sha256':H(b)})
 out={'schema':1,'passed':True,'producer_sha256':H(Path(__file__).read_bytes()),'source_sha256':H(P.b),'v14_sha256':H(V.b),'component_enum':enum,'cases':rows,'windows':windows,'seams':['Component enum executes 497FA0 with only 4976B0 registration sink captured.','Component constructors and virtual methods are deterministic leaf callbacks; this is not complete component behavior.','Selection runs actual 4D21A0 with original linked list and 4CFB00 validation, construction callback seam, publication disabled by owner-query zero.','Create initialization enters the exact 4D71B7..4D71E2 loop with proven register/record setup; pre-loop owner/stance/nested actor construction is excluded.','Existing initialization executes 4D7230 through its complete component loops; GObject/required-object preconditions are explicit seams and later abilities/owner logic is excluded.','PostCreate executes 4CF300 with base callback seam and owner/city notifications disabled by explicit fixture state.','Allocation succeeds; no allocator-failure or corrupted-pointer parity claim.']}
 p=HERE/'components-native-oracle.json';p.write_text(json.dumps(out,indent=2)+'\n');print(json.dumps({'cases':len(rows),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
