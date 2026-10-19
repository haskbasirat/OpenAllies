"""Producer of native-terrain-map.json: the ORIGINAL TerrainMap constructors.

Runs, in both pinned images (original AA.exe and the v14 runtime, via
research/phase8-ai/native_machine.py) and at PC24/PC53/PC64, the original
code that produces every TerrainMap/TerrainBlock field 0x71D840 reads:

  721330  TerrainMap ctor (normals +04/+10/+1C, heightfield +6C, ...)
  723E00  TerrainMap build: 7241CE..724455 dimensions (CRT ceil 96D4C0 SSE2
          path, _ftol 96CC4C, 855D30 assertions), 72464B..7246A9 block loop
          -> 723D30 -> 723960 block ctor (+736E20 water holder, 71AB60,
          71AFD0, 71B8F0 block min/max). The Ni scene tail 7246AB..7252AF
          (props/cloud/detail/macro; static scan: stores only map +34..+68,
          no map pointer escape, no B36518 user in its direct-call closure)
          is skipped by one explicit seam; 7252AF (+BC = 1) then executes.
  720AD0  block mesh load (reached from 42FE10 -> 7212D0 when +44 == 0):
          720130 mesh allocation is the seam (a pooled NiTriShape with
          stride^2 vertex arrays); 71B610 / 4316A0 / 42D750 vertex fill and
          71A690 map fold run natively; the run stops at 720C79 (static scan:
          the remainder writes only block +124..+14E and Ni objects).
  721030  block mesh release (7211F0 when the resource count reaches 0).
  71A750  map min/max recompute (725340 at 725429 when +C0 is set).
  737470 + 738280  block water rebuild (7208D0 at 720A93 when +14F); the
          7377F0 water geometry build is a seam. Only the +9C flag is used.
  71D840  the ray query itself, on the natively built memory (end-to-end).

Every other callee is an explicit seam (Ni/GPU objects, textures, strings,
resource registration, profiling timers, 4F0E50 heights for the 71AFD0
bounding sphere, which 71D840 never reads). Laboratory heightfields (VWorld+BC
layout: +04 heights, +10 step, +14 inverse, +18 columns, +1C rows, +20/+24
limits) and GMap tiles are explicit oracle inputs, not mission values.

  python native_terrain_map.py --write   # create (refuses to overwrite)
  python native_terrain_map.py           # replay: byte-identical check
  python native_terrain_map.py --dry     # print coverage only
"""
from pathlib import Path
import hashlib, json, math, random, struct, sys
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'research/phase8-ai'))
import native_machine as nm  # noqa: E402
from native_machine import Machine, f32, original, modified  # noqa: E402
from unicorn.x86_const import UC_X86_REG_EIP, UC_X86_REG_ESP, UC_X86_REG_ECX, UC_X86_REG_FPCW, UC_X86_REG_FPSW, UC_X86_REG_MXCSR  # noqa: E402
from unicorn import UC_HOOK_MEM_WRITE  # noqa: E402

OUT = HERE / 'native-terrain-map.json'
GWORLD, VWORLD, MAP_GLOBAL, B36530 = 0xB36534, 0xB36538, 0xB36518, 0xB36530
AVARS, UVARS, SSE2_FLAG = 0xB39990, 0xB39940, 0xB4B340
FLT_MAX, NEG_FLT_MAX = 0x7f7fffff, 0xff7fffff
RANGES = {'721330': (0x721330, 0x7218a8), '723E00': (0x723e00, 0x7246ab), '723E00-tail-BC': (0x7252af, 0x7252b6),
          '723D30': (0x723d30, 0x723df4), '723960': (0x723960, 0x723d05), '736E20': (0x736e20, 0x736e3d),
          '71AB60': (0x71ab60, 0x71abc6), '71AFD0': (0x71afd0, 0x71b3f5), '71B8F0': (0x71b8f0, 0x71b9a3),
          '720AD0-71A690': (0x720ad0, 0x720c79), '71B610': (0x71b610, 0x71b8eb), '4316A0': (0x4316a0, 0x4317c9),
          '71A690': (0x71a690, 0x71a748), '71A750': (0x71a750, 0x71a946), '721030': (0x721030, 0x721179),
          '737470': (0x737470, 0x7374ba), '738280': (0x738280, 0x7385b6), '738200': (0x738200, 0x738272),
          '96D4C0-ceil': (0x96d4c0, 0x96d5c1), '96CC4C-ftol': (0x96cc4c, 0x96cc74)}


