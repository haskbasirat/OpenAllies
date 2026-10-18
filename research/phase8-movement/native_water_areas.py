"""Original water-area flood fill and map water-area painting (Map.cpp / FloodFill.cpp).

Executes UNCHANGED original instructions (harness.py: v14 byte check, x87 stack
and nonvolatile-register checks, null page traps) on native-layout memory:
GWorld (global 0xB36534) +0x30 GMap, +0x34 base heightfield, +0x240 flood
solver built by the original constructor 0x539D60.

Routines exercised:
  0x539E30 KKC_FloodFillWaterAreaGoal constructor + solve (goal vtable 0xA36058,
           solver vtable 0xA36048, FIFO open list vtable 0xA3602C),
  0x4F4050 generic flood solver core, iterator 0x554ED0/0x4F2000/0x4F2760/0x4F14A0,
           reach 0x539B10/0x53C630, visited 0x53C430, base height 0x539B70/0x53C410,
           IsIndexBlocked 0x539C40 (0x54EDC0 + 0x550190), paint 0x539C00 ->
           0x552150/0x552110 -> 0x550F20 -> 0x5507C0, finish 0x539AD0,
  0x552440 GMap visit stamp, 0x552C90 free water-area id, 0x42A530 tile at point,
  0x5559F0 stroke + flood trigger (batch 0x559980/0x554480 run natively),
  0x555C10 KKC_GMap::PaintTerrain water/land dispatch.

Seams (recorded, not executed): 0x404F60 diagnostic logger; 0x5526C0 per-step
terrain stamp and 0x559FB0 batch commit (terrain brush subsystem); 0x5528F0
brush margin, 0x551140 water-surface paint and 0x551B40 height paint
(heightfield brush subsystem). Real maps: original RMP TERRAIN/HEIGHT_MAP data.
"""
import copy, json, random, struct, sys, hashlib
from pathlib import Path
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from harness import Machine, write_document, f2b, b2f, NullDereference, ROOT, GAME  # noqa: E402
from unicorn import UC_HOOK_MEM_READ, UC_HOOK_MEM_WRITE  # noqa: E402
from unicorn.x86_const import UC_X86_REG_EIP  # noqa: E402

WORLD_GLOBAL = 0xb36534
BUSY = 0xb3d43d
TILE = 0x30
PRECISIONS = ['Single24', 'Double53', 'Extended64']
JUNK = 0xa5
SVARS = 0xb39ab0


def d2f_bits(lo, hi):
    """Exact binary32 bits of a logged double (asserts it is a widened float)."""
    v = struct.unpack('<d', struct.pack('<II', lo, hi))[0]
    b = f2b(v)
    assert struct.unpack('<d', struct.pack('<d', b2f(b)))[0] == v or v != v, (lo, hi)
    return b


