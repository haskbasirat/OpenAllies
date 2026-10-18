from pathlib import Path
import sys
root=Path.cwd();sys.path.insert(0,str(root/'research/phase8-movement'));import harness as h
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
c=Cs(CS_ARCH_X86,CS_MODE_32)
for i in c.disasm(h.P.range(0x4041f0,130),0x4041f0):print(hex(i.address),i.mnemonic,i.op_str)
