"""Original GWorld placement test 0x4F3C40 and structure-crossing test 0x4F22C0.

Both execute UNCHANGED (harness.Machine: every executed byte equals v14,
page-0 access faults unless FS-prefixed, unseamed allocators trap), together
with their original pure leaves 0x42A530 (tile at point, CRT _ftol 0x96CC4C)
and 0x54ECF0 (tile +0x10 region node, then node +8). Seams, recorded with
their arguments in call order:
  0x4F2220 structure at point (thiscall GWorld, two binary32 words, ret 8)
  0x4D36F0 (thiscall source actor, structure; ret 4)
  0x585280 (thiscall structure word +0x98, actor; ret 4)
  0x585160 (thiscall structure word +0x98; ret 0)
  object virtuals +0xB4, +0x100, +0x164 (structure arg), +0x168 (actor arg)
  0x553BA0 first hit (thiscall GMap, 16-byte result buffer, descriptor; ret 8)
Boolean seams return garbage above AL to prove that only AL is tested.
Data words that the Rust port reads through its world trait are recorded in
native order whenever the reading instruction lies inside 0x4F3C40, 0x4F22C0
or 0x54ECF0: source actor +0x20 (position), actor +0x90, tile +0x10 and
region node +8.

The `composed_cases` group runs the REAL 0x553BA0 (phase7 Query machinery
with a Guarded page-0/allocator class in the MRO, as native_placement.py
does) underneath the real 0x4F3C40 and 0x552250, so the Rust replay drives
aa_spatial::query::first_hit through the same descriptor.
"""
import copy
import hashlib
import json
import random
import struct
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(HERE))
import harness  # noqa: E402
from harness import (Machine, NullDereference, UnseamedAllocator, ALLOCATORS, page_zero_access,  # noqa: E402
                     write_document, f2b, P, V, PRECISION)
sys.path.insert(0, str(ROOT / 'research/phase7-spatial'))
from original_query import Query  # noqa: E402
from original_index import Index, shape  # noqa: E402
from unicorn import UC_HOOK_CODE, UC_HOOK_MEM_READ, UC_HOOK_MEM_WRITE  # noqa: E402
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_ECX, UC_X86_REG_EIP, UC_X86_REG_ESP, UC_X86_REG_FPSW  # noqa: E402

OWN = ((0x4f3c40, 0x4f3d83), (0x4f22c0, 0x4f2415), (0x54ecf0, 0x54ecfc))
EXECUTED = OWN + ((0x42a530, 0x42a573), (0x96cc4c, 0x96ccc1))
SEAMS = (0x4f2220, 0x4d36f0, 0x585280, 0x585160, 0x553ba0)
OBJECT_KEYS = (1, 2, 10, 11, 12)
ACTORS = (1, 2)
STRUCTURES = (10, 11, 12)
NODE_KEYS = (1, 2, 3, 4)
MAX_TILES = 64
TRUE = (0x01, 0x80, 0xff, 0xa5a5a501, 0x12345680)
FALSE = (0x00, 0xa5a5a500, 0xffffff00, 0x00000100)
QNAN, NQNAN, SNAN, NSNAN = 0x7fc00000, 0xffc00000, 0x7f800001, 0xff800001
INF, NINF = 0x7f800000, 0xff800000
MAPS = {
    'base': {'width': 8, 'height': 7, 'shift': 1, 'cell_bits': f2b(2.), 'extent_bits': [f2b(16.), f2b(14.)]},
    'remainder': {'width': 8, 'height': 7, 'shift': 1, 'cell_bits': f2b(2.), 'extent_bits': [f2b(16.75), f2b(14.5)]},
    'shift2': {'width': 5, 'height': 4, 'shift': 2, 'cell_bits': f2b(4.), 'extent_bits': [f2b(20.), f2b(16.)]},
    'shift0': {'width': 8, 'height': 8, 'shift': 0, 'cell_bits': f2b(1.), 'extent_bits': [f2b(8.), f2b(8.)]},
}


class Unexpected(Exception):
    pass


def own(pc):
    return any(a <= pc < b for a, b in OWN)


def al(value):
    return int(value & 0xff != 0)


def s32(v):
    return struct.unpack('<i', struct.pack('<I', v & 0xffffffff))[0]


def ftol(bits):
    """0x96CC4C result (EAX) for a binary32 operand: truncation; NaN, +-inf and
    magnitudes >= 2^63 give the integer-indefinite low dword 0."""
    if bits & 0x7f800000 == 0x7f800000:
        return 0
    value = struct.unpack('<f', struct.pack('<I', bits))[0]
    t = int(value)
    if not -(1 << 63) <= t < (1 << 63):
        return 0
    return s32(t)


def rust_tile_domain(bits):
    """aa_spatial::water_areas::tile_at_point accepts finite values whose
    truncation fits i32; it refuses the rest explicitly."""
    if bits & 0x7f800000 == 0x7f800000:
        return False
    return -(1 << 31) <= int(struct.unpack('<f', struct.pack('<I', bits))[0]) < (1 << 31)


def tile_index(mp, point):
    cx, cy = ftol(point[0]) >> mp['shift'], ftol(point[1]) >> mp['shift']
    return s32(mp['width'] * cy + cx)


