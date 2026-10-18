"""Unicorn oracle for the ORIGINAL AA.exe label DRAW layout
(written by the phase-8 text research subagent in scratch-text2/, promoted
here with paths fixed and a compact export for tests/native_draw.rs; run
from rust-rebuild, ~8.5 min; `--compact` only rewrites native-draw.json
from native-draw.json.gz).

Executes natively 0x735B90 (KKC_RText rebuild, called by the screen-text
update 0x735EE0) on a KKC_RScreenText object (vtable 0xA89B90) and so:
  0x734120 KKC_RText::PreProcessText (row breaking, wrap insertion of
           0xD / 0xE into the processed text, row and glyph records, size,
           overflow flags)
  0x730130 KKC_RText::GetRowStartingXPixels
  0x733B10 KKC_RText::Parse (font/colour/highlight markup, mixed-font
           vertical offsets, use_own_color)
  0x730F00 per-glyph quad builder (pen advance, row step, centering)
  0x72FA00 text batch AddQuad (clamp to screen, clip to the area rect,
           floor to pixels, highlight background quad, overflow tint)
  plus the RScreenText virtuals (0x731CE0.., 0x731BB0/0x731BC0, 0x7301E0,
  0x730260, 0x730280, 0x72F3B0, 0x72F3D0), glyph getters, tokenizer,
  markup 0x7335B0/0x7331E0, ATL CString code, special glyph map lookup
  0x705510/0x492F50.
Seams (in addition to oracle_text.TextMachine's CString manager, _wcsicmp,
_wtof, font-by-name and IAT traps):
  0x7405F0 vertex list append: records (list, 12 floats) and returns;
  0x404EA0 error log: recorded and returned (justification 3 case).
Objects built in emulated memory (no constructors run): the RText fields
the code reads, one text batch per texture (vtable whose +4 is the native
AddQuad 0x72FA00, +0xC/+0x10 = distinct dummy vertex lists), textures with
+0x70 = owning font, fonts in the 0x705020 glyph format with atlas x/y,
the special-glyph hash map (DAT_00B36530+0x3BC), AVars +0xA0/+0xA4 and the
overflow debug flag *(*(DAT_00B39940+0x1E8)).
The Python model (DrawModel) is compared bit-exactly with every output.
"""
from pathlib import Path
import json, struct, sys, math, hashlib, itertools, gzip

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import native_text as OT  # noqa: E402
from native_text import TextMachine, Model, f32, f32bits, bits_f32, msvc_atof, FPCW, Hang  # noqa: E402
from unicorn import UC_HOOK_CODE  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX  # noqa: E402

OUT = HERE / 'native-draw.json.gz'
COMPACT = HERE / 'native-draw.json'   # gzip: ~64 MB of JSON uncompressed
VT_SCREEN_TEXT = 0xa89b90
ADDQUAD = 0x72fa00
APPEND = 0x7405f0


