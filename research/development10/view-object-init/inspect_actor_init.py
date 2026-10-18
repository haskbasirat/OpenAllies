from pathlib import Path
import sys
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parent/'creator-labels'))
from native_creator_labels import PE0
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
md=Cs(CS_ARCH_X86,CS_MODE_32)
for start,size in [(0x451f00,0x90),(0x451f90,0x250),(0x44f320,0x30),(0x5caf80,0x100),(0x85d840,0x50),(0x5c8650,0x70),(0x4177e0,0x240)]:
 print(hex(start))
 for x in md.disasm(PE0.range(start,size),start):print(f'{x.address:08x} {x.bytes.hex()} {x.mnemonic} {x.op_str}')
for vt in [0xa02270,0xa01ef0]:
 print('table',hex(vt),{hex(o):hex(int.from_bytes(PE0.range(vt+o,4),'little'))for o in [0x70,0xc8,0x104,0xf8]})