class Water(Machine):
    def __init__(self, precision, inp, behaviour=None):
        super().__init__(precision)
        self.inp = inp
        self.diags = []
        self.calls = []
        self.behaviour = behaviour or {}
        self.seams[0x404f60] = self.diag
        self.seams[0x5526c0] = self.stamp_seam
        self.seams[0x559fb0] = self.commit_seam
        self.seams[0x5528f0] = self.margin_seam
        self.seams[0x551140] = self.surface_seam
        self.seams[0x551b40] = self.height_seam
        # The harness leaves address 0..3 open for SEH fs:[0] (FS base 0). Trap
        # every other access there too (e.g. a null area record's id byte).
        self.u.hook_add(UC_HOOK_MEM_READ | UC_HOOK_MEM_WRITE, self._low_null, begin=0, end=3)
        self.build(inp)

    def _low_null(self, u, access, address, size, value, _):
        pc = u.reg_read(UC_X86_REG_EIP)
        if u.mem_read(pc, 1)[0] != 0x64:  # fs: segment prefix (SEH chain)
            raise NullDereference(hex(address))

    # ---- seams -------------------------------------------------------------
    def diag(self, m):
        sp = self.esp()
        line = self.r32(sp + 0xc)
        if line == 0x6f:
            v = self.r32(sp + 0x18, 10)
            self.diags.append({'kind': 'spread_blocked', 'area': v[0], 'x': d2f_bits(v[1], v[2]),
                               'y': d2f_bits(v[3], v[4]), 'neighbour_area': v[5],
                               'height': d2f_bits(v[6], v[7]), 'base': d2f_bits(v[8], v[9])})
        elif line == 0x68a:
            self.diags.append({'kind': 'no_free_area'})
        else:
            raise AssertionError(hex(line))
        self.ret(0)

    def point(self, p):
        return [self.r32(p), self.r32(p + 4)]

    def stamp_seam(self, m):
        a = [self.arg(i) for i in range(6)]
        batch = a[4]
        self.calls.append({'op': 'stamp', 'brush': int(a[0] == self.brush), 'a1': a[1], 'point': self.point(a[2]),
                           'a4': 'variation' if a[3] == self.variation else a[3],
                           'batch_stamp': self.r32(batch + 4), 'batch_flag': self.r8(batch + 0xa), 'a7': a[5]})
        self.ret(0x18)

    def commit_seam(self, m):
        batch = self.ecx()
        self.calls.append({'op': 'commit', 'a5': self.arg(0), 'batch_stamp': self.r32(batch + 4),
                           'batch_flag': self.r8(batch + 0xa)})
        self.ret(4)

    def margin_seam(self, m):
        self.calls.append({'op': 'water_margin', 'margin': self.arg(1)})
        self.ret(8)

    def surface_seam(self, m):
        d = self.arg(4)
        self.calls.append({'op': 'water_surface', 'area': self.r8(d + 0x20), 'point': self.point(self.arg(2))})
        self.ret(0x14, int(self.behaviour.get('surface', 0)))

    def height_seam(self, m):
        self.calls.append({'op': 'height_paint', 'point': self.point(self.arg(2))})
        self.ret(0x14)

    # ---- memory ------------------------------------------------------------
    def build(self, inp):
        mp = inp['map']
        W, H = mp['width'], mp['height']
        self.W, self.H = W, H
        rng = random.Random(inp.get('junk_seed', 1))
        self.world = self.alloc(0x300)
        self.w32(WORLD_GLOBAL, self.world)
        g = self.gmap = self.alloc(0x60)
        self.w32(g + 4, mp['shift'], mp['cell'], mp['extent'][0], mp['extent'][1])
        self.w32(g + 0x1c, W, H)
        n = W * H
        tiles = self.tiles = self.alloc(n * TILE)
        self.u.mem_write(tiles, bytes(rng.randrange(256) for _ in range(n * TILE)) if n * TILE <= 1 << 16
                         else bytes([JUNK]) * (n * TILE))
        self.w32(g + 0x28, tiles)
        self.w32(g + 0x48, mp['stamp'])
        table = self.table = self.alloc(255 * 4)
        self.w32(g + 0x50, table, 255, 255)
        self.records = {}
        for i, rec in enumerate(mp['areas']):
            if rec is None:
                continue
            p = self.alloc(8)
            self.w8(p, rec[0])
            self.w32(p + 4, rec[1])
            self.w32(table + 4 * i, p)
            self.records[i] = p
        variations = {}
        for i, (cls, link, stamp, area, came) in enumerate(mp['tiles']):
            t = tiles + i * TILE
            if cls is None:
                var = 0
            else:
                if cls not in variations:
                    typ = self.alloc(0x80)
                    self.w32(typ + 0x1c, cls)
                    var = self.alloc(0x20)
                    self.w32(var + 0x1c, typ)
                    variations[cls] = var
                var = variations[cls]
            self.w32(t, var)
            self.w32(t + 0x24, 0 if link is None else tiles + link * TILE, stamp)
            self.w8(t + 0x2c, area)
            self.w8(t + 0x2d, came)
        self.before_tiles = bytes(self.u.mem_read(tiles, n * TILE))
        hf = inp['field']
        self.hf = self.alloc(0x50)
        heights = self.alloc(4 * len(hf['heights']))
        self.w32(heights, *hf['heights'])
        vb = self.vbytes = self.alloc(len(hf['vertices']))
        self.u.mem_write(vb, bytes(hf['vertices']))
        self.w32(self.hf + 0xc, hf['width'], hf['height'], hf['shift'], hf['cell'], hf['bias'])
        self.w32(self.hf + 0x40, heights, vb)
        self.w32(self.world + 0x30, g, self.hf)
        solver = self.solver = self.alloc(0x40)
        self.w32(self.world + 0x240, solver)
        self.call(0x539d60, solver)
        assert self.r32(solver + 4) == g and self.r32(solver + 8) == solver + 0x34
        s = inp['solver']
        self.w32(solver + 0x38, *[0 if v is None else tiles + v * TILE for v in (s['tail'], s['head'])])
        self.w8(BUSY, int(s['busy']))
        self.brush = 0
        self.variation = None
        # **(SVars 0xB39AB0 +0x18): one binary32 copied into 0x551140's descriptor.
        svars, slot, value = self.alloc(0x20), self.alloc(4), self.alloc(4)
        self.w32(value, f2b(0.5))
        self.w32(slot, value)
        self.w32(svars + 0x18, slot)
        self.w32(SVARS, svars)

    def tile_index(self, p):
        if p == 0:
            return None
        off = p - self.tiles
        assert off % TILE == 0 and 0 <= off < self.W * self.H * TILE, hex(p)
        return off // TILE

    def dump(self):
        n = self.W * self.H
        raw = bytes(self.u.mem_read(self.tiles, n * TILE))
        tiles = []
        for i in range(n):
            t = raw[i * TILE:(i + 1) * TILE]
            b = self.before_tiles[i * TILE:(i + 1) * TILE]
            # Only +0x24 link, +0x28 stamp, +0x2C area and +0x2D direction may change.
            assert t[:0x24] == b[:0x24] and t[0x2e:] == b[0x2e:], i
            link, stamp = struct.unpack_from('<II', t, 0x24)
            tiles.append([self.tile_index(link), stamp, t[0x2c], t[0x2d]])
        areas = []
        for i, rec in enumerate(self.inp['map']['areas']):
            if rec is None:
                assert self.r32(self.table + 4 * i) == 0
                areas.append(None)
            else:
                p = self.records[i]
                assert self.r32(self.table + 4 * i) == p
                areas.append([self.r8(p), self.r32(p + 4)])
        hf = self.inp['field']
        return {
            'stamp': self.r32(self.gmap + 0x48), 'tiles': tiles, 'areas': areas,
            'vertices': list(self.u.mem_read(self.vbytes, len(hf['vertices']))),
            'solver': {'busy': bool(self.r8(BUSY)), 'tail': self.tile_index(self.r32(self.solver + 0x38)),
                       'head': self.tile_index(self.r32(self.solver + 0x3c))},
            'diagnostics': self.diags, 'calls': self.calls,
        }

    def guarded(self, fn):
        try:
            result = fn()
            assert self.fpu_top() == 0, 'x87 stack unbalanced'
            out = {'result': result}
        except NullDereference as e:
            out = {'error': 'null_dereference', 'address': str(e)}
        out.update(self.dump())
        return out


# ---- operations -------------------------------------------------------------
def run_flood(m, inp, count=2_000_000):
    rec = m.r32(m.table + 4 * inp['area'])
    return m.guarded(lambda: m.call(0x539e30, m.solver, (inp['start'][0], inp['start'][1], rec), count))


def run_stamp(m, inp):
    return m.guarded(lambda: m.call(0x552440, m.gmap))


def run_allocate(m, inp):
    return m.guarded(lambda: m.call(0x552c90, m.gmap) & 0xff)


def run_tile_at(m, inp):
    return m.guarded(lambda: m.tile_index(m.call(0x42a530, m.gmap, tuple(inp['point']))))


def void(m, entry, ecx, args):
    m.call(entry, ecx, args)
    return None


