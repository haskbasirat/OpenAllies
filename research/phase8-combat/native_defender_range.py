"""Native oracle for the Defender range refresh 0x566F80 and the range getters
it calls: 0x45D6C0 (definition +218) and 0x45D4C0 (definition +2FC, for an
organization: definition +174 bit 7), each the field itself without a
modifier or the field times the modifier's +54 factor through 0x501520.
0x529C30 resolves component +4 -> actor +4 -> definition. Nothing is hooked.

The getters run alone too (their x87 return stored as 80 bits) so the port's
unrounded value is checked as well as the binary32 store at component +2C
(and the +30 byte). Inputs: random binary32 fields and factors over many
magnitudes, zeros of both signs, products that round into the subnormal
range (where PC24 double rounding differs) and both organization bits, at
PC24, PC53 and PC64.

Usage: native_defender_range.py [--exe PATH_TO_AA_EXE]
(default: the AA.exe next to the rust-rebuild folder).
"""
from pathlib import Path
import random
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import harness as h  # noqa: E402


def word(rng):
    kind = rng.random()
    if kind < 0.4:
        return h.f32(rng.uniform(0.0, 600.0))
    if kind < 0.55:
        return h.f32(rng.uniform(0.0, 3.0))
    if kind < 0.7:
        return (rng.getrandbits(1) << 31) | (rng.randint(40, 200) << 23) | rng.getrandbits(23)
    if kind < 0.8:
        return rng.choice((0, 0x80000000, h.f32(1.0)))
    # Small magnitudes: products near and below the binary32 normal range.
    return (rng.getrandbits(1) << 31) | (rng.randint(1, 70) << 23) | rng.getrandbits(23)


def cases(rng):
    out = []
    for _ in range(800):
        out.append({'mask': rng.choice((0, 0x80, 0x7F, 0xFFFFFFFF, 0x100)),
                    'range_218': word(rng), 'range_2fc': word(rng),
                    'modifier': None if rng.random() < 0.25 else word(rng)})
    return out


def run(precision, inputs):
    m = h.Machine(precision)
    component = m.alloc(0x40)
    actor = m.alloc(0x40)
    definition = m.alloc(0x400)
    modifier = m.alloc(0x80)
    m.w(component + 4, actor)
    m.w(actor + 4, definition)
    rows = []
    for case in inputs:
        m.w(definition + 0x174, case['mask'])
        m.w(definition + 0x218, case['range_218'])
        m.w(definition + 0x2FC, case['range_2fc'])
        handle = 0
        if case['modifier'] is not None:
            m.w(modifier + 0x54, case['modifier'])
            handle = modifier
        m.w(component + 0x2C, 0xDEADBEEF)
        m.wb(component + 0x30, 0)
        m.call(0x566F80, ecx=component, args=(handle,), pop=4)
        stored = m.r(component + 0x2C)
        flag = m.rb(component + 0x30)
        m.call(0x45D6C0, ecx=definition, args=(handle,), pop=4)
        defender = m.st0_ext()
        m.call(0x45D4C0, ecx=definition, args=(handle,), pop=4)
        organization = m.st0_ext()
        rows.append({'stored': stored, 'flag': flag, 'getter_45d6c0': defender,
                     'getter_45d4c0': organization})
    return rows, m.visited


def main():
    args = sys.argv[1:]
    if args[:1] == ['--exe']:
        h.ORIGINAL = h.Image(Path(args[1]), h.ORIGINAL_SHA)
    rng = random.Random(0x566F80)
    inputs = cases(rng)
    visited = {}
    results = {}
    for precision in h.PRECISIONS:
        rows, seen = run(precision, inputs)
        results[precision] = rows
        visited.update(seen)
    for index, case in enumerate(inputs):
        case['result'] = {p: results[p][index] for p in h.PRECISIONS}
    h.report('native-defender-range-fixtures.json',
             '0x566F80 Defender range refresh with 0x529C30, 0x45D6C0, 0x45D4C0 and 0x501520; no seams.',
             inputs, visited)


if __name__ == '__main__':
    main()
