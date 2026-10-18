"""Actual numeric loader tails; TGI readers/attribute lookup are explicit seams."""
from pathlib import Path
import csv,hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(HERE));from original_properties import IMAGE,ACCEPTED,pe,bits
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE
from unicorn.x86_const import *
MOD,NODE,STRING,STRINGPTR,LOOKUP,ATTRIBUTE,AMOUNT,STUB,STACK,STOP=[0x4000000+i*0x1000 for i in range(10)]
class Machine:
 def __init__(self,c):
  self.c=c;self.visited={};self.error=None;self.u=Uc(UC_ARCH_X86,UC_MODE_32);IMAGE.load(self.u);self.u.mem_map(0,4096);self.u.mem_map(MOD,0x10000)
  self.u.reg_write(UC_X86_REG_FPCW,{'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[c['precision']]);self.u.hook_add(UC_HOOK_CODE,self.hook)
  self.w(STRING+12,2);self.w(STRINGPTR,STRING+16);self.w(LOOKUP,ATTRIBUTE);self.w(ATTRIBUTE,c['attribute']);self.w(AMOUNT,c['amount'])
  self.u.mem_write(STUB,b'\xd9\x05'+struct.pack('<I',AMOUNT)+b'\xc2\x04\x00')
 def w(self,a,v):self.u.mem_write(a,struct.pack('<I',v&0xffffffff))
 def r(self,a):return struct.unpack('<I',self.u.mem_read(a,4))[0]
 def ret(self,pop,value=0):
  sp=self.u.reg_read(UC_X86_REG_ESP);self.u.reg_write(UC_X86_REG_EIP,self.r(sp));self.u.reg_write(UC_X86_REG_ESP,sp+4+pop);self.u.reg_write(UC_X86_REG_EAX,value)
 def hook(self,u,a,n,_):
  if 0x400000<=a<0xc00000:self.visited[a]=n
  if a==0x4feaf0:self.ret(4)
  elif a==0x5b5130:u.reg_write(UC_X86_REG_EIP,STUB)
  elif a==0x5b54d0:self.ret(4,STRINGPTR)
  elif a==0x4012c0:self.ret(0,STRING)
  elif a==0x497220:self.ret(4,LOOKUP if self.c['found'] else 0)
  elif a==0x5b5380:self.ret(4,int(self.c['repair']))
  elif a==0x410c40:
   message=self.r(u.reg_read(UC_X86_REG_ESP)+4)
   self.error={0xa2d908:'attribute',0xa2d9d8:'attribute',0xa2d880:'repair',0xa2d980:'range'}[message];u.emu_stop()
 def run(self):
  self.w(STACK,STOP);self.w(STACK+4,NODE);self.u.reg_write(UC_X86_REG_ESP,STACK);self.u.reg_write(UC_X86_REG_ECX,MOD)
  regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
  for i,r in enumerate(regs):self.u.reg_write(r,0x11111111*(i+1))
  self.u.emu_start(0x4fec70 if self.c['kind']=='Add' else 0x4fed70,STOP,count=10000)
  if self.error:return {'error':self.error}
  assert self.u.reg_read(UC_X86_REG_EIP)==STOP and self.u.reg_read(UC_X86_REG_ESP)==STACK+8
  assert [self.u.reg_read(r) for r in regs]==[0x11111111*(i+1) for i in range(4)]
  return {'modifier':{'kind':self.c['kind'],'index':self.r(MOD+16),'amount':self.r(MOD+12),'repair':bool(self.r(MOD+20)&1)},'abi_preserved':True}
def cases():
 rows=[]
 for kind,indices in [('Add',range(13)),('Multiply',range(13,25))]:
  for index in indices:
   for value in [-6,-1.1,-1,-.2,-0.0,0,.1,1,1.1,5,5.1,6]:
    rows.append({'name':f'{kind}_{index}_{value}','kind':kind,'attribute':index,'amount':bits(value),'repair':False,'found':True})
 for index in range(13):rows.append({'name':f'repair_{index}','kind':'Add','attribute':index,'amount':bits(.5),'repair':True,'found':True})
 for kind,index,found in [('Add',-1,True),('Add',13,True),('Multiply',12,True),('Multiply',25,True),('Add',3,False),('Multiply',20,False)]:
  rows.append({'name':f'invalid_{kind}_{index}_{found}','kind':kind,'attribute':index,'amount':bits(.25),'repair':False,'found':found})
 for p in ['Single24','Double53','Extended64']:
  for c in rows:yield {**c,'name':c['name']+'_'+p,'precision':p}
def main():
 rows=[];visited={}
 for c in cases():
  m=Machine(c);rows.append({'input':c,'output':m.run()});visited.update(m.visited)
 windows=[]
 for a,n in sorted(visited.items()):
  raw=IMAGE.at(a,n);assert raw==ACCEPTED.at(a,n);windows.append({'address':hex(a),'bytes':raw.hex()})
 constants=[]
 for a in [0x9fdc48,0xa02b30]:
  raw=IMAGE.at(a,4);assert raw==ACCEPTED.at(a,4);constants.append({'address':hex(a),'bytes':raw.hex()})
 doc={'schema':1,'original_sha256':pe.ORIGINAL_SHA,'v14_sha256':pe.ACCEPTED_SHA,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'cases':rows,'visited_windows':windows,'constants':constants,'seams':['base modifier loading','TGI amount/attribute/repair readers','string copy','attribute ID lookup','diagnostic call stops emulation; partial invalid object state is outside Rust constructor contract']}
 path=HERE/'native-modifier-load.json';path.write_text(json.dumps(doc,indent=2)+'\n');print(json.dumps({'cases':len(rows),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
