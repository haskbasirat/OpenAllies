"""Original free-placement search 0x554850 (with shape query constructor
0x552F70) and its definition wrapper radius rule 0x554AD0.

Builds the native tile index with research/phase7-spatial/original_query.Query,
initializes the original 4096-entry trig tables (constructors 0x409040 /
0x4090E0, as research/phase4-trig does) so the table lookups 0x85DAA0 /
0x85DA80 execute, then runs unchanged 0x554850: 0x552F70, 0x46E0A0, 0x552250,
first-hit query 0x553BA0 (tile scan, marks, exclusion, overlap) and the
ring/rotation loop. Seams: allocation (phase7 Index), source-exclusion and
custom filter leaves (phase7 Query; unused without a source actor).
"""
import json, random, struct, sys, hashlib
from pathlib import Path
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase7-spatial'))
from original_query import Query  # noqa: E402
from original_index import Index  # noqa: E402
from original_index import shape, bits, W  # noqa: E402
from original_rng import original, modified  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX, UC_X86_REG_ESP, UC_X86_REG_EAX  # noqa: E402
from unicorn import UC_HOOK_CODE, UC_HOOK_MEM_READ, UC_HOOK_MEM_WRITE  # noqa: E402
sys.path.insert(0, str(HERE))
from harness import ALLOCATORS, UnseamedAllocator, page_zero_access  # noqa: E402

PRECISION = [('Single24', 0x7f), ('Double53', 0x27f), ('Extended64', 0x37f)]
ARGS = 0x200f000


class Guarded(Index):
    """Installed between phase7 Query and Index (MRO), so the guards exist
    before Query maps page 0 and runs any native code. phase7 maps page 0
    without a guard; FS base is 0, so only an FS-prefixed access to [0..3]
    (SEH link) is legal and anything else there is a native null dereference.
    Allocators other than the phase7 0x6F7810/0x6F7430 seams must never run."""

    def __init__(self, *args):
        super().__init__(*args)
        self.u.hook_add(UC_HOOK_MEM_READ | UC_HOOK_MEM_WRITE,
                        lambda u, access, address, size, value, _: page_zero_access(u, address, size), begin=0, end=0xfff)
        for pc in ALLOCATORS:
            if pc not in (0x6f7810, 0x6f7430):
                self.u.hook_add(UC_HOOK_CODE, lambda u, a, s, d: self.unseamed(a), begin=pc, end=pc)

    @staticmethod
    def unseamed(pc):
        raise UnseamedAllocator(hex(pc))


class Placement(Query, Guarded):
    def __init__(self, row, cw):
        super().__init__(row, cw)
        for pc, obj in [(0x409040, 0xb39a48), (0x4090e0, 0xb39a60)]:
            self.reset(0)
            self.u.reg_write(UC_X86_REG_ECX, obj)
            self.w(self.STACK + 4, 4096)
            self.u.emu_start(pc, self.STOP, count=2000000)
        # Static initializers 0x9C8620/0x9C8640 publish the table objects.
        self.w(0xb39b08, 0xb39a48)
        self.w(0xb39b10, 0xb39a60)

    def table(self, entry):
        self.reset(0)
        out = ARGS + 0x200
        self.w(self.STACK + 4, 0xbf490fdb)
        stub = ARGS + (0x300 if entry == 0x85daa0 else 0x380)  # distinct code: no stale translation
        self.u.mem_write(stub, b'\x68' + struct.pack('<I', 0xbf490fdb) + b'\xe8' +
                         struct.pack('<i', entry - (stub + 10)) + b'\x83\xc4\x04\xd9\x1d' + struct.pack('<I', out) + b'\xc3')
        self.w(self.STACK, self.STOP)
        self.u.reg_write(UC_X86_REG_ESP, self.STACK)
        self.u.emu_start(stub, self.STOP, count=100000)
        return self.r(out)

    def distance_search(self, point, direction, shape_words, def_radius, def_flags, distance, strict):
        self.reset(0)
        self.calls = []
        definition = ARGS + 0x400
        self.u.mem_write(ARGS, W(*point, *direction))
        # Definition +0x3C is the blocking shape's own radius word (+0x38 shape +4).
        assert def_radius == shape_words[1]
        self.u.mem_write(definition + 0x38, W(*shape_words))
        self.w(definition + 0x1c, def_flags)
        args = [ARGS, ARGS + 8, definition, distance, strict, 0]
        for i, a in enumerate(args):
            self.w(self.STACK + 4 + 4 * i, a)
        self.u.reg_write(UC_X86_REG_ECX, self.STATE)
        self.u.emu_start(0x554b30, self.STOP, count=8000000)
        assert self.u.reg_read(UC_X86_REG_ESP) == self.STACK + 4 + 0x18
        return {'found': self.u.reg_read(UC_X86_REG_EAX) & 255, 'point': [self.r(ARGS), self.r(ARGS + 4)],
                'direction': [self.r(ARGS + 8), self.r(ARGS + 12)], 'calls': self.calls}

    def search(self, point, direction, shape_words, radius, rings, keep, strict, large):
        self.reset(0)
        self.calls = []
        self.u.mem_write(ARGS, W(*point, *direction, *shape_words))
        args = [ARGS, ARGS + 8, ARGS + 0x10, radius, rings, keep, strict, large]
        for i, a in enumerate(args):
            self.w(self.STACK + 4 + 4 * i, a)
        self.u.reg_write(UC_X86_REG_ECX, self.STATE)
        self.u.emu_start(0x554850, self.STOP, count=4000000)
        assert self.u.reg_read(UC_X86_REG_ESP) == self.STACK + 4 + 4 * len(args)
        links = {str(k): self.inverse.get(self.r(p + 0x10)) for k, p in self.pointers.items()}
        return {'found': self.u.reg_read(UC_X86_REG_EAX) & 255, 'point': [self.r(ARGS), self.r(ARGS + 4)],
                'direction': [self.r(ARGS + 8), self.r(ARGS + 12)], 'calls': self.calls, 'links': links}


