"""Producer of native-terrain-fog.json: the ORIGINAL terrain fog-of-war code
of AA.exe (image base 0x400000) run under Unicorn, in both pinned images
(every executed instruction byte is checked equal in AA.exe and the v14
image) at PC24/PC53/PC64.

Native functions executed (not seamed):
  71BE70  darken toward a darker neighbour (fsqrt, _ftol 96CC4C)
  71BD00  per-cell brightness (block +B8 == 2 and +88 cell path)
  71BEF0  one fog pixel from its cell and eight neighbours
  71C290  rebuild/blend the block's n*n fog pixels (+10C prev, +118 target,
          +124 pixel data, +124+4C revision)
  720C79..720CC3  the fog-grid sizing tail of 720AD0 (+134, +12C, +130, +13C)
  720670  block cell array +88 ((d0+2)^2 cells of 16 bytes: +0 tile+0
          snapshot, +4 tile, +8 owning block, +C flags = 0x20)
  7203F0  per-cell flags (debug nibble, 0x20 unexplored, 0x10 explored only)
          with 54F3E0/54ED10/54EB70/54EB40, 4B6F90, 40CFA0's callers
  71B500  debug-UVar change detection (+14B)
  7208D0  the per-block fog half: 71B500, 7203F0 loop, +108 fade timer,
          40CEC0/40CEE0, 71C290 (the water tail +14F is not reached: 0)
  725340  TerrainMap update: b403c9, +138/+13C time, +140 local kingdom,
          b285d4, b403c8 ViewShowFogDebug toggle (other branches not reached)
  45BF20  view vt+50: Thing-less (4282A0, 42A530, 54F3E0) and Thing paths
  45BD40  view vt+54: Thing-less and Thing paths
  45BBD0  PerformanceDisableDoodads (4BB3C0, Data vt+40)
  4EAE20  remembered aspect selection; 4EACC0 remembered data selection
  4EAB20  Thing vt+60 remembered; 4EA350 potentially visible; 4EA430 record

Seams (explicit, recorded): 40FCA0 (gate answer), 4B2E60 (UVar records
DebugObjectBlocking / DebugWaterBlocking / PerformanceDisableDoodads),
54EF80 (zone debug answer), 40CFA0, 5C0E50, 5C0E10, 45BBD0 (in the view
cases), 71FFA0, 71CE70, 4325C0, 71F3B0, 71C470, 737470, 738280, 59ABA0,
72EF70, 6F7810 (allocator), 71FEE0, the B36540+20 lock object, Thing
vt+5C/+60/+74/+8C/+A8, Data vt+38/+40, 4EA350, 4EA430, 4EAAA0 (each where
the case says).

  python native_terrain_fog.py --write | (compare) | --dry
"""
from pathlib import Path
import hashlib, json, random, struct, sys
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase8-ai'))
import native_machine as nm  # noqa: E402
from native_machine import f32, from_f32  # noqa: E402
from unicorn.x86_const import UC_X86_REG_EBP, UC_X86_REG_ESP, UC_X86_REG_EIP, UC_X86_REG_FPCW, UC_X86_REG_FPSW  # noqa: E402

OUT = HERE / 'native-terrain-fog.json'
PCS = ['Single24', 'Double53', 'Extended64']
UVARS, AVARS, SVARS = 0xB39940, 0xB39990, 0xB39AB0
SESSION, GWORLD, TERRAIN_MAP, LOCK = 0xB36550, 0xB36534, 0xB36518, 0xB36540
B285D4, B403C8, B403C9 = 0xB285D4, 0xB403C8, 0xB403C9
UVAR_NAMES = {0xA88CA0: 'DebugObjectBlocking', 0xA88C78: 'DebugWaterBlocking', 0xA02D08: 'PerformanceDisableDoodads'}


def wstr(m, a):
    out = []
    while True:
        c = struct.unpack('<H', bytes(m.u.mem_read(a, 2)))[0]
        if c == 0:
            return ''.join(out)
        out.append(chr(c))
        a += 2


