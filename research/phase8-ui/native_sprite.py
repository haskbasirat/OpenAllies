"""Unicorn oracle for the ORIGINAL AA.exe UI sprite vertex grid.

Executes the original x86 of the 2D sprite (vtable 0xA8ADC8):
  0x73EED0  SetRect (-> 0x740900: integral coordinates nudged by -0.1,
            divided by the screen size of the sprite manager +0x1B0/+0x1B4)
  0x73EF70  SetUV (field order (in0,in1,in2,in3) -> (u0,v1,u1,v0) at
            +0x40/+0x4C/+0x48/+0x44)
  0x73F750  vertex counts (vtable+4; 0x73F000/0x73F050 texture size with
            the >= 32 clamp, 0x73F180/0x73F150 span in design pixels)
  0x73FF60  vertex build (vtable+0x10): 0x73F1B0 positions, 0x73F290 UVs,
            packed colours
for the address modes 0 (texture-scaled), 1 (tiled, duplicated seam
vertices), 2 (tiled, shared vertices) and 3 (stretched) on both axes, with
and without a texture, at PC24 and PC53.

Objects built in emulated memory (no constructors run): the sprite fields
the code reads, the sprite manager (+0x1A8 z step, +0x1B0/+0x1B4 screen
size), the AVar table [0xB39990] (+0x20 far z, +0x88/+0x8C
InterfacePositioningWidth/Height) and a texture object whose vtable
+0x34/+0x38 return its GPU width/height as integers (two injected
`mov eax, [ecx+disp]; ret` stubs written once before any execution;
0x73F000 then takes ftol(width * |u1 - u0|)).
UI images are padded to a power of two (KKC_TextureResource 0x5A6A10:
+0x4C/+0x50 = image / padded size) and decal UVs are mapped into that
sub-rect by 0x5A6030, so the oracle feeds GPU sizes and GPU UVs.

Writes native-sprite.json next to this script.
"""
from pathlib import Path
import hashlib, json, struct, sys, itertools

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_layout import Machine, FPCW, SHA, f32bits  # noqa: E402

OUT = HERE / 'native-sprite.json'
VT_SPRITE = 0xA8ADC8
CAP = 16384


def fnv(values):
    h = 0xcbf29ce484222325
    for v in values:
        for b in struct.pack('<I', v & 0xffffffff):
            h ^= b
            h = (h * 0x100000001b3) & 0xffffffffffffffff
    return h


def f32(x):
    return struct.unpack('<f', struct.pack('<f', x))[0]


class SpriteMachine(Machine):
    def __init__(self, fpcw):
        super().__init__(fpcw)
        # texture getter stubs (int in eax): mov eax, [ecx+0x10]; ret /
        # mov eax, [ecx+0x14]; ret
        self.code = self.alloc(0x40)
        self.u.mem_write(self.code, bytes([0x8B, 0x41, 0x10, 0xC3]))
        self.u.mem_write(self.code + 8, bytes([0x8B, 0x41, 0x14, 0xC3]))
        self.tex_vt = self.alloc(0x40)
        self.w32(self.tex_vt + 0x34, self.code)
        self.w32(self.tex_vt + 0x38, self.code + 8)
        # AVar table: +0x20 far z, +0x88 IPW, +0x8C IPH
        self.avars = self.alloc(0x200)
        self.w32(0xB39990, self.avars)
        self.cells = {}
        for off in (0x20, 0x88, 0x8C):
            c = self.alloc(4)
            self.cells[off] = c
            self.w32(self.avars + off, c)
        self.mgr = self.alloc(0x200)
        self.verts = self.alloc(32 * CAP)

    def setup(self, screen, ipw_iph, far, zstep):
        self.wf(self.mgr + 0x1B0, screen[0])
        self.wf(self.mgr + 0x1B4, screen[1])
        self.wf(self.mgr + 0x1A8, zstep)
        self.wf(self.cells[0x20], far)
        self.wf(self.cells[0x88], ipw_iph[0])
        self.wf(self.cells[0x8C], ipw_iph[1])

    def texture(self, size):
        if size is None:
            return 0
        t = self.alloc(0x20)
        self.w32(t, self.tex_vt)
        self.w32(t + 4, 1)
        self.w32(t + 0x10, size[0])
        self.w32(t + 0x14, size[1])
        return t

    def run(self, rect, uv, tex, modes, layer, colour):
        s = self.alloc(0xA0)
        self.w32(s, VT_SPRITE)
        self.w32(s + 4, self.mgr)
        for i, c in enumerate(colour):
            self.wf(s + 0x20 + 4 * i, c)
        for i, c in enumerate(colour[:3]):
            self.wf(s + 0x30 + 4 * i, c)
        r = self.alloc(16)
        for i, v in enumerate(rect):
            self.wf(r + 4 * i, v)
        self.call(0x73EED0, s, [r], 4)
        q = self.alloc(16)
        for i, v in enumerate(uv):
            self.wf(q + 4 * i, v)
        self.call(0x73EF70, s, [q], 4)
        self.w32(s + 0x3C, self.texture(tex))
        self.w32(s + 0x50, modes[0])
        self.w32(s + 0x54, modes[1])
        self.w32(s + 0x58, layer)
        self.w32(s + 0x5C, self.verts)
        self.w32(s + 0x60, 0)
        self.w32(s + 0x64, CAP)
        nx, ny = self.alloc(4), self.alloc(4)
        n = self.call(0x73F750, s, [nx, ny], 8)
        assert n <= CAP, n
        self.call(0x73FF60, s, [n], 4)
        raw = bytes(self.u.mem_read(self.verts, 32 * n))
        words = struct.unpack('<%dI' % (8 * n), raw)
        out = [list(words[8 * k:8 * k + 8]) for k in range(n)]
        fields = [self.r32(s + o) for o in (0x10, 0x14, 0x18, 0x1C, 0x40, 0x44, 0x48, 0x4C)]
        # every vertex word in an FNV-1a 64 digest; the first and last 16
        # vertices in full for diagnostics
        keep = out if n <= 32 else out[:16] + out[-16:]
        return {'counts': [self.r32(nx), self.r32(ny)], 'n': n, 'sprite_bits': fields,
                'fnv': fnv(words), 'vertices': keep}


