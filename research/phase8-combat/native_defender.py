"""Native oracle for the Defender component (towers, bunkers, anti-air):
the update 0x567960, the best intruder 0x567530 and the threat rank 0x566D40.

Original code runs, including 0x4C8300 (context), 0x45E790 (distance),
0x4C81D0 / 0x4BE520 / 0x4C81E0 (ability turret) and the handle table at
[0xB364C0]. Recorded seams (arguments recorded, the case decides results):
0x515F40 (radius refresh), 0x4C8F20 (ability admission; it also writes the
context's +2C word to show the context is shared across abilities),
0x567140 (set target; emulated: ability +14 takes the target's +14 handle),
actor virtual +B4, 0x4364C0 (organization member iteration, scripted
order), component +70 virtual +58, 0x4D03F0, 0x4D1120, and on the owner's
Performer (+74) 0x57E730 (aim) and 0x57E030 (use).

Usage: native_defender.py [AA.exe] (checked against the accepted SHA-256).
"""
from pathlib import Path
import hashlib
import random
import struct
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import harness as h  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX  # noqa: E402

if len(sys.argv) > 1:
    assert hashlib.sha256(Path(sys.argv[1]).read_bytes()).hexdigest() == h.ORIGINAL_SHA, sys.argv[1]


class Rig:
    def __init__(self, precision):
        self.m = m = h.Machine(precision)
        m.u.mem_map(0, 0x1000)
        self.registry = m.alloc(0x20004 + 0x10000 * 4)
        m.w(h.REGISTRY_ROOT, self.registry)
        self.slot = 0x40
        self.events = []
        self.labels = {}
        self.case = None
        self.calls = 0
        m.hooks.update({
            0x515F40: self._radius,
            0x4C8F20: self._can_use,
            0x567140: self._set_target,
            0x4364C0: self._next_member,
            0x4D03F0: self._harmless,
            0x4D1120: self._capturable,
            0x57E730: self._aim,
            0x57E030: self._use,
        })

    def label(self, address):
        return self.labels.get(address, 0 if address == 0 else ('?', address))

    def context(self, address):
        words = list(struct.unpack('<12I', bytes(self.m.u.mem_read(address, 0x30))))
        words[2] = self.label(words[2])
        words[7] = self.label(words[7])
        return words

    # seams -------------------------------------------------------------------
    def _radius(self, mm):
        self.events.append({'radius': [mm.arg(0), mm.arg(1), mm.arg(2)],
                            'zone': int(mm.reg(UC_X86_REG_ECX) != 0)})
        if mm.reg(UC_X86_REG_ECX):
            assert mm.reg(UC_X86_REG_ECX) == self.zone_object
        mm.ret(12)

    def _can_use(self, mm):
        ability = self.label(mm.reg(UC_X86_REG_ECX))
        ctx = mm.arg(0)
        words = self.context(ctx)
        self.events.append({'can_use': ability, 'check_range': mm.arg(1), 'context': words})
        self.calls += 1
        mm.w(ctx + 0x2C, 0x100 + self.calls)
        target = words[7]
        result = self.case['admission'].get(f'{ability}:{target}', 0)
        mm.ret(8, result)

    def _set_target(self, mm):
        assert mm.reg(UC_X86_REG_ECX) == self.defender
        ability_ptr = mm.arg(0)
        target = mm.arg(1)
        self.events.append({'set_target': self.label(ability_ptr), 'target': self.label(target)})
        mm.w(ability_ptr + 0x14, mm.r(target + 0x14) if target else 0)
        mm.ret(8)

    def _qualifies(self, mm):
        actor = self.label(mm.reg(UC_X86_REG_ECX))
        self.events.append({'qualifies': actor})
        mm.ret(0, int(actor in self.case['qualifies']))

    def _aggressive(self, mm):
        actor = self.label(self.m.r(mm.reg(UC_X86_REG_ECX) + 4))
        self.events.append({'aggressive': actor})
        mm.ret(0, int(actor in self.case['aggressive']))

    def _harmless(self, mm):
        actor = self.label(mm.reg(UC_X86_REG_ECX))
        self.events.append({'harmless': actor})
        mm.ret(0, int(actor in self.case['harmless']))

    def _capturable(self, mm):
        actor = self.label(mm.reg(UC_X86_REG_ECX))
        self.events.append({'capturable': actor})
        mm.ret(0, int(actor in self.case['capturable']))

    def _next_member(self, mm):
        it = mm.reg(UC_X86_REG_ECX)
        org = mm.r(it)
        order = self.orders[org]
        index = struct.unpack('<i', struct.pack('<I', mm.r(it + 4)))[0]
        position = order.index(index) + 1 if index != -1 else 0
        mm.w(it + 4, order[position] if position < len(order) else 0x7FFFFFFF)
        mm.ret(0)

    def _aim(self, mm):
        assert mm.reg(UC_X86_REG_ECX) == self.performer
        point = mm.arg(1)
        handle = mm.arg(2)
        target = self.by_handle.get(handle, 0)
        self.events.append({'aim': struct.unpack('<i', struct.pack('<I', mm.arg(0)))[0],
                            'point': [mm.r(point), mm.r(point + 4)], 'handle': target})
        mm.ret(12, int(target in self.case['aimed']))

    def _use(self, mm):
        assert mm.reg(UC_X86_REG_ECX) == self.performer
        self.events.append({'use': self.label(mm.arg(0)), 'context': self.context(mm.arg(1))})
        mm.ret(8)

    # world -------------------------------------------------------------------
    def build(self, case):
        m = self.m
        self.case = case
        self.events = []
        self.labels = {}
        self.calls = 0
        self.orders = {}
        self.by_handle = {}
        handles = {}
        actors = {}
        vt = m.vtable({0xB4: self._qualifies})
        comp_vt = m.vtable({0x58: self._aggressive})
        for spec in case['actors']:
            a = m.alloc(0x300)
            data = m.alloc(0x300)
            m.w(a, vt)
            m.w(a + 4, data)
            m.wb(data + 0x291, spec['data291'])
            m.w(a + 8, spec['flags'])
            m.w(a + 0x20, spec['position'][0])
            m.w(a + 0x24, spec['position'][1])
            m.w(a + 0x60, spec['field60'])
            if spec['component70']:
                c = m.alloc(0x10)
                m.w(c, comp_vt)
                m.w(c + 4, a)
                m.w(a + 0x70, c)
            self.slot += 1
            generation = 0x2000 + self.slot
            handle = (generation << 16) | self.slot
            m.u.mem_write(self.registry + 4 + self.slot * 2, struct.pack('<H', generation))
            m.w(self.registry + 0x20004 + self.slot * 4, 0 if spec['slot'] == 'empty' else a)
            stored = handle if spec['slot'] != 'stale' else handle + 0x10000
            m.w(a + 0x14, stored & 0xFFFFFFFF)
            handles[spec['id']] = stored & 0xFFFFFFFF
            actors[spec['id']] = a
            self.labels[a] = spec['id']
            self.by_handle[stored & 0xFFFFFFFF] = spec['id']
        for spec in case['actors']:
            if spec['members'] is not None:
                org = m.alloc(0x40)
                arr = m.alloc(4 * 16)
                m.w(org + 0x28, arr)
                for k, member in enumerate(spec['members']):
                    m.w(arr + 4 * k, actors.get(member, 0))
                self.orders[org] = spec['order']
                m.w(actors[spec['id']] + 0x7C, org)
        owner = m.alloc(0x300)
        self.performer = m.alloc(0x80)
        m.w(owner + 0x74, self.performer)
        m.w(owner + 0x20, case['owner_position'][0])
        m.w(owner + 0x24, case['owner_position'][1])
        self.zone = m.alloc(0x40)
        self.zone_object = m.alloc(0x40)
        m.w(self.zone + 0x10, self.zone_object)
        m.w(owner + 0x14C, 5 if case['zone'] else 4)
        m.w(owner + 0x148, self.zone)
        self.labels[owner] = 'owner'
        abilities = m.alloc(4 * max(len(case['abilities']), 1))
        for k, ab in enumerate(case['abilities']):
            p = m.alloc(0x40)
            d1 = m.alloc(0x80)
            d2 = m.alloc(0x80)
            m.w(p + 4, d1)
            m.w(p + 8, d2)
            m.w(d1 + 0x54, ab['flags54'])
            m.w(d2 + 0x54, ab['turret'] & 0xFFFFFFFF)
            m.w(p + 0x14, handles.get(ab['target'], ab['target'] if isinstance(ab['target'], int) else 0))
            m.w(abilities + 4 * k, p)
            self.labels[p] = f'a{k}'
        d = m.alloc(0x40)
        self.defender = d
        m.w(d + 4, owner)
        m.w(d + 0x10, abilities)
        m.w(d + 0x14, len(case['abilities']))
        head = 0
        for who in reversed(case['intruders']):
            node = m.alloc(0x10)
            m.w(node, handles.get(who, who if isinstance(who, int) else 0))
            m.w(node + 4, head)
            head = node
        m.w(d + 0x1C, head)
        m.w(d + 0x2C, case['radius'])
        m.wb(d + 0x30, int(case['radius_dirty']))
        self.handles = handles
        return d


