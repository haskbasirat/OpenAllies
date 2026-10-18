"""Roundtrip every decoded tutorial Trigger through original540A90/542B00/548C10.

No serialization function is replaced. Stream methods and localized string
registration are external context seams. This checks decoded field/layout fidelity,
not execution semantics or the separate manager variable/string table ordering.
"""
from pathlib import Path
import hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(HERE));import decode_census as decoded
sys.path.insert(0,str(ROOT/'research/phase4-body'));from original_body import IMAGE,ACCEPTED,pe
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE
from unicorn.x86_const import *
BASE,STREAM,VT,STUB,STACK,STOP=0x4000000,0x4000000,0x4001000,0x4002000,0x400f000,0x400f100
class Writer:
 def __init__(self):
  self.u=Uc(UC_ARCH_X86,UC_MODE_32);IMAGE.load(self.u);self.u.mem_map(0,4096);self.u.mem_map(BASE,0x1000000);self.visited={};self.bump=BASE+0x10000;self.output=bytearray();self.strings={}
  self.u.hook_add(UC_HOOK_CODE,self.hook);self.w(STREAM+0x3c,VT);self.w(VT+4,STUB);self.w(VT+12,STUB+16);self.w(VT+0x48,STUB+16)
  self.tables={}
  for kind,globalp in [('elements',0xb3d478),('arguments',0xb3d49c),('items',0xb3d4c0)]:
   table=self.alloc(512);self.w(globalp,table);self.tables[kind]={}
   for row in decoded.registry[kind]:
    rec=self.alloc(0x30);self.w(rec+4,self.string(row['name']));self.w(table+row['index']*4,rec);self.tables[kind][row['name']]=(row['index'],rec)
 def alloc(self,n):p=self.bump;self.bump+=(n+15)&~15;assert self.bump<BASE+0x1000000;return p
 def w(self,a,v):self.u.mem_write(a,struct.pack('<I',v&0xffffffff))
 def r(self,a):return struct.unpack('<I',self.u.mem_read(a,4))[0]
 def ret(self,pop=0,value=0):
  sp=self.u.reg_read(UC_X86_REG_ESP);self.u.reg_write(UC_X86_REG_EIP,self.r(sp));self.u.reg_write(UC_X86_REG_ESP,sp+4+pop);self.u.reg_write(UC_X86_REG_EAX,value)
 def hook(self,u,a,n,_):
  if 0x400000<=a<0xc00000:self.visited[a]=n
  if a==STUB:
   sp=u.reg_read(UC_X86_REG_ESP);p,n=self.r(sp+4),self.r(sp+8);assert n<100000;self.output.extend(u.mem_read(p,n));self.ret(8)
  elif a==STUB+16:self.ret(value=1)
  elif a==0x59e3b0:self.ret(value=0) # external MAPSTRINGS save context
  elif a==0x5a1120:self.ret(4) # localization registration, no binary payload
 def string(self,s):
  if s in self.strings:return self.strings[s]
  raw=s.encode('utf-16le');p=self.alloc(18+len(raw))+16;self.w(p-12,len(raw)//2);self.w(p-8,len(raw)//2);self.w(p-4,2);self.u.mem_write(p,raw+b'\0\0');self.strings[s]=p;return p
 def parameter(self,p,q):
  self.w(q,p['storage']);self.w(q+0x28,self.tables['arguments'][p['argument']][0]);self.w(q+0x2c,p['item_index']);self.u.mem_write(q+0x30,bytes([p['flag']]))
  value=p['value']
  if p['storage'] in [1,2]:self.w(q+4,value)
  elif p['storage']==5:
   for i,v in enumerate(value):self.w(q+16+i*4,v)
  elif p['storage']==6:self.w(q+28,self.string(value))
  elif p['storage']==7:self.w(q+32,self.string(value['key']));self.w(q+36,self.string(''))
 def element(self,e):
  q=self.alloc(20);definition=self.alloc(0x30);self.w(definition+0x20,e['opcode']);self.w(q,definition);self.w(q+4,len(e['parameters']));self.w(q+16,e['word_10']);params=self.alloc(len(e['parameters'])*0x34);self.w(q+8,params)
  for i,p in enumerate(e['parameters']):self.parameter(p,params+i*0x34)
  return q
 def linked(self,values):
  tail=0
  for value in reversed(values):node=self.alloc(8);self.w(node,value);self.w(node+4,tail);tail=node
  return tail
 def trigger(self,t):
  q=self.alloc(0x38);self.w(q,t['mode']);self.w(q+4,t['activation_bits']);self.w(q+8,t['id']);self.w(q+12,self.string(t['name']));self.w(q+16,self.string(t['description']));self.w(q+44,t['word_2c']);self.u.mem_write(q+52,bytes([t['enabled'],t['repeat']]))
  for offset,label in [(20,'events'),(28,'conditions'),(36,'actions')]:self.w(q+offset,self.linked([self.element(e) for e in t[label]]))
  return q
 def write(self,t):
  self.output=bytearray();q=self.trigger(t);self.w(STACK,STOP);self.w(STACK+4,STREAM);self.u.reg_write(UC_X86_REG_ESP,STACK);self.u.reg_write(UC_X86_REG_ECX,q)
  regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
  for i,r in enumerate(regs):self.u.reg_write(r,0x11111111*(i+1))
  self.u.emu_start(0x540a90,STOP,count=100000);assert self.u.reg_read(UC_X86_REG_EIP)==STOP and self.u.reg_read(UC_X86_REG_ESP)==STACK+8
  assert [self.u.reg_read(r) for r in regs]==[0x11111111*(i+1) for i in range(4)]
  return bytes(self.output)
def main():
 cases=[];machine=Writer()
 for name in ['tutorial01','tutorial02']:
  source=json.loads((HERE/f'{name}-decoded.json').read_text());b,pins=decoded.chunk(ROOT.parent/f'reconstruction/assets/original/Data/Maps/Campaign/{name}.RMP','TRIGGERS')
  for t in source['manager']['triggers']:
   out=machine.write(t);start,n=t['span'];original=b[start:start+n]
   if out!=original:
    first=next((i for i,(x,y) in enumerate(zip(out,original)) if x!=y),min(len(out),len(original)));raise AssertionError((name,t['id'],first,len(out),len(original),out[first:first+30].hex(),original[first:first+30].hex()))
   cases.append({'map':name,'trigger_id':t['id'],'span':t['span'],'bytes_sha256':hashlib.sha256(out).hexdigest(),'map_sha256':pins['map_sha256'],'abi_preserved':True})
 windows=[]
 for a,n in sorted(machine.visited.items()):
  raw=IMAGE.at(a,n);assert raw==ACCEPTED.at(a,n);windows.append({'address':hex(a),'bytes':raw.hex()})
 doc={'schema':1,'original_sha256':pe.ORIGINAL_SHA,'v14_sha256':pe.ACCEPTED_SHA,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'cases':cases,'visited_windows':windows,'seams':['stream write callback','stream write-mode and contextual-interface predicates returntrue','external MAPSTRINGS native save mode59E3B0 returnsfalse','5A1120 localized registration, does not emit binary bytes'],'scope':'All185 actual tutorial Trigger records pass exact native writer replay; manager header/variable/string table order and trigger execution not covered.'}
 path=HERE/'native-saved-writer.json';path.write_text(json.dumps(doc,indent=2)+'\n');print(json.dumps({'records':len(cases),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
