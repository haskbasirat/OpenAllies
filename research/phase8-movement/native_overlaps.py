"""Original all-overlaps spatial query 0x555620 and overlap-list refresh 0x4D67F0.

Builds the native tile index with research/phase7-spatial/original_query.Query
(actual 0x5536B0 insertion, 0x552DD0/0x5531F0 descriptor preparation). Then
runs unchanged 0x552250 positioning and 0x555620 (shape tile iterator
0x554DD0/0x5541F0, tile rectangle 0x5534F0/0x552550, intrusive marks
0x4EFB10/0x4EFB20/0x4EFB50, exclusion 0x552340, overlap 0x46EF30, list
append) or 0x4D67F0 (expiry walk, list removal 0x79E3C0, query constructor
0x554350, 0x555620, prepend and temporary list free).

Seams: allocation/free (phase7 Index), source-exclusion 0x4D5600 and custom
filter (phase7 Query), actor eligibility predicate 0x4D10A0 (recorded).
"""
import copy, json, random, struct, sys
from pathlib import Path
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase7-spatial'))
from original_query import Query  # noqa: E402
from original_index import shape, bits, W  # noqa: E402
from original_rng import original, modified  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX, UC_X86_REG_ESP  # noqa: E402

PRECISION = [('Single24', 0x7f), ('Double53', 0x27f), ('Extended64', 0x37f)]
WORLD = 0x200a000
LIST = 0x200e800


class Overlaps(Query):
    def __init__(self, row, cw):
        self.eligible = None
        self.events = []
        super().__init__(row, cw)

    def hook(self, u, pc, size, user):
        if pc == 0x4d10a0:
            self.events.append({'op': 'eligible', 'actor': self.inverse[u.reg_read(UC_X86_REG_ECX)]})
            self.ret(0, int(self.eligible))
            return
        super().hook(u, pc, size, user)

    def links(self):
        return {str(k): self.inverse.get(self.r(p + 0x10)) for k, p in self.pointers.items()}

    def all_overlaps(self, xy, flag):
        self.reset(0)
        self.calls = []
        self.u.reg_write(UC_X86_REG_ECX, self.QUERY)
        self.u.mem_write(self.STACK + 4, W(*xy))
        self.u.emu_start(0x552250, self.STOP, count=10000)
        prepared = self.prepared()
        self.w(LIST, 0)
        self.w(LIST + 4, 0)
        self.reset(0)
        self.u.reg_write(UC_X86_REG_ECX, self.STATE)
        self.w(self.STACK + 4, self.QUERY)
        self.w(self.STACK + 8, LIST)
        self.w(self.STACK + 12, flag)
        self.u.emu_start(0x555620, self.STOP, count=2000000)
        assert self.u.reg_read(UC_X86_REG_ESP) == self.STACK + 16
        found, node, last = [], self.r(LIST), 0
        while node:
            found.append(self.r(node))
            last = node
            node = self.r(node + 4)
        assert last == self.r(LIST + 4)
        return {'prepared': prepared, 'references': found, 'calls': self.calls, 'links': self.links()}

    def refresh(self, entries, time_bits, eligible, actor_flags):
        actor = self.pointers[1]
        self.w(actor + 8, actor_flags)
        self.eligible = eligible
        self.events = []
        self.calls = []
        nodes = []
        for reference, expiry in entries:
            n = self.heap
            self.heap += 16
            self.u.mem_write(n, W(reference, expiry, 0))
            nodes.append(n)
        for i, n in enumerate(nodes):
            self.w(n + 8, nodes[i + 1] if i + 1 < len(nodes) else 0)
        self.w(actor + 0x110, nodes[0] if nodes else 0)
        self.w(actor + 0x114, nodes[-1] if nodes else 0)
        self.w(WORLD + 0xe0, time_bits)
        self.reset(0)
        self.u.reg_write(UC_X86_REG_ECX, actor)
        self.u.emu_start(0x4d67f0, self.STOP, count=4000000)
        assert self.u.reg_read(UC_X86_REG_ESP) == self.STACK + 4
        out, node, last = [], self.r(actor + 0x110), 0
        while node:
            out.append([self.r(node), self.r(node + 4)])
            last = node
            node = self.r(node + 8)
        assert last == self.r(actor + 0x114)
        return {'entries': out, 'events': self.events, 'calls': self.calls, 'links': self.links()}


