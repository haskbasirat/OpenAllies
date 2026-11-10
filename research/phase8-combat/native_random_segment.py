"""Native oracle for the settlement (wall) component random segment pick
0x5853E0. Original code runs, including the world RNG getter 0x4F02B0, the
LCG 0x85D840 and 0x529C30; recorded seam: 0x77DA70 (the two sync-log lines
of 0x4F02B0, arguments recorded).

Usage: native_random_segment.py [AA.exe] (checked against the accepted
SHA-256 by the shared harness).
"""
from pathlib import Path
import hashlib
import random
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import harness as h  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX  # noqa: E402

if len(sys.argv) > 1:
    assert hashlib.sha256(Path(sys.argv[1]).read_bytes()).hexdigest() == h.ORIGINAL_SHA, sys.argv[1]


def main():
    rng = random.Random(0x5853E0)
    m = h.Machine('Double53')
    m.u.mem_map(0, 0x1000)
    world = m.alloc(0x200)
    m.w(h.WORLD_ROOT, world)
    m.w(0xB3655C, 0x10C0001)
    m.w(0xB36560, 0x10C0002)
    events = []

    def sync_log(mm):
        text = bytes(mm.u.mem_read(mm.arg(0), 40)).decode('utf-16le').split('\x00')[0]
        events.append({'log': mm.reg(UC_X86_REG_ECX) - 0x10C0000, 'text': text, 'value': mm.arg(1),
                       'flag': mm.arg(2)})
        mm.ret(12)
    m.hooks[0x77DA70] = sync_log
    cases = []
    for index in range(160):
        events.clear()
        count = rng.choice((0, 1, 1, 2, 3, 5, 8, 13))
        segments = [0 if rng.random() < 0.25 else 0x7000 + 0x10 * k for k in range(count)]
        kinds = [rng.choice((0, 1, 1, 2, 3)) for _ in range(count)]
        if index % 7 == 0:
            kinds = [1] * count
        seed = rng.getrandbits(32)
        comp = m.alloc(0x40)
        actor = m.alloc(0x40)
        data = m.alloc(0x500)
        rows = m.alloc(0x14 * max(count, 1))
        array = m.alloc(4 * max(count, 1))
        m.w(comp + 4, actor)
        m.w(actor + 4, data)
        m.w(data + 0x47C, rows)
        for k in range(count):
            m.w(rows + 0x14 * k, kinds[k])
            m.w(rows + 0x14 * k + 4, 0xEEEE0000 + k)
            m.w(array + 4 * k, segments[k])
        m.w(comp + 0x24, array)
        m.w(comp + 0x28, count)
        m.w(world + 0xD4, seed)
        result = m.call(0x5853E0, ecx=comp, pop=0)
        cases.append({'segments': segments, 'kinds': kinds, 'seed': seed, 'result': result,
                      'seed_after': m.r(world + 0xD4), 'events': list(events)})
    h.report('native-random-segment-fixtures.json',
             '0x5853E0 settlement random segment pick (4F02B0/85D840 world RNG run; 77DA70 sync log recorded).',
             cases, m.visited)


if __name__ == '__main__':
    main()
