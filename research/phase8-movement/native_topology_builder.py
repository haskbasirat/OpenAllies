"""Unicorn oracle for the original static SAI topology builder 7ACB00.

Runs the UNCHANGED original KKC_SAIMap::StaticMapAnalysis (7ACB00) and its whole
call graph (7A9DF0, 7A9EA0, 7A9B00, 7ABD40, 7AA180 incl. CRT pow/floor/_ftol2,
7BAC90, 7AACD0, 7B9B80, 7BA4F0, 7B99B0, 7AB7A0, 7AA8B0, 7AA2A0, 7AB320,
7BC590, 54EDC0, 7BA3C0, 606750, 4849F0, 50C510, ...) on generated GMap tile
grids, then the original 7BCA60 neighbor build and 7B8FB0 category per region.

Seams (entry hooked, body not executed):
  6F7810/6F7430  allocator / free (bump allocator, frees ignored)
  4B2E60         SVar pointer lookup; returns a cell holding the case value
  404F60/404ED0  diagnostic log / warning (recorded, no effect)
  59AA70/59AB60/59ABA0  QueryPerformanceCounter timer (log text only)
  855D30         assertion dialog; reaching it aborts the case
No game process runs; original files are only read.

Unicorn caveat: x87 FYL2X/F2XM1 inside the CRT pow of 7AA180 are emulated
through binary64. 7AA180 only yields an integer after floor; the producer
checks every case's width margin against a binary64 reference. The Rust port
takes that integer as an explicit input.
"""
from pathlib import Path
import hashlib, json, math, random, struct, sys, time

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(HERE))
from harness import P, V, PRECISION, ORIGINAL_SHA, V14_SHA, ALLOCATORS, UnseamedAllocator, page_zero_access  # noqa: E402
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32, UC_HOOK_CODE, UC_HOOK_BLOCK, UC_HOOK_MEM_READ, UC_HOOK_MEM_WRITE  # noqa: E402
from unicorn.x86_const import *  # noqa: E402,F401,F403

HEAP, HEAP_SIZE = 0x10000000, 0x20000000
STACK_BASE, STACK_SIZE = 0x0e000000, 0x01000000
STOP = 0x0d000000
BLOCKS = {}

SVAR_NAMES = {
    'StaticMapStartingSquareSize': ('starting_square_size', 3),
    'StaticMapUseFastSquareReduction': ('use_fast_square_reduction', 1),
    'StaticMapFastSquareReductionMult': ('fast_square_reduction_mult', 3),
    'StaticMapFastSquareReductionDiv': ('fast_square_reduction_div', 3),
    'StaticMapMinRegionSize': ('min_region_size', 3),
    'StaticMapMinRegionWidth': ('min_region_width', 3),
    'StaticMapMaxRegionSizeForExpansion': ('max_region_size_for_expansion', 3),
    'StaticMapMaxRectangleWidth': ('max_rectangle_width_svar', 3),
    'StaticMapBaseMapWidth': ('base_map_width', 3),
    'StaticMapMaxRectangleWidthExp': ('max_rectangle_width_exp_bits', 2),
}
# Data/Game/SVars.tgi values. The 'fixed' .3333 is the nearest binary32.
GAME_SVARS = {
    'starting_square_size': 10, 'use_fast_square_reduction': 1,
    'fast_square_reduction_mult': 3, 'fast_square_reduction_div': 4,
    'min_region_size': 9, 'min_region_width': 3, 'max_region_size_for_expansion': 1000,
    'max_rectangle_width_svar': 12, 'base_map_width': 64,
    'max_rectangle_width_exp_bits': struct.unpack('<I', struct.pack('<f', 0.3333))[0],
}


def f2b(x):
    return struct.unpack('<I', struct.pack('<f', x))[0]


def b2f(x):
    return struct.unpack('<f', struct.pack('<I', x))[0]


class Halt(Exception):
    pass


