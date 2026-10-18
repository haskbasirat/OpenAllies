"""Disassemble AA.exe ranges with Capstone (read-only helper).
usage: python aadis.py <va_hex> [count|end_hex]
"""
from pathlib import Path
import sys, struct
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT.parent / 'reconstruction/tools/python-packages'))
from capstone import Cs, CS_ARCH_X86, CS_MODE_32

class PE:
    def __init__(self, path):
        self.b = path.read_bytes()
        pe = struct.unpack_from('<I', self.b, 0x3c)[0]
        n = struct.unpack_from('<H', self.b, pe + 6)[0]
        op = pe + 24
        self.base = struct.unpack_from('<I', self.b, op + 28)[0]
        self.image_size = struct.unpack_from('<I', self.b, op + 56)[0]
        sec = op + struct.unpack_from('<H', self.b, pe + 20)[0]
        self.sections = [struct.unpack_from('<IIII', self.b, sec + i * 40 + 8) for i in range(n)]
    def offset(self, va):
        rva = va - self.base
        for vs, r, size, off in self.sections:
            if r <= rva < r + max(size, vs):
                return off + rva - r
        raise ValueError(hex(va))
    def range(self, va, size):
        o = self.offset(va)
        return self.b[o:o + size]
    def u32(self, va):
        return struct.unpack('<I', self.range(va, 4))[0]
    def f32(self, va):
        return struct.unpack('<f', self.range(va, 4))[0]
    def f64(self, va):
        return struct.unpack('<d', self.range(va, 8))[0]
    def wstr(self, va, n=200):
        raw = self.range(va, n * 2)
        out = []
        for i in range(0, len(raw), 2):
            c = raw[i] | raw[i + 1] << 8
            if c == 0: break
            out.append(chr(c))
        return ''.join(out)

P = PE(ROOT.parent / 'AA.exe')

def dis(va, end):
    md = Cs(CS_ARCH_X86, CS_MODE_32)
    code = P.range(va, end - va)
    for i in md.disasm(code, va):
        yield i

if __name__ == '__main__':
    va = int(sys.argv[1], 16)
    arg = sys.argv[2] if len(sys.argv) > 2 else '200'
    if arg.startswith('0x') or len(arg) >= 6:
        end = int(arg, 16)
        n = 100000
    else:
        end = va + 0x2000
        n = int(arg)
    for k, i in enumerate(dis(va, end)):
        if k >= n: break
        extra = ''
        for tok in i.op_str.replace('[', ' ').replace(']', ' ').replace(',', ' ').split():
            if tok.startswith('0x') and len(tok) >= 8:
                try:
                    v = int(tok, 16)
                    if 0x9f0000 <= v < 0xb40000:
                        s = P.wstr(v, 60)
                        if len(s) >= 2 and all(32 <= ord(c) < 127 for c in s):
                            extra += f'  ; L"{s}"'
                        else:
                            extra += f'  ; f32={P.f32(v):g} f64={P.f64(v):g} u32={P.u32(v):#x}'
                except Exception:
                    pass
        print(f'{i.address:08x}  {i.mnemonic:6} {i.op_str}{extra}')
