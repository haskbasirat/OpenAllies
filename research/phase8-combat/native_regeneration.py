"""Native oracle for organization regeneration 0x57B5A0 and regroup 0x57AEB0.

Original code runs for the slot choice, 0x5617C0, 0x4CF110, 0x45D5B0 (with
the 0x5014E0 modifier multiply), 0x45D730, the member iterator 0x4364C0 and
all x87 arithmetic, at all three precisions. Seams (recorded, never
executed): 0x562BA0 plain heal, 0x50C380 regeneration presentation,
0x57AD30 member spawn (returns the case's al and, on success for regroup,
installs the new leader member), 0x561E50 set health, 0x4B2E60 SVar lookup,
owner virtual +108 (allegiance).
"""
from pathlib import Path
import random
import struct
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import harness as h  # noqa: E402
from unicorn import x86_const as X  # noqa: E402

REGENERATE, REGROUP = 0x57B5A0, 0x57AEB0
HEAL, PRESENT, SPAWN, SET_HEALTH, LOOKUP = 0x562BA0, 0x50C380, 0x57AD30, 0x561E50, 0x4B2E60


def f32(v):
    return struct.unpack('<I', struct.pack('<f', v))[0]


class Oracle(h.Machine):
    def __init__(self, precision):
        super().__init__(precision)
        self.u.mem_map(0, 0x1000)
        self.events = []
        self.hooks[HEAL] = self._heal
        self.hooks[PRESENT] = self._present
        self.hooks[SPAWN] = self._spawn
        self.hooks[SET_HEALTH] = self._set_health
        self.hooks[LOOKUP] = self._lookup
        self.world = self.alloc(0x200)
        self.w(h.WORLD_ROOT, self.world)
        self.w(self.world + 0x130, self.alloc(0x10))

    def alloc(self, size, align=16):
        ptr = super().alloc(size, align)
        self.u.mem_write(ptr, bytes(size))
        return ptr

    def _heal(self, m):
        record = m.arg(0)
        self.events.append({'heal': self.member_of_body[m.reg(X.UC_X86_REG_ECX)],
                            'source': m.r(record) == self.owner, 'amount': m.r(record + 4)})
        m.ret(4)

    def _present(self, m):
        record = m.arg(1)
        self.events.append({'present': m.r(record + 4), 'owner': m.arg(0) == self.owner,
                            'source': m.r(record) == self.owner,
                            'object': m.reg(X.UC_X86_REG_ECX) == m.r(self.world + 0x130)})
        m.ret(8)

    def _spawn(self, m):
        slot = m.arg(0)
        self.events.append({'spawn': slot, 'flag': m.arg(1)})
        if self.spawn_result and self.spawn_member is not None:
            self.w(self.org_members + 4 * slot, self.spawn_member)
        m.ret(8, self.spawn_result)

    def _set_health(self, m):
        self.events.append({'set_health': self.member_of_body.get(m.reg(X.UC_X86_REG_ECX)),
                            'value': m.arg(0), 'rest': [m.arg(1), m.arg(2), m.arg(3)]})
        m.ret(16)

    def _lookup(self, m):
        m.ret(8, self.ratio_cell)


def random_case(rng):
    count = rng.randint(1, 6)
    leader = rng.choice((-1, 0, 0, rng.randrange(count)))
    members = []
    for _ in range(count):
        if rng.random() < 0.3:
            members.append(None)
            continue
        maximum = f32(rng.uniform(10, 300))
        health = rng.choice((maximum, f32(rng.uniform(0, 300)), f32(rng.uniform(1, 200))))
        members.append({'body': rng.random() < 0.9, 'health': health, 'maximum': maximum,
                        'attached': rng.random() < 0.6, 'regeneration': f32(rng.uniform(0, 0.2))})
    definitions = [None if rng.random() < 0.15 else
                   {'flags_174': rng.choice((0, 1, 1, 3)), 'regeneration_2cc': f32(rng.uniform(0, 0.3)),
                    'health_284': f32(rng.uniform(10, 300))} for _ in range(count)]
    if leader >= 0 and members[leader] is not None:
        # A body-less leader-slot member would be dereferenced as null.
        members[leader]['body'] = True
    if leader >= 0 and definitions[leader] is None:
        # A null leader-slot definition would be dereferenced (step 3 choice).
        definitions[leader] = {'flags_174': 1, 'regeneration_2cc': f32(0.05), 'health_284': f32(100)}
    primary = None
    if leader >= 0 and members[leader] is not None and members[leader]['body'] and rng.random() < 0.7:
        primary = leader
    return {
        'leader_slot': leader, 'members': members, 'definitions': definitions, 'primary': primary,
        'shortage': rng.choice((f32(0), f32(-0.5), f32(rng.uniform(0, 1)))),
        'modifier_3c': rng.choice((None, f32(rng.uniform(0.5, 2)))),
        'progress': [f32(rng.uniform(0, 300)) for _ in range(count)],
        'delta': f32(rng.uniform(0.01, 3)),
        'spawn_result': rng.choice((0, 1, 1)),
        'ratio': f32(rng.uniform(0, 0.6)),
    }


