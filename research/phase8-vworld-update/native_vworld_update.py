"""VWorld per-frame update oracle (AA.exe image base 0x400000).

Runs the UNCHANGED original instructions of the VWorld update family under
Unicorn (research/phase8-movement/harness.py: every executed in-image byte is
compared with the accepted v14 image and recorded; page-0 null guard; the
engine allocator is a named seam). Every child that is not part of the kernel
under test is an explicit, recorded seam:

  43AB30  VWorld view time (+84/+88/+8C/+90); seam 76B120 (Sim render time).
  76B120  KKC_Sim render time (+34, clamped to World+E0..E4); seams 74B180
          (admin clock estimate) and 76B080 (RenderDumpAVI frame time).
  50C750  World+130 broadcast walk; 50C600 runs natively; its children
          427950 (script Eval), 432E40, 43B270, 440190, 4408A0, 43CD00,
          43CC50 are seams.
  431E10  VWorld+BC rect append (4319A0 growth runs natively over the
          allocator seam).
  4318E0  VWorld+BC rect drain; 431390 runs natively with the CRT floor/ceil
          (96D5E0/96D4C0, both the SSE2 and the x87 path) and 96CC4C; seam
          4F0E50 (World ground height at flags 0x122).
  43DA00  VWorld+98 deferred-view prune; seams view vt+BC, 43CF00 and the
          6F7430 node free.
  43FF60  VWorld vtable 9FDFF8+C update; every child is a seam except
          41C0E0/41C110, which run natively with argument 5.
  43EC60  WorldView update; seams for 50C750, 43A400, 4318E0, 4312F0,
          725340, view vt+74, 45B720, 4289A0, 429050, 45C7A0, 855D30;
          50C510 runs natively over the allocator seam.

No game process is started and no original file is modified.
Usage: python native_vworld_update.py [--write]
"""
from pathlib import Path
import hashlib, itertools, json, struct, sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase8-movement'))
import harness as h  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX, UC_X86_REG_EIP, UC_X86_REG_FPSW  # noqa: E402

PCS = list(h.PRECISION)


def fb(x):
    return struct.unpack('<I', struct.pack('<f', x))[0]


def fx(bits):
    return struct.unpack('<f', struct.pack('<I', bits & 0xffffffff))[0]


class M(h.Machine):
    """Harness machine plus a call trace, CString helper and float wrapper."""

    def __init__(self, precision):
        super().__init__(precision)
        self.trace = []
        self.seams[0x6f7810] = self._alloc
        self.seams[0x6f7430] = self._free
        self.alloc_null = False

    def _alloc(self, m):
        n = m.arg(0)
        p = 0 if self.alloc_null else m.alloc(n)
        self.trace.append(['6F7810', n, int(p == 0)])
        m.ret(0, p)

    def _free(self, m):
        self.trace.append(['6F7430', self.label(m.arg(0))])
        m.ret(0)

    def label(self, p):
        return self.labels.get(p, hex(p)) if hasattr(self, 'labels') else hex(p)

    def cstring(self, text):
        raw = text.encode('utf-16le')
        base = self.alloc(16 + len(raw) + 2)
        self.w32(base, 0x1234, len(text), len(text), 1000)
        self.u.mem_write(base + 16, raw + b'\0\0')
        return base + 16

    def cstring_text(self, p):
        return bytes(self.u.mem_read(p, self.r32(p - 12) * 2)).decode('utf-16le')

    def st0_f32(self, entry, ecx, args):
        """Call entry and store its ST0 result as binary32 (fstp dword)."""
        out = self.alloc(16)
        code = bytearray()
        for a in reversed(args):
            code += b'\x68' + struct.pack('<I', a)
        stub = self.stub_next
        self.stub_next += 64
        # call rel32 to entry
        code += b'\xe8' + struct.pack('<i', entry - (stub + len(code) + 5))
        code += b'\xd9\x1d' + struct.pack('<I', out) + b'\xc3'
        self.u.mem_write(stub, bytes(code))
        self.call(stub, ecx, [])
        assert self.fpu_top() == 0, 'unbalanced x87 stack'
        return self.r32(out)