class DrawMachine(TextMachine):
    def __init__(self, fpcw):
        super().__init__(fpcw)
        self.quads = []
        self.errors = []
        self.add_seam(APPEND, self._s_append)
        self.add_seam(0x404ea0, self._s_error)

    def add_seam(self, a, fn):
        self.seams[a] = fn
        self.u.hook_add(UC_HOOK_CODE, (lambda uc, pc, size, user, fn=fn: fn()), begin=a, end=a)

    def _s_append(self):
        lst = self.u.reg_read(UC_X86_REG_ECX)
        p = self.arg(0)
        self.quads.append((lst, [self.r32(p + 4 * k) for k in range(12)]))
        self.ret(0, 4)

    def _s_error(self):
        self.errors.append(self.wstr(self.arg(4)))
        self.ret(0, 0)

    def setup_draw_world(self, screen, debug_tint):
        self.setup_world(screen)
        av = self.r32(0xb39990)
        for off, val in ((0xa4, 30.0),):
            cell = self.alloc(4)
            self.wf(cell, val)
            self.w32(av + off, cell)
        dbg = self.alloc(0x200)
        flag = self.alloc(4)
        self.w32(flag, debug_tint)
        self.w32(dbg + 0x1e8, flag)
        self.w32(0xb39940, dbg)

    def set_special(self, own):
        """special glyph map: {index: use_own_color}"""
        app = self.r32(0xb36530)
        mp = self.alloc(0x60)
        nb = 7
        buckets = self.alloc(4 * nb)
        for c, oc in own.items():
            info = self.alloc(0x30)
            self.u.mem_write(info + 0x18, struct.pack('<HB', c, oc))
            node = self.alloc(0x10)
            self.u.mem_write(node, struct.pack('<H', c))
            self.w32(node + 4, info)
            b = buckets + 4 * (c % nb)
            self.w32(node + 8, self.r32(b))
            self.w32(b, node)
        self.w32(mp + 0x4c, buckets)
        self.w32(mp + 0x50, nb)
        self.w32(app + 0x3bc, mp)

    def make_draw_font(self, glyphs, int8=0, default=False, name=None):
        """glyphs: {c: (w, h, shadow, outline, ax, ay)}"""
        m = self
        f = m.alloc(0x30 + 6 * 0x10000 + 0x40)
        table = bytearray(6 * 0x10000)
        for c in range(0x10000):
            struct.pack_into('<HI', table, 6 * c, 0x3fff, 0)
        maxw = 0
        for c, (w, h, sh, ol, ax, ay) in glyphs.items():
            u16 = ((ol & 1) << 1 | (sh & 1)) << 14
            u32 = ((((h & 0x7f) << 7 | (w & 0x7f)) << 9 | (ay & 0x1ff)) << 9) | (ax & 0x1ff)
            struct.pack_into('<HI', table, 6 * c, u16, u32 & 0xffffffff)
            maxw = max(maxw, w)
        m.u.mem_write(f + 0x30, bytes(table))
        tex = m.alloc(0x80)
        m.w32(tex + 0x70, f)
        arr = m.alloc(0x10)
        m.w32(arr, tex)
        m.w32(f + 0x24, arr)
        m.w32(f + 0x28, 1)
        m.w32(f + 8, int8)
        m.w32(f + 0xc, maxw + 2)
        if default:
            m.w32(self.fs + 0x10, f)
        if name:
            self.fonts_by_name[name] = f
        return f, tex

    def make_rtext(self, text, font, colour, hl, origin, area, justify, multiline, fixed, textures):
        m = self
        rt = m.alloc(0xd0)
        m.w32(rt, VT_SCREEN_TEXT)
        m.w32(rt + 8, m.cstring(text))
        m.w32(rt + 0xc, m.cstring(''))
        # batches (one per texture, pre-created so Parse never allocates)
        bvt = m.alloc(0x20)
        m.w32(bvt + 4, ADDQUAD)
        barr = m.alloc(4 * len(textures))
        self.lists = {}
        for i, tex in enumerate(textures):
            b = m.alloc(0x20)
            m.w32(b, bvt)
            m.w32(b + 4, rt)
            m.w32(b + 8, tex)
            la, lb = m.alloc(0x10), m.alloc(0x10)
            m.w32(b + 0xc, la)
            m.w32(b + 0x10, lb)
            self.lists[la] = ('main', i)
            self.lists[lb] = ('highlight', i)
            m.w32(barr + 4 * i, b)
        m.w32(rt + 0x10, barr)
        m.w32(rt + 0x14, len(textures))
        m.w32(rt + 0x18, len(textures))
        rows = m.alloc(0x14 * 512)
        m.w32(rt + 0x1c, rows)
        m.w32(rt + 0x20, 0)
        m.w32(rt + 0x24, 512)
        gl = m.alloc(0x20 * 2048)
        m.w32(rt + 0x28, gl)
        m.w32(rt + 0x2c, 0)
        m.w32(rt + 0x30, 2048)
        m.w32(rt + 0x44, font)
        for k in range(4):
            m.wf(rt + 0x48 + 4 * k, colour[k])
            m.wf(rt + 0x58 + 4 * k, hl[k])
        m.wf(rt + 0x74, origin[0])
        m.wf(rt + 0x78, origin[1])
        m.w32(rt + 0x8c, justify)
        m.u.mem_write(rt + 0x98, bytes([multiline, fixed, 0, 0, 0, 0, 1]))
        for k in range(4):
            m.wf(rt + 0xa8 + 4 * k, area[k])
        return rt

    def read_rtext(self, rt):
        m = self
        rows = []
        base = m.r32(rt + 0x1c)
        for i in range(m.r32(rt + 0x20)):
            p = base + 0x14 * i
            rows.append([m.r32(p), m.r32(p + 4), m.r32(p + 8), m.r32(p + 12), m.r32(p + 16)])
        gl = []
        base = m.r32(rt + 0x28)
        for i in range(m.r32(rt + 0x2c)):
            p = base + 0x20 * i
            gl.append([m.r32(p) & 0xffff] + [m.r32(p + 4 * k) for k in range(1, 8)])
        return {'processed': m.read_cstring(m.r32(rt + 0xc)), 'rows': rows, 'glyphs': gl,
                'size_bits': [m.r32(rt + 0x84), m.r32(rt + 0x88)],
                'flags': [m.u.mem_read(rt + 0x9c, 1)[0], m.u.mem_read(rt + 0x9d, 1)[0]],
                'pen_bits': [m.r32(rt + 0x68), m.r32(rt + 0x6c)],
                'quads': [[self.lists[l][0], self.lists[l][1]] + v for l, v in self.quads],
                'errors': list(self.errors)}


