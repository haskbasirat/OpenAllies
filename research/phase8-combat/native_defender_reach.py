"""Native oracle for the Defender component reach test 0x5673B0 and the
radius setter 0x566F80.

Original code runs, including 0x4C8300 (context) and 0x529C30. Recorded
seams (arguments recorded, the case decides results): actor virtual +B4,
0x4364C0 (organization member iteration, scripted order), 0x4C8740,
0x4C84B0 and 0x4C8460 (ability checks), 0x4CF110 (owner modifiers),
0x4BE910 (ability range, returned on the x87 stack as an exact binary64),
0x45D4C0 / 0x45D6C0 (the two radius getters).

Usage: native_defender_reach.py [AA.exe] (checked against the accepted SHA-256).
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

MODIFIERS = 0x00AB0000


class Rig:
    def __init__(self, precision):
        self.m = m = h.Machine(precision)
        m.u.mem_map(0, 0x1000)
        self.events = []
        self.labels = {}
        m.hooks.update({
            0x4364C0: self._next_member,
            0x4C8740: self._check('admits', 8),
            0x4C84B0: self._check('within', 8),
            0x4C8460: self._check('clear', 8),
            0x4CF110: self._modifiers,
            0x4BE910: self._range,
            0x45D4C0: self._radius('wide'),
            0x45D6C0: self._radius('normal'),
        })
        self.vt = m.vtable({0xB4: self._qualifies})

    def label(self, address):
        return self.labels.get(address, 0 if address == 0 else ('?', address))

    def _qualifies(self, mm):
        actor = self.label(mm.reg(UC_X86_REG_ECX))
        self.events.append({'qualifies': actor})
        mm.ret(0, int(actor in self.case['qualifies']))

    def _next_member(self, mm):
        it = mm.reg(UC_X86_REG_ECX)
        order = self.orders[mm.r(it)]
        index = struct.unpack('<i', struct.pack('<I', mm.r(it + 4)))[0]
        position = order.index(index) + 1 if index != -1 else 0
        mm.w(it + 4, order[position] if position < len(order) else 0x7FFFFFFF)
        mm.ret(0)

    def _check(self, name, pop):
        def hook(mm):
            ability = self.label(mm.reg(UC_X86_REG_ECX))
            if name == 'admits':
                ctx = mm.arg(0)
                words = list(struct.unpack('<12I', bytes(mm.u.mem_read(ctx, 0x30))))
                words[2] = self.label(words[2])
                words[7] = self.label(words[7])
                self.events.append({name: ability, 'context': words, 'flag': mm.arg(1)})
                target = words[7]
            else:
                assert mm.arg(0) == self.owner
                point = mm.arg(1)
                xy = [mm.r(point), mm.r(point + 4)]
                self.events.append({name: ability, 'point': xy})
                target = next(t['id'] for t in self.case['actors'] if t['position'] == xy)
            mm.ret(pop, int(f'{name}:{ability}:{target}' not in self.case['refuse']))
        return hook

    def _modifiers(self, mm):
        assert mm.reg(UC_X86_REG_ECX) == self.owner
        self.events.append({'modifiers': 1})
        mm.ret(0, MODIFIERS)

    def _range(self, mm):
        definition = mm.reg(UC_X86_REG_ECX)
        assert mm.arg(0) == MODIFIERS
        ability = self.def_labels[definition]
        self.events.append({'range': ability})
        mm.ret_ext(4, self.case['ranges'][ability])

    def _radius(self, name):
        def hook(mm):
            assert mm.reg(UC_X86_REG_ECX) == self.data
            self.events.append({'radius': name, 'arg': mm.arg(0)})
            mm.ret_ext(4, self.case['radius_value'])
        return hook

    def build(self, case):
        m = self.m
        self.case = case
        self.events = []
        self.labels = {}
        self.orders = {}
        self.def_labels = {}
        actors = {}
        for spec in case['actors']:
            a = m.alloc(0x200)
            m.w(a, self.vt)
            m.w(a + 8, spec['flags'])
            m.w(a + 0x20, spec['position'][0])
            m.w(a + 0x24, spec['position'][1])
            actors[spec['id']] = a
            self.labels[a] = spec['id']
        for spec in case['actors']:
            if spec['members'] is not None:
                org = m.alloc(0x40)
                arr = m.alloc(0x40)
                m.w(org + 0x28, arr)
                for k, member in enumerate(spec['members']):
                    m.w(arr + 4 * k, actors.get(member, 0))
                self.orders[org] = spec['order']
                m.w(actors[spec['id']] + 0x7C, org)
        owner = m.alloc(0x200)
        data = m.alloc(0x200)
        m.w(owner + 4, data)
        m.w(data + 0x174, case['data174'])
        m.w(owner + 0x20, case['owner_position'][0])
        m.w(owner + 0x24, case['owner_position'][1])
        self.owner, self.data = owner, data
        self.labels[owner] = 'owner'
        abilities = m.alloc(0x40)
        for k, ab in enumerate(case['abilities']):
            p = m.alloc(0x40)
            d = m.alloc(0x80)
            m.w(p + 4, d)
            m.w(d + 0x3C, ab['projectile'])
            m.w(abilities + 4 * k, p)
            self.labels[p] = f'a{k}'
            self.def_labels[d] = f'a{k}'
        defender = m.alloc(0x40)
        m.w(defender + 4, owner)
        m.w(defender + 0x10, abilities)
        m.w(defender + 0x14, len(case['abilities']))
        m.w(defender + 0x2C, 0x11111111)
        self.defender = defender
        return actors


def make_case(rng):
    n = rng.randint(1, 5)
    actors = []
    for k in range(n):
        actors.append({'id': f't{k}', 'flags': rng.choice((0, 0x4000, 0x4000, 0x4001)),
                       'position': [h.f32(rng.uniform(-200, 200)), h.f32(rng.uniform(-200, 200))],
                       'members': None, 'order': None})
    if n >= 2 and rng.random() < 0.5:
        members = [a['id'] for a in actors[1:] if rng.random() < 0.8]
        if rng.random() < 0.3:
            members.append('missing')
        order = list(range(len(members)))
        rng.shuffle(order)
        actors[0]['members'] = members
        actors[0]['order'] = order
    ids = [a['id'] for a in actors]
    abilities = [{'projectile': rng.choice((0, 1, 1))} for _ in range(rng.randint(0, 3))]
    refuse = []
    for k in range(len(abilities)):
        for t in ids:
            for name in ('admits', 'within', 'clear'):
                if rng.random() < 0.2:
                    refuse.append(f'{name}:a{k}:{t}')
    owner_position = [h.f32(rng.uniform(-200, 200)), h.f32(rng.uniform(-200, 200))]
    ranges = {}
    for k in range(len(abilities)):
        if rng.random() < 0.3 and actors:
            # a range landing on the squared distance of one candidate
            t = rng.choice(actors)
            dx = h.from_f32(t['position'][0]) - h.from_f32(owner_position[0])
            dy = h.from_f32(t['position'][1]) - h.from_f32(owner_position[1])
            value = (dx * dx + dy * dy) ** 0.5
        else:
            value = rng.uniform(0, 300)
        ranges[f'a{k}'] = h.f64(value)
    return {'actors': actors, 'abilities': abilities, 'refuse': refuse, 'ranges': ranges,
            'owner_position': owner_position,
            'qualifies': [t for t in ids if rng.random() < 0.6],
            'candidate': rng.choice(ids + ['none']),
            'data174': rng.choice((0, 0x80, 0x7F, 0xFF)),
            'radius_value': h.f64(rng.uniform(0, 500)), 'radius_arg': rng.getrandbits(32)}


def main():
    rng = random.Random(0x5673B0)
    specs = [make_case(rng) for _ in range(160)]
    cases = []
    visited = {}
    for precision in h.PRECISIONS:
        rig = Rig(precision)
        m = rig.m
        for spec in specs:
            actors = rig.build(spec)
            candidate = actors.get(spec['candidate'], 0)
            result = m.call(0x5673B0, ecx=rig.defender, args=(candidate,), pop=4) & 0xFF
            reach_events = rig.events
            rig.events = []
            m.call(0x566F80, ecx=rig.defender, args=(spec['radius_arg'],), pop=4)
            cases.append({'precision': precision, 'case': spec, 'result': result, 'events': reach_events,
                          'radius_events': rig.events, 'radius': m.r(rig.defender + 0x2C),
                          'radius_dirty': m.rb(rig.defender + 0x30)})
        visited.update(m.visited)
    h.report('native-defender-reach-fixtures.json',
             '0x5673B0 Defender reach test and 0x566F80 radius setter (+B4/4364C0/4C8740/4C84B0/4C8460/'
             '4CF110/4BE910/45D4C0/45D6C0 recorded) at PC24, PC53, PC64.',
             cases, visited)


if __name__ == '__main__':
    main()
