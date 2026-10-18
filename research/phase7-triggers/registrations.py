"""Execute native trigger enum registration and parameter storage classification.

Only registration allocation/insertion is replaced to capture actual call args.
"""
from pathlib import Path
import csv,hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-body'))
from original_body import IMAGE,ACCEPTED,pe
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE
from unicorn.x86_const import *
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
SPACE,STACK,STOP=0x4000000,0x4010000,0x4011000
def run(address):
 u=Uc(UC_ARCH_X86,UC_MODE_32);IMAGE.load(u);u.mem_map(SPACE,0x20000);rows=[]
 def r(a):return struct.unpack('<I',u.mem_read(a,4))[0]
 def hook(u,a,n,_):
  if a not in [0x5492c0,0x545db0]:return
  sp=u.reg_read(UC_X86_REG_ESP);index,pointer,flags=[r(sp+4+i*4) for i in range(3)];raw=bytearray();offset=0
  while True:
   pair=IMAGE.at(pointer+offset,2);assert pair==ACCEPTED.at(pointer+offset,2);raw.extend(pair);offset+=2
   if pair==b'\0\0':break
  rows.append({'index':index,'name':raw[:-2].decode('utf-16le'),'flags':flags,'string_address':hex(pointer),'string_sha256':hashlib.sha256(raw).hexdigest()})
  u.reg_write(UC_X86_REG_EIP,r(sp));u.reg_write(UC_X86_REG_ESP,sp+16)
 u.hook_add(UC_HOOK_CODE,hook);u.mem_write(STACK,struct.pack('<I',STOP));u.reg_write(UC_X86_REG_ESP,STACK);u.reg_write(UC_X86_REG_ECX,SPACE)
 u.emu_start(address,STOP,count=10000);assert u.reg_read(UC_X86_REG_EIP)==STOP and u.reg_read(UC_X86_REG_ESP)==STACK+4
 return rows
def storage(index):
 u=Uc(UC_ARCH_X86,UC_MODE_32);IMAGE.load(u);u.mem_map(SPACE,0x20000);u.mem_write(STACK,struct.pack('<II',STOP,index));u.reg_write(UC_X86_REG_ESP,STACK)
 u.emu_start(0x548920,STOP,count=1000);assert u.reg_read(UC_X86_REG_EIP)==STOP and u.reg_read(UC_X86_REG_ESP)==STACK+4
 return u.reg_read(UC_X86_REG_EAX)
def schemas():
 u=Uc(UC_ARCH_X86,UC_MODE_32);IMAGE.load(u);u.mem_map(SPACE,0x20000)
 def call(address,owner=0):
  u.mem_write(STACK,struct.pack('<I',STOP));u.reg_write(UC_X86_REG_ESP,STACK);u.reg_write(UC_X86_REG_ECX,owner);u.emu_start(address,STOP,count=10000)
  assert u.reg_read(UC_X86_REG_EIP)==STOP and u.reg_read(UC_X86_REG_ESP)==STACK+4
 call(0x4c5dc0);rows=[]
 for index in range(115):
  u.mem_write(SPACE,b'\0'*0x80);u.mem_write(SPACE+12,struct.pack('<I',index));call(0x4c6e40,SPACE)
  opcode,kind,count=struct.unpack('<3I',u.mem_read(SPACE+0x20,12));assert opcode==index and count<=8
  rows.append({'opcode':index,'kind':kind,'arguments':list(struct.unpack('<'+'I'*count,u.mem_read(SPACE+0x2c,count*4))),'flags':list(u.mem_read(SPACE+0x4c,2))})
 return rows
def main():
 sizes={int(r['address'],16):int(r['bodyBytes']) for r in csv.DictReader((ROOT.parent/'reconstruction/research/native-aa/functions.tsv').open(),delimiter='\t')};ranges=[]
 for address in [0x545ec0,0x549440,0x549720,0x548920,0x4c5dc0,0x4c6e40,0x54c280,0x5411d0,0x54bdc0,0x4db300,0x540e20,0x540a30,0x540a90,0x541410,0x542b00,0x5468a0,0x548c10,0x54a640,0x54d3d0]:
  raw=IMAGE.at(address,sizes[address]);assert raw==ACCEPTED.at(address,len(raw));ranges.append({'address':hex(address),'size':len(raw),'sha256':hashlib.sha256(raw).hexdigest()})
  (HERE/f'{address:08x}.asm.txt').write_text('\n'.join(f'{i.address:08x}: {i.mnemonic} {i.op_str}' for i in Cs(CS_ARCH_X86,CS_MODE_32).disasm(raw,address))+'\n')
 doc={'schema':1,'original_sha256':pe.ORIGINAL_SHA,'v14_sha256':pe.ACCEPTED_SHA,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'native_ranges':ranges,'elements':run(0x545ec0),'arguments':run(0x549440),'items':run(0x549720),'element_schemas':schemas()}
 for item in doc['items']:item['storage']=storage(item['index'])
 path=HERE/'native-registrations.json';path.write_text(json.dumps(doc,indent=2)+'\n');print(json.dumps({'elements':len(doc['elements']),'arguments':len(doc['arguments']),'items':len(doc['items']),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
