"""Unicorn oracle for the original UI view-element bounds code in AA.exe.

Executes the ORIGINAL x86 code of:
  0x5E3150  bounds string parse (L/T/R/B units -> floats + unit flags).
            Its CRT __wtof call (0x96D3E8) is a SEAM: the MSVC CRT converts
            via the thread locale (TLS/kernel32), which this harness does not
            host. The seam implements MSVC atof prefix rules (skip leading
            iswspace, [sign] digits [. digits] [e|E|d|D [sign] digits],
            stop at the first other character, empty -> 0.0) and returns the
            correctly rounded double in ST0 through an injected fld/ret stub;
  0x5E3370  bounds resolve against a parent rect with x/y scale, which calls
            0x5E2460 (degenerate clamp / inside test) and 0x5E1580 (clip);
  0x5E34B0  whole view-element layout (outer rect relative to the parent VE's
            outer rect, clip accumulation, client rect relative to own outer
            rect, empty flag), including 0x5C1DB0/0x5C1DD0 (scale = screen
            size / InterfacePositioningWidth|Height AVars);
  0x5E25C0  widget-state -> VE visibility mask (when-mask test) including
            0x5EE2C0 (widget shown flag + render threshold); only the
            vtable+0x60 visibility setter is a recording seam;
  0x5E4D00  the 'when' keyword table construction is NOT executed (hash
            container seams); the keyword values are read from its
            instruction stream instead (see when_table()).
Each case runs under both x87 precision-control settings, PC24 (FPCW 0x007F,
what Direct3D9 sets without D3DCREATE_FPU_PRESERVE) and PC53 (FPCW 0x027F,
the CRT startup default). Results are stored as raw IEEE bit patterns.

Writes native-layout.json next to this script.
"""
from pathlib import Path
import hashlib, json, re, struct, sys, itertools

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT.parent / 'reconstruction/tools/python-packages'))
sys.path.insert(0, str(HERE))
from aadis import P  # noqa: E402
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32, UC_HOOK_CODE, UC_HOOK_MEM_READ, UC_HOOK_MEM_WRITE  # noqa: E402
from unicorn.x86_const import *  # noqa: E402,F401,F403

SHA = hashlib.sha256(P.b).hexdigest()
assert SHA == '5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26', SHA

HEAP = 0x3000000
STACK_TOP = 0x3ff0000
STOP = 0x3ff8000
FPCW = {'pc24': 0x007F, 'pc53': 0x027F}


WHITESPACE = ''.join(chr(c) for c in (0x20, 0x09, 0x0A, 0x0B, 0x0C, 0x0D))


def msvc_atof(text):
    """MSVC atof prefix grammar (seam model, see module doc)."""
    i, n = 0, len(text)
    while i < n and text[i] in WHITESPACE:
        i += 1
    m = re.match(r'[+-]?(\d+\.?\d*|\.\d+)([eEdD][+-]?\d+)?', text[i:])
    if not m:
        return 0.0
    num = m.group(0).replace('d', 'e').replace('D', 'e')
    return float(num)


def f32bits(x):
    return struct.unpack('<I', struct.pack('<f', x))[0]


def bits_f32(b):
    return struct.unpack('<f', struct.pack('<I', b))[0]


