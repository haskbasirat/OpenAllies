"""Unicorn oracle for the AA.exe in-game camera input path (KKC_UI.cpp).

Runs the ORIGINAL machine code of the camera controls on laboratory inputs at
x87 PC24/PC53/PC64 and records every camera call at explicit seams:
  5CCC20 scroller acceleration            5995F0 impulse tick dt clamp
  5CD560/5CCD30/5CC7D0/5CCC90/5CD410/5CD6B0/5CD420/5CD320 scroller state
    machine with 59AB60/59AAD0 (QueryPerformanceCounter IAT 9F021C seamed),
    401290 clamp, _ftol2 96CC4C, 5E0570/5E0590/5E05B0/5C1FD0 state slot and
    the scroller/rotate native vtables A4DA84/A4D968 (seam: 700500 pan)
  5C2930/5C2950/5C2970/5C2990/5C29B0/5C2A10 impulses -> 5C2090/5C20F0/
    700440/7004A0/7010F0 (seams: 6FF220 pose, 7005F0 zoom)
  43BC80 DeltaZoom tail (seam: 426330 returns the parsed binary32 in ST0),
    43BC20 ZoomMin, 43BC50 ZoomMax
  5EFC70 wheel event -> double argument of 426FB0 (seam; 40A610 seamed)
  5C21C0/5CCEB0/5CCF00/5CC870 drag rotation (seams: 6FEBA0, 6FF130)
Every executed instruction of the image is checked byte-equal in the v14
image. Inputs are declared lab values, not an observed battle.

  python native_camera_input.py           verify the frozen fixture
  python native_camera_input.py --write   regenerate it
"""
from pathlib import Path
import sys, json, struct, hashlib, itertools
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/engine-baseline'))
sys.path.insert(0, str(ROOT / 'research/phase8-restoration'))
from audit_baseline import PE
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32, UC_HOOK_CODE
from unicorn.x86_const import *
import null_guard

P = PE(ROOT.parent / 'AA.exe')
V = PE(ROOT.parent / 'reconstruction/native-game/runtime-expanded-v14/AA.exe')
assert hashlib.sha256(P.b).hexdigest() == '5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26'
assert hashlib.sha256(V.b).hexdigest() == '3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1'
CW = {'Single24': 0x7f, 'Double53': 0x27f, 'Extended64': 0x37f}
HEAP, STUBS, STACK, STOP = 0x2000000, 0x2e00000, 0x2f08000, 0x2f10000
PINS = {}


def f32(v):
    return struct.unpack('<I', struct.pack('<f', v))[0]


