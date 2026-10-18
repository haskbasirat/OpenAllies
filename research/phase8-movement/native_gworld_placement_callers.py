"""Original 0x53CE40 (destination acceptance of 0x53D340) and 0x4F46E0
(nearest placeable position) around the GWorld placement test 0x4F3C40.

Both execute UNCHANGED under harness.Machine (every executed byte equals v14,
page-0 access faults unless FS-prefixed, unseamed allocators trap), with their
inlined 0x42A530 tile arithmetic and CRT _ftol 0x96CC4C. Recorded seams:
  0x552250 descriptor position (thiscall descriptor, x, y; ret 8). The seam
           stores a deliberately different "snapped" position so later reads
           show which words are used.
  0x54F370 tile collision (thiscall tile, descriptor; ret 4) [0x53CE40]
  0x4F3C40 placement test (thiscall GWorld, source, descriptor; ret 8)
  0x554350 descriptor construction (thiscall descriptor, actor, 0; ret 8)
           [0x4F46E0], actor virtual +0xB4 [0x4F46E0]
Data reads modelled as world reads are recorded in native order when the
reading instruction lies in the function under test: tile byte +0x20
(0x53CE40), actor +0x7C, object +0x40 and actor facing +0x2C (0x4F46E0).

0x4F46E0 runs the REAL rotation table lookups 0x85DAA0/0x85DA80 over the
original 4096-entry tables built by the unchanged constructors 0x409040 /
0x4090E0 (allocation seamed) and published as the static initializers
0x9C8620/0x9C8640 do. The constructors' FSIN/FCOS execute in Unicorn
(binary64 inside, binary32 table stores); the looked-up -pi/4 words equal the
aa-trig/phase7 table words recorded by native_placement.py.
"""
import copy
import hashlib
import json
import random
import struct
import sys
from fractions import Fraction
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import harness  # noqa: E402
from harness import Machine, NullDereference, write_document, f2b, b2f, PRECISION  # noqa: E402
from unicorn import UC_HOOK_CODE, UC_HOOK_MEM_READ  # noqa: E402
from unicorn.x86_const import UC_X86_REG_EIP  # noqa: E402

ENDPOINT = (0x53ce40, 0x53cf3e)
NEAREST = (0x4f46e0, 0x4f48d2)
EXECUTED = (ENDPOINT, NEAREST, (0x96cc4c, 0x96ccc1), (0x85da10, 0x85da74), (0x85da80, 0x85da91),
            (0x85daa0, 0x85dab1), (0x409040, 0x4090c9), (0x4090e0, 0x409169), (0x408de0, 0x408e30))
SEAMS = (0x552250, 0x54f370, 0x4f3c40, 0x554350, 0x6f7810)
TRUE = (0x01, 0x80, 0xff, 0xa5a5a501, 0x12345680)
FALSE = (0x00, 0xa5a5a500, 0xffffff00, 0x00000100)
QNAN, NQNAN, SNAN, NSNAN = 0x7fc00000, 0xffc00000, 0x7f800001, 0xff800001
INF, NINF = 0x7f800000, 0xff800000
JUNK_ARG0 = 0x0badf00d  # 0x53CE40 never reads its first argument: unmapped
MAPS = {
    'base': {'width': 8, 'height': 7, 'shift': 1, 'cell_bits': f2b(2.), 'extent_bits': [f2b(16.), f2b(14.)]},
    'remainder': {'width': 8, 'height': 7, 'shift': 1, 'cell_bits': f2b(2.), 'extent_bits': [f2b(16.75), f2b(14.5)]},
    'shift2': {'width': 5, 'height': 4, 'shift': 2, 'cell_bits': f2b(4.), 'extent_bits': [f2b(20.), f2b(16.)]},
    'shift0': {'width': 8, 'height': 8, 'shift': 0, 'cell_bits': f2b(1.), 'extent_bits': [f2b(8.), f2b(8.)]},
}
MAX_TILES = 64
ACTOR_KEYS = (1, 2, 3)
OWNER_KEYS = (50, 51)
# placement.rs ROTATION_SIN_BITS / ROTATION_COS_BITS (native_placement.py).
SIN_BITS, COS_BITS = 0xbf3504ef, 0x3f3504f7


