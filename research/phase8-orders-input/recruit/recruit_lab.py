"""Shared laboratory for the recruit order oracles (original AA.exe code only).

native_machine.Machine over either pinned image (original AA.exe or the v14
runtime image), a zeroing laboratory heap, MSVC CString laboratory headers
(manager at -0x10, length -0x0C, allocated -0x08, refcount -0x04) and
instruction coverage over named native function ranges. Nothing here writes
game files.
"""
from pathlib import Path
import json, struct, sys
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'research/phase8-ai'))
import native_machine as nm  # noqa: E402
sys.path.insert(0, str(nm.GAME / 'reconstruction/tools/python-packages'))
from capstone import Cs, CS_ARCH_X86, CS_MODE_32  # noqa: E402

IMAGES = {'original': nm.original, 'v14': nm.modified}


class Lab(nm.Machine):
    def __init__(self, precision, image):
        super().__init__(precision)
        self.image = image
        if image is nm.modified:
            u = self.u
            u.mem_unmap(nm.original.base, (nm.original.image_size + 0xfff) & ~0xfff)
            u.mem_map(nm.modified.base, (nm.modified.image_size + 0xfff) & ~0xfff)
            for vs, rva, raw, off in nm.modified.sections:
                u.mem_write(nm.modified.base + rva, nm.modified.b[off:off + raw])
        self.frees = []
        # CString manager whose vt+4 frees a header (recorded, never reused).
        self.manager = self.alloc(0x20)
        self.w(self.manager, self.array([0, self.stub(self._string_free), 0, self.stub(self._nil_string)] + [0] * 4))

    def alloc(self, size, align=16):
        p = super().alloc(size, align)
        self.u.mem_write(p, bytes(max(size, 4)))
        return p

    def cstring(self, text):
        """Laboratory CString: returns the character pointer."""
        units = list(text.encode('utf-16-le'))
        n = len(units) // 2
        h = self.alloc(0x10 + 2 * (n + 1))
        self.w(h, self.manager)
        self.w(h + 4, n)
        self.w(h + 8, n)
        self.w(h + 12, 1)
        self.u.mem_write(h + 0x10, bytes(units) + b'\0\0')
        return h + 0x10

    def wide(self, p, limit=2000):
        out = []
        for i in range(limit):
            c = struct.unpack('<H', self.u.mem_read(p + 2 * i, 2))[0]
            if c == 0:
                break
            out.append(c)
        return out

    def text(self, p):
        u = self.wide(p)
        return struct.pack('<%dH' % len(u), *u).decode('utf-16-le', 'surrogatepass')

    def _string_free(self, m):
        m.frees.append(m.arg(1))
        m.ret(0, pop=4)

    def _nil_string(self, m):
        # Manager vt+C (856370's returned manager): a fresh empty header.
        h = m.alloc(0x14)
        m.w(h, m.manager)
        m.w(h + 12, 1)
        m.ret(h)


def coverage(functions, visited):
    """Executed/total instruction counts of each [start, end) native range."""
    md = Cs(CS_ARCH_X86, CS_MODE_32)
    out = {}
    for name, (start, end) in functions.items():
        code = nm.original.range(start, end - start)
        total = [i.address for i in md.disasm(code, start) if i.mnemonic != 'int3']
        hit = [a for a in total if a in visited]
        out[name] = {'executed': len(hit), 'instructions': len(total),
                     'unexecuted': [hex(a) for a in total if a not in visited]}
    return out


def write_or_replay(path, document, write):
    text = json.dumps(document, indent=1, sort_keys=False) + '\n'
    if write:
        if path.exists():
            raise SystemExit(f'{path} exists; refusing to overwrite')
        path.write_text(text, encoding='utf-8', newline='\n')
        print('wrote', path)
    else:
        old = path.read_text(encoding='utf-8')
        assert old == text, f'{path} replay differs'
        print('replay identical', path)