# ---------------------------------------------------------------- seamed group
class GMachine(Machine):
    def __init__(self, precision):
        super().__init__(precision)
        self.gworld = self.alloc(0x300)
        self.gmap = self.alloc(0x100)
        self.w32(self.gworld + 0x30, self.gmap)
        self.tiles = self.alloc(MAX_TILES * 0x30)
        self.nodes = {k: self.alloc(0x20) for k in NODE_KEYS}
        self.node_key = {a: k for k, a in self.nodes.items()}
        self.desc = self.alloc(0x58)
        self.obj, self.key_of, self.footprint_of = {}, {}, {}
        for key in OBJECT_KEYS:
            o, vt, fp = self.alloc(0x100), self.alloc(0x200), self.alloc(0x40)
            self.obj[key] = o
            self.key_of[o] = key
            self.footprint_of[fp] = key
            self.w32(o, vt)
            self.w32(o + 0x98, fp)
            for off, handler in ((0xb4, self.v_b4), (0x100, self.v_100), (0x164, self.v_164), (0x168, self.v_168)):
                self.w32(vt + off, self.stub(handler))
        s = self.seams
        s[0x4f2220] = self.structure_at
        s[0x4d36f0] = self.actor_4d36f0
        s[0x585280] = self.footprint_585280
        s[0x585160] = self.footprint_585160
        s[0x553ba0] = self.first_hit
        self.u.hook_add(UC_HOOK_MEM_READ, self.watch_read, begin=harness.HEAP, end=harness.HEAP_END)
        self.case = None
        self.watch = {}

    def stub(self, handler):
        """Each virtual slot gets its own never-rewritten address."""
        address = self.stub_next
        self.stub_next += 16
        self.seams[address] = handler
        return address

    def ev(self, *row):
        self.events.append(list(row))

    def key(self, pointer):
        return 0 if pointer == 0 else self.key_of[pointer]

    def ptr(self, key):
        return 0 if key == 0 else self.obj[key]

    def answer(self, table, *keys):
        for row in self.case['answers'][table]:
            if row[:len(keys)] == list(keys):
                return row[len(keys)]
        raise Unexpected(f'{table}{keys}')

    # ---- seams -----------------------------------------------------------
    def v_b4(self, m):
        a = self.key(m.ecx())
        v = self.answer('actor_b4', a)
        self.ev('actor_b4', a, al(v))
        m.ret(0, v)

    def structure_at(self, m):
        assert m.ecx() == self.gworld
        x, y = m.arg(0), m.arg(1)
        k = self.answer('structure_at', x, y)
        self.ev('structure_at', x, y, k)
        m.ret(8, self.ptr(k))

    def actor_4d36f0(self, m):
        a, s = self.key(m.ecx()), self.key(m.arg(0))
        v = self.answer('actor_4d36f0', a, s)
        self.ev('actor_4d36f0', a, s, al(v))
        m.ret(4, v)

    def v_168(self, m):
        s, a = self.key(m.ecx()), self.key(m.arg(0))
        v = self.answer('structure_168', s, a)
        self.ev('structure_168', s, a, al(v))
        m.ret(4, v)

    def footprint_585280(self, m):
        s, a = self.footprint_of[m.ecx()], self.key(m.arg(0))
        v = self.answer('footprint_585280', s, a)
        self.ev('footprint_585280', s, a, al(v))
        m.ret(4, v)

    def v_100(self, m):
        a = self.key(m.ecx())
        k = self.answer('actor_100', a)
        self.ev('actor_100', a, k)
        m.ret(0, self.ptr(k))

    def footprint_585160(self, m):
        s = self.footprint_of[m.ecx()]
        v = self.answer('footprint_585160', s)
        self.ev('footprint_585160', s, al(v))
        m.ret(0, v)

    def v_164(self, m):
        a, s = self.key(m.ecx()), self.key(m.arg(0))
        v = self.answer('actor_164', a, s)
        self.ev('actor_164', a, s, al(v))
        m.ret(4, v)

    def first_hit(self, m):
        assert m.ecx() == self.gmap and m.arg(1) == self.desc
        out = m.arg(0)
        # Result buffer = 0x4F3C40's 16-byte frame block (it aliases the x/y copy).
        assert out == m.esp() + 0x1c, (hex(out), hex(m.esp()))
        assert m.r32(self.desc + 0x4c, 2) == self.case['descriptor']['position']
        h = self.case['first_hit']
        m.w32(out, h['kind'], h['reference'], *h['cell'])
        self.ev('first_hit', h['kind'], h['reference'], *h['cell'])
        m.ret(8, out)

    def watch_read(self, u, access, address, size, value, _):
        w = self.watch.get(address)
        if w is None or not own(u.reg_read(UC_X86_REG_EIP)):
            return
        assert size == 4
        kind, key = w
        v = self.r32(address)
        if kind == 'actor_position':
            self.ev(kind, key)
        elif kind == 'actor_word_90':
            self.ev(kind, key, int(v != 0))
        elif kind == 'tile_node':
            self.ev(kind, key, 0 if v == 0 else self.node_key[v])
        else:
            self.ev(kind, key, v)

    # ---- cases -----------------------------------------------------------
    def setup(self, c):
        self.case = c
        self.events = []
        mp = c['map']
        n = mp['width'] * mp['height']
        assert n <= MAX_TILES
        self.u.mem_write(self.tiles, b'\0' * (MAX_TILES * 0x30))
        self.w32(self.gmap + 4, mp['shift'], mp['cell_bits'], *mp['extent_bits'])
        self.w32(self.gmap + 0x1c, mp['width'], mp['height'])
        self.w32(self.gmap + 0x28, self.tiles)
        for tile, node in c['tile_nodes']:
            self.w32(self.tiles + tile * 0x30 + 0x10, self.nodes[node])
        for a in self.nodes.values():
            self.w32(a + 8, 0)
        for node, region in c['node_regions']:
            self.w32(self.nodes[node] + 8, region)
        for o in c['objects']:
            p = self.obj[o['key']]
            self.w32(p + 0x20, *o['position'])
            self.w32(p + 0x90, o['word90'])
        d = c['descriptor']
        words = [0xdd000000 | (i * 4) for i in range(22)]
        words[7] = self.ptr(c['source'])  # +0x1C, as the native callers pass it
        words[8] = self.ptr(d['ignore_actor'])
        words[10] = self.ptr(d['collision_owner'])
        words[15] = d['mask']
        words[19], words[20] = d['position']
        self.w32(self.desc, *words)
        self.descriptor_words = words
        self.watch = {}
        for k, p in self.obj.items():
            self.watch[p + 0x20] = ('actor_position', k)
            self.watch[p + 0x90] = ('actor_word_90', k)
        for t in range(n):
            self.watch[self.tiles + t * 0x30 + 0x10] = ('tile_node', t)
        for k, a in self.nodes.items():
            self.watch[a + 8] = ('node_region', k)

    def run(self, c):
        self.setup(c)
        out = {'result': None, 'fault': None}
        try:
            if c['entry'] == '4f3c40':
                eax = self.call(0x4f3c40, self.gworld, (self.ptr(c['source']), self.desc))
                assert self.final_esp == harness.STACK + 4 + 8
            else:
                cr = c['crossing']
                eax = self.call(0x4f22c0, self.gworld,
                                (self.ptr(c['source']), *cr['from'], *cr['to'], self.desc))
                assert self.final_esp == harness.STACK + 4 + 0x18
            assert self.fpu_top() == 0
            out['result'] = eax
        except NullDereference as error:
            out['fault'] = str(error)
        assert self.r32(self.desc, 22) == self.descriptor_words, 'descriptor written'
        out['events'] = self.events
        return out


