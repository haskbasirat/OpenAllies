"""Producer of native-terrain-ray-masked.json: 0x71D840 with masked x87
special values (binary32 overflow stores +-infinity, invalid operations give
the real indefinite NaN, unordered compares).

Runs probe_terrain_ray_v4.run (research/phase8-orders-input/terrain-ray,
unchanged, no seams) in the original and v14 images at PC24/PC53/PC64 (the
outputs must agree) over explicit laboratory TerrainMaps:

* first-frame: the state of the first VWorld update 0x43EC60 after 0x723E00
  (TerrainMap +8C/+90 = +FLT_MAX/-FLT_MAX from 0x724657, every cell +80/+84
  = +FLT_MAX/-FLT_MAX from 0x723960, no +44 mesh, +9C water byte 0),
  loaded byte +BC set (the 0x42EED0 corner casts) and clear, two grids;
* partial: finite map extremes with every third cell still +-FLT_MAX;
* finite: the v4 square-random and float-random terrains under rays whose
  plane parameters overflow binary32 (near-horizontal directions).

Rays: the camera eye and four corner rays (+2EC/+2D4/+2E0/+2F8) of every
frame recorded by research/phase8-orders-input/terrain-visibility (the real
0x42EED0 arguments of that oracle), plus generated camera-like and
near-horizontal rays. A code hook records, per FPU compare site, how often the
compare was unordered (C3 = C2 = C0 = 1). Laboratory words are explicit
oracle inputs.

  python native_terrain_ray_masked.py --write   # create (refuses to overwrite)
  python native_terrain_ray_masked.py           # replay: byte-identical check
  python native_terrain_ray_masked.py --dry     # summary only
"""
from pathlib import Path
import collections, hashlib, itertools, json, math, random, struct, sys
HERE = Path(__file__).resolve().parent
RAY = HERE.parent / 'terrain-ray'
sys.path.insert(0, str(RAY))
import probe_terrain_ray_v4 as probe  # noqa: E402
import native_terrain_ray_v4 as v4  # noqa: E402
h = probe.h
P, V = h.P, h.V
from unicorn import UC_HOOK_CODE  # noqa: E402
from capstone import Cs, CS_ARCH_X86, CS_MODE_32  # noqa: E402
OUT = HERE / 'native-terrain-ray-masked.json'
VISIBILITY = HERE.parent / 'terrain-visibility' / 'native-terrain-visibility.json'
FLT_MAX, NEG_FLT_MAX = 0x7f7fffff, 0xff7fffff

# Instruction after every FPU compare in the reported functions -> compare.
COMPARE_AFTER = {}
_md = Cs(CS_ARCH_X86, CS_MODE_32)
for _lo, _hi in v4.RANGES.values():
    _ins = list(_md.disasm(P.range(_lo, _hi - _lo), _lo))
    for _a, _b in zip(_ins, _ins[1:]):
        if _a.mnemonic.startswith(('fcom', 'fucom')):
            COMPARE_AFTER[_b.address] = _a.address
UNORDERED = collections.Counter()
ORDERED = collections.Counter()
_init = h.Machine.__init__


def _machine(self, pc):
    _init(self, pc)

    def hook(u, address, size, data):
        site = COMPARE_AFTER.get(address)
        if site is not None:
            c = (u.reg_read(h.UC_X86_REG_FPSW) >> 8) & 0x45
            (UNORDERED if c == 0x45 else ORDERED)[site] += 1
    self.u.hook_add(UC_HOOK_CODE, hook)


h.Machine.__init__ = _machine


def first_frame(spec, loaded=1):
    t = probe.terrain_words(spec)
    t['min'], t['max'], t['loaded'] = FLT_MAX, NEG_FLT_MAX, loaded
    for c in t['cells']:
        c.update(min=FLT_MAX, max=NEG_FLT_MAX, mesh=None, water=0, water_mesh=None)
    return t


def partial(spec):
    t = probe.terrain_words(spec)
    for i, c in enumerate(t['cells']):
        if i % 3 == 1:
            c.update(min=FLT_MAX, max=NEG_FLT_MAX, mesh=None, water=0, water_mesh=None)
    return t


def terrains():
    s = v4.specs()
    return {'first-frame': first_frame(s['square-random']),
            'first-frame-unloaded': first_frame(s['square-random'], loaded=0),
            'first-frame-wide': first_frame(s['wide-ridge']),
            'partial': partial(s['square-water']),
            'finite': probe.terrain_words(s['square-random']),
            'finite-float': probe.terrain_words(s['float-random'])}


def camera_rays():
    doc = json.loads(VISIBILITY.read_text())
    out, seen = [], set()
    for row in doc['rows']:
        for frame in row['frames']:
            c = frame['camera']
            eye = [c['0x90'], c['0x94'], c['0x98']]
            for o in (0x2ec, 0x2d4, 0x2e0, 0x2f8):
                d = [c[hex(o)], c[hex(o + 4)], c[hex(o + 8)]]
                key = tuple(eye + d)
                if key not in seen:
                    seen.add(key)
                    out.append((eye, d))
    return out