class Machine:
    def __init__(self, precision, svars):
        self.svars = svars
        u = self.u = Uc(UC_ARCH_X86, UC_MODE_32)
        u.mem_map(0, 0x1000)
        u.mem_map(P.base, (P.image_size + 0xfff) & ~0xfff)
        u.mem_write(P.base, P.b[:0x1000])
        for vs, r, raw, off in P.sections:
            if raw:
                u.mem_write(P.base + r, P.b[off:off + min(raw, max(vs, raw))])
        u.mem_map(HEAP, HEAP_SIZE)
        u.mem_map(STACK_BASE, STACK_SIZE)
        u.mem_map(STOP & ~0xfff, 0x1000)
        self.next = HEAP + 0x1000
        u.reg_write(UC_X86_REG_FPCW, PRECISION[precision])
        self.events = []
        self.svar_reads = []
        seams = {0x6f7810: self.s_alloc, 0x6f7430: self.s_free, 0x4b2e60: self.s_svar,
                 0x404f60: self.s_log, 0x404ed0: self.s_warn, 0x59aa70: self.s_timer,
                 0x59ab60: self.s_timer, 0x59aba0: self.s_elapsed, 0x855d30: self.s_assert}
        for pc, f in seams.items():
            u.hook_add(UC_HOOK_CODE, lambda uc, a, s, d, f=f: f(), begin=pc, end=pc)
        for pc in ALLOCATORS:
            if pc not in seams:
                u.hook_add(UC_HOOK_CODE, lambda uc, a, s, d: self.unseamed(a), begin=pc, end=pc)
        u.hook_add(UC_HOOK_BLOCK, self.block)
        # FS base is 0: only an FS-prefixed access to [0..3] (SEH link) is legal.
        u.hook_add(UC_HOOK_MEM_READ | UC_HOOK_MEM_WRITE, self.null, begin=0, end=0xfff)

    # memory
    def alloc(self, n, align=16):
        p = (self.next + align - 1) & ~(align - 1)
        self.next = p + max(n, 4)
        assert self.next < HEAP + HEAP_SIZE
        self.u.mem_write(p, b'\0' * max(n, 4))
        return p

    def w32(self, a, *v):
        self.u.mem_write(a, struct.pack('<' + 'I' * len(v), *[x & 0xffffffff for x in v]))

    def r32(self, a):
        return struct.unpack('<I', self.u.mem_read(a, 4))[0]

    def s32(self, a):
        return struct.unpack('<i', self.u.mem_read(a, 4))[0]

    def s16(self, a):
        return struct.unpack('<h', self.u.mem_read(a, 2))[0]

    def u16(self, a):
        return struct.unpack('<H', self.u.mem_read(a, 2))[0]

    def r8(self, a):
        return self.u.mem_read(a, 1)[0]

    def esp(self):
        return self.u.reg_read(UC_X86_REG_ESP)

    def arg(self, i):
        return self.r32(self.esp() + 4 + 4 * i)

    def ret(self, pop=0, value=None):
        sp = self.esp()
        self.u.reg_write(UC_X86_REG_EIP, self.r32(sp))
        self.u.reg_write(UC_X86_REG_ESP, sp + 4 + pop)
        if value is not None:
            self.u.reg_write(UC_X86_REG_EAX, value & 0xffffffff)

    def wide(self, a):
        out = []
        while True:
            c = self.u16(a)
            if not c:
                return ''.join(out)
            out.append(chr(c))
            a += 2

    # hooks
    def block(self, u, pc, size, _):
        if P.base <= pc < P.base + P.image_size:
            BLOCKS[pc] = max(size, BLOCKS.get(pc, 0))

    def null(self, u, access, address, size, value, _):
        page_zero_access(u, address, size)

    def unseamed(self, pc):
        raise UnseamedAllocator(hex(pc))

    def s_alloc(self):
        self.ret(value=self.alloc(self.arg(0)))

    def s_free(self):
        self.ret()

    def s_svar(self):
        name, kind = self.wide(self.arg(0)), self.arg(1)
        key, expected = SVAR_NAMES[name]
        assert kind == expected, (name, kind)
        value = self.svars[key]
        cell = self.alloc(8)
        if kind == 1:
            self.u.mem_write(cell, bytes([value & 255]))
        else:
            self.w32(cell, value)
        self.svar_reads.append(name)
        self.ret(8, cell)

    def s_log(self):
        self.events.append({'log_line': self.arg(2)})
        self.ret()

    def s_warn(self):
        self.events.append({'warning_line': self.arg(2), 'value': self.arg(5)})
        self.ret()

    def s_timer(self):
        self.ret()

    def s_elapsed(self):
        self.ret(value=0)

    def s_assert(self):
        raise Halt('855D30 assertion reached')

    def call(self, entry, ecx=0, args=(), count=0):
        sp = STACK_BASE + STACK_SIZE - 0x1000
        self.w32(sp, STOP, *args)
        self.u.reg_write(UC_X86_REG_ESP, sp)
        self.u.reg_write(UC_X86_REG_ECX, ecx)
        keep = [(UC_X86_REG_EBX, 0x11223344), (UC_X86_REG_ESI, 0x33445566),
                (UC_X86_REG_EDI, 0x55667788), (UC_X86_REG_EBP, 0x778899aa)]
        for r, v in keep:
            self.u.reg_write(r, v)
        self.u.emu_start(entry, STOP, count=count)
        if self.u.reg_read(UC_X86_REG_EIP) != STOP:
            return None
        assert all(self.u.reg_read(r) == v for r, v in keep), 'nonvolatile register changed'
        assert (self.u.reg_read(UC_X86_REG_FPSW) >> 11) & 7 == 0, 'x87 stack unbalanced'
        return self.u.reg_read(UC_X86_REG_EAX)


def tile_code(flags, kind):
    assert 0 <= kind < 8 and flags & ~0x300 == 0
    return '0123456789abcdefghijklmnopqrstuv'[(flags >> 8) << 3 | kind]


def tile_inputs(case):
    """(flags20, definition type) per tile: base32 codes, or plain types."""
    if 'tile_definition_types' in case:
        return [(0, k) for k in case['tile_definition_types']]
    return [((int(c, 32) >> 3) << 8, int(c, 32) & 7) for c in case['tiles']]


