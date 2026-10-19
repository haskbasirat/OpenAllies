"""Producer of native-terrain-visibility.json: the ORIGINAL visibility pass.

Builds laboratory TerrainMaps with the original 721330/723E00 (as in
../terrain-map/native_terrain_map.py, whose Lab and seams are reused; here
4F0E50 returns deterministic laboratory heights so 71AFD0 builds non-trivial
block spheres), constructs VWorld+94 with the original 430290, and then runs,
frame after frame, in both pinned images and at PC24/PC53/PC64:

  4312F0  42EED0 (camera copy, 4x 71D840 into VWorld+144 via 43B2F0,
          869C90 base-height planes, 8x 42ED60), 430E50 (footprint, 42F330,
          430710 rasterizer with 430080 / 42F1F0 (42ED10) / 4300D0 callbacks,
          42FE80 / 71A230), 42FF50 (720080, 42F980, 42FED0, 42FE10 with the
          block vtable Request 721180 / Load 7212D0 -> 720AD0 / Release
          7211F0 -> 721030, 7208D0 incl. 737470/738280 water).
  725340  only its 725429 branch: 71A750 while +C0, then +C0 = 0.
  71D840  end-to-end rays on the final natively built memory.

Seams (explicit, recorded): everything listed in native_terrain_map.Seams,
plus 4F0E50 (laboratory heights), 42FAD0 and 42F670 (object visibility),
720670 (block texture array; +88 set), 5AA3F0 / 5AA550 (resource request
count +4 modelled: += / -= delta), 5AAE10, 71F250, 71A1A0, 71C470, 71CCB0,
71CD00, 7208D0's 71B500/7203F0/71C290/71CE70/4325C0/71F3B0, and the 720AD0
tail 720C79 -> 72100E (stores only block +124..+14D; 72100E..72102D then
runs natively, including the +14E = 0 store). The camera words are
explicit laboratory inputs (a pinhole frustum: inward planes, corner rays
+2D4/+2E0/+2EC/+2F8 = left-top, right-top, left-bottom, right-bottom).

  python native_terrain_visibility.py --write | (replay) | --dry
"""
from pathlib import Path
import hashlib, json, math, random, struct, sys
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'research/phase8-ai'))
sys.path.insert(0, str(ROOT / 'research/phase8-orders-input/terrain-map'))
import native_machine as nm  # noqa: E402
from native_machine import f32, original, modified  # noqa: E402
import native_terrain_map as tm  # noqa: E402
from unicorn.x86_const import UC_X86_REG_EIP  # noqa: E402

OUT = HERE / 'native-terrain-visibility.json'
CAMERA_HOLDER, STATS = 0xB40048, 0xB39BB8
RANGES = {'4312F0': (0x4312f0, 0x431328), '42EED0': (0x42eed0, 0x42f1f0), '42F1F0': (0x42f1f0, 0x42f267),
          '42ED10': (0x42ed10, 0x42ed5c), '869C90': (0x869c90, 0x869ccc), '430E50': (0x430e50, 0x4312d2),
          '430710': (0x430710, 0x430e1a), '42F330': (0x42f330, 0x42f442), '430080': (0x430080, 0x4300c6),
          '4300D0': (0x4300d0, 0x430116), '42FE80': (0x42fe80, 0x42fecb), '71A230': (0x71a230, 0x71a24d),
          '42FF50': (0x42ff50, 0x430045), '720080': (0x720080, 0x720128), '42F980': (0x42f980, 0x42f9c5),
          '42F7D0': (0x42f7d0, 0x42f82f), '42FED0': (0x42fed0, 0x42ff46), '42FE10': (0x42fe10, 0x42fe7e),
          '721180': (0x721180, 0x7211ea), '7212D0': (0x7212d0, 0x721323), '7211F0': (0x7211f0, 0x7212cc),
          '7208D0': (0x7208d0, 0x720ac2), '43B2F0': (0x43b2f0, 0x43b35d), '430290': (0x430290, 0x430328),
          '71AFD0': (0x71afd0, 0x71b4fc), '96D5E0-floor': (0x96d5e0, 0x96d6d0)}


