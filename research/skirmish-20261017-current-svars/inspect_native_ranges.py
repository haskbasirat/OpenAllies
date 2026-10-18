from pathlib import Path
import sys
root=Path.cwd();sys.path.insert(0,str(root/'research/phase8-movement'));import harness as h
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
cs=Cs(CS_ARCH_X86,CS_MODE_32)
for a,n in[(0x5065d0,110),(0x508550,220),(0x4b35f0,35)]:
 print(hex(a))
 for i in cs.disasm(h.P.range(a,n),a):print(hex(i.address),i.mnemonic,i.op_str)
