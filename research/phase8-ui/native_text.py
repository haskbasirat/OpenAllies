"""Unicorn oracle for the ORIGINAL AA.exe text measure / ellipsis code
(written by the phase-8 text research subagent in scratch-text2/, promoted
here unchanged except for paths; run from rust-rebuild, ~5.5 min).

Executes natively (x86 under Unicorn, AA.exe sha256 checked by native_layout):
  0x7352C0 KKC_RText::GetTextExtent(text*, size*, font, px, fixed, trail,
           wrap_w, rows*, clip)            (cdecl, 9 args)
  0x735C00 reference-unit wrapper (wrap_w * sx, then size / sx, / sy)
  0x735D20 world-unit wrapper (px = 0)
  0x735D50 ellipsis truncation (list items 0x5D3E10, label 0x5F29F0)
  and everything they call: tokenizer 0x731790/0x732720/0x731e80, markup
  0x7335B0/0x7331E0, glyph getters 0x7051A0/0x7055D0/0x705580/0x7055F0
  (run natively over a synthetic packed glyph table - the getters are NOT
  seamed, the table is the input), range lookups 0x705800/0x705840/
  0x705870, the ellipsis cache 0x7080B0, visible length 0x732C60,
  markup-aware Left 0x732E60, scale 0x5C1DB0/0x5C1DD0, ATL CString code
  (0x4012C0, 0x401330, 0x4011E0, 0x492F70, 0x4931E0, 0x405A50, 0x405B80,
  0x401B00, 0x41D2F0...), CRT wcschr/wcslen/iswspace/memcpy.

Seams (everything else that would leave the image):
  * CString manager: the static managers 0xB31E30/0xB31E50 get a stub
    vtable (+0 Allocate, +4 Free, +8 Reallocate: Python bump allocator;
    +0xC GetNilString 0x85CC30 and +0x10 Clone 0x8D6180 stay native).
  * 0x96CAD6 _wcsicmp: needs _getptd (TLS). Seam = the C-locale path of
    the same function (fold only 'A'..'Z', return a-b of the first mismatch).
  * 0x96D3E8 _wtof: native_layout's MSVC atof seam (Color/SetX/SetY values).
  * 0x709280 font lookup by name (<Font=x>): returns a test font or 0.
  * 0x4B9F40 localization lookup: grammar_ellipsis -> "..." (strings_rtse.tgi).
  * every IAT slot is replaced by a trap that fails the case; operator
    new/delete (0x6F7810/0x6F7430) stop the run (Machine).
The Python model (model_extent / model_ellipsis) is the algorithm as read
from the instructions; every native result is compared with it bit-exactly.
"""
from pathlib import Path
import json, struct, sys, hashlib

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import native_layout as NL  # noqa: E402
from native_layout import P, f32bits, bits_f32, msvc_atof, FPCW  # noqa: E402
from unicorn import Uc, UcError, UC_ARCH_X86, UC_MODE_32, UC_HOOK_CODE, UC_HOOK_MEM_READ, UC_HOOK_MEM_WRITE, UC_HOOK_MEM_FETCH_UNMAPPED  # noqa: E402
from unicorn.x86_const import *  # noqa: E402,F401,F403

STUB = 0x2c00000
TRAP = 0x10000000
HEAP = 0x20000000
STACK = HEAP + 0x3ff0000
STOP = 0x2d00000
INSN_LIMIT = 4_000_000
GROWTH_LIMIT = 4000     # chars: a CString this long only comes from the wrap loop


class Hang(Exception):
    """Native code did not return: instruction budget exhausted or the
    token CString grew past GROWTH_LIMIT (endless newline insertion)."""
MGR_A, MGR_B = 0xb31e30, 0xb31e50
OUT = HERE / 'native-text.json'


def f32(x):
    return struct.unpack('<f', struct.pack('<f', x))[0]


def imports():
    b = P.b
    pe = struct.unpack_from('<I', b, 0x3c)[0]
    imp_rva = struct.unpack_from('<I', b, pe + 24 + 104)[0]
    out = []

    def cstr(va):
        s = b''
        while True:
            c = P.range(va, 1)
            if c == b'\0':
                return s.decode('latin1')
            s += c
            va += 1
    va = P.base + imp_rva
    while True:
        oft, ts, fc, name, ft = struct.unpack('<IIIII', P.range(va, 20))
        if name == 0:
            break
        dll = cstr(P.base + name)
        thunk = oft or ft
        i = 0
        while True:
            t = struct.unpack('<I', P.range(P.base + thunk + 4 * i, 4))[0]
            if t == 0:
                break
            fn = 'ord%d' % (t & 0xffff) if t & 0x80000000 else cstr(P.base + t + 2)
            out.append((P.base + ft + 4 * i, dll + '!' + fn))
            i += 1
        va += 20
    return out