def setup(m, case):
    lay = case['layout']
    w, h = lay['width'], lay['height']
    world = m.alloc(0x400)
    m.w32(0xb36534, world)
    grid = m.alloc(0x100)
    m.w32(world + 0x30, grid)
    m.w32(grid + 4, lay['coordinate_shift'])
    m.w32(grid + 8, case['cell_size_bits'])
    m.w32(grid + 0xc, lay['extent_bits'][0])
    m.w32(grid + 0x10, lay['extent_bits'][1])
    m.w32(grid + 0x1c, w)
    m.w32(grid + 0x20, h)
    variants = {}
    tiles = m.alloc(w * h * 0x30)
    raw = bytearray(w * h * 0x30)
    for i, (flags, kind) in enumerate(tile_inputs(case)):
        if kind not in variants:
            v, d = m.alloc(0x40), m.alloc(0x40)
            m.w32(v + 0x1c, d)
            m.w32(d + 0x1c, kind)
            variants[kind] = v
        struct.pack_into('<I', raw, i * 0x30, variants[kind])
        struct.pack_into('<I', raw, i * 0x30 + 0x20, flags)
    m.u.mem_write(tiles, bytes(raw))
    m.w32(grid + 0x28, tiles)
    sai = m.alloc(0x100)
    m.w32(0xb3652c, sai)
    flag = m.alloc(0x200)
    m.w32(0xb36518, flag)
    return grid, tiles, sai, flag


def extract(m, case, grid, tiles, sai, flag, full_tiles):
    w, h = case['layout']['width'], case['layout']['height']
    index = {}
    n = m.s32(sai + 0x24)
    arr = m.r32(sai + 0x20)
    rect_ptrs = [m.r32(arr + 4 * i) for i in range(n)]
    for i, p in enumerate(rect_ptrs):
        assert m.r32(p) == i
        index[p] = i
    rects = []
    for p in rect_ptrs:
        sides = []
        for s in range(4):
            node, last, out = m.r32(p + 0x14 + 8 * s), 0, []
            while node:
                so = m.r32(node)
                assert m.r32(so + 4) == s
                out.append([[m.u16(so + 8 + 2 * k) for k in range(4)], m.r32(so)])
                last, node = node, m.r32(node + 4)
            assert m.r32(p + 0x18 + 8 * s) == last
            sides.append(out)
        rects.append({'region': m.s32(p + 4), 'island': m.s32(p + 8),
                      'bounds': [m.s16(p + 0xc + 2 * k) for k in range(4)], 'sides': sides})
    regions = []
    rn = m.s32(sai + 0x18)
    rarr = m.r32(sai + 0x14)
    region_ptrs = [m.r32(rarr + 4 * i) for i in range(rn)]
    for i, p in enumerate(region_ptrs):
        assert m.r32(p) == i and m.s32(p + 0xc) == -1
        node, out = m.r32(p + 4), []
        while node:
            out.append(index[m.r32(node)])
            node = m.r32(node + 4)
        buckets, count = m.r32(p + 0x5c), m.r32(p + 0x60)
        pairs = []
        for b in range(count):
            e = m.r32(buckets + 4 * b)
            while e:
                pairs.append([m.r32(e), m.r32(e + 4)])
                e = m.r32(e + 8)
        assert len(pairs) == m.r32(p + 0x64)
        regions.append({'rectangles': out, 'position_bits': [m.r32(p + 0x54), m.r32(p + 0x58)],
                        'bucket_count': count, 'distance_pairs': pairs})
    islands = []
    for i in range(m.s32(sai + 0xc)):
        p = m.r32(m.r32(sai + 8) + 4 * i)
        assert m.r32(p) == i
        a = m.r32(p + 4)
        islands.append([m.r32(m.r32(a + 4 * j)) for j in range(m.s32(p + 8))])
    q = m.r32(sai + 0x2c)
    tile_rects = []
    for i in range(w * h):
        p = m.r32(tiles + i * 0x30 + 0x10)
        tile_rects.append(index[p] if p else None)
    packed = struct.pack('<' + 'I' * len(tile_rects), *[0xffffffff if v is None else v for v in tile_rects])
    out = {'rectangles': rects, 'regions': regions, 'islands': islands,
           'primary_island': m.r32(q) if q else None, 'diagonal_bits': m.r32(sai + 0x30),
           'analyzed': m.r8(sai + 4), 'world_flag_101': m.r8(flag + 0x101),
           'tile_rectangles_sha256': hashlib.sha256(packed).hexdigest()}
    if full_tiles:
        out['tile_rectangles'] = tile_rects
    # Original 7BCA60 (descending, as the loader) and 7B8FB0 on the built state.
    for p in reversed(region_ptrs):
        m.call(0x7bca60, p)
    graph = []
    for p in region_ptrs:
        m.call(0x7b8fb0, p)
        category = m.u.reg_read(UC_X86_REG_EAX)
        graph.append({'category': category,
                      'neighbors': [m.r32(m.r32(m.r32(p + 0x74) + 4 * j)) for j in range(m.r32(p + 0x70))]})
    out['graph'] = graph
    return out


def width_reference(case):
    """binary64 margin of the floor() argument used by 7AA180 (not ground truth)."""
    s = case['svars']
    w, h = case['layout']['width'], case['layout']['height']
    v = s['max_rectangle_width_svar'] * math.pow((h + w) * 0.5 / s['base_map_width'],
                                                  b2f(s['max_rectangle_width_exp_bits'])) + 0.5
    return math.floor(v), abs(v - round(v))


