"""GMap ring search for a free spot 0x554500(point, direction, definition,
radius, scale, non_blocking, alternate).

The whole routine executes unchanged with 0x553000 (descriptor from the
definition), 0x46E0A0 (shape copy), 0x552250 (position with the snap guard),
0x46EE00 (half-cell snap with the CRT floor) and the table lookups
0x85DAA0/0x85DA80/0x85DA10 over the original 4096-entry tables, built per
precision by the unchanged constructors 0x409040/0x4090E0 (allocation
seamed) and published as the static initializers 0x9C8620/0x9C8640 do.
Seams (recorded in order, scripted results): definition virtuals +28
(alternate shape) and +30, and the first-hit query 0x553BA0 (records the
descriptor's position, required flags, mask, facing and +54, checks its
shape copy and that +18 is the definition). The table words of both turn
angles are recorded per case for the Rust host.

Usage: python native_open_spot.py [path/to/AA.exe]
Writes native-open-spot.json next to this script.
"""
import os, sys
from pathlib import Path
if len(sys.argv) > 1:
    os.environ['AA_GAME_DIR'] = str(Path(sys.argv[1]).resolve().parent)
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import copy, math, random, struct  # noqa: E402
import harness as h  # noqa: E402
from harness import f2b, b2f  # noqa: E402

TURNS = (0xbf490fdb, 0xc0060a92)
OBJECTS = (0xb39a48, 0xb39a60)
TABLES = {}


def stub(m, fn):
    a = m.stub_next
    m.stub_next += 16
    m.u.mem_write(a, b'\xcc' * 16)
    m.seams[a] = fn
    return a


def build_tables(precision):
    """Run the original table constructors once per precision."""
    m = h.Machine(precision)
    m.seams[0x6f7810] = lambda mm: mm.ret(0, mm.alloc(max(mm.arg(0), 4)))
    for pc, table in ((0x409040, OBJECTS[0]), (0x4090e0, OBJECTS[1])):
        m.call(pc, table, (4096,), count=5_000_000)
        assert m.final_esp == h.STACK + 8
    out = []
    for obj in OBJECTS:
        words = m.r32(obj, 6)
        count = words[1]
        out.append((words, bytes(m.u.mem_read(words[2], 4 * count))))
    return out


def install_tables(m):
    if m.precision not in TABLES:
        TABLES[m.precision] = build_tables(m.precision)
    for obj, (words, data) in zip(OBJECTS, TABLES[m.precision]):
        p = m.alloc(len(data))
        m.u.mem_write(p, data)
        m.w32(obj, *words)
        m.w32(obj + 8, p)
    m.w32(0xb39b08, OBJECTS[0])
    m.w32(0xb39b10, OBJECTS[1])


def lookup(m, entry, angle):
    out = m.alloc(4)
    code = (b'\x68' + struct.pack('<I', angle) + b'\xe8')
    s = m.stub_next
    m.stub_next += 32
    code += struct.pack('<i', entry - (s + 10)) + b'\x83\xc4\x04\xd9\x1d' + struct.pack('<I', out) + b'\xc3'
    m.u.mem_write(s, code)
    m.call(s)
    assert m.fpu_top() == 0
    return m.r32(out)