def answers():
    return {
        'actor_b4': [[k, TRUE[0]] for k in OBJECT_KEYS],
        'structure_at': [],
        'actor_4d36f0': [[a, s, FALSE[0]] for a in ACTORS for s in STRUCTURES],
        'structure_168': [[s, a, FALSE[0]] for s in STRUCTURES for a in ACTORS],
        'footprint_585280': [[s, a, FALSE[0]] for s in STRUCTURES for a in ACTORS],
        'actor_100': [[k, 0] for k in OBJECT_KEYS],
        'footprint_585160': [[s, FALSE[0]] for s in STRUCTURES],
        'actor_164': [[a, s, TRUE[0]] for a in ACTORS for s in STRUCTURES],
    }


def template(name, entry='4f3c40', mp='base'):
    m = MAPS[mp]
    return {
        'name': name, 'entry': entry, 'map': copy.deepcopy(m),
        'tile_nodes': [[t, 1] for t in range(m['width'] * m['height'])],
        'node_regions': [[1, 0x51], [2, 0x52], [3, 0x51], [4, 0]],
        'objects': [{'key': k, 'position': [f2b(3.), f2b(3.)], 'word90': 0} for k in OBJECT_KEYS],
        'answers': answers(), 'source': 1,
        'descriptor': {'ignore_actor': 0, 'collision_owner': 0, 'mask': 0x300, 'position': [f2b(5.), f2b(5.)]},
        'first_hit': {'kind': 0, 'reference': 0, 'cell': [0, 0]},
        'crossing': None, 'structures': {'from': 0, 'to': 0},
    }


def put(c, table, *row):
    keys = list(row[:-1])
    for r in c['answers'][table]:
        if r[:len(keys)] == keys:
            r[len(keys)] = row[-1]
            return c
    c['answers'][table].append(list(row))
    return c


def obj(c, key):
    return next(o for o in c['objects'] if o['key'] == key)


def finish(c):
    """structure_at answers for the two points 0x4F22C0 would query; identical
    points must give one answer."""
    if c['entry'] == '4f3c40':
        src = obj(c, c['source'])['position'] if c['source'] else [0, 0]
        points = [src, c['descriptor']['position']]
        src_ok = c['source'] == 0 or rust_tile_domain(src[0]) and rust_tile_domain(src[1])
        c['rust_tile_domain'] = bool(src_ok)
        mp = c['map']
        for p, in_bounds in ((src, True), (c['descriptor']['position'], False)):
            if c['source'] == 0 and p is src:
                continue
            index = tile_index(mp, p)
            if not 0 <= index < mp['width'] * mp['height']:
                # Only reached when the descriptor point passes the bounds test.
                c['note'] = f'tile {index} outside the allocation'
    else:
        points = [c['crossing']['from'], c['crossing']['to']]
    s = c['structures']
    if points[0] == points[1]:
        s['to'] = s['from']
        c['answers']['structure_at'] = [[*points[0], s['from']]]
    else:
        c['answers']['structure_at'] = [[*points[0], s['from']], [*points[1], s['to']]]
    return c


