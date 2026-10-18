"""Read-only exact references to native Element/Regiment reset latches."""
from pathlib import Path
import sys,json,struct,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));from native_machine import original,modified
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
from capstone.x86_const import X86_OP_IMM,X86_OP_MEM
md=Cs(CS_ARCH_X86,CS_MODE_32);md.detail=True;md.skipdata=True
refs={a:[]for a in(0xb3cf58,0xb3d2d4)}
for vs,rva,size,off in original.sections:
 if original.base+rva!=0x401000:continue
 data=original.b[off:off+size]
 for i in md.disasm(data,original.base+rva):
  if i.id==0:continue
  for o in i.operands:
   address=o.imm if o.type==X86_OP_IMM else o.mem.disp if o.type==X86_OP_MEM and not o.mem.base and not o.mem.index else None
   if address in refs:
    assert original.range(i.address,i.size)==modified.range(i.address,i.size)
    refs[address].append({'va':hex(i.address),'bytes':i.bytes.hex(),'instruction':i.mnemonic+' '+i.op_str})
rows=[]
for address,uses in refs.items():
 sec=next(s for s in original.sections if s[1]<=address-original.base<s[1]+s[0]);vs,rva,raw,off=sec;idx=address-original.base-rva
 init=original.b[off+idx] if idx<raw else 0
 assert init==0
 vsec=next(s for s in modified.sections if s[1]<=address-modified.base<s[1]+s[0]);vvs,vrva,vraw,voff=vsec;vidx=address-modified.base-vrva
 vinit=modified.b[voff+vidx] if vidx<vraw else 0
 assert vinit==0
 assert len(uses)>0
 rows.append({'address':hex(address),'initial_byte':init,'v14_initial_byte':vinit,'section_raw_size':raw,'section_virtual_size':vs,'offset_in_section':idx,'literal_references':uses})
result={'scope':'linear native .text literal-address audit plus PE initialized/BSS byte; does not exclude arbitrary unsupported computed-pointer memory corruption','rows':rows}
p=HERE/'native-reset-guards.json'
if '--write' in sys.argv:p.write_text(json.dumps(result,indent=2))
else:assert json.loads(p.read_text())==result
print(json.dumps({'rows':len(rows),'references':sum(len(v)for v in refs.values()),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}))
