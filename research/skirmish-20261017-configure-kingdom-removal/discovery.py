from pathlib import Path
import sys
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'research/development10/creator-labels'))
from native_creator_labels import PE0,PE1
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
md=Cs(CS_ARCH_X86,CS_MODE_32)
for start,size in [(0x4e4130,0xf0),(0x4e6bc0,0x20),(0x4e57c0,0x400),(0x4f09c0,0x80),(0x79eec0,0x140),(0x79f590,0x40),(0x76c7c0,0xb0),(0x4d7580,0x290)]:
 raw=PE0.range(start,size)
 assert raw==PE1.range(start,size)
 print(f'FUNCTION WINDOW {start:08X}')
 for ins in md.disasm(raw,start):print(f'{ins.address:08x} {ins.bytes.hex():20} {ins.mnemonic} {ins.op_str}')
for off in [0x8c,0x90,0xf0,0xe0]:
 print(f'VTABLE A299D8+{off:X}: {int.from_bytes(PE0.range(0xa299d8+off,4),"little"):08X}')