def run(c):
    m = h.Machine(c['precision'])
    install_tables(m)
    tables = {str(a): [lookup(m, 0x85daa0, a), lookup(m, 0x85da80, a)] for a in TURNS}
    events = []
    gmap = m.alloc(0x100)
    m.w32(gmap + 8, c['cell'])
    world = m.alloc(0x100)
    m.w32(0xb36534, world)
    m.w32(world + 0x30, gmap)
    points = m.alloc(0x100)
    point_obj = m.alloc(0x20)
    m.w32(0xb39990, points)
    m.w32(points + 0x64, point_obj)
    m.w32(point_obj + 4, 0, f2b(-1.0))
    definition = m.alloc(0x100)

    def v28(mm):
        assert mm.ecx() == definition
        events.append(['v28'])
        m.w32(mm.arg(0), *c['alt_shape'])
        mm.ret(4, mm.arg(0))

    def v30(mm):
        assert mm.ecx() == definition
        events.append(['v30'])
        mm.ret(0, c['v30'])

    vt = m.alloc(0x100)
    m.w32(definition, vt)
    m.w32(vt + 0x28, stub(m, v28))
    m.w32(vt + 0x30, stub(m, v30))
    m.w32(definition + 0x1c, c['def_flags'])
    m.w32(definition + 0x38, *c['shape'])
    point = m.alloc(8)
    m.w32(point, *c['point'])
    direction = m.alloc(8)
    m.w32(direction, *c['direction'])
    calls = {'hit': 0}
    shape = c['alt_shape'] if c['alternate'] else c['shape']

    def s_hit(mm):
        assert mm.ecx() == gmap
        result, desc = mm.arg(0), mm.arg(1)
        assert m.r32(desc, 6) == shape, 'shape copy'
        assert m.r32(desc + 0x18) == definition
        events.append(['hit', *m.r32(desc + 0x4c, 2), m.r32(desc + 0x38), m.r32(desc + 0x3c),
                       *m.r32(desc + 0x44, 2), m.r32(desc + 0x54)])
        k = calls['hit']
        calls['hit'] += 1
        seq = c['hits']
        m.w32(result, seq[k] if k < len(seq) else c['hit_default'])
        mm.ret(8, result)

    m.seams[0x553ba0] = s_hit
    r = m.call(0x554500, ecx=gmap, args=(point, direction, definition, c['radius'], c['scale'],
                                          int(c['non_blocking']), int(c['alternate'])), count=5_000_000)
    assert m.final_esp == h.STACK + 4 + 0x1c
    assert m.fpu_top() == 0
    return {'result': r & 0xff, 'point': m.r32(point, 2), 'direction': m.r32(direction, 2),
            'events': events, 'tables': tables}


def cases():
    rnd = random.Random(0x554500)
    rows = []
    for i in range(420):
        a = rnd.uniform(0, 2 * math.pi)
        direction = rnd.choice([[0, f2b(-1.0)], [f2b(1.0), 0], [f2b(0.6), f2b(0.8)],
                                [f2b(math.cos(a)), f2b(math.sin(a))], [f2b(2.5), f2b(-1.5)], [0, 0]])
        kind = rnd.choice([1, 2, 3])
        hx, hy = rnd.choice([(0.5, 0.5), (1.25, 0.75), (2.0, 1.0)])
        shape = [kind, f2b(math.hypot(hx, hy)), f2b(-hx), f2b(-hy), f2b(hx), f2b(hy)]
        alt = [rnd.choice([1, 3]), f2b(3.0), f2b(-2.0), f2b(-1.0), f2b(2.0), f2b(1.0)]
        n = rnd.randrange(0, 34)
        hits = [rnd.choice([1, 1, 2]) for _ in range(n)] + ([0] if rnd.random() < 0.75 else [])
        rows.append({'name': f'spot{i}', 'cell': f2b(rnd.choice([1.0, 2.0, 4.0, 0.5])),
                     'point': [f2b(rnd.uniform(5, 300)), f2b(rnd.uniform(5, 300))],
                     'direction': direction,
                     'def_flags': rnd.choice([0, 1 << 21, 1 << 16, (1 << 16) | (1 << 21), 0x400]),
                     'v30': rnd.choice([0, 1, 1]), 'shape': shape, 'alt_shape': alt,
                     'radius': f2b(rnd.choice([1.0, 2.5, 0.75, 5.0, 1.0 / 3.0])),
                     'scale': f2b(rnd.choice([2.0, 4.0, 10.0, 0.0, 1.5])),
                     'non_blocking': rnd.random() < 0.5, 'alternate': rnd.random() < 0.3,
                     'hits': hits, 'hit_default': rnd.choice([0, 1, 1])})
    out = []
    for p in h.PRECISION:
        for c in rows:
            out.append({**copy.deepcopy(c), 'name': c['name'] + '_' + p, 'precision': p})
    return out


def main():
    rows = []
    for c in cases():
        try:
            rows.append({'input': c, 'output': run(c)})
        except Exception:
            print(c['name'])
            raise
    print({'cases': len(rows), 'found': sum(r['output']['result'] for r in rows),
           'snapped': sum(1 for r in rows if r['input']['v30'] and r['input']['def_flags'] >> 16 & 1)})
    h.write_document('native-open-spot.json', '0x554500 open spot ring search', rows)


if __name__ == '__main__':
    main()