# ---------------------------------------------------------------- model
REMAP_FROM, REMAP_TO = '\x11\x12\x13\x14\x15', '<>"\\&'


class DrawModel:
    """0x735B90 -> 0x734120 / 0x730130 / 0x733B10 / 0x730F00 / 0x72FA00."""

    def __init__(self, pc, fonts, default_font, own, screen, world_scale=25.0, baseline_scale=30.0,
                 debug_tint=0):
        self.pc = pc
        self.fonts = fonts          # ptr -> {'g': {c: (w,h,sh,ol,ax,ay)}, 'fixed', 'int8', 'tex'}
        self.default = default_font
        self.own = own              # special glyph index -> use_own_color
        self.sw, self.sh = screen
        self.ws = f32(world_scale)
        self.bs = f32(baseline_scale)
        self.debug_tint = debug_tint
        self.sets = [(32, 383, 149, 116), (0x4e00, 0x9fff, 152, 116)]
        self.by_name = {}

    def r(self, x):
        return f32(x) if self.pc == 'pc24' else x

    # glyph table --------------------------------------------------------
    def entry(self, font, c):
        return self.fonts[font]['g'].get(c)

    def adv(self, font, c):                 # 0x7051A0 (absent entry: u32 0 -> 0)
        e = self.entry(font, c)
        if not e:
            return 0
        return e[0] - 1 if e[2] else e[0]

    def lineh(self, font):                  # 0x705580
        e = self.entry(font, 65)
        return e[1] if e else 0

    def cellh(self, font, c):               # 0x705240
        e = self.entry(font, c)
        return e[1] if e else 0

    def atlas(self, font, c):               # 0x705160 / 0x705180
        e = self.entry(font, c)
        return (e[4], e[5]) if e else (0, 0)

    def has(self, font, c):
        return c in self.fonts[font]['g']

    def set_of(self, c):
        for lo, hi, und, sp in self.sets:
            if lo <= c <= hi:
                return und, sp
        return None

    def glyph_for(self, font, c):
        g = c
        if c in (10, 13, 14):
            s = self.set_of(c)
            g = s[1] if s else 116
        if not self.has(font, g):
            s = self.set_of(g)
            g = s[0] if s else 149
            if not self.has(font, g):
                g = 35
        return g

    # markup 0x7335B0 / 0x7331E0 ----------------------------------------
    def markup(self, tok, st):
        r = 7
        pos = 0
        while True:
            sub, pos = Model.next_token(tok, pos, ';', True)
            if not sub:
                return r
            k, p2 = Model.next_token(sub, 0, '=', True)
            v, _ = Model.next_token(sub, p2, '=', True)
            k, v = k.strip(), v.strip()
            kl = k.lower()
            if kl == 'h_on':
                st['flag'] = 1
                r = 0
            elif kl == 'h_off':
                st['flag'] = 0
                r = 1
            elif kl == 'font':
                st['font'] = self.by_name.get(v, 0)
                r = 2
            elif kl in ('rc', 'restorecolor'):
                st['colour'] = list(st['saved'])
                r = 4
            elif kl == 'color':
                comps, p = [], 0
                for n in range(4):
                    t, p = Model.next_token(v, p, ',', True)
                    t = t.strip()
                    comps.append((1.0 if n == 3 else 0.0) if not t else f32(msvc_atof(t)))
                st['saved'] = list(st['colour'])
                a = f32(self.r(comps[3] * st['colour'][3]))
                st['colour'] = [comps[0], comps[1], comps[2], a]
                r = 3
            elif kl == 'setx':
                x = msvc_atof(v)
                if x < st['pen_x']:
                    sp = float(self.adv(st['font'], 32)) if st['font'] else 4.0
                    x = self.r(sp + st['pen_x'])
                st['pen_x'] = f32(x)
                r = 5
            elif kl == 'sety':
                st['pen_y'] = f32(msvc_atof(v))
                r = 6
            else:
                r = 7

    # 0x734120 ------------------------------------------------------------
    def preprocess(self, text, rt):
        R = self.r
        full = list(text)
        font = rt['font']
        pen_x, pen_y = rt['ox'], rt['oy']
        size = [0.0, 0.0]
        rows, gl = [], []
        row_w = row_h = trailing = w_sp = 0.0
        idx = row_first = 0
        last_space = -1
        retry = False
        vis = 0
        xstart = 0.0
        has_xs = wrapped = False
        L, T, Rr, B = rt['area']
        W = R(Rr - L)
        colour = list(rt['colour'])
        st = {'font': font, 'colour': colour, 'saved': list(colour), 'flag': 0}
        pos = 0
        in_markup = text[:1] == '<'
        while True:
            tok_start = pos
            tok, pos = Model.next_token(text, pos, '<>', False)
            if not tok:
                break
            if in_markup:
                st.update(font=font, pen_x=pen_x, pen_y=pen_y)
                r = self.markup(tok, st)
                font, pen_x, pen_y = st['font'], st['pen_x'], st['pen_y']
                if r == 5:
                    row_w = pen_x
                    xstart = pen_x
                in_markup = text[pos:pos + 1] == '<'
                continue
            flag = pos < len(text)
            tok = list(tok)
            i = 0
            while i < len(tok):
                c = ord(tok[i])
                k = REMAP_FROM.find(chr(c))
                if k >= 0:
                    c = ord(REMAP_TO[k])
                if font == 0:
                    font = self.default
                g = self.glyph_for(font, c)
                adv = float(self.fonts[font]['fixed']) if rt['fixed'] else float(self.adv(font, g))
                h = float(self.lineh(font))
                if c in (10, 13, 14):
                    if w_sp > size[0]:
                        size[0] = w_sp
                    size[1] = f32(R(row_h + size[1]))
                    if c == 10:
                        gl.append([c, len(rows), idx, i + vis, pen_x, pen_y, f32(R(pen_x + 1.0)), f32(R(pen_y + h))])
                    else:
                        gl.append([c, len(rows), idx, -1, w_sp, pen_y, w_sp, f32(R(pen_y + h))])
                    rows.append([row_w, row_h, row_first, len(gl) - 1, xstart if has_xs else 0.0])
                    row_first = len(gl)
                    row_h = h
                    idx = 0
                    trailing = 0.0
                    w_sp = 0.0
                    if wrapped:
                        row_w = xstart
                        has_xs = True
                        pen_x = f32(R(xstart + rt['ox']))
                    else:
                        row_w = 0.0
                        pen_x = rt['ox']
                        has_xs = False
                    pen_y = f32(R(pen_y + h))
                    if not wrapped:
                        xstart = 0.0
                    wrapped = False
                    i += 1
                    continue
                if c == 32:
                    gl.append([c, len(rows), idx, i + vis, pen_x, pen_y, f32(R(adv + pen_x)), f32(R(pen_y + h))])
                    pen_x = f32(R(adv + pen_x))
                    idx += 1
                    last_space = i + tok_start
                    retry = False
                    w_sp = row_w
                    trailing = f32(R(adv + trailing))
                    i += 1
                    continue
                if rt['multiline'] and W < f32(R(adv + pen_x)):
                    if last_space == -1:
                        if retry:
                            break                       # 0x7347AC: rest of the token dropped
                        tok.insert(i, '\r')
                        full.insert(i + tok_start, '\r')
                        retry = True
                        w_sp = row_w
                        idx = 0
                        continue                        # i-- then i++
                    s = last_space - tok_start
                    retry = False
                    if s < 0:
                        n = i + 1
                        for j in range(n):
                            row_w = f32(R(row_w - self.adv(font, gl[-1 - j][0])))
                        del gl[len(gl) - n:]
                        tok.insert(0, '\r')
                        tok_start -= 1
                        i = 0
                    else:
                        n = i - s
                        for j in range(n):
                            row_w = f32(R(row_w - self.adv(font, gl[-1 - j][0])))
                        if n:
                            del gl[len(gl) - n:]
                        tok[s] = '\x0e'
                        i = s
                    full[last_space] = '\x0e'
                    wrapped = True
                    last_space = -1
                    continue
                gl.append([c, len(rows), idx, i + vis, pen_x, pen_y, f32(R(adv + pen_x)), f32(R(pen_y + h))])
                pen_x = f32(R(adv + pen_x))
                idx += 1
                retry = False
                row_w = f32(R(R(adv + trailing) + row_w))
                trailing = 0.0
                if h > row_h:
                    row_h = h
                i += 1
            vis += len(tok)
            in_markup = flag
        gl.append([0, len(rows), idx, vis, pen_x, pen_y, f32(R(pen_x + 1.0)), f32(R(self.lineh(font) + pen_y))])
        if row_w > 0.0:
            if row_w > size[0]:
                size[0] = row_w
            rows.append([row_w, row_h, row_first, len(gl) - 1, xstart if has_xs else 0.0])
        size[1] = f32(R(row_h + size[1]))
        H = R(B - T)
        f9c = 1 if H < size[1] else 0
        f9d = 1 if (W < size[0] or H < size[1]) else 0
        return ''.join(full), rows, gl, size, [f9c, f9d]

    # 0x730130 ------------------------------------------------------------
    def row_start(self, rt, rows, r):
        R = self.r
        if r >= len(rows):
            return 0.0, None
        L, T, Rr, B = rt['area']
        W = f32(R(Rr - L))
        s = rows[r][4]
        if s > 0.0:
            return s, None
        j = rt['justify']
        if j == 0:
            return rt['ox'], None
        if j == 1:
            return float(math.floor(R(R(R(W - rows[r][0]) * 0.5) + 0.5))), None
        if j == 2:
            return R(W - rows[r][0]), None
        return 0.0, 'Unsupported justification mode.'

    # 0x72FA00 ------------------------------------------------------------
    def addquad(self, rt, x, y, w, h, u, v, col, hlf, hl, ov, tex_i, quads):
        R = self.r
        L, T, Rr, B = rt['area']

        def clamp_floor(val, lim):
            if val < 0.0:
                val = 0.0
            elif val > lim:
                val = lim
            return float(math.floor(val))
        sh1 = f32(R(self.sh - 1.0))
        sw1 = f32(R(self.sw - 1.0))
        yq = f32(clamp_floor(y, sh1))
        tq = clamp_floor(T, sh1)
        hq = h
        if yq < tq:
            d = R(tq - yq)
            if d > h:
                return
            yq = f32(R(yq + d))
            v = f32(R(v + d))
            hq = f32(R(h - d))
        bq = clamp_floor(B, sh1)
        if R(hq + yq) > bq:
            d = R(bq - yq)
            if d <= 0.0:
                return
            hq = f32(d)
        xq = f32(clamp_floor(x, sw1))
        lq = clamp_floor(L, sw1)
        wq = w
        if xq < lq:
            d = R(lq - xq)
            if d > w:
                return
            xq = f32(R(xq + d))
            u = f32(R(u + d))
            wq = f32(R(w - d))
        rq = clamp_floor(Rr, sw1)
        if R(wq + xq) > rq:
            d = R(rq - xq)
            if d <= 0.0:
                return
            wq = f32(d)
        col = list(col)
        geo = [xq, yq, f32(R(wq + xq)), f32(R(hq + yq)), u, v, f32(R(wq + u)), f32(R(hq + v))]
        if hlf:
            if col == list(hl):
                col = [f32(R(1.0 - col[0])), f32(R(1.0 - col[1])), f32(R(1.0 - col[2])), col[3]]
            quads.append(['highlight', tex_i] + [f32bits(z) for z in geo + list(hl)])
        if ov and self.debug_tint:
            col = [1.0, 0.0, 0.0, 1.0]
        quads.append(['main', tex_i] + [f32bits(z) for z in geo + col])

    # 0x733B10 + 0x730F00 -------------------------------------------------
    def parse(self, full, rt, rows, flags, pen_x, pen_y):
        R = self.r
        L, T, Rr, B = rt['area']
        quads, errors = [], []
        font = rt['font']
        st = {'font': font, 'colour': list(rt['colour']), 'saved': [0.0] * 4, 'flag': 0,
              'pen_x': pen_x, 'pen_y': pen_y}
        hmax = yacc = ysub = 0.0     # Parse locals 0x14, 0x3C, 0x30
        base = 0.0                   # Parse local 0x38: never reset
        row_idx = 1
        pos = 0
        in_markup = full[:1] == '<'
        while True:
            tok, pos = Model.next_token(full, pos, '<>', False)
            if not tok:
                break
            if in_markup:
                st['font'] = font
                self.markup(tok, st)
                font = st['font']
                in_markup = full[pos:pos + 1] == '<'
                continue
            in_markup = pos < len(full)
            for ch in tok:
                c = ord(ch)
                k = REMAP_FROM.find(chr(c))
                if k >= 0:
                    c = ord(REMAP_TO[k])
                if font == 0:
                    font = self.default
                g = self.glyph_for(font, c)
                tex_i = self.fonts[font]['tex']
                H = float(self.lineh(font))
                if H > hmax:
                    base = f32(R(R(self.fonts[font]['int8'] / self.bs) + base))
                    hmax = H
                elif R(self.lineh(font) / self.ws) < hmax:
                    ysub = base
                    yacc = f32(R(hmax - H))
                own = self.own.get(g, 0)
                cv = [1.0, 1.0, 1.0, 1.0] if own else list(st['colour'])
                yoff = f32(R(yacc - ysub))
                # 0x730F00 (font = texture owner = current font here)
                adv = float(self.adv(font, g))
                fixoff = f32(R(R(self.fonts[font]['fixed'] - adv) * 0.5)) if rt['fixed'] else 0.0
                hc = float(self.lineh(font))
                ch_ = float(self.cellh(font, g))
                vc = f32(R(R(hc - ch_) * 0.5))
                if c in (10, 13, 14):
                    s, err = self.row_start(rt, rows, row_idx)
                    if err:
                        errors.append(err)
                    st['pen_x'] = f32(s)
                    st['pen_y'] = f32(R(hmax + st['pen_y']))
                    row_idx += 1
                    hmax = yacc = ysub = 0.0
                    continue
                y = f32(R(R(T + st['pen_y']) + yoff))
                if B > y:
                    ax, ay = self.atlas(font, g)
                    if rt['fixed']:
                        yq = f32(R(R(T + st['pen_y']) + yoff))
                        x = f32(R(R(L + st['pen_x']) + fixoff))
                    else:
                        yq = f32(R(R(R(T + st['pen_y']) + vc) + yoff))
                        x = f32(R(L + st['pen_x']))
                    self.addquad(rt, x, yq, adv, ch_, float(ax), float(ay), cv, st['flag'],
                                 rt['hl'], flags[1], tex_i, quads)
                if rt['fixed']:
                    st['pen_x'] = f32(R(float(self.fonts[font]['fixed']) + st['pen_x']))
                else:
                    st['pen_x'] = f32(R(adv + st['pen_x']))
        return quads, errors, st['pen_x'], st['pen_y']

    # 0x735B90 ------------------------------------------------------------
    def rebuild(self, text, rt):
        full, rows, gl, size, flags = self.preprocess(text, rt)
        pen = [rt['ox'], rt['oy']]
        quads, errors = [], []
        if len(text):
            s, err = self.row_start(rt, rows, 0)
            if err:
                errors.append(err)
            quads, e2, px, py = self.parse(full, rt, rows, flags, f32(s), rt['oy'])
            errors += e2
            pen = [px, py]
        return {'processed': full,
                'rows': [[f32bits(r[0]), f32bits(r[1]), r[2], r[3] & 0xffffffff, f32bits(r[4])] for r in rows],
                'glyphs': [[g[0], g[1], g[2], g[3] & 0xffffffff] + [f32bits(z) for z in g[4:]] for g in gl],
                'size_bits': [f32bits(size[0]), f32bits(size[1])], 'flags': flags,
                'pen_bits': [f32bits(pen[0]), f32bits(pen[1])], 'quads': quads, 'errors': errors}


