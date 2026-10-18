"""Read-only helpers for the phase-8 zone/detection/fog research (original AA.exe).

usage: tools.py dis <start-hex> [end-hex]     capstone listing (end defaults to functions.tsv size)
       tools.py vt <addr-hex> [n]             dump n vtable slots (default 24)
       tools.py u32 <addr-hex> [n]            dump n dwords
       tools.py str <addr-hex>                read a UTF-16/ASCII string
       tools.py callers <addr-hex>            callers from calls.tsv
       tools.py callees <addr-hex>            callees from calls.tsv
"""
from pathlib import Path
import struct, sys
HERE = Path(__file__).resolve().parent
GAME = HERE.parents[2]
NATIVE = GAME / 'reconstruction/research/native-aa'
sys.path.insert(0, str(GAME / 'reconstruction/tools/python-packages'))
B = (GAME / 'AA.exe').read_bytes()

def off(va):
    pe = struct.unpack_from('<I', B, 0x3c)[0]; n = struct.unpack_from('<H', B, pe + 6)[0]; op = pe + 24
    sec = op + struct.unpack_from('<H', B, pe + 20)[0]
    for i in range(n):
        vs, rva, rs, ro = struct.unpack_from('<IIII', B, sec + i * 40 + 8)
        if rva <= va - 0x400000 < rva + rs: return ro + va - 0x400000 - rva
    raise ValueError(hex(va))

def u32(va): return struct.unpack_from('<I', B, off(va))[0]

_sizes = None
def fsize(a):
    global _sizes
    if _sizes is None:
        _sizes = {}
        for line in open(NATIVE / 'functions.tsv', encoding='utf-8'):
            f = line.split('\t')
            if f[0] != 'address': _sizes[int(f[0], 16)] = int(f[3])
    return _sizes.get(a)

def dis(start, end=None):
    from capstone import Cs, CS_ARCH_X86, CS_MODE_32
    if end is None: end = start + (fsize(start) or 0x100)
    md = Cs(CS_ARCH_X86, CS_MODE_32)
    o = off(start)
    return ['%08x  %-24s %s %s' % (i.address, i.bytes.hex(), i.mnemonic, i.op_str) for i in md.disasm(B[o:o + end - start], start)]

def calls(col):
    out = []
    for line in open(NATIVE / 'calls.tsv', encoding='utf-8'):
        f = line.rstrip('\n').split('\t')
        out.append(f)
    return out

if __name__ == '__main__':
    cmd = sys.argv[1]
    if cmd == 'dis':
        print('\n'.join(dis(int(sys.argv[2], 16), int(sys.argv[3], 16) if len(sys.argv) > 3 else None)))
    elif cmd == 'vt':
        a = int(sys.argv[2], 16); n = int(sys.argv[3]) if len(sys.argv) > 3 else 24
        for i in range(n): print('+%02x  %08x' % (i * 4, u32(a + 4 * i)))
    elif cmd == 'u32':
        a = int(sys.argv[2], 16); n = int(sys.argv[3]) if len(sys.argv) > 3 else 8
        for i in range(n): print('%08x  %08x' % (a + 4 * i, u32(a + 4 * i)))
    elif cmd in ('callers', 'callees'):
        a = '%08x' % int(sys.argv[2], 16)
        for f in calls(0):
            if cmd == 'callers' and f[1] == a: print(f[0])
            if cmd == 'callees' and f[0] == a: print(f[1], f[2] if len(f) > 2 else '')

def find_dword(value):
    """All VAs whose aligned dword equals value (searches every section)."""
    pe = struct.unpack_from('<I', B, 0x3c)[0]; n = struct.unpack_from('<H', B, pe + 6)[0]; op = pe + 24
    sec = op + struct.unpack_from('<H', B, pe + 20)[0]
    out = []
    pat = struct.pack('<I', value)
    for i in range(n):
        vs, rva, rs, ro = struct.unpack_from('<IIII', B, sec + i * 40 + 8)
        data = B[ro:ro + rs]
        j = data.find(pat)
        while j >= 0:
            out.append(0x400000 + rva + j)
            j = data.find(pat, j + 1)
    return out