def run_paint(m, inp):
    rec = m.r32(m.table + 4 * inp['area'])
    entry = 0x552110 if inp['paint_op'] == 'add' else 0x552150
    return m.guarded(lambda: void(m, entry, rec, (m.tiles + inp['tile'] * TILE,)))


def brush_block(m, b):
    p = m.alloc(0x20)
    m.w32(p, b['kind'], b['w04'], 0, 0, b['w10'], b['w14'])
    return p


def run_stroke(m, inp):
    s = inp['stroke']
    brush = m.brush = brush_block(m, s['brush'])
    a = m.alloc(16)
    m.w32(a, *s['from'], *s['to'])
    args = (brush, s['a1'], a, a + 8, s['a4'], s['a5'], s['area'], s['a7'], s['a8'])
    return m.guarded(lambda: void(m, 0x5559f0, m.gmap, args))


def run_paint_terrain(m, inp):
    s = inp['paint']
    brush = m.brush = brush_block(m, s['brush'])
    a = m.alloc(16)
    m.w32(a, *s['point'], *s['end'])
    typ = m.alloc(0x80)
    m.w32(typ + 0x1c, s['class'])
    m.w32(typ + 0x5c, f2b(1.5), f2b(0.75), f2b(2.0))
    m.w32(typ + 0x70, f2b(0.25))
    var = m.variation = m.alloc(0x20)
    m.w32(var + 0x1c, typ)
    args = (brush, s['a1'], a, a + 8, var, s['a5'], s['factor'], s['a9'], s['a10'], s['a7'], s['a8'])
    return m.guarded(lambda: void(m, 0x555c10, m.gmap, args))


OPS = {'flood': run_flood, 'stamp': run_stamp, 'allocate': run_allocate, 'tile_at': run_tile_at,
       'paint': run_paint, 'stroke': run_stroke, 'paint_terrain': run_paint_terrain}


# ---- inputs -----------------------------------------------------------------
def field_for(W, H, rng, hs=1, gs=1, levels=None, extra=2, bias=None):
    hw = ((W << gs) >> hs) + extra
    hh = ((H << gs) >> hs) + extra
    heights = [f2b(rng.choice(levels or [0.0])) for _ in range(hw * hh)]
    return {'width': hw, 'height': hh, 'shift': hs, 'cell': f2b(float(1 << hs)),
            'bias': (1 << hs) >> 1 if bias is None else bias, 'heights': heights,
            'vertices': [0] * (hw * hh)}


def areas_from(tiles, overrides=None, missing=()):
    counts = [0] * 255
    for t in tiles:
        if 0 < t[3] < 255:
            counts[t[3]] += 1
    out = [None] + [[i, counts[i]] for i in range(1, 255)]
    for i, c in (overrides or {}).items():
        out[i] = [i, c]
    for i in missing:
        out[i] = None
    return out


def recompute_vertices(field, tiles, W, H, gs):
    """Initial vertex bytes: native 0x5507C0 rule applied to every vertex (input only)."""
    hw, hh, hs = field['width'], field['height'], field['shift']
    v = field['vertices']
    for vy in range(hh):
        for vx in range(hw):
            tx, ty = (vx << hs) >> gs, (vy << hs) >> gs
            val = 0
            for ox in (-1, 0):
                for oy in (-1, 0):
                    x, y = tx + ox, ty + oy
                    if 0 <= x < W and 0 <= y < H:
                        c = tiles[y * W + x][3]
                        if c and val != c:
                            if val:
                                val = 0xff
                            else:
                                val = c
                if val == 0xff:
                    break
            v[vy * hw + vx] = val


def base_map(W, H, shift=1, cell=None, stamp=7):
    ext = float(W << shift)
    return {'width': W, 'height': H, 'shift': shift,
            'cell': f2b(float(1 << shift) if cell is None else cell),
            'extent': [f2b(float(W << shift)), f2b(float(H << shift))], 'stamp': stamp, 'tiles': [], 'areas': []}


def grid(rows, W=None):
    """rows of chars: '~' water, '.' land, '#' class0, 'n' null variation; digits = water with area."""
    tiles = []
    for row in rows:
        for ch in row:
            if ch == '~':
                tiles.append([2, None, 0, 0, 0])
            elif ch == '.':
                tiles.append([1, None, 0, 0, 0])
            elif ch == '#':
                tiles.append([0, None, 0, 0, 0])
            elif ch == 'n':
                tiles.append([None, None, 0, 0, 0])
            elif ch.isdigit():
                tiles.append([2, None, 0, int(ch), 0])
            elif ch.isalpha():
                tiles.append([1, None, 0, ord(ch) - ord('a') + 1, 0])
            else:
                raise ValueError(ch)
    return tiles


def flood_case(name, rows, start, area, rng, levels=None, heights=None, stamp=7, busy=False, overrides=None,
               missing=(), shift=1, hs=1, cell=None, vertex_recompute=True, head_tail=(None, None), tweak=None):
    H, W = len(rows), len(rows[0])
    mp = base_map(W, H, shift, cell, stamp)
    mp['tiles'] = grid(rows)
    for t in mp['tiles']:
        t[2] = rng.choice([0, 1, stamp - 1 if stamp else 0, 0x1234])
        t[4] = rng.randrange(9)
        t[1] = rng.choice([None, rng.randrange(W * H)])
    mp['areas'] = areas_from(mp['tiles'], overrides, missing)
    field = field_for(W, H, rng, hs, shift, levels)
    if heights:
        for (vx, vy), h in heights.items():
            field['heights'][vy * field['width'] + vx] = f2b(h) if isinstance(h, float) else h
    if vertex_recompute:
        recompute_vertices(field, mp['tiles'], W, H, shift)
    inp = {'op': 'flood', 'map': mp, 'field': field, 'start': list(start), 'area': area,
           'solver': {'busy': busy, 'tail': head_tail[1], 'head': head_tail[0]}, 'junk_seed': rng.randrange(1 << 30)}
    if tweak:
        tweak(inp)
    return {'name': name, 'input': inp}


THR = 0x3dcccccd  # 0x9FE0C0 binary32 0.1


