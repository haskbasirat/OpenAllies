"""Original AA.exe drag-box selection geometry.

Runs, in both pinned images (original AA.exe and the v14 runtime) and at the
three x87 precisions, the ORIGINAL code of:
  0x436310  normalize(a x b) (the plane normal helper)
  0x436DD0  release-box planes between two events' world points (+0xC)
            from the current camera *B40048 (+0x90 eye, +0x114 direction,
            +0x24C point); its tail call 0x4367A0 is the seam that captures
            the four planes it passes.
  0x4371A0  drag-box planes for a screen rectangle (B396D0/B396D4 viewport,
            0x6FE590 corner rays, camera +0x90 eye); the selection manager's
            vt+0x44 (0x6406B0, clear candidates) is a recording stub and
            0x4367A0 again captures the planes.
  0x715850  AABB-corner classification against one plane.
  0x454B50 -> 0x715D40  model bound inside four planes; 0x86E8D0 (node world
            update through NiAVObject vt+0x48 / child vt+0x7C) is a seam that
            installs the scripted post-update rotation/translation and records
            the call.
Inputs are generated finite binary32 values. Writes native-box.json.
"""
from pathlib import Path
import sys, json, random, math
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'research/phase8-ai'))
import native_machine as nm
from native_machine import Machine, f32, from_f32, PRECISIONS

rng = random.Random(0xb0c5e1)

def r32(lo, hi):
    return f32(rng.uniform(lo, hi))

def vec(n, lo=-100.0, hi=100.0):
    return [r32(lo, hi) for _ in range(n)]

def unit(v):
    l = math.sqrt(sum(x * x for x in v)) or 1.0
    return [x / l for x in v]

def put(m, words):
    p = m.alloc(4 * max(len(words), 1))
    for i, w in enumerate(words):
        m.w(p + 4 * i, w)
    return p

def get(m, p, n):
    return [m.r(p + 4 * i) for i in range(n)]

CAMERA_OFFSETS = [0x90, 0x94, 0x98] + list(range(0x114, 0x148, 4)) + [0x24C, 0x250, 0x254]

def camera_words(kind):
    """A camera's words: eye +0x90, direction +0x114, up +0x120, right
    +0x12C, frustum left/right/top/bottom +0x138..+0x144, point +0x24C."""
    eye = [rng.uniform(-800, 800), rng.uniform(-800, 800), rng.uniform(20, 400)]
    if kind == 'random':
        words = {o: r32(-3, 3) for o in CAMERA_OFFSETS}
        for i, o in enumerate((0x90, 0x94, 0x98)):
            words[o] = f32(eye[i])
        for i, o in enumerate((0x24C, 0x250, 0x254)):
            words[o] = f32(eye[i] + rng.uniform(-300, 300))
        return words
    d = unit([rng.uniform(-1, 1), rng.uniform(-1, 1), rng.uniform(-1, -0.2)])
    up0 = [0.0, 0.0, 1.0]
    right = unit([d[1] * up0[2] - d[2] * up0[1], d[2] * up0[0] - d[0] * up0[2], d[0] * up0[1] - d[1] * up0[0]])
    up = [right[1] * d[2] - right[2] * d[1], right[2] * d[0] - right[0] * d[2], right[0] * d[1] - right[1] * d[0]]
    half = rng.uniform(0.3, 0.8)
    aspect = rng.choice([4 / 3, 16 / 9, 1.25])
    dist = rng.uniform(50, 900)
    point = [eye[i] + dist * d[i] for i in range(3)]
    if kind == 'eye-at-point':
        point = list(eye)
    words = {}
    for base, v in ((0x90, eye), (0x114, d), (0x120, up), (0x12C, right), (0x24C, point)):
        for i in range(3):
            words[base + 4 * i] = f32(v[i])
    words[0x138] = f32(-half * aspect)
    words[0x13C] = f32(half * aspect)
    words[0x140] = f32(half)
    words[0x144] = f32(-half)
    return words

class Run:
    def __init__(self, precision):
        self.precision = precision
        self.visited = {}

    def done(self, m):
        m.verify_v14()
        self.visited.update(m.visited)

def install_camera(m, words):
    cam = m.alloc(0x300)
    for o, w in words.items():
        m.w(cam + o, w)
    holder = m.alloc(4)
    m.w(holder, cam)
    m.w(0xB40048, holder)
    return cam

def capture_planes(m, out):
    def seam(m):
        out.append(get(m, m.arg(1), 16))
        m.ret(0, 4)
    m.seam(0x4367A0, seam)

def case_cross(run, a, c):
    m = Machine(run.precision)
    pa, pc = put(m, a), put(m, c)
    out = put(m, [0xabababab] * 3)
    eax = m.call(0x436310, ecx=pa, args=(out, pc), pop=8)
    assert eax == out
    run.done(m)
    return {'out': get(m, out, 3)}