class Lab:
    """One Machine with the global tables the fog code reads."""

    def __init__(self, pc, image='original'):
        m = self.m = nm.Machine(pc)
        if image == 'v14':
            old = (nm.original.image_size + 4095) & ~4095
            new = (nm.modified.image_size + 4095) & ~4095
            if new > old:
                m.u.mem_map(nm.original.base + old, new - old)
            for vs, r, size, off in nm.modified.sections:
                if size:
                    m.u.mem_write(nm.modified.base + r, nm.modified.b[off:off + size])
        self.calls = []
        # UVar/AVar/SVar manager arrays: slot -> pointer to its value cell.
        self.uvar = {}
        self.avar = {}
        for base, table in ((UVARS, self.uvar), (AVARS, self.avar)):
            arr = m.alloc(4 * 0x120)
            for i in range(0x120):
                cell = m.alloc(8)
                m.w(arr + 4 * i, cell)
                table[i] = cell
            m.w(base, arr)
        svars = m.alloc(4 * 0x80)
        self.fog_svar = m.alloc(8)
        m.w(svars + 0x108, self.fog_svar)
        m.w(SVARS, svars)
        self.session = m.alloc(0x200)
        m.w(SESSION, self.session)
        self.gworld = m.alloc(0x400)
        m.w(GWORLD, self.gworld)
        lock = m.alloc(0x40)
        lock_vt = m.alloc(0x20)
        m.w(lock_vt + 4, m.stub(lambda mm: mm.ret(0, 0)))
        m.w(lock_vt + 8, m.stub(lambda mm: mm.ret(0, 0)))
        m.w(lock + 0x20, lock_vt)
        m.w(LOCK, lock)
        self.records = {}
        for name in UVAR_NAMES.values():
            self.records[name] = m.alloc(0x30)

        def uvar_lookup(mm):
            name = wstr(mm, mm.arg(1))
            self.calls.append(['4B2E60', name, mm.arg(2)])
            mm.ret(self.records[name], 8)
        m.seam(0x4B2E60, uvar_lookup)

        def alloc(mm):
            mm.ret(mm.alloc(mm.arg(1)), 0)
        m.seam(0x6F7810, alloc)
        self.gate = 0
        self.god = 0
        self.kingdom = 0
        self.zone_answer = {}

        def gate(mm):
            self.calls.append(['40FCA0', mm.arg(1), mm.r(mm.arg(2)) if mm.arg(2) == 0xB43B40 else mm.arg(2)])
            mm.ret(self.gate, 8)
        m.seam(0x40FCA0, gate)

        def god(mm):
            self.calls.append(['40CFA0'])
            mm.ret(self.god, 0)
        m.seam(0x40CFA0, god)

        def kingdom(mm):
            self.calls.append(['5C0E50'])
            mm.ret(self.kingdom, 0)
        m.seam(0x5C0E50, kingdom)

        def zone(mm):
            bit = mm.arg(1) & 0xff
            self.calls.append(['54EF80', bit])
            mm.ret(self.zone_answer.get(bit, 0), 12)
        m.seam(0x54EF80, zone)

        def terrain_change(mm):
            self.calls.append(['71FFA0', mm.arg(1), mm.arg(2), mm.arg(3)])
            mm.ret(0, 12)
        m.seam(0x71FFA0, terrain_change)
        for a, pop in ((0x71CE70, 0), (0x4325C0, 4), (0x71F3B0, 0), (0x71C470, 0), (0x737470, 0), (0x738280, 0),
                       (0x71C660, 0), (0x7237E0, 0), (0x7232B0, 0), (0x7262B0, 0), (0x71A750, 0), (0x71A600, 4),
                       (0x738C20, 4)):
            m.seam(a, (lambda name, pop: lambda mm: (self.calls.append([name]), mm.ret(0, pop)))('%X' % a, pop))
        self.time_ms = 0

        def timer(mm):
            self.calls.append(['59ABA0'])
            mm.ret(self.time_ms, 0)
        m.seam(0x59ABA0, timer)
        m.seam(0x72EF70, lambda mm: (self.calls.append(['72EF70']), mm.ret(0, 0)))

        def texture_use(mm):
            self.calls.append(['71FEE0', self.tile_name(mm.arg(1)), mm.arg(2)])
            mm.ret(0, 8)
        m.seam(0x71FEE0, texture_use)
        self.variations = {}

    def tile_name(self, v):
        return self.variations.get(v, v)

    def set_uvar_byte(self, slot, v):
        self.m.wb(self.uvar[slot], v)

    def set_record(self, name, v):
        self.m.wb(self.records[name], v)


def f32b(x):
    return f32(x)


# ------------------------------------------------------------------ block --
class Block:
    def __init__(self, lab, d0, n, step_bits, col=0, row=0, map_ptr=None):
        m = lab.m
        self.lab, self.d0, self.n = lab, d0, n
        p = self.p = m.alloc(0x180)
        m.w(p + 0x40, map_ptr or 0)
        m.w(p + 0xac, col)
        m.w(p + 0xb0, row)
        m.w(p + 0xb8, 2)
        m.w(p + 0xd0, d0)
        m.w(p + 0x12c, n)
        m.w(p + 0x13c, step_bits)
        cells = (d0 + 2) ** 2
        self.cells = m.alloc(16 * cells)
        m.w(p + 0x88, self.cells)
        self.prev = m.alloc(n * n)
        self.target = m.alloc(n * n)
        m.w(p + 0x10c, self.prev)
        m.w(p + 0x110, n * n)
        m.w(p + 0x118, self.target)
        m.w(p + 0x11c, n * n)
        pix = self.pix = m.alloc(0x60)
        self.buf = m.alloc(4 * n * n)
        lv = m.alloc(8)
        m.w(lv, self.buf)
        m.w(lv + 4, self.buf + 4 * n * n)
        m.w(pix + 0x40, lv)
        m.w(pix + 0x34, 0)
        m.w(pix + 0x4c, 0)
        m.w(p + 0x124, pix)

    def set_flags(self, flags):
        for i, f in enumerate(flags):
            self.lab.m.wb(self.cells + 16 * i + 0xc, f)

    def flags(self):
        return [self.lab.m.rb(self.cells + 16 * i + 0xc) for i in range((self.d0 + 2) ** 2)]

    def pixels(self):
        return list(bytes(self.lab.m.u.mem_read(self.buf, 4 * self.n * self.n)))

    def set_pixels(self, data):
        self.lab.m.u.mem_write(self.buf, bytes(data))

    def bytes_of(self, a):
        return list(bytes(self.lab.m.u.mem_read(a, self.n * self.n)))

    def state(self):
        m, p = self.lab.m, self.p
        return {'fade_108': m.r(p + 0x108), 'loaded_141': m.rb(p + 0x141), 'fading_142': m.rb(p + 0x142),
                'debug_14b': m.rb(p + 0x14b), 'fog_14c': m.rb(p + 0x14c), 'flags': self.flags(),
                'pixels': self.pixels(), 'prev': self.bytes_of(self.prev), 'target': self.bytes_of(self.target),
                'revision': m.r(self.pix + 0x4c)}