# ---------------------------------------------------------------- cases
def table(h=17, shadow=1, seed=0):
    g = {}
    for c in range(32, 384):
        w = 3 + (c * 7 + 3 + seed) % 11
        if c == 32:
            w = 5
        g[c] = (w, h, shadow, 0, (c * 13) % 480, (c * 7) % 480)
    g[ord('W')] = (17, h, shadow, 0, 400, 3)
    for c, (w, hh) in ((128, (17, 21)), (129, (15, 15)), (133, (9, 30))):   # special images
        g[c] = (w, hh, shadow, 0, 300 + c, 100 + c)
    return g


TEXTS = [
    'Hello world',
    'The quick brown fox jumps over the lazy dog',
    'Line one\nLine two\n\nLine four',
    'Supercalifragilisticexpialidocious',
    'Two  spaces  between  words here and there',
    'Trailing   ',
    '<Color=1,0,0,0.5>Red half<RC> normal <Color=0,1,0>green',
    '<Font=big>Big <Font=small>small <Font=big>BIG again and more words',
    'small <Font=big>BIG<Font=small> small after',
    '<Font=base7>Base seven offset text<Font=small> back',
    'Money \x80 ammo \x81 xp \x85 end',
    '<h_on>high<h_off> lighted <h_on>again',
    'Before<SetX=120>After set x and wrap words',
    '<SetX=30>indented words that will wrap around',
    'ab<SetY=40>cd\nef',
    'Remap \x11tag\x12 \x13q\x13',
    'Carriage\r\nreturn',
    'Missing 一 han',
    'Wrap<Color=0,0,1>inside a markup token boundary here',
    'x' * 40,
    '<Color=1,1,1,1>' + 'W' * 12,
    '',
    'aaaa bbbb<Color=1,1,1>cccccccccc dd',
    'one two <RC>threefourfivesix seven',
]
AREAS = [
    (100.0, 50.0, 300.0, 120.0),
    (100.0, 50.0, 161.0, 400.0),
    (-20.0, -10.0, 150.0, 60.0),
    (900.0, 740.0, 1100.0, 800.0),
    (10.5, 20.25, 140.75, 200.5),
    (100.0, 50.0, 110.0, 400.0),
    (10.3, 20.7, 140.9, 200.1),
]
ORIGINS = [(0.0, 0.0), (-12.0, -5.0), (-12.37, -5.11)]


