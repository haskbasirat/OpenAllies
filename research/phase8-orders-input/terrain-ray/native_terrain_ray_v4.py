"""Producer of native-terrain-ray-v4.json: whole 0x71D840 walker coverage.

Runs probe_terrain_ray_v4.run in the original and v14 images at PC24/PC53/
PC64 (outputs must agree) over generated explicit laboratory TerrainMaps:
square/non-square block grids, full-mantissa heights, water ties, 2/4/6 quads per block, block sizes 8 and 12,
mixed mesh/absent-mesh cells, water children (above/below land, tilted,
single triangle, count 7, byte off), flat plateau, unloaded map, margins
0/1/2/5 (and -1/-3/-9 for the inline clamps and faults), secondary null/non-null, shallow/steep/up/axis-aligned rays from
inside and outside the map. Rows whose native run faults (a negative cell
index reads below the isolated cell array) are kept with `fault` and must
be errors in Rust. Laboratory words are explicit oracle inputs, not native
constructor receipts or current mission values.

  python native_terrain_ray_v4.py --write   # create (refuses to overwrite)
  python native_terrain_ray_v4.py           # replay: byte-identical check
"""
from pathlib import Path
import hashlib, itertools, json, math, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import probe_terrain_ray_v4 as probe  # noqa: E402
h = probe.h
P, V = h.P, h.V
OUT = HERE / 'native-terrain-ray-v4.json'
# Function ranges (entry, end) whose executed-instruction coverage is reported.
RANGES = {'71D840': (0x71d840, 0x71e012), '71C930': (0x71c930, 0x71cbf3), '71CD30': (0x71cd30, 0x71ce65),
          '71B9B0': (0x71b9b0, 0x71bc0d), '71A320': (0x71a320, 0x71a475), '71D570': (0x71d570, 0x71d5e8),
          '5E8910': (0x5e8910, 0x5e89c0), '53A3A0': (0x53a3a0, 0x53a48d), '53A7D0': (0x53a7d0, 0x53a81b),
          '42ED60': (0x42ed60, 0x42ee13), '42EE20': (0x42ee20, 0x42ee6f), '42EE70': (0x42ee70, 0x42eebf),
          '71AB10': (0x71ab10, 0x71ab60), '71ADC0': (0x71adc0, 0x71adf8), '7151C0': (0x7151c0, 0x71524f),
          '436310': (0x436310, 0x4363a2), '875610': (0x875610, 0x8756f8), '736C60': (0x736c60, 0x736d4f),
          '736E70': (0x736e70, 0x736e7a), '42D750': (0x42d750, 0x42d7ad), '860680': (0x860680, 0x8606b6)}


def eighths(rng, lo, hi, n):
    return [rng.randrange(lo * 8, hi * 8) / 8 for _ in range(n)]


