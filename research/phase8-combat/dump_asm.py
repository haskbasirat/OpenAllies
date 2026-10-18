"""Dump original AA.exe instructions for the listed functions (read-only).

Sizes come from the Ghidra function inventory (bodyBytes). Output files are
review aids; they pin bytes, they do not claim behaviour.
"""
from pathlib import Path
import struct, sys
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
GAME = ROOT.parent
sys.path.insert(0, str(GAME / 'reconstruction/tools/python-packages'))
from capstone import Cs, CS_ARCH_X86, CS_MODE_32
INV = GAME / 'reconstruction/research/native-aa/functions.tsv'
data = (GAME / 'AA.exe').read_bytes()
pe = struct.unpack_from('<I', data, 0x3c)[0]
count = struct.unpack_from('<H', data, pe + 6)[0]
opt = pe + 24
base = struct.unpack_from('<I', data, opt + 28)[0]
sect = opt + struct.unpack_from('<H', data, pe + 20)[0]
sections = [struct.unpack_from('<IIII', data, sect + i * 40 + 8) for i in range(count)]
def offset(va):
    for vsize, rva, size, off in sections:
        if rva <= va - base < rva + size:
            return off + va - base - rva
    raise ValueError(hex(va))
sizes = {}
for line in INV.read_text().splitlines()[1:]:
    parts = line.split('\t')
    sizes[int(parts[0], 16)] = int(parts[3])
md = Cs(CS_ARCH_X86, CS_MODE_32)
for arg in sys.argv[1:]:
    va = int(arg, 16)
    size = sizes[va]
    o = offset(va)
    lines = [f'{i.address:08x}  {i.bytes.hex():<20} {i.mnemonic} {i.op_str}' for i in md.disasm(data[o:o + size], va)]
    (HERE / 'asm' / f'{va:08x}.asm.txt').write_text('\n'.join(lines) + '\n')
    print(f'{va:08x} {size} bytes {len(lines)} instructions')
