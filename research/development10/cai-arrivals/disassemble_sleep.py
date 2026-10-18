from pathlib import Path
import sys
root=Path.cwd();sys.path.insert(0,str(root/'research/phase8-movement'));import harness as h
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
c=Cs(CS_ARCH_X86,CS_MODE_32)
for a,n in [(0x520120,0x100),(0x522f60,0xa0),(0x523000,0x100),(0x57bf30,0x130),(0x578140,0xe0),(0x51d0f0,0x50)]:
 print('FUNCTION',hex(a))
 for i in c.disasm(h.P.range(a,n),a):print(f'{i.address:08X}',i.bytes.hex(),i.mnemonic,i.op_str)
