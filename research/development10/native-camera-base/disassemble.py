from pathlib import Path
import sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'));import native_machine as h
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
md=Cs(CS_ARCH_X86,CS_MODE_32)
for start,end in[(0x874b40,0x874cd3),(0x8751c0,0x8751ee),(0x86f710,0x86f7dd),(0x86b240,0x86b263),(0x86a680,0x86a6e0),(0x88a5a0,0x88a5b0),(0x869c30,0x869c90),(0x86fdd0,0x86fe00)]:
 print(f'FUNCTION{start:X}..{end:X}')
 for i in md.disasm(h.P.range(start,end-start),start):print(f'{i.address:08x} {i.bytes.hex():24} {i.mnemonic:8} {i.op_str}')