class TextMachine:
    """Standalone Unicorn machine (same rules as native_layout.Machine:
    page 0 faults except the FS:[0] SEH word, operator new/delete stop the
    run) but seams use per-address hooks and IAT slots point at an unmapped
    trap range, so an import call faults instead of running garbage."""

    def __init__(self, fpcw):
        u = self.u = Uc(UC_ARCH_X86, UC_MODE_32)
        u.mem_map(0, 0x1000)
        self.null_accesses = []
        u.hook_add(UC_HOOK_MEM_READ | UC_HOOK_MEM_WRITE, self._null, begin=0, end=0xfff)
        u.mem_map(P.base, (P.image_size + 0xfff) & ~0xfff)
        for vs, rva, size, off in P.sections:
            u.mem_write(P.base + rva, P.b[off:off + size])
        u.mem_map(HEAP, 0x4000000)
        self.next = HEAP + 0x1000
        self.fpcw = fpcw
        self.stub = 0x2e00000          # fld qword [stub+0x10]; ret  (wtof result)
        u.mem_map(self.stub, 0x1000)
        u.mem_write(self.stub, bytes([0xDD, 0x05]) + struct.pack('<I', self.stub + 0x10) + bytes([0xC3]))
        u.mem_map(STUB, 0x1000)
        u.mem_write(STUB, bytes([0xC3]) * 0x1000)
        self.seam_log = []
        self.trap_hits = []
        self.heap_calls = []
        self.imports = {}
        for k, (slot, name) in enumerate(imports()):
            a = TRAP + 0x10 * k
            self.imports[a] = name
            self.w32(slot, a)
        u.hook_add(UC_HOOK_MEM_FETCH_UNMAPPED, self._fetch_unmapped)
        vt = STUB + 0x100
        self.alloc_stub, self.free_stub, self.realloc_stub = STUB, STUB + 0x10, STUB + 0x20
        for i, f in enumerate([self.alloc_stub, self.free_stub, self.realloc_stub, 0x85cc30, 0x8d6180]):
            self.w32(vt + 4 * i, f)
        for mgr in (MGR_A, MGR_B):
            self.w32(mgr, vt)
            self.w32(mgr + 4, mgr)
            self.w32(mgr + 8, 0)
            self.w32(mgr + 12, 0)
            self.w32(mgr + 16, 0x40000000)
            self.w32(mgr + 20, 0)
        self.seams = {self.alloc_stub: self._s_alloc, self.free_stub: self._s_free,
                      self.realloc_stub: self._s_realloc, 0x96cad6: self._s_wcsicmp,
                      0x709280: self._s_font_by_name, 0x4b9f40: self._s_localize,
                      0x96D3E8: self._s_wtof}
        for a in (0x6F7810, 0x6F7430):
            self.seams[a] = (lambda a=a: self._heap(a))
        for a, fn in self.seams.items():
            u.hook_add(UC_HOOK_CODE, (lambda uc, pc, size, user, fn=fn: fn()), begin=a, end=a)
        self.fonts_by_name = {}
        self.loc = {'grammar_ellipsis': '...'}

    def _null(self, u, access, address, size, value, _):
        pc = u.reg_read(UC_X86_REG_EIP)
        if address + size > 4 or not self._fs_prefixed(pc):
            self.null_accesses.append((pc, address, size))
            u.emu_stop()

    def _fs_prefixed(self, pc):
        for byte in bytes(self.u.mem_read(pc, 4)):
            if byte == 0x64:
                return True
            if byte not in (0x26, 0x2e, 0x36, 0x3e, 0x65, 0x66, 0x67, 0xf0, 0xf2, 0xf3):
                return False
        return False

    def _fetch_unmapped(self, u, access, address, size, value, _):
        self.trap_hits.append(self.imports.get(address, hex(address)))
        return False

    def _heap(self, addr):
        self.heap_calls.append(addr)
        self.u.emu_stop()

    def alloc(self, n):
        p = self.next
        self.next += (n + 15) & ~15
        assert self.next < HEAP + 0x3f00000
        self.u.mem_write(p, bytes(n))
        return p

    def w32(self, a, v):
        self.u.mem_write(a, struct.pack('<I', v & 0xffffffff))

    def r32(self, a):
        return struct.unpack('<I', self.u.mem_read(a, 4))[0]

    def wf(self, a, x):
        self.u.mem_write(a, struct.pack('<f', x))

    def ret(self, value=0, pop=0):
        sp = self.u.reg_read(UC_X86_REG_ESP)
        self.u.reg_write(UC_X86_REG_EAX, value)
        self.u.reg_write(UC_X86_REG_EIP, self.r32(sp))
        self.u.reg_write(UC_X86_REG_ESP, sp + 4 + pop)

    def arg(self, n):
        return self.r32(self.u.reg_read(UC_X86_REG_ESP) + 4 + 4 * n)

    def call(self, pc, ecx, args, pop):
        sp = STACK - 0x100
        self.w32(sp, STOP)
        for i, a in enumerate(args):
            self.w32(sp + 4 + 4 * i, a)
        self.u.reg_write(UC_X86_REG_ESP, sp)
        self.u.reg_write(UC_X86_REG_EBP, STACK - 0x10)
        self.u.reg_write(UC_X86_REG_ECX, ecx)
        self.u.reg_write(UC_X86_REG_FPCW, self.fpcw)
        self.hang = False
        try:
            self.u.emu_start(pc, STOP, count=INSN_LIMIT)
        except UcError as e:
            raise AssertionError('emulation error %s at %#x traps=%s' % (e, self.u.reg_read(UC_X86_REG_EIP), self.trap_hits))
        assert not self.null_accesses, f'null-pointer access {self.null_accesses}'
        assert not self.heap_calls, f'allocator reached {self.heap_calls}'
        assert not self.trap_hits, self.trap_hits
        if self.hang or self.u.reg_read(UC_X86_REG_EIP) != STOP:
            raise Hang(hex(self.u.reg_read(UC_X86_REG_EIP)))
        assert self.u.reg_read(UC_X86_REG_EIP) == STOP, hex(self.u.reg_read(UC_X86_REG_EIP))
        assert self.u.reg_read(UC_X86_REG_ESP) == sp + 4 + pop
        return self.u.reg_read(UC_X86_REG_EAX)

    # ---------------------------------------------------------- seams
    def _hdr(self, n):
        h = self.alloc(16 + 2 * (n + 1) + 16)
        self.w32(h, MGR_B)
        self.w32(h + 4, 0)
        self.w32(h + 8, n)
        self.w32(h + 12, 1)
        return h

    def _s_alloc(self):
        n, cs = self.arg(0), self.arg(1)
        assert cs == 2, cs
        if n > GROWTH_LIMIT:
            self.hang = True
            self.u.emu_stop()
            return
        h = self._hdr(n)
        self.w32(h, self.u.reg_read(UC_X86_REG_ECX))
        self.seam_log.append('Allocate')
        self.ret(h, 8)

    def _s_free(self):
        self.seam_log.append('Free')
        self.ret(0, 4)

    def _s_realloc(self):
        old, n, cs = self.arg(0), self.arg(1), self.arg(2)
        assert cs == 2
        if n > GROWTH_LIMIT:
            self.hang = True
            self.u.emu_stop()
            return
        oalloc = self.r32(old + 8)
        h = self._hdr(n)
        data = bytes(self.u.mem_read(old, 16 + 2 * (min(oalloc, n) + 1)))
        self.u.mem_write(h, data)
        self.w32(h + 8, n)
        self.seam_log.append('Reallocate')
        self.ret(h, 0xc)

    def wstr(self, p):
        out = []
        while True:
            c = struct.unpack('<H', self.u.mem_read(p + 2 * len(out), 2))[0]
            if c == 0:
                return ''.join(out)
            out.append(chr(c))

    def _s_wcsicmp(self):
        a, b = self.wstr(self.arg(0)) + '\0', self.wstr(self.arg(1)) + '\0'
        self.seam_log.append('_wcsicmp')
        fold = lambda c: ord(c) + 0x20 if 'A' <= c <= 'Z' else ord(c)
        r = 0
        for x, y in zip(a, b):
            if fold(x) != fold(y) or x == '\0':
                r = fold(x) - fold(y)
                break
        self.ret(r & 0xffffffff, 0)

    def _s_wtof(self):
        self.seam_log.append('_wtof')
        p = self.arg(0)
        self.u.mem_write(self.stub + 0x10, struct.pack('<d', msvc_atof(self.wstr(p))))
        self.u.reg_write(UC_X86_REG_EIP, self.stub)

    def _s_font_by_name(self):
        name = self.wstr(self.r32(self.arg(0)))
        self.seam_log.append('font_by_name(%s)' % name)
        self.ret(self.fonts_by_name.get(name, 0), 4)

    def _s_localize(self):
        slot = self.arg(0)
        key = self.wstr(self.r32(slot))
        self.seam_log.append('localize(%s)' % key)
        self.w32(slot, self.cstring(self.loc[key]))
        self.ret(slot, 8)

    # ---------------------------------------------------------- data
    def cstring(self, text):
        raw = text.encode('utf-16-le') + b'\0\0'
        h = self.alloc(16 + len(raw) + 16)
        self.w32(h, MGR_B)
        self.w32(h + 4, len(text))
        self.w32(h + 8, len(text))
        self.w32(h + 12, 1)
        self.u.mem_write(h + 16, raw)
        return h + 16

    def read_cstring(self, data):
        n = self.r32(data - 12)
        return bytes(self.u.mem_read(data, 2 * n)).decode('utf-16-le')

    def setup_world(self, screen, scale_factor=25.0):
        m = self
        av = m.alloc(0x200)
        for off, val in ((0x88, 1024.0), (0x8c, 768.0), (0xa0, scale_factor)):
            cell = m.alloc(4)
            m.wf(cell, val)
            m.w32(av + off, cell)
        m.w32(0xb39990, av)
        m.w32(0xb396d0, screen[0])
        m.w32(0xb396d4, screen[1])
        m.w32(0xb3653c, m.alloc(0x100))
        m.w32(0xb36544, m.alloc(0x100))
        # font system (DAT_00b39bc0)
        fs = m.alloc(0x100)
        m.u.mem_write(fs + 0x1c, struct.pack('<5H', ord('<'), ord('>'), 149, ord('#'), ord('t')))
        m.w32(0xb39bc0, fs)
        self.fs = fs
        # markup / remap globals (0x707D70 / 0x732A50 / 0x732B00 / 0x732340)
        m.w32(0xb405f0, m.cstring('<'))
        m.w32(0xb405f4, m.cstring('>'))
        m.w32(0xb405f8, m.cstring('<>'))
        m.w32(0xb405fc, m.cstring('<>"\\&'))
        m.w32(0xb40600, m.cstring('\x11\x12\x13\x14\x15'))
        # character sets (DAT_00b36530 + 0x3c0): base 32..383 undisplayable
        # 149, spacing_standin 't'; han 0x4e00..0x9fff undisplayable 152
        app = m.alloc(0x400)
        lst = m.alloc(0x40)
        arr = m.alloc(0x10)
        sets = []
        for lo, hi, und, sp in ((32, 383, 149, 116), (0x4e00, 0x9fff, 152, 116)):
            s = m.alloc(0x20)
            m.u.mem_write(s + 0x18, struct.pack('<4H', lo, hi, und, sp))
            sets.append(s)
        for i, s in enumerate(sets):
            m.w32(arr + 4 * i, s)
        m.w32(lst + 0x24, arr)
        m.w32(lst + 0x28, len(sets))
        m.w32(app + 0x3c0, lst)
        m.w32(0xb36530, app)

    def make_font(self, glyphs, default=False, name=None):
        """glyphs: {codepoint: (w, h, shadow, outline)} packed exactly as
        0x705020 does (x/y 0, texture index 0)."""
        m = self
        f = m.alloc(0x30 + 6 * 0x10000 + 0x40)
        table = bytearray(6 * 0x10000)
        for c in range(0x10000):
            struct.pack_into('<HI', table, 6 * c, 0x3fff, 0)
        maxw = 0
        for c, (w, h, sh, ol) in glyphs.items():
            u16 = (((ol & 1) << 1 | (sh & 1)) << 14) ^ 0
            u32 = ((((h & 0x7f) << 7 | (w & 0x7f)) << 9) << 9)
            struct.pack_into('<HI', table, 6 * c, u16, u32 & 0xffffffff)
            maxw = max(maxw, w)
        m.u.mem_write(f + 0x30, bytes(table))
        tex = m.alloc(0x10)
        m.w32(tex, m.alloc(0x40))           # texture 0 (non-null = glyph present)
        m.w32(f + 0x24, tex)
        m.w32(f + 0x28, 1)
        m.w32(f + 0xc, maxw + 2)            # 0x708F66: max cell width + 2
        if default:
            m.w32(self.fs + 0x10, f)
        if name:
            self.fonts_by_name[name] = f
        return f

    def run(self, pc, args):
        self.seam_log = []
        return self.call(pc, 0, args, 0)


