"""Original SAI goal actor assignment: goal evaluate 7A9150 and goal assign
7A7E30 (the two per-goal passes of 7A93B0) with every callee executing
unchanged: required actor 7A35D0/7A3740, pool gathering 7A7EB0, steal from
lower/higher goals 7A3E10/7A3F30, pool assignment 7A6C70, trade 7A50E0, swap
7A33E0, player pool release 7AFA40 with the mask test 7A1120 and __allshl
96D710, list removal 56A1F0, compaction 7A6480 with predicate 7A2FB0, node
release 473300, smart-pointer assignment 7A1030 and the profiler marks
41C0E0/41C110 (ids 0E..11 are no-ops).

Seams (recorded in order): goal virtuals +8 +C +14 +24 +28 +2C +30 +38 +44 +64
+68 +6C +80 (any other slot fails the run), actor virtuals +0 (destructor)
+C +20, affordability 7A3860, actor-adjusted priority 7A3B00, fiber yield
79DF70, free 6F7430 and the assertion handler 855D30 (reported as a fault).
The add/remove stubs perform the list part of the base 7A6930/7A6A90 (push a
new head node, unlink the first matching node, count +-1, owner +10) so the
native list walks run under mutation. Virtual values are binary64 numbers
returned in ST0 (k - bias) / div from a case-seeded hash; every actor starts
with 1000 world-held references plus one per list node holding it, and the
final counts are checked against the final list memberships.
"""
from pathlib import Path
import json, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import GAME, Machine, PRECISIONS, f32, header, instructions, original, sha

CATEGORIES = 17
NO_CATEGORY = 0x11
FUNCTIONS = ['7a9150', '7a7e30', '7a35d0', '7a3740', '7a7eb0', '7a6c70', '7a3e10', '7a3f30', '7a50e0',
             '7a33e0', '7afa40', '56a1f0', '7a6480']


def coverage(visited):
    from capstone import Cs, CS_ARCH_X86, CS_MODE_32
    sizes = {}
    for line in open(GAME / 'reconstruction/research/native-aa/functions.tsv', encoding='utf-8'):
        f = line.split('\t')
        sizes[f[0]] = f[3]
    md = Cs(CS_ARCH_X86, CS_MODE_32)
    out = {}
    for name in FUNCTIONS:
        lo = int(name, 16); hi = lo + int(sizes['%08x' % lo])
        total = [i.address for i in md.disasm(original.range(lo, hi - lo), lo)]
        out[name.upper()] = '%d/%d' % (len([a for a in total if a in visited]), len(total))
    return out


def mix(*xs):
    h = 0x811c9dc5
    for x in xs:
        h ^= x & 0xffffffff
        h = (h * 0x01000193) & 0xffffffff
        h ^= h >> 13
    return h


def f64b(v):
    return struct.unpack('<Q', struct.pack('<d', v))[0]


def value(c, kind, *ids):
    k = mix(c['seed'], kind, *ids) % 61
    return f64b((k - c['bias']) / c['div'])


def chance(c, kind, *ids):
    return mix(c['seed'], kind, *ids) % 100 < c['pct'][str(kind)]