def named_placement_cases():
    out = []

    def add(c):
        out.append(finish(c))
        return c

    add(template('clear_after_first_hit_kind_0'))
    for mask in (0, 0x100, 0x200, 0x300, 0x400, 0xfffffcff, 0x80000100, 0x00000200 | 0xf0000000):
        add(dict(template(f'mask_{mask:08x}'), descriptor={
            'ignore_actor': 0, 'collision_owner': 0, 'mask': mask, 'position': [f2b(5.), f2b(5.)]}))
    bad = {'neg1': f2b(-1.), 'neg_subnormal': 0x80000001, 'neg_inf': NINF, 'pos_inf': INF, 'qnan': QNAN,
           'neg_qnan': NQNAN, 'snan': SNAN, 'neg_snan': NSNAN}
    good = {'neg_zero': 0x80000000, 'zero': 0, 'subnormal': 1}
    for axis, extent, below, above in ((0, f2b(16.), 0x417fffff, 0x41800001), (1, f2b(14.), 0x415fffff, 0x41600001)):
        values = dict(bad, extent=extent, above_extent=above)
        values.update({'below_extent': below}, **good)
        for label, v in values.items():
            for mask in (0x300, 0):
                c = template(f'bounds_{"xy"[axis]}_{label}_mask{mask:x}')
                c['descriptor']['position'][axis] = v
                c['descriptor']['mask'] = mask
                add(c)
    c = template('bounds_both_nan')
    c['descriptor']['position'] = [QNAN, SNAN]
    add(c)
    c = template('bounds_x_bad_y_good_nullsource')
    c['descriptor']['position'][0] = f2b(-3.)
    c['source'] = 0
    add(c)
    for label, ext in (('extent_x_nan', [QNAN, f2b(14.)]), ('extent_y_nan', [f2b(16.), NQNAN]),
                       ('extent_x_inf', [INF, f2b(14.)]), ('extent_x_zero', [0, f2b(14.)]),
                       ('extent_y_neg_zero', [f2b(16.), 0x80000000])):
        c = template(label)
        c['map']['extent_bits'] = ext
        add(c)
    c = template('extent_x_inf_far_point')
    c['map']['extent_bits'] = [INF, f2b(14.)]
    c['descriptor']['position'][0] = f2b(15.5)
    add(c)
    # Regions: node words and +8 region words.
    width = 8
    src_tile, dst_tile = 1 * width + 1, 2 * width + 2

    def regions(name, src_node, dst_node, node_regions=None):
        c = template(name)
        c['tile_nodes'] = [[t, 1] for t in range(56) if t not in (src_tile, dst_tile)]
        if src_node:
            c['tile_nodes'].append([src_tile, src_node])
        if dst_node:
            c['tile_nodes'].append([dst_tile, dst_node])
        if node_regions is not None:
            c['node_regions'] = node_regions
        return add(c)
    regions('region_same_node', 1, 1)
    regions('region_different_node_same_word', 3, 1)
    regions('region_different_word', 2, 1)
    regions('region_source_null_node_target_node', 0, 1)
    regions('region_source_node_target_null_node', 1, 0)
    regions('region_both_null_nodes', 0, 0)
    regions('region_null_node_vs_null_region_word', 0, 4)
    regions('region_null_region_word_vs_region', 4, 1)
    regions('region_both_null_region_words', 4, 4)
    regions('region_words_all_zero', 1, 2, [[1, 0], [2, 0], [3, 0], [4, 0]])
    regions('region_word_high_bit', 1, 2, [[1, 0x80000000], [2, 0x00000000], [3, 0], [4, 0]])
    # 0x42A530 wraps without a clamp.
    for label, pos, wrapped in (('source_x_neg_wraps_to_previous_row', [f2b(-2.5), f2b(3.)], 7),
                                ('source_x_beyond_extent_wraps_to_next_row', [f2b(17.), f2b(3.)], 16),
                                ('source_x_neg_fraction_truncates_to_zero', [f2b(-0.75), f2b(3.)], 8),
                                ('source_neg_zero', [0x80000000, 0x80000000], 0)):
        c = template(label)
        obj(c, 1)['position'] = pos
        c['tile_nodes'] = [[t, 2 if t == wrapped else 1] for t in range(56)]
        add(c)
    # The remainder map: the descriptor passes bounds in the extra half cell.
    c = template('remainder_extent_wraps_descriptor_tile', mp='remainder')
    c['descriptor']['position'] = [f2b(16.5), f2b(5.)]
    c['tile_nodes'] = [[t, 2 if t == 2 * 8 + 8 else 1] for t in range(56)]
    add(c)
    c = template('remainder_extent_same_region', mp='remainder')
    c['descriptor']['position'] = [f2b(16.5), f2b(5.)]
    add(c)
    for mp in ('shift2', 'shift0'):
        c = template(f'map_{mp}', mp=mp)
        add(c)
        c = template(f'map_{mp}_region_change', mp=mp)
        dst = tile_index(c['map'], c['descriptor']['position'])
        c['tile_nodes'] = [[t, 2 if t == dst else 1] for t in range(c['map']['width'] * c['map']['height'])]
        add(c)
    # Source positions the Rust tile lookup refuses (native: _ftol low dword 0).
    for label, pos in (('source_x_nan', [QNAN, f2b(3.)]), ('source_y_pos_inf', [f2b(3.), INF]),
                       ('source_x_two_pow_32', [0x4f800000, f2b(3.)]), ('source_y_neg_inf', [f2b(3.), NINF])):
        c = template(label)
        obj(c, 1)['position'] = pos
        add(c)
    # Crossing through 0x4F3C40.
    c = template('crossing_4d36f0_returns_2')
    c['structures'] = {'from': 10, 'to': 0}
    put(c, 'actor_4d36f0', 1, 10, TRUE[3])
    add(c)
    c = template('crossing_target_164_false_returns_2')
    c['structures'] = {'from': 0, 'to': 11}
    put(c, 'footprint_585160', 11, TRUE[1])
    put(c, 'actor_164', 1, 11, FALSE[2])
    add(c)
    c = template('crossing_source_b4_false_continues')
    put(c, 'actor_b4', 1, FALSE[1])
    c['structures'] = {'from': 10, 'to': 11}
    add(c)
    c = template('descriptor_at_source_point_same_structure')
    c['descriptor']['position'] = [f2b(3.), f2b(3.)]
    c['structures'] = {'from': 12, 'to': 12}
    add(c)
    c = template('crossing_clear_then_blocked')
    c['structures'] = {'from': 0, 'to': 11}
    put(c, 'footprint_585160', 11, TRUE[0])
    c['first_hit'] = {'kind': 2, 'reference': 0x30005, 'cell': [3, -1]}
    add(c)
    for kind in (1, 2, 3, 0x100, 0x80000000, 0):
        c = template(f'first_hit_kind_{kind:x}')
        c['first_hit'] = {'kind': kind, 'reference': 0xabcdef, 'cell': [-2, 7]}
        add(c)
    # Null source: fault only when the region step reads it.
    for mask in (0x300, 0x100, 0):
        c = template(f'null_source_mask_{mask:x}')
        c['source'] = 0
        c['descriptor']['mask'] = mask
        add(c)
    c = template('null_source_outside')
    c['source'] = 0
    c['descriptor']['position'][1] = f2b(14.)
    add(c)
    c = template('source_is_structure_actor')
    c['source'] = 2
    obj(c, 2)['position'] = [f2b(9.), f2b(1.)]
    c['tile_nodes'] = [[t, 1] for t in range(56)]
    c['structures'] = {'from': 11, 'to': 10}
    put(c, 'actor_4d36f0', 2, 11, FALSE[3])
    put(c, 'footprint_585160', 11, TRUE[2])
    put(c, 'actor_164', 2, 11, TRUE[4])
    put(c, 'footprint_585160', 10, TRUE[2])
    put(c, 'actor_164', 2, 10, TRUE[1])
    c['first_hit'] = {'kind': 1, 'reference': 0, 'cell': [2, 2]}
    add(c)
    return out