class Seams:
    """Explicit replacements for callees that do not produce 71D840 fields."""

    def __init__(self, m):
        self.m = m
        self.calls = {}
        self.asserts = []
        self.vt_null = None

    def count(self, name):
        self.calls[name] = self.calls.get(name, 0) + 1

    def fake(self, size=0x200, refcount=0x1000):
        m = self.m
        p = m.alloc(size)
        m.w(p, self.vtable())
        m.w(p + 4, refcount)
        return p

    def vtable(self):
        """A vtable whose every slot is a recording stub for one pop size."""
        if self.vt_null is None:
            m = self.m
            vt = m.alloc(0x200)
            pops = {0x00: 4, 0x04: 4, 0x0c: 0, 0x10: 8, 0x14: 4, 0x18: 0, 0x2c: 4}
            for slot in range(0, 0x200, 4):
                pop = pops.get(slot)
                if pop is None:
                    def fail(m, slot=slot):
                        raise AssertionError('unmodelled virtual slot %#x' % slot)
                    m.w(vt + slot, m.stub(fail))
                else:
                    def stub(m, slot=slot, pop=pop):
                        self.count('vt+%#x' % slot)
                        m.ret(self.string_header() if slot == 0x0c else 0, pop)
                    m.w(vt + slot, m.stub(stub))
            self.vt_null = vt
        return self.vt_null

    def string_header(self):
        m = self.m
        h = m.alloc(0x40)
        m.w(h, self.fake(0x20))      # string manager (released through [[h]]+4)
        m.w(h + 0xc, 0x1000)         # reference count (never reaches zero)
        return h

    def string(self):
        return self.string_header() + 0x10

    def install(self):
        m = self.m
        S = self

        def ret(value, pop):
            return lambda m: m.ret(value(m) if callable(value) else value, pop)

        def named(name, fn):
            def run(m):
                S.count(name)
                fn(m)
            return run

        def new(m):
            m.ret(m.alloc(max(m.arg(1), 4)), 0)
        m.seam(0x6F7810, named('6F7810 new', new))
        m.seam(0x6F7430, named('6F7430 delete', ret(0, 0)))

        def eh_vector_ctor(m):
            ptr, size, count, ctor = m.arg(1), m.arg(2), m.arg(3), m.arg(4)
            for k in range(count):
                obj = ptr + k * size
                if ctor == 0x4DD140:
                    m.w(obj, 0)
                elif ctor == 0x71AAF0:     # 45C6E0 zero +4/+8, then vtable
                    m.w(obj, S.vtable())
                    m.w(obj + 4, 0)
                    m.w(obj + 8, 0)
                elif ctor == 0x736E00:     # water body record: +0 0, +4 list
                    for o in range(0, size, 4):
                        m.w(obj + o, 0)
                else:
                    raise AssertionError('eh vector ctor %#x' % ctor)
            m.ret(0, 0x14)
        m.seam(0x96D18D, named('96D18D eh vector ctor', eh_vector_ctor))
        for a in (0x72B160, 0x72B040, 0x728DF0, 0x86B240, 0x86CCC0):
            m.seam(a, named('%X ctor' % a, lambda m: m.ret(m.ecx(), 0)))
        m.seam(0x4B6AA0, named('4B6AA0', lambda m: m.ret(S.fake(), 4)))
        m.seam(0x4B2E60, named('4B2E60 var', lambda m: m.ret(S.fake(), 8)))

        def named_var(m):
            slot = m.alloc(4)
            m.w(slot, S.fake())
            m.ret(slot, 4)
        m.seam(0x405370, named('405370 var', named_var))

        def zone_manager(m):
            z = S.fake(0x100)
            m.w(z + 4, m.alloc(64 * 64 * 4))
            m.w(z + 0x2c, 64)
            m.ret(z, 4)
        m.seam(0x72EC90, named('72EC90 zone manager', zone_manager))

        def profile(m):                    # 40E970 profile scope (array + timer)
            p = m.ecx()
            m.w(p, m.alloc(5 * 8))
            m.w(p + 4, 0)
            m.w(p + 8, 5)
            m.w(p + 0xc, S.string())
            m.wb(p + 0x18, 1)
            m.wb(p + 0x19, 0)
            m.ret(p, 0)
        m.seam(0x40E970, named('40E970 profile', profile))
        m.seam(0x59AB60, named('59AB60 timer', lambda m: m.ret(m.ecx(), 0)))
        m.seam(0x59ABA0, named('59ABA0 timer', ret(0, 0)))
        m.seam(0x59AAD0, named('59AAD0 timer', lambda m: m.ret_st0_f32(0, 0)))

        def string_out(m):
            m.w(m.arg(1), S.string())
            m.ret(m.arg(1), 8)
        m.seam(0x410CB0, named('410CB0 string', string_out))

        def string_this(m):
            m.w(m.ecx(), S.string())
            m.ret(m.ecx(), 4)
        m.seam(0x40F9D0, named('40F9D0 string', string_this))
        m.seam(0x5A77E0, named('5A77E0 texture', lambda m: m.ret(S.fake(), 0)))
        m.seam(0x4BDB70, named('4BDB70', ret(0, 4)))

        def smart_out(m):
            m.w(m.arg(1), S.fake())
            m.ret(m.arg(1), 4)
        m.seam(0x5A63D0, named('5A63D0', smart_out))
        for a, pop in ((0x709B50, 4), (0x71AE00, 4), (0x43B5B0, 4), (0x72D320, 0), (0x5ACEC0, 4), (0x5AA270, 8),
                       (0x8704C0, 0x10), (0x89F5F0, 8), (0x43C3B0, 4), (0x71FD20, 0), (0x71FB20, 0), (0x86E8D0, 8),
                       (0x8677A0, 4), (0x86F090, 0), (0x720330, 4), (0x7377F0, 4)):
            m.seam(a, named('%X' % a, ret(0, pop)))
        m.seam(0x867690, named('867690 NiNode', lambda m: m.ret(m.ecx(), 4)))
        m.seam(0x6FF7D0, named('6FF7D0', ret(0, 0)))

        def smart_null(m):
            m.w(m.arg(1), 0)
            m.ret(m.arg(1), 0)
        m.seam(0x6FF850, named('6FF850', smart_null))

        def assertion(m):
            S.asserts.append(m.arg(3))
            m.ret(0, 0)
        m.seam(0x855D30, named('855D30 assert', assertion))

        def string_manager(m):
            m.ret(S.fake(), 0)
        m.seam(0x856370, named('856370 string manager', string_manager))
        m.seam(0x405D60, named('405D60 format', ret(0, 0)))

        def copy_string(m):
            m.w(m.ecx(), m.arg(1))
            m.ret(m.ecx(), 4)
        m.seam(0x401B00, named('401B00 string copy', copy_string))
        m.seam(0x5AA890, named('5AA890 resource ctor', lambda m: m.ret(m.ecx(), 4)))
        m.seam(0x4F0E50, named('4F0E50 height', lambda m: m.ret_st0_f32(0, 8)))

        def mesh(m):                       # 720130: pooled NiTriShape, stride^2 vertices
            terrain = m.ecx()
            stride = m.r(terrain + 0xa4)
            quads = m.r(terrain + 0xa8) & 0xffff
            n = stride * stride
            ni = S.fake(0x100, refcount=0x100)
            data = m.alloc(0x60)
            m.w(ni + 0xcc, data)
            m.u.mem_write(data + 0xc, struct.pack('<H', n & 0xffff))
            for off, size in ((0x20, 12), (0x24, 12), (0x28, 16), (0x2c, 8)):
                m.w(data + off, m.alloc(n * size))
            m.w(data + 0x44, m.alloc(quads * quads * 12))
            m.w(m.arg(1), ni)
            m.ret(m.arg(1), 4)
        m.seam(0x720130, named('720130 mesh', mesh))