class Machine:
    def __init__(self, pc):
        self.pc = pc
        u = self.u = Uc(UC_ARCH_X86, UC_MODE_32)
        u.mem_map(0, 0x1000)
        u.mem_map(P.base, (P.image_size + 0xfff) & ~0xfff)
        for vs, r, size, off in P.sections:
            u.mem_write(P.base + r, P.b[off:off + size])
        u.mem_map(HEAP, 0x1000000)
        u.mem_write(STUBS, b'\xcc' * 0x10000)
        self.next, self.next_stub = HEAP, STUBS + 0x100
        self.seams, self.calls, self.qpc = {}, [], []
        u.hook_add(UC_HOOK_CODE, self._hook)
        null_guard.install(self)
        self.cw = self.alloc(4)
        self.w(self.cw, CW[pc])
        # QueryPerformanceCounter import: scripted counters.
        self.w(0x9f021c, self.stub(self._qpc))

    def alloc(self, n):
        p = self.next
        self.next += (max(n, 4) + 15) & ~15
        self.u.mem_write(p, b'\0' * max(n, 4))
        return p

    def w(self, a, v):
        self.u.mem_write(a, struct.pack('<I', v & 0xffffffff))

    def r(self, a):
        return struct.unpack('<I', self.u.mem_read(a, 4))[0]

    def wb(self, a, v):
        self.u.mem_write(a, bytes([v & 0xff]))

    def b(self, a):
        return self.u.mem_read(a, 1)[0]

    def w64(self, a, v):
        self.u.mem_write(a, struct.pack('<q', v))

    def r64(self, a):
        return struct.unpack('<q', self.u.mem_read(a, 8))[0]

    def esp(self):
        return self.u.reg_read(UC_X86_REG_ESP)

    def arg(self, n):
        return self.r(self.esp() + 4 + 4 * n)

    def ret(self, pop=0, eax=None):
        sp = self.esp()
        self.u.reg_write(UC_X86_REG_EIP, self.r(sp))
        self.u.reg_write(UC_X86_REG_ESP, sp + 4 + pop)
        if eax is not None:
            self.u.reg_write(UC_X86_REG_EAX, eax & 0xffffffff)

    def stub(self, handler):
        a = self.next_stub
        self.next_stub += 16
        self.seams[a] = handler
        return a

    def code(self, raw):
        a = self.next_stub
        self.next_stub += (len(raw) + 15) & ~15
        self.u.mem_write(a, raw)
        return a

    def fld_ret(self, word, pop):
        """Leave a binary32 in ST0 and return popping `pop` argument bytes."""
        data = self.alloc(4)
        self.w(data, word)
        return self.code(b'\xd9\x05' + struct.pack('<I', data) + b'\xc2' + struct.pack('<H', pop))

    def _qpc(self, m):
        assert m.qpc, 'unscripted QueryPerformanceCounter'
        value = m.qpc.pop(0)
        m.w64(m.arg(0), value)
        m.calls.append(['qpc', value])
        m.ret(4, 1)

    def _hook(self, u, pc, size, _):
        if P.base <= pc < P.base + P.image_size:
            raw = P.range(pc, size)
            if pc not in PINS:
                assert raw == V.range(pc, size), hex(pc)
                PINS[pc] = raw.hex()
        h = self.seams.get(pc)
        if h is not None:
            h(self)
        elif STUBS <= pc < STUBS + 0x100:
            raise RuntimeError(('unhooked stub', hex(pc)))

    def call(self, entry, this, args, pop=None):
        # Fresh x87 state at the declared precision.
        init = self.code(b'\xdb\xe3\xd9\x2d' + struct.pack('<I', self.cw) + b'\xf4')
        self.u.emu_start(init, init + 8)
        self.w(STACK, STOP)
        for i, a in enumerate(args):
            self.w(STACK + 4 + 4 * i, a)
        self.u.reg_write(UC_X86_REG_ESP, STACK)
        self.u.reg_write(UC_X86_REG_ECX, this)
        self.u.emu_start(entry, STOP, count=500000)
        assert not self.null_accesses, self.null_accesses
        assert self.u.reg_read(UC_X86_REG_EIP) == STOP, hex(self.u.reg_read(UC_X86_REG_EIP))
        want = STACK + 4 + (4 * len(args) if pop is None else pop)
        assert self.esp() == want, ('stack', hex(entry), hex(self.esp()), hex(want))
        return self.u.reg_read(UC_X86_REG_EAX)

    def seam(self, address, handler):
        self.seams[address] = handler


def uvars(m, words):
    table = m.alloc(0x300)
    m.w(0xb39940, table)
    for off, word in words.items():
        p = m.alloc(4)
        m.w(p, word)
        m.w(table + off, p)
    return table


# ------------------------------------------------------------- 5CCC20
ACCEL_DT = [10000.0, 16666.666, 33333.332, 100000.0, 12345.678]
ACCEL_ACC = [0.0, 10000.0, 250000.0, 1999999.9, 5e6]
ACCEL_SPEED = [(2.0, 4.0, 10.0), (2.0, 0.25, 20.0), (3.3, 7.1, 9.9), (1.0, 20.0, 30.0)]
ACCEL_DIR = [(1.0, 0.0), (-1.0, 1.0), (0.0, -1.0), (1.0, 1.0)]