def make_case(rng, index):
    n = rng.randint(1, 7)
    actors = []
    for k in range(n):
        actors.append({
            'id': f't{k}',
            'position': [h.f32(rng.uniform(-300, 300)), h.f32(rng.uniform(-300, 300))],
            'flags': rng.choice((0, 0, 0, 1, 0x800)),
            'field60': rng.choice((0, 1, 1)),
            'data291': rng.choice((0, 1, 1)),
            'component70': rng.random() < 0.3,
            'slot': rng.choice(('live',) * 8 + ('empty', 'stale')),
            'members': None,
            'order': None,
        })
    # One organization actor with members drawn from the others.
    if n >= 3 and rng.random() < 0.5:
        org = actors[0]
        members = [a['id'] for a in actors[1:] if rng.random() < 0.7]
        if rng.random() < 0.3:
            members.append('missing')
        org['members'] = members
        order = list(range(len(members)))
        rng.shuffle(order)
        org['order'] = order
    if index % 11 == 0:
        # equal distances: a tie at the same rank keeps the first
        for a in actors[1:3]:
            a['position'] = list(actors[0]['position'])
    ids = [a['id'] for a in actors]
    abilities = []
    for k in range(rng.randint(1, 3)):
        abilities.append({
            'flags54': rng.choice((0, 8, 0x10, 0x18)),
            'turret': rng.choice((-1, -1, 0, 1)),
            'target': rng.choice(ids + [0, 0]),
        })
    admission = {}
    for k in range(len(abilities)):
        for t in ids:
            if rng.random() < 0.35:
                admission[f'a{k}:{t}'] = rng.choice((1, 3, 7, 8, 5))
    return {
        'actors': actors,
        'abilities': abilities,
        'intruders': [rng.choice(ids) for _ in range(rng.randint(0, 6))],
        'owner_position': [h.f32(rng.uniform(-300, 300)), h.f32(rng.uniform(-300, 300))],
        'radius': h.f32(rng.uniform(10, 400)),
        'radius_dirty': rng.random() < 0.3,
        'zone': rng.random() < 0.6,
        'admission': admission,
        'qualifies': [t for t in ids if rng.random() < 0.6],
        'aggressive': [t for t in ids if rng.random() < 0.3],
        'harmless': [t for t in ids if rng.random() < 0.5],
        'capturable': [t for t in ids if rng.random() < 0.5],
        'aimed': [t for t in ids if rng.random() < 0.6],
    }


def main():
    rng = random.Random(0x567960)
    specs = [make_case(rng, i) for i in range(150)]
    cases = []
    visited = {}
    for precision in h.PRECISIONS:
        rig = Rig(precision)
        m = rig.m
        for spec in specs:
            d = rig.build(spec)
            m.call(0x567960, ecx=d + 8, args=(0, 0), pop=8)
            after = [m.r(m.r(m.r(d + 0x10) + 4 * k) + 0x14) for k in range(len(spec['abilities']))]
            cases.append({'precision': precision, 'case': spec, 'handles': rig.handles,
                          'events': rig.events, 'targets_after': [rig.by_handle.get(x, x) for x in after],
                          'radius_dirty_after': m.rb(d + 0x30)})
        visited.update(m.visited)
    h.report('native-defender-fixtures.json',
             '0x567960 Defender update with 0x567530 best intruder and 0x566D40 threat rank '
             '(515F40/4C8F20/567140/+B4/4364C0/+70+58/4D03F0/4D1120/57E730/57E030 recorded) at PC24, PC53, PC64.',
             cases, visited)


if __name__ == '__main__':
    main()