def run_all():
    cases, stats = [], {'total': 0, 'mismatch': 0}
    seams_seen = set()
    for prec, cw in FPCW.items():
        for screen, debug in (((1024, 768), 0), ((2560, 1440), 1)):
            m = DrawMachine(cw)
            m.setup_draw_world(screen, debug)
            own = {128: 1, 129: 0, 133: 1}
            m.set_special(own)
            specs = {'small': (table(17, 1), 0), 'big': (table(23, 1, 4), 0), 'base7': (table(19, 1, 2), 7),
                     'noshadow': (table(15, 0, 1), 0)}
            ptr, tex, fonts = {}, {}, {}
            for i, (name, (g, int8)) in enumerate(specs.items()):
                ptr[name], tex[name] = m.make_draw_font(g, int8, default=(name == 'small'), name=name)
                fonts[ptr[name]] = {'g': g, 'fixed': max(v[0] for v in g.values()) + 2, 'int8': int8, 'tex': i}
            textures = [tex[n] for n in specs]
            mark = m.next
            for fname, text, area, origin, justify, multi, fixed in itertools.product(
                    ('small', 'noshadow'), TEXTS, AREAS, ORIGINS, (0, 1, 2), (0, 1), (0, 1)):
                if fixed and (justify != 0 or origin != (0.0, 0.0) or area not in AREAS[:2]):
                    continue
                if fname == 'noshadow' and (area not in AREAS[:2] or origin != (0.0, 0.0)):
                    continue
                for colour, hl in (((1.0, 1.0, 1.0, 1.0), (0.2, 0.3, 0.4, 0.5)),
                                   ((0.25, 0.5, 0.75, 1.0), (0.25, 0.5, 0.75, 1.0))):
                    if colour[0] != 1.0 and '<h_on>' not in text:
                        continue
                    model = DrawModel(prec, fonts, ptr['small'], own, screen, debug_tint=debug)
                    model.by_name = dict(ptr)
                    m.next = mark
                    m.quads, m.errors = [], []
                    rt = m.make_rtext(text, ptr[fname], colour, hl, origin, area, justify, multi, fixed, textures)
                    m.seam_log = []
                    try:
                        m.call(0x735b90, rt, [], 0)
                        nat = m.read_rtext(rt)
                    except Hang:
                        nat = {'hang': True}
                    seams_seen.update(m.seam_log)
                    rtm = {'font': ptr[fname], 'colour': [f32(z) for z in colour], 'hl': [f32(z) for z in hl],
                           'ox': f32(origin[0]), 'oy': f32(origin[1]), 'area': [f32(z) for z in area],
                           'justify': justify, 'multiline': multi, 'fixed': fixed}
                    mod = model.rebuild(text, rtm)
                    ok = nat == mod
                    stats['total'] += 1
                    stats['mismatch'] += 0 if ok else 1
                    cases.append({'precision': prec, 'screen': list(screen), 'debug_tint': debug, 'font': fname,
                                  'text': text, 'area_bits': [f32bits(z) for z in area],
                                  'origin_bits': [f32bits(z) for z in origin], 'justify': justify,
                                  'multiline': multi, 'fixed': fixed,
                                  'colour_bits': [f32bits(z) for z in colour], 'hl_bits': [f32bits(z) for z in hl],
                                  'native': nat, 'model_agrees': ok, **({} if ok else {'model': mod})})
            # justification 3 (error path), once per machine
            m.next = mark
            m.quads, m.errors = [], []
            rt = m.make_rtext('Bad justify', ptr['small'], (1, 1, 1, 1), (0, 0, 0, 0), (0.0, 0.0), AREAS[0], 3, 1, 0,
                              textures)
            m.call(0x735b90, rt, [], 0)
            nat = m.read_rtext(rt)
            model = DrawModel(prec, fonts, ptr['small'], own, screen, debug_tint=debug)
            mod = model.rebuild('Bad justify', {'font': ptr['small'], 'colour': [1.0] * 4, 'hl': [0.0] * 4,
                                                'ox': 0.0, 'oy': 0.0, 'area': [f32(z) for z in AREAS[0]],
                                                'justify': 3, 'multiline': 1, 'fixed': 0})
            ok = nat == mod
            stats['total'] += 1
            stats['mismatch'] += 0 if ok else 1
            cases.append({'precision': prec, 'screen': list(screen), 'text': 'Bad justify', 'justify': 3,
                          'native': nat, 'model_agrees': ok, **({} if ok else {'model': mod})})
    return cases, stats, sorted(seams_seen)


