"""KKC_GMap::FindOpenPositionLine 0x553FA0 and
KKC_GMap::FindOpenPositionLineThenRadius 0x554BF0.

Whole routines execute unchanged with 0x552F70 (descriptor constructor) and
0x46E0A0 (shape copy). Seams (recorded in order, scripted results): the map
clamp 0x552640 (a scripted box: below the low edge takes it, above the high
edge takes it), the first-hit query 0x553BA0 (records the descriptor's
position, required flags, mask, facing, +54 and +18 words and checks its
shape copy; answers a scripted result word per call) and, for 0x554BF0, the
radius search 0x554B30 (records its arguments; scripted success and point).
The descriptor +18 word is recorded to show that the 0x46EE00 snap guard is
never met. The diagnostic 0x404F60 body returns (log level 0).

Usage: python native_open_position.py [path/to/AA.exe]
Writes native-open-position.json next to this script.
"""
import os, sys
from pathlib import Path
if len(sys.argv) > 1:
    os.environ['AA_GAME_DIR'] = str(Path(sys.argv[1]).resolve().parent)
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import copy, random  # noqa: E402
import harness as h  # noqa: E402
from harness import f2b, b2f  # noqa: E402


def run(c):
    m = h.Machine(c['precision'])
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
    m.w32(definition + 0x1c, c['def_flags'])
    m.w32(definition + 0x38, *c['shape'])
    point = m.alloc(8)
    m.w32(point, *c['point'])
    facing = m.alloc(8)
    m.w32(facing, *c['facing'])
    lo, hi = c['box']
    calls = {'hit': 0}

    def s_clamp(mm):
        assert mm.ecx() == gmap
        p = mm.arg(0)
        x, y = m.r32(p, 2)
        events.append(['clamp', x, y])
        out = []
        for v in (x, y):
            if b2f(v) < b2f(lo):
                v = lo
            elif b2f(v) > b2f(hi):
                v = hi
            out.append(v)
        m.w32(p, *out)
        mm.ret(4)

    def s_hit(mm):
        assert mm.ecx() == gmap
        result, desc = mm.arg(0), mm.arg(1)
        assert m.r32(desc, 6) == c['shape'], 'shape copy'
        events.append(['hit', *m.r32(desc + 0x4c, 2), m.r32(desc + 0x38), m.r32(desc + 0x3c),
                       *m.r32(desc + 0x44, 2), m.r32(desc + 0x54), m.r32(desc + 0x18)])
        k = calls['hit']
        calls['hit'] += 1
        seq = c['hits']
        m.w32(result, seq[k] if k < len(seq) else c['hit_default'])
        mm.ret(8, result)

    def s_radius(mm):
        assert mm.ecx() == gmap
        p, f, d, radius, a5, a6 = (mm.arg(i) for i in range(6))
        assert f == facing and d == definition
        events.append(['radius', *m.r32(p, 2), radius, a5, a6])
        if c['radius_ok']:
            m.w32(p, *c['radius_point'])
        mm.ret(0x18, int(c['radius_ok']))

    m.seams[0x552640] = s_clamp
    m.seams[0x553ba0] = s_hit
    m.seams[0x554b30] = s_radius
    if c['entry'] == 'line':
        r = m.call(0x553fa0, ecx=gmap, args=(point, facing, definition, *c['from'], int(c['non_blocking'])),
                   count=5_000_000)
    else:
        r = m.call(0x554bf0, args=(point, facing, definition, *c['from'], c['radius']), count=5_000_000)
    assert m.final_esp == h.STACK + 4 + 0x18
    assert m.fpu_top() == 0
    return {'result': r & 0xff, 'point': m.r32(point, 2), 'events': events}


def ulp(bits, k):
    out = bits + k
    if out < 0 or out > 0xffffffff or (out ^ bits) >> 31 or out & 0x7f800000 == 0x7f800000:
        return bits
    return out


def cases():
    rnd = random.Random(0x553fa0)
    rows = []
    for i in range(700):
        cell = f2b(rnd.choice([1.0, 2.0, 4.0, 0.5, 3.0]))
        fx, fy = rnd.uniform(5, 200), rnd.uniform(5, 200)
        pick = rnd.randrange(10)
        if pick == 0:
            px, py = fx, fy
        elif pick == 1:
            px, py = fx + rnd.choice([1e-4, -5e-5, 2e-4]), fy
        else:
            d = rnd.choice([0.5, 1.0, 3.0, 7.5, 12.0, 25.0, rnd.uniform(0, 30)])
            a = rnd.uniform(0, 6.283)
            import math
            px, py = fx + d * math.cos(a), fy + d * math.sin(a)
        frm = [f2b(fx), f2b(fy)]
        pt = [f2b(px), f2b(py)]
        if pick == 2:
            pt = [ulp(frm[0], rnd.choice([-3, -1, 1, 4])), frm[1]]
        kind = rnd.choice([1, 2, 3])
        radius = rnd.choice([0.0, 0.1, 0.3, 1.0, 2.5, 0.75])
        hx, hy = rnd.choice([(0.5, 0.5), (1.25, 0.75), (2.0, 1.0)])
        shape = [kind, f2b(radius), f2b(-hx), f2b(-hy), f2b(hx), f2b(hy)]
        n = rnd.randrange(0, 12)
        hits = [rnd.choice([1, 1, 2, 3]) for _ in range(n)] + ([0] if rnd.random() < 0.8 else [])
        lo = f2b(rnd.choice([0.0, 2.0, 50.0]))
        hi = f2b(rnd.choice([1000.0, 150.0, 100.0]))
        rows.append({'name': f'open{i}', 'entry': rnd.choice(['line', 'line', 'then_radius']),
                     'cell': cell, 'point': pt, 'from': frm,
                     'facing': rnd.choice([[0, f2b(-1.0)], [f2b(0.6), f2b(0.8)]]),
                     'def_flags': rnd.choice([0, 1 << 21, 0x400, (1 << 21) | 0x20000000]),
                     'shape': shape, 'non_blocking': rnd.random() < 0.5, 'box': [lo, hi],
                     'hits': hits, 'hit_default': rnd.choice([0, 1]),
                     'radius': f2b(rnd.choice([5.0, 10.0, 0.0])), 'radius_ok': rnd.random() < 0.6,
                     'radius_point': [f2b(rnd.uniform(0, 200)), f2b(rnd.uniform(0, 200))]})
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
    print({'cases': len(rows), 'max_hits': max(sum(e[0] == 'hit' for e in r['output']['events']) for r in rows),
           'moved': sum(r['output']['point'] != r['input']['point'] for r in rows)})
    h.write_document('native-open-position.json', '0x553FA0 0x554BF0 open position line', rows)


if __name__ == '__main__':
    main()