# --------------------------------------------------------------- 71BE70 --
def be70_cases(rng):
    d2s = [0.0, 1e-9, 0.00999999, 0.01, 0.0100001, 0.25, 0.5, 0.8, 0.8999999, 0.9, 0.9000001, 1.0, 2.0,
           0.0625, 0.140625, 0.390625]
    rows = []
    for _ in range(140):
        rows.append({'res': rng.randrange(256), 'neighbor': rng.choice([0, 1, 127, 128, 254, 255, rng.randrange(256)]),
                     'd2': f32b(rng.choice(d2s + [rng.random() * 1.2])), 'junk': rng.randrange(1 << 32)})
    for res in (0, 1, 127, 128, 255):
        for nb in (0, 127, 128, 254, 255):
            for d2 in (0.01, 0.0099, 0.89, 0.25):
                rows.append({'res': res, 'neighbor': nb, 'd2': f32b(d2), 'junk': 0x12345600 | res})
    return rows


def run_be70(lab, c):
    m = lab.m
    out = m.alloc(4)
    m.w(out, 0xcdcdcd00 | c['res'])
    m.call(0x71BE70, 0, [out, c['junk'], 0xabcdef00 | c['neighbor'], c['d2']], pop=0)
    return m.rb(out), m.r(out) >> 8


# ------------------------------------------------------------- 71BEF0 ---
def grid_case(rng, d0):
    choices = [0x00, 0x00, 0x10, 0x20, 0x30, 0x01, 0x11, 0x0f]
    return [rng.choice(choices) for _ in range((d0 + 2) ** 2)]


