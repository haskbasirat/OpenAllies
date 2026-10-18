"""Bounded raw-address audit of default property globals and producer inputs.

All literal-address occurrences in both PEs are retained, including data and
address-of candidates. Nearby decoding is diagnostic, not control-flow or
whole-program alias closure. Constructors/shutdowns and alias-exposed property
objects remain writable; this does not assert native lifetime immutability.
"""
from pathlib import Path
import sys,json,hashlib,struct
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE))
from native_model_defaults import PE0,PE1,GLOBALS,zero_tail
from capstone import Cs,CS_ARCH_X86,CS_MODE_32,CS_AC_WRITE
from capstone.x86_const import X86_OP_MEM,X86_OP_IMM
ADDRESSES=GLOBALS+[0xb43e68,0xb43e6c,0xb43e70,0xb323c0,0xb43c64,0xb43c68,0xb45f54,0xb45f58]
def audit(pe,image):
 e=struct.unpack_from('<I',pe.b,0x3c)[0];count=struct.unpack_from('<H',pe.b,e+6)[0];table=e+24+struct.unpack_from('<H',pe.b,e+20)[0];sections=[]
 for i in range(count):
  p=table+40*i;name=pe.b[p:p+8].split(b'\0')[0].decode('ascii');virtual,rva,raw,offset=struct.unpack_from('<IIII',pe.b,p+8);flags=struct.unpack_from('<I',pe.b,p+36)[0];sections.append((name,virtual,rva,raw,offset,flags))
 decoder=Cs(CS_ARCH_X86,CS_MODE_32);decoder.detail=True;references=[]
 for address in ADDRESSES:
  needle=struct.pack('<I',address);offset=pe.b.find(needle)
  while offset!=-1:
   section=next((s for s in sections if s[4]<=offset<s[4]+s[3]),None);candidates=[];va=None
   if section:
    name,virtual,rva,raw,base,flags=section;va=pe.base+rva+offset-base
    if flags&0x20000000:
     for back in range(1,12):
      start=offset-back
      if start<base:continue
      decoded=list(decoder.disasm(pe.b[start:offset+12],va-back,count=1))
      if not decoded:continue
      instruction=decoded[0]
      if instruction.address+instruction.size<va+4:continue
      if not any(o.type==X86_OP_MEM and o.mem.base==0 and o.mem.index==0 and o.mem.disp==address or o.type==X86_OP_IMM and o.imm==address for o in instruction.operands):continue
      candidates.append(dict(va=hex(instruction.address),bytes=instruction.bytes.hex(),mnemonic=instruction.mnemonic,operands=instruction.op_str,direct_memory_write=any(o.type==X86_OP_MEM and o.mem.base==0 and o.mem.index==0 and o.mem.disp==address and o.access&CS_AC_WRITE for o in instruction.operands)))
   references.append(dict(target=hex(address),file_offset=offset,va=None if va is None else hex(va),section=None if section is None else section[0],candidates=candidates));offset=pe.b.find(needle,offset+1)
 return dict(image=image,sha256=hashlib.sha256(pe.b).hexdigest(),references=references)
def main():
 images=[audit(PE0,'original'),audit(PE1,'v14')]
 #Record the extra NiCore child reached before8C3ED0. Its actual callback
 #remains a separate initialization boundary, not an assumed no-op.
 decoder=Cs(CS_ARCH_X86,CS_MODE_32);prelude=[]
 for start,size in [(0x8b77d0,0x30),(0x908630,0x70),(0x928150,0x10)]:
  for i in decoder.disasm(PE0.range(start,size),start):
   raw=PE0.range(i.address,i.size);assert raw==PE1.range(i.address,i.size);prelude.append(dict(va=hex(i.address),bytes=raw.hex(),instruction=i.mnemonic+' '+i.op_str))
 doc=dict(scope=__doc__,images=images,pre_ni_main_child=prelude,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
 path=HERE/'model-default-global-audit.json'
 if '--create'in sys.argv:path.open('x').write(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_bytes())==doc
 print(json.dumps(dict(references=[len(i['references'])for i in images],sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
