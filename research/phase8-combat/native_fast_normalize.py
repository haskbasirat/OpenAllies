"""Native oracle for the table-based fast vector normalize 0x43A970 and the
startup builder of its square-root mantissa table, 0x8693B0 (stored at
[B43DC4] by 0x869DC0).

0x8693B0 runs at PC24, PC53 and PC64 with its allocation 0x6F7810 answered
from the laboratory heap (the only seam); the 256 table words are recorded.
0x43A970 then runs unchanged with [B43DC4] pointing at that table, over
random binary32 vectors of many magnitudes, zero and signed-zero vectors,
vectors whose squared length is subnormal, and odd and even exponents, at
all three precisions. Every input keeps the squared length finite.

Usage: native_fast_normalize.py [--exe PATH_TO_AA_EXE]
(default: the AA.exe next to the rust-rebuild folder).
"""
from pathlib import Path
import random
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import harness as h  # noqa: E402

TABLE_ROOT = 0xB43DC4


def component(rng):
    kind = rng.random()
    if kind < 0.4:
        return h.f32(rng.uniform(-10.0, 10.0))
    if kind < 0.6:
        return h.f32(rng.uniform(-5000.0, 5000.0))
    if kind < 0.75:
        return (rng.getrandbits(1) << 31) | (rng.randint(70, 180) << 23) | rng.getrandbits(23)
    if kind < 0.85:
        return rng.choice((0, 0x80000000))
    # Tiny: squared lengths near and below the binary32 normal range.
    return (rng.getrandbits(1) << 31) | (rng.randint(1, 66) << 23) | rng.getrandbits(23)


def vectors(rng):
    out = [[component(rng) for _ in range(3)] for _ in range(1500)]
    out += [[0, 0, 0], [0x80000000, 0x80000000, 0x80000000], [h.f32(1.0), 0, 0],
            [0, h.f32(-2.0), 0], [0, 0, h.f32(3.0)], [1, 0, 0], [0x80000001, 1, 1]]
    return out


def build_table(m):
    def new(mm):
        size = mm.arg(0)
        block = mm.alloc(size)
        mm.u.mem_write(block, b'\xA5' * size)
        mm.ret(0, block)
    m.hooks[0x6F7810] = new
    m.call(0x869DC0, pop=0)
    table = m.r(TABLE_ROOT)
    return [m.r(table + 4 * i) for i in range(256)]


def run(precision, inputs):
    m = h.Machine(precision)
    table = build_table(m)
    vector = m.alloc(12)
    rows = []
    for v in inputs:
        for k in range(3):
            m.w(vector + 4 * k, v[k])
        m.call(0x43A970, args=(vector,), pop=0)
        rows.append([m.r(vector + 4 * k) for k in range(3)])
    return table, rows, m.visited


def main():
    args = sys.argv[1:]
    if args[:1] == ['--exe']:
        h.ORIGINAL = h.Image(Path(args[1]), h.ORIGINAL_SHA)
    rng = random.Random(0x43A970)
    inputs = vectors(rng)
    visited = {}
    tables = {}
    results = {}
    for precision in h.PRECISIONS:
        table, rows, seen = run(precision, inputs)
        tables[precision] = table
        results[precision] = rows
        visited.update(seen)
    cases = [{'vector': v, 'result': {p: results[p][i] for p in h.PRECISIONS}}
             for i, v in enumerate(inputs)]
    h.report('native-fast-normalize-fixtures.json',
             '0x8693B0 square-root mantissa table (via 0x869DC0; 0x6F7810 answered) and 0x43A970.',
             cases, visited, {'tables': tables})


if __name__ == '__main__':
    main()
