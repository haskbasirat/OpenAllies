"""Native oracle for 0x4CFB70, the actor shape point (the projectile launch
origin 0x4BE780 takes for a non-Kingdom source whose ability definition has
no projectile entry at +3C and whose virtual +B4 holds).

Nothing is hooked. The actor carries a real AA.exe vtable: 0xA273F0 (the
base actor class) or 0xA29490 (the other non-Kingdom class whose +B4 is
true); both resolve virtual +0 to 0x4D1E30 (-> +10 0x4EA020, the f32 at
definition +54) and +14 to 0x4EA030 (definition +68 when the byte at
definition +80 is set, else definition +50). The shape operation 0x46E150
and the normalize 0x409230 run as original code. Inputs: every shape type
(0..4), both shape selections, boxes that clamp on each side or not, short
scaled facings (the 1e-4 length gate), random finite positions, facings,
scales and definition +1DC heights, at PC24, PC53 and PC64.

Usage: native_launch_point.py [--exe PATH_TO_AA_EXE]
(default: the AA.exe next to the rust-rebuild folder).
"""
from pathlib import Path
import math
import random
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import harness as h  # noqa: E402

ENTRY = 0x4CFB70
VTABLES = (0xA273F0, 0xA29490)


def f(value):
    return h.f32(value)


def shape(rng, kind):
    """Six words: type, radius, min_u, min_v, max_u, max_v."""
    lo_u = rng.uniform(-6.0, 1.0)
    lo_v = rng.uniform(-6.0, 1.0)
    hi_u = lo_u + rng.choice((0.0, rng.uniform(0.0, 8.0)))
    hi_v = lo_v + rng.choice((0.0, rng.uniform(0.0, 8.0)))
    return [kind, f(rng.choice((0.0, rng.uniform(0.0, 12.0), rng.uniform(-3.0, 0.0)))),
            f(lo_u), f(lo_v), f(hi_u), f(hi_v)]


def build_cases(rng):
    cases = []
    for index in range(900):
        base = shape(rng, rng.choice((0, 1, 1, 2, 2, 3, 4, 7)))
        alternate = shape(rng, rng.choice((0, 1, 2, 2, 3, 4)))
        use_alternate = rng.choice((0, 0, 1, 0x80))
        scale_kind = rng.random()
        if scale_kind < 0.1:
            base[1] = 0
        elif scale_kind < 0.2:
            base[1] = f(rng.uniform(0.0, 2e-4))
        angle = rng.uniform(-math.pi, math.pi)
        length = rng.choice((1.0, 1.0, 1.0, rng.uniform(0.0, 3.0), rng.uniform(0.0, 1e-3), 0.0))
        facing = [f(math.cos(angle) * length), f(math.sin(angle) * length)]
        spread = rng.choice((3000.0, 1.0, 1e-3, 0.0))
        position = [f(rng.uniform(-spread, spread)) for _ in range(3)]
        height = f(rng.choice((0.0, rng.uniform(-5.0, 40.0))))
        cases.append({'vtable': VTABLES[index % 2], 'base': base, 'alternate': alternate,
                      'use_alternate': use_alternate, 'facing': facing,
                      'position': position, 'height': height})
    return cases


def run(precision, cases):
    m = h.Machine(precision)
    actor = m.alloc(0x40)
    definition = m.alloc(0x200)
    out = m.alloc(16)
    rows = []
    for case in cases:
        m.w(actor, case['vtable'])
        m.w(actor + 4, definition)
        for k in range(3):
            m.w(actor + 0x20 + 4 * k, case['position'][k])
        m.w(actor + 0x2C, case['facing'][0])
        m.w(actor + 0x30, case['facing'][1])
        for k in range(6):
            m.w(definition + 0x50 + 4 * k, case['base'][k])
            m.w(definition + 0x68 + 4 * k, case['alternate'][k])
        m.wb(definition + 0x80, case['use_alternate'])
        m.w(definition + 0x1DC, case['height'])
        for k in range(3):
            m.w(out + 4 * k, 0xDEADBEEF)
        returned = m.call(ENTRY, ecx=actor, args=(out,), pop=4)
        assert returned == out
        rows.append([m.r(out + 4 * k) for k in range(3)])
    return rows, m.visited


def main():
    args = sys.argv[1:]
    if args[:1] == ['--exe']:
        h.ORIGINAL = h.Image(Path(args[1]), h.ORIGINAL_SHA)
    rng = random.Random(0x4CFB70)
    cases = build_cases(rng)
    visited = {}
    results = {}
    for precision in h.PRECISIONS:
        rows, seen = run(precision, cases)
        results[precision] = rows
        visited.update(seen)
    for index, case in enumerate(cases):
        case['result'] = {p: results[p][index] for p in h.PRECISIONS}
    default = [h.ORIGINAL.at(0xB279C0, 4), h.ORIGINAL.at(0xB279C4, 4)]
    h.report('native-launch-point-fixtures.json',
             '0x4CFB70 actor shape point with 0x4D1E30/0x4EA020/0x4EA030 (real vtables '
             '0xA273F0/0xA29490), 0x46E150 and 0x409230; no seams.',
             cases, visited,
             {'default_vector_words': [int.from_bytes(w, 'little') for w in default]})


if __name__ == '__main__':
    main()