def specs():
    rng = random.Random(0x71d840 + 4)
    out = {}

    def grid(bx, by, q):
        return (bx * q + 1) * (by * q + 1)
    square = eighths(rng, -2, 5, grid(4, 4, 4))
    out['square-random'] = dict(size=[32, 32], blocks=[4, 4], quads=4, quad_size=2, heights=square, mesh=[True] * 16, loaded=True)
    out['square-mixed'] = dict(out['square-random'], mesh=[(i + i // 4) % 2 == 0 for i in range(16)])
    gx = 6 * 4 + 1
    ridge = [round(8 * (2.5 * math.sin(i % gx / 3.0) + 1.5 * math.cos(i // gx / 2.0))) / 8 for i in range(grid(6, 3, 4))]
    out['wide-ridge'] = dict(size=[48, 24], blocks=[6, 3], quads=4, quad_size=2, heights=ridge, mesh=[True] * 18, loaded=True)
    out['tall-mixed'] = dict(size=[16, 40], blocks=[2, 5], quads=4, quad_size=2, heights=eighths(rng, -1, 4, grid(2, 5, 4)),
                             mesh=[i % 3 != 2 for i in range(10)], loaded=True)
    water = {}
    for i in range(16):
        if i % 3 == 0:
            continue
        water[str(i)] = dict(levels=[0.5] * 4)
    water['1'] = dict(levels=[0.25, 0.5, 0.75, 1.0])
    water['2'] = dict(levels=[0.5] * 4, count=7)
    water['4'] = dict(levels=[0.5] * 4, count=3)
    water['5'] = dict(levels=[0.5] * 4, enabled=False)
    water['10'] = dict(levels=[6.0] * 4)
    water['13'] = dict(levels=[-0.75, -0.25, 0.0, 0.5])
    water['14'] = dict(levels=[0.5] * 4, count=9, triangles=[[0, 0, 1], [0, 1, 2], [1, 3, 2]])
    out['square-water'] = dict(size=[32, 32], blocks=[4, 4], quads=4, quad_size=2, heights=eighths(rng, -2, 2, grid(4, 4, 4)),
                               mesh=[i != 8 for i in range(16)], loaded=True, water=water)
    out['coarse-quads'] = dict(size=[24, 24], blocks=[3, 3], quads=2, quad_size=4, heights=eighths(rng, -2, 4, grid(3, 3, 2)),
                               mesh=[True] * 9, loaded=True)
    out['fine-quads-12'] = dict(size=[24, 24], blocks=[2, 2], quads=6, quad_size=2, heights=eighths(rng, -2, 4, grid(2, 2, 6)),
                                mesh=[True, False, True, True], loaded=True)
    out['flat-plateau'] = dict(size=[32, 32], blocks=[4, 4], quads=4, quad_size=2, heights=[1.5] * grid(4, 4, 4), mesh=[True] * 16, loaded=True)
    out['square-unloaded'] = dict(out['square-random'], loaded=False)
    # Full-mantissa binary32 heights: cross products and plane constants round.
    floats = [struct.unpack('<f', struct.pack('<f', rng.uniform(-2, 4)))[0] for _ in range(grid(4, 4, 4))]
    out['float-random'] = dict(size=[32, 32], blocks=[4, 4], quads=4, quad_size=2, heights=floats, mesh=[True] * 16, loaded=True)
    # Flat land exactly at the water level in water cells: 0x71BB7F ties.
    tie = eighths(rng, -1, 3, grid(4, 4, 4))
    gx = 4 * 4 + 1
    tie_water = {}
    for i in (0, 5, 6, 9, 10, 15):
        bx, by = i % 4, i // 4
        for yy in range(5):
            for xx in range(5):
                tie[(by * 4 + yy) * gx + bx * 4 + xx] = 0.5
        tie_water[str(i)] = dict(levels=[0.5] * 4, triangles=[[0, 1, 2], [1, 3, 2]] if i % 2 else [[0, 2, 1], [1, 2, 3]])
    out['water-tie'] = dict(size=[32, 32], blocks=[4, 4], quads=4, quad_size=2, heights=tie, mesh=[True] * 16, loaded=True, water=tie_water)
    # Block 0 rises along x through the 0.5 water plane at x = 4: rays aimed
    # at that contour give land/water hits with rounded-equal z (0x71BB7F).
    contour = [0.25 + 0.125 * (i % 9) if i % 9 <= 4 and i // 9 <= 4 else -2.0 for i in range(grid(2, 2, 4))]
    out['water-contour'] = dict(size=[16, 16], blocks=[2, 2], quads=4, quad_size=2, heights=contour, mesh=[True] * 4, loaded=True,
                                water={'0': dict(levels=[0.5] * 4)})
    return out


def rays(rng, spec, n):
    w, hgt = spec['size']
    top, bottom = max(spec['heights']), min(spec['heights'])
    out = []

    def outside(lo, hi, pad):
        return rng.choice([rng.uniform(lo - pad, lo), rng.uniform(hi, hi + pad)])
    for k in range(n):
        kind = rng.random()
        grazing = False
        if kind < 0.5:
            eye = [rng.uniform(0, w), rng.uniform(0, hgt), top + rng.uniform(0.05, 4)]
        elif kind < 0.62:
            eye = [outside(0, w, 8), rng.uniform(-4, hgt + 4), top + rng.uniform(0.5, 6)]
        elif kind < 0.74:
            eye = [rng.uniform(-4, w + 4), outside(0, hgt, 8), top + rng.uniform(0.5, 6)]
        elif kind < 0.9:
            grazing = True
            eye = [outside(0, w, 3), rng.uniform(0, hgt), rng.uniform(bottom, top + 0.3)]
            if rng.random() < 0.5:
                eye = [eye[1] * w / hgt, outside(0, hgt, 3), eye[2]]
        else:
            eye = [rng.uniform(0, w), rng.uniform(0, hgt), top + rng.uniform(8, 30)]
        a = rng.random()
        theta = rng.uniform(0, 2 * math.pi)
        if a < 0.1:
            theta = rng.choice([0, math.pi / 2, math.pi, 3 * math.pi / 2])
        elif a < 0.15:
            theta = rng.choice([math.pi / 4, 3 * math.pi / 4, 5 * math.pi / 4, 7 * math.pi / 4])
        if grazing:
            # Aim across the map from outside: towards the far side.
            cx, cy = rng.uniform(0, w), rng.uniform(0, hgt)
            theta = math.atan2(cy - eye[1], cx - eye[0])
            slope = rng.choice([0.005, 0.015, 0.03, 0.06, -0.01, -0.03])
        else:
            slope = rng.choice([0.03, 0.08, 0.15, 0.3, 0.7, 2.0, 10.0])
            if rng.random() < 0.08:
                slope = -rng.choice([0.05, 0.2, 0.5])
            if eye[2] > top + 7:
                slope = max(slope, 0.7)
        d = [math.cos(theta), math.sin(theta), -slope]
        d = [0.0 if abs(x) < 1e-12 else x for x in d]
        out.append(([probe.f32(x) for x in eye], [probe.f32(x) for x in d]))
    return out


def contour_rays(rng, n):
    out = []
    for k in range(n):
        target = [4 + rng.choice([0, 0, 1e-3, -1e-3, 0.3, -0.3]), rng.uniform(1, 7), 0.5]
        theta = rng.choice([math.pi / 2, 3 * math.pi / 2, rng.uniform(0, 2 * math.pi)])
        slope = rng.choice([0.01, 0.02, 0.05, 0.1, 0.3, 1.0])
        d = [math.cos(theta), math.sin(theta), -slope]
        k_back = rng.uniform(0.2, 1.5) / slope
        eye = [target[i] - k_back * d[i] for i in range(3)]
        d = [0.0 if abs(x) < 1e-12 else x for x in d]
        out.append(([probe.f32(x) for x in eye], [probe.f32(x) for x in d]))
    return out


def cases():
    rng = random.Random(0x5e8910)
    terrains = {name: probe.terrain_words(s) for name, s in specs().items()}
    counts = {'square-unloaded': 12, 'flat-plateau': 30}
    out = []
    for name, spec in specs().items():
        generated = contour_rays(rng, 60) if name == 'water-contour' else rays(rng, spec, counts.get(name, 110))
        for i, (eye, direction) in enumerate(generated):
            out.append(dict(name=f'{name}-{i}', terrain=name, eye=eye, direction=direction,
                            secondary=rng.random() < 0.8, border=rng.choice([0, 0, 1, 2, 5]) if rng.random() < 0.9 else rng.choice([-1, -3, -9])))
    return terrains, out


def main():
    h.VISITED.clear()
    probe.READS.clear()
    terrains, case_list = cases()
    rows = []
    for pc, c in itertools.product(h.PRECISION, case_list):
        outputs = []
        for image in [P, V]:
            h.P = image
            outputs.append(probe.run(terrains[c['terrain']], c['eye'], c['direction'], pc=pc, secondary=c['secondary'], border=c['border']))
        h.P = P
        assert outputs[0] == outputs[1], (pc, c, outputs)
        rows.append(dict(precision=pc, input=c, native=outputs[0]))
    pins = {hex(a): v for a, v in sorted(h.VISITED.items())}
    for a, v in pins.items():
        assert P.range(int(a, 16), len(bytes.fromhex(v))) == V.range(int(a, 16), len(bytes.fromhex(v)))
    coverage = {}
    from capstone import Cs, CS_ARCH_X86, CS_MODE_32
    md = Cs(CS_ARCH_X86, CS_MODE_32)
    for fn, (lo, hi) in RANGES.items():
        listed = [i.address for i in md.disasm(P.range(lo, hi - lo), lo) if i.mnemonic != 'int3']
        missing = [hex(a) for a in listed if a not in h.VISITED]
        coverage[fn] = dict(instructions=len(listed), executed=len(listed) - len(missing), missing=missing)
    reads = {k: sorted(hex(x) for x in v) for k, v in sorted(probe.READS.items())}
    doc = dict(schema=1, scope=__doc__, terrains=terrains, rows=rows, executed_pins=pins, coverage=coverage, observed_reads=reads,
               producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
               probe_sha256=hashlib.sha256(Path(probe.__file__).read_bytes()).hexdigest(),
               harness_sha256=hashlib.sha256(Path(h.__file__).read_bytes()).hexdigest())
    data = (json.dumps(doc, indent=1) + '\n').encode()
    if '--dry' in sys.argv:
        print(json.dumps({k: (v['instructions'], v['executed'], v['missing'][:40]) for k, v in coverage.items()}, indent=0))
    elif '--write' in sys.argv:
        with OUT.open('xb') as f:
            f.write(data)
    else:
        assert OUT.read_bytes() == data
    faults = sum(1 for r in rows if 'fault' in r['native'])
    print(json.dumps(dict(rows=len(rows), executions=2 * len(rows), faults=faults, executed_pins=len(pins),
                          sha256=hashlib.sha256(data).hexdigest())))


if __name__ == '__main__':
    main()