def tile_height(field, x, y, value):
    """Tile (x,y) centre sample with shift1/bias1 reads vertex (x+1,y+1)."""
    field['heights'][(y + 1) * field['width'] + (x + 1)] = value


def handmade(rng):
    cases = []
    lake = ['......',
            '.~~~..',
            '.~~~~.',
            '..~~..',
            '......']
    cases.append(flood_case('single_lake_new_area', lake, (2, 2), 1, rng))
    cases.append(flood_case('single_lake_start_on_land', lake, (0, 0), 3, rng))
    cases.append(flood_case('start_corner_water_edges', ['~~~.', '~~..', '~...'], (0, 0), 4, rng))
    cases.append(flood_case('full_water_edges', ['~~~~~', '~~~~~', '~~~~~', '~~~~~'], (4, 3), 9, rng))
    cases.append(flood_case('two_lakes_separated', ['~~.~~', '~~.~~', '~~.~~'], (0, 1), 2, rng))
    cases.append(flood_case('diagonal_only_connection', ['~..', '.~.', '..~'], (0, 0), 5, rng))
    cases.append(flood_case('same_area_refill', ['1111', '11.1', '1111'], (1, 1), 1, rng))
    cases.append(flood_case('start_other_area_repaint', ['2222', '22.2', '2222'], (0, 0), 6, rng))
    cases.append(flood_case('class0_and_land_block', ['~#~~', '~.#~', '~~~#'], (0, 0), 7, rng))
    # Neighbouring area within / beyond the 0.1 threshold (heights per tile centre sample).
    for name, hv in [('takeover_equal_height', 0.0), ('takeover_at_threshold', THR),
                     ('blocked_above_threshold', 0x3dccccce), ('blocked_far_below', -3.5),
                     ('takeover_negative_threshold', THR | 0x80000000)]:
        c = flood_case(name, ['~~~33', '~~~33', '~~~33'], (0, 1), 8, rng)
        f = c['input']['field']
        f['heights'] = [f2b(0.0)] * len(f['heights'])
        for y in range(3):
            for x in (3, 4):
                tile_height(f, x, y, hv if isinstance(hv, int) else f2b(hv))
        cases.append(c)
    # Precision-sensitive difference: h = 0.1f+ulp, base = 5*2^-30 -> PC24 rounds to 0.1f (not blocked).
    for name, h, base in [('pc_sensitive_positive', THR + 1, 0x31a00000), ('pc_sensitive_negative', 0x31a00000, THR + 1),
                          ('pc_sensitive_large', 0x4b800001, 0x3e800000)]:
        c = flood_case(name, ['~~44', '~~44'], (0, 0), 9, rng)
        f = c['input']['field']
        f['heights'] = [base] * len(f['heights'])
        for y in range(2):
            for x in (2, 3):
                tile_height(f, x, y, h)
        cases.append(c)
    # Non-finite heights: unordered compares never block, infinities do.
    for name, base, h in [('nan_base_spreads', 0x7fc00000, f2b(5.0)), ('snan_neighbour_spreads', 0, 0x7f800001),
                          ('inf_neighbour_blocks', 0, 0x7f800000), ('neg_inf_neighbour_blocks', 0, 0xff800000),
                          ('inf_minus_inf_spreads', 0x7f800000, 0x7f800000),
                          ('inf_opposite_signs_block', 0xff800000, 0x7f800000)]:
        c = flood_case(name, ['~~33', '~~33'], (0, 0), 5, rng)
        f = c['input']['field']
        f['heights'] = [base] * len(f['heights'])
        for y in range(2):
            for x in (2, 3):
                tile_height(f, x, y, h)
        if name.startswith('snan'):
            # docs/NATIVE-PORT-RULES.md: mark rows whose inputs are signalling NaNs.
            c['emulator_caveat'] = ('signalling NaN height: Unicorn 2.1.4 does not quiet SNaN on FLD m32 (hardware '
                                    'does); only the unordered compare is observable in this row, and it is the same '
                                    'on hardware, so the recorded result stands; Rust follows hardware')
        cases.append(c)
    # Mixed areas: partial takeover, vertex conflicts and recompute.
    cases.append(flood_case('three_areas_mixed', ['1~~2', '~~~2', '3~~2', '33~~'], (1, 1), 1, rng))
    cases.append(flood_case('busy_guard', lake, (2, 2), 1, rng, busy=True))
    cases.append(flood_case('stamp_wraps', lake, (2, 2), 1, rng, stamp=0x7ffffffe))
    cases.append(flood_case('stamp_large_unsigned', lake, (2, 2), 1, rng, stamp=0xfffffffe))

    def previsit(inp):
        st = inp['map']['stamp'] + 1
        W = inp['map']['width']
        for (x, y) in [(2, 1), (3, 2)]:
            inp['map']['tiles'][y * W + x][2] = st
    cases.append(flood_case('previsited_tiles', lake, (1, 1), 2, rng, tweak=previsit))
    cases.append(flood_case('null_area_record', lake, (2, 2), 0, rng))
    cases.append(flood_case('missing_record_neighbour', ['~5~', '~~~'], (0, 0), 1, rng, missing=(5,)))
    cases.append(flood_case('null_variation_neighbour', ['~~n', '~~~'], (0, 0), 1, rng))
    cases.append(flood_case('null_variation_start_only', ['n..', '...'], (0, 0), 1, rng))
    cases.append(flood_case('nonnormal_start_wraps_row', ['~~~~', '~~~~', '....'], (-1, 1), 2, rng))
    cases.append(flood_case('nonnormal_start_past_row', ['~~~~', '~~~~', '....'], (4, 0), 2, rng))
    cases.append(flood_case('stale_queue_reset', lake, (2, 2), 1, rng, head_tail=(5, 9)))
    cases.append(flood_case('counts_wrap', ['77', '~~'], (0, 1), 1, rng,
                            overrides={7: -2147483648, 1: 2147483647}))
    cases.append(flood_case('land_area_ids_ignored', ['~ab~', '~~~~'], (0, 0), 3, rng))
    # Odd cell size: tile centres need binary32 rounding (diagnostic X/Y bits).
    c = flood_case('odd_cell_size', ['~~~22', '~~~22'], (0, 0), 3, rng, cell=1.7000000476837158)
    f = c['input']['field']
    f['heights'] = [f2b(0.0)] * len(f['heights'])
    for y in range(2):
        for x in (3, 4):
            tile_height(f, x, y, f2b(1.0))
    cases.append(c)
    # Different heightfield/vertex resolution (shift 0 and 2) and GMap shift 2.
    cases.append(flood_case('vertex_shift0', ['~~2', '~~2', '1~~'], (1, 1), 4, rng, hs=0))
    cases.append(flood_case('vertex_shift2_gmap2', ['~~2~', '~~2~', '1~~~'], (1, 1), 4, rng, shift=2, hs=2))
    cases.append(flood_case('gmap_shift3_vertex1', ['~~2', '~~2', '1~~'], (0, 0), 4, rng, shift=3, hs=1))
    return cases


