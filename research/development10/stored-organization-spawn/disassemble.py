from pathlib import Path
import sys
root=Path.cwd();sys.path.insert(0,str(root/'research/phase8-movement'));import harness as h
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
c=Cs(CS_ARCH_X86,CS_MODE_32)
for a,n in [(0x578290,0x70),(0x554ad0,0x150),(0x57ad30,0xc0),(0x57a730,0x40),(0x57d860,0xf0),(0x578040,0x110)]:
 print('FUNCTION',hex(a))
 for i in c.disasm(h.P.range(a,n),a):print(f'{i.address:08X}',i.mnemonic,i.op_str)