def accel_rows(pc):
    rows = []
    for dt, acc, speed, d in itertools.product(ACCEL_DT, ACCEL_ACC, ACCEL_SPEED, ACCEL_DIR):
        if (ACCEL_DT.index(dt) + ACCEL_ACC.index(acc) + ACCEL_SPEED.index(speed) + ACCEL_DIR.index(d)) % 3:
            continue
        m = Machine(pc)
        uvars(m, {0x18: f32(speed[0]), 0x1c: f32(speed[1]), 0x20: f32(speed[2])})
        s = m.alloc(0x30)
        m.w(s + 0x20, f32(acc))
        out, dr = m.alloc(8), m.alloc(8)
        m.w(dr, f32(d[0]))
        m.w(dr + 4, f32(d[1]))
        m.call(0x5ccc20, s, [out, dr, f32(dt)])
        rows.append(dict(precision=pc, accumulated=f32(acc), dt=f32(dt), direction=[f32(d[0]), f32(d[1])],
                         speed=[f32(x) for x in speed], stored=m.r(s + 0x20), out=[m.r(out), m.r(out + 4)]))
    return rows


# ------------------------------------------------------------- 5995F0
def tick_rows(pc):
    rows = []
    cases = [(0.0, 16666.0), (5e6, 5e6 + 16666.67), (5e6 + 100.0, 5e6), (1.0, 100001.0), (0.0, 100000.0),
             (0.0, 9999.5), (0.0, 10000.0), (3e7, 3e7 + 100000.5), (1.5e9, 1.5e9 + 9999.0), (2.0, 1e6),
             (1234.5, 1234.5), (8e8, 8e8 + 50000.0)]
    for last, now in cases:
        m = Machine(pc)
        got = []

        def fn(x):
            got.append(x.arg(0))
            x.ret(0)
        e = m.alloc(8)
        m.w(e, m.stub(fn))
        m.w(e + 4, f32(last))
        m.call(0x5995f0, e, [f32(now)])
        rows.append(dict(precision=pc, last=f32(last), now=f32(now), dt=got[0], stored_last=m.r(e + 4)))
    return rows