def bits(x):
    return f32(x)


def map_fields(m, t):
    cells = m.r(t + 0xb8)
    bx, by = m.r(t + 0x84), m.r(t + 0x88)
    index = {}
    if cells:
        for i in range(bx * by):
            index[m.r(cells + 4 * i)] = i

    def holder(v):
        return index.get(v, None if v == 0 else 'foreign')
    words = {o: m.r(t + o) for o in (0x04, 0x08, 0x0c, 0x10, 0x14, 0x18, 0x1c, 0x20, 0x24, 0x74, 0x78, 0x7c, 0x80, 0x84,
                                      0x88, 0x8c, 0x90, 0xa0, 0xa4, 0xa8, 0xac, 0xb0, 0xb4)}
    return dict(up=[words[4], words[8], words[0xc]], x_normal=[words[0x10], words[0x14], words[0x18]],
                y_normal=[words[0x1c], words[0x20], words[0x24]], width=words[0x74], height=words[0x78],
                quads_x=words[0x7c], quads_y=words[0x80], blocks=[words[0x84], words[0x88]], min=words[0x8c], max=words[0x90],
                dirty=m.rb(t + 0x94), max_holder=holder(m.r(t + 0x98)), min_holder=holder(m.r(t + 0x9c)),
                block_size=words[0xa0], stride=words[0xa4], quads=words[0xa8], quad_size=words[0xac],
                gtile=words[0xb0], quads_per_gtile=words[0xb4], loaded=m.rb(t + 0xbc), recompute=m.rb(t + 0xc0))