def random_cases(rng, count):
    cases = []
    for k in range(count):
        W, H = rng.randint(3, 14), rng.randint(3, 12)
        rows = []
        water = rng.random() * 0.6 + 0.3
        ids = [rng.randint(1, 12) for _ in range(rng.randint(0, 4))]
        for y in range(H):
            row = ''
            for x in range(W):
                r = rng.random()
                if r < water:
                    row += str(rng.choice(ids)) if ids and rng.random() < 0.35 and max(ids) < 10 else '~'
                elif r < water + 0.03:
                    row += '#'
                else:
                    row += '.'
            rows.append(row)
        sx, sy = rng.randrange(W), rng.randrange(H)
        area = rng.choice([1, 2, 3, rng.randint(1, 254)])
        levels = rng.choice([[0.0], [0.0, 0.05, 0.1, -0.1, 0.25], [1.0, 1.05, 1.1000000238418579, 0.9]])
        c = flood_case('random_%d' % k, rows, (sx, sy), area, rng, levels=levels, stamp=rng.choice([7, 100, 3]))
        cases.append(c)
    return cases


def helper_cases(rng):
    cases = []
    lake = ['~~1~', '~2~~', '~~~3']
    for st in [0, 5, 0x7ffffffd, 0x7ffffffe, 0x7fffffff, 0xffffffff]:
        c = flood_case('stamp_%x' % st, lake, (0, 0), 1, rng, stamp=st)
        c['input']['op'] = 'stamp'
        cases.append(c)
    for name, used in [('allocate_first', []), ('allocate_gap', [1, 2, 3, 5]), ('allocate_last', list(range(1, 254))),
                       ('allocate_exhausted', list(range(1, 255))), ('allocate_negative_count', [1])]:
        c = flood_case(name, lake, (0, 0), 1, rng)
        areas = [None] + [[i, 0] for i in range(1, 255)]
        for i in used:
            areas[i] = [i, 3]
        if name == 'allocate_negative_count':
            areas[1] = [1, -1]
        c['input']['map']['areas'] = areas
        c['input']['op'] = 'allocate'
        cases.append(c)
    for pt in [(0.0, 0.0), (5.9, 3.1), (7.999999, 5.5), (2.0, 1.0), (8.5, 0.5), (-0.5, 2.0)]:
        c = flood_case('tile_at_%s_%s' % pt, lake, (0, 0), 1, rng)
        c['input']['op'] = 'tile_at'
        c['input']['point'] = [f2b(pt[0]), f2b(pt[1])]
        cases.append(c)
    mixed = ['1122', '1~~2', '3~~2', '3344']
    for op in ['add', 'remove']:
        for tile, area in [(5, 1), (6, 2), (0, 1), (15, 4), (10, 3), (9, 3), (5, 9)]:
            c = flood_case('paint_%s_%d_%d' % (op, tile, area), mixed, (0, 0), 1, rng)
            c['input'].update({'op': 'paint', 'paint_op': op, 'tile': tile, 'area': area})
            cases.append(c)
    return cases


def stroke_cases(rng):
    cases = []
    lake = ['......', '.~~~..', '.~~~~.', '..~~..', '......']
    base = lambda name, b, f, t, area, **kw: dict(name=name, brush=b, frm=f, to=t, area=area, **kw)
    plans = [
        base('stroke_point', {'kind': 0, 'w04': f2b(1.0), 'w10': 0, 'w14': 0}, (5.0, 5.0), (5.0, 5.0), 1),
        base('stroke_line', {'kind': 1, 'w04': f2b(1.5), 'w10': 0, 'w14': 0}, (2.5, 3.0), (9.25, 6.5), 2),
        base('stroke_min_step', {'kind': 1, 'w04': f2b(0.1), 'w10': 0, 'w14': 0}, (3.0, 3.0), (6.0, 3.5), 0),
        base('stroke_rect_brush', {'kind': 3, 'w04': f2b(9.0), 'w10': f2b(2.25), 'w14': f2b(1.75)}, (1.0, 9.0), (10.0, 1.0), 3),
        base('stroke_rect_equal', {'kind': 2, 'w04': f2b(9.0), 'w10': f2b(2.0), 'w14': f2b(2.0)}, (4.0, 4.0), (4.0, 9.5), 4),
        base('stroke_exact_multiple', {'kind': 1, 'w04': f2b(2.0), 'w10': 0, 'w14': 0}, (1.0, 1.0), (7.0, 1.0), 5),
        base('stroke_third', {'kind': 1, 'w04': f2b(1.0 / 3.0), 'w10': 0, 'w14': 0}, (0.5, 0.5), (11.0, 9.0), 6),
        base('stroke_reverse', {'kind': 0, 'w04': f2b(1.25), 'w10': 0, 'w14': 0}, (11.5, 9.5), (0.25, 0.75), 1),
    ]
    for p in plans:
        c = flood_case(p['name'], lake, (0, 0), 1, rng)
        c['input']['op'] = 'stroke'
        c['input']['stroke'] = {'brush': p['brush'], 'from': [f2b(v) for v in p['frm']],
                                'to': [f2b(v) for v in p['to']], 'a1': 0x11, 'a4': 0x44, 'a5': 0x55,
                                'area': p['area'], 'a7': 0x77, 'a8': rng.randrange(256)}
        cases.append(c)
    return cases


