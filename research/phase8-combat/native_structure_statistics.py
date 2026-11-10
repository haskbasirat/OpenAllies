"""Native oracle for the structure statistics 0x58EF60 and 0x584B70.

Original code runs, including 0x4CEEC0 and 0x58E870 (score add). Recorded
seams: 0x58EBA0 (leave category) and 0x58E8B0 (join category), arguments
recorded, and the owner virtual +108 (its kingdom, scripted).

Usage: native_structure_statistics.py [AA.exe] (checked against the accepted SHA-256).
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

CATEGORIES = (0x2F, 0x32, 0x33, 0x38, 0x10)


def main():
    rng = random.Random(0x58EF60)
    cases = []
    visited = {}
    specs = []
    for _ in range(120):
        specs.append({'function': 'set_category', 'enabled': rng.random() < 0.85,
                      'current': rng.choice(CATEGORIES), 'category': rng.choice(CATEGORIES),
                      'amount': h.f32(rng.choice((1.0, 3.0, rng.uniform(-5, 50)))),
                      'score33': h.f32(rng.uniform(0, 1000)), 'score18': h.f32(rng.uniform(0, 100))})
    for _ in range(80):
        specs.append({'function': 'count_lost', 'enabled': rng.random() < 0.85,
                      'current': rng.choice(CATEGORIES), 'component_count': rng.randint(-2, 12),
                      'count': rng.randint(-2, 12), 'kingdom': rng.random() < 0.8,
                      'statistics': rng.random() < 0.8,
                      'score33': h.f32(rng.uniform(0, 1000)), 'score18': h.f32(rng.uniform(0, 100))})
    for precision in h.PRECISIONS:
        m = h.Machine(precision)
        m.u.mem_map(0, 0x1000)
        world = m.alloc(0x300)
        m.w(h.WORLD_ROOT, world)
        events = []
        state = {}
        m.hooks[0x58EBA0] = lambda mm: (events.append({'leave': mm.arg(0) == state['actor']}), mm.ret(4))
        m.hooks[0x58E8B0] = lambda mm: (events.append({'join': mm.arg(0) == state['actor'], 'category': mm.arg(1),
                                                        'amount': mm.arg(2)}), mm.ret(12))
        owner_vt = m.vtable({0x108: lambda mm: mm.ret(0, state['kingdom'])})
        for spec in specs:
            events.clear()
            m.wb(world + 0x274, int(spec['enabled']))
            stats = m.alloc(0x108)
            m.w(stats + 0xF4, spec['score33'])
            m.w(stats + 0x28 + 4 * 0x18, spec['score18'])
            actor = m.alloc(0x200)
            m.w(actor, owner_vt)
            m.w(actor + 0x13C, spec['current'])
            state['actor'] = actor
            if spec['function'] == 'set_category':
                m.call(0x58EF60, ecx=stats, args=(actor, spec['category'], spec['amount']), pop=12)
            else:
                kingdom = m.alloc(0x400) if spec['kingdom'] else 0
                if kingdom:
                    m.w(kingdom + 0x32C, stats if spec['statistics'] else 0)
                state['kingdom'] = kingdom
                comp = m.alloc(0x40)
                m.w(comp + 4, actor)
                m.w(comp + 0x10, spec['component_count'] & 0xFFFFFFFF)
                m.call(0x584B70, ecx=comp, args=(0xDEAD, spec['count'] & 0xFFFFFFFF), pop=8)
            cases.append({'precision': precision, 'case': spec, 'events': list(events),
                          'score33': m.r(stats + 0xF4), 'score18': m.r(stats + 0x28 + 4 * 0x18)})
        visited.update(m.visited)
    h.report('native-structure-statistics-fixtures.json',
             '0x58EF60 structure statistic category and 0x584B70 count loss (58EBA0/58E8B0/+108 recorded) '
             'at PC24, PC53, PC64.', cases, visited)


if __name__ == '__main__':
    main()
