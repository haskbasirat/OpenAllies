"""Original/v14 87C5C0 (87BDD0, 87C280, 875AC0) power-of-two texture pad as
5A6A10 calls it: target = next power of two (floats), border {0,0,0,0},
FormatPrefs B323B4, B323B0 = 0 (after renderer init 710433).

Executed: 875AC0 for the input PixelData (fixture construction) and the whole
87C5C0 including 87BDD0/87B930, 87C280 and the new 875AC0. Named seams: the
engine allocator 6F7810 (returns 0xCD-filled storage so unwritten bytes stay
visible) and 6F7430. Real rows: UI/Game/BuildingButtons/IconCorps.png and
IconBunker.png (42x42 8-bit RGB PNG; the native PNG reader canonicalizes to
RGBA, so the input texels are RGBA with alpha 255).
"""
from pathlib import Path
import hashlib, json, struct, sys, zlib
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'research/phase8-movement'))
import harness as h  # noqa: E402

DATA = ROOT.parent / 'reconstruction/assets/original/Data'
RGB = [0, 0, 255, 65280, 16711680, 0, 24, 8521824, 16640]
RGBA = [1, 0, 255, 65280, 16711680, 4278190080, 32, 545392769, 803072]
DXT1 = [4, 0, 0, 0, 0, 0, 0, 4, 0]
f = lambda v: struct.unpack('<I', struct.pack('<f', v))[0]