def named_crossing_cases():
    out = []

    def base(name, sf=0, st=0):
        c = template(name, '4f22c0')
        c['crossing'] = {'from': [f2b(3.), f2b(3.)], 'to': [f2b(5.), f2b(5.)]}
        c['structures'] = {'from': sf, 'to': st}
        return c

    def add(c):
        out.append(finish(c))

    c = base('null_source')
    c['source'] = 0
    add(c)
    c = base('b4_false', 10, 11)
    put(c, 'actor_b4', 1, FALSE[2])
    add(c)
    add(base('both_null'))
    add(base('same_structure', 10, 10))
    c = base('same_point_same_structure', 11)
    c['crossing']['to'] = list(c['crossing']['from'])
    add(c)
    c = base('nan_points_pass_through', 0, 12)
    c['crossing'] = {'from': [SNAN, NQNAN], 'to': [0x80000000, NINF]}
    add(c)
    c = base('4d36f0_true', 10, 11)
    put(c, 'actor_4d36f0', 1, 10, TRUE[1])
    add(c)
    c = base('4d36f0_true_target_null', 10, 0)
    put(c, 'actor_4d36f0', 1, 10, TRUE[4])
    add(c)
    # Source structure checks (target null).
    for label, v168, f280, f160, v164 in (('168_false', 0, 0, 1, 0), ('168_true_585280_false', 1, 0, 1, 0),
                                          ('admitted_skips_source', 1, 1, 1, 0), ('585160_false', 0, 0, 0, 0),
                                          ('164_true', 0, 0, 1, 1), ('164_false', 0, 0, 1, 0)):
        c = base(f'source_{label}', 10, 0)
        put(c, 'structure_168', 10, 1, (TRUE if v168 else FALSE)[2])
        put(c, 'footprint_585280', 10, 1, (TRUE if f280 else FALSE)[3])
        put(c, 'footprint_585160', 10, (TRUE if f160 else FALSE)[1])
        put(c, 'actor_164', 1, 10, (TRUE if v164 else FALSE)[3])
        add(c)
    # Target structure checks (source null).
    for label, f160, v164 in (('585160_false', 0, 0), ('164_true', 1, 1), ('164_false', 1, 0)):
        c = base(f'target_{label}', 0, 11)
        put(c, 'footprint_585160', 11, (TRUE if f160 else FALSE)[4 % 4])
        put(c, 'actor_164', 1, 11, (TRUE if v164 else FALSE)[2])
        add(c)
    # Exempt structure: descriptor +0x28, or +0x20 actor virtual +0x100.
    exempt_rows = (
        ('owner_word_28_is_source', 10, 0, None),
        ('owner_word_28_is_target', 11, 0, None),
        ('owner_word_28_other_with_ignore_set', 12, 2, None),
        ('ignore_b4_false', 0, 2, ('b4', 0)),
        ('ignore_word90_null', 0, 2, ('w90', 0)),
        ('ignore_100_is_source', 0, 2, ('v100', 10)),
        ('ignore_100_is_target', 0, 2, ('v100', 11)),
        ('ignore_100_other', 0, 2, ('v100', 12)),
        ('ignore_100_null', 0, 2, ('v100', 0)),
        ('ignore_is_source_actor', 0, 1, ('v100', 11)),
        ('ignore_is_structure', 0, 10, ('v100', 10)),
    )
    for label, owner, ignore, how in exempt_rows:
        for sf, st in ((10, 11), (0, 11), (10, 0)):
            c = base(f'exempt_{label}_{sf}_{st}', sf, st)
            c['descriptor']['collision_owner'] = owner
            c['descriptor']['ignore_actor'] = ignore
            if ignore:
                obj(c, ignore)['word90'] = 0x1234
            for s in STRUCTURES:
                put(c, 'footprint_585160', s, TRUE[1])
                put(c, 'actor_164', 1, s, FALSE[1])
            if how:
                kind, v = how
                if kind == 'b4':
                    put(c, 'actor_b4', ignore, FALSE[3])
                elif kind == 'w90':
                    obj(c, ignore)['word90'] = 0
                else:
                    put(c, 'actor_100', ignore, v)
            add(c)
    # Source passes, target decides.
    for label, f160, v164 in (('target_null', None, None), ('target_585160_false', 0, 0),
                              ('target_164_true', 1, 1), ('target_164_false', 1, 0)):
        st = 0 if f160 is None else 11
        c = base(f'source_passes_{label}', 10, st)
        put(c, 'footprint_585160', 10, TRUE[0])
        put(c, 'actor_164', 1, 10, TRUE[2])
        if f160 is not None:
            put(c, 'footprint_585160', 11, (TRUE if f160 else FALSE)[2])
            put(c, 'actor_164', 1, 11, (TRUE if v164 else FALSE)[4 % 4])
        add(c)
    c = base('admitted_source_target_still_checked', 10, 11)
    put(c, 'structure_168', 10, 1, TRUE[4])
    put(c, 'footprint_585280', 10, 1, TRUE[1])
    put(c, 'footprint_585160', 10, TRUE[1])
    put(c, 'actor_164', 1, 10, FALSE[1])
    put(c, 'footprint_585160', 11, TRUE[1])
    put(c, 'actor_164', 1, 11, FALSE[3])
    add(c)
    c = base('source_actor_2', 12, 10)
    c['source'] = 2
    put(c, 'footprint_585160', 12, TRUE[3])
    put(c, 'actor_164', 2, 12, TRUE[0])
    put(c, 'footprint_585160', 10, TRUE[3])
    put(c, 'actor_164', 2, 10, FALSE[0])
    add(c)
    return out


def randomize_answers(rng, c):
    a = c['answers']
    pick = lambda p: rng.choice(TRUE) if rng.random() < p else rng.choice(FALSE)  # noqa: E731
    for row in a['actor_b4']:
        row[1] = pick(0.85)
    for table in ('actor_4d36f0', 'structure_168', 'footprint_585280'):
        for row in a[table]:
            row[2] = pick(0.35)
    for row in a['footprint_585160']:
        row[1] = pick(0.65)
    for row in a['actor_164']:
        row[2] = pick(0.5)
    for row in a['actor_100']:
        row[1] = rng.choice([0, 10, 11, 12, 1, 2])
    for o in c['objects']:
        o['word90'] = rng.choice([0, 0x1234, 0x80000000])
    d = c['descriptor']
    d['ignore_actor'] = rng.choice([0, 0, 1, 2, 10, 11])
    d['collision_owner'] = rng.choice([0, 0, 0, 10, 11, 12, 2])
    c['structures'] = {'from': rng.choice([0, 0, 10, 11, 12]), 'to': rng.choice([0, 0, 10, 11, 12])}


def random_crossing_cases(rng, count):
    out = []
    pts = [f2b(3.), f2b(5.), f2b(7.5), QNAN, 0x80000000, f2b(-1.)]
    for i in range(count):
        c = template(f'crossing_random_{i}', '4f22c0')
        c['crossing'] = {'from': [rng.choice(pts), rng.choice(pts)], 'to': [rng.choice(pts), rng.choice(pts)]}
        randomize_answers(rng, c)
        c['source'] = rng.choice([1, 1, 1, 1, 2, 2, 0])
        out.append(finish(c))
    return out