RECTS = [(0.0, 0.0, 1024.0, 768.0), (52.5, 262.5, 1255.0, 849.375), (100.25, 50.75, 160.5, 90.125),
         (10.0, 10.0, 12.0, 11.0), (300.0, 200.0, 1100.0, 260.0), (7.3, 1200.6, 2553.9, 1431.1)]
TEX = [None, (32, 32), (64, 16), (128, 256), (16, 64), (512, 512)]
UVS = [(0.0, 1.0, 1.0, 0.0), (0.0, 0.5, 0.5, 0.0), (0.5, 1.0, 1.0, 0.5), (0.0, 0.75, 0.9375, 0.0),
       (-0.125, 1.0, 1.125, 0.0), (0.25, 0.8, 0.75, 0.1)]
SCREENS = [((1024.0, 768.0), (1024.0, 768.0)), ((2560.0, 1440.0), (1024.0, 768.0)),
           ((1440.0, 900.0), (1024.0, 768.0))]
MODES = [0, 1, 2, 3]


def main():
    cases = []
    for prec, cw in FPCW.items():
        m = SpriteMachine(cw)
        mark = m.next
        for screen, ip in SCREENS:
            m.setup(screen, ip, 1.0, 1.0 / 16777216.0)
            for rect, tex, uv, mx, my in itertools.product(RECTS, TEX, UVS, MODES, MODES):
                if tex is None and (uv != UVS[0] or (mx, my) not in ((1, 1), (3, 3))):
                    continue
                if uv not in UVS[:2] and (mx, my) not in ((1, 1), (3, 3), (1, 3), (3, 1)):
                    continue
                m.next = mark
                layer = 7
                colour = (1.0, 0.5, 0.25, 0.75)
                res = m.run(rect, uv, tex, (mx, my), layer, colour)
                cases.append({'precision': prec, 'screen': list(screen), 'ip': list(ip),
                              'rect_bits': [f32bits(v) for v in rect], 'uv_bits': [f32bits(v) for v in uv],
                              'tex': list(tex) if tex else None, 'modes': [mx, my], 'layer': layer,
                              **res})
    report = {'schema': 1, 'original_sha256': SHA, 'scope': __doc__, 'far': 1.0, 'zstep_bits': f32bits(1.0 / 16777216.0),
              'colour': [1.0, 0.5, 0.25, 0.75], 'cases': cases}
    OUT.write_text(json.dumps(report, separators=(',', ':')) + '\n', encoding='ascii')
    print(len(cases), 'cases', hashlib.sha256(OUT.read_bytes()).hexdigest())


if __name__ == '__main__':
    main()