# --------------------------------------------------------------- 43AB30
def view_time(pc, t, old8c, e0, e4):
    m = M(pc)
    world, sim, vworld = m.alloc(0x300), m.alloc(0x40), m.alloc(0x300)
    m.w32(0xb36534, world)
    m.w32(0xb36554, sim)
    m.w32(world + 0xe0, e0, e4)
    m.w32(vworld + 0x84, 0xa5a5a5a5, 0x5a5a5a5a, old8c, 0x3c3c3c3c)

    def sim_time(mm):
        assert mm.ecx() == sim and mm.arg(0) == 0
        m.trace.append(['76B120', 0])
        mm.ret_float(t, 'f32', 4)
    m.seams[0x76b120] = sim_time
    m.call(0x43ab30, vworld, [])
    assert m.fpu_top() == 0
    return dict(words=m.r32(vworld + 0x84, 4), trace=m.trace)


def view_time_cases():
    rows = []
    ts = [0, fb(1234.5678), fb(0.0625), fb(-3.0), fb(1e-30), fb(7.0)]
    olds = [0, fb(1234.5), fb(0.1)]
    spans = [(0, 0), (fb(1000.1), fb(1000.1625)), (fb(3.3), fb(3.3)), (fb(3.3), fb(3.30001)),
             (fb(5.0), fb(4.0)), (fb(0.0), fb(1e-4)), (fb(0.0), fb(1.000001e-4)), (fb(-2.5), fb(9.75))]
    for pc, t, old, (e0, e4) in itertools.product(PCS, ts, olds, spans):
        c = dict(precision=pc, t=t, old8c=old, e0=e0, e4=e4)
        rows.append(dict(input=c, output=view_time(pc, t, old, e0, e4)))
    return rows


# --------------------------------------------------------------- 76B120
def sim_time(c):
    m = M(c['precision'])
    world, session, uvars, node, sim = (m.alloc(0x300), m.alloc(0x200), m.alloc(0x400), m.alloc(0x40), m.alloc(0x40))
    auto, avi = m.alloc(4), m.alloc(4)
    m.w32(0xb36534, world)
    m.w32(0xb36550, session)
    m.w32(0xb39940, uvars)
    m.w32(0xb36558, node)
    m.w32(0xb396c4, c['b396c4'])
    m.u.mem_write(session + 0xf4, bytes([c['session_f4']]))
    m.w32(uvars + 0x350, auto)
    m.w32(uvars + 0x1c8, avi)
    m.u.mem_write(auto, bytes([c['auto_pause']]))
    m.u.mem_write(avi, bytes([c['avi']]))
    m.w32(world + 0xe0, c['e0'], c['e4'])
    m.w32(sim + 0x10, *c['times'])
    m.u.mem_write(sim + 0x28, bytes([c['paused'], c['frozen']]))
    m.w32(sim + 0x34, c['current'])

    def admin(mm):
        assert mm.ecx() == node
        m.trace.append(['74B180', mm.arg(0)])
        mm.ret_float(c['admin'], 'f32', 4)

    def avi_time(mm):
        assert mm.ecx() == sim
        m.trace.append(['76B080'])
        mm.ret_float(c['avi_time'], 'f32', 0)
    m.seams[0x74b180] = admin
    m.seams[0x76b080] = avi_time
    result = m.st0_f32(0x76b120, sim, [c['argument']])
    return dict(result=result, current=m.r32(sim + 0x34), trace=m.trace)