def run(m, c, entry):
    m.events = []
    count = len(c['members'])
    owner = m.alloc(0x100)
    m.owner = owner
    alleg = m.alloc(0x200)
    m.w(alleg + 0x1F0, c['shortage'])
    m.w(owner, m.vtable({0x108: lambda mm: mm.ret(0, alleg)}))
    if c['modifier_3c'] is not None:
        comp = m.alloc(0x100)
        m.w(comp + 0x1C + 0x3C, c['modifier_3c'])
        m.w(owner + 0x78, comp)
    org = m.alloc(0x100)
    orgdef = m.alloc(0x40)
    types = m.alloc(4 * count)
    slotdefs = m.alloc(4 * count)
    members = m.alloc(4 * count)
    progress = m.alloc(4 * count)
    m.org_members = members
    m.w(org + 4, owner)
    m.w(org + 0x18, orgdef)
    m.w(orgdef + 0x18, c['leader_slot'] & 0xFFFFFFFF)
    m.w(orgdef + 0x28, types)
    m.w(org + 0x1C, slotdefs)
    m.w(org + 0x28, members)
    m.w(org + 0x2C, count)
    m.w(org + 0x50, progress)
    m.member_of_body = {}
    actors = []
    for i in range(count):
        m.w(types + 4 * i, count - 1 - i)
        d = c['definitions'][i]
        if d is not None:
            dp = m.alloc(0x300)
            m.w(dp + 0x174, d['flags_174'])
            m.w(dp + 0x2CC, d['regeneration_2cc'])
            m.w(dp + 0x284, d['health_284'])
            m.w(slotdefs + 4 * (count - 1 - i), dp)
        m.w(progress + 4 * i, c['progress'][i])
        mem = c['members'][i]
        actor = 0
        if mem is not None:
            actor = m.alloc(0x100)
            membership = m.alloc(0x40)
            m.wb(membership + 0x1C, int(mem['attached']))
            m.w(membership + 0x20, mem['regeneration'])
            m.w(actor + 0x80, membership)
            if mem['body']:
                body = m.alloc(0x40)
                m.w(body + 0x10, mem['health'])
                m.w(body + 0x14, mem['maximum'])
                m.w(actor + 0x60, body)
                m.member_of_body[body] = i
            m.w(members + 4 * i, actor)
        actors.append(actor)
    if c['primary'] is not None:
        m.w(org + 0x44, actors[c['primary']])
    m.spawn_result = c['spawn_result']
    m.spawn_member = None
    if entry == 'regroup' and c['leader_slot'] >= 0:
        new = m.alloc(0x100)
        body = m.alloc(0x40)
        m.w(new + 0x60, body)
        m.member_of_body[body] = 'spawned'
        m.spawn_member = new
    m.ratio_cell = m.alloc(4)
    m.w(m.ratio_cell, c['ratio'])
    m.w(0xB3D6B4, 0)
    if entry == 'regenerate':
        m.call(REGENERATE, ecx=org, args=(c['delta'],), pop=4)
        result = None
    else:
        result = m.call(REGROUP, ecx=org) & 0xFF
    return {'precision': m.precision, 'entry': entry, 'case': c, 'result': result,
            'progress_after': [m.r(progress + 4 * i) for i in range(count)], 'events': m.events}


def main():
    rng = random.Random(0x57B5A0)
    cases, visited = [], {}
    for precision in h.PRECISIONS:
        m = Oracle(precision)
        done = 0
        while done < 160:
            c = random_case(rng)
            entry = 'regenerate' if done % 4 else 'regroup'
            if entry == 'regroup' and (c['leader_slot'] < 0 or
                                       (c['spawn_result'] and False)):
                continue
            cases.append(run(m, c, entry))
            done += 1
        visited.update(m.visited)
    h.report('native-regeneration-fixtures.json',
             '0x57B5A0 organization regeneration and 0x57AEB0 regroup (spawn/heal/set-health/presentation '
             'seams recorded); all precisions.', cases, visited)
    kinds = {}
    for c in cases:
        for e in c['events']:
            k = next(iter(e))
            kinds[k] = kinds.get(k, 0) + 1
    print(kinds)


if __name__ == '__main__':
    main()
