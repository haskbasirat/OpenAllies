from pathlib import Path
import sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'));import native_machine as h
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
md=Cs(CS_ARCH_X86,CS_MODE_32)
for start,end in[(0x86ba10,0x86bc70),(0x86e8d0,0x86e8f6),(0x86b940,0x86ba10)]:
 print(f'FUNCTION{start:X}..{end:X}')
 for i in md.disasm(h.P.range(start,end-start),start):print(f'{i.address:08x} {i.bytes.hex():24} {i.mnemonic:8} {i.op_str}')
vt=0xab70a0;fn=int.from_bytes(h.P.range(vt+0x48,4),'little');assert h.P.range(vt+0x48,4)==h.V.range(vt+0x48,4)
print(f'VIRTUAL48 {vt:X}+48={fn:X}')
for i in md.disasm(h.P.range(fn,0x120),fn):print(f'{i.address:08x} {i.bytes.hex():24} {i.mnemonic:8} {i.op_str}')