# ---------------------------------------------------------------- model
def is_nan(x):
    return x != x


class Model:
    """The algorithm of 0x7352C0 as read from the instructions."""

    def __init__(self, pc, fonts, default_font, fs, sets, scale_factor, screen):
        self.pc = pc            # 'pc24' or 'pc53'
        self.fonts = fonts      # id -> {'glyphs': {...}, 'fixed': int, 'ell': [x, y]}
        self.default = default_font
        self.fs = fs
        self.sets = sets
        self.scale = f32(scale_factor)
        self.screen = screen

    def r(self, x):             # x87 register rounding at the precision control
        return f32(x) if self.pc == 'pc24' else x

    # glyph table queries ------------------------------------------------
    def has(self, font, c):
        return c in self.fonts[font]['glyphs']

    def adv(self, font, c):     # 0x7051A0
        w, h, sh, ol = self.fonts[font]['glyphs'][c]
        return w - 1 if sh else w

    def line_h(self, font):     # 0x705580: height of the 'A' entry
        g = self.fonts[font]['glyphs'].get(65)
        return g[1] if g else 0

    def set_of(self, c):        # 0x705800
        for lo, hi, und, sp in self.sets:
            if lo <= c <= hi:
                return und, sp
        return None

    # tokenizer 0x732720 with delimiters --------------------------------
    @staticmethod
    def next_token(text, pos, delims, skip_ws):
        n = len(text)
        if pos >= n:
            return '', pos
        c = text[pos]
        if skip_ws:
            while c in ' \t\n\v\f\r':
                pos += 1
                if pos >= n:
                    return '', pos
                c = text[pos]
        while c in delims:
            pos += 1
            if pos >= n:
                return '', pos
            c = text[pos]
        tok = ''
        while True:
            tok += c
            pos += 1
            c = text[pos] if pos < n else '\0'
            if c != '\0' and c in delims:
                return tok, pos + 1
            if pos >= n:
                return tok, pos

    def markup(self, tok, st):
        """0x7335B0/0x7331E0 restricted to what GetTextExtent observes."""
        r = 7
        pos = 0
        while True:
            sub, pos = self.next_token(tok, pos, ';', True)
            if not sub:
                return r
            k, p2 = self.next_token(sub, 0, '=', True)
            v, _ = self.next_token(sub, p2, '=', True)
            k, v = k.strip(), v.strip()
            kl = k.lower()
            if kl == 'h_on':
                r = 0
            elif kl == 'h_off':
                r = 1
            elif kl == 'font':
                st['font'] = self.font_by_name.get(v, 0)
                r = 2
            elif kl in ('rc', 'restorecolor'):
                r = 4
            elif kl == 'color':
                r = 3
            elif kl == 'setx':
                x = msvc_atof(v)
                if x < st['pen']:
                    sp = float(self.adv(st['font'], 32)) if st['font'] else 4.0
                    x = self.r(sp + st['pen'])
                st['pen'] = f32(x)
                r = 5
            elif kl == 'sety':
                st['yacc'] = f32(msvc_atof(v))
                r = 6
            else:
                r = 7

    def extent(self, text, size_in, font, px, fixed, trail, wrap, clip):
        R = self.r
        ow, oh = size_in
        sx = sy = 0.0
        rows = 0
        new_row = True
        pen = 0.0       # 0x20
        yacc = 0.0      # 0x24
        row_w = 0.0     # 0x30
        row_h = 0.0     # 0x34
        trailing = 0.0  # 0x38
        xstart = 0.0    # 0x40
        last_space = -1
        wrapped = False
        count = 0
        st = {'font': font, 'pen': 0.0, 'yacc': 0.0}
        pos = 0
        in_markup = text[:1] == '<'
        early = None
        while early is None:
            tok_start = pos
            tok, pos = self.next_token(text, pos, '<>', False)
            if not tok:
                break
            if in_markup:
                st['font'], st['pen'], st['yacc'] = font, pen, yacc
                r = self.markup(tok, st)
                font, pen, yacc = st['font'], st['pen'], st['yacc']
                if r == 5:
                    row_w = pen
                    xstart = pen
                in_markup = text[pos:pos + 1] == '<'
                continue
            in_markup = pos < len(text)
            tok = list(tok)
            i = 0
            while i < len(tok):
                ch = ord(tok[i])
                rem = '\x11\x12\x13\x14\x15'.find(chr(ch))
                if rem >= 0:
                    ch = ord('<>"\\&'[rem])
                if font == 0:
                    font = self.default
                g = ch
                if ch in (10, 13, 14):
                    s = self.set_of(ch)
                    g = s[1] if s else self.fs['standin']
                if new_row:
                    rows += 1
                    new_row = False
                if not self.has(font, g):
                    s = self.set_of(g)
                    g = s[0] if s else self.fs['undisplayable']
                    if not self.has(font, g):
                        g = self.fs['fallback']
                if px:
                    adv = float(self.fonts[font]['fixed']) if fixed else float(self.adv(font, g))
                    h = float(self.line_h(font))
                else:
                    if fixed:
                        adv = f32(R(self.fonts[font]['fixed'] / self.scale))
                    else:
                        adv = f32(R(self.adv(font, g) / self.scale))
                    h = R(self.line_h(font) / self.scale)
                if ch == 10:
                    if clip and wrap == 0.0:
                        if pen > sx:
                            sx = pen
                        early = True
                        break
                    if row_w > sx:
                        sx = row_w
                    sy = f32(R((h if row_h == 0.0 else row_h) + sy))
                    trailing = 0.0
                    if not wrapped:
                        xstart = 0.0
                    row_w = pen = xstart
                    wrapped = False
                    new_row = True
                    yacc = f32(R(yacc + h))
                    if clip and R(yacc + row_h) > oh:
                        early = True
                        break
                    row_h = 0.0
                    i += 1
                    continue
                if ch == 32:
                    if clip and R(adv + pen) > ow:
                        if pen > sx:
                            sx = pen
                        if wrap == 0.0 or R(yacc + row_h) > oh:
                            early = True
                            break
                    pen = f32(R(adv + pen))
                    count += 1
                    last_space = i + tok_start
                    trailing = f32(R(adv + trailing))
                    i += 1
                    continue
                if wrap != 0.0 and R(adv + pen) > wrap:
                    new_row = True
                    if last_space == -1:
                        # 0x7357DB: insert a newline before the glyph. If the
                        # glyph is wider than the wrap width the newline resets
                        # pen to 0 (not a wrapped newline: xstart cleared) and the
                        # same glyph fails again: endless loop unless the clip
                        # test (0x735769) ends the call.
                        if len(tok) > GROWTH_LIMIT:
                            return {'hang': True}
                        tok.insert(i, '\n')
                        i -= 1
                    else:
                        s = last_space - tok_start
                        if s < 0:
                            count = count - i
                            tok.insert(0, '\n')
                            tok_start -= 1
                            i = -1
                            wrapped = True
                            last_space = -1
                        else:
                            tok[s] = '\n'
                            count = count + (s - i) + 1
                            i = s - 1
                            wrapped = True
                            last_space = -1
                    i += 1
                    continue
                if clip and R(adv + pen) > ow:
                    if pen > sx:
                        sx = pen
                    if wrap == 0.0 or R(yacc + row_h) > oh:
                        early = True
                        break
                pen = f32(R(adv + pen))
                count += 1
                row_w = f32(R(R(adv + trailing) + row_w))
                trailing = 0.0
                if h > row_h:
                    row_h = f32(h)
                i += 1
        if early:
            return {'ret': count, 'size': [sx, sy], 'rows': None}
        if row_w > 0.0 and row_w > sx:
            sx = row_w
        sy = f32(R(row_h + sy))
        if trail:
            sx = f32(R(trailing + sx))
        return {'ret': count, 'size': [sx, sy], 'rows': rows}

    def ref_extent(self, text, font, fixed, trail, width):   # 0x735C00
        sxs = self.r(self.screen[0] / f32(1024.0))
        sys_ = self.r(self.screen[1] / f32(768.0))
        w = f32(self.r(sxs * width))
        res = self.extent(text, (0.0, 0.0), font, 1, fixed, trail, w, False)
        x, y = res['size']
        res['size'] = [f32(self.r(x / sxs)), f32(self.r(y / sys_))]
        return res

    def visible_len(self, text):                             # 0x732C60
        pos, n, in_m = 0, 0, text[:1] == '<'
        while True:
            tok, pos = self.next_token(text, pos, '<>', False)
            if not tok:
                return n
            if in_m:
                in_m = text[pos:pos + 1] == '<'
            else:
                in_m = pos < len(text)
                n += len(tok)

    def left(self, text, cnt):                               # 0x732E60(.., 1)
        out, acc, pos, in_m = '', 0, 0, text[:1] == '<'
        while True:
            tok, pos = self.next_token(text, pos, '<>', False)
            if not tok:
                return out
            if not in_m:
                in_m = pos < len(text)
                if cnt <= len(tok) + acc:
                    return out + tok[:max(cnt - acc, 0)]
                out += tok
                acc += len(tok)
            else:
                out += '<' + tok + '>'
                in_m = text[pos:pos + 1] == '<'

    def ellipsis(self, text, w, h, font):                    # 0x735D50
        n = self.extent(text, (w, h), font, 1, 0, 0, 0.0, True)['ret']
        if n == self.visible_len(text):
            return 0, text
        e = self.fonts[font]['ell']
        if e[0] == 0.0:
            e[:] = self.extent('...', (0.0, 0.0), font, 1, 0, 0, 0.0, False)['size']
        w2 = self.r(w - e[0])
        if not w2 > 0.0:
            return 1, text
        n2 = self.extent(text, (f32(w2), h), font, 1, 0, 0, 0.0, True)['ret']
        return 1, self.left(text, n2) + '...'


