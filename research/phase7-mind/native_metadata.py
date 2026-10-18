"""Execute registered Mind state metadata CRT constructors, preserving flags."""
from pathlib import Path
import json,struct,sys,hashlib
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE))
from native_constructors import ConstructorMachine,original,modified,EMPTY,ALLOC
from unicorn.x86_const import *
def va(raw_offset):
 for _,rva,size,offset in original.sections:
  if offset<=raw_offset<offset+size:return original.base+rva+raw_offset-offset
 raise ValueError(raw_offset)
def find_initializer(address):
 needle=b'\xa3'+struct.pack('<I',address);offset=original.b.find(needle)
 assert offset>=0
 start=original.b.rfind(b'\x6a\xff\x68',offset-100,offset)
 assert start>=0,(hex(address),hex(va(offset)))
 return va(start)
class MetadataMachine(ConstructorMachine):
 def __init__(self,address):
  super().__init__({'kind':'base','precision':'Double53','seed':0,'abilities':0,'fill':0xcc});self.address=address;self.name=None;self.w(EMPTY+12,10000)
 def hook(self,u,pc,size,data):
  if pc==0x401b00:
   ptr=self.r(u.reg_read(UC_X86_REG_ESP)+4);units=[]
   while self.r(ptr)&65535:units.append(self.r(ptr)&65535);ptr+=2
   self.name=''.join(chr(v)for v in units);self.w(u.reg_read(UC_X86_REG_ECX),EMPTY+16);self.ret(EMPTY+16,pop=4)
  elif pc==0x405b80:
   p=u.reg_read(UC_X86_REG_ECX);source=self.r(u.reg_read(UC_X86_REG_ESP)+4);self.w(p,self.r(source));self.ret(p,pop=4)
  elif pc==0x96cab0:self.ret()
  else:super().hook(u,pc,size,data)
 def run_metadata(self):
  self.reset(0);entry=find_initializer(self.address);self.u.emu_start(entry,self.STOP,count=10000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+4
  return {'metadata_va':hex(self.address),'initializer_va':hex(entry),'name':self.name,'word4':self.r(self.address+4),'discard_on_suspend':bool(self.u.mem_read(self.address+8,1)[0]),'words_0c_14':[self.r(self.address+i)for i in [12,16,20]]}
def main():
 doc=json.loads((HERE/'native-constructors.json').read_text());addresses=set();layouts={};visited={}
 for row in doc['cases']:
  kind=row['input']['kind']
  if kind in layouts:continue
  data=bytes.fromhex(row['output']['initialized_hex']);words=struct.unpack('<'+'I'*(len(data)//4),data);states=[]
  for i in range(len(words)-3):
   if words[i+1]==int(doc['addresses']['this'],16) and 0xb3ce00<=words[i+2]<0xb3d500:
    states.append({'offset':i*4,'vtable_va':hex(words[i]),'metadata_va':hex(words[i+2]),'status':words[i+3]});addresses.add(words[i+2])
  layouts[kind]=states
 records=[]
 for address in sorted(addresses):
  m=MetadataMachine(address);records.append(m.run_metadata());visited.update(m.visited)
 for a,b in visited.items():assert original.range(a,len(b))==b==modified.range(a,len(b))
 result={'schema':1,'state_layouts':layouts,'metadata':records,'scope':'Exact initialized embedded state offsets/vtables/metadata/status from constructor executions; registered metadata CRT bodies executed with CString storage and atexit registration seams.','instructions':[{'va':hex(a),'bytes':b.hex()}for a,b in sorted(visited.items())]}
 p=HERE/'native-metadata.json';p.write_text(json.dumps(result,indent=2),encoding='utf-8');print(json.dumps({'states':sum(map(len,layouts.values())),'metadata':len(records),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
