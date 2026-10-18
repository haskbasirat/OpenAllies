from pathlib import Path
import sys,struct
sys.path.insert(0,str(Path(__file__).resolve().parent.parent))
from native_bootstrap import PE0
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
cs=Cs(CS_ARCH_X86,CS_MODE_32)
needle=struct.pack('<I',0xb43b48)
at=0
while True:
 at=PE0.b.find(needle,at)
 if at<0:break
 va=None
 for virtual_size,rva,raw_size,raw_offset in PE0.sections:
  if raw_offset<=at<raw_offset+raw_size:va=PE0.base+rva+at-raw_offset;break
 at+=4
 if va is None:continue
 if va>0x900000:
  print(hex(va))
  for i in cs.disasm(PE0.range(va-16,45),va-16):print(hex(i.address),i.mnemonic,i.op_str)