class AssignMachine(Machine):
    def __init__(self, c):
        super().__init__(c['precision'])
        self.c = c
        self.asserted = False
        sai = self.alloc(0x100); self.w(0xb36528, sai); self.sai = sai
        self.seam(0x79df70, self.s_yield)
        self.seam(0x6f7430, lambda m: self.ret(0))
        self.seam(0x855d30, self.s_assert)
        self.seam(0x7a3860, self.s_affordable)
        self.seam(0x7a3b00, self.s_priority)
        econ = self.econ = self.alloc(0x200)
        self.w(econ + 0x1c4, self.array(c['economy'])); self.w(econ + 0x1c8, len(c['economy']))
        player = self.alloc(0x80); self.w(player + 8, econ)
        ctl = self.alloc(0x40); self.w(ctl + 4, player)
        avt = self.alloc(0x40)
        for slot in range(0, 0x40, 4):
            self.w(avt + slot, self.stub(self.unexpected('actor', slot)))
        self.w(avt + 0, self.stub(self.a_destroy)); self.w(avt + 0xc, self.stub(self.a_dead))
        self.w(avt + 0x20, self.stub(self.a_serve))
        self.actors = {}
        for aid, a in c['actors'].items():
            p = self.alloc(0x40); self.w(p, avt); self.w(p + 4, 1000)
            self.w(p + 0x18, a['mask'] & 0xffffffff); self.w(p + 0x1c, a['mask'] >> 32)
            self.actors[int(aid)] = p
        self.aids = {p: a for a, p in self.actors.items()}
        gvt = self.alloc(0x100)
        for slot in range(0, 0x100, 4):
            self.w(gvt + slot, self.stub(self.unexpected('goal', slot)))
        slots = {0x8: self.g_category, 0xc: self.g_satisfied, 0x14: self.g_expired, 0x24: self.g_swap,
                 0x28: self.g_value, 0x2c: self.g_held, 0x30: self.g_spare, 0x38: self.g_required,
                 0x44: self.g_critical, 0x64: self.g_add, 0x68: self.g_remove, 0x6c: self.g_excluded,
                 0x80: self.g_deselect}
        for slot, fn in slots.items():
            self.w(gvt + slot, self.stub(fn))
        self.goals = {}
        for gid, g in c['goals'].items():
            p = self.alloc(0x60); self.w(p, gvt); self.w(p + 4, ctl)
            self.w(p + 8, g['status']); self.w(p + 0x34, g['current']); self.w(p + 0x38, g['base'])
            self.goals[int(gid)] = p
        self.gids = {p: g for g, p in self.goals.items()}
        for gid, g in c['goals'].items():
            self.write_list(self.goals[int(gid)] + 0xc, g['actors'], g['count'])
        for aid, a in c['actors'].items():
            self.w(self.actors[int(aid)] + 0x10, self.goals[a['owner']] if a['owner'] else 0)
        self.pools = []
        for pl in c['pools']:
            if pl is None:
                self.pools.append(0)
            else:
                p = self.alloc(0x10); self.write_list(p, pl['actors'], pl['count']); self.pools.append(p)
        self.w(player + 0x54, self.array(self.pools))
        for aid in self.actors:
            self.w(self.actors[aid] + 4, 1000 + self.memberships(aid))
        self.running = self.alloc(0x10)
        self.w(self.running, self.array(c['economy'])); self.w(self.running + 4, len(c['economy']))
        self.order = self.alloc(0x10)
        self.w(self.order, self.array([self.goals[g] for g in c['order']])); self.w(self.order + 4, len(c['order']))

    # lists --------------------------------------------------------------
    def aptr(self, aid):
        return 0 if aid is None else self.actors[aid]

    def aid(self, p):
        return None if p == 0 else self.aids[p]

    def write_list(self, h, actors, count):
        head = tail = 0
        for a in reversed(actors):
            node = self.alloc(8); self.w(node, self.aptr(a)); self.w(node + 4, head); head = node
            if not tail:
                tail = node
        self.w(h, head); self.w(h + 4, tail); self.w(h + 8, count)

    def read_list(self, h):
        items, node = [], self.r(h)
        while node:
            items.append(self.aid(self.r(node))); node = self.r(node + 4)
        tail = self.r(h + 4)
        return {'actors': items, 'tail': [self.aid(self.r(tail))] if tail else [], 'count': self.r(h + 8) - (1 << 32) * (self.r(h + 8) >> 31)}

    def memberships(self, aid):
        heads = [p + 0xc for p in self.goals.values()] + [p for p in getattr(self, 'pools', []) if p]
        return sum(self.read_list(h)['actors'].count(aid) for h in heads)

    def retain(self, actor, delta):
        if actor:
            self.w(actor + 4, self.r(actor + 4) + delta)

    def push_front(self, h, actor):
        node = self.alloc(8); self.w(node, actor); self.w(node + 4, self.r(h)); self.w(h, node)
        if not self.r(h + 4):
            self.w(h + 4, node)

    def unlink_first(self, h, actor):
        prev, node = 0, self.r(h)
        while node:
            if self.r(node) == actor:
                nxt = self.r(node + 4)
                if prev:
                    self.w(prev + 4, nxt)
                else:
                    self.w(h, nxt)
                if self.r(h + 4) == node:
                    self.w(h + 4, prev)
                return True
            prev, node = node, self.r(node + 4)
        return False

    # seams --------------------------------------------------------------
    def gid(self):
        return self.gids[self.ecx()]

    def ev(self, name, **kw):
        self.events.append({'event': name, **kw})

    def res(self, p):
        if p == 0:
            return 'null'
        if p == self.running:
            return 'running'
        if p == self.econ + 0x1c4:
            return 'economy'
        raise AssertionError('unexpected resource argument %#x' % p)

    def unexpected(self, kind, slot):
        def fn(m):
            raise AssertionError('unexpected %s virtual +%x' % (kind, slot))
        return fn

    def g_category(self, m):
        g = self.gid(); r = self.c['category'][str(g)]
        self.ev('category', goal=g, result=r); self.ret(r)

    def g_satisfied(self, m):
        g = self.gid(); need = self.c['need'][str(g)]
        r = need is not None and self.read_list(self.goals[g] + 0xc)['count'] >= need
        self.ev('satisfied', goal=g, result=r); self.ret(int(r))

    def g_expired(self, m):
        g = self.gid(); r = g in self.c['expired']
        self.ev('expired', goal=g, result=r); self.ret(int(r))

    def g_swap(self, m):
        g = self.gid(); gained, lost = self.aid(self.arg(1)), self.aid(self.arg(2))
        assert self.arg(3) == 1
        r = value(self.c, 3, g, gained or 0, lost or 0)
        self.ev('swap_value', goal=g, gained=gained, lost=lost, result=r); self.ret_st0_f64(r, 12)

    def g_value(self, m):
        g = self.gid(); a = self.aid(self.arg(1)); assert self.arg(2) == 1
        r = value(self.c, 1, g, a or 0)
        self.ev('value', goal=g, actor=a, result=r); self.ret_st0_f64(r, 8)

    def g_held(self, m):
        g = self.gid(); a = self.aid(self.arg(1))
        r = value(self.c, 2, g, a or 0)
        self.ev('held_value', goal=g, actor=a, result=r); self.ret_st0_f64(r, 4)

    def g_spare(self, m):
        g = self.gid(); r = self.c['spare'][str(g)]
        self.ev('spare', goal=g, result=r); self.ret(r)

    def g_required(self, m):
        g = self.gid(); r = self.c['required'][str(g)]
        self.ev('required_actor', goal=g, result=r); self.ret(self.aptr(r))

    def g_critical(self, m):
        g = self.gid(); r = g in self.c['critical']
        self.ev('critical', goal=g, result=r); self.ret(int(r))

    def g_add(self, m):
        g = self.gid(); a = self.aid(self.arg(1))
        self.ev('add_actor', goal=g, actor=a, res=self.res(self.arg(2)), a=self.arg(3), b=self.arg(4))
        h = self.goals[g] + 0xc
        self.push_front(h, self.aptr(a)); self.w(h + 8, self.r(h + 8) + 1); self.retain(self.aptr(a), 1)
        if a is not None:
            self.w(self.actors[a] + 0x10, self.goals[g])
        self.ret(0, 16)

    def g_remove(self, m):
        g = self.gid(); a = self.aid(self.arg(1))
        self.ev('remove_actor', goal=g, actor=a, res=self.res(self.arg(2)), a=self.arg(3), b=self.arg(4), c=self.arg(5))
        h = self.goals[g] + 0xc
        if self.unlink_first(h, self.aptr(a)):
            self.w(h + 8, self.r(h + 8) - 1); self.retain(self.aptr(a), -1)
        if a is not None and self.r(self.actors[a] + 0x10) == self.goals[g]:
            self.w(self.actors[a] + 0x10, 0)
        self.ret(0, 20)

    def g_excluded(self, m):
        g = self.gid(); a = self.aid(self.arg(1))
        r = chance(self.c, 6, g, a or 0)
        self.ev('excluded', goal=g, actor=a, result=r); self.ret(int(r), 4)

    def g_deselect(self, m):
        g = self.gid()
        self.ev('deselect', goal=g)
        if self.r(self.goals[g] + 8) != 0:
            self.w(self.goals[g] + 8, 1)
        self.ret(0)

    def a_destroy(self, m):
        self.ev('actor_destroyed', actor=self.aid(self.ecx())); self.ret(0, 4)

    def a_dead(self, m):
        a = self.aid(self.ecx()); r = a in self.c['dead']
        self.ev('actor_dead', actor=a, result=r); self.ret(int(r))

    def a_serve(self, m):
        a = self.aid(self.ecx()); g = self.gids[self.arg(1)]; assert self.arg(2) == 1
        r = chance(self.c, 7, a, g)
        self.ev('actor_can_serve', actor=a, goal=g, result=r); self.ret(int(r), 8)

    def s_affordable(self, m):
        g = self.gid(); assert self.arg(1) == self.running
        r = g in self.c['affordable']
        self.ev('affordable', goal=g, relaxed=self.arg(2) & 0xff, result=r); self.ret(int(r), 8)

    def s_priority(self, m):
        g = self.gid(); assert self.arg(1) == 0 and self.arg(2) == 0
        r = self.c['priority'][str(g)]
        self.ev('priority_with_actors', goal=g, result=r); self.ret_st0_f64(r, 8)

    def s_yield(self, m):
        assert self.ecx() == self.sai and self.arg(1) == 0
        self.ev('yield'); self.ret(0, 4)

    def s_assert(self, m):
        self.asserted = True
        self.u.emu_stop()

    def state(self):
        goals = {}
        for g, p in sorted(self.goals.items()):
            goals[str(g)] = {'status': self.r(p + 8), 'current': self.r(p + 0x34), 'base': self.r(p + 0x38),
                             **self.read_list(p + 0xc)}
        pools = [None if p == 0 else self.read_list(p) for p in self.pools]
        owners = {str(a): self.gids.get(self.r(p + 0x10)) for a, p in sorted(self.actors.items())}
        # Native retain/release must leave every count at 1000 + list memberships.
        for a, p in self.actors.items():
            assert self.r(p + 4) == 1000 + self.memberships(a), (a, self.r(p + 4))
        return {'goals': goals, 'pools': pools, 'owners': owners}

    def run(self):
        from unicorn import UcError
        entry = {'evaluate': 0x7a9150, 'assign': 0x7a7e30}[self.c['entry']]
        goal = self.goals[self.c['order'][self.c['index']]]
        try:
            self.call(entry, ecx=goal, args=(self.running, self.order, self.c['index']), pop=12)
        except (AssertionError, UcError) as e:
            if self.asserted:
                return {'fault': 'assertion 855D30', 'events': self.events}
            if self.null_accesses or isinstance(e, UcError):
                return {'fault': 'null dereference', 'events': self.events}
            raise
        return {'events': self.events, **self.state()}