def png_rgba(path):
    b = path.read_bytes()
    assert b[:8] == b'\x89PNG\r\n\x1a\n'
    pos, idat, ihdr = 8, b'', None
    while pos < len(b):
        n = struct.unpack('>I', b[pos:pos + 4])[0]
        kind = b[pos + 4:pos + 8]
        body = b[pos + 8:pos + 8 + n]
        if kind == b'IHDR':
            ihdr = struct.unpack('>IIBBBBB', body)
        elif kind == b'IDAT':
            idat += body
        pos += 12 + n
    w, hgt, depth, color, _, _, lace = ihdr
    assert depth == 8 and color == 2 and lace == 0
    raw = zlib.decompress(idat)
    bpp, stride = 3, 3 * w
    rows, prev, i = [], bytearray(stride), 0
    for _ in range(hgt):
        ft = raw[i]; line = bytearray(raw[i + 1:i + 1 + stride]); i += 1 + stride
        for x in range(stride):
            a = line[x - bpp] if x >= bpp else 0
            up = prev[x]
            c = prev[x - bpp] if x >= bpp else 0
            if ft == 1: line[x] = (line[x] + a) & 255
            elif ft == 2: line[x] = (line[x] + up) & 255
            elif ft == 3: line[x] = (line[x] + (a + up) // 2) & 255
            elif ft == 4:
                p = a + up - c; pa, pb, pc = abs(p - a), abs(p - up), abs(p - c)
                line[x] = (line[x] + (a if pa <= pb and pa <= pc else up if pb <= pc else c)) & 255
        rows.append(bytes(line)); prev = line
    out = bytearray()
    for r in rows:
        for x in range(w):
            out += r[3 * x:3 * x + 3] + b'\xff'
    return w, hgt, bytes(out), hashlib.sha256(b).hexdigest()


def import_slots():
    b = h.P.b
    pe = struct.unpack_from('<I', b, 0x3c)[0]
    op = pe + 24
    base = struct.unpack_from('<I', b, op + 28)[0]
    def off(rva):
        for vs, r, raw, o in h.P.sections:
            if r <= rva < r + max(vs, raw):
                return o + rva - r
    imp = struct.unpack_from('<I', b, op + 96 + 8)[0]
    out, i = {}, 0
    while True:
        oft, ts, fc, name, ft = struct.unpack_from('<IIIII', b, off(imp) + 20 * i)
        if name == 0:
            return out
        k = 0
        while True:
            th = struct.unpack_from('<I', b, off(oft or ft) + 4 * k)[0]
            if th == 0:
                break
            if not th & 0x80000000:
                o = off(th) + 2
                out[base + ft + 4 * k] = b[o:b.index(bytes(1), o)].decode()
            k += 1
        i += 1


IAT = import_slots()


class Machine(h.Machine):
    def __init__(self, pc, image):
        super().__init__(pc)
        if image is not h.P:
            original_end = (h.P.image_size + 4095) & ~4095
            image_end = (image.image_size + 4095) & ~4095
            if image_end > original_end:
                self.u.mem_map(image.base + original_end, image_end - original_end)
            self.u.mem_write(image.base, image.b[:4096])
            for vs, r, raw, off in image.sections:
                if raw:
                    self.u.mem_write(image.base + r, image.b[off:off + min(raw, max(vs, raw))])
        self.allocations = []

        def new(_):
            size = self.arg()
            p = self.alloc(size + 16)
            self.u.mem_write(p, b'\xcd' * size)
            self.allocations.append(size)
            self.ret(0, p)
        self.seams[0x6f7810] = new
        self.seams[0x6f7430] = lambda _: self.ret()
        self.w8(0xb323b0, 0)
        # Kernel32 imports reached by 87B930's source-list lock.
        imports = {'GetCurrentThreadId': lambda _: self.ret(0, 0x1234),
                   'EnterCriticalSection': lambda _: self.ret(4),
                   'LeaveCriticalSection': lambda _: self.ret(4)}
        for slot, name in IAT.items():
            def missing(_, name=name):
                raise AssertionError('unseamed import ' + name)
            stub = h.STUBS + 0x800 + 16 * len(self.seams)
            self.seams[stub] = imports.get(name, missing)
            self.w32(slot, stub)

    def pixel_fields(self, p):
        levels = self.r32(p + 0x44)
        widths = self.r32(self.r32(p + 0x38), levels)
        heights = self.r32(self.r32(p + 0x3c), levels)
        offsets = self.r32(self.r32(p + 0x40), levels + 1)
        size = offsets[-1]
        data = bytes(self.u.mem_read(self.r32(p + 0x34), size)) if size else b''
        return dict(format=self.r32(p + 0xc, 9), levels=levels, widths=widths, heights=heights, offsets=offsets,
                    bpp=self.r32(p + 0x48), revision=self.r32(p + 0x4c), palette=self.r32(p + 0x30), refs=self.r32(p + 4),
                    bytes=data.hex())

    def run(self, fmt, w, hgt, levels, texels):
        old = self.alloc(0x60)
        fmt_ptr = self.alloc(0x24)
        self.w32(fmt_ptr, *fmt)
        self.call(0x875ac0, old, [w, hgt, fmt_ptr, levels])
        before = self.pixel_fields(old)
        size = before['offsets'][-1]
        payload = texels(size)
        self.u.mem_write(self.r32(old + 0x34), payload)
        pw = 1 << (w - 1).bit_length() if w > 1 else 1
        ph = 1 << (hgt - 1).bit_length() if hgt > 1 else 1
        target, border, prefs = self.alloc(8), self.alloc(16), self.alloc(12)
        self.w32(target, f(pw), f(ph))
        self.w32(border, 0, 0, 0, 0)
        self.w32(prefs, *[h.P.u32(0xb323b4 + 4 * i) for i in range(3)])
        source = self.call(0x87c5c0, 0, [old, target, border, prefs])
        new = self.r32(source + 0x40)
        return dict(target=[pw, ph], input=dict(before, bytes=payload.hex()),
                    source=dict(prefs=self.r32(source + 0x20, 3), pixel_is_new=new not in (0, old), refs=self.r32(source + 4)),
                    output=self.pixel_fields(new), old_after=dict(refs=self.r32(old + 4), palette=self.r32(old + 0x30)))


def cases():
    for name in ('IconCorps.png', 'IconBunker.png'):
        path = DATA / 'UI/Game/BuildingButtons' / name
        w, hgt, rgba, sha = png_rgba(path)
        yield dict(name='UI/Game/BuildingButtons/' + name, source_sha256=sha, format=RGBA, w=w, h=hgt, levels=1,
                   texels=lambda size, rgba=rgba: (rgba + b'\0' * size)[:size])
    pattern = lambda size: bytes((i * 37 + 11) & 255 for i in range(size))
    yield dict(name='rgba 3x5', format=RGBA, w=3, h=5, levels=1, texels=pattern)
    yield dict(name='rgb 5x3', format=RGB, w=5, h=3, levels=1, texels=pattern)
    yield dict(name='rgba 6x5 three levels', format=RGBA, w=6, h=5, levels=3, texels=pattern)
    yield dict(name='rgb 42x42', format=RGB, w=42, h=42, levels=1, texels=pattern)
    yield dict(name='rgba 1x3', format=RGBA, w=1, h=3, levels=2, texels=pattern)
    yield dict(name='dxt1 6x6 (unproven border)', format=DXT1, w=6, h=6, levels=1, texels=pattern)


def main():
    rows = []
    for case in cases():
        for pc in h.PRECISION:
            outs = [Machine(pc, image).run(case['format'], case['w'], case['h'], case['levels'], case['texels']) for image in (h.P, h.V)]
            assert outs[0] == outs[1], case['name']
            rows.append(dict(name=case['name'], source_sha256=case.get('source_sha256'), precision=pc, **outs[0]))
    static = []
    for a, n in ((0x87c5c0, 0x65), (0x87c280, 0x268), (0xb323b4, 12), (0x5a6b84, 0x52)):
        raw = h.P.range(a, n)
        assert raw == h.V.range(a, n)
        static.append(dict(address=hex(a), size=n, sha256=hashlib.sha256(raw).hexdigest()))
    executed = [dict(address=hex(a), bytes=b) for a, b in sorted(h.VISITED.items())]
    out = dict(schema=1, scope=__doc__, original_sha256=h.ORIGINAL_SHA, v14_sha256=h.V14_SHA, rows=rows,
               static_pins=static, executed_address_byte_pins=executed)
    raw = (json.dumps(out, indent=1) + '\n').encode()
    path = HERE / 'native-texture-resize.json'
    if '--write' in sys.argv:
        path.write_bytes(raw)
    else:
        assert path.read_bytes() == raw, 'fixture differs from fresh native replay'
    print(json.dumps(dict(rows=len(rows), machines=2 * len(rows), executed_pins=len(executed), sha256=hashlib.sha256(raw).hexdigest())))


if __name__ == '__main__':
    main()