class Machine:
    def __init__(self, fpcw):
        u = self.u = Uc(UC_ARCH_X86, UC_MODE_32)
        # FS base is 0 in this emulator, so the MSVC SEH link fs:[0] is linear
        # 0..3. Page 0 is mapped for that word only: any other access there
        # (a null-pointer dereference) stops the run and fails the case
        # (docs/NATIVE-PORT-RULES.md, emulator caveats).
        u.mem_map(0, 0x1000)
        self.null_accesses = []
        u.hook_add(UC_HOOK_MEM_READ | UC_HOOK_MEM_WRITE, self._null, begin=0, end=0xfff)
        u.mem_map(P.base, (P.image_size + 0xfff) & ~0xfff)
        for vs, rva, size, off in P.sections:
            u.mem_write(P.base + rva, P.b[off:off + size])
        u.mem_map(HEAP, 0x1000000)
        self.next = HEAP + 0x1000
        self.fpcw = fpcw
        # injected stub: fld qword ptr [STUB+0x10]; ret
        self.stub = 0x2e00000
        u.mem_map(self.stub, 0x1000)
        u.mem_write(self.stub, bytes([0xDD, 0x05]) + struct.pack('<I', self.stub + 0x10) + bytes([0xC3]))
        self.seams = {}
        self.visited = set()
        self.calls = []
        u.hook_add(UC_HOOK_CODE, self._hook)
        self.seams[0x96D3E8] = self.wtof_seam
        # allocator entry points: the code under test must not reach the
        # real heap (operator new / delete, CString data allocator)
        self.heap_calls = []
        for a in (0x6F7810, 0x6F7430, 0x856370):
            self.seams[a] = (lambda a=a: self._heap(a))

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

    def _heap(self, addr):
        self.heap_calls.append((addr, self.r32(self.u.reg_read(UC_X86_REG_ESP))))
        self.u.emu_stop()

    def alloc(self, n):
        p = self.next
        self.next += (n + 15) & ~15
        self.u.mem_write(p, b'\0' * n)
        return p

    def w32(self, a, v):
        self.u.mem_write(a, struct.pack('<I', v & 0xffffffff))

    def r32(self, a):
        return struct.unpack('<I', self.u.mem_read(a, 4))[0]

    def wf(self, a, x):
        self.u.mem_write(a, struct.pack('<f', x))

    def cstring(self, text):
        """MFC-style CString: data pointer with header at -16
        (allocator, length, capacity, refcount)."""
        raw = text.encode('utf-16-le') + b'\0\0'
        h = self.alloc(16 + len(raw) + 2)
        self.w32(h, 0)
        self.w32(h + 4, len(text))
        self.w32(h + 8, len(text))
        self.w32(h + 12, 1)
        self.u.mem_write(h + 16, raw)
        return h + 16

    def _hook(self, u, pc, size, user):
        if P.base <= pc < P.base + P.image_size:
            self.visited.add(pc)
        fn = self.seams.get(pc)
        if fn:
            fn()

    def wtof_seam(self):
        p = self.arg(0)
        chars = []
        while True:
            c = struct.unpack('<H', self.u.mem_read(p + 2 * len(chars), 2))[0]
            if c == 0:
                break
            chars.append(chr(c))
        value = msvc_atof(''.join(chars))
        self.u.mem_write(self.stub + 0x10, struct.pack('<d', value))
        self.u.reg_write(UC_X86_REG_EIP, self.stub)

    def ret(self, value=0, pop=0):
        sp = self.u.reg_read(UC_X86_REG_ESP)
        self.u.reg_write(UC_X86_REG_EAX, value)
        self.u.reg_write(UC_X86_REG_EIP, self.r32(sp))
        self.u.reg_write(UC_X86_REG_ESP, sp + 4 + pop)

    def arg(self, n):
        return self.r32(self.u.reg_read(UC_X86_REG_ESP) + 4 + 4 * n)

    def call(self, pc, ecx, args, pop):
        sp = STACK_TOP - 0x100
        self.w32(sp, STOP)
        for i, a in enumerate(args):
            self.w32(sp + 4 + 4 * i, a)
        self.u.reg_write(UC_X86_REG_ESP, sp)
        self.u.reg_write(UC_X86_REG_EBP, STACK_TOP - 0x10)
        self.u.reg_write(UC_X86_REG_ECX, ecx)
        self.u.reg_write(UC_X86_REG_FPCW, self.fpcw)
        self.u.emu_start(pc, STOP, count=2_000_000)
        assert not self.null_accesses, f'null-pointer access {self.null_accesses}'
        assert not self.heap_calls, f'allocator reached {self.heap_calls}'
        assert self.u.reg_read(UC_X86_REG_EIP) == STOP
        assert self.u.reg_read(UC_X86_REG_ESP) == sp + 4 + pop, hex(self.u.reg_read(UC_X86_REG_ESP) - sp)
        return self.u.reg_read(UC_X86_REG_EAX)


# ------------------------------------------------------------------ parse
PARSE_VALUES = ['0', '26', '783', '12.5', '-3', '50%', '100%', '+50%', '25.5%',
                '0r', '10r', '13R', '0b', '8b', '6B', '+237', '+758', '+0',
                '+10r', '+5b', '10t', ' 7', '1e1', '', '%', 'r', '+', 'abc',
                '3.75b', '+12.25']