# ---------------------------------------------------------------- cases
def glyph_table(seed_h=17, shadow=1):
    """Synthetic but realistic per-glyph cell widths (ceil(advance)+1 with
    shadow), all cells seed_h high like GDI+ single-char measures."""
    g = {}
    for c in range(32, 384):
        w = 3 + (c * 7 + 3) % 11            # 3..13
        if c == 32:
            w = 5
        g[c] = (w, seed_h, shadow, 0)
    g[ord('W')] = (17, seed_h, shadow, 0)
    g[ord('M')] = (15, seed_h, shadow, 0)
    for c in (128, 129, 133):               # special glyph images 17x17
        g[c] = (17, 17, shadow, 0)
    return g


TEXTS = [
    'Hello',
    'Hello world',
    'The quick brown fox jumps over the lazy dog',
    'Line one\nLine two',
    'A\n\nB',
    '\nLeading newline',
    'Trailing spaces   ',
    '   leading',
    'Supercalifragilisticexpialidocious',
    'Two  spaces  between  words here',
    'Carriage\r\nreturn',
    'Remap \x11tag\x12 and \x13quote\x13 \x14 \x15',
    'Missing 一 han and Ѐ cyrillic',
    'Money \x80 ammo \x81',
    '<Color=1,0,0,1>Red<RC> normal',
    '<Font=big>Big text<Font=small> small',
    'Before<SetX=120>After set x',
    'Wide<SetX=10>after backwards setx',
    '<SetY=40>Line\nnext',
    '<h_on>high<h_off> lighted',
    'A<b>c',
    'Wrap<Color=0,1,0>inside a markup token boundary here',
    'x' * 60,
    'Word ' * 20,
    '',
    '<Color=1,1,1>',
    '<SetX=20>aaaa bbbb cccc dddd eeee',
    '<SetX=20>' + 'y' * 30,
    'ab<SetX=30;Color=1,0,0>cd ef gh ij',
]
WRAPS = [0.0, 30.0, 61.0, 100.0, 157.5]
WRAPS_WORLD = [0.0, 0.5, 1.25, 2.37]