# ------------------------------------------------------------- scroller
class World:
    """Laboratory UI/VWorld/Input/Camera objects for the scroller and rotate
    state machines. Vtables are the native ones in the image."""

    def __init__(self, pc, speed=(2.0, 4.0, 10.0), rect=(0, 0, 799, 599), freq=3579545):
        m = self.m = Machine(pc)
        uvars(m, {0x18: f32(speed[0]), 0x1c: f32(speed[1]), 0x20: f32(speed[2])})
        self.ui = m.alloc(0x200)
        m.w(0xb3653c, self.ui)
        self.scroller = m.alloc(0x30)
        m.w(self.scroller, 0xa4da84)
        self.rotate = m.alloc(0x20)
        m.w(self.rotate, 0xa4d968)
        self.cursor = m.alloc(0x20)
        m.w(self.ui + 0x40, self.scroller)
        m.w(self.ui + 0x48, self.rotate)
        m.w(self.ui + 0x58, self.cursor)
        self.vworld = m.alloc(0x200)
        m.w(0xb36538, self.vworld)
        self.widget = m.alloc(0x10)
        m.w(self.vworld + 0x14, self.widget)
        # UI+8C points at a node whose first DWORD is the hover widget (5EFA86).
        self.hover = m.alloc(8)
        m.w(self.hover, self.widget)
        pick = m.alloc(8)
        pvt = m.alloc(0x40)
        m.w(pick, pvt)
        m.w(pvt + 0x38, m.stub(lambda x: (x.calls.append(['pick', x.arg(0), x.arg(1)]), x.ret(8))))
        m.w(self.vworld + 0x13c, pick)
        self.visual = m.alloc(0x100)
        m.w(0xb36518, self.visual)
        self.camera = m.alloc(0x400)
        cell = m.alloc(4)
        m.w(cell, self.camera)
        m.w(0xb40048, cell)
        self.input = m.alloc(0x40)
        m.w(0xb39b18, self.input)
        for i, v in enumerate(rect):
            m.w(self.input + 0x14 + 4 * i, v)
        m.w64(0xb3d760, freq)
        m.seam(0x700500, self._pan)
        self.set(vworld_ad=1, vworld_bit19=1, visual_bc=1, app_active=1, inside=1, hover=1, d8=0, d9=0, locked=0)

    def _pan(self, x):
        d = x.arg(1)
        x.calls.append(['pan', x.arg(0), [x.r(d), x.r(d + 4)]])
        x.ret(8)

    def set(self, **k):
        m = self.m
        if 'vworld_ad' in k:
            m.wb(self.vworld + 0xad, k['vworld_ad'])
        if 'vworld_bit19' in k:
            m.w(self.vworld + 0x24, (k['vworld_bit19'] << 19) | 0x5)
        if 'visual_bc' in k:
            m.wb(self.visual + 0xbc, k['visual_bc'])
        if 'app_active' in k:
            m.w(0xb396c4, 0x1 | (k['app_active'] << 1))
        if 'inside' in k:
            m.wb(self.input + 0x24, k['inside'])
        if 'hover' in k:
            m.w(self.ui + 0x8c, self.hover if k['hover'] else 0)
        if 'd8' in k:
            m.wb(0xb396d8, k['d8'])
        if 'd9' in k:
            m.wb(0xb396d9, k['d9'])
        if 'locked' in k:
            m.wb(self.camera + 0x214, k['locked'])

    def active(self):
        a = self.m.r(self.ui + 0xc0)
        return {0: None, self.scroller: 'scroller', self.rotate: 'rotate'}[a]

    def snapshot(self):
        m, s = self.m, self.scroller
        return dict(bits=m.r(s + 4), edge=m.r(s + 8), keys=m.r(s + 0xc), direction=[m.r(s + 0x10), m.r(s + 0x14)],
                    anchor=m.r64(s + 0x18), accumulated=m.r(s + 0x20), millis=m.r(s + 0x24), active=self.active())

    def op(self, op):
        m = self.m
        kind = op[0]
        m.calls = []
        if kind == 'begin':
            m.call(0x5cd560, self.scroller, [op[1]])
        elif kind == 'end':
            m.call(0x5ccd30, self.scroller, [op[1]])
        elif kind == 'cursor':
            m.w(self.cursor + 4, op[1])
            m.w(self.cursor + 8, op[2])
            if m.r(self.ui + 0xc0) == self.scroller:
                m.call(0x5cd410, self.scroller, [])
        elif kind == 'frame':
            m.qpc = list(op[1])
            m.call(0x5cd6b0, 0, [])
            if m.r(self.ui + 0xc0) == self.scroller:
                m.call(0x5cd320, self.scroller, [])
            assert not m.qpc, 'unused scripted counters'
        elif kind == 'set':
            self.set(**op[1])
        elif kind == 'qpc':
            m.qpc = list(op[1])
            return
        return [c for c in m.calls]


SEQUENCES = {
    'arrow_hold_left_up': ([
        ('qpc', [1000]), ('begin', 4), ('frame', [1000 + 20000, 1000 + 20000]),
        ('frame', [21000 + 59659, 21000 + 59659]), ('begin', 1), ('frame', [80659 + 3580000, 80659 + 3580000]),
        ('end', 4), ('frame', [3660659 + 50000, 3660659 + 50000]), ('end', 1), ('frame', [])],
        dict(start=0)),
    'opposite_keys_and_locked': ([
        ('qpc', [7]), ('begin', 4), ('begin', 8), ('frame', [100007, 100007]), ('end', 8),
        ('set', dict(locked=1)), ('frame', [200007]), ('set', dict(locked=0)),
        ('set', dict(inside=0)), ('frame', [300007]), ('set', dict(inside=1)),
        ('frame', [300008, 300008]), ('end', 4)],
        dict(start=0)),
    'edge_scroll': ([
        ('cursor', 0, 300), ('frame', [5000, 5000 + 60000, 5000 + 60000]),
        ('frame', [65000 + 60000, 65000 + 60000]), ('cursor', 799, 0),
        ('frame', [125000 + 70000, 125000 + 70000]), ('cursor', 400, 599),
        ('frame', [195000 + 30000, 195000 + 30000]), ('cursor', 400, 300), ('frame', [])],
        dict(start=0)),
    'edge_gates': ([
        ('set', dict(hover=0)), ('cursor', 0, 0), ('frame', []), ('set', dict(hover=1, d8=1, d9=0)),
        ('frame', []), ('set', dict(d9=1)), ('frame', [10, 3000000, 3000000]),
        ('set', dict(visual_bc=0)), ('frame', []), ('set', dict(visual_bc=1, vworld_ad=0)),
        ('begin', 2), ('frame', []), ('set', dict(vworld_ad=1)), ('cursor', 5, 5), ('frame', []),
        ('set', dict(vworld_bit19=0)), ('cursor', 0, 0), ('frame', []), ('set', dict(vworld_bit19=1, app_active=0)),
        ('frame', [4000000, 4000000 + 40000]), ('set', dict(app_active=1)), ('frame', [4100000, 4100000])],
        dict(start=0)),
    'key_then_edge_accumulate': ([
        ('qpc', [0]), ('begin', 2), ('frame', [30000, 30000]), ('cursor', 0, 599),
        ('frame', [60000, 60000]), ('end', 2), ('frame', [90000, 90000]), ('cursor', 300, 300),
        ('frame', [])],
        dict(start=0)),
}


