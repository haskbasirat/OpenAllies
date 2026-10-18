from pathlib import Path
import sys
root=Path.cwd();sys.path.insert(0,str(root/'research/phase8-ai'))
from native_machine import original
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
cs=Cs(CS_ARCH_X86,CS_MODE_32)
for a,b in [(0x464480,0x464980),(0x47df70,0x47e010)]:
 print('\nRANGE',hex(a),hex(b))
 for i in cs.disasm(original.range(a,b-a),a):print(f'{i.address:08x}: {i.bytes.hex():20} {i.mnemonic:8} {i.op_str}')
