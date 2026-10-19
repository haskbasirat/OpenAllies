"""Original AA.exe pure leaves of the battle picking/order input path.

Runs, in both pinned images (original AA.exe and the v14 runtime) and at the
three x87 precisions, the ORIGINAL code of:
  0x875700 ray-sphere, 0x8707E0 3x3 inverse, 0x870720 3x3*vector,
  0x715430 ray-AABB, 0x436480 distance, 0x42D750 normalize3,
  0x44B600 normalize2, 0x640A90 XY centroid of a view list.
No seam is installed: every executed instruction is original. Inputs are
generated finite binary32 values; outputs are the raw words/AL/ST0.
Writes native-pick-math.json next to this file.
"""
from pathlib import Path
import sys, json, random, struct
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'research/phase8-ai'))
import native_machine as nm
from native_machine import Machine, f32, from_f32, PRECISIONS

rng = random.Random(0x5e1ec7)

def r32(lo, hi):
    return f32(rng.uniform(lo, hi))

def vec(n, lo=-100.0, hi=100.0):
    return [r32(lo, hi) for _ in range(n)]

def put(m, words):
    p = m.alloc(4 * max(len(words), 1))
    for i, w in enumerate(words):
        m.w(p + 4 * i, w)
    return p

def get(m, p, n):
    return [m.r(p + 4 * i) for i in range(n)]

class Run:
    def __init__(self, precision):
        self.precision = precision
        self.visited = {}

    def machine(self):
        m = Machine(self.precision)
        return m

    def done(self, m):
        m.verify_v14()
        self.visited.update(m.visited)

def case_sphere(run, o, d, c, r):
    m = run.machine()
    po, pd, pc = put(m, o), put(m, d), put(m, c)
    eax = m.call(0x875700, args=(po, pd, pc, r), pop=0)
    run.done(m)
    return {'al': eax & 0xff}

def case_inverse(run, mat):
    m = run.machine()
    pm = put(m, mat)
    out = put(m, [0xabababab] * 9)
    eax = m.call(0x8707e0, ecx=pm, args=(out,), pop=4)
    run.done(m)
    return {'al': eax & 0xff, 'out': get(m, out, 9)}

def case_transform(run, mat, v):
    m = run.machine()
    pm, pv = put(m, mat), put(m, v)
    out = put(m, [0xabababab] * 3)
    m.call(0x870720, ecx=pm, args=(out, pv), pop=8)
    run.done(m)
    return {'out': get(m, out, 3)}

def case_box(run, box, o, d):
    m = run.machine()
    pb, po, pd = put(m, box), put(m, o), put(m, d)
    eax = m.call(0x715430, ecx=pb, args=(po, pd), pop=8)
    run.done(m)
    return {'al': eax & 0xff}

def case_distance(run, a, c):
    m = run.machine()
    pa, pc = put(m, a), put(m, c)
    m.call(0x436480, args=(pa, pc), pop=0)
    st = m.st0()
    run.done(m)
    return {'st0': st}

def case_normalize3(run, v):
    m = run.machine()
    pv = put(m, v)
    m.call(0x42d750, ecx=pv, args=(), pop=0)
    st = m.st0()
    run.done(m)
    return {'out': get(m, pv, 3), 'st0_f32': st['f32_bits']}

def case_normalize2(run, v):
    m = run.machine()
    pv = put(m, v)
    m.call(0x44b600, ecx=pv, args=(), pop=0)
    st = m.st0()
    run.done(m)
    return {'out': get(m, pv, 2), 'st0_f32': st['f32_bits']}

def case_centroid(run, points):
    m = run.machine()
    head = 0
    # Build the {obj, next} list so that traversal order equals `points`.
    nodes = []
    for (x, y) in points:
        obj = m.alloc(0x60)
        m.w(obj + 0x38, x)
        m.w(obj + 0x3c, y)
        nodes.append(obj)
    for obj in reversed(nodes):
        node = m.alloc(8)
        m.w(node, obj)
        m.w(node + 4, head)
        head = node
    lst = put(m, [head, 0])
    out = put(m, [0xabababab, 0xabababab])
    m.call(0x640a90, ecx=0, args=(out, lst), pop=8)
    run.done(m)
    return {'out': get(m, out, 2)}

def gen():
    cases = []
    for k in range(40):
        o = vec(3)
        d = vec(3, -1, 1)
        c = vec(3)
        r = r32(0.1, 60.0)
        if k % 5 == 0:
            # aim at the centre
            c = [f32(from_f32(o[i]) + 20 * from_f32(d[i])) for i in range(3)]
        cases.append(('sphere', [o, d, c, r]))
    for k in range(30):
        mat = vec(9, -2, 2)
        if k == 0:
            mat = [f32(v) for v in (1, 0, 0, 0, 1, 0, 0, 0, 1)]
        if k == 1:
            mat = [f32(v) for v in (1, 2, 3, 2, 4, 6, 1, 1, 1)]  # singular
        if k == 2:
            mat = [f32(v) for v in (0, -1, 0, 1, 0, 0, 0, 0, 2)]
        cases.append(('inverse', [mat]))
        cases.append(('transform', [mat, vec(3)]))
    for k in range(60):
        lo = vec(3, -20, 0)
        hi = [f32(from_f32(lo[i]) + rng.uniform(0.5, 30)) for i in range(3)]
        box = lo + hi
        o = vec(3, -60, 60)
        d = vec(3, -1, 1)
        if k % 4 == 0:
            tgt = [rng.uniform(from_f32(lo[i]), from_f32(hi[i])) for i in range(3)]
            d = [f32(tgt[i] - from_f32(o[i])) for i in range(3)]
        if k % 7 == 0:
            d[rng.randrange(3)] = 0
        if k % 11 == 0:
            d = [0, 0, f32(-1.0)]
        cases.append(('box', [box, o, d]))
    for k in range(30):
        cases.append(('distance', [vec(3, -500, 500), vec(3, -500, 500)]))
    for k in range(30):
        v = vec(3, -50, 50)
        if k == 0:
            v = [0, 0, 0]
        if k == 1:
            v = [f32(1e-7), 0, 0]
        cases.append(('normalize3', [v]))
        v2 = vec(2, -50, 50)
        if k == 0:
            v2 = [0, 0]
        if k == 1:
            v2 = [f32(5e-7), f32(-5e-7)]
        cases.append(('normalize2', [v2]))
    for k in range(25):
        n = rng.randint(1, 9)
        cases.append(('centroid', [[(r32(-300, 300), r32(-300, 300)) for _ in range(n)]]))
    return cases

FUNS = {'sphere': case_sphere, 'inverse': case_inverse, 'transform': case_transform, 'box': case_box,
        'distance': case_distance, 'normalize3': case_normalize3, 'normalize2': case_normalize2,
        'centroid': case_centroid}

def main():
    cases = gen()
    rows = []
    visited = {}
    for precision in PRECISIONS:
        run = Run(precision)
        for kind, args in cases:
            out = FUNS[kind](run, *args)
            rows.append({'precision': precision, 'kind': kind, 'args': args, 'native': out})
        visited.update(run.visited)
    doc = nm.header(__file__, 'Unicorn 2.1.4, original and v14 images, no seams')
    doc['rows'] = rows
    doc['executed'] = nm.instructions(visited)
    out = HERE / 'native-pick-math.json'
    out.write_text(json.dumps(doc, indent=0))
    print(len(rows), 'rows', len(visited), 'instructions ->', out)

if __name__ == '__main__':
    main()