def font_specs():
    small = glyph_table(17, 1)
    big = {c: (w + 4, 23, 1, 0) for c, (w, h, s, o) in small.items()}
    noshadow = {c: (w, 15, 0, 0) for c, (w, h, s, o) in small.items()}
    del noshadow[149]                      # undisplayable falls back to '#'
    outline = {c: (w + 1, 18, 1, 1) for c, (w, h, s, o) in small.items()}
    return {'small': small, 'big': big, 'noshadow': noshadow, 'outline': outline}


def build(m, specs):
    ids = {}
    for name, g in specs.items():
        ids[name] = m.make_font(g, default=(name == 'small'), name=name)
    return ids


def compare(native, model):
    def norm(d):
        if d.get('hang'):
            return 'hang'
        return (d['ret'], [f32bits(x) for x in d['size']], d['rows'])
    return norm(native) == norm(model)


def run_all():
    specs = font_specs()
    cases = []
    stats = {'total': 0, 'mismatch': 0}
    seams_seen = set()
    for prec, cw in FPCW.items():
        for screen in ((1024, 768), (2560, 1440), (1440, 900)):
            m = TextMachine(cw)
            m.setup_world(screen)
            ids = build(m, specs)
            mark = m.next
            inv = {v: k for k, v in ids.items()}
            fonts_model = {}
            for name, g in specs.items():
                fonts_model[ids[name]] = {'glyphs': g, 'fixed': max(w for w, h, s, o in g.values()) + 2,
                                          'ell': [0.0, 0.0]}
            model = Model(prec, fonts_model, ids['small'],
                          {'undisplayable': 149, 'fallback': 35, 'standin': 116},
                          [(32, 383, 149, 116), (0x4e00, 0x9fff, 152, 116)], 25.0, screen)
            model.font_by_name = dict(ids)
            for fname in ('small', 'noshadow', 'outline', 0):
                font = ids[fname] if fname else 0
                for text in TEXTS:
                    for px, fixed, trail in ((1, 0, 0), (1, 1, 0), (1, 0, 1), (0, 0, 0), (0, 1, 1)):
                        for wrap in (WRAPS if px else WRAPS_WORLD):
                            for clip, box in (((0, (0.0, 0.0)), (1, (80.0, 40.0)), (1, (1000.0, 17.0))) if px else
                                              ((0, (0.0, 0.0)), (1, (3.0, 1.5)), (1, (40.0, 0.68)))):
                                if clip and (fixed or trail):
                                    continue
                                if screen != (1024, 768) and (px == 0 or clip or fixed):
                                    continue
                                m.next = mark
                                size = m.alloc(8)
                                m.wf(size, box[0])
                                m.wf(size + 4, box[1])
                                rows = m.alloc(4)
                                m.w32(rows, 0xdeadbeef)
                                tp = m.alloc(4)
                                m.w32(tp, m.cstring(text))
                                try:
                                    ret = m.run(0x7352C0, [tp, size, font, px, fixed, trail,
                                                           f32bits(wrap), rows, clip])
                                    rv = m.r32(rows)
                                    nat = {'ret': ret, 'size': [bits_f32(m.r32(size)), bits_f32(m.r32(size + 4))],
                                           'rows': None if rv == 0xdeadbeef else rv}
                                except Hang:
                                    nat = {'hang': True}
                                seams_seen.update(m.seam_log)
                                mod = model.extent(text, (f32(box[0]), f32(box[1])), font, px, fixed, trail,
                                                   f32(wrap), bool(clip))
                                ok = compare(nat, mod)
                                stats['total'] += 1
                                if not ok:
                                    stats['mismatch'] += 1
                                cases.append({'fn': '0x7352C0', 'precision': prec, 'screen': list(screen),
                                              'font': inv.get(font, 'default(0)'), 'text': text,
                                              'px': px, 'fixed': fixed, 'trail': trail,
                                              'wrap_bits': f32bits(wrap), 'clip': clip,
                                              'box_bits': [f32bits(box[0]), f32bits(box[1])],
                                              'native': ({'hang': True} if nat.get('hang') else
                                                         {'ret': nat['ret'],
                                                          'size_bits': [f32bits(x) for x in nat['size']],
                                                          'rows': nat['rows']}),
                                              'model_agrees': ok,
                                              **({} if ok else {'model': mod})})
            # 0x735C00 reference-unit wrapper
            for text in TEXTS:
                for wd in (0.0, 40.0, 100.0, 133.3):
                    for fixed, trail in ((0, 0), (0, 1)):
                        m.next = mark
                        size = m.alloc(8)
                        rows = m.alloc(4)
                        m.w32(rows, 0xdeadbeef)
                        tp = m.alloc(4)
                        m.w32(tp, m.cstring(text))
                        ret = m.run(0x735C00, [tp, size, ids['small'], fixed, trail, f32bits(wd), rows])
                        nat = {'ret': ret, 'size': [bits_f32(m.r32(size)), bits_f32(m.r32(size + 4))],
                               'rows': m.r32(rows)}
                        mod = model.ref_extent(text, ids['small'], fixed, trail, f32(wd))
                        ok = compare(nat, mod)
                        stats['total'] += 1
                        stats['mismatch'] += 0 if ok else 1
                        seams_seen.update(m.seam_log)
                        cases.append({'fn': '0x735C00', 'precision': prec, 'screen': list(screen),
                                      'text': text, 'fixed': fixed, 'trail': trail,
                                      'width_bits': f32bits(wd),
                                      'native': {'ret': ret, 'size_bits': [f32bits(x) for x in nat['size']],
                                                 'rows': nat['rows']},
                                      'model_agrees': ok, **({} if ok else {'model': mod})})
            # 0x735D50 ellipsis
            for fname in ('small', 'noshadow'):
                for text in TEXTS:
                    for w in (0.0, 5.0, 12.0, 20.0, 47.0, 80.0, 150.5, 400.0):
                        m.next = mark
                        tp = m.alloc(4)
                        m.w32(tp, m.cstring(text))
                        outp = m.alloc(4)
                        m.w32(outp, m.cstring(''))
                        ret = m.run(0x735D50, [tp, outp, f32bits(w), f32bits(0.0), ids[fname]]) & 0xff
                        out = m.read_cstring(m.r32(outp))
                        seams_seen.update(m.seam_log)
                        mod = model.ellipsis(text, f32(w), 0.0, ids[fname])
                        ok = (ret, out) == mod
                        stats['total'] += 1
                        stats['mismatch'] += 0 if ok else 1
                        cases.append({'fn': '0x735D50', 'precision': prec, 'screen': list(screen),
                                      'font': fname, 'text': text, 'w_bits': f32bits(w),
                                      'native': {'ret': ret, 'out': out}, 'model_agrees': ok,
                                      **({} if ok else {'model': list(mod)})})
    return cases, stats, sorted(seams_seen)


def main():
    cases, stats, seams = run_all()
    report = {'schema': 1, 'original_sha256': NL.SHA, 'scope': __doc__, 'stats': stats,
              'seams_reached': seams, 'cases': cases}
    OUT.write_text(json.dumps(report, separators=(',', ':'), ensure_ascii=True) + '\n')
    print(json.dumps(stats), 'seams:', seams)
    bad = [c for c in cases if not c['model_agrees']]
    for c in bad[:15]:
        print(json.dumps(c, ensure_ascii=True)[:600])
    print(hashlib.sha256(OUT.read_bytes()).hexdigest())


if __name__ == '__main__':
    main()
