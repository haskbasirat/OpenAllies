from pathlib import Path
import sys
root=Path.cwd();sys.path.insert(0,str(root/'research/phase8-movement'));import harness as h
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
c=Cs(CS_ARCH_X86,CS_MODE_32)
for a,n in [(0x4d0900,0x45),(0x524b80,0xc0),(0x524d70,0x3b0),(0x521c30,0x1d0),(0x528c20,0x1b0)]:
 print('FUNCTION',hex(a))
 for i in c.disasm(h.P.range(a,n),a):print(f'{i.address:08X}',i.bytes.hex(),i.mnemonic,i.op_str)