def run_case(case, full_tiles=True, limit=0):
    # Standalone 7AA180 on a separate machine (its SVar guard is process-static).
    m = Machine(case['precision'], case['svars'])
    setup(m, case)
    width = m.call(0x7aa180)
    width = struct.unpack('<i', struct.pack('<I', width))[0]
    reference, margin = width_reference(case)
    assert width == reference and margin > 1e-6, (width, reference, margin)
    case['params']['max_rectangle_width'] = width
    m = Machine(case['precision'], case['svars'])
    grid, tiles, sai, flag = setup(m, case)
    started = time.monotonic()
    result = m.call(0x7acb00, sai, count=limit)
    if result is None:
        return {'max_rectangle_width_7aa180': width, 'terminated': False, 'instruction_limit': limit}
    native = extract(m, case, grid, tiles, sai, flag, full_tiles)
    native['max_rectangle_width_7aa180'] = width
    native['width_binary64_margin'] = margin
    native['terminated'] = True
    native['warnings'] = [e for e in m.events if 'warning_line' in e]
    native['log_lines'] = [e['log_line'] for e in m.events if 'log_line' in e]
    native['svar_reads'] = m.svar_reads
    print('  native seconds', round(time.monotonic() - started, 3), flush=True)
    return native


def layout(w, h, shift=1):
    return {'width': w, 'height': h, 'coordinate_shift': shift,
            'extent_bits': [f2b(float(w << shift)), f2b(float(h << shift))]}


# terrain codes: L land (flags 0x200 -> type 0), B blocked (0x300 -> 1),
# W water (0x100 -> 2), digits k: definition type k with no flag bits.
def encode(rows):
    table = {'L': tile_code(0x200, 0), 'B': tile_code(0x300, 0), 'W': tile_code(0x100, 0)}
    out = []
    for row in rows:
        for c in row:
            out.append(table[c] if c in table else tile_code(0, int(c)))
    return ''.join(out)