def make_case(rnd, name, entry, precision):
    n_goals = rnd.randrange(1, 8)
    n_actors = rnd.randrange(0, 13)
    used = rnd.sample(range(CATEGORIES), rnd.randrange(1, 4))
    goals, category, need, spare, required, priority = {}, {}, {}, {}, {}, {}
    gids = list(range(1, n_goals + 1))
    aids = list(range(1, n_actors + 1))
    actors = {}
    for a in aids:
        mask = 0
        for cat in used:
            if rnd.random() < 0.6:
                mask |= 1 << cat
        if rnd.random() < 0.15:
            mask |= 1 << rnd.randrange(CATEGORIES)
        actors[str(a)] = {'mask': mask, 'owner': None}
    for g in gids:
        status = rnd.choice([0, 1, 2, 2, 2, 2])
        goals[str(g)] = {'status': status,
                         'current': f32(rnd.choice([0, 3, 10, -2, rnd.uniform(-5, 40)])),
                         'base': f32(rnd.choice([0, 5, 12, -1, rnd.uniform(-5, 40)])),
                         'actors': [], 'count': 0}
        category[str(g)] = rnd.choice(used + [NO_CATEGORY] + used)
        need[str(g)] = rnd.choice([None, 1, 2, 3, 4, 6])
        spare[str(g)] = rnd.choice([0, 1, 1])
        required[str(g)] = None
        priority[str(g)] = f64b(rnd.choice([0.0, -1.5, 7.0, rnd.uniform(-10, 60), rnd.uniform(0, 60)]))
    for a in aids:
        if gids and rnd.random() < 0.45:
            g = rnd.choice(gids)
            actors[str(a)]['owner'] = g
            goals[str(g)]['actors'].append(a)
    for g in gids:
        if rnd.random() < 0.05:
            goals[str(g)]['actors'].insert(rnd.randrange(len(goals[str(g)]['actors']) + 1), None)
        rnd.shuffle(goals[str(g)]['actors'])
        goals[str(g)]['count'] = len(goals[str(g)]['actors'])
    for g in gids:
        if aids and rnd.random() < 0.25:
            required[str(g)] = rnd.choice(aids)
    pools = []
    for cat in range(CATEGORIES):
        members = [a for a in aids if actors[str(a)]['mask'] >> cat & 1 and (actors[str(a)]['owner'] is None or rnd.random() < 0.1)]
        rnd.shuffle(members)
        if rnd.random() < 0.04:
            members.insert(rnd.randrange(len(members) + 1), None)
        if cat not in used and rnd.random() < 0.5 and not members:
            pools.append(None)
        else:
            pools.append({'actors': members, 'count': len(members)})
    order = gids[:]
    rnd.shuffle(order)
    c = {'name': name, 'entry': entry, 'precision': precision, 'seed': rnd.getrandbits(32),
         'bias': rnd.choice([10, 20, 30, 45]), 'div': rnd.choice([1.0, 4.0, 3.0, 7.0]),
         'pct': {'6': rnd.choice([0, 15, 40]), '7': rnd.choice([30, 70, 100])},
         'goals': goals, 'actors': actors, 'pools': pools, 'order': order, 'index': rnd.randrange(len(order)),
         'category': category, 'need': need, 'spare': spare, 'required': required, 'priority': priority,
         'expired': [g for g in gids if rnd.random() < 0.08],
         'critical': [g for g in gids if rnd.random() < 0.2],
         'affordable': [g for g in gids if rnd.random() < 0.75],
         'dead': [a for a in aids if rnd.random() < 0.1],
         'economy': [f32(rnd.uniform(0, 300)) for _ in range(rnd.randrange(0, 4))]}
    return c