def flt(bits):
    return struct.unpack('<f', struct.pack('<I', bits & 0xffffffff))[0]


def ground_value(x_bits, y_bits, flags):
    """Laboratory 4F0E50: a smooth binary32 surface, +0.25 at flags 0x123."""
    x, y = flt(x_bits), flt(y_bits)
    v = 1.5 * math.sin(x / 7.0) + 1.25 * math.cos(y / 5.0) + 0.03125 * ((int(x) * 7 + int(y) * 3) % 11)
    if flags == 0x123:
        v += 0.25
    return f32(v)


class VisLab(tm.Lab):
    def __init__(self, precision, spec):
        super().__init__(precision, spec)
        m, S = self.m, self.seams
        self.ground = {}

        def ground(m):
            xy = m.arg(1)
            x, y, flags = m.r(xy), m.r(xy + 4), m.arg(2)
            v = ground_value(x, y, flags)
            self.ground[(x, y, flags)] = v
            S.count('4F0E50 ground')
            m.ret_st0_f32(v, 8)
        m.seam(0x4F0E50, ground)
        gw = m.r(tm.GWORLD)
        base = m.alloc(0x60)
        m.w(gw + 0x34, base)
        m.w(base + 0x48, spec['base_limits'][0])
        m.w(base + 0x4c, spec['base_limits'][1])
        self.camera = m.alloc(0x400)
        holder = m.alloc(4)
        m.w(holder, self.camera)
        m.w(CAMERA_HOLDER, holder)
        m.w(STATS, m.alloc(0x60))
        uv = m.r(tm.UVARS)
        m.w(uv + 0x190, m.array([0]))
        # Mesh objects: vt+4 (7211F0 RTTI walk) returns NULL, vt+0 deletes.
        mesh_vt = m.alloc(0x40)
        m.w(mesh_vt + 0, m.stub(lambda m: m.ret(0, 4)))
        m.w(mesh_vt + 4, m.stub(lambda m: m.ret(0, 0)))
        original_mesh = m.seams[0x720130]

        def mesh(m):
            out = m.arg(1)
            original_mesh(m)
            m.w(m.r(out), mesh_vt)
        m.seam(0x720130, mesh)

        def named(name, pop, value=0):
            def run(m):
                S.count(name)
                m.ret(value, pop)
            return run
        self.view_calls = []

        def sweep(m):
            S.count('42FAD0 view cells')
            block = m.arg(1)
            self.view_calls.append(['42FAD0', m.r(block + 0xac), m.r(block + 0xb0), m.r(block + 0xb8)])
            m.ret(0, 4)
        m.seam(0x42FAD0, sweep)

        def traverse(m):
            S.count('42F670 view traversal')
            self.view_calls.append(['42F670'])
            m.ret(0, 0)
        m.seam(0x42F670, traverse)
        for a, pop in ((0x5AAE10, 0), (0x71F250, 0), (0x71A1A0, 0), (0x71C470, 0),
                       (0x71CCB0, 0), (0x71CD00, 0), (0x71B500, 0), (0x7203F0, 4), (0x71C290, 8), (0x71CE70, 0),
                       (0x4325C0, 4), (0x71F3B0, 0)):
            m.seam(a, named('%X' % a, pop))

        def textures(m):
            S.count('720670 textures')
            m.w(m.ecx() + 0x88, m.alloc(16))
            m.ret(0, 0)
        m.seam(0x720670, textures)
        self.requests = []

        def request(m):
            res, delta, mode = m.ecx(), m.arg(1), m.arg(2)
            m.w(res + 4, m.r(res + 4) + delta)
            self.requests.append(('request', res, delta, mode))
            assert delta == 1 and mode == 4 and m.r(res + 4) == 1, (delta, mode, m.r(res + 4))
            m.ret(0, 8)
        m.seam(0x5AA3F0, request)

        def release(m):
            res, delta = m.ecx(), m.arg(1)
            m.w(res + 4, (m.r(res + 4) - delta) & 0xffffffff)
            self.requests.append(('release', res, delta))
            m.ret(0, 4)
        m.seam(0x5AA550, release)
        m.seam(0x720C79, lambda m: m.u.reg_write(UC_X86_REG_EIP, 0x72100E))

    def build(self):
        built = self.construct()
        m = self.m
        vis = self.vis = m.alloc(0x110)
        assert m.call(0x430290, ecx=vis, pop=0) == vis
        m.w(m.r(tm.VWORLD) + 0x94, vis)
        return built

    def set_camera(self, words):
        m, c = self.m, self.camera
        for o, v in words.items():
            m.w(c + o, v)

    def frame(self):
        m, t = self.m, self.map
        m.call(0x4312F0, ecx=self.vis, pop=0, count=50_000_000)
        after_pass = tm.map_fields(m, t)
        if m.rb(t + 0xc0):                       # 725340 at 725429
            m.call(0x71A750, ecx=t, pop=0)
            m.wb(t + 0xc0, 0)
        return after_pass

    def snapshot(self):
        m, t, vis = self.m, self.map, self.vis
        bx, by = m.r(t + 0x84), m.r(t + 0x88)
        cells = m.r(t + 0xb8)
        index = {m.r(cells + 4 * i): i for i in range(bx * by)}
        blocks = []
        for i in range(bx * by):
            c = m.r(cells + 4 * i)
            blocks.append(dict(bc=m.r(c + 0xbc), b8=m.r(c + 0xb8), b4=m.r(c + 0xb4), active=m.rb(c + 0x140),
                               requests=m.r(c + 4), mesh=m.r(c + 0x44) != 0, water=m.rb(c + 0x9c), water_dirty=m.rb(c + 0x14f),
                               refresh=m.rb(c + 0x14e)))
        active = []
        node = m.r(vis + 4)
        while node:
            active.append(index[m.r(node)])
            node = m.r(node + 4)
        hits = [m.r(vis + o) for o in range(0x28, 0x88, 4)]
        vw = m.r(tm.VWORLD)
        footprint = [m.r(vw + o) for o in range(0x144, 0x174, 4)]
        mf = tm.map_fields(m, t)
        return dict(blocks=blocks, active=active, hits=hits, footprint=footprint,
                    map=dict(min=mf['min'], max=mf['max'], min_holder=mf['min_holder'], max_holder=mf['max_holder'],
                             recompute=mf['recompute'], dirty=mf['dirty']))

    def spheres(self):
        m, t = self.m, self.map
        bx, by = m.r(t + 0x84), m.r(t + 0x88)
        cells = m.r(t + 0xb8)
        return [[m.r(m.r(cells + 4 * i) + o) for o in (0xe8, 0xec, 0xf0, 0xf4)] for i in range(bx * by)]


