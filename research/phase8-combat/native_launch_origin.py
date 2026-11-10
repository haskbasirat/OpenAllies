"""Native oracle for the launch origin 0x4BE780 over all of its branches, with
the actor shape point 0x4CFB70 it reaches (see native_launch_point.py) and
the copy 0x4BE740.

Nothing is hooked. The source is null or an object with a real AA.exe
vtable: 0xA299D8 (virtual +A8 true: a Kingdom), 0xA273F0 and 0xA29490 (+A8
false, +B4 true) or 0xA28A00 (+A8 and +B4 false). The local definition
object (this) holds the spawn offset at +8/+C/+10 and its first word points
at the ability definition, whose +3C is zero or not. Inputs are random
binary32 positions, facings, offsets and shape fields at PC24, PC53, PC64.

Usage: native_launch_origin.py [--exe PATH_TO_AA_EXE]
(default: the AA.exe next to the rust-rebuild folder).
"""
from pathlib import Path
import math
import random
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import harness as h  # noqa: E402
import native_launch_point as lp  # noqa: E402

SOURCES = (None, 0xA299D8, 0xA273F0, 0xA29490, 0xA28A00)


def f(value):
    return h.f32(value)


def build_cases(rng):
    cases = []
    for index in range(600):
        angle = rng.uniform(-math.pi, math.pi)
        spread = rng.choice((3000.0, 1.0, 0.0))
        base = lp.shape(rng, rng.choice((0, 1, 2, 3)))
        alternate = lp.shape(rng, rng.choice((1, 2)))
        if rng.random() < 0.2:
            base[1] = 0
        cases.append({
            'source': SOURCES[index % len(SOURCES)],
            'projectile': rng.choice((0, 0, 0x1234)),
            'offset': [f(rng.uniform(-4.0, 4.0)) for _ in range(3)],
            'fallback': [f(rng.uniform(-spread, spread)) for _ in range(3)],
            'position': [f(rng.uniform(-spread, spread)) for _ in range(3)],
            'facing': [f(math.cos(angle)), f(math.sin(angle))],
            'base': base, 'alternate': alternate,
            'use_alternate': rng.choice((0, 1)),
            'height': f(rng.choice((0.0, rng.uniform(-5.0, 40.0)))),
        })
    return cases


def run(precision, cases):
    m = h.Machine(precision)
    local = m.alloc(0x40)
    ability = m.alloc(0x80)
    actor = m.alloc(0x40)
    definition = m.alloc(0x200)
    out = m.alloc(16)
    m.w(local, ability)
    m.w(actor + 4, definition)
    rows = []
    for case in cases:
        m.w(ability + 0x3C, case['projectile'])
        for k in range(3):
            m.w(local + 8 + 4 * k, case['offset'][k])
            m.w(actor + 0x20 + 4 * k, case['position'][k])
            m.w(out + 4 * k, 0xDEADBEEF)
        m.w(actor + 0x2C, case['facing'][0])
        m.w(actor + 0x30, case['facing'][1])
        for k in range(6):
            m.w(definition + 0x50 + 4 * k, case['base'][k])
            m.w(definition + 0x68 + 4 * k, case['alternate'][k])
        m.wb(definition + 0x80, case['use_alternate'])
        m.w(definition + 0x1DC, case['height'])
        source = 0
        if case['source'] is not None:
            m.w(actor, case['source'])
            source = actor
        fb = case['fallback']
        returned = m.call(0x4BE780, ecx=local, args=(out, source, fb[0], fb[1], fb[2]), pop=0x14)
        assert returned == out
        rows.append([m.r(out + 4 * k) for k in range(3)])
    return rows, m.visited


def main():
    args = sys.argv[1:]
    if args[:1] == ['--exe']:
        h.ORIGINAL = h.Image(Path(args[1]), h.ORIGINAL_SHA)
    rng = random.Random(0x4BE780)
    cases = build_cases(rng)
    visited = {}
    results = {}
    for precision in h.PRECISIONS:
        rows, seen = run(precision, cases)
        results[precision] = rows
        visited.update(seen)
    for index, case in enumerate(cases):
        case['result'] = {p: results[p][index] for p in h.PRECISIONS}
    h.report('native-launch-origin-fixtures.json',
             '0x4BE780 launch origin (every branch) with 0x4BE740, 0x4CFB70 and the real '
             'vtables 0xA299D8/0xA273F0/0xA29490/0xA28A00; no seams.',
             cases, visited)


if __name__ == '__main__':
    main()