def scenario(rng, index):
    members, definitions = [], {}
    for i in range(9):
        key = i + 1
        kind = rng.randrange(4)
        radius = 0.0 if kind == 0 else rng.uniform(0.3, 2.5)
        flags = rng.choice([0x20000400, 0x400, 0x20000000, 0x0, 0x20000400 | (1 << 12)])
        if i == 0:
            flags = rng.choice([0x400, 0x20000400, 0x0, 0x20000000]) | rng.choice([0, 1 << 12, 1 << 18, 1 << 21])
        xy = [rng.uniform(0.5, 15.5), rng.uniform(0.5, 13.5)] if i else [rng.uniform(3, 13), rng.uniform(3, 11)]
        if i in (1, 2) and rng.random() < 0.7:
            xy = [xy[0] * 0 + 1.0, 0.0]
        facing = rng.choice([[0., -1.], [.6, .8], [1., 0.], [-0.70710677, 0.70710677]])
        hx, hy = rng.uniform(0.25, 2.5), rng.uniform(0.25, 2.5)
        members.append({'key': key, 'reference': ((i + 2) << 16) | key,
                        'actor': {'definition': key + 10, 'flags': flags, 'position': list(map(bits, xy)),
                                  'facing': list(map(bits, facing))},
                        'height_bits': bits(2.), 'facing_z_bits': 0, 'cached_tile': [0, 0],
                        'blocking': shape(kind, radius, hx, hy), 'soft': shape(3, 3., 2.5, 2.5),
                        'target': shape(kind, radius, hx, hy), 'actor_category': True,
                        'wall_present': False})
        definitions[str(key + 10)] = {'flags': flags, 'object_data': rng.random() < 0.85,
                                      'flag_1a8': False}
    # Cluster some actors around the source so overlaps occur.
    sx, sy = [struct.unpack('<f', struct.pack('<I', v))[0] for v in members[0]['actor']['position']]
    for m in members[1:6]:
        m['actor']['position'] = list(map(bits, [sx + rng.uniform(-2.5, 2.5), sy + rng.uniform(-2.5, 2.5)]))
    return {'grid': {'dimensions': [8, 7], 'coordinate_shift': 1, 'cell_size_bits': bits(2.)},
            'extent_bits': list(map(bits, [16., 14.])),
            'flags': [0x200] * 56, 'members': members, 'definitions': definitions,
            'strict': False, 'extra_flags': 0,
            'ignore_actor': 2 if index % 7 == 1 else None,
            'ignore_definition_actor': 3 if index % 7 == 2 else None,
            'ignore_all': index % 11 == 3, 'custom_filter': index % 5 == 4,
            'excluded': [4] if index % 3 == 0 else [], 'pre_marked': [5] if index % 6 == 2 else [],
            'retired': [6] if index % 4 == 3 else []}


def main():
    rng = random.Random(0x555620)
    rows = [scenario(rng, i) for i in range(64)]
    cases, windows = [], {}
    for precision, cw in PRECISION:
        for i, row in enumerate(rows):
            m = Overlaps(row, cw)
            source = [struct.unpack('<f', struct.pack('<I', v))[0] for v in row['members'][0]['actor']['position']]
            points = [source, [source[0] + 1.25, source[1] - 0.75], [0.0, 0.0], [-0.5, 3.0], [15.99, 13.99],
                      [rng.uniform(0, 16), rng.uniform(0, 14)]]
            steps = []
            for point in points:
                for flag in (0, 1):
                    steps.append({'point': list(map(bits, point)), 'flag10_only': flag,
                                  'native': m.all_overlaps(list(map(bits, point)), flag)})
            refreshes = []
            for j in range(3):
                entries = [[rng.choice([0x20007, 0x30008, 0x50009]), bits(rng.choice([0.0, 1.0, 5.0, 7.5, -1.0]))]
                           for _ in range(rng.randrange(4))]
                flags = row['members'][0]['actor']['flags'] | rng.choice([0x400, 0x400, 0, 1 << 29])
                eligible = rng.random() < 0.8
                time_bits = bits(rng.choice([0.0, 5.0, 7.5, 100.0]))
                refreshes.append({'entries': entries, 'time_bits': time_bits, 'eligible': eligible,
                                  'actor_flags': flags,
                                  'native': m.refresh(entries, time_bits, eligible, flags)})
            windows.update(m.visited)
            cases.append({'precision': precision, 'input': row, 'geometry': m.geometry, 'initial': m.prepared_initial,
                          'steps': steps, 'refreshes': refreshes})
    pins = []
    for a, n in sorted(windows.items()):
        raw = original.range(a, n)
        assert raw == modified.range(a, n)
        pins.append({'va': hex(a), 'bytes': raw.hex()})
    import hashlib
    out = {'schema': 1, 'scope': __doc__, 'original_sha256': hashlib.sha256(original.b).hexdigest(),
           'v14_sha256': hashlib.sha256(modified.b).hexdigest(), 'case_count': len(cases), 'cases': cases,
           'instructions': pins}
    p = HERE / 'native-overlaps.json'
    p.write_text(json.dumps(out, indent=1) + '\n')
    print({'cases': len(cases), 'queries': sum(len(c['steps']) for c in cases),
           'refreshes': sum(len(c['refreshes']) for c in cases), 'sha256': hashlib.sha256(p.read_bytes()).hexdigest()})


if __name__ == '__main__':
    main()