def sim_time_cases():
    rows = []
    gates = [dict(session_f4=1, b396c4=0, auto_pause=1), dict(session_f4=0, b396c4=2, auto_pause=1),
             dict(session_f4=0, b396c4=1, auto_pause=1), dict(session_f4=0, b396c4=0, auto_pause=0)]
    timelines = [[fb(10.0), fb(4.0), fb(10.0625), fb(5.0)], [fb(0.5), fb(1.0), fb(0.5625), fb(2.0)],
                 [fb(100.1), fb(3.3), fb(100.2), fb(3.4)]]
    admins = [fb(4.5), fb(5.0), fb(1.7), fb(3.31), fb(0.0)]
    currents = [0, fb(10.03), fb(100.15)]
    for pc, g, (paused, frozen), times, admin, current, (e0, e4) in itertools.product(
            PCS, gates, [(0, 0), (1, 0), (0, 1)], timelines, admins, currents,
            [(fb(10.0), fb(10.0625)), (fb(0.0), fb(200.0)), (fb(100.125), fb(100.1875))]):
        if (paused or frozen) and (admin != admins[0] or times != timelines[0]):
            continue
        c = dict(precision=pc, argument=0, avi=0, avi_time=0, paused=paused, frozen=frozen, times=times,
                 admin=admin, current=current, e0=e0, e4=e4, **g)
        rows.append(dict(input=c, output=sim_time(c)))
    for pc, current in itertools.product(PCS, [0, fb(3.0)]):
        c = dict(precision=pc, argument=0, avi=1, avi_time=fb(12.25), paused=0, frozen=0, times=timelines[0],
                 admin=admins[0], current=current, e0=fb(10.0), e4=fb(10.0625), session_f4=1, b396c4=0, auto_pause=1)
        rows.append(dict(input=c, output=sim_time(c)))
    return rows


# --------------------------------------------------------------- 50C750
KIND_SEAMS = {
    0x432e40: '432E40', 0x43b270: '43B270', 0x440190: '440190', 0x4408a0: '4408A0',
    0x43cd00: '43CD00', 0x43cc50: '43CC50',
}