def scroller_rows(pc):
    rows = []
    for name, (ops, cfg) in SEQUENCES.items():
        for speed in [(2.0, 4.0, 10.0), (2.0, 0.25, 20.0)]:
            w = World(pc, speed)
            m = w.m
            w.m.w64(w.scroller + 0x18, cfg['start'])
            m.w(w.cursor + 4, 400)
            m.w(w.cursor + 8, 300)
            steps = []
            for op in ops:
                calls = w.op(op)
                if calls is None:
                    continue
                steps.append(dict(op=list(op[:1]) + [list(x) if isinstance(x, tuple) else x for x in op[1:]],
                                  calls=calls, state=w.snapshot()))
            rows.append(dict(precision=pc, name=name, speed=[f32(x) for x in speed], frequency=3579545,
                             rect=[0, 0, 799, 599], start_anchor=cfg['start'], cursor=[400, 300], steps=steps))
    return rows


# ------------------------------------------------------------- impulses
def camera_world(m, cam):
    k = m.alloc(0x400)
    cell = m.alloc(4)
    m.w(cell, k)
    m.w(0xb40048, cell)
    for off, word in cam.items():
        if off == 0x214:
            m.wb(k + off, word)
        else:
            m.w(k + off, word)
    return k


def impulse_rows(pc):
    rows = []
    cams = [
        {0x214: 0, 0x2bc: f32(-0.7853982), 0x2b8: f32(0.6981317), 0x208: f32(1.0), 0x24c: f32(64.5),
         0x250: f32(32.25), 0x254: f32(12.5)},
        {0x214: 0, 0x2bc: f32(3.0), 0x2b8: f32(1.2), 0x208: f32(0.7), 0x24c: f32(1.0), 0x250: f32(2.0),
         0x254: f32(3.0)},
        {0x214: 1, 0x2bc: f32(0.0), 0x2b8: f32(0.5), 0x208: f32(1.2), 0x24c: 0, 0x250: 0, 0x254: 0},
    ]
    var_sets = [{0x18: f32(2.0), 0x2c: f32(10.0), 0x30: f32(10.0), 0x34: f32(0.5)},
                {0x18: f32(7.25), 0x2c: f32(3.3), 0x30: f32(0.125), 0x34: f32(1.15)}]
    pitch_avars = [(f32(35.0), f32(10.0), f32(80.0), f32(15.0)), (f32(40.0), f32(41.5), f32(42.0), f32(41.9)),
                   (f32(40.0), f32(0.0), f32(90.0), f32(80.0))]
    fns = [(0x5c2930, 'yawleft'), (0x5c2950, 'yawright'), (0x5c2970, 'PitchUp'), (0x5c2990, 'PitchDown'),
           (0x5c29b0, 'ZoomIn'), (0x5c2a10, 'ZoomOut')]
    dts = [10000.0, 16666.666, 100000.0, 33333.0]
    for (entry, name), cam, vs, pa, dt in itertools.product(fns, cams, var_sets, pitch_avars, dts):
        if (fns.index((entry, name)) + cams.index(cam) + var_sets.index(vs) + pitch_avars.index(pa) + dts.index(dt)) % 2:
            continue
        if entry not in (0x5c2970, 0x5c2990) and pitch_avars.index(pa):
            continue
        m = Machine(pc)
        uvars(m, vs)
        k = camera_world(m, cam)
        at = m.alloc(0x100)
        m.w(0xb39990, at)
        rec = m.alloc(0x30)
        m.w(rec, pa[0])
        m.w(rec + 0x14, pa[1])
        m.w(rec + 0x18, pa[2])
        m.w(at + 0x50, rec)
        mn = m.alloc(4)
        m.w(mn, pa[3])
        m.w(at + 0x54, mn)
        m.calls = []
        m.seam(0x6ff220, lambda x: (x.calls.append(['pose', [x.arg(i) for i in range(6)]]), x.ret(0x18)))
        m.seam(0x7005f0, lambda x: (x.calls.append(['zoom', x.arg(0)]), x.ret(4)))
        m.call(entry, 0, [f32(dt)], pop=0)
        rows.append(dict(precision=pc, function=name, dt=f32(dt), camera={hex(o): v for o, v in cam.items()},
                         uvars={hex(o): v for o, v in vs.items()}, pitch_avar=list(pa), calls=m.calls))
    # 700440/7004A0 equality and lock semantics on direct calls.
    for entry, which in [(0x700440, 'yaw'), (0x7004a0, 'pitch')]:
        for value, cam in [(f32(-0.7853982), cams[0]), (f32(0.6981317), cams[0]), (f32(0.25), cams[0]),
                           (f32(0.25), cams[2])]:
            m = Machine(pc)
            k = camera_world(m, cam)
            m.calls = []
            m.seam(0x6ff220, lambda x: (x.calls.append(['pose', [x.arg(i) for i in range(6)]]), x.ret(0x18)))
            m.call(entry, k, [value])
            rows.append(dict(precision=pc, function=which, value=value,
                             camera={hex(o): v for o, v in cam.items()}, calls=m.calls))
    return rows


