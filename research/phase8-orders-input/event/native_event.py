"""Original AA.exe mouse event creation KKC_Input::AddMouseEvent 0x59A6E0.

Runs, in both pinned images and at the three x87 precisions, the ORIGINAL
0x59A6E0 with its children 0x5C2770 (camera eye +0x90 and the 0x6FE590 ray
of the pixel: binary32 + _ftol 0x96CC4C, Y/B396D4 then X/B396D0) and the
event constructor 0x5997F0. Seams:
  0x404EA0   "Mouse code %d out of range" log (recorded)
  0x71D840   TerrainMap ray query on B36518 (records eye/direction/margin/
             secondary, then writes a scripted subset of the point words)
  0x59A500   event queue push (captures the 0x30-byte event)
Each case is a sequence of events on one KKC_Input (B39B18 layout: +0x28
modifiers, +0x44 per-code flags); after every call the event and the
Input+0x44 table are recorded. Writes native-event.json.
"""
from pathlib import Path
import sys, json, random, math, struct
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'research/phase8-ai'))
import native_machine as nm
from native_machine import Machine, f32, PRECISIONS

rng = random.Random(0x59A6E0)

def r32(lo, hi):
    return f32(rng.uniform(lo, hi))

def unit(v):
    l = math.sqrt(sum(x * x for x in v)) or 1.0
    return [x / l for x in v]

def camera_words():
    eye = [rng.uniform(-800, 800), rng.uniform(-800, 800), rng.uniform(20, 400)]
    d = unit([rng.uniform(-1, 1), rng.uniform(-1, 1), rng.uniform(-1, -0.2)])
    right = unit([d[1], -d[0], 0.0])
    up = [right[1] * d[2] - right[2] * d[1], right[2] * d[0] - right[0] * d[2], right[0] * d[1] - right[1] * d[0]]
    half = rng.uniform(0.3, 0.8)
    aspect = rng.choice([4 / 3, 16 / 9])
    words = {}
    for base, v in ((0x90, eye), (0x114, d), (0x120, up), (0x12C, right)):
        for i in range(3):
            words[base + 4 * i] = f32(v[i])
    words[0x138], words[0x13C], words[0x140], words[0x144] = f32(-half * aspect), f32(half * aspect), f32(half), f32(-half)
    return words

TABLE = 0x111

def run_case(precision, spec):
    m = Machine(precision)
    cam = m.alloc(0x300)
    for o, w in spec['camera']:
        m.w(cam + o, w)
    holder = m.alloc(4)
    m.w(holder, cam)
    m.w(0xB40048, holder)
    m.w(0xB396D0, spec['viewport'][0] & 0xffffffff)
    m.w(0xB396D4, spec['viewport'][1] & 0xffffffff)
    for i, w in enumerate(spec['seed']):
        m.w(0xB43CF8 + 4 * i, w)
    terrain = 0
    if spec['terrain'] != 'none':
        terrain = m.alloc(0x100)
        m.wb(terrain + 0xBC, 1 if spec['terrain'] == 'ready' else 0)
    m.w(0xB36518, terrain)
    m.w(0xB3653C, m.alloc(0x100))
    inp = m.alloc(0x600)
    calls = []
    def log(m):
        assert (m.arg(3), m.arg(7), m.arg(8)) == (0x107, 0x100, 0x110)
        calls.append(['log', m.arg(6)])
        m.ret(0, 0)
    def ray(m):
        assert m.ecx() == terrain
        eye = [m.r(m.arg(1) + 4 * i) for i in range(3)]
        point = m.arg(5)
        calls.append(['terrain', eye, [m.arg(2), m.arg(3), m.arg(4)], m.arg(6), m.arg(7)])
        for i, w in calls_writes.pop(0):
            m.w(point + 4 * i, w)
        m.ret(1, 0x1C)
    def push(m):
        assert m.ecx() == inp + 0x488
        calls.append(['event', [m.r(m.arg(1) + 4 * i) for i in range(12)]])
        m.ret(0, 4)
    m.seam(0x404EA0, log)
    m.seam(0x71D840, ray)
    m.seam(0x59A500, push)
    out = []
    for ev in spec['events']:
        m.w(0xB396C0, ev['kind'])
        m.w(inp + 0x28, ev['modifiers'])
        calls_writes = [list(ev['writes'])]
        del calls[:]
        m.call(0x59A6E0, ecx=inp, args=(ev['code'], ev['flags'], ev['x'] & 0xffff, ev['y'] & 0xffff, ev['delta'] & 0xffff), pop=0x14)
        table = [m.r(inp + 0x44 + 4 * i) for i in range(TABLE)]
        out.append({'calls': list(calls), 'table': table})
    m.verify_v14()
    return out, m.visited

def gen():
    cases = []
    for k in range(60):
        vp = rng.choice([(1024, 768), (800, 600), (1280, 1024), (1600, 900)])
        events = []
        for j in range(8):
            code = rng.choice([0x100, 0x101, 0x100, 0x101, 0x108, 0x109, 0x102, 0x110, 0x103, 0xFF])
            flags = rng.choice([1, 2, 1, 2, 0])
            x = rng.choice([rng.randrange(0, vp[0]), rng.randrange(-40, vp[0] + 40), rng.randrange(-32768, 32768)])
            y = rng.choice([rng.randrange(0, vp[1]), rng.randrange(-40, vp[1] + 40), rng.randrange(-32768, 32768)])
            writes = rng.choice([[], [(0, r32(-3000, 3000)), (1, r32(-3000, 3000)), (2, r32(-10, 90))],
                                 [(2, r32(-10, 90)), (0, r32(-3000, 3000)), (1, r32(-3000, 3000))], [(2, r32(-10, 90))]])
            events.append({'kind': 3 if rng.random() < 0.08 else rng.choice([0, 1, 2]), 'modifiers': rng.randrange(0, 8),
                           'code': code, 'flags': flags, 'x': x, 'y': y, 'delta': rng.randrange(-32768, 32768) if code == 0x110 else 0,
                           'writes': writes})
        cases.append({'camera': sorted(camera_words().items()), 'viewport': list(vp), 'seed': [r32(-5, 5) for _ in range(3)],
                      'terrain': rng.choice(['ready', 'ready', 'ready', 'loading', 'none']), 'events': events})
    return cases

def main():
    rows = []
    visited = {}
    cases = gen()
    for precision in PRECISIONS:
        for spec in cases:
            out, v = run_case(precision, spec)
            visited.update(v)
            rows.append({'precision': precision, 'args': spec, 'native': out})
    doc = nm.header(__file__, 'Unicorn 2.1.4, original and v14 images; seams 0x404EA0, 0x71D840 (scripted point writes), 0x59A500')
    doc['rows'] = rows
    doc['executed'] = nm.instructions(visited)
    if '--dry' in sys.argv:
        print(len(rows), 'rows', len(visited), 'instructions;', 'in 59A6E0:', sum(1 for a in visited if 0x59A6E0 <= a < 0x59A879),
              'in 5C2770:', sum(1 for a in visited if 0x5C2770 <= a < 0x5C27FE))
        return
    out = HERE / 'native-event.json'
    out.write_text(json.dumps(doc, indent=0))
    print(len(rows), 'rows', len(visited), 'instructions ->', out)

if __name__ == '__main__':
    main()
