"""Read-only original/v14 instruction windows for registration grid order."""
from pathlib import Path
import sys
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
cs=Cs(CS_ARCH_X86,CS_MODE_32)
for address,size in [(0x4cfea0,107),(0x4ee700,79),(0x554cc0,259),
                     (0x5541f0,310),(0x54ef10,42),(0x515620,0x140)]:
    raw=h.P.range(address,size);assert raw==h.V.range(address,size),hex(address)
    print(f'original/v14 identical {address:08x}+{size:x}')
    for ins in cs.disasm(raw,address):
        print(f'{ins.address:08x}: {ins.bytes.hex()} {ins.mnemonic} {ins.op_str}')