if __name__ == '__main__' and sys.argv[1] == 'xref':
    for v in sys.argv[2:]:
        print(v, ' '.join('%08x' % a for a in find_dword(int(v, 16))))

def find_calls(target):
    """VAs of E8/E9 rel32 instructions whose destination is target (text section scan)."""
    pe = struct.unpack_from('<I', B, 0x3c)[0]; n = struct.unpack_from('<H', B, pe + 6)[0]; op = pe + 24
    sec = op + struct.unpack_from('<H', B, pe + 20)[0]
    vs, rva, rs, ro = struct.unpack_from('<IIII', B, sec + 8)
    out = []
    data = B[ro:ro + rs]
    base = 0x400000 + rva
    for j in range(len(data) - 5):
        if data[j] in (0xe8, 0xe9):
            rel = struct.unpack_from('<i', data, j + 1)[0]
            if base + j + 5 + rel == target:
                out.append(base + j)
    return out

def containing(va):
    best = None
    for a in sorted(_load_sizes()):
        if a <= va: best = a
        else: break
    return best

def _load_sizes():
    fsize(0)
    return _sizes

if __name__ == '__main__' and sys.argv[1] == 'calls':
    for v in sys.argv[2:]:
        hits = find_calls(int(v, 16))
        print(v, ' '.join('%08x(in %08x)' % (h, containing(h) or 0) for h in hits))

def cstr(va, limit=200):
    o = off(va)
    e = B.index(b'\0', o)
    return B[o:min(e, o + limit)].decode('latin1')

def rtti_name(vtable):
    """MSVC RTTI class name of a vtable (COL at vtable-4), or None."""
    try:
        col = u32(vtable - 4)
        if u32(col) != 0: return None
        td = u32(col + 12)
        return cstr(td + 8)
    except Exception:
        return None

def vtable_of(slot_va, back=200):
    """Walk back from a slot address to the first slot whose [-4] has RTTI."""
    a = slot_va
    for _ in range(back):
        n = rtti_name(a)
        if n and n.startswith('.?A'): return a, n
        a -= 4
    return None, None

if __name__ == '__main__' and sys.argv[1] == 'class':
    for v in sys.argv[2:]:
        for ref in find_dword(int(v, 16)):
            vt, name = vtable_of(ref)
            print(v, '%08x' % ref, 'vtable %08x slot +%x %s' % (vt, ref - vt, name) if vt else '?')

def text_imm_refs(value):
    """VAs in .text where value appears as a 4-byte immediate after C7 (mov m32, imm32)."""
    pe = struct.unpack_from('<I', B, 0x3c)[0]; op = pe + 24
    sec = op + struct.unpack_from('<H', B, pe + 20)[0]
    vs, rva, rs, ro = struct.unpack_from('<IIII', B, sec + 8)
    data = B[ro:ro + rs]; base = 0x400000 + rva
    pat = struct.pack('<I', value); out = []
    j = data.find(pat)
    while j >= 0:
        if 0xc7 in data[max(0, j - 3):j]: out.append(base + j)
        j = data.find(pat, j + 1)
    return out

def vtable_start(slot_va, back=120):
    a = slot_va
    for _ in range(back):
        if text_imm_refs(a): return a
        a -= 4
    return None

if __name__ == '__main__' and sys.argv[1] == 'vstart':
    for v in sys.argv[2:]:
        for ref in find_dword(int(v, 16)):
            s = vtable_start(ref)
            if s:
                ctor = [containing(r) for r in text_imm_refs(s)]
                print(v, 'slot %08x vtable %08x +%x ctors %s' % (ref, s, ref - s, ' '.join('%08x' % (c or 0) for c in ctor)))
            else:
                print(v, 'slot %08x vtable ?' % ref)