def paint_cases(rng):
    cases = []
    lake = ['......', '.~~~..', '.3~~~.', '..~~..', '......']
    brush = {'kind': 1, 'w04': f2b(1.5), 'w10': 0, 'w14': 0}
    for name, pt, end, cls, surface, exhausted in [
        ('paint_outside_negative', (-0.5, 3.0), (4.0, 4.0), 2, 1, False),
        ('paint_outside_extent', (12.0, 3.0), (4.0, 4.0), 2, 1, False),
        ('paint_land', (3.0, 3.0), (6.0, 5.0), 1, 1, False),
        ('paint_class0', (3.0, 3.0), (6.0, 5.0), 0, 1, False),
        ('paint_water_new_area', (5.0, 3.0), (6.0, 5.0), 2, 1, False),
        ('paint_water_existing_area', (3.0, 4.5), (6.0, 6.0), 2, 1, False),
        ('paint_water_surface_rejects', (5.0, 3.0), (6.0, 5.0), 2, 0, False),
        ('paint_water_exhausted', (5.0, 3.0), (6.0, 5.0), 2, 1, True),
        ('paint_water_exhausted_existing', (3.0, 4.5), (6.0, 6.0), 2, 1, True),
    ]:
        c = flood_case(name, lake, (0, 0), 1, rng)
        if exhausted:
            c['input']['map']['areas'] = [None] + [[i, 1 + (i == 3) * 0] for i in range(1, 255)]
        c['input']['op'] = 'paint_terrain'
        c['input']['paint'] = {'brush': brush, 'point': [f2b(v) for v in pt], 'end': [f2b(v) for v in end],
                               'class': cls, 'a1': 0x11, 'a5': 0x55, 'factor': f2b(0.5), 'a9': 0x99, 'a10': 0,
                               'a7': 0x77, 'a8': 1}
        c['behaviour'] = {'surface': surface}
        cases.append(c)
    return cases


