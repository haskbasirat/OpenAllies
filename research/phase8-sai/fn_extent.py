"""Linear disassembly of an original AA.exe function up to its int3/next-function
padding, following in-function jump targets (read-only helper).
usage: fn_extent.py <start-hex> [--calls]"""
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
md = Cs(CS_ARCH_X86, CS_MODE_32)
def extent(start):
    """Return the instruction list of the function: explore reachable code."""
    seen = {}
    work = [start]
    while work:
        a = work.pop()
        while a not in seen:
            o = off(a)
            ins = next(md.disasm(b[o:o + 16], a))
            seen[a] = ins
            m = ins.mnemonic
            if m.startswith('j'):
                try:
                    t = int(ins.op_str, 16)
                    work.append(t)
                except ValueError:
                    pass
                if m == 'jmp':
                    break
            if m in ('ret', 'retn', 'int3'):
                break
            a += ins.size
    return [seen[k] for k in sorted(seen)]
if __name__ == '__main__':
    start = int(sys.argv[1], 16)
    ins = extent(start)
    if '--calls' in sys.argv:
        print('%x size~%d' % (start, ins[-1].address + ins[-1].size - start))
        for i in ins:
            if i.mnemonic == 'call': print('  %08x call %s' % (i.address, i.op_str))
    else:
        for i in ins:
            print('%08x  %-20s %s %s' % (i.address, i.bytes.hex(), i.mnemonic, i.op_str))