def case_release(run, camera, s, e):
    m = Machine(run.precision)
    install_camera(m, camera)
    planes = []
    capture_planes(m, planes)
    ev1, ev2 = m.alloc(0x30), m.alloc(0x30)
    for i in range(3):
        m.w(ev1 + 0xC + 4 * i, s[i])
        m.w(ev2 + 0xC + 4 * i, e[i])
    this = m.alloc(0x40)
    m.call(0x436DD0, ecx=this, args=(ev1, ev2), pop=8)
    assert len(planes) == 1
    run.done(m)
    return {'planes': planes[0]}

def case_drag(run, camera, viewport, rect):
    m = Machine(run.precision)
    install_camera(m, camera)
    m.w(0xB396D0, viewport[0])
    m.w(0xB396D4, viewport[1])
    events = []
    planes = []
    def seam(m):
        events.append('planes')
        planes.append(get(m, m.arg(1), 16))
        m.ret(0, 4)
    m.seam(0x4367A0, seam)
    vt = m.alloc(0x80)
    def clear(m):
        events.append('clear')
        m.ret(0, 0)
    m.w(vt + 0x44, m.stub(clear))
    this = m.alloc(0x40)
    m.w(this, vt)
    m.call(0x4371A0, ecx=this, args=tuple(v & 0xffffffff for v in rect), pop=0x10)
    assert events == ['clear', 'planes'], events
    run.done(m)
    return {'planes': planes[0]}

def case_classify(run, plane, box):
    m = Machine(run.precision)
    pp, pb = put(m, plane), put(m, box)
    eax = m.call(0x715850, ecx=pp, args=(pb,), pop=4)
    run.done(m)
    return {'class': eax}

def case_inside(run, model, update, planes):
    m = Machine(run.precision)
    vworld = m.alloc(0x100)
    m.w(vworld + 0x8C, 0x13572468)
    m.w(0xB36538, vworld)
    node = m.alloc(0xC0)
    for i, w in enumerate(model['center']):
        m.w(node + 0x28 + 4 * i, w)
    m.w(node + 0x34, model['radius'])
    for i, w in enumerate(model['rotate']):
        m.w(node + 0x6C + 4 * i, w)
    for i, w in enumerate(model['translate']):
        m.w(node + 0x90 + 4 * i, w)
    resource = m.alloc(0x80)
    for i, w in enumerate(model['box']):
        m.w(resource + 0x4C + 4 * i, w)
    inst = m.alloc(0x20)
    m.w(inst + 8, resource)
    m.w(inst + 0xC, node)
    view = m.alloc(0x100)
    m.w(view + 0x78, inst)
    calls = []
    def update_seam(m):
        assert m.ecx() == node and m.arg(1) == 0x13572468 and m.arg(2) == 1
        calls.append(1)
        for i, w in enumerate(update['rotate']):
            m.w(node + 0x6C + 4 * i, w)
        for i, w in enumerate(update['translate']):
            m.w(node + 0x90 + 4 * i, w)
        m.ret(0, 8)
    m.seam(0x86E8D0, update_seam)
    pp = put(m, [w for p in planes for w in p])
    eax = m.call(0x454B50, ecx=view, args=(pp,), pop=4)
    run.done(m)
    return {'al': eax & 0xff, 'updated': len(calls)}

def random_plane(center=None, radius=None, u=None):
    n = unit([rng.uniform(-1, 1) for _ in range(3)])
    nb = [f32(x) for x in n]
    if center is None:
        return nb + [r32(-60, 60)]
    if u is None:
        u = rng.choice([rng.uniform(-1.4, 1.4), rng.uniform(-0.9, 0.9), rng.uniform(1.05, 3), rng.uniform(-3, -1.05)])
    dot = sum(from_f32(nb[i]) * from_f32(center[i]) for i in range(3))
    return nb + [f32(dot - u * from_f32(radius))]

def rotation():
    ax = unit([rng.uniform(-1, 1) for _ in range(3)])
    t = rng.uniform(-math.pi, math.pi)
    c, s, k = math.cos(t), math.sin(t), 1 - math.cos(t)
    x, y, z = ax
    mat = [c + x * x * k, x * y * k - z * s, x * z * k + y * s,
           y * x * k + z * s, c + y * y * k, y * z * k - x * s,
           z * x * k - y * s, z * y * k + x * s, c + z * z * k]
    scale = rng.choice([1.0, 1.0, rng.uniform(0.5, 2)])
    return [f32(v * scale) for v in mat]