def main():
    cases, stats, seams = run_all()
    report = {'schema': 1, 'original_sha256': OT.NL.SHA, 'scope': __doc__, 'stats': stats,
              'seams_reached': seams, 'cases': cases}
    with gzip.open(OUT, 'wt', encoding='ascii') as f:
        f.write(json.dumps(report, separators=(',', ':'), ensure_ascii=True) + '\n')
    print(json.dumps(stats), 'seams:', seams)
    bad = [c for c in cases if not c['model_agrees']]
    for c in bad[:6]:
        print(json.dumps(c, ensure_ascii=True)[:1500])
    print(hashlib.sha256(OUT.read_bytes()).hexdigest())


def fnv(values):
    """FNV-1a 64 over u32 values (little-endian bytes)."""
    h = 0xcbf29ce484222325
    for v in values:
        for b in struct.pack('<I', v & 0xffffffff):
            h ^= b
            h = (h * 0x100000001b3) & 0xffffffffffffffff
    return h


def compact():
    """native-draw.json: every case with its inputs, the processed text,
    row records, size, flags, pen and errors in full, glyph records and
    quads as counts + FNV-1a 64 digests (glyph: c, row, idx, text_idx,
    4 rect bits; quad: list 0 main / 1 highlight, texture, 12 bits)."""
    with gzip.open(OUT, 'rt', encoding='ascii') as f:
        d = json.load(f)
    out = []
    for c in d['cases']:
        n = c['native']
        e = {k: c[k] for k in ('precision', 'screen', 'text', 'justify')}
        if 'font' not in c:          # justification-3 error case
            e.update(font='small', area_bits=[f32bits(z) for z in AREAS[0]], origin_bits=[0, 0],
                     multiline=1, fixed=0, colour_bits=[f32bits(1.0)] * 4, hl_bits=[0] * 4,
                     debug_tint=1 if c['screen'][0] == 2560 else 0)
        else:
            for k in ('font', 'area_bits', 'origin_bits', 'multiline', 'fixed', 'colour_bits', 'hl_bits', 'debug_tint'):
                e[k] = c[k]
        if n.get('hang'):
            e['hang'] = True
        else:
            g = [v for r in n['glyphs'] for v in r]
            q = [v for r in n['quads'] for v in ([0 if r[0] == 'main' else 1, r[1]] + r[2:])]
            e.update(processed=n['processed'], rows=n['rows'], size_bits=n['size_bits'], flags=n['flags'],
                     pen_bits=n['pen_bits'], errors=n['errors'], n_glyphs=len(n['glyphs']), glyph_fnv=fnv(g),
                     n_quads=len(n['quads']), quad_fnv=fnv(q))
        out.append(e)
    report = {'schema': 1, 'original_sha256': d['original_sha256'], 'stats': d['stats'],
              'seams_reached': d['seams_reached'], 'source': OUT.name, 'cases': out}
    COMPACT.write_text(json.dumps(report, separators=(',', ':'), ensure_ascii=True) + chr(10), encoding='ascii')
    print(len(out), 'cases ->', COMPACT, COMPACT.stat().st_size)


if __name__ == '__main__':
    if '--compact' not in sys.argv:
        main()
    compact()
