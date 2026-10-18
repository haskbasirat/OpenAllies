"""Actual4CEF10/541FE0->5817A0->5030F0; applicability and581630 are seams."""
from pathlib import Path
import copy,hashlib,itertools,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(HERE));from original_properties import IMAGE,ACCEPTED,pe
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE
from unicorn.x86_const import *
ACTOR,COMPONENT,OTHER,PROPERTY,SOURCE,WORLD,STACK,STOP,HEAP=range(0x3000000,0x3009000,0x1000)
class Machine:
 def __init__(self,c):
  self.c=c;self.events=[];self.visited={};self.u=Uc(UC_ARCH_X86,UC_MODE_32);IMAGE.load(self.u);self.u.mem_map(0,4096);self.u.mem_map(0x3000000,0x10000);self.u.hook_add(UC_HOOK_CODE,self.hook)
  self.w(ACTOR+0x78,COMPONENT if c['component'] else 0);self.w(ACTOR+0x98,OTHER if c['special'] else 0);self.w(SOURCE+0x14,c['source_id']);self.w(0xb36534,WORLD);self.w(WORLD+0xe0,c['time_bits'])
 def w(self,a,v):self.u.mem_write(a,struct.pack('<I',v&0xffffffff))
 def r(self,a):return struct.unpack('<I',self.u.mem_read(a,4))[0]
 def arg(self,i=0):return self.r(self.u.reg_read(UC_X86_REG_ESP)+4+4*i)
 def ret(self,pop=0,value=0):
  sp=self.u.reg_read(UC_X86_REG_ESP);self.u.reg_write(UC_X86_REG_EIP,self.r(sp));self.u.reg_write(UC_X86_REG_ESP,sp+4+pop);self.u.reg_write(UC_X86_REG_EAX,value)
 def hook(self,u,a,n,_):
  if 0x400000<=a<0xc00000:self.visited[a]=n
  if a==0x5035e0:
   assert [self.arg(),self.arg(1)]==[ACTOR,PROPERTY];self.events.append({'Applicable':[1,4]})
   if self.c['replace']:self.w(ACTOR+0x78,OTHER)
   self.ret(value=int(self.c['applicable']))
  elif a==0x6f7810:assert self.arg()==0x18;self.ret(value=HEAP)
  elif a==0x581630:
   p=self.arg();component=u.reg_read(UC_X86_REG_ECX);assert component in [COMPONENT,OTHER]
   self.events.append({'Insert':{'component':2 if component==COMPONENT else 3,'record':{'handle':9,'definition':4,'kind':self.r(p+8),'created':self.r(p+4),'source_id':self.r(p+20),'disabled':bool(self.u.mem_read(p+16,1)[0])}}});self.ret(4,value=0)
 def run(self):
  args=[ACTOR if self.c['actor'] else 0,PROPERTY if self.c['property'] else 0] if self.c['trigger'] else [PROPERTY if self.c['property'] else 0,self.c['kind'],SOURCE if self.c['source'] else 0]
  self.w(STACK,STOP)
  for i,arg in enumerate(args):self.w(STACK+4+4*i,arg)
  self.u.reg_write(UC_X86_REG_ESP,STACK);self.u.reg_write(UC_X86_REG_ECX,ACTOR)
  regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
  for i,r in enumerate(regs):self.u.reg_write(r,0x11111111*(i+1))
  self.u.emu_start(0x541fe0 if self.c['trigger'] else 0x4cef10,STOP,count=100000)
  assert self.u.reg_read(UC_X86_REG_EIP)==STOP
  assert self.u.reg_read(UC_X86_REG_ESP)==STACK+4+(8 if self.c['trigger'] else 12)
  assert [self.u.reg_read(r) for r in regs]==[0x11111111*(i+1) for i in range(4)]
  return {'result':bool(self.u.reg_read(UC_X86_REG_EAX)&255),'component':0 if not self.r(ACTOR+0x78) else 3 if self.r(ACTOR+0x78)==OTHER else 2,'events':self.events,'abi_preserved':True}
def cases():
 base={'actor':True,'property':True,'component':True,'special':False,'applicable':True,'replace':False,'source':True,'source_id':0xabcd1234,'time_bits':0x41200000,'trigger':False,'kind':2}
 for kind,special,applicable,replace in itertools.product([0,2,8,10,0xffffffff],[False,True],[False,True],[False,True]):
  yield {**base,'name':f'add_{kind}_{special}_{applicable}_{replace}','kind':kind,'special':special,'applicable':applicable,'replace':replace}
 for actor,property,component,applicable in itertools.product([False,True],repeat=4):
  yield {**base,'name':f'trigger_{actor}_{property}_{component}_{applicable}','trigger':True,'kind':2,'source':False,'actor':actor,'property':property,'component':component,'applicable':applicable}
 for changes in [{'property':False},{'component':False},{'source':False},{'time_bits':0x7fc00000}]:yield {**base,'name':str(changes),**changes}
def main():
 rows=[];visited={}
 for c in cases():
  m=Machine(c);rows.append({'input':c,'output':m.run()});visited.update(m.visited)
 windows=[]
 for a,n in sorted(visited.items()):
  raw=IMAGE.at(a,n);assert raw==ACCEPTED.at(a,n);windows.append({'address':hex(a),'bytes':raw.hex()})
 doc={'schema':1,'original_sha256':pe.ORIGINAL_SHA,'v14_sha256':pe.ACCEPTED_SHA,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'cases':rows,'visited_windows':windows,'seams':['5035E0 applicability (separately implemented in aa-actors)','581630 insert (separately implemented as property_flags::insert); return false supplied to prove outer ignores it','allocation of one24byte instance'],'scope':'Actual4CEF10/541FE0/5817A0/5030F0; original instance initialization and component reread execute; null replacement crash excluded.'}
 path=HERE/'native-admission.json';path.write_text(json.dumps(doc,indent=2)+'\n');print(json.dumps({'cases':len(rows),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