def random_placement_cases(rng, count):
    out = []
    boundary = {0: [0, 0x80000000, 1, 0x417fffff, f2b(16.), QNAN, f2b(-0.5)],
                1: [0, 0x80000000, 0x415fffff, f2b(14.), NQNAN, f2b(-0.5)]}
    while len(out) < count:
        mp = rng.choice(['base', 'base', 'base', 'remainder', 'shift2', 'shift0'])
        c = template(f'placement_random_{len(out)}', mp=mp)
        m = c['map']
        w, h = m['width'] * (1 << m['shift']), m['height'] * (1 << m['shift'])
        ext = [struct.unpack('<f', struct.pack('<I', e))[0] for e in m['extent_bits']]
        pos = [f2b(rng.uniform(0, ext[0])), f2b(rng.uniform(0, ext[1]))]
        for axis in (0, 1):
            if mp == 'base' and rng.random() < 0.1:
                pos[axis] = rng.choice(boundary[axis])
        c['descriptor']['position'] = pos
        src = [f2b(rng.uniform(0, w - 0.01)), f2b(rng.uniform(0, h - 0.01))]
        if rng.random() < 0.15:
            src = list(pos)
        randomize_answers(rng, c)
        c['source'] = rng.choice([1, 1, 1, 2])
        obj(c, c['source'])['position'] = src
        c['descriptor']['mask'] = rng.choice([0, 0x100, 0x200, 0x300, 0x300, 0x400, 0xfffffcff, 0x80000300,
                                              rng.getrandbits(32)])
        n = m['width'] * m['height']
        c['tile_nodes'] = [[t, k] for t in range(n) for k in [rng.choice([1, 1, 1, 1, 2, 3, 4, None])] if k]
        c['first_hit'] = {'kind': rng.choice([0, 0, 0, 1, 2, 3, 0x100]), 'reference': rng.getrandbits(32),
                          'cell': [rng.randrange(-3, 9), rng.randrange(-3, 9)]}
        finish(c)
        if 'note' in c:
            continue
        out.append(c)
    return out


# ------------------------------------------------------- composed with 0x553BA0
class Guarded(Index):
    """As native_placement.py: phase7 maps page 0 without a guard. Installed
    between Query and Index in the MRO so the guards exist before any native
    code runs. Only an FS-prefixed access to [0..3] is legal on page 0, and
    allocators other than phase7's 0x6F7810/0x6F7430 seams must never run."""

    def __init__(self, *args):
        super().__init__(*args)
        self.u.hook_add(UC_HOOK_MEM_READ | UC_HOOK_MEM_WRITE,
                        lambda u, access, address, size, value, _: page_zero_access(u, address, size),
                        begin=0, end=0xfff)
        for pc in ALLOCATORS:
            if pc not in (0x6f7810, 0x6f7430):
                self.u.hook_add(UC_HOOK_CODE, lambda u, a, s, d: self.unseamed(a), begin=pc, end=pc)

    @staticmethod
    def unseamed(pc):
        raise UnseamedAllocator(hex(pc))


class Composed(Query, Guarded):
    GWORLD = 0x200a000  # phase7 Index publishes [0xB36534] = 0x200A000, [+0x30] = GMap
    NODES = 0x200f000

    def __init__(self, row, cw):
        self.recording = False
        self.events = []
        super().__init__(row, cw)
        assert self.r(self.GWORLD + 0x30) == self.STATE
        n = row['grid']['dimensions'][0] * row['grid']['dimensions'][1]
        assert all(self.r(self.TILES + t * 48 + 16) == 0 for t in range(n)), 'tile +0x10 used by the index'
        self.node_addr = {k: self.NODES + k * 0x20 for k in NODE_KEYS}
        self.node_key = {a: k for k, a in self.node_addr.items()}
        for tile, node in row['tile_nodes']:
            self.w(self.TILES + tile * 48 + 16, self.node_addr[node])
        for node, region in row['node_regions']:
            self.w(self.node_addr[node] + 8, region)
        self.watch = {}
        for key, p in self.pointers.items():
            self.watch[p + 0x20] = ('actor_position', key)
            self.watch[p + 0x90] = ('actor_word_90', key)
        for t in range(n):
            self.watch[self.TILES + t * 48 + 16] = ('tile_node', t)
        for k, a in self.node_addr.items():
            self.watch[a + 8] = ('node_region', k)
        for begin, size in ((self.ACTORS, 0x100000), (self.TILES, 0x100000), (self.NODES, 0x100)):
            self.u.hook_add(UC_HOOK_MEM_READ, self.watch_read, begin=begin, end=begin + size - 1)
        self.definition_key = {self.DEFS + i * 0x400: m['actor']['definition'] for i, m in enumerate(row['members'])}

    def watch_read(self, u, access, address, size, value, _):
        if not self.recording:
            return
        w = self.watch.get(address)
        if w is None or not own(u.reg_read(UC_X86_REG_EIP)):
            return
        kind, key = w
        v = self.r(address)
        if kind == 'actor_position':
            self.events.append([kind, key])
        elif kind == 'actor_word_90':
            self.events.append([kind, key, int(v != 0)])
        elif kind == 'tile_node':
            self.events.append([kind, key, 0 if v == 0 else self.node_key[v]])
        else:
            self.events.append([kind, key, v])

    def hook(self, u, pc, size, user):
        if self.recording:
            if pc == 0x4f2220:
                assert u.reg_read(UC_X86_REG_ECX) == self.GWORLD
                sp = u.reg_read(UC_X86_REG_ESP)
                self.events.append(['structure_at', self.r(sp + 4), self.r(sp + 8), 0])
                self.ret(8, 0)
            elif pc in (0x6a71d0, 0x6a71f0) and own(self.r(u.reg_read(UC_X86_REG_ESP))):
                # Real virtual +0xB4 bodies (mov al,1 / xor al,al), observed only
                # when 0x4F22C0 calls them.
                self.events.append(['actor_b4', self.inverse[u.reg_read(UC_X86_REG_ECX)], int(pc == 0x6a71d0)])
            elif pc == 0x4f3d63:
                out = u.reg_read(UC_X86_REG_EAX)
                w = struct.unpack('<4I', u.mem_read(out, 16))
                self.events.append(['first_hit', w[0], w[1], s32(w[2]), s32(w[3])])
            elif pc in (0x4d36f0, 0x585280, 0x585160):
                raise Unexpected(hex(pc))
        super().hook(u, pc, size, user)

    def actor_key(self, p):
        return None if p == 0 else self.inverse[p]

    def descriptor(self):
        w = struct.unpack('<22I', self.u.mem_read(self.QUERY, 0x58))
        assert w[10] == 0 and w[11] == 0
        return {'placement': {'shape': {'kind': w[0], 'radius_bits': w[1], 'bounds': list(w[2:6])},
                              'facing': [w[17], w[18]], 'position': [w[19], w[20]]},
                'descriptor': {'source_definition': None if w[6] == 0 else self.definition_key[w[6]],
                               'source_actor': self.actor_key(w[7]), 'ignore_actor': self.actor_key(w[8]),
                               'ignore_definition': None if w[9] == 0 else self.definition_key[w[9]],
                               'collision_owner': None, 'hostility_context': None,
                               'ignore_all': bool(w[12] & 0xff), 'ignore_moving': bool((w[12] >> 8) & 0xff),
                               'custom_filter': w[13] != 0, 'mask': w[15]},
                'required_actor_flags': w[14]}

    def place(self, xy):
        # 0x552250 on the descriptor first, as 0x53A6F0 and 0x4F46E0 do.
        self.reset(0)
        self.u.reg_write(UC_X86_REG_ECX, self.QUERY)
        self.u.mem_write(self.STACK + 4, struct.pack('<2I', *xy))
        self.u.emu_start(0x552250, self.STOP, count=10000)
        descriptor = self.descriptor()
        self.reset(0)
        self.calls, self.events = [], []
        self.w(self.STACK + 4, self.pointers[1])
        self.w(self.STACK + 8, self.QUERY)
        self.u.reg_write(UC_X86_REG_ECX, self.GWORLD)
        self.recording = True
        try:
            self.u.emu_start(0x4f3c40, self.STOP, count=4000000)
        finally:
            self.recording = False
        assert self.u.reg_read(UC_X86_REG_EIP) == self.STOP
        assert self.u.reg_read(UC_X86_REG_ESP) == self.STACK + 12
        assert (self.u.reg_read(UC_X86_REG_FPSW) >> 11) & 7 == 0
        assert self.descriptor() == descriptor
        return {'point': list(xy), 'descriptor': descriptor, 'result': self.u.reg_read(UC_X86_REG_EAX),
                'events': self.events, 'calls': self.calls}


