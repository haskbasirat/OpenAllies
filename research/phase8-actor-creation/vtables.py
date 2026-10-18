"""Dump the component vtables built by the 4CF480 constructors (read-only)."""
import sys, struct
from pathlib import Path
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / 'phase8-combat'))
import harness as h
from capstone import Cs, CS_ARCH_X86, CS_MODE_32
md = Cs(CS_ARCH_X86, CS_MODE_32)
CTORS = {0: 0x561960, 1: 0x580ed0, 2: 0x563670, 3: 0x576920, '4org': 0x526260, '4air': 0x51bff0,
         '4elem': 0x535ba0, 5: 0x57f4d0, 6: 0x5812f0, 7: 0x579bd0, 8: 0x570930, 9: 0x56f370,
         10: 0x56b720, 11: 0x58aa50, 12: 0x58cdb0, 13: 0x588fb0, 14: 0x5867a0, 15: 0x565fa0,
         16: 0x581e40, 17: 0x571690, 18: 0x562f10, 19: 0x575940, 20: 0x567cd0, 21: 0x584210,
         22: 0x56eab0, 23: 0x55e8e0, 24: 0x58b9a0, 25: 0x574fa0, 26: 0x572180, 27: 0x93fec0,
         28: 0x580120, 29: 0x564160, 30: 0x574490}
SIZES = {0: 0x44, 1: 0x74, 2: 0x30, 3: 0x8c, '4org': 0x2c4, '4air': 0x1d8, '4elem': 0x1bc, 5: 100, 6: 0xdc, 7: 0xdc,
         8: 0x34, 9: 0x54, 10: 0x94, 11: 0x20, 12: 0x24, 13: 0x38, 14: 0x88, 15: 0x2c, 16: 0x20, 17: 0x10,
         18: 0x10, 19: 0x18, 20: 0x34, 21: 0x18, 22: 0x18, 23: 0xb4, 24: 0x60, 25: 0x24, 26: 0x14, 27: 0x38,
         28: 0x10, 29: 0x24, 30: 0x2c}
def u32(va): return struct.unpack('<I', h.ORIGINAL.at(va, 4))[0]
def vtable_stores(va, limit=0x600):
    out = []
    for i in md.disasm(h.ORIGINAL.at(va, limit), va):
        if i.mnemonic == 'mov' and i.op_str.startswith('dword ptr [') and ', 0x' in i.op_str:
            imm = int(i.op_str.split(', ')[1], 16)
            if 0xa00000 <= imm < 0xb00000:
                out.append((i.address, i.op_str))
        if i.mnemonic == 'ret':
            break
    return out
for k, va in CTORS.items():
    st = vtable_stores(va)
    print(k, hex(va), 'size', hex(SIZES[k]), [f'{a:x}:{o}' for a, o in st])