# ---- real maps --------------------------------------------------------------
def real_map_cases():
    sys.path.insert(0, str(ROOT / 'research/phase6-mapheader'))
    sys.path.insert(0, str(GAME / 'reconstruction/tools'))
    from inspect_header import Reader, decode
    from map_extract import read_chunks
    source = GAME / 'reconstruction/assets/original/Data'
    types = json.loads((ROOT / 'research/engine-baseline/terrain-variation-metadata.json').read_text())['variations']
    render = json.loads((ROOT / 'research/phase6-terrain/terrain-render-metadata.json').read_text())
    out = []
    plan = [('Maps/DeveloperMaps/GBR_ARCTIC_ART.RMP', [('existing', 1), ('new', 2)]),
            ('Maps/DeveloperMaps/(1v1)BorealForest.RMP', [('existing', 1), ('existing', 3), ('new', 7)]),
            ('Maps/Campaign/tutorial01.RMP', [('new', 3)]),
            # Water areas touching other areas: height-threshold takeover/blocking.
            ('Maps/(2v2)Kursk.RMP', [('existing', 2), ('existing', 4)]),
            ('Maps/Campaign/allies01_el_alamein.RMP', [('existing', 4)]),
            ('Maps/Campaign/axis02_el_alamein.RMP', [('existing', 8)])]
    for rel, seeds in plan:
        path = source / rel
        data = path.read_bytes()
        ch = read_chunks(path)
        header = decode(ch['MAP_HEADER'][0])
        r = Reader(ch['TERRAIN'][0])
        version, res, biome = r.u32(), r.u32(), r.string()
        palette = [r.string() for _ in range(r.u32())]
        count, stride = r.u32(), r.u32()
        recs = []
        for _ in range(count):
            b = r.take(stride)
            if version == 6:
                index, _, _, byte, _ = struct.unpack('<BBHBB', b)
            else:
                index, _, word, _, _ = struct.unpack('<HHHBB', b)
                byte = word & 255
            recs.append((index, byte))
        initial = render['biome_defaults'][biome]['variation_id']

        def native_type(key):
            if key not in render['variants']:
                return None
            return types[render['variants'][key]['canonical']]['native_type']
        nts = [native_type(k) for k in palette]
        prev = native_type(initial)
        classes = []
        for index, _ in recs:
            t = nts[index]
            if t is None:
                t = prev
            classes.append(t)
            prev = t
        ext = [b2f(x) for x in header['creator_size_bits']]
        W, H = int(ext[0]) >> 1, int(ext[1]) >> 1
        assert W * H == len(recs) and res == 1
        hm = ch['HEIGHT_MAP'][0]
        hv = struct.unpack_from('<6I', hm, 0)
        hw, hh = W + 1, H + 1
        assert hv[0] == 1 and hv[4] == 1 and hv[5] == hw * hh
        heights = list(struct.unpack_from('<%dI' % (hw * hh), hm, 24))
        flags = list(hm[24 + 4 * hw * hh:24 + 5 * hw * hh])
        tiles = [[c, None, 0, b, 0] for c, (_, b) in zip(classes, recs)]
        mp = {'width': W, 'height': H, 'shift': 1, 'cell': f2b(2.0), 'extent': list(header['creator_size_bits']),
              'stamp': 1, 'tiles': tiles, 'areas': areas_from(tiles)}
        field = {'width': hw, 'height': hh, 'shift': 1, 'cell': f2b(2.0), 'bias': 1, 'heights': heights,
                 'vertices': flags}
        for mode, area in seeds:
            # First water tile (row-major) of the requested existing area or of the largest area.
            want = area if mode == 'existing' else max(range(1, 255), key=lambda a: mp['areas'][a][1])
            start = next(i for i, t in enumerate(tiles) if t[0] == 2 and t[3] == want)
            if mode == 'new':
                assert mp['areas'][area][1] == 0
            out.append({'name': 'real_%s_%s_%d' % (Path(rel).stem, mode, area), 'real_map': rel,
                        'map_sha256': hashlib.sha256(data).hexdigest(),
                        'input': {'op': 'flood', 'map': mp, 'field': field, 'start': [start % W, start // W],
                                  'area': area, 'solver': {'busy': False, 'tail': None, 'head': None},
                                  'junk_seed': 0}})
    return out


def input_digest(inp):
    """sha256 over (class i32 or -1, area u8) per tile, heights u32, vertex bytes."""
    h = hashlib.sha256()
    for t in inp['map']['tiles']:
        h.update(struct.pack('<iB', -1 if t[0] is None else t[0], t[3]))
    f = inp['field']
    h.update(struct.pack('<%dI' % len(f['heights']), *f['heights']))
    h.update(bytes(f['vertices']))
    return h.hexdigest()


def compact_real(case, out):
    """Real-map fixtures store tile/vertex changes, not the whole source map."""
    inp = case['input']
    case['input_sha256'] = input_digest(inp)
    before = inp['map']['tiles']
    changed = [[i] + t for i, t in enumerate(out['tiles'])
               if [before[i][1], before[i][2], before[i][3], before[i][4]] != t]
    vchanged = [[i, v] for i, v in enumerate(out['vertices']) if inp['field']['vertices'][i] != v]
    out['tiles_changed'] = changed
    out['vertices_changed'] = vchanged
    out['areas_changed'] = [[i] + a for i, a in enumerate(out['areas'])
                            if a is not None and inp['map']['areas'][i] != a]
    del out['tiles'], out['vertices'], out['areas']
    case['input'] = {'op': 'flood', 'start': inp['start'], 'area': inp['area'], 'stamp': inp['map']['stamp']}


FINDINGS = {
    'ported': {
        '0x539E30': 'KKC_FloodFillWaterAreaGoal ctor+solve: stack goal (vtable 0xA36058; +0xC/+0x10 start, '
                    '+0x14 area record, +0x18 base height) on solver GWorld+0x240 -> water_areas::flood_fill_water_area',
        '0x53C570': 'goal base ctor (vtable 0xA365C8, +4=0, byte +8=0)',
        '0x4F4050': 'generic flood solver core (busy byte 0xB3D43D, reach/queue/pop/expand/neighbour loop); '
                    'goal-constant branches (0x4F42FC early success, solver+4 != 3, push failure) are unreachable '
                    'for this goal',
        '0x539D60': 'flood solver ctor (vtable 0xA36048: +0 0x53F8D0 visit neighbour, +4 0x53F890 expand, '
                    '+8 0x4F0550 reach, +C 0x4F0500 exhausted=1; FIFO at +0x34 vtable 0xA3602C); executed to build '
                    'the oracle solver',
        '0x539AC0/0x539DB0/0x539D20/0x53F5D0/0x539DF0': 'FIFO reset / push (tile +0x24 links, +4 tail, +8 head) / '
                                                        'pop / pointer->cell',
        '0x554ED0/0x4F2000/0x4F2760/0x4F14A0': 'neighbour iterator: origin +0x2D reached-from selects 1..8 (start), '
                                               'd-2..d+2 (diagonal arrival) or d-1..d+1 (orthogonal arrival); skips '
                                               'the reverse direction, out-of-map cells and IsIndexBlocked cells',
        '0x539B10/0x53C630': 'reach: +0x28 = GMap+0x48 stamp, +0x2D = direction from tables 0xAB78BC/0xAB78C8/'
                             '0xAB78D0, +0x24 link cleared',
        '0x53C430': 'visited = (+0x28 == GMap+0x48)',
        '0x539B70/0x53C410/0x552440': 'base height at the start centre (stored binary32) after advancing the GMap '
                                      'visit stamp',
        '0x539C40': 'IsIndexBlocked: class [[tile+0]+0x1C]+0x1C != 2 blocks; another nonzero area blocks only when '
                    '|h - base| > 0.1f (FSUB at PC, FCHS on C0, FCOMP 0x9FE0C0, unordered never blocks); logs '
                    'FloodFill.cpp:0x6F',
        '0x539C00/0x552150/0x552110': 'paint: remove from the old record (area=0 first, count-1) then add '
                                      '(area=id, count+1)',
        '0x54EDC0': 'tile centre binary32 (fild(c<<shift) + cell*0.5)',
        '0x550F20/0x5507C0': 'heightfield +0x44 vertex bytes: add 0->id, other->0xFF; remove rebuilds from the 4 '
                             'surrounding tiles (Y via binary32 store)',
        '0x550190': 'nearest height sample index (as aa-heightfield Heightfield::sample_nearest; raw bits kept for '
                    'unordered compares)',
        '0x552C90': 'first free area id in 1..=254 (count==0) else 0',
        '0x42A530': 'tile at a binary32 point',
        '0x5559F0': 'stroke + flood: batch 0x559980 (advances the visit stamp), segment walk (dy stored, dx extended, '
                    'FSQRT, step = max(brush step, binary32(trunc(cell)*0.5)), cursor stores, end stamped once), '
                    'commit, flood table[area] from the tile of the start point, 0x554480 release',
        '0x555C10': 'KKC_GMap::PaintTerrain dispatch: point in [0,extent) else nothing; class 1 land height stage; '
                    'class 2 area = tile area or 0x552C90, none -> Map.cpp:0x68A diagnostic and stop; margin '
                    'binary32(cell*5.656854+0.0001); surface stage may stop; bed stage; then 0x5559F0',
    },
    'data_written': {
        'GMap+0x48': 'visit stamp (+1, or 0 with all tile stamps cleared when the increment reaches 0x7FFFFFFF '
                     'unsigned)',
        'tile+0x24': 'FIFO link (cleared on reach, next queued tile when another tile is pushed)',
        'tile+0x28': 'visit stamp of reached tiles (blocked tiles are never reached)',
        'tile+0x2C': 'water-area id of painted tiles',
        'tile+0x2D': 'reached-from direction 0..8',
        'record+4': 'tile count (wrapping i32)',
        'heightfield+0x44': 'per-vertex area byte (0 none, id, 0xFF shared)',
        'solver+0x38/+0x3C, 0xB3D43D': 'FIFO tail/head and process-wide busy byte (left set on a fault)',
        'untouched': 'the oracle asserts every other tile byte (+0x00..+0x23, +0x2E/+0x2F) is unchanged; solver '
                     'scratch words +0x10..+0x30 are rewritten each solve and never read across solves',
    },
    'caps': [
        {'address': '0x552CAD (cmp eax,0xFF) / 0x556570 (table capacity 0xFF at GMap+0x58, records 1..254)',
         'value': 254, 'kept': True,
         'why': 'area ids are bytes in tile+0x2C, the saved TERRAIN record and the vertex bytes where 0xFF marks '
                'shared vertices; data semantics, not an engine buffer'},
        {'address': '0x552446 (cmp eax,0x7FFFFFFF)', 'value': '0x7FFFFFFF', 'kept': True,
         'why': 'visit-stamp epoch restart is observable state (all tiles read as visited after a restart)'},
    ],
    'seams': {
        '0x404F60': 'diagnostic logger (no state effect); both messages are recorded and returned as '
                    'water_areas::Diagnostic',
        '0x5526C0': 'terrain-brush stamp per stroke step (StrokeWorld::stamp)',
        '0x559FB0': 'terrain-brush batch commit (StrokeWorld::commit)',
        '0x5528F0': 'brush margin application on the copied brush (PaintTerrainStages::water_margin)',
        '0x551140': 'water-surface heightfield brush; false aborts (PaintTerrainStages::water_surface)',
        '0x551B40': 'height brush on GWorld+0x38 (land_height / water_bed)',
    },
    'unported': [
        '0x5526C0, 0x559FB0, 0x5528F0 (0x46D8D0/0x46D8E0/0x46EB60/0x46E0E0), 0x551140 (0x558CE0/0x551050/0x5504B0/'
        '0x558900/0x559470/0x550430), 0x551B40 (0x551680), 0x551F60, 0x46E0A0: terrain/height brush subsystems '
        'behind boundaries',
        '0x5972F0 random-map caller: not executed; input contract only (records {u8 area +0, i32 x +4, i32 y +8} '
        'at +0x108 data / +0x10C count, flag +0xD4, flood table[area] in list order)',
        '0x597530 random-map rivers (another caller of 0x552C90 / 0x5559F0)',
        'map-load restoration of GMap+0x50 record counts (not located; counts are inputs)',
    ],
    'non_finite': 'Binary is finite-only: NaN/inf heights keep x87 unordered/infinite compare semantics '
                  'explicitly; non-finite stroke steps, cell sizes or coordinates and overflowing truncations return '
                  'explicit errors (the original continues; a NaN step never terminates)',
    'unicorn_caveat': 'only FADD/FSUB/FMUL/FDIV/FSQRT/FILD/FISTP/FCOM(P)/FCHS run (no FPATAN/FSIN/FCOS/FYL2X); '
                      'results are compared after binary32 stores and through branch decisions',
}


def write_findings(digest):
    doc = json.loads((HERE / 'native-water-areas.json').read_text())
    stats = {}
    for c in doc['cases']:
        o = c['output']
        key = 'real_map' if c.get('real_map') else c['input']['op']
        s = stats.setdefault(key, {'cases': 0, 'faults': 0, 'with_diagnostics': 0})
        s['cases'] += 1
        s['faults'] += int('error' in o)
        s['with_diagnostics'] += int(bool(o['diagnostics']))
    real = sorted({(c['real_map'], c['name']) for c in doc['cases'] if c.get('real_map')})
    out = dict(FINDINGS)
    out.update({'schema': 1, 'fixture': 'native-water-areas.json', 'fixture_sha256': digest,
                'producer_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                'precisions': PRECISIONS, 'case_count': doc['case_count'], 'case_stats': stats,
                'real_map_cases': [n for _, n in real],
                'executed_instructions': len(doc['instructions']),
                'rust': {'module': 'crates/aa-spatial/src/water_areas.rs',
                         'tests': ['crates/aa-spatial/tests/water_areas_native.rs',
                                   'crates/aa-spatial/src/water_areas_tests.rs']}})
    path = HERE / 'native-water-areas-findings.json'
    path.write_text(json.dumps(out, indent=1) + '\n')
    print(json.dumps({'file': path.name, 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}))


def main():
    if '--findings-only' in sys.argv:
        write_findings(hashlib.sha256((HERE / 'native-water-areas.json').read_bytes()).hexdigest())
        return
    rng = random.Random(0x3a7e5)
    templates = handmade(rng) + random_cases(rng, 40) + helper_cases(rng) + stroke_cases(rng) + paint_cases(rng)
    cases = []
    for precision in PRECISIONS:
        for t in templates:
            c = copy.deepcopy(t)
            c['precision'] = precision
            inp = c['input']
            op = inp['op']
            m = Water(precision, inp, c.get('behaviour'))
            c['output'] = OPS[op](m, inp)
            cases.append(c)
    real = []
    if '--no-real' not in sys.argv:
        for t in real_map_cases():
            for precision in PRECISIONS:
                c = copy.deepcopy(t)
                c['precision'] = precision
                m = Water(precision, c['input'])
                out = run_flood(m, c['input'], 1_000_000_000)
                compact_real(c, out)
                c['output'] = out
                real.append(c)
                print(c['name'], precision, len(out['tiles_changed']), len(out['diagnostics']), flush=True)
    counts = {}
    for c in cases:
        counts[c['input']['op']] = counts.get(c['input']['op'], 0) + 1
    digest = write_document('native-water-areas.json', __doc__, cases + real,
                            {'generated_counts': counts, 'real_case_count': len(real)})
    write_findings(digest)


if __name__ == '__main__':
    main()
