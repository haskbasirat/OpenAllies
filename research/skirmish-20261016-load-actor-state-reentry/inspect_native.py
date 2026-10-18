from pathlib import Path
import sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
cs=Cs(CS_ARCH_X86,CS_MODE_32)
for a,n in [(0x4d4a90,0x1cf),(0x45e600,0x59),(0x4b4590,0x4e),(0x4d0a00,0x7b),(0x4f0460,0x20),(0x79e130,16),(0x4624b0,0x130),(0x464dc0,0x250),(0x464f99,0x6e),(0x4985e0,0x100),(0x42b8e0,0x90),(0x474270,0xd0),(0x47dd32,0x129)]:
 b=h.P.range(a,n);assert b==h.V.range(a,n),hex(a)
 print(f'original/v14 identical {a:08x}+{n:x}')
 for i in cs.disasm(b,a):print(f'{i.address:08x}: {i.bytes.hex()} {i.mnemonic} {i.op_str}')
