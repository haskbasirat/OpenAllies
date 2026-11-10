"""Native oracle for the Performer attack efficiency 0x57DBF0: the shooter's
organization attack efficiency (actor +80 membership, +10 organization
actor, its +7C component +84), else the actor's modifier offense preference
(0x4CF110 -> 0x501530: +58, then +5C, then 1.0).

Original code runs, all of it (0x57DBF0, 0x4CF110, 0x501530); there are no
seams. The x87 result is read back as an 80-bit register image.

Usage: native_attack_efficiency.py [AA.exe] (checked against the accepted SHA-256).
"""
from pathlib import Path
import hashlib
import random
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import harness as h  # noqa: E402

if len(sys.argv) > 1:
    assert hashlib.sha256(Path(sys.argv[1]).read_bytes()).hexdigest() == h.ORIGINAL_SHA, sys.argv[1]


def value(rng):
    return rng.choice((h.f32(rng.uniform(0.01, 4)), 0, 0x80000000, h.f32(-rng.uniform(0.01, 4)),
                       h.f32(1.0), 1, h.f32(rng.uniform(0.01, 4))))


def main():
    rng = random.Random(0x57DBF0)
    cases = []
    visited = {}
    specs = []
    for _ in range(200):
        specs.append({
            'membership': rng.random() < 0.6,
            'organization': rng.random() < 0.7,
            'efficiency': value(rng),
            'modifiers': rng.random() < 0.7,
            'primary': value(rng),
            'secondary': value(rng),
        })
    for precision in h.PRECISIONS:
        m = h.Machine(precision)
        m.u.mem_map(0, 0x1000)
        for spec in specs:
            perf = m.alloc(0x40)
            actor = m.alloc(0x100)
            m.w(perf + 4, actor)
            if spec['membership']:
                membership = m.alloc(0x40)
                m.w(actor + 0x80, membership)
                if spec['organization']:
                    org = m.alloc(0x100)
                    component = m.alloc(0x100)
                    m.w(membership + 0x10, org)
                    m.w(org + 0x7C, component)
                    m.w(component + 0x84, spec['efficiency'])
            if spec['modifiers']:
                holder = m.alloc(0x100)
                m.w(actor + 0x78, holder)
                m.w(holder + 0x1C + 0x58, spec['primary'])
                m.w(holder + 0x1C + 0x5C, spec['secondary'])
            m.call(0x57DBF0, ecx=perf, pop=0)
            ext = m.st0_ext()
            cases.append({'precision': precision, 'case': spec,
                          'result': [ext['mantissa'], ext['sign_exponent']]})
        visited.update(m.visited)
    h.report('native-attack-efficiency-fixtures.json',
             '0x57DBF0 Performer attack efficiency with 0x4CF110 and 0x501530 (no seams) at PC24, PC53, PC64.',
             cases, visited)


if __name__ == '__main__':
    main()