def composed_row(rng, index):
    bits = f2b
    members, definitions = [], {}
    for i in range(10):
        key = i + 1
        kind = rng.randrange(4)
        radius = 0.0 if kind == 0 else rng.uniform(0.4, 1.6)
        flags = rng.choice([0x20000400, 0x400, 0x20000400, 0x0])
        if i == 0 and index % 5 == 4:
            flags |= 1 << 12
        if i == 0 and index % 3 == 1:
            flags |= 1 << 21
        xy = [rng.uniform(4, 12), rng.uniform(3, 11)]
        facing = rng.choice([[0., -1.], [.6, .8], [1., 0.]])
        members.append({'key': key, 'reference': ((i + 2) << 16) | key,
                        'actor': {'definition': key + 10, 'flags': flags, 'position': list(map(bits, xy)),
                                  'facing': list(map(bits, facing))},
                        'height_bits': bits(2.), 'facing_z_bits': 0, 'cached_tile': [0, 0],
                        'blocking': shape(kind, radius, rng.uniform(.3, 1.2), rng.uniform(.3, 1.2)),
                        'soft': shape(3, 3., 2.5, 2.5), 'target': shape(kind, radius, 1., 1.),
                        'actor_category': not (i == 0 and index % 4 == 3), 'wall_present': False})
        definitions[str(key + 10)] = {'flags': flags | (1 << 16 if i == 0 and index % 6 == 2 else 0),
                                      'object_data': True, 'flag_1a8': False}
    n = 56
    tile_nodes = []
    for t in range(n):
        r = rng.random()
        if r < 0.75:
            tile_nodes.append([t, 1])
        elif r < 0.85:
            tile_nodes.append([t, 3])
        elif r < 0.93:
            tile_nodes.append([t, 2])
    return {'grid': {'dimensions': [8, 7], 'coordinate_shift': 1, 'cell_size_bits': bits(2.)},
            'extent_bits': list(map(bits, [16., 14.])), 'flags': [0x300 if rng.random() < 0.08 else 0x200 for _ in range(n)],
            'members': members, 'definitions': definitions, 'strict': bool(index % 2), 'extra_flags': 0,
            'ignore_actor': 2 if index % 4 == 1 else None, 'ignore_definition_actor': None, 'ignore_all': False,
            'custom_filter': index % 5 == 2, 'excluded': [2] if index % 3 == 0 else [], 'pre_marked': [],
            'retired': [3] if index % 7 == 5 else [],
            'tile_nodes': tile_nodes, 'node_regions': [[1, 0x51], [2, 0x52], [3, 0x51], [4, 0]]}


def composed_cases():
    rng = random.Random(0x553BA0)
    rows = [composed_row(rng, i) for i in range(16)]
    points = []
    for _ in rows:
        pts = [[f2b(rng.uniform(0.5, 15.5)), f2b(rng.uniform(0.5, 13.5))] for _ in range(7)]
        pts += [[f2b(-0.25), f2b(5.)], [f2b(16.), f2b(5.)], [f2b(15.99), f2b(13.99)], [0x80000000, f2b(7.)]]
        points.append(pts)
    out = []
    for precision, cw in PRECISION.items():
        for row, pts in zip(rows, points):
            m = Composed(row, cw)
            steps = [m.place(p) for p in pts]
            for pc, size in m.visited.items():
                raw = P.range(pc, size)
                assert raw == V.range(pc, size), hex(pc)
                harness.VISITED[pc] = raw.hex()
            out.append({'precision': precision, 'input': row, 'geometry': m.geometry, 'steps': steps})
    return out


