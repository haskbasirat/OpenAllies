"""Native oracle for the actor health fraction 4D0230(members) (with the
real 4CEDA0), all precisions, fully native: random actors with Bodies
(+60 +10/+14), +98 components (+14 leader, +18/+1C list, +24/+28 array) and
OrganizationComponents (+28/+2C members, +18 layout +28 slot indices, +1C
enabled table). ST0 is recorded as the 80-bit register image (a mean over
no parts is the x87 indefinite).

Usage: native_health_fraction.py  (AA.exe is located by the shared harness)
"""
from pathlib import Path
import random
import struct
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import harness as h  # noqa: E402


def f32(v):
    return struct.unpack('<I', struct.pack('<f', v))[0]


def body(rng):
    maximum = rng.choice((1.0, 3.0, 7.0, 100.0, rng.uniform(1, 500), rng.uniform(0.1, 3)))
    health = rng.choice((maximum, 0.0, maximum / 3, rng.uniform(0, maximum)))
    return [f32(health), f32(maximum)]


def make_world(rng):
    n_leaf = rng.randint(1, 7)
    leaves = []
    for _ in range(n_leaf):
        leaves.append({'body': body(rng) if rng.random() < 0.8 else None, 'c98': None, 'org': None})
    top = {'body': body(rng) if rng.random() < 0.1 else None, 'c98': None, 'org': None}
    if rng.random() < 0.7:
        list_entries = [rng.randrange(n_leaf) for _ in range(rng.randint(0, 3))]
        array = [rng.randrange(n_leaf) if rng.random() < 0.7 else None for _ in range(rng.randint(0, 3))]
        top['c98'] = {'leader': rng.randrange(n_leaf) if rng.random() < 0.6 else None,
                      'list': list_entries, 'list_count': len(list_entries) if rng.random() < 0.9 else 0,
                      'array': array, 'array_count': len(array) if rng.random() < 0.9 else -1}
    if rng.random() < 0.6 or top['c98'] is None:
        slots = rng.randint(0, 5)
        top['org'] = {'members': [rng.randrange(n_leaf) if rng.random() < 0.75 else None for _ in range(slots)],
                      'slot_index': [rng.randrange(4) for _ in range(slots)],
                      'enabled': [rng.choice((0, 1, 1, 0x100)) for _ in range(4)]}
    return {'leaves': leaves, 'top': top}


def build(m, world):
    things = world['leaves'] + [world['top']]
    ptr = [m.alloc(0x100) for _ in things]
    for p, t in zip(ptr, things):
        if t['body'] is not None:
            b = m.alloc(0x20)
            m.w(b + 0x10, t['body'][0])
            m.w(b + 0x14, t['body'][1])
            m.w(p + 0x60, b)
        if t['c98'] is not None:
            c = t['c98']
            comp = m.alloc(0x40)
            m.w(comp + 0x14, ptr[c['leader']] if c['leader'] is not None else 0)
            arr = m.alloc(4 * max(1, len(c['list'])))
            for k, e in enumerate(c['list']):
                m.w(arr + 4 * k, ptr[e])
            m.w(comp + 0x18, arr)
            m.w(comp + 0x1C, c['list_count'] & 0xFFFFFFFF)
            arr = m.alloc(4 * max(1, len(c['array'])))
            for k, e in enumerate(c['array']):
                m.w(arr + 4 * k, ptr[e] if e is not None else 0)
            m.w(comp + 0x24, arr)
            m.w(comp + 0x28, c['array_count'] & 0xFFFFFFFF)
            m.w(p + 0x98, comp)
        if t['org'] is not None:
            o = t['org']
            comp = m.alloc(0x40)
            members = m.alloc(4 * max(1, len(o['members'])))
            for k, e in enumerate(o['members']):
                m.w(members + 4 * k, ptr[e] if e is not None else 0)
            layout = m.alloc(0x40)
            indices = m.alloc(4 * max(1, len(o['slot_index'])))
            for k, e in enumerate(o['slot_index']):
                m.w(indices + 4 * k, e)
            m.w(layout + 0x28, indices)
            enabled = m.alloc(4 * 4)
            for k, e in enumerate(o['enabled']):
                m.w(enabled + 4 * k, e)
            m.w(comp + 0x18, layout)
            m.w(comp + 0x1C, enabled)
            m.w(comp + 0x28, members)
            m.w(comp + 0x2C, len(o['members']))
            m.w(p + 0x7C, comp)
    return ptr


def main():
    rng = random.Random(0x4D0230)
    cases, visited = [], {}
    for precision in h.PRECISIONS:
        m = h.Machine(precision)
        for _ in range(200):
            world = make_world(rng)
            ptr = build(m, world)
            flag = rng.choice((1, 1, 1, 0, 0x101))
            m.call(0x4D0230, ecx=ptr[-1], args=(flag,), pop=4)
            st = m.st0_ext()
            cases.append({'precision': precision, 'world': world, 'flag': flag,
                          'mantissa': f'{st["mantissa"]:016x}', 'sign_exponent': st['sign_exponent']})
        visited.update(m.visited)
    h.report('native-health-fraction-fixtures.json', '4D0230 health fraction (+4CEDA0), all precisions.',
             cases, visited)


if __name__ == '__main__':
    main()