def generated(rng, n, size):
    w, hgt = size
    out = []
    for k in range(n):
        eye = [rng.uniform(-8, w + 8), rng.uniform(-8, hgt + 8), rng.uniform(1, 60)]
        theta = rng.uniform(0, 2 * math.pi)
        if rng.random() < 0.15:
            theta = rng.choice([0, math.pi / 2, math.pi, 3 * math.pi / 2])
        kind = rng.random()
        if kind < 0.5:
            pitch = math.radians(rng.choice([0.5, 2, 10, 30, 45, 60, 80, 89, -0.5, -5]))
            d = [math.cos(theta) * math.cos(pitch), math.sin(theta) * math.cos(pitch), -math.sin(pitch)]
        else:
            # Near-horizontal: the plane parameter overflows binary32.
            dz = rng.choice([1e-7, 1e-12, 1e-20, 1e-30, 1e-38, -1e-7, -1e-20, -1e-38])
            d = [math.cos(theta), math.sin(theta), dz]
        d = [0.0 if abs(x) < 1e-12 and abs(x) > 0 and i < 2 else x for i, x in enumerate(d)]
        out.append(([probe.f32(x) for x in eye], [probe.f32(x) for x in d]))
    return out


def cases():
    rng = random.Random(0x71d840 + 0x7f)
    ts = terrains()
    cam = camera_rays()
    out = []
    for name, t in ts.items():
        rays = (cam if name.startswith('first-frame') or name == 'partial' else []) + generated(rng, 60, t['size'])
        for i, (eye, d) in enumerate(rays):
            out.append(dict(name=f'{name}-{i}', terrain=name, eye=eye, direction=d,
                            secondary=rng.random() < 0.7, border=rng.choice([0, 0, 0, 1, 2])))
    return ts, out


def special(word):
    return (word >> 23) & 0xff == 0xff


def main():
    h.VISITED.clear()
    probe.READS.clear()
    UNORDERED.clear()
    ORDERED.clear()
    ts, case_list = cases()
    rows = []
    for pc, c in itertools.product(h.PRECISION, case_list):
        outputs = []
        for image in [P, V]:
            h.P = image
            outputs.append(probe.run(ts[c['terrain']], c['eye'], c['direction'], pc=pc, secondary=c['secondary'], border=c['border']))
        h.P = P
        assert outputs[0] == outputs[1], (pc, c, outputs)
        rows.append(dict(precision=pc, input=c, native=outputs[0]))
    pins = {hex(a): v for a, v in sorted(h.VISITED.items())}
    for a, v in pins.items():
        assert P.range(int(a, 16), len(bytes.fromhex(v))) == V.range(int(a, 16), len(bytes.fromhex(v)))
    coverage = {}
    for fn, (lo, hi) in v4.RANGES.items():
        listed = [i.address for i in _md.disasm(P.range(lo, hi - lo), lo) if i.mnemonic != 'int3']
        missing = [hex(a) for a in listed if a not in h.VISITED]
        coverage[fn] = dict(instructions=len(listed), executed=len(listed) - len(missing), missing=missing)
    # Both images run every case, so the counters are halved per image.
    compares = {hex(s): dict(unordered=UNORDERED[s] // 2, ordered=ORDERED[s] // 2) for s in sorted(set(UNORDERED) | set(ORDERED))}
    tally = collections.Counter()
    for r in rows:
        n = r['native']
        words = [w for _, _, w in n.get('writes', [])]
        kind = 'fault' if 'fault' in n else 'nan' if any(special(w) and w & 0x7fffff for w in words) else \
            'infinity' if any(special(w) for w in words) else 'finite'
        tally[f"{r['input']['terrain']}:{kind}"] += 1
    doc = dict(schema=1, scope=__doc__, terrains=ts, rows=rows, executed_pins=pins, coverage=coverage,
               compares=compares, tally=dict(sorted(tally.items())),
               producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
               probe_sha256=hashlib.sha256(Path(probe.__file__).read_bytes()).hexdigest(),
               v4_producer_sha256=hashlib.sha256(Path(v4.__file__).read_bytes()).hexdigest(),
               harness_sha256=hashlib.sha256(Path(h.__file__).read_bytes()).hexdigest(),
               visibility_sha256=hashlib.sha256(VISIBILITY.read_bytes()).hexdigest())
    data = (json.dumps(doc, indent=1) + '\n').encode()
    summary = dict(rows=len(rows), tally=doc['tally'], unordered={k: v['unordered'] for k, v in compares.items() if v['unordered']},
                   sha256=hashlib.sha256(data).hexdigest())
    if '--dry' in sys.argv:
        print(json.dumps(summary, indent=1))
        return
    if '--write' in sys.argv:
        with OUT.open('xb') as f:
            f.write(data)
    else:
        assert OUT.read_bytes() == data
    print(json.dumps(summary))


if __name__ == '__main__':
    main()
