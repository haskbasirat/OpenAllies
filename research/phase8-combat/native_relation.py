"""Native oracle for the actor relation tests 4D4FF0 (ability relation code,
organization minimum over members) and 4D4420 (is attacking the target or its
organization), with the real 4364C0 / 437940 member iterators executing.

Random acyclic worlds of elements, organizations and an organization of
organizations are built in memory: actor +70 Mind (virtual +4C returns its
target), +74 Performer, +7C OrganizationComponent (+28 slots, +2C count),
+80 membership (+10 organization actor). Recorded seams: actor virtuals
+B4 (targetable), +108 (kingdom word), +84(kingdom) and Performer 57E920.
Virtual returns carry garbage above AL where only AL is tested.

Usage: native_relation.py  (AA.exe is located by the shared harness)
"""
from pathlib import Path
import random
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import harness as h  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX  # noqa: E402

KINGDOMS = (1, 2, 3, 0x80000001)
TRUE_BYTES = (1, 0x201, 0xff, 0x7f01)
FALSE_BYTES = (0, 0x100, 0xff00)
CODES = (-1, 0, 0, 1, 2, 3, 4, 5, 6, 6, 7)


def byte_value(rng, p_true):
    return rng.choice(TRUE_BYTES) if rng.random() < p_true else rng.choice(FALSE_BYTES)


def make_world(rng):
    n_leaf = rng.randint(1, 5)
    n_org = rng.randint(0, 3)
    n_super = rng.randint(0, 1) if n_org else 0
    leaves = list(range(n_leaf))
    orgs = list(range(n_leaf, n_leaf + n_org))
    supers = list(range(n_leaf + n_org, n_leaf + n_org + n_super))
    total = n_leaf + n_org + n_super
    actors = []
    for i in range(total):
        kind = 'leaf' if i in leaves else 'org' if i in orgs else 'super'
        a = {
            'targetable': byte_value(rng, 0.85),
            'kingdom': rng.choice(KINGDOMS),
            'accepts': {str(k): byte_value(rng, 0.8) for k in KINGDOMS},
            'performer': rng.random() < (0.85 if kind == 'leaf' else 0.3),
            'mind': None,
            'membership': None,
            'organization': None,
        }
        if kind != 'leaf':
            pool = leaves if kind == 'org' else orgs
            slots = [rng.choice(pool) if rng.random() < 0.8 else None for _ in range(rng.randint(0, 4))]
            count = len(slots)
            roll = rng.random()
            if roll < 0.1 and slots:
                count = len(slots) - 1
            elif roll < 0.15:
                count = 0
            elif roll < 0.2:
                count = -1
            a['organization'] = {'slots': slots, 'count': count}
        if rng.random() < (0.6 if kind == 'leaf' else 0.2):
            roll = rng.random()
            if roll < 0.7 and orgs:
                owner = rng.choice(orgs + supers)
            elif roll < 0.85:
                owner = rng.randrange(total)
            else:
                owner = None
            a['membership'] = {'organization': owner, 'attached': rng.randint(0, 1)}
        actors.append(a)
    for a in actors:
        if rng.random() < 0.75:
            a['mind'] = {'target': rng.randrange(total) if rng.random() < 0.85 else None}
    perform = [[rng.choice(CODES) for _ in range(total)] for _ in range(total)]
    return {'actors': actors, 'perform': perform}


