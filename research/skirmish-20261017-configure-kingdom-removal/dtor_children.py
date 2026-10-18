from pathlib import Path
import sys
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'research/development10/creator-labels'))
from native_creator_labels import PE0,PE1
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
md=Cs(CS_ARCH_X86,CS_MODE_32)
for start,size in [(0x55a5b0,0x80),(0x4a6ad0,0xc0),(0x4e29b0,0x100),(0x96be33,0xc0),(0x4e5bba,0x30),(0x50b7a0,0xb0)]:
 raw=PE0.range(start,size);assert raw==PE1.range(start,size)
 print(f'WINDOW {start:X}')
 for i in md.disasm(raw,start):print(f'{i.address:x} {i.bytes.hex()} {i.mnemonic} {i.op_str}')