def parse_case(m, vals, prior):
    b = m.alloc(0x40)
    for i, x in enumerate(prior):
        m.wf(b + 4 * i, x)
    m.w32(b + 0x30, 0xdeadbeef)
    ptrs = []
    for v in vals:
        cell = m.alloc(4)
        m.w32(cell, m.cstring(v))
        ptrs.append(cell)
    ok = m.call(0x5E3150, b, ptrs, 0x10) & 0xff
    return {'ok': ok, 'ltrb_bits': [m.r32(b + 4 * i) for i in range(4)],
            'flags': m.r32(b + 0x30)}


def parse_cases():
    rows = []
    # every value in every slot with neutral partners, plus combinations
    neutral = ['0', '0', '0r', '0b']
    combos = []
    for slot in range(4):
        for v in PARSE_VALUES:
            vals = list(neutral)
            vals[slot] = v
            combos.append(vals)
    for l, t, r, b in itertools.product(['26', '50%', '10r', '0'],
                                        ['204', '25%', '8b', '0'],
                                        ['+237', '100%', '13r', '+50%', '20'],
                                        ['+758', '100%', '6b', '+25%', '30']):
        combos.append([l, t, r, b])
    prior = [1.5, -2.25, 3.0, 4.75]
    for prec, cw in FPCW.items():
        m = Machine(cw)
        for vals in combos:
            rows.append({'precision': prec, 'input': vals, 'prior_bits': [f32bits(x) for x in prior],
                         **parse_case(m, vals, prior)})
    return rows


# ------------------------------------------------------------------ resolve
def resolve_case(m, raw, flags, parent, sx, sy, clip):
    b = m.alloc(0x40)
    for i, x in enumerate(raw):
        m.wf(b + 4 * i, x)
    for i in range(4, 12):
        m.w32(b + 4 * i, 0x7fc00001)  # poison: must be overwritten when relevant
    m.w32(b + 0x30, flags)
    p = m.alloc(16)
    for i, x in enumerate(parent):
        m.wf(p + 4 * i, x)
    ok = m.call(0x5E3370, b, [p, f32bits(sx), f32bits(sy), clip], 0x10) & 0xff
    return {'ok': ok, 'out_bits': [m.r32(b + 0x10 + 4 * i) for i in range(4)],
            'clip_bits': [m.r32(b + 0x20 + 4 * i) for i in range(4)],
            'parent_after_bits': [m.r32(p + 4 * i) for i in range(4)]}


FLAG_SETS = [0, 0x1, 0x2, 0x4, 0x8, 0x10, 0x20, 0x40, 0x80, 0x100, 0x200,
             0x100 | 0x40, 0x200 | 0x80, 0x10 | 0x20 | 0x40 | 0x80, 0x1 | 0x2 | 0x4 | 0x8,
             0x100 | 0x200, 0x10 | 0x100, 0x20 | 0x200 | 0x80, 0x4 | 0x8, 0x1 | 0x100,
             0x2 | 0x200, 0x1 | 0x4 | 0x20 | 0x80, 0x5, 0xA]
RAWS = [(26, 204, 237, 758), (0, 0, 0, 0), (783, 8, 237, 758), (50, 25, 100, 100),
        (13, 12, 13, 8), (-5, -7, 1200, 900), (12.5, 33.3, 0.1, 99.9), (300, 400, 10, 20)]
PARENTS = [(0, 0, 1024, 768), (0, 0, 2560, 1440), (1957.5, 13.333333, 2550, 1276.6666),
           (100, 50, 50, 40), (-10.25, 5.5, 300.75, 220.125)]
SCALES = [(1.0, 1.0), (2.5, 1.875), (1.25, 1.25), (0.78125, 0.78125), (1.875, 1.40625), (1.3339844, 1.0)]


def resolve_cases():
    rows = []
    for prec, cw in FPCW.items():
        m = Machine(cw)
        for raw, flags, parent, (sx, sy), clip in itertools.product(RAWS, FLAG_SETS, PARENTS, SCALES, (0, 1)):
            r = resolve_case(m, raw, flags, parent, sx, sy, clip)
            rows.append({'precision': prec, 'raw_bits': [f32bits(x) for x in raw], 'flags': flags,
                         'parent_bits': [f32bits(x) for x in parent], 'sx_bits': f32bits(sx),
                         'sy_bits': f32bits(sy), 'clip': clip, **r})
    return rows