FINDINGS = {
    'addresses': {
        '0x4F3C40': 'GWorld placement test: thiscall GWorld (source actor, collision descriptor), ret 8; '
                    '0 clear, 1 region change, 2 structure crossing, 3 outside world, 4 first-hit blocked',
        '0x4F22C0': 'structure crossing: thiscall GWorld (ECX is forwarded to 0x4F2220; Ghidra shows a plain '
                    'function) (source, from.x, from.y, to.x, to.y, descriptor), ret 0x18; returns 0 or 2',
        '0x42A530': 'GMap tile at point (executed; Rust reuses water_areas::tile_at_point)',
        '0x54ECF0': 'tile +0x10 region node, null gives 0, else node +8 (executed; ported as tile_region)',
        '0x96CC4C': 'CRT _ftol used by 0x42A530 (executed)',
        '0x553BA0': 'first hit; seamed in the main group, REAL in composed_cases',
        'leaves': '0x4F2220, 0x4D36F0, 0x585280/0x585160 (on structure +0x98), object virtuals +0xB4/+0x100/'
                  '+0x164/+0x168 and data words actor +0x20/+0x90, tile +0x10, node +8 are world-trait callbacks',
    },
    'native_facts': [
        'Bounds: x<0 / y<0 use FCOMP + test ah,1: C0 is also set by unordered, so NaN (quiet or signalling, '
        'either sign) returns 3 at the first test. x<extent / y<extent use test ah,5 + jp: only "less" passes, '
        'so x == extent, x > extent and an unordered compare (NaN point or NaN extent) return 3. -0.0 passes '
        '(equal to 0.0). The extent is [GWorld+0x30]+0xC/+0x10. No arithmetic: precision-independent.',
        'Mask: descriptor +0x3C & 0x300 (test dh,3). Zero skips every callback; the source is never read.',
        'The source pointer is dereferenced (+0x20) at 0x4F3CC0 without a null test when the mask passes; '
        'the native faults (fixture rows null_source_mask_*). Rust returns Error::NullSource. 0x4F22C0 is '
        'reached only from 0x4F3C40 (calls.tsv), so its own null-source test never fires from there.',
        'Source +0x20/+0x24 is read twice: 0x4F3CC0 for the region, 0x4F3D28 for the 0x4F22C0 arguments.',
        'The descriptor position is copied once at entry (+0x4C/+0x50); that copy is the target tile point '
        'and the 0x4F22C0 target. 0x553BA0 reads the descriptor itself.',
        'Ghidra prints 0x553BA0(&local_10, descriptor) as if a position were passed: the first argument is '
        'the 16-byte result buffer (it aliases the frame x/y copy); 4F3C40 tests only result word 0 with a '
        'full dword compare (kinds 1, 2, 3, 0x100, 0x80000000 all give 4).',
        'Region compare: 0x54ECF0 words. A null node and a node whose +8 word is 0 compare equal; distinct '
        'nodes with the same +8 word compare equal.',
        '0x42A530 has no clamp: trunc(x)>>shift and trunc(y)>>shift form width*cy+cx, so x<0 or x>=width '
        'cells wraps into the adjacent row (rows source_x_*). _ftol gives low dword 0 for NaN, +-inf and '
        'multiples of 2^32; aa_spatial::water_areas::tile_at_point refuses non-finite or non-i32 truncations '
        '(rows with rust_tile_domain false): explicit unsupported error, not ported.',
        'All boolean leaves (0x4D36F0, 0x585280, 0x585160, virtuals +0xB4/+0x164/+0x168) are tested with '
        'test al,al: the seams return garbage above AL.',
        '0x4F22C0 order: B4(source); 4F2220(from); 4F2220(to); null/null -> 0; s==d -> 0; 4D36F0(source,s) '
        '-> 2; admitted=s when s.168(source) and 585280(s.+98, source); exempt=desc+0x28, else when '
        'desc+0x20 is nonnull, its B4 is true and its +0x90 is nonnull, exempt=desc+0x20.vt100 (desc+0x20 '
        're-read after B4); s (not exempt, not admitted) with 585160 true and 164(source,s) false -> 2; '
        'd (not exempt; admitted is NOT tested for d) with 585160 true and 164(source,d) false -> 2; else 0.',
        'Callers 0x53A6F0 (source = descriptor +0x1C), 0x53DC00, 0x53CE40 (source = descriptor +0x1C) and '
        '0x4F46E0 only compare the result with 0.',
    ],
    'seam_choice': '0x553BA0 is seamed in the main group so every return code and its exact call (ECX = '
                   'GMap, result buffer in the 4F3C40 frame, unchanged descriptor) can be forced and checked '
                   'at every precision; 0x553BA0 itself is proven in phase7 (native-query.json). '
                   'composed_cases additionally run the real 0x553BA0 (and 0x552250) beneath the real '
                   '0x4F3C40 so the Rust replay calls aa_spatial::query::first_hit with the same descriptor.',
    'removed_caps': 'none: both functions are fixed-size control flow',
}


def main():
    rng = random.Random(0x4F3C40)
    templates = (named_placement_cases() + named_crossing_cases()
                 + random_crossing_cases(rng, 500) + random_placement_cases(rng, 500))
    cases = []
    for precision in PRECISION:
        m = GMachine(precision)
        for t in templates:
            c = copy.deepcopy(t)
            c['precision'] = precision
            c['output'] = m.run(c)
            cases.append(c)
    allowed = set(SEAMS)
    for pc in harness.VISITED:
        assert pc in allowed or any(a <= pc < b for a, b in EXECUTED), hex(pc)
    by_name = {}
    for c in cases:
        by_name.setdefault(c['name'], []).append(json.dumps(c['output'], sort_keys=True))
    assert all(len(set(v)) == 1 for v in by_name.values()), 'precision-dependent output'
    results = {}
    for c in cases:
        o = c['output']
        key = f"{c['entry']}:{'fault' if o['fault'] else o['result']}"
        results[key] = results.get(key, 0) + 1
    composed = composed_cases()
    composed_results = {}
    for c in composed:
        for s in c['steps']:
            composed_results[s['result']] = composed_results.get(s['result'], 0) + 1
    findings = dict(FINDINGS, result_counts=results, composed_result_counts=composed_results,
                    producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
    write_document('native-gworld-placement.json', __doc__, cases,
                   {'composed_cases': composed, 'findings': findings})
    print(json.dumps({'results': results, 'composed': composed_results}))


if __name__ == '__main__':
    main()