def norm(v):
    l = math.sqrt(sum(x * x for x in v))
    return [x / l for x in v]


def cross(a, c):
    return [a[1] * c[2] - a[2] * c[1], a[2] * c[0] - a[0] * c[2], a[0] * c[1] - a[1] * c[0]]


def dot(a, c):
    return sum(x * y for x, y in zip(a, c))


def camera_words(eye, yaw, pitch, hfov, aspect, near, far):
    d = [math.cos(pitch) * math.cos(yaw), math.cos(pitch) * math.sin(yaw), -math.sin(pitch)]
    d = [0.0 if abs(x) < 1e-12 else x for x in d]          # exact axis yaws: horizontal footprint edges
    right = [0.0 if abs(x) < 1e-12 else x for x in norm(cross(d, [0, 0, 1]))]
    up = cross(right, d)
    h, v = math.tan(hfov), math.tan(hfov) / aspect
    corner = lambda sh, sv: [d[i] + sv * v * up[i] + sh * h * right[i] for i in range(3)]
    lt, rt, lb, rb = corner(-1, 1), corner(1, 1), corner(-1, -1), corner(1, -1)
    planes = []
    planes.append((d, dot(d, [eye[i] + near * d[i] for i in range(3)])))
    nd = [-x for x in d]
    planes.append((nd, dot(nd, [eye[i] + far * d[i] for i in range(3)])))
    for a, c in ((lb, lt), (rt, rb), (lt, rt), (rb, lb)):
        nn = norm(cross(a, c))
        if dot(nn, d) < 0:
            nn = [-x for x in nn]
        planes.append((nn, dot(nn, eye)))
    words = {}
    for i, x in enumerate(eye):
        words[0x90 + 4 * i] = f32(x)
    for i, x in enumerate(d):
        words[0x114 + 4 * i] = f32(x)
    for k, (nn, kk) in enumerate(planes):
        for i in range(3):
            words[0x19c + 16 * k + 4 * i] = f32(nn[i])
        words[0x19c + 16 * k + 12] = f32(kk)
    for k, r in enumerate((lt, rt, lb, rb)):
        for i in range(3):
            words[0x2d4 + 12 * k + 4 * i] = f32(r[i])
    return words