# ------------------------------------------------------------- zoom commands
def zoom_rows(pc):
    rows = []
    for parsed, zoom in itertools.product([0.1, -0.1, 1.0, -1.0, 0.025, 0.0, 0.8333333], [1.0, 0.7, 1.21, 0.0123]):
        m = Machine(pc)
        k = camera_world(m, {0x208: f32(zoom)})
        m.calls = []
        stub = m.fld_ret(f32(parsed), 8)

        def parse(x, stub=stub):
            x.calls.append(['parse', x.arg(0), x.arg(1)])
            x.u.reg_write(UC_X86_REG_EIP, stub)
        m.seam(0x426330, parse)
        m.seam(0x7005f0, lambda x: (x.calls.append(['zoom', x.arg(0)]), x.ret(4)))
        m.call(0x43bc80, 0, [m.alloc(16)])
        rows.append(dict(precision=pc, command='DeltaZoom', parsed=f32(parsed), zoom=f32(zoom), calls=m.calls))
    for entry, name, off in [(0x43bc20, 'ZoomMin', 0x20c), (0x43bc50, 'ZoomMax', 0x210)]:
        for limit in [0.7, 1.21, 0.1, 1.5, 0.3333333]:
            m = Machine(pc)
            camera_world(m, {off: f32(limit)})
            m.calls = []
            m.seam(0x7005f0, lambda x: (x.calls.append(['zoom', x.arg(0)]), x.ret(4)))
            m.call(entry, 0, [0])
            rows.append(dict(precision=pc, command=name, limit=f32(limit), calls=m.calls))
    return rows


def wheel_rows(pc):
    rows = []
    for delta in [120, -120, 240, -360, 30, -1, 1, 7, 32767, -32768]:
        m = Machine(pc)
        m.calls = []
        h = m.alloc(0x20)
        s = m.alloc(0x40)
        m.w(s + 4, 12)  # nonzero CString length at data-0xC
        m.w(h + 8, s + 0x10)
        m.seam(0x40a610, lambda x, h=h: x.ret(8, h))
        m.seam(0x426fb0, lambda x: (x.calls.append(['format', struct.unpack('<Q', bytes(x.u.mem_read(x.esp() + 12, 8)))[0]]), x.ret(0)))
        e = m.alloc(0x20)
        m.w(e + 4, 1 | 0x80)
        m.u.mem_write(e + 8, struct.pack('<Hh', 0x110, delta))
        m.call(0x5efc70, m.alloc(0x100), [e])
        rows.append(dict(precision=pc, delta=delta, argument=m.calls[0][1]))
    return rows


