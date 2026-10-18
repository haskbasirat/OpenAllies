from pathlib import Path
import sys,struct
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'research/development10/creator-labels'))
from native_creator_labels import PE0,PE1
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
md=Cs(CS_ARCH_X86,CS_MODE_32)
slots=[]
for table in [0xa20770,0xa13318,0xa05540,0xa13448,0xa0f6e0]:
 va=table+0x20;raw=PE0.range(va,4);assert raw==PE1.range(va,4)
 getter=struct.unpack('<I',raw)[0];slots.append(getter);print(f'VIRTUAL {va:X} {raw.hex()} {getter:X}')
for table in [0xa02d40,0xa02798,0xa02270,0xa01ef0,0xa019f8,0xa01ca0]:
 for offset in [0xc8,0x14]:
  va=table+offset;raw=PE0.range(va,4);assert raw==PE1.range(va,4)
  print(f'VIEW VIRTUAL {va:X} {raw.hex()} {int.from_bytes(raw,"little"):X}')
for start,size in [(0x43d160,0x60),(0x43ac20,0x780),(0x43c420,0x90),(0x50c220,0x35),(0x4f0750,0x170)]+[(g,0x20)for g in slots]:
 raw=PE0.range(start,size);assert raw==PE1.range(start,size)
 print(f'WINDOW{start:X}')
 for i in md.disasm(raw,start):print(f'{i.address:x} {i.bytes.hex()} {i.mnemonic} {i.op_str}')
