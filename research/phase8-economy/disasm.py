"""Print original AA.exe disassembly for function addresses (read-only)."""
from pathlib import Path
import csv, struct, sys
ROOT = Path(__file__).resolve().parents[2]
GAME = ROOT.parent
sys.path.insert(0, str(GAME / 'reconstruction/tools/python-packages'))
from capstone import Cs, CS_ARCH_X86, CS_MODE_32
NATIVE = GAME / 'reconstruction/research/native-aa'
DATA = (GAME / 'AA.exe').read_bytes()
p = struct.unpack_from('<I', DATA, 0x3c)[0]
count = struct.unpack_from('<H', DATA, p + 6)[0]
opt = struct.unpack_from('<H', DATA, p + 20)[0]
BASE = struct.unpack_from('<I', DATA, p + 24 + 28)[0]
SECTIONS = []
for i in range(count):
    s = p + 24 + opt + i * 40
    vs, rva, size, off = struct.unpack_from('<4I', DATA, s + 8)
    SECTIONS.append((rva, size, off))
def at(addr, n):
    rva = addr - BASE
    for va, size, off in SECTIONS:
        if va <= rva < va + size:
            return DATA[off + rva - va: off + rva - va + n]
    raise ValueError(hex(addr))
SIZES = {int(r['address'], 16): int(r['bodyBytes']) for r in csv.DictReader(open(NATIVE / 'functions.tsv', encoding='utf-8'), delimiter='\t')}
def dis(addr, size=None):
    size = size or SIZES.get(addr, 0x200)
    md = Cs(CS_ARCH_X86, CS_MODE_32)
    out = []
    for ins in md.disasm(at(addr, size), addr):
        out.append(f'{ins.address:08x}: {ins.mnemonic} {ins.op_str}')
    return out
if __name__ == '__main__':
    for a in sys.argv[1:]:
        if ':' in a:
            x, n = a.split(':'); addr, n = int(x, 16), int(n, 16)
        else:
            addr, n = int(a, 16), None
        print(f'=== {addr:08x}')
        print('\n'.join(dis(addr, n)))