def bef0_cases(rng):
    rows = []
    for _ in range(60):
        d0 = rng.choice([1, 2, 3, 4])
        n = d0 * rng.choice([1, 2, 4])
        step = f32b(d0 / max(n - 1, 1)) if n > 1 else f32b(float(d0))
        flags = grid_case(rng, d0)
        pts = []
        for _ in range(6):
            pts.append([f32b(rng.random() * d0), f32b(rng.random() * d0)])
        for i in (0, n - 1):
            for j in (0, n // 2, n - 1):
                pts.append([f32b(from_f32(step) * i), f32b(from_f32(step) * j)])
        rows.append({'d0': d0, 'b285d4': rng.choice([0x7f, 0x80, 0x00, 0xff, rng.randrange(256)]), 'flags': flags,
                     'points': pts})
    return rows


def run_bef0(lab, c):
    m = lab.m
    blk = Block(lab, c['d0'], 1, f32b(1.0))
    blk.set_flags(c['flags'])
    m.wb(B285D4, c['b285d4'])
    return [m.call(0x71BEF0, blk.p, [x, y]) & 0xff for x, y in c['points']]


# -------------------------------------------------------------- 71C290 ---
def c290_cases(rng):
    rows = []
    ts = [0.0, 1.0, 0.5, 0.25, 0.75, 0.1, 0.999, 0.0001]
    for _ in range(36):
        d0 = rng.choice([1, 2, 2, 3, 4])
        per = rng.choice([1, 2, 4])
        n = d0 * per
        if n < 2:
            n, per = 2 * d0, 2
        rows.append({'d0': d0, 'n': n, 'step_bits': f32b(d0 / (n - 1)), 'b403c8': rng.choice([0, 0, 1]),
                     'b285d4': rng.choice([0x7f, 0x80]), 'flags': grid_case(rng, d0),
                     't': f32b(rng.choice(ts + [rng.random()])), 'rebuild': rng.choice([0, 1, 1]),
                     'pixels': [rng.randrange(256) for _ in range(4 * n * n)],
                     'prev': [rng.randrange(256) for _ in range(n * n)],
                     'target': [rng.randrange(256) for _ in range(n * n)], 'revision': rng.randrange(1000)})
    return rows


def run_c290(lab, c):
    m = lab.m
    blk = Block(lab, c['d0'], c['n'], c['step_bits'])
    blk.set_flags(c['flags'])
    blk.set_pixels(c['pixels'])
    m.u.mem_write(blk.prev, bytes(c['prev']))
    m.u.mem_write(blk.target, bytes(c['target']))
    m.w(blk.pix + 0x4c, c['revision'])
    m.wb(B285D4, c['b285d4'])
    m.wb(B403C8, c['b403c8'])
    m.call(0x71C290, blk.p, [c['t'], 0xffffff00 | c['rebuild']])
    s = blk.state()
    return {'pixels': s['pixels'], 'prev': s['prev'], 'target': s['target'], 'revision': s['revision']}


# ------------------------------------------------------- 720AD0 sizing ---
def sizing_cases(rng):
    rows = []
    for uv in (1, 2, 3):
        for av in (1, 2, 3, 4):
            for d0 in (1, 2, 3, 4, 5, 8):
                rows.append({'uvar0': uv, 'avar61': av, 'd0': d0})
    return rows


def run_sizing(lab, c):
    m, u = lab.m, lab.m.u
    m.w(lab.uvar[0], c['uvar0'])
    m.w(lab.avar[61], c['avar61'])
    blk = m.alloc(0x180)
    m.w(blk + 0xd0, c['d0'])
    sp = nm.STACK - 0x100
    u.reg_write(UC_X86_REG_ESP, sp)
    u.reg_write(UC_X86_REG_EBP, blk)
    u.reg_write(UC_X86_REG_FPCW, nm.PRECISIONS[m.precision])
    u.reg_write(UC_X86_REG_FPSW, 0)
    u.emu_start(0x720C79, 0x720CC9)
    assert u.reg_read(UC_X86_REG_EIP) == 0x720CC9
    return {'w134': m.r(blk + 0x134), 'n12c': m.r(blk + 0x12c), 'w130': m.r(blk + 0x130), 'step13c': m.r(blk + 0x13c)}


# ------------------------------------------------- world for 720670 etc ---
class World:
    """A GMap of tiles, a TerrainMap with its block array, and kingdoms."""

    def __init__(self, lab, w, h, d0):
        m = self.m = lab.m
        self.lab, self.w, self.h, self.d0 = lab, w, h, d0
        self.tiles = m.alloc(0x30 * w * h)
        for i in range(w * h):
            v = m.alloc(0x20)
            lab.variations[v] = i
            m.w(self.tiles + 0x30 * i, v)
        gmap = self.gmap = m.alloc(0x40)
        m.w(gmap + 4, 0)
        m.w(gmap + 0x1c, w)
        m.w(gmap + 0x20, h)
        m.w(gmap + 0x28, self.tiles)
        m.w(lab.gworld + 0x30, gmap)
        m.w(lab.gworld + 0x290, 0)
        tm = self.tm = m.alloc(0x200)
        self.bx, self.by = w // d0, h // d0
        quad = 1
        m.w(tm + 0xa8, d0 * 2)  # quads per block axis (gtile = 2 quads)
        m.w(tm + 0xb4, 2)
        m.w(tm + 0x84, self.bx)
        m.w(tm + 0x88, self.by)
        self.block_ptrs = m.alloc(4 * self.bx * self.by)
        for i in range(self.bx * self.by):
            b = m.alloc(0x10)
            m.w(self.block_ptrs + 4 * i, b)
        m.w(tm + 0xb8, self.block_ptrs)
        m.w(TERRAIN_MAP, tm)
        self.kingdoms = {}

    def tile(self, i):
        return self.tiles + 0x30 * i

    def set_fog(self, words):
        for i, w in enumerate(words):
            self.m.w(self.tile(i) + 0x14, w)

    def kingdom(self, name, w190, w228):
        p = self.kingdoms.get(name) or self.m.alloc(0x300)
        self.kingdoms[name] = p
        self.m.w(p + 0x190, w190)
        self.m.w(p + 0x228, w228)
        return p

    def cell_view(self, blk):
        """Each cell's +0 (tile index), +4 (tile index or None), +8 (block
        index or None), +C flags."""
        out = []
        for i in range((blk.d0 + 2) ** 2):
            a = blk.cells + 16 * i
            v0, t, b8, fl = self.m.r(a), self.m.r(a + 4), self.m.r(a + 8), self.m.rb(a + 0xc)
            tidx = (t - self.tiles) // 0x30 if t else None
            bidx = None
            if b8:
                for j in range(self.bx * self.by):
                    if self.m.r(self.block_ptrs + 4 * j) == b8:
                        bidx = j
            out.append([self.lab.variations.get(v0, v0 if v0 == 0 else -1), tidx, bidx, fl])
        return out


def make_block(lab, world, col, row, n_per_gtile):
    m = lab.m
    d0 = world.d0
    n = d0 * n_per_gtile
    blk = Block(lab, d0, n, f32b(d0 / (n - 1)), col, row)
    # 720670 builds +88: clear the lab array first so the native one is used.
    m.w(blk.p + 0x88, 0)
    m.call(0x720670, blk.p, [])
    blk.cells = m.r(blk.p + 0x88)
    return blk


def cells_cases():
    rows = []
    for (w, h, d0) in ((4, 4, 2), (6, 4, 2), (6, 6, 3), (8, 4, 4)):
        for col in range(w // d0):
            for row in range(h // d0):
                rows.append({'w': w, 'h': h, 'd0': d0, 'col': col, 'row': row})
    return rows


def run_cells(c, pc, image):
    lab = Lab(pc, image)
    world = World(lab, c['w'], c['h'], c['d0'])
    blk = make_block(lab, world, c['col'], c['row'], 2)
    return {'cells': world.cell_view(blk), 'calls': lab.calls}, lab.m.visited


# ------------------------------------------------------- 7203F0 direct ---
def cell_cases(rng):
    rows = []
    for _ in range(160):
        rows.append({
            'in_map': rng.random() < 0.9, 'flags_in': rng.choice([0, 0x20, 0x10, 0x30, 0x0f, 0x3f, rng.randrange(256)]),
            'gate': rng.choice([0, 1]), 'dob': rng.choice([0, 0, 1]), 'dwb': rng.choice([0, 0, 1]),
            'u22': rng.choice([0, 0, 1]), 'u25': rng.choice([0, 1]), 'u27': rng.choice([0, 1]), 'u29': rng.choice([0, 1]),
            'zone_set': rng.choice([0, 1]), 'zone': {str(b): rng.choice([0, 1]) for b in (2, 4, 5)},
            'blocking_20': rng.randrange(1 << 12) << 4 | rng.randrange(16), 'god': rng.choice([0, 0, 0, 1]),
            'kingdom': rng.choice([None, 'k', 'k', 'k']), 'k190': rng.choice([0, 0, 0, 1]),
            'k228': rng.choice([0x10001, 0x30003, 0x20002, 0x80008, rng.randrange(1 << 32)]),
            'fog_word': rng.choice([0, 0x1, 0x10001, 0x3, 0x30003, 0x20000, rng.randrange(1 << 32)]),
            'fogofwar': rng.choice([0, 1, 2, 2]), 'variation_changed': rng.random() < 0.08,
            'b14b': rng.choice([0, 0, 1]), 'b14c': rng.choice([0, 0, 1])})
    return rows


def run_cell(c, pc, image):
    lab = Lab(pc, image)
    m = lab.m
    world = World(lab, 4, 4, 2)
    blk = make_block(lab, world, 0, 0, 2)
    lab.calls.clear()
    cell = blk.cells + 16 * 5  # an interior cell (tile 0)
    t = world.tile(0)
    if not c['in_map']:
        m.w(cell + 4, 0)
        m.w(cell + 8, 0)
    m.wb(cell + 0xc, c['flags_in'])
    lab.gate, lab.god = c['gate'], c['god']
    lab.set_record('DebugObjectBlocking', c['dob'])
    lab.set_record('DebugWaterBlocking', c['dwb'])
    for slot, key in ((22, 'u22'), (25, 'u25'), (27, 'u27'), (29, 'u29')):
        lab.set_uvar_byte(slot, c[key])
    m.w(t + 4, m.alloc(0x10) if c['zone_set'] else 0)
    lab.zone_answer = {int(k): v for k, v in c['zone'].items()}
    m.w(t + 0x20, c['blocking_20'])
    m.w(t + 0x14, c['fog_word'])
    m.w(lab.fog_svar, c['fogofwar'])
    lab.kingdom = world.kingdom('k', c['k190'], c['k228']) if c['kingdom'] else 0
    if c['variation_changed']:
        nv = m.alloc(0x20)
        lab.variations[nv] = 'new'
        m.w(t, nv)
    m.wb(blk.p + 0x14b, c['b14b'])
    m.wb(blk.p + 0x14c, c['b14c'])
    m.call(0x7203F0, cell, [blk.p])
    calls = []
    for call in lab.calls:
        if call[0] == '71FFA0':
            calls.append(['71FFA0', lab.tile_name(call[2]), lab.tile_name(call[3])])
        else:
            calls.append(call)
    return {'flags': m.rb(cell + 0xc), 'b14b': m.rb(blk.p + 0x14b), 'b14c': m.rb(blk.p + 0x14c),
            'snapshot': lab.tile_name(m.r(cell)), 'calls': calls}, m.visited


# ---------------------------------------------- frames: 7208D0 + 725340 ---
def frame_cases(rng):
    rows = []
    for seed in range(5):
        r = random.Random(1000 + seed)
        steps = []
        t = r.randrange(1000, 5000)
        fog = [0] * 8  # the lab GMap is 4 x 2 tiles
        for i in range(8):
            fog[i] = r.choice([0, 0x1, 0x2, 0x3])
        kingdom = r.choice(['k', 'k', None])
        for f in range(10):
            # Units reveal/hide: random visible-bit changes on some tiles.
            if f > 0:
                for _ in range(r.randrange(0, 4)):
                    i = r.randrange(8)
                    fog[i] ^= r.choice([0x10000, 0x20000, 0x1, 0x10001])
                if r.random() < 0.15:
                    kingdom = r.choice(['k', 'j', None])
            t += r.choice([16, 17, 33, 100, 250, 400])
            steps.append({'kind': '7208D0', 'block': r.randrange(2), 'fog': list(fog), 'kingdom': kingdom,
                          'app_e4': r.choice([1, 1, 1, 3, 5]) if r.random() < 0.15 else 1})
            steps.append({'kind': '7208D0', 'block': 1 - steps[-1]['block'], 'fog': list(fog), 'kingdom': kingdom,
                          'app_e4': steps[-1]['app_e4']})
            steps.append({'kind': '725340', 'time_ms': t, 'kingdom': kingdom,
                          'fog_debug': r.choice([0, 0, 0, 0, 1]) if seed == 3 else 0})
        rows.append({'seed': seed, 'fogofwar': [1, 2, 0, 1, 2][seed], 'k190': 0, 'k228': [0x10001, 0x30003][seed % 2],
                     'j228': 0x20002, 'gray_64': f32b([0.5, 0.5, 0.25, 0.5, 0.7][seed]),
                     'fade_65': f32b([0.5, 0.5, 0.3, 1.0, 0.5][seed]), 'god': 1 if seed == 4 else 0,
                     'n_per_gtile': 2 if seed != 2 else 4, 'steps': steps})
    return rows


def run_frames(c, pc, image):
    lab = Lab(pc, image)
    m = lab.m
    world = World(lab, 4, 2, 2)
    tm = world.tm
    # TerrainMap fields that 725340 reads; +13C/+138/+140 start 0 (721330).
    for off in (0x100, 0xbf, 0xc0, 0x101):
        m.wb(tm + off, 0)
    m.w(tm + 0x14c, 0)
    m.w(tm + 0x134, m.alloc(0x40))
    blocks = [make_block(lab, world, col, 0, c['n_per_gtile']) for col in range(2)]
    for b in blocks:
        m.w(b.p + 0x40, tm)
        m.wb(b.p + 0x141, 1)  # 71CCB0 after the 42FE10 load
    m.w(lab.fog_svar, c['fogofwar'])
    m.w(lab.avar[64], c['gray_64'])
    m.w(lab.avar[65], c['fade_65'])
    lab.god = c['god']
    ks = {'k': world.kingdom('k', c['k190'], c['k228']), 'j': world.kingdom('j', 0, c['j228'])}
    names = {v: k for k, v in ks.items()}
    out = []
    for step in c['steps']:
        lab.calls.clear()
        lab.kingdom = ks[step['kingdom']] if step['kingdom'] else 0
        if step['kind'] == '7208D0':
            world.set_fog(step['fog'])
            m.w(lab.session + 0xe4, step['app_e4'])
            m.call(0x7208D0, blocks[step['block']].p, [])
            out.append({'block': blocks[step['block']].state(), 'calls': [x for x in lab.calls if x[0] != '40FCA0']})
        else:
            lab.time_ms = step['time_ms']
            lab.set_uvar_byte(31, step['fog_debug'])
            m.call(0x725340, tm, [])
            out.append({'b403c8': m.rb(B403C8), 'b403c9': m.rb(B403C9), 'b285d4': m.rb(B285D4),
                        'w138': m.r(tm + 0x138), 'w13c': m.r(tm + 0x13c),
                        'kingdom_140': names.get(m.r(tm + 0x140)), 'calls': lab.calls[:]})
    return out, m.visited


# ------------------------------------------- 45BF20 / 45BD40 Thing-less ---
def view_cases(rng):
    rows = []
    for _ in range(80):
        rows.append({'doodads': rng.choice([0, 0, 0, 1]), 'god': rng.choice([0, 0, 0, 1]),
                     'player': rng.choice([0, 1, 1, 1]), 'flags_54': rng.choice([0, 0, 1 << 26, 1 << 20, 1 << 3]),
                     'kingdom': rng.choice([None, 'k', 'k']), 'k190': rng.choice([0, 0, 1]),
                     'k228': rng.choice([0x10001, 0x20002, 0x30003]),
                     'x': f32b(rng.choice([-0.5, 0.0, 0.25, 3.9, 7.99, 8.0, 9.5, rng.random() * 8])),
                     'y': f32b(rng.choice([-1.0, 0.0, 2.5, 3.999, 4.0, rng.random() * 4])),
                     'fogofwar': rng.choice([0, 1, 2]), 'fog': [rng.choice([0, 0x10000, 0x20000, 0x30001, 0x1])
                                                               for _ in range(8)], 'a': rng.choice([0, 1])})
    return rows


def run_view(c, pc, image):
    lab = Lab(pc, image)
    m = lab.m
    gmap = m.alloc(0x40)
    tiles = m.alloc(0x30 * 8)
    m.w(gmap + 4, 1)  # world-to-tile shift: 2 m tiles
    m.w(gmap + 0xc, f32b(8.0))
    m.w(gmap + 0x10, f32b(4.0))
    m.w(gmap + 0x1c, 4)
    m.w(gmap + 0x20, 2)
    m.w(gmap + 0x28, tiles)
    for i, w in enumerate(c['fog']):
        m.w(tiles + 0x30 * i + 0x14, w)
    m.w(lab.gworld + 0x30, gmap)
    m.w(lab.fog_svar, c['fogofwar'])
    k = m.alloc(0x300)
    m.w(k + 0x190, c['k190'])
    m.w(k + 0x228, c['k228'])
    lab.kingdom = k if c['kingdom'] else 0
    lab.god = c['god']
    m.seam(0x45BBD0, lambda mm: (lab.calls.append(['45BBD0']), mm.ret(c['doodads'], 0)))
    m.seam(0x5C0E10, lambda mm: (lab.calls.append(['5C0E10']), mm.ret(c['player'], 0)))
    view = m.alloc(0x100)
    m.w(view + 0x14, 0)
    m.w(view + 0x10, m.alloc(0x40))
    m.w(view + 0x38, c['x'])
    m.w(view + 0x3c, c['y'])
    m.w(view + 0x54, c['flags_54'])
    out = {}
    for name, addr in (('vt50', 0x45BF20), ('vt54', 0x45BD40)):
        lab.calls.clear()
        out[name] = m.call(addr, view, [c['a']]) & 0xff
        out[name + '_calls'] = lab.calls[:]
    return out, m.visited


# ------------------------------------------------ 4EAE20 / 4EACC0 -------
def apparent_cases(rng):
    rows = []
    for _ in range(90):
        rows.append({'visible_5c': rng.choice([0, 1]), 'remembered_60': rng.choice([0, 1]),
                     'flags_8': rng.choice([0, 1 << 7, 0x81]), 'actor_38': rng.choice([0, 1]),
                     'mask_174': rng.choice([0, 1 << 26]), 'potential_4ea350': rng.choice([0, 1]),
                     'kingdom': rng.choice([None, 'k'])})
    return rows


def run_apparent(c, pc, image):
    lab = Lab(pc, image)
    m = lab.m
    k = m.alloc(0x300) if c['kingdom'] else 0
    record = m.alloc(0x40)
    m.w(record + 0x10, 0xD0D0D0D0)
    m.w(record + 0x18, 0xA5A5A5A5)
    thing = m.alloc(0x100)
    data = m.alloc(0x200)
    m.w(data + 0x174, c['mask_174'])
    dvt = m.alloc(0x100)
    m.w(dvt + 0x38, m.stub(lambda mm: (lab.calls.append(['data_38']), mm.ret(c['actor_38'], 0))))
    m.w(data, dvt)
    vt = m.alloc(0x200)

    def seam(name, value, pop):
        def run(mm):
            lab.calls.append([name, 'k' if mm.arg(1) == k and k else None, mm.arg(2) & 0xff])
            mm.ret(value, pop)
        return run
    m.w(vt + 0x5c, m.stub(seam('vt5c', c['visible_5c'], 8)))
    m.w(vt + 0x60, m.stub(seam('vt60', c['remembered_60'], 8)))
    m.w(thing, vt)
    m.w(thing + 4, data)
    m.w(thing + 8, c['flags_8'])
    m.seam(0x4EA350, lambda mm: (lab.calls.append(['4EA350']), mm.ret(c['potential_4ea350'], 0)))

    def seen(mm):
        lab.calls.append(['4EA430', 'k' if mm.arg(1) == k and k else None])
        mm.ret(record, 4)
    m.seam(0x4EA430, seen)
    m.seam(0x4EAAA0, lambda mm: (lab.calls.append(['4EAAA0']), mm.ret(0xC0C0C0C0, 0)))
    out = {}
    for name, addr in (('aspect_4eae20', 0x4EAE20), ('data_4eacc0', 0x4EACC0)):
        lab.calls.clear()
        r = m.call(addr, thing, [k])
        out[name] = {0xC0C0C0C0: 'current', 0xA5A5A5A5: 'remembered_18', 0xD0D0D0D0: 'remembered_10', 0: 'default',
                     data: 'thing_data'}.get(r, hex(r))
        out[name + '_calls'] = lab.calls[:]
    return out, m.visited


# --------------------------------- 45BF20 / 45BD40 with a Thing (view+14) --
def thing_view_cases(rng):
    rows = []
    for _ in range(64):
        rows.append({'doodads': rng.choice([0, 0, 0, 1]), 'god': rng.choice([0, 0, 0, 1]),
                     'player': rng.choice([0, 1, 1, 1]), 'flags_54': rng.choice([0, 0, 1 << 26, 1 << 20, 1 << 7]),
                     'kingdom': rng.choice([None, 'k', 'k']), 'visible_5c': rng.choice([0, 1]),
                     'remembered_60': rng.choice([0, 1]), 'a': rng.choice([0, 1])})
    return rows


def run_thing_view(c, pc, image):
    lab = Lab(pc, image)
    m = lab.m
    k = m.alloc(0x300)
    lab.kingdom = k if c['kingdom'] else 0
    lab.god = c['god']
    m.seam(0x45BBD0, lambda mm: (lab.calls.append(['45BBD0']), mm.ret(c['doodads'], 0)))
    m.seam(0x5C0E10, lambda mm: (lab.calls.append(['5C0E10']), mm.ret(c['player'], 0)))
    thing = m.alloc(0x100)
    vt = m.alloc(0x200)

    def seam(name, value):
        def run(mm):
            lab.calls.append([name, 'k' if mm.arg(1) == k else mm.arg(1), mm.arg(2) & 0xff])
            mm.ret(value, 8)
        return run
    m.w(vt + 0x5c, m.stub(seam('vt5c', c['visible_5c'])))
    m.w(vt + 0x60, m.stub(seam('vt60', c['remembered_60'])))
    m.w(thing, vt)
    view = m.alloc(0x100)
    m.w(view + 0x14, thing)
    m.w(view + 0x10, m.alloc(0x40))
    m.w(view + 0x54, c['flags_54'])
    out = {}
    for name, addr in (('vt50', 0x45BF20), ('vt54', 0x45BD40)):
        lab.calls.clear()
        out[name] = m.call(addr, view, [c['a']]) & 0xff
        out[name + '_calls'] = lab.calls[:]
    return out, m.visited


# ----------------------------------------- 4EAB20, 4EA350, 4EA430 ----------
def remembered_cases(rng):
    rows = []
    for _ in range(64):
        rows.append({'visible_5c': rng.choice([0, 1]), 'explored_74': rng.choice([0, 1]),
                     'potential_4ea350': rng.choice([0, 1]), 'record': rng.choice([0, 1]),
                     'kingdom': rng.choice([None, 'k']), 'a': rng.choice([0, 1])})
    return rows


def run_remembered(c, pc, image):
    lab = Lab(pc, image)
    m = lab.m
    k = m.alloc(0x300) if c['kingdom'] else 0
    thing = m.alloc(0x100)
    vt = m.alloc(0x200)

    def seam(name, value):
        def run(mm):
            lab.calls.append([name, 'k' if mm.arg(1) == k and k else None, mm.arg(2) & 0xff])
            mm.ret(value, 8)
        return run
    m.w(vt + 0x5c, m.stub(seam('vt5c', c['visible_5c'])))
    m.w(vt + 0x74, m.stub(seam('vt74', c['explored_74'])))
    m.w(thing, vt)
    m.seam(0x4EA350, lambda mm: (lab.calls.append(['4EA350']), mm.ret(c['potential_4ea350'], 0)))

    def seen(mm):
        lab.calls.append(['4EA430', 'k' if mm.arg(1) == k and k else None])
        mm.ret(0x5EE00000 if c['record'] else 0, 4)
    m.seam(0x4EA430, seen)
    r = m.call(0x4EAB20, thing, [k, c['a']]) & 0xff
    return {'remembered': r, 'calls': lab.calls[:]}, m.visited


def potential_cases(rng):
    rows = []
    for _ in range(96):
        rows.append({'doodad_40': rng.choice([0, 0, 1]), 'actor_38': rng.choice([0, 1]),
                     'mask_174': rng.choice([0, 1 << 26, 0x3]),
                     'flags_8': rng.choice([0, (1 << 29) | (1 << 11), (1 << 29) | (1 << 11) | (1 << 23),
                                            (1 << 29) | (1 << 11) | (1 << 22), 1 << 29, 1 << 11,
                                            (1 << 29) | (1 << 11) | 0x80]),
                     'byte_16d': rng.choice([0, 0, 1]), 'vt_a8': rng.choice([0, 1])})
    return rows


def run_potential(c, pc, image):
    lab = Lab(pc, image)
    m = lab.m
    data = m.alloc(0x200)
    dvt = m.alloc(0x100)
    m.w(dvt + 0x40, m.stub(lambda mm: (lab.calls.append(['data_40']), mm.ret(c['doodad_40'], 0))))
    m.w(dvt + 0x38, m.stub(lambda mm: (lab.calls.append(['data_38']), mm.ret(c['actor_38'], 0))))
    m.w(data, dvt)
    m.w(data + 0x174, c['mask_174'])
    m.wb(data + 0x16d, c['byte_16d'])
    thing = m.alloc(0x100)
    vt = m.alloc(0x200)
    m.w(vt + 0xa8, m.stub(lambda mm: (lab.calls.append(['vt_a8']), mm.ret(c['vt_a8'], 0))))
    m.w(thing, vt)
    m.w(thing + 4, data)
    m.w(thing + 8, c['flags_8'])
    r = m.call(0x4EA350, thing, []) & 0xff
    return {'potential': r, 'calls': lab.calls[:]}, m.visited


def seen_cases(rng):
    rows = []
    for _ in range(48):
        rows.append({'kingdom': rng.choice([None, 'k', 'k']), 'k190': rng.choice([0, 0, 1]),
                     'k18c': rng.randrange(4), 'vt_8c': rng.choice([0, 1]),
                     'slots': [rng.choice([0, 1]) for _ in range(4)]})
    return rows


def run_seen(c, pc, image):
    lab = Lab(pc, image)
    m = lab.m
    k = 0
    if c['kingdom']:
        k = m.alloc(0x300)
        m.w(k + 0x190, c['k190'])
        m.w(k + 0x18c, c['k18c'])
    thing = m.alloc(0x100)
    vt = m.alloc(0x200)
    m.w(vt + 0x8c, m.stub(lambda mm: (lab.calls.append(['vt_8c']), mm.ret(c['vt_8c'], 0))))
    m.w(thing, vt)
    arr = m.alloc(16)
    for i, s in enumerate(c['slots']):
        m.w(arr + 4 * i, (0x5EE00000 + i) if s else 0)
    m.w(thing + 0x5c, arr)
    r = m.call(0x4EA430, thing, [k])
    return {'record': (r - 0x5EE00000) if r else None, 'calls': lab.calls[:]}, m.visited


def doodad_cases(rng):
    rows = []
    for _ in range(48):
        rows.append({'uvar': rng.choice([0, 1, 1]), 'thing': rng.choice([0, 1, 1]),
                     'flags_8': rng.choice([0, 1 << 10, 0x3]), 'category': rng.choice([1, 0x26, 2, 24, 0]),
                     'doodad_40': rng.choice([0, 0, 1])})
    return rows


def run_doodads(c, pc, image):
    """45BBD0 takes ESI = view+14 (Thing) and EDI = view+10 (Data)."""
    lab = Lab(pc, image)
    m = lab.m
    lab.set_record('PerformanceDisableDoodads', c['uvar'])
    data = m.alloc(0x200)
    dvt = m.alloc(0x100)
    m.w(dvt + 0x40, m.stub(lambda mm: (lab.calls.append(['data_40']), mm.ret(c['doodad_40'], 0))))
    m.w(data, dvt)
    cat = m.alloc(0x20)
    m.w(cat + 0xc, c['category'])
    m.w(data + 0x18, cat)
    thing = 0
    if c['thing']:
        thing = m.alloc(0x40)
        m.w(thing + 8, c['flags_8'])
    stub = m.alloc(32)
    code = b'\x56\x57\xbe' + struct.pack('<I', thing) + b'\xbf' + struct.pack('<I', data) + b'\xe8'
    rel = 0x45BBD0 - (stub + len(code) + 4)
    code += struct.pack('<i', rel) + b'\x5f\x5e\xc3'
    m.u.mem_write(stub, code)
    r = m.call(stub, 0, []) & 0xff
    return {'doodads': r, 'calls': lab.calls[:]}, m.visited


# ------------------------------------------------------------------ main --
def both(fn, c, pc):
    """Run in both images; outputs must agree."""
    a, va = fn(c, pc, 'original')
    b, vb = fn(c, pc, 'v14')
    assert a == b, c
    return a, {**va, **vb}


def main():
    rng = random.Random(20261019)
    doc = {**nm.header(__file__, __doc__)}
    pins = {}
    for key, cases, runner in (('be70', be70_cases(rng), run_be70), ('bef0', bef0_cases(rng), run_bef0),
                               ('c290', c290_cases(rng), run_c290), ('sizing', sizing_cases(rng), run_sizing)):
        rows = []
        for pc in PCS:
            for image in ('original', 'v14'):
                lab = Lab(pc, image)
                outs = [runner(lab, c) for c in cases]
                pins.update(lab.m.visited)
                if image == 'original':
                    first = outs
                else:
                    assert outs == first, key
            rows.append({'precision': pc, 'cases': [{'input': c, 'output': o} for c, o in zip(cases, first)]})
        doc[key] = rows
        print(key, len(cases), file=sys.stderr)
    for key, cases, runner, pcs in (('cells', cells_cases(), run_cells, ['Double53']),
                                    ('cell7203f0', cell_cases(rng), run_cell, ['Double53']),
                                    ('frames', frame_cases(rng), run_frames, PCS),
                                    ('view', view_cases(rng), run_view, ['Double53']),
                                    ('apparent', apparent_cases(rng), run_apparent, ['Double53']),
                                    ('thing_view', thing_view_cases(rng), run_thing_view, ['Double53']),
                                    ('remembered', remembered_cases(rng), run_remembered, ['Double53']),
                                    ('potential', potential_cases(rng), run_potential, ['Double53']),
                                    ('seen', seen_cases(rng), run_seen, ['Double53']),
                                    ('doodads', doodad_cases(rng), run_doodads, ['Double53'])):
        rows = []
        for pc in pcs:
            out = []
            for c in cases:
                o, v = both(runner, c, pc)
                pins.update(v)
                out.append({'input': c, 'output': o})
            rows.append({'precision': pc, 'cases': out})
        doc[key] = rows
        print(key, len(cases), file=sys.stderr)
    for a, b in pins.items():
        assert b == nm.original.range(a, len(b)) == nm.modified.range(a, len(b)), hex(a)
    doc['instructions'] = nm.instructions(pins)
    text = json.dumps(doc, separators=(',', ':'))
    if '--dry' in sys.argv:
        print(json.dumps({'bytes': len(text), 'pins': len(pins)}))
        return
    if '--write' in sys.argv:
        OUT.write_text(text)
    else:
        assert json.loads(OUT.read_text()) == json.loads(text), 'fixture differs'
    print(json.dumps({'pins': len(pins), 'sha256': hashlib.sha256(OUT.read_bytes()).hexdigest()}))


if __name__ == '__main__':
    main()