def specs():
    rng = random.Random(0x4312f0)
    out = {}

    def spec(name, extents, gtile, block, quad, columns, rows, step, kind, limits, frames):
        s = dict(extents=[f32(extents[0]), f32(extents[1])], gtile=f32(gtile), block=block, quad=quad,
                 heightfield=tm.heightfield(rng, columns, rows, f32(step), kind), base_limits=[f32(limits[0]), f32(limits[1])])
        gw, gh = int(extents[0] / gtile), int(extents[1] / gtile)
        s['tiles'] = dict(size=[gw, gh], classes=[(i * 7) % 3 % 2 for i in range(gw * gh)])  # land/mountain only
        s['frames'] = frames
        out[name] = s
    w = 64.0
    path = [([8, -20, 30], 1.2, 0.6), ([16, -10, 26], 1.3, 0.7), ([30, 4, 22], 1.5, 0.75), ([40, 20, 18], 1.9, 0.55),
            ([60, 40, 34], 2.6, 0.9), ([32, 32, 60], 0.3, 1.45), ([10, 50, 12], -0.6, 0.35), ([-10, 30, 20], 0.0, 0.5),
            ([50, 70, 25], -1.6, 0.6), ([32, 32, 14], 0.9, 0.3), ([32, -30, 15], math.pi / 2, 0.15),
            ([-20, 32, 25], 0.0, 0.8), ([8, -20, 30], 1.2, 0.6)]
    spec('retail-64', [w, w], 2.0, 8, 2, 33, 33, 2.0, 'ridge', [-3.0, 5.5],
         [dict(eye=e, yaw=y, pitch=p, hfov=0.5, aspect=4 / 3, near=1.0, far=70.0) for e, y, p in path])
    spec('wide-48x32', [48.0, 32.0], 2.0, 8, 2, 25, 17, 2.0, 'float', [-4.0, 6.0],
         [dict(eye=[rng.uniform(-10, 58), rng.uniform(-10, 42), rng.uniform(8, 40)], yaw=rng.uniform(-3, 3),
               pitch=rng.uniform(0.3, 1.3), hfov=rng.choice([0.35, 0.5, 0.7]), aspect=rng.choice([4 / 3, 16 / 9]),
               near=1.0, far=rng.choice([30.0, 60.0, 120.0])) for _ in range(9)])
    spec('block12-36', [36.0, 36.0], 2.0, 12, 2, 19, 19, 2.0, 'eighths', [-2.0, 5.0],
         [dict(eye=[rng.uniform(-6, 42), rng.uniform(-6, 42), rng.uniform(10, 35)], yaw=rng.uniform(-3, 3),
               pitch=rng.uniform(0.4, 1.2), hfov=0.5, aspect=4 / 3, near=1.0, far=rng.choice([40.0, 80.0])) for _ in range(8)])
    return out


