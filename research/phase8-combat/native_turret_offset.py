"""Native oracle for 0x4C76A0, the turret definition's mount offset (+24, +28)
turned by a unit facing vector (cos, sin), at PC24, PC53 and PC64.

The original body runs unchanged; nothing is hooked. Inputs are random
binary32 values over several magnitudes plus edge patterns: signed zeros,
subnormals, exact and near cancellation, and mixed large/small products. All
inputs stay finite and every result fits binary32 (the port's finite domain).

Usage: native_turret_offset.py [--exe PATH_TO_AA_EXE]
(default: the AA.exe next to the rust-rebuild folder).
"""
from pathlib import Path
import random
import struct
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import harness as h  # noqa: E402

ENTRY = 0x4C76A0


def bits(value):
    return h.f32(value)


def random_word(rng):
    kind = rng.random()
    if kind < 0.35:
        return bits(rng.uniform(-2.0, 2.0))
    if kind < 0.55:
        return bits(rng.uniform(-200.0, 200.0))
    if kind < 0.70:
        # Any finite binary32 with a moderate exponent.
        exponent = rng.randint(90, 160)
        return (rng.getrandbits(1) << 31) | (exponent << 23) | rng.getrandbits(23)
    if kind < 0.80:
        # Subnormal binary32.
        return (rng.getrandbits(1) << 31) | rng.randint(1, 0x7FFFFF)
    if kind < 0.88:
        return rng.choice((0x00000000, 0x80000000))
    # Small exponents near the normal edge.
    exponent = rng.randint(1, 8)
    return (rng.getrandbits(1) << 31) | (exponent << 23) | rng.getrandbits(23)


def cases(rng):
    out = []
    for _ in range(600):
        out.append([random_word(rng) for _ in range(4)])
    # Exact and near cancellation of x*A - y*B and y*A + x*B.
    for _ in range(200):
        a = bits(rng.uniform(-50.0, 50.0))
        b = bits(rng.uniform(-50.0, 50.0))
        x = bits(rng.uniform(-1.0, 1.0))
        flip = rng.choice((0, 1, -1, 2, -2))
        # y chosen so that y*B is close to x*A.
        fa = h.from_f32(a)
        fb = h.from_f32(b)
        fx = h.from_f32(x)
        if fb != 0.0:
            y = (bits(fx * fa / fb) + flip) & 0xFFFFFFFF
        else:
            y = bits(rng.uniform(-1.0, 1.0))
        if rng.random() < 0.5:
            out.append([a, b, x, y])
        else:
            # Swap roles so the second word cancels instead.
            out.append([a, b, y ^ 0x80000000, x])
    # Signed zero combinations.
    zeros = (0x00000000, 0x80000000)
    for a in zeros + (bits(1.0), bits(-1.0)):
        for b in zeros + (bits(1.0), bits(-1.0)):
            for x in zeros + (bits(1.0),):
                for y in zeros + (bits(-1.0),):
                    out.append([a, b, x, y])
    # Real unit facings and realistic mount offsets.
    import math
    for _ in range(200):
        angle = rng.uniform(-math.pi, math.pi)
        out.append([bits(rng.uniform(-8.0, 8.0)), bits(rng.uniform(-8.0, 8.0)),
                    bits(math.cos(angle)), bits(math.sin(angle))])
    return out


def run(precision, inputs):
    m = h.Machine(precision)
    definition = m.alloc(0x40)
    facing = m.alloc(8)
    result = m.alloc(8)
    rows = []
    for a, b, x, y in inputs:
        m.w(definition + 0x24, a)
        m.w(definition + 0x28, b)
        m.w(facing, x)
        m.w(facing + 4, y)
        m.w(result, 0xDEADBEEF)
        m.w(result + 4, 0xDEADBEEF)
        m.call(ENTRY, ecx=definition, args=(result, facing), pop=8)
        rows.append([m.r(result), m.r(result + 4)])
    return rows, m.visited


def main():
    args = sys.argv[1:]
    if args[:1] == ['--exe']:
        h.ORIGINAL = h.Image(Path(args[1]), h.ORIGINAL_SHA)
    rng = random.Random(0x4C76A0)
    inputs = cases(rng)
    out = []
    visited = {}
    results = {}
    for precision in h.PRECISIONS:
        rows, seen = run(precision, inputs)
        results[precision] = rows
        visited.update(seen)
    for index, words in enumerate(inputs):
        out.append({'offset': words[:2], 'facing': words[2:],
                    'result': {p: results[p][index] for p in h.PRECISIONS}})
    h.report('native-turret-offset-fixtures.json',
             '0x4C76A0 turret mount offset turned by the facing vector; no seams.',
             out, visited)


if __name__ == '__main__':
    main()