# ------------------------------------------------------------------ VE layout
def ve_layout_case(m, screen, chain):
    """chain: list of (outer_raw, outer_flags, client_raw, client_flags, ve_flags)
    from root VE (no parent) to leaf. Returns every VE's state after laying
    out each in order (the native code lays a VE out from its parent's
    already-computed rects)."""
    avars = m.alloc(0x100)
    wref = m.alloc(4)
    href = m.alloc(4)
    m.wf(wref, 1024.0)
    m.wf(href, 768.0)
    m.w32(avars + 0x88, wref)
    m.w32(avars + 0x8c, href)
    m.w32(0xb39990, avars)
    m.w32(0xb396d0, screen[0])
    m.w32(0xb396d4, screen[1])
    ves = []
    parent = 0
    for outer, of, client, cf, vf in chain:
        ve = m.alloc(0x100)
        m.w32(ve + 8, parent)
        for i, x in enumerate(outer):
            m.wf(ve + 0x10 + 4 * i, x)
        m.w32(ve + 0x40, of)
        for i, x in enumerate(client):
            m.wf(ve + 0x44 + 4 * i, x)
        m.w32(ve + 0x74, cf)
        m.w32(ve + 0x88, vf)
        m.call(0x5E34B0, ve, [], 0)
        ves.append(ve)
        parent = ve
    out = []
    for ve in ves:
        out.append({'outer_rect_bits': [m.r32(ve + 0x20 + 4 * i) for i in range(4)],
                    'outer_clip_bits': [m.r32(ve + 0x30 + 4 * i) for i in range(4)],
                    'client_rect_bits': [m.r32(ve + 0x54 + 4 * i) for i in range(4)],
                    'client_clip_bits': [m.r32(ve + 0x64 + 4 * i) for i in range(4)],
                    've_flags': m.r32(ve + 0x88)})
    return out


SCREENS = [(1024, 768), (2560, 1440), (800, 600), (1280, 960), (1920, 1080), (1366, 768), (1440, 900), (3840, 2160)]
CHAINS = [
    # main menu panel group VE at L783 T8 +237 +758, with child decal and a
    # button (view L26 T204 +R +B) whose label client is L13 T12 13r 8b.
    [((783, 8, 237, 758), 0x300, (0, 0, 0, 0), 0xC, 0),
     ((22, 198, 191, 488), 0x300, (0, 0, 0, 0), 0xC, 0)],
    [((783, 8, 237, 758), 0x300, (0, 0, 0, 0), 0xC, 0),
     ((26, 204, 185, 46), 0x300, (17, 14, 13, 6), 0xC, 0x1000),
     ((13, 12, 13, 8), 0xC, (0, 0, 0, 0), 0xC, 0x1000)],
    # percent and right/bottom anchored, clipped against a small parent
    [((10, 10, 300, 200), 0x300, (5, 5, 5, 5), 0xC, 0),
     ((50, 50, 100, 100), 0x10 | 0x20 | 0x40 | 0x80, (0, 0, 0, 0), 0xC, 0x1000),
     ((-20, -20, 400, 400), 0x300, (0, 0, 0, 0), 0xC, 0x1000)],
    [((0, 0, 0, 0), 0xC, (0, 0, 0, 0), 0xC, 0),
     ((25, 10, 25, 10), 0x1 | 0x2 | 0x4 | 0x8, (1, 2, 3, 4), 0x10 | 0x20 | 0x100 | 0x200, 0),
     ((0, 0, 0, 0), 0xC, (0, 0, 0, 0), 0xC, 0x1000)],
    # zero-size VE sets the empty flag 0x400
    [((10, 10, 0, 50), 0x300, (0, 0, 0, 0), 0xC, 0x400)],
    [((10, 10, 30, 0), 0x300, (0, 0, 0, 0), 0xC, 0)],
]


def ve_cases():
    rows = []
    for prec, cw in FPCW.items():
        for screen in SCREENS:
            m = Machine(cw)
            for ci, chain in enumerate(CHAINS):
                rows.append({'precision': prec, 'screen': list(screen), 'chain_index': ci,
                             'chain': [{'outer_bits': [f32bits(x) for x in o], 'outer_flags': of,
                                        'client_bits': [f32bits(x) for x in c], 'client_flags': cf,
                                        've_flags': vf} for o, of, c, cf, vf in chain],
                             'result': ve_layout_case(m, screen, chain)})
    return rows


