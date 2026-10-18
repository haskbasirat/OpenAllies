"""Disassemble original AA.exe functions (function bounds from functions.tsv).

Usage: python disasm.py <hexaddr> [<hexaddr> ...]
Reads E:/Games/Axis and Alies/AA.exe; prints Capstone x86-32 listings and the
float constants referenced by absolute memory operands.
"""
from pathlib import Path
import sys, struct, csv
ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'reconstruction/tools/python-packages'))
from capstone import Cs, CS_ARCH_X86, CS_MODE_32
EXE = (ROOT / 'AA.exe').read_bytes()
pe = struct.unpack_from('<I', EXE, 0x3c)[0]
n = struct.unpack_from('<H', EXE, pe + 6)[0]
op = pe + 24
BASE = struct.unpack_from('<I', EXE, op + 28)[0]
sec = op + struct.unpack_from('<H', EXE, pe + 20)[0]
SECTIONS = [struct.unpack_from('<IIII', EXE, sec + i * 40 + 8) for i in range(n)]

def off(va):
    rva = va - BASE
    for vs, r, size, o in SECTIONS:
        if r <= rva < r + max(size, vs):
            return o + rva - r
    raise ValueError(hex(va))

def rd(va, size):
    o = off(va)
    return EXE[o:o + size]

SIZES = {}
with open(ROOT / 'reconstruction/research/native-aa/functions.tsv', newline='') as fh:
    for row in csv.reader(fh, delimiter='\t'):
        if row[0] == 'address':
            continue
        SIZES[int(row[0], 16)] = int(row[3])

def listing(va, size=None):
    size = size or SIZES.get(va) or 0x200
    md = Cs(CS_ARCH_X86, CS_MODE_32)
    out = []
    for ins in md.disasm(rd(va, size), va):
        out.append(f'{ins.address:08x}: {ins.mnemonic} {ins.op_str}')
    return out

if __name__ == '__main__':
    for a in sys.argv[1:]:
        if ':' in a:
            a, s = a.split(':')
            va, size = int(a, 16), int(s, 16)
        else:
            va, size = int(a, 16), None
        print(f'=== {va:08x} size {size or SIZES.get(va)}')
        print('\n'.join(listing(va, size)))