def synthetic():
    rng = random.Random(0x7acb00)
    cases = []

    def add(name, rows, svars=None, cell=2.0, shift=1, **extra):
        h, w = len(rows), len(rows[0])
        s = dict(GAME_SVARS)
        s.update(svars or {})
        params = {k: s[k] for k in ('starting_square_size', 'fast_square_reduction_mult',
                                    'fast_square_reduction_div', 'min_region_size', 'min_region_width',
                                    'max_region_size_for_expansion')}
        params['use_fast_square_reduction'] = bool(s['use_fast_square_reduction'])
        cases.append(dict(name=name, layout=layout(w, h, shift), cell_size_bits=f2b(cell),
                          tiles=encode(rows), svars=s, params=params, **extra))

    add('uniform_land_8x8', ['L' * 8] * 8)
    add('uniform_land_40x30_split', ['L' * 40] * 30)
    rows = [['W'] * 24 for _ in range(20)]
    for y in range(4, 15):
        for x in range(5, 18):
            rows[y][x] = 'L'
    for y in range(9, 12):
        for x in range(19, 22):
            rows[y][x] = 'L'
    rows[2][2] = 'B'
    add('water_with_islands', [''.join(r) for r in rows])
    rows = [['L'] * 30 for _ in range(16)]
    for y in range(16):
        rows[y][14] = rows[y][15] = 'B'
    rows[7][14] = rows[7][15] = 'L'
    add('narrow_passage', [''.join(r) for r in rows])
    rows = [[rng.choice('LLLLWWB') for _ in range(13)] for _ in range(7)]
    add('odd_edges_13x7', [''.join(r) for r in rows])
    rows = [[rng.choice('LLLLLLWWB0123') for _ in range(32)] for _ in range(32)]
    add('noise_32x32_many_regions', [''.join(r) for r in rows])
    rows = [''.join('LW'[(x + y) & 1] for x in range(12)) for y in range(10)]
    add('checkerboard_small_islands', rows)
    # One big region ringed by many single-tile regions: distance hash rehash.
    rows = [['L'] * 36 for _ in range(36)]
    for i in range(36):
        for (x, y) in ((i, 0), (i, 35), (0, i), (35, i)):
            rows[y][x] = 'WB'[(x + y) % 2]
    add('ringed_big_region_hash_growth', [''.join(r) for r in rows],
        svars={'max_rectangle_width_svar': 60, 'starting_square_size': 40})
    rows = [['L'] * 24 for _ in range(18)]
    for y in range(18):
        for x in range(24):
            if (x // 3 + y // 2) % 4 == 0:
                rows[y][x] = 'W'
    add('stripes_slow_reduction', [''.join(r) for r in rows],
        svars={'use_fast_square_reduction': 0, 'starting_square_size': 6})
    add('small_expansion_limit', [''.join(r) for r in rows],
        svars={'max_region_size_for_expansion': 8, 'min_region_size': 12, 'min_region_width': 4})
    other = random.Random(0x7a9b00)
    noise = [''.join(other.choice('LLLWB') for _ in range(17)) for _ in range(13)]
    add('reduction_skips_one_uncovered', noise,
        svars={'fast_square_reduction_mult': 1, 'fast_square_reduction_div': 4, 'starting_square_size': 9})
    add('no_squares', ['LLW', 'WWL'], svars={'starting_square_size': 0})
    rows = [[rng.choice('LLLLLW') for _ in range(20)] for _ in range(20)]
    add('fractional_cell_shift2', [''.join(r) for r in rows], cell=1.7, shift=2)
    add('wide_rectangles_thin_map', ['L' * 70, 'L' * 70, 'W' * 70, 'L' * 70])
    # More than 0x1000 rectangles: only the 404ED0 warning, no cap.
    add('over_0x1000_rectangles_warning', [''.join('LW'[(x + y) & 1] for x in range(66)) for y in range(64)],
        precisions=['Double53'])
    return cases


REAL_MAPS = ['Maps/DeveloperMaps/AI_Test_Map_1.RMP', 'Maps/DeveloperMaps/GBR_TEMPERATE_ART.RMP']


def real_maps(names):
    sys.path.insert(0, str(ROOT / 'research/phase7-navigation'))
    sys.path.insert(0, str(ROOT / 'research/phase6-spatial'))
    import topology_oracle as T
    from original_map_terrain import parse
    source = json.loads((ROOT / 'research/phase6-spatial/native-real-map-terrain.json').read_text())
    types = json.loads((ROOT / 'research/engine-baseline/terrain-variation-metadata.json').read_text())['variations']
    meta = json.loads((ROOT / 'research/phase6-terrain/terrain-render-metadata.json').read_text())
    shipped = json.loads((ROOT / 'research/phase7-navigation/native-topology.json').read_text())['maps']
    out = []
    for path in names:
        row = source['maps'][path]
        chunks = T.read_chunks(ROOT.parent / 'reconstruction/assets/original/Data' / path)
        raw = chunks['TERRAIN'][0]
        version, resolution, biome, palette, records, start, end, _ = parse(raw)
        previous = meta['variants'][meta['biome_defaults'][biome]['variation_id']]
        flags = []
        for index, flag, region, byte in records:
            d = meta['variants'].get(palette[index], previous)
            previous = d
            kind = types[d['canonical']]['native_type']
            f = (flag & ~0x70) | (kind << 4)
            flags.append(f | 0x300 if f & 1 else (f & ~0x300) | (0 if d['land'] else 0x100) | (0 if d['water'] else 0x200))
        assert hashlib.sha256(struct.pack('<' + 'I' * len(flags), *flags)).hexdigest() == row['flags_sha256']
        lay = row['layout']
        s = dict(GAME_SVARS)
        params = {k: s[k] for k in ('starting_square_size', 'fast_square_reduction_mult',
                                    'fast_square_reduction_div', 'min_region_size', 'min_region_width',
                                    'max_region_size_for_expansion')}
        params['use_fast_square_reduction'] = True
        out.append(dict(name='real:' + path, map=path, terrain_sha256=hashlib.sha256(raw).hexdigest(),
                        layout={'width': lay['width'], 'height': lay['height'],
                                'coordinate_shift': lay['coordinate_shift'],
                                'extent_bits': [f2b(float(v)) for v in lay['extent_i32']]},
                        cell_size_bits=lay['cell_size_bits'],
                        tiles=''.join(tile_code(f & 0x300, (f >> 4) & 7) for f in flags),
                        svars=s, params=params, shipped=shipped[path],
                        shipped_topology=decode_saved(raw[end:])))
    return out


def decode_saved(b):
    """The 7AA9B0 TERRAIN topology prefix (same layout as topology.rs)."""
    at = [0]

    def take(fmt):
        v = struct.unpack_from(fmt, b, at[0])
        at[0] += struct.calcsize(fmt)
        return v[0] if len(v) == 1 else list(v)
    assert take('<I') == 0 and take('<B') == 1
    regions, rects = [], []
    for _ in range(take('<I')):
        assert take('<I') == 0
        center = take('<2I')
        regions.append({'position_bits': center, 'distance_pairs': [take('<2I') for _ in range(take('<I'))]})
    for _ in range(take('<I')):
        assert take('<I') == 0
        region, island = take('<i'), take('<i')
        bounds, sides = [], []
        for _ in range(4):
            bounds.append(take('<h'))
            side = []
            for _ in range(take('<I')):
                assert take('<I') == 0
                side.append([take('<4H'), take('<I')])
            sides.append(side)
        rects.append({'region': region, 'island': island, 'bounds': bounds, 'sides': sides})
    return {'regions': regions, 'rectangles': rects}


def shipped_partition(case):
    """Analysis-time tile classes recovered from the shipped topology.

    The builder compares terrain types only for equality, except 7AA2A0 which
    selects type 0. Shipped islands are flood components of equal-type
    rectangles, so one class per island reproduces every equality the original
    analysis made. Islands that are land in the terrain flags get class 0 unless
    adjacent to an island already given 0 (adjacent equal types would have been
    one island); every other island gets its own class.
    """
    topo = case['shipped_topology']
    w, h = case['layout']['width'], case['layout']['height']
    owner = [None] * (w * h)
    for i, r in enumerate(topo['rectangles']):
        y0, x1, y1, x0 = r['bounds']
        for y in range(y0, y1 + 1):
            for x in range(x0, x1 + 1):
                owner[y * w + x] = i
    if None in owner:
        return None
    kinds = [k for _, k in tile_inputs(case)]
    flags = [f for f, _ in tile_inputs(case)]
    terrain = [(1 if f & 0x100 else 0) if f & 0x200 else (2 if f & 0x100 else k) for f, k in zip(flags, kinds)]
    islands = {}
    for t, r in enumerate(owner):
        islands.setdefault(topo['rectangles'][r]['island'], []).append(terrain[t])
    adjacent = {i: set() for i in islands}
    for r in topo['rectangles']:
        for side in r['sides']:
            for _, other in side:
                o = topo['rectangles'][other]['island']
                if o != r['island']:
                    adjacent[r['island']].add(o)
    label = {}
    for island in sorted(islands, key=lambda i: -len(islands[i])):
        land = islands[island].count(0) * 2 > len(islands[island])
        if land and not any(label.get(o) == 0 for o in adjacent[island]):
            label[island] = 0
        else:
            label[island] = 100 + island
    out = {k: v for k, v in case.items() if k not in ('tiles', 'shipped')}
    out['name'] = 'shipped_partition:' + case['map']
    out['tile_definition_types'] = [label[topo['rectangles'][r]['island']] for r in owner]
    return out


def compare_shipped(native, saved, shipped):
    """Compare the rebuilt topology with the serialized topology the map ships."""
    got_rects = [{'region': r['region'], 'island': r['island'], 'bounds': r['bounds'], 'sides': r['sides']}
                 for r in native['rectangles']]
    rect_equal = got_rects == saved['rectangles']
    got = [{'position_bits': r['position_bits'], 'rectangles': r['rectangles'],
            'neighbors': g['neighbors'], 'category': g['category'],
            'distance_cache': {str(k): v for k, v in r['distance_pairs']}}
           for r, g in zip(native['regions'], native['graph'])]
    want = [{'position_bits': r['position_bits'], 'rectangles': r['rectangles'],
             'neighbors': r['neighbors'], 'category': r['category'], 'distance_cache': r['distance_cache']}
            for r in shipped['regions']]
    same = sum(a == b for a, b in zip(got, want))
    order = [r['distance_pairs'] for r in native['regions']] == [r['distance_pairs'] for r in saved['regions']]

    # Load normalization: 7AA9B0 rebuilds region rectangle lists by prepending
    # in rectangle-array order (descending ids); 7BCA60 neighbor order follows
    # that list, and distance-pair stream order is hash-chain order at save.
    def normal(rows):
        return [dict(r, rectangles=sorted(r['rectangles'], reverse=True), neighbors=sorted(r['neighbors']))
                for r in rows]
    normalized = rect_equal and normal(got) == normal(want) and \
        native['tile_rectangles_sha256'] == shipped['tile_rectangles_sha256']
    return {'native_rectangles': len(got_rects), 'shipped_rectangles': len(saved['rectangles']),
            'rectangles_sides_equal': rect_equal,
            'native_regions': len(got), 'shipped_regions': len(want), 'equal_region_records': same,
            'equal_after_load_normalization': normalized,
            'distance_pair_stream_order_equal': order,
            'tile_rectangles_equal': native['tile_rectangles_sha256'] == shipped['tile_rectangles_sha256'],
            'primary_island_equal': native['primary_island'] == shipped['primary_island'],
            'diagonal_equal': native['diagonal_bits'] == shipped['diagonal_bits'],
            'exact': rect_equal and got == want
            and native['tile_rectangles_sha256'] == shipped['tile_rectangles_sha256']}


def rle(values):
    out = []
    for v in values:
        if out and out[-1][0] == v:
            out[-1][1] += 1
        else:
            out.append([v, 1])
    return out


def evidence_one(path):
    case = real_maps([path])[0]
    shipped, saved = case['shipped'], case['shipped_topology']
    part = shipped_partition(case)
    if part is None:
        return path, {'skipped': 'shipped topology leaves tiles uncovered'}
    dims = [max(r['bounds'][1] - r['bounds'][3] + 1, r['bounds'][2] - r['bounds'][0] + 1)
            for r in saved['rectangles']]
    row = {'terrain_sha256': case['terrain_sha256'], 'layout': part['layout'],
           'cell_size_bits': part['cell_size_bits'], 'class_rle': rle(part['tile_definition_types']),
           'largest_shipped_rectangle_side': max(dims), 'attempts': []}

    def attempt(width, precision):
        c = json.loads(json.dumps(part))
        c['precision'] = precision
        if width is not None:
            # pow(x, 0) == 1: the original 7AA180 then returns exactly `width`.
            c['svars'].update(max_rectangle_width_svar=width, max_rectangle_width_exp_bits=0)
        native = run_case(c, full_tiles=False)
        got = compare_shipped(native, saved, shipped)
        got['max_rectangle_width_7aa180'] = native['max_rectangle_width_7aa180']
        return got
    first = attempt(None, 'Double53')
    row['current_formula_width'] = first['max_rectangle_width_7aa180']
    row['attempts'].append(first)
    matched = None
    if first['equal_after_load_normalization']:
        matched = 'current'
    else:
        for width in range(max(dims), max(dims) + 5):
            if width == row['current_formula_width']:
                continue
            got = attempt(width, 'Double53')
            row['attempts'].append(got)
            if got['equal_after_load_normalization']:
                matched = width
                break
    row['matched_width'] = matched
    if matched is not None:
        width = None if matched == 'current' else matched
        row['Double53'] = row['attempts'][-1]
        for p in ('Single24', 'Extended64'):
            row[p] = attempt(width, p)
    return path, row


def shipped_evidence():
    """Every original map: original 7ACB00 on the recovered analysis classes
    versus the topology the map ships, under PC24/PC53/PC64."""
    from multiprocessing import Pool
    source = json.loads((ROOT / 'research/phase6-spatial/native-real-map-terrain.json').read_text())
    started = time.monotonic()
    maps = {}
    with Pool(6) as pool:
        for path, row in pool.imap_unordered(evidence_one, sorted(source['maps'])):
            maps[path] = row
            print(path, row.get('matched_width'), {p: row[p]['equal_after_load_normalization']
                                                   for p in ('Single24', 'Double53', 'Extended64') if p in row},
                  flush=True)
    maps = dict(sorted(maps.items()))
    doc = {'schema': 1, 'original_sha256': ORIGINAL_SHA, 'v14_sha256': V14_SHA,
           'producer_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
           'svars': GAME_SVARS, 'map_count': len(maps), 'maps': maps,
           'scope': 'Original 7ACB00 (same seams as native-topology-builder.json) on one terrain class per '
                    'shipped island (land islands class 0), compared with the TERRAIN topology each '
                    'original map ships. First with the shipped SVars (7AA180 formula), else with '
                    'the smallest fixed rectangle width >= the largest shipped rectangle side that '
                    'reproduces it (StaticMapMaxRectangleWidthExp=0). Load normalization: region '
                    'rectangle lists in descending id order, neighbor sets, distance caches as maps.'}
    out = HERE / 'native-topology-builder-shipped.json'
    out.write_text(json.dumps(doc, separators=(',', ':')) + '\n')
    print(json.dumps({'file': out.name, 'maps': len(maps), 'sha256': hashlib.sha256(out.read_bytes()).hexdigest()}))


def main():
    if '--shipped' in sys.argv:
        return shipped_evidence()
    precisions = ['Single24', 'Double53', 'Extended64']
    cases = []
    started = time.monotonic()
    for base in synthetic():
        for p in base.pop('precisions', precisions):
            case = json.loads(json.dumps(base))
            case['name'] += '_' + p
            case['precision'] = p
            case['native'] = run_case(case)
            cases.append(case)
            print(case['name'], 'rects', len(case['native']['rectangles']), 'regions',
                  len(case['native']['regions']), flush=True)
    # Non-terminating native configuration: the square size never decreases.
    base = synthetic()[0]
    base['name'] = 'fixed_point_reduction_nonterminating'
    base['svars']['fast_square_reduction_mult'] = 4
    base['params']['fast_square_reduction_mult'] = 4
    base['precision'] = 'Double53'
    base['native'] = run_case(base, limit=20_000_000)
    assert not base['native']['terminated']
    cases.append(base)
    real = []
    for case in real_maps(REAL_MAPS):
        shipped, saved = case['shipped'], case['shipped_topology']
        for variant in (case, shipped_partition(case)):
            variant = {k: v for k, v in variant.items() if k not in ('shipped', 'shipped_topology')}
            for p in precisions:
                c = json.loads(json.dumps(variant))
                c['name'] += '_' + p
                c['precision'] = p
                c['native'] = run_case(c, full_tiles=False)
                c['shipped_comparison'] = compare_shipped(c['native'], saved, shipped)
                print(c['name'], c['shipped_comparison'], flush=True)
                real.append(c)
    pins = []
    for a, n in sorted(BLOCKS.items()):
        raw = P.range(a, n)
        assert raw == V.range(a, n), hex(a)
        pins.append({'va': hex(a), 'bytes': raw.hex()})
    doc = {'schema': 1, 'original_sha256': ORIGINAL_SHA, 'v14_sha256': V14_SHA,
           'producer_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
           'scope': 'Complete original 7ACB00 static map analysis on cleared SAI maps, then original '
                    '7BCA60/7B8FB0 per region. Seams: allocator/free, SVar lookup, log/timer, assertion.',
           'seams': ['6F7810', '6F7430', '4B2E60', '404F60', '404ED0', '59AA70', '59AB60', '59ABA0', '855D30'],
           'case_count': len(cases), 'cases': cases, 'real_map_count': len(real), 'real_maps': real,
           'native_basic_block_pins': pins}
    out = HERE / 'native-topology-builder.json'
    out.write_text(json.dumps(doc, separators=(',', ':')) + '\n')
    fixture_sha = hashlib.sha256(out.read_bytes()).hexdigest()
    print(json.dumps({'file': out.name, 'cases': len(cases), 'real': len(real), 'blocks': len(pins),
                      'sha256': fixture_sha, 'seconds': round(time.monotonic() - started, 1)}))
    write_findings(cases, real, fixture_sha)


def write_findings(cases, real, fixture_sha):
    shipped_path = HERE / 'native-topology-builder-shipped.json'
    shipped = None
    if shipped_path.exists():
        doc = json.loads(shipped_path.read_text())
        rows = doc['maps'].values()
        shipped = {
            'file': shipped_path.name,
            'sha256': hashlib.sha256(shipped_path.read_bytes()).hexdigest(),
            'producer_sha256': doc['producer_sha256'],
            'maps': doc['map_count'],
            'reproduced_with_current_7aa180_width': sum(r.get('matched_width') == 'current' for r in rows),
            'reproduced_with_fixed_width': {str(w): sum(r.get('matched_width') == w for r in rows)
                                            for w in sorted({r.get('matched_width') for r in rows}
                                                            - {'current', None})},
            'not_reproduced': [p for p, r in doc['maps'].items() if r.get('matched_width') is None],
            'equal_after_load_normalization': {
                p: sum(bool(r.get(p, {}).get('equal_after_load_normalization')) for r in rows)
                for p in ('Single24', 'Double53', 'Extended64')},
        }
    findings = {
        'schema': 1, 'original_sha256': ORIGINAL_SHA, 'v14_sha256': V14_SHA,
        'producer_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        'fixture': {'file': 'native-topology-builder.json', 'sha256': fixture_sha,
                    'synthetic_cases': len(cases), 'real_map_cases': len(real),
                    'terminated': sum(c['native']['terminated'] for c in cases + real)},
        'shipped_evidence': shipped,
        'rust': {'module': 'crates/aa-navigation/src/topology_builder.rs',
                 'test': 'crates/aa-navigation/tests/topology_builder_native.rs'},
        'ported': {
            '7ACB00': 'StaticMapAnalysis: square loop, then 7AB7A0, 7AA2A0, diagonal, 7AB320, 7BC590 descending',
            '7A9B00': 'next square size (fast mult/div idiv or size-1)',
            '7A9DF0': 'square free and uniform', '7A9EA0': 'alternating column/row growth',
            '7A9AC0/54ECE0': 'tile terrain type', '7ABD40': 'split by max width, publish tiles, region or small list',
            '7BAC90': 'rectangle ctor (bounds +C minY,+E maxX,+10 maxY,+12 minX)',
            '7AACD0/7B9B80': 'side runs above(0) below(2) left(3) right(1), head insertion',
            '7BA4F0/7B99B0': 'region ctor (center -1,-1; 11-bucket hash) / head-insert rectangle',
            '7AB7A0/7AA8B0': 'island flood; recursion -> explicit stack, same order',
            '7AA2A0': 'primary land island by binary32-stored area', '7ACD43': 'diagonal (Topology::diagonal_bits)',
            '7AB320/7B9900/7B8E50/50C510': 'small-rectangle merge (cyclic list)',
            '7BC590/54EDC0/7BA3C0/606750': 'region center and distance hash with rehash'},
        'explicit_inputs': {
            '7AA180': 'max rectangle width (CRT _CIpow 96D890 + floor 96D5E0 + _ftol2 96CC4C); not ported; '
                      'Unicorn emulates FYL2X/F2XM1 through binary64, fixtures check a binary64 margin',
            'SVars': 'StaticMapStartingSquareSize, UseFastSquareReduction, FastSquareReductionMult/Div, '
                     'MinRegionSize, MinRegionWidth, MaxRegionSizeForExpansion (4B2E60 lookups)'},
        'seams': {'6F7810/6F7430': 'allocator', '4B2E60': 'SVar pointer lookup -> explicit parameters',
                  '404F60/404ED0': 'diagnostic log text', '59AA70/59AB60/59ABA0': 'QPC timer for log text only',
                  '855D30': 'assertion dialog (never reached with valid grids; Rust returns Error::Native)'},
        'timing': 'No yield, budget or time-dependent branch: the timer only feeds the [time]/[elapsed] log text.',
        'caller_duties': {
            '7AC9A0': 'ClearStaticMapAnalysis precondition (cleared SAI map, tile +10 null). External teardown '
                      'not ported: 79FF10 when SAI+5 set (before islands freed); 5D85C0(0) when B3974C and '
                      '[B3974C+18C] non-null (after islands freed, before regions freed).',
            '7ACDAA': 'native sets SAI map +4 = 1 and, when B36518 non-null, byte [B36518]+101 = 1'},
        'removed_caps': [
            {'address': '7AA8B0', 'original': 'native recursion on the 0x100000-byte main thread stack '
                                              '(PE SizeOfStackReserve)', 'now': 'explicit heap stack'}],
        'not_caps': ['7ACD81: 0x1000 rectangles is only a 404ED0 warning (rectangle_count_warning)',
                     'arrays grow by 4849F0 doubling; hash tables rehash at 606750; no fixed pools'],
        'retained_limits': ['rectangle bounds are native signed 16-bit (+C..+12, saved format); grids wider or '
                            'taller than 32767 tiles are rejected as Error::Input instead of wrapping'],
        'non_termination': ['7ACB00 when the 7A9B00 size sequence revisits a value',
                            '7AB320 when a full cycle of the small list removes nothing'],
        'fpu': 'All three precisions replayed. PC24 differs from PC53/PC64 in many cases; no generated case '
               'distinguishes PC53 from PC64 (binary32 operands make those intermediates exact).',
    }
    out = HERE / 'native-topology-builder-findings.json'
    out.write_text(json.dumps(findings, indent=1) + '\n')
    print(json.dumps({'file': out.name, 'sha256': hashlib.sha256(out.read_bytes()).hexdigest()}))


if __name__ == '__main__':
    main()