def scenario(rng):
    members, definitions = [], {}
    for i in range(10):
        key = i + 1
        kind = rng.randrange(4)
        radius = 0.0 if kind == 0 else rng.uniform(0.4, 1.6)
        flags = rng.choice([0x20000400, 0x400, 0x20000400, 0x0])
        xy = [rng.uniform(4, 12), rng.uniform(3, 11)]
        facing = rng.choice([[0., -1.], [.6, .8], [1., 0.]])
        members.append({'key': key, 'reference': ((i + 2) << 16) | key,
                        'actor': {'definition': key + 10, 'flags': flags, 'position': list(map(bits, xy)),
                                  'facing': list(map(bits, facing))},
                        'height_bits': bits(2.), 'facing_z_bits': 0, 'cached_tile': [0, 0],
                        'blocking': shape(kind, radius, rng.uniform(.3, 1.2), rng.uniform(.3, 1.2)),
                        'soft': shape(3, 3., 2.5, 2.5),
                        'target': shape(kind, radius, 1., 1.), 'actor_category': True, 'wall_present': False})
        definitions[str(key + 10)] = {'flags': flags, 'object_data': True, 'flag_1a8': False}
    blocked = [0x300 if rng.random() < 0.08 else 0x200 for _ in range(56)]
    return {'grid': {'dimensions': [8, 7], 'coordinate_shift': 1, 'cell_size_bits': bits(2.)},
            'extent_bits': list(map(bits, [16., 14.])), 'flags': blocked, 'members': members,
            'definitions': definitions, 'strict': False, 'extra_flags': 0, 'ignore_actor': None,
            'ignore_definition_actor': None, 'ignore_all': False, 'custom_filter': False, 'excluded': [],
            'pre_marked': [], 'retired': []}


def main():
    rng = random.Random(0x554850)
    rows = [scenario(rng) for _ in range(24)]
    cases, windows, tables = [], {}, {}
    for precision, cw in PRECISION:
        for row in rows:
            m = Placement(row, cw)
            tables[precision] = {'sin': m.table(0x85daa0), 'cos': m.table(0x85da80)}
            searches = []
            for _ in range(12):
                kind = rng.randrange(4)
                r = rng.uniform(0.3, 1.5)
                sh = [kind, bits(0.0 if kind == 0 else r), *map(bits, [-r, -r * .7, r, r * .7])]
                import math
                a = rng.uniform(-math.pi, math.pi)
                args = {'point': list(map(bits, [rng.uniform(1, 15), rng.uniform(1, 13)])),
                        'direction': list(map(bits, [math.cos(a), math.sin(a)])), 'shape': sh,
                        'radius': bits(rng.choice([0.5, 1.0, 1.75, 2.5])), 'rings': rng.choice([0, 1, 3, 5]),
                        'keep': rng.randrange(2), 'strict': rng.randrange(2), 'large': rng.randrange(2)}
                args['native'] = m.search(args['point'], args['direction'], sh, args['radius'], args['rings'],
                                          args['keep'], args['strict'], args['large'])
                searches.append(args)
            distance_searches = []
            for _ in range(6):
                import math
                kind = rng.randrange(4)
                r = rng.uniform(0.3, 1.5)
                sh = [kind, bits(0.0 if kind == 0 else r), *map(bits, [-r, -r * .7, r, r * .7])]
                a = rng.uniform(-math.pi, math.pi)
                d = {'point': list(map(bits, [rng.uniform(1, 15), rng.uniform(1, 13)])),
                     'direction': list(map(bits, [math.cos(a), math.sin(a)])), 'shape': sh,
                     'definition_radius': sh[1],
                     'definition_flags': rng.choice([0, 1 << 21]),
                     'distance': bits(rng.choice([0.1, 2.0, 7.5, 60.0])), 'strict': rng.randrange(2)}
                d['native'] = m.distance_search(d['point'], d['direction'], sh, d['definition_radius'],
                                                d['definition_flags'], d['distance'], d['strict'])
                distance_searches.append(d)
            windows.update(m.visited)
            cases.append({'precision': precision, 'input': row, 'geometry': m.geometry, 'searches': searches,
                          'distance_searches': distance_searches})
    pins = []
    for a, n in sorted(windows.items()):
        raw = original.range(a, n)
        assert raw == modified.range(a, n)
        pins.append({'va': hex(a), 'bytes': raw.hex()})
    out = {'schema': 1, 'scope': __doc__, 'original_sha256': hashlib.sha256(original.b).hexdigest(),
           'v14_sha256': hashlib.sha256(modified.b).hexdigest(), 'tables': tables, 'case_count': len(cases),
           'cases': cases, 'instructions': pins}
    p = HERE / 'native-placement.json'
    p.write_text(json.dumps(out, indent=1) + '\n')
    found = sum(s['native']['found'] for c in cases for s in c['searches'])
    moved = sum(1 for c in cases for s in c['searches'] if s['native']['point'] != s['point'])
    print({'cases': len(cases), 'searches': sum(len(c['searches']) for c in cases), 'found': found, 'moved': moved,
           'tables': tables, 'sha256': hashlib.sha256(p.read_bytes()).hexdigest()})


if __name__ == '__main__':
    main()
