"""Native oracle for kingdom statistics class tracking: 0x58F010 (track),
0x58EFE0 (untrack) with their helpers 0x58EBA0/0x58E8B0/0x58E890, the
structure class 0x58EE00 (+0x45ECA0) and the predicate 0x58EA80.

Everything runs natively (no seams): the statistics object (0x108 bytes,
scores at +0x28 + 4*i), the actor (class word +0x13C, components, virtual
+0x100 through a vtable stub) and the world byte [0xB36534]+0x274.
All three x87 precisions.
"""
from pathlib import Path
import random
import struct
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import harness as h  # noqa: E402

TRACK, UNTRACK, PREDICATE = 0x58F010, 0x58EFE0, 0x58EA80
SCORES = list(range(33, 34)) + list(range(47, 56))


def f32(v):
    return struct.unpack('<I', struct.pack('<f', v))[0]


def case(m, rng, entry):
    facts = {
        'component_7c': rng.random() < 0.5, 'component_d8': rng.random() < 0.25,
        'structure': rng.random() < 0.5, 'virtual_100': rng.random() < 0.6,
        'construction': rng.random() < 0.5, 'data_list_43c': rng.random() < 0.4,
        'site_class': rng.choice((0, 0, 3)), 'component_88': rng.random() < 0.4,
        'component_90': rng.random() < 0.3, 'definition_4d0': rng.random() < 0.5,
        'component_98': rng.choice((None, None, rng.randint(0, 9), rng.randint(-3, 40))),
        'component_d4': rng.random() < 0.3,
        'component_80': rng.random() < 0.6, 'component_a4': rng.random() < 0.6,
    }
    classes = list(range(47, 57)) + [0, 60]
    initial = rng.choice(classes)
    if initial in (50, 51) and facts['component_98'] is None:
        facts['component_98'] = rng.randint(0, 9)
    enabled = rng.random() < 0.85
    scores = {i: rng.choice((0, f32(rng.randint(0, 20)), f32(rng.uniform(0, 50)))) for i in SCORES}
    m.w(0xB36534 + 0, m.world)
    m.wb(m.world + 0x274, int(enabled))
    stats = m.alloc(0x108)
    for i, bits in scores.items():
        m.w(stats + 0x28 + 4 * i, bits)
    actor = m.alloc(0x200)
    definition = m.alloc(0x500)
    m.w(actor + 4, definition)
    calls = []

    def virtual(mm):
        calls.append('virtual_100')
        mm.ret(0, int(facts['virtual_100']))
    m.w(actor, m.vtable({0x100: virtual}))
    for key, offset in (('component_7c', 0x7C), ('component_d8', 0xD8), ('structure', 0x94),
                        ('construction', 0x9C), ('component_88', 0x88), ('component_90', 0x90),
                        ('component_d4', 0xD4), ('component_80', 0x80), ('component_a4', 0xA4)):
        if facts[key]:
            m.w(actor + offset, m.alloc(0x20))
    if facts['component_98'] is not None:
        c = m.alloc(0x20)
        m.w(c + 0x10, facts['component_98'] & 0xFFFFFFFF)
        m.w(actor + 0x98, c)
    m.w(definition + 0x4D0, int(facts['definition_4d0']))
    m.w(definition + 0x4E0, facts['site_class'])
    m.w(definition + 0x43C, m.alloc(8) if facts['data_list_43c'] else 0)
    m.w(actor + 0x13C, initial)
    if entry == 'track':
        m.call(TRACK, ecx=stats, args=(actor,), pop=4)
    elif entry == 'untrack':
        m.call(UNTRACK, ecx=stats, args=(actor,), pop=4)
    else:
        result = m.call(PREDICATE, args=(actor,), pop=0)
        return {'entry': entry, 'facts': facts, 'result': result}
    return {'precision': m.precision, 'entry': entry, 'facts': facts, 'enabled': enabled,
            'initial_class': initial, 'scores': {str(k): v for k, v in scores.items()},
            'final_class': m.r(actor + 0x13C),
            'final_scores': {str(i): m.r(stats + 0x28 + 4 * i) for i in SCORES},
            'virtual_calls': len(calls)}


def main():
    rng = random.Random(0x58F010)
    cases, visited = [], {}
    for precision in h.PRECISIONS:
        m = h.Machine(precision)
        m.world = m.alloc(0x400)
        for i in range(400):
            cases.append(case(m, rng, 'track' if i % 4 else 'untrack'))
        for i in range(40):
            cases.append(case(m, rng, 'predicate'))
        visited.update(m.visited)
    h.report('native-statistics-tracking-fixtures.json',
             '0x58F010/0x58EFE0 statistics class tracking (0x58EBA0, 0x58E8B0, 0x58E890, 0x58EE00, '
             '0x45ECA0 native) and the 0x58EA80 predicate; all precisions.',
             cases, visited)


if __name__ == '__main__':
    main()
