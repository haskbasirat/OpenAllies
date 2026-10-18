from pathlib import Path
import sys
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
cs=Cs(CS_ARCH_X86,CS_MODE_32)
for address,size in [(0x4f1660,955),(0x54ece0,7),(0x54ef80,0x2c),(0x4b87a0,0x245)]:
 b=h.P.range(address,size)
 assert b==h.V.range(address,size)
 print(f'Original/v14 identical window {address:08x}+{size:x}')
 for i in cs.disasm(b,address):print(f'{i.address:08x}: {i.bytes.hex()} {i.mnemonic} {i.op_str}')
for table in [0xa2a2a0,0xa273f0]:
 address=int.from_bytes(h.P.range(table+0xc,4),'little')
 assert h.P.range(table+0xc,4)==h.V.range(table+0xc,4)
 print(f'Object/Actor vtable {table:08x}+0C -> {address:08x}')
 for i in cs.disasm(h.P.range(address,48),address):print(f'{i.address:08x}: {i.bytes.hex()} {i.mnemonic} {i.op_str}')
