"""Static cross-reference helper over the ORIGINAL AA.exe image (read only).

Usage: python xref.py <hex address> [...]
  Lists rel32 CALL/JMP sites and absolute dword references (vtables,
  push imm32, mov imm32) to each target, and disassembles a function window
  when invoked with --dis <start> <count>.
"""
from pathlib import Path
import struct, sys
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'research/engine-baseline'))
sys.path.insert(0, str(ROOT.parent / 'reconstruction/tools/python-packages'))
from audit_baseline import PE
from capstone import Cs, CS_ARCH_X86, CS_MODE_32

P = PE(ROOT.parent / 'AA.exe')


def image():
    mem = bytearray(P.image_size)
    for vs, r, size, off in P.sections:
        mem[r:r + size] = P.b[off:off + size]
    return bytes(mem)


MEM = image()


def xrefs(target):
    out = []
    for i in range(len(MEM) - 5):
        op = MEM[i]
        if op in (0xE8, 0xE9):
            rel = struct.unpack_from('<i', MEM, i + 1)[0]
            if (P.base + i + 5 + rel) & 0xffffffff == target:
                out.append(('call' if op == 0xE8 else 'jmp', P.base + i))
    needle = struct.pack('<I', target)
    start = 0
    while True:
        j = MEM.find(needle, start)
        if j < 0:
            break
        out.append(('abs', P.base + j))
        start = j + 1
    return out


def dis(start, count):
    md = Cs(CS_ARCH_X86, CS_MODE_32)
    code = MEM[start - P.base:start - P.base + count * 15]
    lines = []
    for n, ins in enumerate(md.disasm(code, start)):
        if n >= count:
            break
        lines.append(f'{ins.address:08x} {ins.bytes.hex():<24} {ins.mnemonic} {ins.op_str}')
    return lines


if __name__ == '__main__':
    if sys.argv[1] == '--dis':
        print('\n'.join(dis(int(sys.argv[2], 16), int(sys.argv[3]))))
    else:
        for a in sys.argv[1:]:
            t = int(a, 16)
            print(hex(t), [(k, hex(v)) for k, v in xrefs(t)])


def r32(a):
    return struct.unpack_from('<I', MEM, a - P.base)[0]


def funcinfo(fi):
    """MSVC FuncInfo: magic, maxState, pUnwindMap, nTryBlocks, pTryBlockMap."""
    magic, maxstate, unwind, ntry, trymap = struct.unpack_from('<5I', MEM, fi - P.base)
    blocks = []
    for i in range(ntry):
        lo, hi, catchhigh, ncatch, handlers = struct.unpack_from('<5I', MEM, trymap - P.base + i * 20)
        hs = []
        for j in range(ncatch):
            adj, ptype, disp, addr = struct.unpack_from('<4I', MEM, handlers - P.base + j * 16)
            hs.append({'adjectives': hex(adj), 'type': hex(ptype), 'disp': disp, 'handler': hex(addr)})
        blocks.append({'try_low': lo, 'try_high': hi, 'catch_high': catchhigh, 'handlers': hs})
    unw = [struct.unpack_from('<iI', MEM, unwind - P.base + 8 * i) for i in range(maxstate)]
    return {'magic': hex(magic), 'max_state': maxstate, 'unwind': [(a, hex(b)) for a, b in unw], 'try_blocks': blocks}


_ABS = None


def abs_refs(lo, hi):
    """All absolute dword references whose value lies in [lo, hi)."""
    global _ABS
    if _ABS is None:
        import array
        _ABS = {}
        for i in range(0, len(MEM) - 3):
            v = MEM[i] | MEM[i + 1] << 8 | MEM[i + 2] << 16 | MEM[i + 3] << 24
            if 0xa00000 <= v < 0xb50000:
                _ABS.setdefault(v, []).append(P.base + i)
    return {k: v for k, v in _ABS.items() if lo <= k < hi}
