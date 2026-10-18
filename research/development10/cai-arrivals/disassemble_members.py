from pathlib import Path
import sys
root=Path.cwd();sys.path.insert(0,str(root/'research/phase8-movement'));import harness as h
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
c=Cs(CS_ARCH_X86,CS_MODE_32)
for a,n in [(0x4364c0,0xc0),(0x4d1080,0x30),(0x51c220,0x45),(0x51c850,0x60)]:
 print('FUNCTION',hex(a))
 for i in c.disasm(h.P.range(a,n),a):print(f'{i.address:08X}',i.bytes.hex(),i.mnemonic,i.op_str)
