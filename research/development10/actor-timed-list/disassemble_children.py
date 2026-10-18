from pathlib import Path
import sys
root=Path.cwd();sys.path.insert(0,str(root/'research/phase8-movement'));import harness as h
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
c=Cs(CS_ARCH_X86,CS_MODE_32)
for a,n in [(0x4d3500,0x2e0),(0x4f1430,0xb0),(0x5ac280,0xe0)]:
 print('FUNCTION',hex(a))
 for i in c.disasm(h.P.range(a,n),a):print(f'{i.address:08X}',i.bytes.hex(),i.mnemonic,i.op_str)