class Built:
    def __init__(self, m, world):
        self.m = m
        self.world = world
        self.events = []
        n = len(world['actors'])
        self.ptr = [m.alloc(0x100) for _ in range(n)]
        self.index = {p: i for i, p in enumerate(self.ptr)}
        self.performer_owner = {}
        self.mind_owner = {}
        actor_vt = m.vtable({0xB4: self.targetable, 0x108: self.kingdom, 0x84: self.accepts})
        mind_vt = m.vtable({0x4C: self.mind_target})
        for i, a in enumerate(world['actors']):
            p = self.ptr[i]
            m.w(p, actor_vt)
            if a['performer']:
                perf = m.alloc(0x20)
                self.performer_owner[perf] = i
                m.w(p + 0x74, perf)
            if a['mind'] is not None:
                mind = m.alloc(0x20)
                m.w(mind, mind_vt)
                self.mind_owner[mind] = i
                m.w(p + 0x70, mind)
            if a['membership'] is not None:
                ms = m.alloc(0x20)
                owner = a['membership']['organization']
                m.w(ms + 0x10, self.ptr[owner] if owner is not None else 0)
                m.wb(ms + 0x1C, a['membership']['attached'])
                m.w(p + 0x80, ms)
            if a['organization'] is not None:
                comp = m.alloc(0x40)
                slots = a['organization']['slots']
                arr = m.alloc(4 * max(1, len(slots)))
                for k, s in enumerate(slots):
                    m.w(arr + 4 * k, self.ptr[s] if s is not None else 0)
                m.w(comp + 0x28, arr)
                m.w(comp + 0x2C, a['organization']['count'] & 0xFFFFFFFF)
                m.w(p + 0x7C, comp)
        m.hooks[0x57E920] = self.perform

    def this(self):
        return self.index[self.m.reg(UC_X86_REG_ECX)]

    def targetable(self, m):
        i = self.this()
        self.events.append(['targetable', i])
        m.ret(0, self.world['actors'][i]['targetable'])

    def kingdom(self, m):
        i = self.this()
        self.events.append(['kingdom', i])
        m.ret(0, self.world['actors'][i]['kingdom'])

    def accepts(self, m):
        i = self.this()
        k = m.arg(0)
        self.events.append(['accepts', i, k])
        m.ret(4, self.world['actors'][i]['accepts'][str(k)])

    def mind_target(self, m):
        i = self.mind_owner[m.reg(UC_X86_REG_ECX)]
        self.events.append(['mind_target', i])
        t = self.world['actors'][i]['mind']['target']
        m.ret(0, self.ptr[t] if t is not None else 0)

    def perform(self, m):
        owner = self.performer_owner[m.reg(UC_X86_REG_ECX)]
        target = self.index[m.arg(0)]
        a, b = m.arg(1), m.arg(2)
        self.events.append(['perform', owner, target, a, b])
        m.ret(12, self.world['perform'][owner][target] & 0xFFFFFFFF)


def signed(v):
    return v - (1 << 32) if v & 0x80000000 else v


def main():
    rng = random.Random(0x4D4FF0)
    m = h.Machine('Double53')
    worlds, cases = [], []
    for w in range(90):
        world = make_world(rng)
        worlds.append(world)
        m.u.mem_write(m.HEAP, bytes(m.cursor - m.HEAP))
        m.cursor = m.HEAP
        m.next_virtual = m.VIRTUALS
        m.hooks = {}
        built = Built(m, world)
        n = len(world['actors'])
        for _ in range(5):
            actor = rng.randrange(n)
            target = rng.randrange(n) if rng.random() < 0.92 else None
            a, b = rng.choice((1, 3)), rng.choice((0, 1))
            built.events = []
            r = m.call(0x4D4FF0, ecx=built.ptr[actor],
                       args=(built.ptr[target] if target is not None else 0, a, b), pop=12)
            cases.append({'world': w, 'kind': 'relation', 'actor': actor, 'target': target, 'a': a, 'b': b,
                          'result': signed(r), 'events': built.events})
        for _ in range(5):
            actor, target = rng.randrange(n), rng.randrange(n)
            built.events = []
            r = m.call(0x4D4420, ecx=built.ptr[actor], args=(built.ptr[target],), pop=4)
            cases.append({'world': w, 'kind': 'attacking', 'actor': actor, 'target': target,
                          'result': r & 0xFF, 'events': built.events})
    h.report('native-relation-fixtures.json',
             '4D4FF0 relation code and 4D4420 is-attacking over random organizations (4364C0/437940 real).',
             cases, m.visited, extra={'worlds': worlds})


if __name__ == '__main__':
    main()