def cases():
    rnd = random.Random(0x7a7e30)
    precisions = list(PRECISIONS)
    rows = []
    for i in range(240):
        rows.append(make_case(rnd, 'evaluate%d' % i, 'evaluate', precisions[i % 3]))
    for i in range(300):
        rows.append(make_case(rnd, 'assign%d' % i, 'assign', precisions[i % 3]))
    # Swap-heavy trades: every goal selected with pool categories and actors.
    for i in range(60):
        c = make_case(rnd, 'trade%d' % i, 'assign', precisions[i % 3])
        for g in c['goals'].values():
            g['status'] = 2
            g['current'] = f32(rnd.uniform(1, 40))
        for g in c['category']:
            if c['category'][g] == NO_CATEGORY:
                c['category'][g] = next(i for i, p in enumerate(c['pools']) if p is not None)
        c['index'] = 0
        c['pct']['7'] = 100
        rows.append(c)
    # Null pool pointer for the goal's own category (7A7EB0 asserts, 7A6C70
    # dereferences it).
    for i in range(6):
        c = make_case(rnd, 'null_pool%d' % i, ['evaluate', 'assign'][i % 2], precisions[i % 3])
        g = str(c['order'][c['index']])
        c['required'][g] = None
        c['expired'] = []
        cat = rnd.randrange(CATEGORIES)
        c['category'][g] = cat
        c['pools'][cat] = None
        rows.append(c)
    return rows


def main():
    rows, visited, faults = [], {}, 0
    for c in cases():
        m = AssignMachine(c)
        out = m.run()
        m.verify_v14()
        visited.update(m.visited)
        faults += 'fault' in out
        rows.append({'input': c, 'output': out})
    covered = coverage(visited)
    doc = header(__file__, 'Unicorn execution of original 7A9150/7A7E30 and callees with seamed goal/actor virtuals')
    doc.update({'cases': rows, 'faults': faults, 'coverage_instructions': covered,
                'instructions': instructions(visited)})
    out = HERE / 'native-goal-assign.json'
    out.write_text(json.dumps(doc, indent=1) + '\n', encoding='utf-8')
    print(len(rows), 'cases', faults, 'faults', covered)


if __name__ == '__main__':
    main()