def block_fields(m, t, cell, vertices=False):
    out = dict(owner=m.r(cell + 0x40) == t, mesh=m.r(cell + 0x44) != 0, min=m.r(cell + 0x80), max=m.r(cell + 0x84),
               water=m.rb(cell + 0x9c), water_holder=m.r(cell + 0x90) != 0, index=[m.r(cell + 0xac), m.r(cell + 0xb0)],
               stride=m.r(cell + 0xc8), quads=m.r(cell + 0xcc), gtiles=m.r(cell + 0xd0), quad_size=m.r(cell + 0xd4),
               water_dirty=m.rb(cell + 0x14f))
    if vertices and out['mesh']:
        data = m.r(m.r(cell + 0x44) + 0xcc)
        v = m.r(data + 0x20)
        n = out['stride'] * out['stride']
        out['vertices'] = [m.r(v + 4 * i) for i in range(3 * n)]
    return out


def state(m, t, loaded=()):
    bx, by = m.r(t + 0x84), m.r(t + 0x88)
    cells = m.r(t + 0xb8)
    return dict(map=map_fields(m, t), blocks=[block_fields(m, t, m.r(cells + 4 * i), i in loaded) for i in range(bx * by)])


class Lab:
    """One laboratory World/VWorld: native memory plus the run's records."""

    def __init__(self, precision, spec):
        self.spec = spec
        m = self.m = Machine(precision)
        self.seams = Seams(m)
        self.seams.install()
        m.w(SSE2_FLAG, 1)                   # every SSE2 CPU: CRT ceil 96D500 path
        m.u.reg_write(UC_X86_REG_MXCSR, 0x1f80)  # Windows thread default (all masked)
        gw = m.alloc(0x300)
        m.w(GWORLD, gw)
        m.w(gw + 0x3c, spec['extents'][0])
        m.w(gw + 0x40, spec['extents'][1])
        gmap = m.alloc(0x100)
        m.w(gw + 0x30, gmap)
        m.w(gmap + 8, spec['gtile'])
        cfg = self.cfg = m.alloc(0x100)
        m.w(gmap + 0x24, cfg)
        m.w(cfg + 0x7c, m.array([self.seams.fake()]))   # 721600: [cfg+7C][i] for i < cfg+80
        m.w(cfg + 0x80, 1)
        tiles = spec.get('tiles')
        if tiles is not None:
            self.install_tiles(gmap, tiles)
        holder = m.alloc(0x20)
        m.w(holder + 0xc, self.seams.fake())
        m.w(B36530, holder)
        av, uv = m.alloc(0x200), m.alloc(0x200)
        m.w(AVARS, av)
        m.w(UVARS, uv)
        m.w(av, m.array([spec['block'] & 0xffffffff]))
        m.w(uv, m.array([spec['quad'] & 0xffffffff]))
        hf = self.hf = m.alloc(0x40)
        h = spec['heightfield']
        m.w(hf + 4, m.array(h['heights']))
        m.w(hf + 0x10, h['step'])
        m.w(hf + 0x14, h['inverse'])
        m.w(hf + 0x18, h['columns'])
        m.w(hf + 0x1c, h['rows'])
        m.w(hf + 0x20, h['limits'][0])
        m.w(hf + 0x24, h['limits'][1])
        vworld = m.alloc(0x200)
        m.w(VWORLD, vworld)
        m.w(vworld + 0xbc, hf)
        self.map = None

    def install_tiles(self, gmap, tiles):
        m = self.m
        w, hgt = tiles['size']
        arr = m.alloc(0x30 * w * hgt)
        kinds = {}
        for k in sorted(set(c for c in tiles['classes'] if c is not None)):
            data = m.alloc(0x40)
            m.w(data + 0x1c, k & 0xffffffff)
            var = m.alloc(0x40)
            m.w(var + 0x1c, data)
            kinds[k] = var
        for i, c in enumerate(tiles['classes']):
            m.w(arr + 0x30 * i, 0 if c is None else kinds[c])
        m.w(gmap + 0x1c, w)
        m.w(gmap + 0x20, hgt)
        m.w(gmap + 0x28, arr)

    def call_until(self, entry, ecx, args, until):
        """Like Machine.call but stops at an address inside the callee."""
        m, u = self.m, self.m.u
        u.reg_write(UC_X86_REG_FPCW, nm.PRECISIONS[m.precision])
        u.reg_write(UC_X86_REG_FPSW, 0)
        sp = nm.STACK - 4 * len(args)
        for i, v in enumerate(args):
            m.w(sp + 4 * i, v)
        sp -= 4
        m.w(sp, nm.STOP)
        u.reg_write(UC_X86_REG_ESP, sp)
        u.reg_write(UC_X86_REG_ECX, ecx)
        u.emu_start(entry, until, count=5_000_000)
        assert not m.null_accesses, [tuple(map(hex, a)) for a in m.null_accesses]
        assert u.reg_read(UC_X86_REG_EIP) == until, hex(u.reg_read(UC_X86_REG_EIP))

    def construct(self):
        m, spec = self.m, self.spec
        t = self.map = m.alloc(0x158)
        quad, block = spec['quad'], spec['block']
        clamped = block if quad > block else quad          # 43FD0F..43FD13
        assert m.call(0x721330, ecx=t, args=(self.hf, self.cfg, clamped & 0xffffffff, block & 0xffffffff), pop=0x10) == t
        after_721330 = map_fields(m, t)
        m.w(MAP_GLOBAL, t)                                 # 43FE21
        m.seam(0x7246AB, lambda m: m.u.reg_write(UC_X86_REG_EIP, 0x7252AF))
        self.call_until(0x723E00, t, (), 0x7252B6)
        return dict(after_721330=dict(min=after_721330['min'], max=after_721330['max'], up=after_721330['up'],
                                      x_normal=after_721330['x_normal'], y_normal=after_721330['y_normal'],
                                      block_size=after_721330['block_size'], quad_size=after_721330['quad_size'],
                                      loaded=after_721330['loaded'], recompute=after_721330['recompute']),
                    asserts=list(self.seams.asserts), state=state(m, t))

    def cell(self, i):
        return self.m.r(self.m.r(self.map + 0xb8) + 4 * i)

    def op(self, op):
        m, t = self.m, self.map
        kind = op['op']
        if kind == 'load':
            cell = self.cell(op['cell'])
            assert m.r(cell + 0x44) == 0      # 7212D0 reaches 720AD0 only without a mesh
            self.call_until(0x720AD0, cell, (), 0x720C79)
            return state(m, t, loaded=(op['cell'],))
        if kind == 'release':
            cell = self.cell(op['cell'])
            m.call(0x721030, ecx=cell, args=(0,), pop=4)
            return state(m, t)
        if kind == 'update':                  # 725340 at 725429..725438
            if m.rb(t + 0xc0):
                m.call(0x71A750, ecx=t, pop=0)
                m.wb(t + 0xc0, 0)
            return state(m, t)
        if kind == 'water':                   # 7208D0 at 720A93 when +14F
            cell = self.cell(op['cell'])
            if m.rb(cell + 0x14f):
                m.wb(cell + 0x14f, 0)
                m.call(0x737470, ecx=cell + 0x8c, pop=0)
                m.call(0x738280, ecx=cell + 0x8c, pop=0)
            return state(m, t)
        if kind == 'invalidate':              # 71FFA0: +14F = 1
            m.wb(self.cell(op['cell']) + 0x14f, 1)
            return state(m, t)
        if kind == 'tiles':                   # lab: GMap tile class rewrite
            gmap = m.r(m.r(GWORLD) + 0x30)
            arr = m.r(gmap + 0x28)
            for i, c in op['set']:
                var = m.r(arr + 0x30 * i)
                data = m.alloc(0x40)
                m.w(data + 0x1c, c)
                nvar = m.alloc(0x40)
                m.w(nvar + 0x1c, data)
                m.w(arr + 0x30 * i, nvar)
            return state(m, t)
        if kind == 'heights':                 # lab: 431390-style sample rewrite
            hp = m.r(self.hf + 4)
            for i, v in op['set']:
                m.w(hp + 4 * i, v)
            return state(m, t)
        raise ValueError(kind)

    def ray(self, eye, direction, secondary, margin):
        m, t = self.m, self.map
        eye_ptr = m.array(eye)
        out = m.array([0xcdcdcdcd] * 3)
        sec = m.array([0xabababab] * 3) if secondary else 0
        writes = []

        def write(u, access, address, size, value, data):
            if out <= address < out + 12:
                writes.append(['point', address - out, value & 0xffffffff])
            if secondary and sec <= address < sec + 12:
                writes.append(['secondary', address - sec, value & 0xffffffff])
        hook = m.u.hook_add(UC_HOOK_MEM_WRITE, write)
        try:
            result = m.call(0x71D840, ecx=t, args=[eye_ptr, *direction, out, sec, margin & 0xffffffff], pop=0x1c, count=20_000_000)
        finally:
            m.u.hook_del(hook)
        return dict(result=result & 0xff, out=[m.r(out + 4 * i) for i in range(3)],
                    secondary=[m.r(sec + 4 * i) for i in range(3)] if secondary else None, writes=writes)


