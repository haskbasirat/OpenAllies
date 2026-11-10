"""Native oracle for the denizen slot health 0x5690F0: a slot {+0 handle,
+8 binary32 health} reads the live denizen's Body (+60) +10, else its own +8.

Original code runs, all of it, with the handle table at [0xB364C0]; there
are no seams. The x87 result is read back as an 80-bit register image.

Usage: native_denizen_slot.py [AA.exe] (checked against the accepted SHA-256).
"""
from pathlib import Path
import hashlib
import random
import struct
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import harness as h  # noqa: E402

if len(sys.argv) > 1:
    assert hashlib.sha256(Path(sys.argv[1]).read_bytes()).hexdigest() == h.ORIGINAL_SHA, sys.argv[1]


def main():
    rng = random.Random(0x5690F0)
    m = h.Machine('Double53')
    m.u.mem_map(0, 0x1000)
    registry = m.alloc(0x20004 + 0x10000 * 4)
    m.w(h.REGISTRY_ROOT, registry)
    cases = []
    for k in range(80):
        kind = rng.choice(('live', 'live', 'stale', 'empty', 'zero'))
        body = rng.random() < 0.7
        stored = h.f32(rng.choice((0.0, rng.uniform(0, 500))))
        health = h.f32(rng.choice((0.0, rng.uniform(0, 500))))
        actor = m.alloc(0x80)
        if body:
            b = m.alloc(0x40)
            m.w(b + 0x10, health)
            m.w(actor + 0x60, b)
        m.wb(actor + 8, rng.choice((0, 1)))
        slot_index = 0x40 + k
        generation = 0x4000 + k
        m.u.mem_write(registry + 4 + slot_index * 2, struct.pack('<H', generation))
        m.w(registry + 0x20004 + slot_index * 4, 0 if kind == 'empty' else actor)
        handle = (generation << 16) | slot_index
        if kind == 'stale':
            handle += 0x10000
        if kind == 'zero':
            handle = 0
        slot = m.alloc(0x10)
        m.w(slot, handle)
        m.w(slot + 8, stored)
        m.call(0x5690F0, ecx=slot, pop=0)
        ext = m.st0_ext()
        cases.append({'kind': kind, 'body': body, 'stored': stored, 'health': health,
                      'result': [ext['mantissa'], ext['sign_exponent']]})
    h.report('native-denizen-slot-fixtures.json',
             '0x5690F0 denizen slot health (no seams).', cases, m.visited)


if __name__ == '__main__':
    main()