def rays(rng, st, n, extents):
    w, h = flt(extents[0]), flt(extents[1])
    lo, hi = flt(st['map']['min']), flt(st['map']['max'])
    out = []
    for k in range(n):
        target = [rng.uniform(0, w), rng.uniform(0, h), rng.uniform(lo, hi)]
        theta = rng.uniform(0, 2 * math.pi)
        slope = rng.choice([0.3, 0.8, 1.5, 4.0])
        d = [math.cos(theta), math.sin(theta), -slope]
        back = (hi - target[2] + rng.uniform(1, 6)) / slope
        eye = [target[i] - back * d[i] for i in range(3)]
        out.append(dict(eye=[f32(x) for x in eye], direction=[f32(x) for x in d], secondary=rng.random() < 0.5, margin=0))
    return out


def run_case(precision, name, s):
    lab = VisLab(precision, s)
    built = lab.build()
    assert not built['asserts']
    row = dict(precision=precision, case=name, spheres=lab.spheres(), frames=[])
    for f in s['frames']:
        cam = camera_words(f['eye'], f['yaw'], f['pitch'], f['hfov'], f['aspect'], f['near'], f['far'])
        lab.set_camera(cam)
        lab.requests.clear()
        lab.view_calls.clear()
        after_pass = lab.frame()
        snap = lab.snapshot()
        snap['after_pass'] = dict(min=after_pass['min'], max=after_pass['max'], recompute=after_pass['recompute'])
        snap['camera'] = {hex(o): v for o, v in sorted(cam.items())}
        snap['requests'] = len(lab.requests)
        snap['view_calls'] = list(lab.view_calls)
        row['frames'].append(snap)
    rng = random.Random(sum(map(ord, name)))
    row['rays'] = []
    for r in rays(rng, row['frames'][-1], 30, s['extents']):
        r['native'] = lab.ray(r['eye'], r['direction'], r['secondary'], r['margin'])
        row['rays'].append(r)
    row['ground'] = [[x, y, fl, v] for (x, y, fl), v in sorted(lab.ground.items())]
    lab.m.verify_v14()
    return row, lab.m.visited, lab.seams.calls


def main():
    all_specs = specs()
    rows, visited, seams = [], {}, {}
    for precision in nm.PRECISIONS:
        for name, s in all_specs.items():
            row, v, calls = run_case(precision, name, s)
            visited.update(v)
            for k, c in calls.items():
                seams[k] = seams.get(k, 0) + c
            rows.append(row)
    from capstone import Cs, CS_ARCH_X86, CS_MODE_32
    md = Cs(CS_ARCH_X86, CS_MODE_32)
    coverage = {}
    for fn, (lo, hi) in RANGES.items():
        listed = [i.address for i in md.disasm(original.range(lo, hi - lo), lo) if i.mnemonic != 'int3']
        missing = [hex(a) for a in listed if a not in visited]
        coverage[fn] = dict(instructions=len(listed), executed=len(listed) - len(missing), missing=missing)
    for a, b in visited.items():
        assert b == original.range(a, len(b)) == modified.range(a, len(b)), hex(a)
    doc = dict(nm.header(__file__, 'unicorn original+v14 images, PC24/PC53/PC64'), scope=__doc__, specs=all_specs, rows=rows,
               coverage=coverage, seam_calls=dict(sorted(seams.items())), executed_pins=len(visited),
               harness_sha256=nm.sha(Path(nm.__file__).read_bytes()), base_producer_sha256=nm.sha(Path(tm.__file__).read_bytes()))
    data = (json.dumps(doc, indent=0, separators=(',', ':')) + '\n').encode()
    if '--dry' in sys.argv:
        for k, v in coverage.items():
            print(k, v['executed'], '/', v['instructions'], ' '.join(v['missing'][:20]))
    elif '--write' in sys.argv:
        with OUT.open('xb') as f:
            f.write(data)
    else:
        assert OUT.read_bytes() == data, 'native-terrain-visibility.json differs from this producer'
    print(json.dumps(dict(rows=len(rows), frames=sum(len(r['frames']) for r in rows), rays=sum(len(r['rays']) for r in rows),
                          executed_pins=len(visited), sha256=hashlib.sha256(data).hexdigest())))


if __name__ == '__main__':
    main()
