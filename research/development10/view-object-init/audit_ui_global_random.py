"""Bounded static audit of UI Init global RNG B39714 source references.

Raw address occurrences include code/data aliases, not merely disassembler
xrefs. Nearby decoded candidates are diagnostic, not a whole-program alias
proof. The data section is writable. The explicitly bounded PE virtual tail
starts zero, before actual constructor/time32 stores through the App+84 alias.
This does NOT claim a current random state, complete writer/draw census, or
native lifetime immutability.
"""
from pathlib import Path
import sys,json,hashlib,struct
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parent/'creator-labels'))
from native_creator_labels import PE0,PE1
from capstone import Cs,CS_ARCH_X86,CS_MODE_32,CS_AC_WRITE
from capstone.x86_const import X86_OP_MEM,X86_OP_IMM
ADDRESSES=[0xb39714]
def audit(pe,image):
 e=struct.unpack_from('<I',pe.b,0x3c)[0];count=struct.unpack_from('<H',pe.b,e+6)[0]
 optional=struct.unpack_from('<H',pe.b,e+20)[0];table=e+24+optional
 sections=[]
 for i in range(count):
  pos=table+40*i;name=pe.b[pos:pos+8].split(b'\0')[0].decode('ascii')
  virtual,rva,raw,offset=struct.unpack_from('<IIII',pe.b,pos+8);flags=struct.unpack_from('<I',pe.b,pos+36)[0]
  sections.append((name,virtual,rva,raw,offset,flags))
 decoder=Cs(CS_ARCH_X86,CS_MODE_32);decoder.detail=True
 raw_refs=[]
 for address in ADDRESSES:
  needle=struct.pack('<I',address);offset=pe.b.find(needle)
  while offset!=-1:
   section=next((s for s in sections if s[4]<=offset<s[4]+s[3]),None)
   candidates=[];va=None
   if section:
    name,virtual,rva,raw,base,flags=section;va=pe.base+rva+offset-base
    if flags&0x20000000:
     for back in range(1,12):
      start=offset-back
      if start<base:continue
      instructions=list(decoder.disasm(pe.b[start:offset+12],va-back,count=1))
      if not instructions:continue
      instruction=instructions[0]
      if instruction.address+instruction.size<va+4:continue
      if not any(o.type==X86_OP_MEM and o.mem.base==0 and o.mem.index==0 and o.mem.disp==address
                 or o.type==X86_OP_IMM and o.imm==address for o in instruction.operands):continue
      candidates.append(dict(va=hex(instruction.address),bytes=instruction.bytes.hex(),mnemonic=instruction.mnemonic,operands=instruction.op_str,
        direct_memory_write=any(o.type==X86_OP_MEM and o.mem.base==0 and o.mem.index==0 and o.mem.disp==address and o.access&CS_AC_WRITE for o in instruction.operands)))
   raw_refs.append(dict(address=hex(address),file_offset=offset,va=None if va is None else hex(va),section=None if section is None else section[0],
    executable=False if section is None else bool(section[5]&0x20000000),candidates=candidates))
   offset=pe.b.find(needle,offset+1)
 cells=[]
 for address in ADDRESSES:
  section=next(s for s in sections if pe.base+s[2]<=address and address+4<=pe.base+s[2]+s[1])
  try:raw=pe.range(address,4);tail=False
  except ValueError:
   assert pe.base+section[2]+section[3]<=address
   assert address+4<=pe.base+section[2]+section[1]
   raw=bytes(4);tail=True
  cells.append(dict(va=hex(address),bytes=raw.hex(),word=int.from_bytes(raw,'little'),section=section[0],writable=bool(section[5]&0x80000000),zero_fill_tail=tail))
 return dict(image=image,pe_sha256=hashlib.sha256(pe.b).hexdigest(),cells=cells,raw_address_references=raw_refs)
def main():
 images=[audit(PE0,'original'),audit(PE1,'v14')]
 assert images[0]['cells']==images[1]['cells']
 doc=dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),images=images)
 out=HERE/'ui-global-random-audit.json'
 if '--write'in sys.argv:out.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(out.read_text())==doc
 print(json.dumps(dict(raw_refs=[len(i['raw_address_references'])for i in images],write_candidates=[sum(c['direct_memory_write']for r in i['raw_address_references']for c in r['candidates'])for i in images],sha256=hashlib.sha256(out.read_bytes()).hexdigest())))
if __name__=='__main__':main()
