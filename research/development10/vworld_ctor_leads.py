from pathlib import Path
import sys,struct
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'research/development10/creator-labels'))
from native_creator_labels import PE0,PE1
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
md=Cs(CS_ARCH_X86,CS_MODE_32)
for start,size in [(0x442c00,0x1c0),(0x43ec60,0x70),(0x54bde0,0x70)]:
 raw=PE0.range(start,size);assert raw==PE1.range(start,size)
 print(f'WINDOW{start:X}')
 for i in md.disasm(raw,start):print(f'{i.address:x} {i.bytes.hex()} {i.mnemonic} {i.op_str}')