class Unexpected(Exception):
    pass


def inside(pc, window):
    return window[0] <= pc < window[1]


def al(v):
    return int(v & 0xff != 0)


def s32(v):
    return struct.unpack('<i', struct.pack('<I', v & 0xffffffff))[0]


def ftol(bits):
    if bits & 0x7f800000 == 0x7f800000:
        return 0
    t = int(b2f(bits))
    return 0 if not -(1 << 63) <= t < (1 << 63) else s32(t)


def tile_index(mp, p):
    return s32(mp['width'] * (ftol(p[1]) >> mp['shift']) + (ftol(p[0]) >> mp['shift']))


class Base(Machine):
    def __init__(self, precision):
        super().__init__(precision)
        self.gworld = self.alloc(0x300)
        self.gmap = self.alloc(0x100)
        self.w32(self.gworld + 0x30, self.gmap)
        self.w32(0xb36534, self.gworld)
        self.desc_ptr = None
        self.events = []
        self.watch = {}
        self.window = None
        self.u.hook_add(UC_HOOK_MEM_READ, self.watch_read, begin=harness.HEAP, end=harness.HEAP_END)

    def ev(self, *row):
        self.events.append(list(row))

    def watch_read(self, u, access, address, size, value, _):
        w = self.watch.get(address)
        if w is None or self.window is None or not inside(u.reg_read(UC_X86_REG_EIP), self.window):
            return
        w(address, size)

    def write_map(self, mp):
        self.w32(self.gmap + 4, mp['shift'], mp['cell_bits'], *mp['extent_bits'])
        self.w32(self.gmap + 0x1c, mp['width'], mp['height'])

    def set_position(self, m):
        assert m.ecx() == self.desc_ptr
        x, y = m.arg(0), m.arg(1)
        self.ev('set_position', x, y)
        sx, sy = self.snap(x, y)
        m.w32(self.desc_ptr + 0x4c, sx, sy)
        m.ret(8)


