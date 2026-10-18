from pathlib import Path
import sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'));import native_machine as h
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
md=Cs(CS_ARCH_X86,CS_MODE_32)
for start,end in[(0x6fedb0,0x6ff1a7),(0x46e0e0,0x46e150),(0x46ef30,0x46f150),(0x46e4f0,0x46e540),(0x6fe670,0x6fe6c0)]:
 print(f'FUNCTION{start:X}..{end:X}')
 for i in md.disasm(h.P.range(start,end-start),start):print(f'{i.address:08x} {i.bytes.hex():24} {i.mnemonic:8} {i.op_str}')