def gen():
    cases = []
    for k in range(40):
        a, c = vec(3, -50, 50), vec(3, -50, 50)
        if k == 0:
            c = list(a)
        if k == 1:
            c = [f32(2 * from_f32(v)) for v in a]
        if k == 2:
            a, c = [0, 0, 0], vec(3)
        if k == 3:
            a, c = [f32(1e-4), 0, 0], [0, f32(1e-3), 0]
        cases.append(('cross', [a, c]))
    for k in range(60):
        kind = 'random' if k % 6 == 5 else ('eye-at-point' if k == 7 else 'look')
        cam = camera_words(kind)
        s = [r32(-1000, 1000), r32(-1000, 1000), r32(-5, 60)]
        e = [f32(from_f32(s[0]) + rng.uniform(-300, 300)), f32(from_f32(s[1]) + rng.uniform(-300, 300)), r32(-5, 60)]
        if k % 10 == 3:
            e = list(s)
        if k % 10 == 4:
            e = [s[0], e[1], s[2]]
        cases.append(('release', [[[o, w] for o, w in sorted(cam.items())], s, e]))
    for k in range(60):
        cam = camera_words('random' if k % 6 == 5 else 'look')
        vp = rng.choice([(1024, 768), (800, 600), (1280, 1024), (1600, 900), (640, 480)])
        x0, y0 = rng.randrange(-20, vp[0] + 20), rng.randrange(-20, vp[1] + 20)
        x1, y1 = rng.randrange(-20, vp[0] + 20), rng.randrange(-20, vp[1] + 20)
        if k % 9 == 2:
            x1, y1 = x0, y0
        if k % 9 == 4:
            x1 = x0
        cases.append(('drag', [[[o, w] for o, w in sorted(cam.items())], list(vp), [x0, y0, x1, y1]]))
    for k in range(80):
        lo = vec(3, -30, 0)
        hi = [f32(from_f32(lo[i]) + rng.uniform(0.2, 40)) for i in range(3)]
        n = unit([rng.uniform(-1, 1) for _ in range(3)])
        nb = [f32(x) for x in n]
        corner = [lo[i] if rng.random() < 0.5 else hi[i] for i in range(3)]
        dot = sum(from_f32(nb[i]) * from_f32(corner[i]) for i in range(3))
        if k % 5 == 0:
            constant = f32(dot)              # through a corner
        elif k % 5 == 1:
            constant = f32(dot + rng.uniform(-60, 60))
        else:
            constant = r32(-80, 80)
        if k == 2:
            nb = [0, 0, f32(1.0)]
            constant = lo[2]                 # exactly on the min face
        cases.append(('classify', [nb + [constant], lo + hi]))
    for k in range(150):
        center = vec(3, -80, 80)
        radius = r32(0.5, 40)
        mode = k % 3
        planes = []
        for _ in range(4):
            if mode == 0:
                u = rng.uniform(1.05, 3)
            elif mode == 1:
                u = rng.uniform(-0.95, 0.95)
            else:
                u = None
            planes.append(random_plane(center, radius, u))
        if k % 10 == 9:
            planes = [random_plane() for _ in range(4)]
        lo = vec(3, -20, 0)
        hi = [f32(from_f32(lo[i]) + rng.uniform(0.5, 25)) for i in range(3)]
        rot = rotation()
        post = rot if k % 4 else rotation()
        bc = [(from_f32(lo[i]) + from_f32(hi[i])) / 2 for i in range(3)]
        rb = [sum(from_f32(post[3 * r + c]) * bc[c] for c in range(3)) for r in range(3)]
        post_t = [f32(from_f32(center[i]) - rb[i] + rng.uniform(-2, 2)) for i in range(3)]
        model = {'center': center, 'radius': radius, 'rotate': rot, 'translate': vec(3, -80, 80), 'box': lo + hi}
        update = {'rotate': post, 'translate': post_t}
        cases.append(('inside', [model, update, planes]))
    return cases

FUNS = {'cross': case_cross, 'release': case_release, 'drag': case_drag, 'classify': case_classify, 'inside': case_inside}

def native_args(kind, args):
    if kind in ('release', 'drag'):
        return [{o: w for o, w in args[0]}] + args[1:]
    return args

def main():
    cases = gen()
    rows = []
    visited = {}
    for precision in PRECISIONS:
        run = Run(precision)
        for kind, args in cases:
            out = FUNS[kind](run, *native_args(kind, args))
            rows.append({'precision': precision, 'kind': kind, 'args': args, 'native': out})
        visited.update(run.visited)
    doc = nm.header(__file__, 'Unicorn 2.1.4, original and v14 images; seams: 0x4367A0 (plane capture), '
                    'selection vt+0x44 stub, 0x86E8D0 (scripted node update)')
    doc['rows'] = rows
    doc['executed'] = nm.instructions(visited)
    if '--dry' in sys.argv:
        from collections import Counter
        print(len(rows), 'rows', len(visited), 'instructions')
        print(Counter((r['kind'], json.dumps(r['native'].get('al', r['native'].get('class', '')))) for r in rows if r['precision'] == 'Double53'))
        for lo, hi in ((0x436310, 0x4363A2), (0x436DD0, 0x437195), (0x4371A0, 0x43748F), (0x715850, 0x7159B6), (0x715D40, 0x715E81)):
            print(hex(lo), sum(1 for a in visited if lo <= a < hi))
        return
    out = HERE / 'native-box.json'
    out.write_text(json.dumps(doc, indent=0))
    print(len(rows), 'rows', len(visited), 'instructions ->', out)

if __name__ == '__main__':
    main()