# --- laboratory inputs -------------------------------------------------------
def heightfield(rng, columns, rows, step, kind, extra=0):
    if kind == 'eighths':
        hs = [bits(rng.randrange(-16, 40) / 8) for _ in range(columns * rows)]
    elif kind == 'float':
        hs = [bits(struct.unpack('<f', struct.pack('<f', rng.uniform(-3, 5)))[0]) for _ in range(columns * rows)]
    elif kind == 'zeros':      # signed zeros and ties: native fold order decides the stored bits
        hs = [rng.choice([0, 0x80000000, bits(0.5), bits(-0.5)]) for _ in range(columns * rows)]
    elif kind == 'ridge':
        hs = [bits(round(8 * (2.5 * math.sin((i % columns) / 3.0) + 1.5 * math.cos((i // columns) / 2.0))) / 8)
              for i in range(columns * rows)]
    elif kind == 'flat':
        hs = [bits(1.25)] * (columns * rows)
    else:
        raise ValueError(kind)
    lo = min(hs, key=lambda b: struct.unpack('<f', struct.pack('<I', b))[0])
    hi = max(hs, key=lambda b: struct.unpack('<f', struct.pack('<I', b))[0])
    inv = bits(1.0 / struct.unpack('<f', struct.pack('<I', step))[0])
    return dict(columns=columns, rows=rows, step=step, inverse=inv, heights=hs, limits=[bits(-7.5), bits(9.25)])


def tiles_for(rng, w, h, water):
    classes = []
    for i in range(w * h):
        c = rng.choice([0, 0, 0, 1])
        classes.append(c)
    for i in water:
        classes[i] = 2
    return dict(size=[w, h], classes=classes)


def specs():
    rng = random.Random(0x723e00)
    out = {}

    def spec(name, extents, gtile, block, quad, columns, rows, step, kind, **extra):
        s = dict(extents=[bits(extents[0]), bits(extents[1])], gtile=bits(gtile), block=block, quad=quad,
                 heightfield=heightfield(rng, columns, rows, bits(step), kind))
        s.update(extra)
        out[name] = s
    # Retail shape: block 8, quad 2, gtile 2 (MapGTileResolutionBits 1), 16x16 map.
    spec('retail-16', [16.0, 16.0], 2.0, 8, 2, 9, 9, 2.0, 'eighths')
    spec('retail-32x24', [32.0, 24.0], 2.0, 8, 2, 17, 13, 2.0, 'ridge')
    spec('float-32', [32.0, 32.0], 4.0, 8, 2, 17, 17, 2.0, 'float')
    spec('zeros-24', [24.0, 24.0], 2.0, 8, 2, 13, 13, 2.0, 'zeros')
    spec('coarse-q4', [24.0, 24.0], 4.0, 8, 4, 7, 7, 4.0, 'eighths')
    spec('fine-q1', [16.0, 8.0], 2.0, 8, 1, 17, 9, 1.0, 'float')
    spec('block12', [24.0, 36.0], 2.0, 12, 2, 13, 19, 2.0, 'eighths')
    # Fractional extents (ceil) and a wider heightfield row than the map needs.
    spec('ceil-extents', [15.25, 15.75], 2.0, 8, 2, 11, 9, 2.0, 'eighths')
    # Lab-only fractional step: 71B610 x/y accumulation rounds per precision.
    spec('fraction-step', [16.0, 16.0], 2.0, 8, 2, 9, 9, 0.7, 'float')
    spec('big-coordinates', [16.0, 16.0], 2.0, 8, 2, 9, 9, 3355443.25, 'float')
    spec('flat', [16.0, 16.0], 2.0, 8, 2, 9, 9, 2.0, 'flat')
    # 3x3 blocks with the unique pit/peak in cells 8/7: 71A750's remainder loop.
    spec('recompute-tail', [24.0, 24.0], 2.0, 8, 2, 13, 13, 2.0, 'eighths')
    hs = out['recompute-tail']['heightfield']['heights']
    hs[11 * 13 + 10] = bits(-6.0)
    hs[10 * 13 + 6] = bits(7.5)
    # Water flag rule (738280) over GMap tiles: g = 8/2 / (gtile/quad).
    spec('water-tiles', [32.0, 32.0], 2.0, 8, 2, 17, 17, 2.0, 'eighths',
         tiles=dict(size=[16, 16], classes=[2 if i in (0, 37, 70, 255, 136) else (1 if i % 7 == 0 else 0) for i in range(256)]))
    spec('water-gtile4', [32.0, 32.0], 4.0, 8, 2, 17, 17, 2.0, 'eighths',
         tiles=dict(size=[8, 8], classes=[2 if i in (9, 27, 63) else 0 for i in range(64)]))
    # GMap wider than the block coverage: null and water tiles only as
    # neighbours (column/row 16); a narrower GMap whose centre reads wrap rows.
    edge = [None if (i % 17 == 16 or i // 17 == 16) and (i * 7) % 3 == 0 else (2 if i in (16, 33, 50, 152, 288, 280) else 0)
            for i in range(17 * 17)]
    spec('water-null-edge', [32.0, 32.0], 2.0, 8, 2, 17, 17, 2.0, 'eighths', tiles=dict(size=[17, 17], classes=edge))
    spec('water-wrap', [32.0, 32.0], 2.0, 8, 2, 17, 17, 2.0, 'eighths',
         tiles=dict(size=[8, 32], classes=[2 if i in (7, 100, 135) else 1 for i in range(256)]))
    # Assertion / division paths of 723E00 (Rust: explicit errors).
    spec('assert-width-quad', [15.0, 16.0], 2.0, 8, 2, 9, 9, 2.0, 'eighths')
    spec('assert-block-quad', [24.0, 24.0], 4.0, 6, 4, 7, 7, 4.0, 'eighths')
    spec('assert-quad-gtile', [16.0, 16.0], 1.0, 8, 2, 9, 9, 2.0, 'eighths')
    return out


def sequences(rng, name, s):
    """Operations applied after construction (cell indices row-major)."""
    blocks = [int(math.ceil(struct.unpack('<f', struct.pack('<I', s['extents'][i]))[0])) // s['block'] for i in range(2)]
    n = blocks[0] * blocks[1]
    order = list(range(n))
    rng.shuffle(order)
    ops = []
    if name.startswith('water'):
        for i in range(n):
            ops.append(dict(op='water', cell=i))
        # PaintTerrain-style class changes, 71FFA0 +14F, then 7208D0 again.
        w, h = s['tiles']['size']
        changed = sorted(rng.sample(range(w * h), 12))
        ops.append(dict(op='tiles', set=[[i, rng.choice([0, 1, 2, 2])] for i in changed]))
        for i in rng.sample(range(n), n // 2):
            ops.append(dict(op='invalidate', cell=i))
            ops.append(dict(op='water', cell=i))
        return ops
    keep = order[:max(1, (2 * n) // 3)]
    for i in keep:
        ops.append(dict(op='load', cell=i))
    ops.append(dict(op='update'))
    if n > 2:
        ops.append(dict(op='release', cell=keep[0]))
        ops.append(dict(op='load', cell=keep[0]))
    if name in ('retail-32x24', 'float-32', 'zeros-24', 'recompute-tail'):
        # 431390-style sample rewrite of the block holding the map minimum,
        # then its reload: 71A690 clears the holder, 725340 runs 71A750.
        for which in ('min', 'max'):
            ops.append(dict(op='heights', set='raise-min' if which == 'min' else 'lower-max'))
            ops.append(dict(op='release', cell=which + '-holder'))
            ops.append(dict(op='load', cell=which + '-holder'))
            ops.append(dict(op='update'))
    if name in ('retail-16', 'float-32', 'block12', 'flat', 'big-coordinates'):
        # Leave the rest absent: 71D840 then also takes 71CD30 for them.
        return ops
    for i in order[len(keep):]:
        ops.append(dict(op='load', cell=i))
    return ops


def rays(rng, s, st, n):
    """Rays from above (aimed at points inside the map), from outside, and
    grazing; eye/direction words are explicit binary32 inputs."""
    w, h = st['map']['width'], st['map']['height']
    lo = struct.unpack('<f', struct.pack('<I', st['map']['min']))[0]
    hi = struct.unpack('<f', struct.pack('<I', st['map']['max']))[0]
    out = []
    for k in range(n):
        kind = rng.random()
        theta = rng.uniform(0, 2 * math.pi)
        if rng.random() < 0.1:
            theta = rng.choice([0, math.pi / 2, math.pi, 3 * math.pi / 2, math.pi / 4])
        if kind < 0.65:
            target = [rng.uniform(0, w), rng.uniform(0, h), rng.uniform(lo, hi)]
            slope = rng.choice([0.15, 0.4, 0.8, 1.5, 3.0, 12.0])
            d = [math.cos(theta), math.sin(theta), -slope]
            back = rng.uniform(1.0, 10.0) / slope + (hi - target[2] + 0.5) / slope
            eye = [target[i] - back * d[i] for i in range(3)]
        elif kind < 0.85:
            eye = [rng.uniform(-6, w + 6), rng.uniform(-6, h + 6), hi + rng.uniform(1, 12)]
            d = [math.cos(theta), math.sin(theta), -rng.choice([0.04, 0.1, 0.3, 0.8, 2.0, 8.0])]
        else:
            eye = [rng.choice([rng.uniform(-4, 0), rng.uniform(w, w + 4)]), rng.uniform(0, h), rng.uniform(lo, hi + 0.5)]
            cx, cy = rng.uniform(0, w), rng.uniform(0, h)
            theta = math.atan2(cy - eye[1], cx - eye[0])
            d = [math.cos(theta), math.sin(theta), -rng.choice([0.01, 0.03, 0.08])]
        d = [0.0 if abs(x) < 1e-12 else x for x in d]
        out.append(dict(eye=[bits(x) for x in eye], direction=[bits(x) for x in d], secondary=rng.random() < 0.7,
                        margin=rng.choice([0, 0, 0, 1, 2])))
    return out


def resolve(op, st):
    """Turn symbolic lab operations into concrete ones against the state."""
    op = dict(op)
    for which in ('min', 'max'):
        if op.get('cell') == which + '-holder':
            op['cell'] = st['map'][which + '_holder']
    return op


def run_case(precision, name, s, ray_count):
    rng = random.Random(sum(map(ord, name)) * 7919)
    lab = Lab(precision, s)
    row = dict(precision=precision, case=name, ops=[], rays=[])
    try:
        built = lab.construct()
    except nm.UcError as e:
        # 723E00 idiv #DE (zero quad size or quads-per-gtile): Rust error.
        row['construct'] = dict(fault=str(e), eip=hex(lab.m.u.reg_read(UC_X86_REG_EIP)), asserts=list(lab.seams.asserts))
        lab.m.verify_v14()
        row['_visited'], row['_seams'] = lab.m.visited, lab.seams.calls
        return row
    row['construct'] = built
    if built['asserts']:
        lab.m.verify_v14()
        row['_visited'], row['_seams'] = lab.m.visited, lab.seams.calls
        return row
    st = built['state']
    resident = set()
    for op in sequences(random.Random(sum(map(ord, name))), name, s):
        op = resolve(op, st)
        if op['op'] == 'heights':
            # Raise every sample of the min-holder block above the current
            # map minimum (or lower the max-holder's below the maximum).
            m = lab.m
            key = 'min_holder' if op['set'] == 'raise-min' else 'max_holder'
            cell = st['map'][key]
            blk = st['blocks'][cell]
            q, cols = blk['quads'], s['heightfield']['columns']
            sets = []
            for yy in range(q + 1):
                for xx in range(q + 1):
                    i = (blk['index'][1] * q + yy) * cols + blk['index'][0] * q + xx
                    v = struct.unpack('<f', struct.pack('<I', m.r(m.r(lab.hf + 4) + 4 * i)))[0]
                    nv = v + 0.375 if op['set'] == 'raise-min' else v - 0.375
                    sets.append([i, bits(nv)])
            op = dict(op='heights', set=sets)
        if op['op'] == 'load' and op['cell'] in resident:
            continue
        if op['op'] == 'release' and op['cell'] not in resident:
            continue
        st = lab.op(op)
        if op['op'] == 'load':
            resident.add(op['cell'])
        if op['op'] == 'release':
            resident.discard(op['cell'])
        row['ops'].append(dict(op=op, state=st))
    row['resident'] = sorted(resident)
    for r in rays(rng, s, st, ray_count):
        r['native'] = lab.ray(r['eye'], r['direction'], r['secondary'], r['margin'])
        row['rays'].append(r)
    lab.m.verify_v14()
    row['_visited'] = lab.m.visited
    row['_seams'] = lab.seams.calls
    return row


def main():
    all_specs = specs()
    rows = []
    visited = {}
    seams = {}
    for precision in nm.PRECISIONS:
        for name, s in all_specs.items():
            count = 0 if name.startswith(('water', 'assert')) else 40
            row = run_case(precision, name, s, count)
            visited.update(row.pop('_visited', {}))
            for k, v in row.pop('_seams', {}).items():
                seams[k] = seams.get(k, 0) + v
            rows.append(row)
    from capstone import Cs, CS_ARCH_X86, CS_MODE_32
    md = Cs(CS_ARCH_X86, CS_MODE_32)
    coverage = {}
    for fn, (lo, hi) in RANGES.items():
        listed = [i.address for i in md.disasm(original.range(lo, hi - lo), lo) if i.mnemonic != 'int3']
        missing = [hex(a) for a in listed if a not in visited]
        coverage[fn] = dict(instructions=len(listed), executed=len(listed) - len(missing), missing=missing)
    for a, b in visited.items():
        assert b == original.range(a, len(b)) == modified.range(a, len(b)), hex(a)
    doc = dict(nm.header(__file__, 'unicorn original+v14 images, PC24/PC53/PC64'), scope=__doc__, specs=all_specs, rows=rows,
               coverage=coverage, seam_calls=dict(sorted(seams.items())), executed_pins=len(visited),
               harness_sha256=nm.sha(Path(nm.__file__).read_bytes()))
    data = (json.dumps(doc, indent=0, separators=(',', ':')) + '\n').encode()
    if '--dry' in sys.argv:
        for k, v in coverage.items():
            print(k, v['executed'], '/', v['instructions'], ' '.join(v['missing'][:24]))
    elif '--write' in sys.argv:
        with OUT.open('xb') as f:
            f.write(data)
    else:
        assert OUT.read_bytes() == data, 'native-terrain-map.json differs from this producer'
    print(json.dumps(dict(rows=len(rows), rays=sum(len(r['rays']) for r in rows), ops=sum(len(r['ops']) for r in rows),
                          executed_pins=len(visited), sha256=hashlib.sha256(data).hexdigest())))


if __name__ == '__main__':
    main()
