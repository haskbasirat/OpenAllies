"""Print original AA.exe instructions for a function range (read-only helper).
usage: disasm.py <start-hex> [end-hex]; end defaults to the functions.tsv size."""
from pathlib import Path
import struct, sys
HERE = Path(__file__).resolve().parent
GAME = HERE.parents[2]
sys.path.insert(0, str(GAME / 'reconstruction/tools/python-packages'))
from capstone import Cs, CS_ARCH_X86, CS_MODE_32
b = (GAME / 'AA.exe').read_bytes()
def off(va):
    pe = struct.unpack_from('<I', b, 0x3c)[0]; n = struct.unpack_from('<H', b, pe + 6)[0]; op = pe + 24
    sec = op + struct.unpack_from('<H', b, pe + 20)[0]
    for i in range(n):
        vs, rva, rs, ro = struct.unpack_from('<IIII', b, sec + i * 40 + 8)
        if rva <= va - 0x400000 < rva + max(vs, rs): return ro + va - 0x400000 - rva
    raise ValueError(hex(va))
start = int(sys.argv[1], 16)
if len(sys.argv) > 2: end = int(sys.argv[2], 16)
else:
    size = None
    for line in open(GAME / 'reconstruction/research/native-aa/functions.tsv', encoding='utf-8'):
        f = line.split('\t')
        if f[0] == '%08x' % start: size = int(f[3]); break
    end = start + size
md = Cs(CS_ARCH_X86, CS_MODE_32)
o = off(start)
for ins in md.disasm(b[o:o + end - start], start):
    print('%08x  %-20s %s %s' % (ins.address, ins.bytes.hex(), ins.mnemonic, ins.op_str))