def messages(c):
    """c: view, messages [{kind, text|thing|...}], script (B36500)."""
    m = M(c['precision'])
    vworld, script, manager = m.alloc(0x300), m.alloc(0x40), m.alloc(0x20)
    m.w32(0xb36538, vworld)
    m.w32(0xb36500, script)
    count = len(c['messages'])
    data = m.alloc(0x4c * max(count, 1))
    m.w32(manager + 4, vworld if c['view'] else 0)
    m.w32(manager + 0x10, data, count, max(count, 1))
    things = {}
    for i, msg in enumerate(c['messages']):
        p = data + i * 0x4c
        m.w32(p, msg['kind'])
        m.w32(p + 4, msg.get('gtid', 0), 0x1100 + i)
        m.w32(p + 0x24, 0x2400 + i, msg.get('gtid28', 0))
        m.w32(p + 0x2c, fb(1.0 + i), fb(2.0), fb(3.0), fb(0.5), fb(0.25), fb(0.125))
        m.w32(p + 0x44, 0x4400 + i)
        m.w32(p + 0x48, m.cstring(msg.get('text', '')))
        if msg.get('thing'):
            t = m.alloc(0x20)
            m.w32(t + 0xc, 0xc000 + i)
            things[p + 4] = t
            things[p + 0x28] = t
    created = {}

    def eval_427950(mm):
        assert mm.ecx() == script
        m.trace.append(['427950', m.cstring_text(m.r32(mm.arg(0))), mm.arg(1)])
        mm.ret(8)

    def lookup(mm):
        key = mm.ecx()
        m.trace.append(['432E40', key - data])
        mm.ret(0, things.get(key, 0))

    def relative(v):
        if data <= v < data + 0x4c * max(count, 1):
            return f'msg{(v - data) // 0x4c}+{(v - data) % 0x4c:x}'
        if v in things.values():
            return 'thing'
        return v

    def generic(name, pop, result=None):
        def seam(mm):
            m.trace.append([name, mm.ecx() == vworld] + [relative(mm.arg(i)) for i in range(pop // 4)])
            value = 0
            if result:
                value = m.alloc(0x60)
                created[name] = value
            mm.ret(pop, value)
        return seam
    m.seams[0x427950] = eval_427950
    m.seams[0x432e40] = lookup
    m.seams[0x43b270] = generic('43B270', 8)
    m.seams[0x440190] = generic('440190', 0)
    m.seams[0x4408a0] = generic('4408A0', 0)
    m.seams[0x43cd00] = generic('43CD00', 24, c.get('vfx_result', True))
    m.seams[0x43cc50] = generic('43CC50', 20, c.get('vfx_result', True))
    m.call(0x50c750, manager, [])
    return dict(count=m.r32(manager + 0x14), array=m.r32(manager + 0x10) == data,
                vfx_50=[m.r32(v + 0x50) for v in created.values()], trace=m.trace)


def message_cases():
    rows = []
    sets = [
        [],
        [dict(kind=1, text='WaitWorld'), dict(kind=1, text='TriggerListUpdate')],
        [dict(kind=1, text='')],
        [dict(kind=1, text='a'), dict(kind=1, text='b c'), dict(kind=1, text='TriggerListUpdate')],
        [dict(kind=0, gtid=7, thing=True), dict(kind=0, gtid=8), dict(kind=1, text='x')],
        [dict(kind=2), dict(kind=3), dict(kind=1, text='after')],
        [dict(kind=4, thing=True), dict(kind=4, gtid28=0), dict(kind=4, gtid28=5), dict(kind=5)],
    ]
    for pc, view, msgs in itertools.product(PCS, [True, False], sets):
        c = dict(precision=pc, view=view, messages=msgs)
        rows.append(dict(input=c, output=messages(c)))
    return rows


# --------------------------------------------------------------- 431E10 / 4318E0
def heightfield(m, c):
    hf = m.alloc(0x40)
    cols, rows = c['columns'], c['rows']
    heights = m.alloc(4 * cols * rows)
    for i in range(cols * rows):
        m.w32(heights + 4 * i, 0x7f7f0000 + i)
    m.w32(hf + 4, heights, cols * rows, cols * rows)
    m.w32(hf + 0x10, c['step'], c['inverse'], cols, rows, c['limits'][0], c['limits'][1])
    rects = m.alloc(16 * 10)
    for i in range(10):
        m.w32(rects + 16 * i, 0, 0, 0xa5a5a5a5, 0xa5a5a5a5)
    m.w32(hf + 0x28, rects, 0, 10)
    return hf, heights


def rects(c):
    m = M(c['precision'])
    m.w32(0xb4b340, c['sse2'])
    world = m.alloc(0x300)
    m.w32(0xb36534, world)
    hf, heights = heightfield(m, c)
    for x, y, wdt, hgt in c['append']:
        xy = m.alloc(8)
        m.w32(xy, x, y)
        m.call(0x431e10, hf, [xy, wdt, hgt])
    appended = [m.r32(m.r32(hf + 0x28) + 16 * i, 4) for i in range(m.r32(hf + 0x2c))]
    samples = []

    def ground(mm):
        assert mm.ecx() == world and mm.arg(1) == 0x122
        p = mm.arg(0)
        xy = m.r32(p, 2)
        i = len(samples)
        samples.append(xy)
        mm.ret_float(c['height'][i % len(c['height'])], 'f32', 8)
    m.seams[0x4f0e50] = ground
    m.call(0x4318e0, hf, [])
    assert m.fpu_top() == 0
    cols, rows = c['columns'], c['rows']
    return dict(appended=appended, count=m.r32(hf + 0x2c), samples=samples,
                heights=m.r32(heights, cols * rows), limits=m.r32(hf + 0x20, 2), trace=m.trace)


def rect_cases():
    rows = []
    hs = [[fb(-1.5), fb(4.25), fb(0.0), fb(17.0), fb(-0.0)], [fb(3.0)]]
    appends = [
        [],
        [[fb(0.0), fb(0.0), fb(2.0), fb(2.0)]],
        [[fb(1.0), fb(3.0), fb(4.0), fb(4.0)], [fb(-5.0), fb(-5.0), fb(4.0), fb(4.0)]],
        [[fb(2.5), fb(0.3), fb(1.7), fb(0.2)], [fb(100.0), fb(100.0), fb(8.0), fb(8.0)]],
        [[fb(0.1), fb(0.1), fb(0.1), fb(0.1)]] * 11,
        [[fb(-0.5), fb(1.25), fb(6.0), fb(3.0)], [fb(3.999), fb(4.001), fb(0.002), fb(0.002)]],
    ]
    grids = [dict(columns=6, rows=5, step=fb(2.0), inverse=fb(0.5)),
             dict(columns=4, rows=7, step=fb(3.0), inverse=fb(1 / 3)),
             dict(columns=5, rows=5, step=fb(0.7), inverse=fb(1 / 0.7))]
    for pc, sse2, grid, app, hgt, limits in itertools.product(PCS, [0, 1], grids, appends, hs,
                                                             [(fb(0.0), fb(0.0)), (fb(-10.0), fb(30.0))]):
        if sse2 and (hgt is not hs[0] or limits[0] != fb(0.0)):
            continue
        c = dict(precision=pc, sse2=sse2, append=app, height=hgt, limits=list(limits), **grid)
        rows.append(dict(input=c, output=rects(c)))
    return rows


# --------------------------------------------------------------- 43DA00
def deferred(c):
    m = M(c['precision'])
    vworld = m.alloc(0x300)
    m.labels = {}
    m.u.mem_write(vworld + 0xae, bytes([c['uninit_ae']]))
    views, nodes = [], []
    answers = {}

    def vbc(mm):
        v = mm.ecx()
        name = m.labels[v]
        m.trace.append(['vtBC', name])
        mm.ret(0, answers[name])
    vt = m.alloc(0x100)
    stub = m.stub_next
    m.stub_next += 16
    m.u.mem_write(stub, b'\xc3')
    m.seams[stub] = vbc
    m.w32(vt + 0xbc, stub)
    for i, v in enumerate(c['views']):
        p = m.alloc(0x200)
        m.w32(p, vt)
        m.w32(p + 0x54, v['flags'])
        m.w32(p + 0x19c, v['count'])
        name = f'v{i}'
        m.labels[p] = name
        answers[name] = v['vbc']
        views.append(p)
    prev_link = vworld + 0x98
    for i, p in enumerate(views):
        n = m.alloc(12)
        m.labels[n] = f'n{i}'
        nodes.append(n)
    for i, n in enumerate(nodes):
        m.w32(n, views[i], prev_link, nodes[i + 1] if i + 1 < len(nodes) else 0)
        prev_link = n + 8
    m.w32(vworld + 0x98, nodes[0] if nodes else 0)

    def destroy(mm):
        assert mm.ecx() == vworld
        m.trace.append(['43CF00', m.labels[mm.arg(0)]])
        mm.ret(4)
    m.seams[0x43cf00] = destroy
    m.call(0x43da00, vworld, [])
    remaining, link, p = [], vworld + 0x98, m.r32(vworld + 0x98)
    while p:
        assert m.r32(p + 4) == link, 'pprev link'
        remaining.append(m.labels[m.r32(p)])
        link, p = p + 8, m.r32(p + 8)
    return dict(remaining=remaining, trace=m.trace)


def deferred_cases():
    rows = []
    kinds = [dict(flags=0x40000, vbc=1, count=1), dict(flags=0x80000, vbc=1, count=3), dict(flags=0xc0000, vbc=0, count=1),
             dict(flags=0x40000, vbc=1, count=0), dict(flags=0x20000000, vbc=1, count=1), dict(flags=0x80000, vbc=1, count=-1 & 0xffffffff)]
    lists = [[], [0], [3], [0, 1], [2, 0, 4], [0, 5, 1, 3], [1, 1, 2, 2, 0]]
    for pc, ae, lst in itertools.product(PCS, [0, 1], lists):
        c = dict(precision=pc, uninit_ae=ae, views=[kinds[k] for k in lst])
        rows.append(dict(input=c, output=deferred(c)))
    return rows


# --------------------------------------------------------------- 43FF60
def update(c):
    """One 43FF60 call with every child seamed except 41C0E0/41C110(5)."""
    m = M(c['precision'])
    m.alloc_null = c['alloc_null']
    vworld, world, gmap, camera, cameras, ui, cursor, profiler = (
        m.alloc(0x300), m.alloc(0x300), m.alloc(0x40), m.alloc(0x40), m.alloc(8), m.alloc(0x80), m.alloc(0x10), m.alloc(0x100))
    state13c, minimap = m.alloc(0x40), m.alloc(0x88)
    m.labels = {vworld: 'vworld', camera: 'camera', state13c: '13c', minimap: 'minimap'}
    m.w32(0xb36534, world)
    m.w32(world + 0x30, gmap)
    m.w32(gmap + 0xc, c['extent'][0], c['extent'][1])
    m.w32(cameras, camera)
    m.w32(0xb40048, cameras)
    m.w32(0xb3653c, ui)
    m.w32(ui + 0x58, cursor)
    m.w32(cursor + 4, 17, 23)
    m.w32(0xb39bb8, profiler)
    visual = m.alloc(0x100) if c['visual_map'] else 0
    if visual:
        m.labels[visual] = 'visual'
    m.w32(0xb36518, visual)
    m.u.mem_write(vworld + 0xad, bytes([c['ad']]))
    m.w32(vworld + 0x13c, state13c)
    m.w32(vworld + 0x140, minimap if c['minimap'] else 0)
    m.w32(vworld + 0x174, c['look_at_174'])
    m.w32(vworld + 0x178, 0x1781781, 0x1781782, 0x1781783)
    m.w32(vworld + 0x184, 0x1841841, 0x1841842, 0x1841843)
    m.w32(0xb39f70, c['guard_b39f70'])
    before_profiler = bytes(m.u.mem_read(profiler, 0x100))
    draws = list(c['rng'])

    def name(v):
        return m.labels.get(v, hex(v))

    def simple(label, pop=0, value=None, check=None):
        def seam(mm):
            entry = [label, name(mm.ecx())] + [mm.arg(i) for i in range(pop // 4)]
            if check:
                check(mm)
            m.trace.append(entry)
            mm.ret(pop, value(mm) if callable(value) else value)
        return seam

    def construct(mm):
        p = mm.ecx()
        m.labels[p] = 'new140'
        m.trace.append(['4341A0', 'new140'])
        mm.ret(0, p)

    def rng(mm):
        assert mm.arg(0) == 0xb39714
        value = draws.pop(0)
        m.trace.append(['85D840', value])
        mm.ret(0, value)

    def look_at(mm):
        m.trace.append(['6FF130', name(mm.ecx()), mm.arg(0), mm.arg(1)])
        mm.ret(8)

    def project(mm):
        out, pixel, normal = mm.arg(0), mm.arg(1), mm.arg(2)
        assert pixel == cursor + 4 and normal == vworld + 0x178 and mm.ecx() == vworld
        m.w32(out, *c['point'])
        m.w32(normal, 0x11, 0x22, 0x33)
        m.trace.append(['43C4B0', name(mm.ecx())])
        mm.ret(12, out)
    m.seams.update({
        0x5eec70: simple('5EEC70'),
        0x4341a0: construct,
        0x59aa70: simple('59AA70'),
        0x59ab60: simple('59AB60'),
        0x96cab0: lambda mm: (m.trace.append(['96CAB0', hex(mm.arg(0))]), mm.ret(0, 0)),
        0x59aba0: simple('59ABA0', 0, c['milliseconds']),
        0x85d840: rng,
        0x6ff130: look_at,
        0x701590: simple('701590'),
        0x43da00: simple('43DA00'),
        0x43c4b0: project,
        0x43ec60: simple('43EC60'),
        0x43a1c0: simple('43A1C0'),
        0x71fac0: simple('71FAC0'),
        0x4361b0: simple('4361B0'),
    })
    m.call(0x43ff60, vworld, [])
    assert m.fpu_top() == 0
    assert bytes(m.u.mem_read(profiler, 0x100)) == before_profiler, '41C0E0/41C110(5) stored'
    p140 = m.r32(vworld + 0x140)
    return dict(trace=m.trace, field_140=name(p140) if p140 else 0, guard_b39f70=m.r32(0xb39f70),
                normal_178=m.r32(vworld + 0x178, 3), cursor_184=m.r32(vworld + 0x184, 3), rng_left=len(draws))


def update_cases():
    rows = []
    base = dict(precision='Double53', ad=1, minimap=1, alloc_null=0, look_at_174=0, guard_b39f70=0, milliseconds=0,
                rng=[0, 0], extent=[fb(512.0), fb(384.0)], visual_map=1, point=[fb(1.5), fb(2.5), fb(3.5)])
    variants = [dict(ad=0), dict(), dict(minimap=0), dict(minimap=0, alloc_null=1), dict(visual_map=0)]
    for a174 in [fb(-1.0), 0x80000000, fb(1e-6)]:
        variants.append(dict(look_at_174=a174))
    for pc in PCS:
        for a174, ms, guard in itertools.product([fb(2.0), fb(0.4), fb(4294967.5)], [0, 399, 400, 401, 2000, 0xffffffff, 0x80000000], [0, 1]):
            for draws in ([0, 0], [0xffffffff, 0x80000000], [12345678, 0x7fffffff], [0x9e3779b9, 0x3c6ef372]):
                variants.append(dict(precision=pc, look_at_174=a174, milliseconds=ms, guard_b39f70=guard, rng=draws,
                                     extent=[fb(511.7), fb(1023.3)]))
    for v in variants:
        c = dict(base)
        c.update(v)
        rows.append(dict(input=c, output=update(c)))
    return rows


# --------------------------------------------------------------- 43EC60
def world_view(c):
    m = M(c['precision'])
    vworld, world, manager = m.alloc(0x300), m.alloc(0x300), m.alloc(0x20)
    objects = {n: m.alloc(0x80) for n in ['108', 'bc', '94', 'visual']}
    m.labels = {vworld: 'vworld', manager: 'manager', **{v: k for k, v in objects.items()}}
    m.w32(0xb39f64, c['latch'])
    m.w32(0xb36534, world)
    m.w32(world + 0xd8, c['world_d8'])
    m.w32(world + 0x130, manager)
    m.w32(0xb3cc94, c['b3cc94'])
    m.w32(vworld + 0xb8, c['vworld_b8'])
    m.u.mem_write(vworld + 0xad, bytes([c['ad']]))
    m.w32(vworld + 0x108, objects['108'])
    m.w32(vworld + 0xbc, objects['bc'])
    m.w32(vworld + 0x94, objects['94'])
    m.w32(0xb36518, objects['visual'])
    # +208 request list: {view, next}; view vt+74 readiness.
    ready = {}

    def vf74(mm):
        name = m.labels[mm.ecx()]
        m.trace.append(['vt74', name])
        mm.ret(0, ready[name])
    vt = m.alloc(0x80)
    stub = m.stub_next
    m.stub_next += 16
    m.seams[stub] = vf74
    m.w32(vt + 0x74, stub)
    link = vworld + 0x208
    for i, r in enumerate(c['requests']):
        v, n = m.alloc(0x40), m.alloc(8)
        m.w32(v, vt)
        m.labels[v], m.labels[n] = f'r{i}', f'q{i}'
        ready[f'r{i}'] = r
        m.w32(link, n)
        m.w32(n, v, 0)
        m.w32(vworld + 0x20c, n)
        link = n + 4
    # +E0/+E4/+E8 FX command list linked by +68.
    commands = []
    for i, r in enumerate(c['commands']):
        node = m.alloc(0x80)
        cmd = m.alloc(0x20)
        m.w32(node + 4, cmd)
        m.labels[node], m.labels[cmd], m.labels[node + 0x5c] = f'f{i}', f'c{i}', f'f{i}+5c'
        commands.append((node, r))
    for i, (node, _) in enumerate(commands):
        m.w32(node + 0x68, commands[i + 1][0] if i + 1 < len(commands) else 0)
    m.w32(vworld + 0xe0, commands[0][0] if commands else 0, commands[-1][0] if commands else 0, len(commands))
    fx_answers = {m.labels[n]: r for n, r in commands}

    def name(v):
        return m.labels.get(v, hex(v))

    def simple(label, pop=0, value=0):
        def seam(mm):
            m.trace.append([label, name(mm.ecx())] + [name(mm.arg(i)) for i in range(pop // 4)])
            mm.ret(pop, value)
        return seam

    def assertion(mm):
        m.trace.append(['855D30', m.read_text(mm.arg(0)), m.read_text(mm.arg(1)), mm.arg(2)])
        mm.ret(0, c['assert_result'])

    def fx(mm):
        node = mm.arg(0)
        assert m.labels[mm.ecx()] == 'c' + m.labels[node][1:]
        m.trace.append(['4289A0', m.labels[node]])
        mm.ret(4, fx_answers[m.labels[node]])
    m.read_text = lambda p: bytes(m.u.mem_read(p, 64)).decode('utf-16le').split('\0')[0]
    m.seams.update({
        0x855d30: assertion,
        0x50c750: simple('50C750'),
        0x43a400: simple('43A400', 0, 1),
        0x4318e0: simple('4318E0'),
        0x4312f0: simple('4312F0'),
        0x725340: simple('725340'),
        0x45b720: simple('45B720'),
        0x4289a0: fx,
        0x429050: simple('429050'),
        0x45c7a0: simple('45C7A0'),
    })
    m.call(0x43ec60, vworld, [])
    assert m.fpu_top() == 0
    left, p = [], m.r32(vworld + 0x208)
    while p:
        left.append(m.labels[m.r32(p)])
        p = m.r32(p + 4)
    fx_left, p = [], m.r32(vworld + 0xe0)
    for _ in range(m.r32(vworld + 0xe8)):
        fx_left.append(m.labels[p])
        p = m.r32(p + 0x68)
    tail = m.r32(vworld + 0xe4)
    return dict(trace=m.trace, latch=m.r32(0xb39f64) & 0xff, requests=left, fx=fx_left, fx_count=m.r32(vworld + 0xe8),
                fx_tail=m.labels.get(tail, 0 if tail == 0 else hex(tail)))


def world_view_cases():
    rows = []
    base = dict(precision='Double53', latch=0, world_d8=3, b3cc94=7, vworld_b8=7, ad=1, assert_result=0, requests=[], commands=[])
    variants = [dict(), dict(latch=1, ad=0), dict(ad=0), dict(vworld_b8=8), dict(world_d8=6), dict(world_d8=6, assert_result=1),
                dict(requests=[1, 1, 0, 1]), dict(requests=[0, 1]), dict(requests=[1]),
                dict(commands=[0]), dict(commands=[1]), dict(commands=[0, 1, 0]), dict(commands=[1, 1, 0, 0, 1]),
                dict(requests=[1, 0], commands=[1, 0])]
    for pc in PCS:
        for v in variants:
            c = dict(base, precision=pc)
            c.update(v)
            rows.append(dict(input=c, output=world_view(c)))
    return rows


SECTIONS = {
    'view_time_43ab30': view_time_cases,
    'sim_time_76b120': sim_time_cases,
    'messages_50c750': message_cases,
    'rects_4318e0': rect_cases,
    'deferred_43da00': deferred_cases,
    'update_43ff60': update_cases,
    'world_view_43ec60': world_view_cases,
}


def main():
    out = dict(schema=1, scope=__doc__, original_sha256=h.ORIGINAL_SHA, v14_sha256=h.V14_SHA)
    counts = {}
    for name, producer in SECTIONS.items():
        rows = producer()
        out[name] = rows
        counts[name] = len(rows)
    out['instructions'] = [dict(va=hex(a), bytes=b) for a, b in sorted(h.VISITED.items())]
    path = HERE / 'native-vworld-update.json'
    text = json.dumps(out, indent=1) + '\n'
    if '--write' in sys.argv:
        path.write_text(text)
    else:
        assert json.loads(path.read_text()) == json.loads(text), 'native replay differs from the fixture'
    print(json.dumps(dict(sections=counts, instructions=len(h.VISITED),
                          sha256=hashlib.sha256(path.read_bytes()).hexdigest())))


if __name__ == '__main__':
    main()
