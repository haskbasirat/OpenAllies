from pathlib import Path
import sys
ROOT=Path.cwd();sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
c=Cs(CS_ARCH_X86,CS_MODE_32)
for a,n in [(0x773ce0,0x180),(0x40ca40,0x60),(0x682e70,0x70),(0x40ca80,0x55),(0x6e8cc0,0x30)]:
 print(hex(a))
 for i in c.disasm(h.P.range(a,n),a):print(hex(i.address),i.mnemonic,i.op_str)