# --------------------------------------------------------------- 0x53CE40
class EndpointMachine(Base):
    def __init__(self, precision):
        super().__init__(precision)
        self.tiles = self.alloc(MAX_TILES * 0x30)
        self.w32(self.gmap + 0x28, self.tiles)
        self.desc_ptr = self.alloc(0x58)
        self.point = self.alloc(8)
        self.obj = {k: self.alloc(0x40) for k in ACTOR_KEYS}
        self.key_of = {p: k for k, p in self.obj.items()}
        self.seams[0x552250] = self.set_position
        self.seams[0x54f370] = self.tile_blocked
        self.seams[0x4f3c40] = self.placement

    def key(self, p):
        return 0 if p == 0 else self.key_of[p]

    def snap(self, x, y):
        s = self.case['snap']
        return (x, y) if s is None else tuple(s)

    def tile_blocked(self, m):
        offset = m.ecx() - self.tiles
        assert offset % 0x30 == 0 and m.arg(0) == self.desc_ptr
        tile = offset // 0x30
        v = self.case['tile_blocked']
        self.ev('tile_blocked', tile, *m.r32(self.desc_ptr + 0x4c, 2), al(v))
        if self.case['reassign_source'] is not None:
            m.w32(self.desc_ptr + 0x1c, self.ptr(self.case['reassign_source']))
        m.ret(4, v)

    def placement(self, m):
        assert m.ecx() == self.gworld and m.arg(1) == self.desc_ptr
        v = self.case['placement']
        self.ev('placement', self.key(m.arg(0)), *m.r32(self.desc_ptr + 0x4c, 2), v)
        m.ret(8, v)

    def ptr(self, k):
        return 0 if k == 0 else self.obj[k]

    def run(self, c):
        self.case = c
        self.events = []
        mp = c['map']
        self.write_map(mp)
        self.u.mem_write(self.tiles, b'\0' * (MAX_TILES * 0x30))
        n = mp['width'] * mp['height']
        for t, byte in c['tile_bytes']:
            self.w32(self.tiles + t * 0x30 + 0x20, 0x5a5a5a00 | byte)  # only the low byte is read
        words = [0xdd000000 | (i * 4) for i in range(22)]
        words[7] = self.ptr(c['source'])
        words[19], words[20] = 0xdeadbeef, 0xdeadbeef
        self.w32(self.desc_ptr, *words)
        self.w32(self.point, *c['point'])

        def byte_read(address, size):
            assert size == 1
            self.ev('tile_byte_20', (address - 0x20 - self.tiles) // 0x30, self.r8(address))
        self.watch = {self.tiles + t * 0x30 + 0x20: byte_read for t in range(n)}
        self.window = ENDPOINT
        eax = self.call(0x53ce40, 0x13572468, (JUNK_ARG0, self.point, self.desc_ptr))
        self.window = None
        assert self.final_esp == harness.STACK + 4  # cdecl
        assert self.fpu_top() == 0
        assert self.r32(self.point, 2) == c['point']
        return {'result': eax & 0xff, 'events': self.events}


def endpoint_template(name, mp='base'):
    return {'name': name, 'map': copy.deepcopy(MAPS[mp]), 'tile_bytes': [], 'point': [f2b(5.), f2b(5.)],
            'source': 1, 'snap': None, 'tile_blocked': FALSE[0], 'reassign_source': None, 'placement': 0}


def endpoint_cases(rng):
    out = []
    bad = {'neg1': f2b(-1.), 'neg_subnormal': 0x80000001, 'neg_inf': NINF, 'pos_inf': INF, 'qnan': QNAN,
           'neg_qnan': NQNAN, 'snan': SNAN, 'neg_snan': NSNAN}
    for axis, extent, below, above in ((0, f2b(16.), 0x417fffff, 0x41800001), (1, f2b(14.), 0x415fffff, 0x41600001)):
        values = dict(bad, extent=extent, above_extent=above, below_extent=below, neg_zero=0x80000000, zero=0,
                      subnormal=1)
        for label, v in values.items():
            c = endpoint_template(f'bounds_{"xy"[axis]}_{label}')
            c['point'][axis] = v
            out.append(c)
    for label, ext in (('extent_x_nan', [QNAN, f2b(14.)]), ('extent_y_nan', [f2b(16.), NQNAN]),
                       ('extent_x_inf', [INF, f2b(14.)]), ('extent_y_zero', [f2b(16.), 0])):
        c = endpoint_template(label)
        c['map']['extent_bits'] = ext
        out.append(c)
    out.append(endpoint_template('accepted'))
    for byte in (0x01, 0x03, 0xff, 0x02, 0xfe, 0x80):
        c = endpoint_template(f'tile_byte_{byte:02x}')
        c['tile_bytes'] = [[18, byte]]
        out.append(c)
    for v in TRUE + FALSE:
        c = endpoint_template(f'tile_blocked_{v:08x}')
        c['tile_blocked'] = v
        out.append(c)
    for v in (0, 1, 2, 3, 4, 0x100, 0x80000000, 0xffffff00):
        c = endpoint_template(f'placement_{v:08x}')
        c['placement'] = v
        out.append(c)
    # The tile comes from the caller's point, not the snapped descriptor.
    c = endpoint_template('snapped_far_tile_from_original_point')
    c['snap'] = [f2b(13.), f2b(1.)]
    c['tile_bytes'] = [[0 * 8 + 6, 0x01]]
    out.append(c)
    c = endpoint_template('snapped_original_tile_blocked')
    c['snap'] = [f2b(13.), f2b(1.)]
    c['tile_bytes'] = [[18, 0x01]]
    out.append(c)
    # Descriptor +0x1C is read after 0x54F370.
    for k in (2, 0):
        c = endpoint_template(f'source_reassigned_by_tile_test_{k}')
        c['reassign_source'] = k
        out.append(c)
    c = endpoint_template('null_source_word')
    c['source'] = 0
    out.append(c)
    for label, p in (('neg_zero_point', [0x80000000, 0x80000000]), ('subnormal_point', [1, 1])):
        c = endpoint_template(label)
        c['point'] = p
        c['tile_bytes'] = [[0, 0x02]]
        out.append(c)
    c = endpoint_template('remainder_wrap_tile', 'remainder')
    c['point'] = [f2b(16.5), f2b(5.)]
    c['tile_bytes'] = [[2 * 8 + 8, 0x01]]
    out.append(c)
    for mp in ('shift2', 'shift0'):
        out.append(endpoint_template(f'map_{mp}', mp))
        c = endpoint_template(f'map_{mp}_bit0', mp)
        c['tile_bytes'] = [[tile_index(c['map'], c['point']), 0x01]]
        out.append(c)
    count = 0
    while count < 400:
        mp = rng.choice(['base', 'base', 'remainder', 'shift2', 'shift0'])
        c = endpoint_template(f'random_{count}', mp)
        m = c['map']
        ext = [b2f(e) for e in m['extent_bits']]
        c['point'] = [f2b(rng.uniform(-1, ext[0] + 1)), f2b(rng.uniform(-1, ext[1] + 1))]
        if rng.random() < 0.1:
            c['point'][rng.randrange(2)] = rng.choice([0, 0x80000000, QNAN, m['extent_bits'][0]])
        n = m['width'] * m['height']
        c['tile_bytes'] = [[t, rng.choice([0, 0, 0, 1, 2, 3, 0xfe])] for t in range(n)]
        c['source'] = rng.choice([1, 1, 2, 0])
        if rng.random() < 0.3:
            c['snap'] = [f2b(rng.uniform(0, ext[0])), f2b(rng.uniform(0, ext[1]))]
        c['tile_blocked'] = rng.choice(TRUE) if rng.random() < 0.25 else rng.choice(FALSE)
        c['reassign_source'] = rng.choice([None, None, None, 3, 0])
        c['placement'] = rng.choice([0, 0, 0, 1, 2, 3, 4, 0x100])
        index = tile_index(m, c['point'])
        x, y = b2f(c['point'][0]), b2f(c['point'][1])
        passes = 0 <= x < ext[0] and 0 <= y < ext[1]
        if passes and not 0 <= index < n:
            continue
        out.append(c)
        count += 1
    return out


# --------------------------------------------------------------- 0x4F46E0
class NearestMachine(Base):
    def __init__(self, precision):
        super().__init__(precision)
        self.seams[0x6f7810] = lambda m: m.ret(0, m.alloc(max(m.arg(0), 4)))
        for pc, table in ((0x409040, 0xb39a48), (0x4090e0, 0xb39a60)):
            self.call(pc, table, (4096,), count=5_000_000)
            assert self.final_esp == harness.STACK + 8
        # Static initializers 0x9C8620/0x9C8640 publish the table objects.
        self.w32(0xb39b08, 0xb39a48)
        self.w32(0xb39b10, 0xb39a60)
        self.tables = {'sin': self.table(0x85daa0), 'cos': self.table(0x85da80)}
        self.desc_ptr = None
        self.point = self.alloc(8)
        self.obj, self.key_of, self.vtable = {}, {}, {}
        for k in ACTOR_KEYS + OWNER_KEYS:
            p = self.alloc(0x100)
            self.obj[k] = p
            self.key_of[p] = k
        for k in ACTOR_KEYS:
            vt = self.alloc(0x200)
            self.w32(self.obj[k], vt)
            self.w32(vt + 0xb4, self.stub(self.v_b4))
        self.seams[0x554350] = self.prepare
        self.seams[0x552250] = self.set_position
        self.seams[0x4f3c40] = self.placement
        self.table_calls = 0
        self.u.hook_add(UC_HOOK_CODE, self.table_call, begin=0x85da80, end=0x85daa0)

    def stub(self, handler):
        address = self.stub_next
        self.stub_next += 16
        self.seams[address] = handler
        return address

    def table(self, entry):
        """Direct call through a fresh stub: push 0xBF490FDB; call; fstp dword."""
        out = self.alloc(4)
        stub = self.stub_next
        self.stub_next += 32
        code = (b'\x68' + struct.pack('<I', 0xbf490fdb) + b'\xe8' + struct.pack('<i', entry - (stub + 10))
                + b'\x83\xc4\x04\xd9\x1d' + struct.pack('<I', out) + b'\xc3')
        self.u.mem_write(stub, code)
        self.call(stub)
        assert self.fpu_top() == 0
        return self.r32(out)

    def table_call(self, u, pc, size, _):
        if pc in (0x85da80, 0x85daa0) and inside(self.r32(self.esp()), NEAREST):
            assert self.r32(self.esp() + 4) == 0xbf490fdb
            self.table_calls += 1

    def key(self, p):
        return 0 if p == 0 else self.key_of[p]

    def ptr(self, k):
        return 0 if k == 0 else self.obj[k]

    def snap(self, x, y):
        mask = self.case['snap_mask']
        return x & mask, y & mask

    def v_b4(self, m):
        a = self.key(m.ecx())
        v = self.actor(a)['b4']
        self.ev('actor_b4', a, al(v))
        m.ret(0, v)

    def prepare(self, m):
        # Descriptor on the 0x4F46E0 frame: ECX = esp+0x3C at the seam.
        assert m.ecx() == m.esp() + 0x3c and m.arg(1) == 0
        self.desc_ptr = m.ecx()
        a = self.key(m.arg(0))
        self.ev('prepare_descriptor', a)
        m.w32(self.desc_ptr, *self.case['descriptor_words'])
        m.ret(8)

    def placement(self, m):
        assert m.ecx() == self.gworld and m.arg(1) == self.desc_ptr
        results = self.case['results']
        if self.calls >= len(results):
            raise Unexpected('placement results exhausted')
        v = results[self.calls]
        self.calls += 1
        self.ev('placement', self.key(m.arg(0)), *m.r32(self.desc_ptr + 0x4c, 2), m.r32(self.desc_ptr + 0x38), v)
        m.ret(8, v)

    def actor(self, k):
        return next(a for a in self.case['actors'] if a['key'] == k)

    def run(self, c):
        self.case = c
        self.events = []
        self.calls = 0
        self.table_calls = 0
        self.desc_ptr = None
        self.w32(self.gmap + 8, c['cell_bits'])
        for a in c['actors']:
            p = self.obj[a['key']]
            self.w32(p + 0x7c, self.ptr(a['word7c']))
            self.w32(p + 0x2c, *a['facing'])
        for o in c['owners']:
            self.w32(self.obj[o['key']] + 0x40, self.ptr(o['word40']))
        self.w32(self.point, *c['point'])

        def word(name, offset):
            def read(address, size):
                k = self.key_of[address - offset]
                v = self.r32(address)
                if name == 'actor_facing':
                    self.ev(name, k)
                else:
                    self.ev(name, k, self.key(v))
            return read
        self.watch = {}
        for k in ACTOR_KEYS:
            self.watch[self.obj[k] + 0x7c] = word('actor_word_7c', 0x7c)
            self.watch[self.obj[k] + 0x2c] = word('actor_facing', 0x2c)
        for k in OWNER_KEYS:
            self.watch[self.obj[k] + 0x40] = word('object_word_40', 0x40)
        self.window = NEAREST
        out = {'result': None, 'fault': None}
        try:
            eax = self.call(0x4f46e0, self.gworld, (self.ptr(c['actor']), self.point, c['radius']),
                            count=5_000_000)
            assert self.final_esp == harness.STACK + 4 + 0xc
            assert self.fpu_top() == 0
            out['result'] = eax & 0xff
        except NullDereference as error:
            out['fault'] = str(error)
        self.window = None
        failures = sum(1 for e in self.events[1:] if e[0] == 'placement' and e[-1] != 0)
        placements = [e for e in self.events if e[0] == 'placement']
        if placements and out['result'] is not None:
            # Two table pairs per failed ring candidate (one rotation is discarded).
            assert self.table_calls == 4 * sum(1 for e in placements[1:] if e[-1] != 0), (self.table_calls, failures)
        out['point'] = self.r32(self.point, 2)
        out['events'] = self.events
        out['table_calls'] = self.table_calls
        return out


def nearest_template(name, cell=2., radius=4.):
    words = [0xdd000000 | (i * 4) for i in range(22)]
    words[14] = 0x400  # +0x38 required actor flags before the OR
    return {'name': name, 'cell_bits': f2b(cell), 'radius': f2b(radius), 'actor': 1,
            'actors': [{'key': k, 'b4': FALSE[0], 'word7c': 0, 'facing': [f2b(0.), f2b(-1.)]} for k in ACTOR_KEYS],
            'owners': [{'key': k, 'word40': 0} for k in OWNER_KEYS],
            'descriptor_words': words, 'snap_mask': 0xffffffff, 'results': [0],
            'point': [f2b(5.), f2b(5.)]}


def actor_of(c, k):
    return next(a for a in c['actors'] if a['key'] == k)


def rings(cell, radius):
    """Upper bound on rings (generous: exact arithmetic, never fewer)."""
    c, r = b2f(cell), b2f(radius)
    if c != c or r != r or not c <= r or c <= 0:
        return 0
    return int(r / c) + 2


def precision_split_cell():
    """A cell whose exact 3*cell exceeds f32(3*cell): with radius = f32(3*cell)
    PC24 (sum rounded to 24 bits) tries a third ring, PC53/PC64 do not."""
    for x in (1.7, 1.3, 0.7, 2.3, 1.1, 0.9, 1.9, 2.9, 0.3, 3.7, 1.45, 2.65):
        c = Fraction(b2f(f2b(x)))
        exact = 3 * c
        rounded = Fraction(b2f(f2b(float(exact))))
        if exact > rounded:
            return f2b(x), f2b(float(exact))
    raise AssertionError('no precision-splitting cell')


def nearest_cases(rng):
    out = []
    c = nearest_template('null_actor')
    c['actor'] = 0
    out.append(c)
    out.append(nearest_template('point_itself_accepted'))
    c = nearest_template('point_itself_accepted_b4_true_no_owner')
    actor_of(c, 1)['b4'] = TRUE[3]
    out.append(c)
    c = nearest_template('replaced_by_owner_actor')
    actor_of(c, 1).update(b4=TRUE[1], word7c=50)
    c['owners'][0]['word40'] = 2
    c['results'] = [3, 0]
    out.append(c)
    c = nearest_template('owner_word40_null_returns_0')
    actor_of(c, 1).update(b4=TRUE[2], word7c=51)
    out.append(c)
    c = nearest_template('b4_false_owner_ignored')
    actor_of(c, 1).update(b4=FALSE[1], word7c=50)
    c['owners'][0]['word40'] = 3
    c['results'] = [1, 0]
    out.append(c)
    for label, cell, radius in (('cell_above_radius', 2., 1.5), ('radius_nan', 2., float('nan')),
                                ('cell_equals_radius', 2., 2.), ('radius_neg_inf', 2., float('-inf'))):
        c = nearest_template(f'{label}_all_fail', cell, radius)
        if radius != radius:
            c['radius'] = QNAN
        c['results'] = [4] * (1 + 8 * max(rings(c['cell_bits'], c['radius']), 0))
        out.append(c)
    for k in range(1, 18):
        c = nearest_template(f'success_at_call_{k}')
        c['results'] = [2] * k + [0]
        actor_of(c, 1)['facing'] = [f2b(0.6), f2b(0.8)]
        c['point'] = [f2b(3.3), f2b(7.7)]
        out.append(c)
    c = nearest_template('all_rings_fail')
    c['results'] = [1] * 17
    actor_of(c, 1)['facing'] = [f2b(0.6), f2b(0.8)]
    out.append(c)
    for v in (0x80000000, 0x100, 0xffffffff):
        c = nearest_template(f'nonzero_result_{v:08x}')
        c['results'] = [v, v, 0]
        out.append(c)
    c = nearest_template('snapped_descriptor_point_uses_offsets')
    c['snap_mask'] = 0xfff00000
    c['results'] = [1, 1, 1, 0]
    actor_of(c, 1)['facing'] = [f2b(-0.70710677), f2b(0.70710677)]
    c['point'] = [f2b(6.123), f2b(2.987)]
    out.append(c)
    c = nearest_template('zero_facing')
    actor_of(c, 1)['facing'] = [0, 0x80000000]
    c['results'] = [1] * 9 + [0]
    out.append(c)
    c = nearest_template('unnormalized_facing')
    actor_of(c, 1)['facing'] = [f2b(3.25), f2b(-1.5)]
    c['results'] = [1] * 12 + [0]
    out.append(c)
    cell, radius = precision_split_cell()
    for k in (17, 25, 26):
        c = nearest_template(f'extended_step_compare_{k}')
        c['cell_bits'], c['radius'] = cell, radius
        c['results'] = [5] * (k - 1) + [0]
        actor_of(c, 1)['facing'] = [f2b(0.6), f2b(-0.8)]
        out.append(c)
    c = nearest_template('extended_step_compare_all_fail')
    c['cell_bits'], c['radius'] = cell, radius
    c['results'] = [5] * 25
    out.append(c)
    c = nearest_template('required_flags_or_kept')
    c['descriptor_words'][14] = 0x20000400
    c['results'] = [1, 0]
    out.append(c)
    for i in range(300):
        cellv = rng.choice([2., 1.7, 1.0, 0.5, 2.3, 0.37])
        radv = rng.choice([0.25, 2.0, 4.0, 5.0, 7.3, 3.3])
        c = nearest_template(f'random_{i}', cellv, radv)
        a = rng.uniform(-3.2, 3.2)
        import math
        actor_of(c, 1)['facing'] = [f2b(math.cos(a)), f2b(math.sin(a))]
        actor_of(c, 1)['b4'] = rng.choice(FALSE + TRUE)
        c['point'] = [f2b(rng.uniform(0, 20)), f2b(rng.uniform(0, 20))]
        c['snap_mask'] = rng.choice([0xffffffff, 0xffff0000])
        n = 1 + 8 * rings(c['cell_bits'], c['radius'])
        stop = rng.randrange(n + 3)
        c['results'] = [rng.choice([1, 2, 3, 4]) for _ in range(min(stop, n))] + ([0] if stop < n else [])
        out.append(c)
    return out


FINDINGS = {
    '0x53CE40': [
        'cdecl (unused, point*, descriptor*), returns AL (neg/sbb/inc of the 0x4F3C40 result, else xor al,al).',
        'GWorld comes from the global 0xB36534; the extents are GMap +0xC/+0x10.',
        'x<0: FCOM (no pop) + test ah,1 (NaN fails; the failure path pops with fstp st0 at 0x53CF32); '
        'x<extent: FCOMP + test ah,5/jp; y likewise from a frame copy. Same predicate as 0x4F3C40.',
        '0x552250(descriptor, point by value), then the point words are re-read from the CALLER\'S memory '
        '(not the snapped descriptor) for the tile index: _ftol, sar by GMap+4, GMap+0x1C*ty+tx, no clamp.',
        'Tile byte +0x20 bit 0 rejects; then 0x54F370 is thiscall on the tile with the descriptor (AL); '
        'descriptor +0x1C is read after it and passed with the descriptor to 0x4F3C40 (ECX = global GWorld).',
        'Its first argument (0x53D340 passes &original_start) is never read (an unmapped pointer is passed).',
    ],
    '0x4F46E0': [
        'thiscall GWorld (actor, point*, radius binary32), ret 0xC, returns AL.',
        'Actor virtual +0xB4 is called before any null test (a null actor faults at 0x4F46EC); when true and '
        '+0x7C is nonnull the actor becomes [[actor+0x7C]+0x40]; a null result returns 0.',
        '0x554350 builds the descriptor on the frame (this+0x30 from the caller esp), +0x38 |= 0x20000000.',
        '0x4F3C40 is called with ECX = global [0xB36534], the cell size comes from this+0x30 (+8).',
        'Facing +0x2C/+0x30 is read after the first failure, before the cell<=radius test (test ah,0x41/jp: '
        'only ordered less-or-equal continues; NaN radius returns 0).',
        'Ring offsets f32(dir*step); candidates f32(off+point) (point re-read every time); success re-adds '
        'and stores into the caller point.',
        'Each failure rotates the offset with S=f32(table sin) and the unstored table cos (an exact binary32 '
        'table word): x\'=f32(x*C - y*S), y\'=f32(y*C + S*x). It then performs two more table calls and the '
        'same rotation on a facing copy at frame+0x20/+0x24 that the next ring resets and nothing reads.',
        'step = f32(step+cell) but the comparison with the radius uses the unstored sum (fst + fcomp).',
    ],
    'tables': 'table words looked up from the emulated original tables',
}


def main():
    rng = random.Random(0x53CE40)
    endpoint_templates = endpoint_cases(rng)
    nearest_templates = nearest_cases(rng)
    cases, nearest, tables = [], [], {}
    harness.VISITED.clear()
    for precision in PRECISION:
        m = EndpointMachine(precision)
        for t in endpoint_templates:
            c = copy.deepcopy(t)
            c['precision'] = precision
            c['output'] = m.run(c)
            cases.append(c)
        n = NearestMachine(precision)
        tables[precision] = n.tables
        assert n.tables == {'sin': SIN_BITS, 'cos': COS_BITS}, n.tables
        for t in nearest_templates:
            c = copy.deepcopy(t)
            c['precision'] = precision
            c['output'] = n.run(c)
            nearest.append(c)
    for pc in harness.VISITED:
        assert pc in SEAMS or any(inside(pc, w) for w in EXECUTED), hex(pc)
    by_name = {}
    for c in cases:
        by_name.setdefault(c['name'], []).append(json.dumps(c['output'], sort_keys=True))
    assert all(len(set(v)) == 1 for v in by_name.values()), 'precision-dependent 0x53CE40'
    split = {}
    for c in nearest:
        split.setdefault(c['name'], set()).add(json.dumps(c['output'], sort_keys=True))
    precision_dependent = sorted(k for k, v in split.items() if len(v) > 1)
    counts = {
        'endpoint': {str(r): sum(1 for c in cases if c['output']['result'] == r) for r in (0, 1)},
        'nearest': {k: sum(1 for c in nearest if (c['output']['fault'] and 'fault' or str(c['output']['result'])) == k)
                    for k in ('0', '1', 'fault')},
        'nearest_precision_dependent_templates': len(precision_dependent),
    }
    findings = dict(FINDINGS, tables=tables, counts=counts, precision_dependent=precision_dependent,
                    producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
    write_document('native-gworld-placement-callers.json', __doc__, cases,
                   {'nearest_cases': nearest, 'findings': findings})
    print(json.dumps(counts))
    assert 'extended_step_compare_25' in precision_dependent, precision_dependent


if __name__ == '__main__':
    main()