# ------------------------------------------------------------- drag rotation
def rotate_rows(pc):
    rows = []
    for speed, moves, rotate_flag in [(0.5, [(400, 300), (410, 300), (405, 290), (405, 290), (-20, 0)], 1),
                                      (1.75, [(100, 100), (99, 100), (300, 400)], 1),
                                      (0.5, [(400, 300), (420, 300)], 0)]:
        w = World(pc)
        m = w.m
        table = m.r(0xb39940)
        for off, word in [(0x38, f32(speed))]:
            p = m.alloc(4)
            m.w(p, word)
            m.w(table + off, p)
        flag = m.alloc(4)
        m.wb(flag, rotate_flag)
        m.w(table + 0xb4, flag)
        for off, word in {0x2bc: f32(-0.7853982), 0x2b8: f32(0.6981317), 0x24c: f32(64.5), 0x250: f32(32.25),
                          0x254: f32(12.5)}.items():
            m.w(w.camera + off, word)
        m.seam(0x6fe9c0, lambda x: x.ret(0, 0))
        m.seam(0x6feba0, lambda x: (x.calls.append(['rotate', x.arg(0), x.arg(1)]), x.ret(8)))
        m.seam(0x6ff130, lambda x: (x.calls.append(['look_at', x.arg(0), x.arg(1)]), x.ret(8)))
        steps = []
        m.calls = []
        m.call(0x5c21c0, w.ui, [0])
        steps.append(dict(op=['begin'], calls=m.calls, active=w.active(), rate=m.r(w.rotate + 4),
                          first=m.b(w.rotate + 0x10)))
        for x, y in moves:
            m.calls = []
            m.w(w.cursor + 4, x & 0xffffffff)
            m.w(w.cursor + 8, y & 0xffffffff)
            if m.r(w.ui + 0xc0) == w.rotate:
                m.call(0x5ccf00, w.rotate, [])
            steps.append(dict(op=['cursor', x, y], calls=m.calls, active=w.active(),
                              last=[m.r(w.rotate + 8), m.r(w.rotate + 0xc)]))
        m.calls = []
        m.call(0x5c1370, w.ui, [0])
        steps.append(dict(op=['end'], calls=m.calls, active=w.active(), rate=m.r(w.rotate + 4)))
        rows.append(dict(precision=pc, rotate_speed=f32(speed), camera_rotate=rotate_flag,
                         camera={'0x2bc': f32(-0.7853982), '0x2b8': f32(0.6981317), '0x24c': f32(64.5),
                                 '0x250': f32(32.25), '0x254': f32(12.5)}, steps=steps))
    return rows


def main():
    doc = dict(scope=__doc__.strip().splitlines()[0], sections={})
    for name, fn in [('accelerate_5ccc20', accel_rows), ('impulse_dt_5995f0', tick_rows),
                     ('scroller', scroller_rows), ('impulses', impulse_rows), ('zoom', zoom_rows),
                     ('wheel', wheel_rows), ('rotate', rotate_rows)]:
        rows = []
        for pc in CW:
            rows += fn(pc)
        doc['sections'][name] = rows
    doc['instructions'] = {f'{a:08x}': PINS[a] for a in sorted(PINS)}
    doc['harness_sha256'] = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    out = HERE / 'native-camera-input.json'
    text = json.dumps(doc, indent=1) + '\n'
    if '--write' in sys.argv:
        out.write_text(text)
    else:
        assert json.loads(out.read_text()) == json.loads(text), 'fixture differs from fresh native execution'
    print(json.dumps({k: len(v) for k, v in doc['sections'].items()} | dict(pins=len(PINS),
          sha256=hashlib.sha256(out.read_bytes()).hexdigest())))


if __name__ == '__main__':
    main()