# ------------------------------------------------------------------ state mask
def state_cases():
    """0x5E25C0: VE state bits from widget flags (widget+0x24) and the
    visibility decision against the VE when-mask (VE+0x8c). 0x5EE2C0 (widget
    shown = flag bit 1 and render_threshold (widget+0x38) >= UI render level
    [*0xb3653c+0xdc]) executes natively; the vtable+0x60 visibility setter is
    a recording seam."""
    rows = []
    m = Machine(FPCW['pc53'])
    vt = m.alloc(0x100)
    setter = 0x2f00000
    m.u.mem_map(0x2f00000, 0x1000)
    m.u.mem_write(setter, bytes([0xC3]))
    m.w32(vt + 0x60, setter)
    seen = {}

    def set_visible():
        seen['arg'] = m.arg(0) & 0xff
        m.ret(0, 4)
    m.seams[setter] = set_visible
    gstate = m.alloc(0x400)
    m.w32(0xb3653c, gstate)
    m.w32(gstate + 500, 0x1234)
    m.w32(gstate + 0xdc, 5)
    bits = [19, 3, 8, 7, 9, 22, 1]
    whens = [0, 1, 2, 4, 8, 0x10, 0x20, 0x40, 0x80, 1 | 8, 1 | 4 | 0x20, 1 | 4 | 0x10, 0x100, 0x200, 0x400 | 1, 0x800,
             0x40 | 4 | 1, 0x80 | 8]
    for mask in range(128):
        wflags = 0
        for i, b in enumerate(bits):
            if mask >> i & 1:
                wflags |= 1 << b
        for when in whens:
            for threshold in (4, 5, 6):
                widget = m.alloc(0x40)
                m.w32(widget + 0x24, wflags)
                m.w32(widget + 0x38, threshold)
                ve = m.alloc(0x100)
                m.w32(ve, vt)
                m.w32(ve + 0xc, widget)
                m.w32(ve + 0x8c, when)
                seen.clear()
                m.call(0x5E25C0, ve, [], 0)
                rows.append({'widget_flags': wflags, 'render_threshold': threshold, 'render_level': 5,
                             'when': when, 'state': m.r32(ve + 0x90), 'visible': seen['arg'],
                             'frame': m.r32(ve + 0xa0)})
    return rows


def keyword_table(start, end):
    """Keyword -> value pairs stored by a table-building function, read from
    its instruction stream: each keyword string push is followed by the store
    of its value to [esp+0x10] (ebx is zero there, from 'xor ebx, ebx')."""
    from aadis import dis
    out = []
    keyword = None
    for ins in dis(start, end):
        if ins.mnemonic == 'push' and ins.op_str.startswith('0x'):
            v = int(ins.op_str, 16)
            if 0xa00000 <= v < 0xa60000:
                text = P.wstr(v, 40)
                if text.isupper() and text.replace('_', '').isalpha():
                    keyword = (text, v)
        elif keyword and ins.mnemonic == 'mov' and ins.op_str.startswith('dword ptr [esp + 0x10], '):
            src = ins.op_str.split(', ')[1]
            value = 0 if src == 'ebx' else int(src, 0)
            out.append({'keyword': keyword[0], 'bit': value, 'string_va': hex(keyword[1]), 'store_va': hex(ins.address)})
            keyword = None
    return out


def main():
    report = {
        'schema': 1,
        'original_sha256': SHA,
        'scope': __doc__,
        'when_table': keyword_table(0x5E4D00, 0x5E4D00 + 1291),
        'label_attribute_table': keyword_table(0x5E5210, 0x5E5210 + 654),
        'parse': parse_cases(),
        'resolve': resolve_cases(),
        've_layout': ve_cases(),
        'state': state_cases(),
    }
    out = HERE / 'native-layout.json'
    out.write_text(json.dumps(report, separators=(',', ':')) + '\n')
    print(json.dumps({k: len(v) for k, v in report.items() if isinstance(v, list)}),
          hashlib.sha256(out.read_bytes()).hexdigest())


if __name__ == '__main__':
    main()
