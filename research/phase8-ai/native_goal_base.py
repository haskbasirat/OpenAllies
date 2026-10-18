"""Original SAI base goal virtuals executing unchanged: 7A6930 add actor,
7A6A90 remove actor, 7A67B0 re-evaluate (with 7A6480 + predicate 7A10E0),
7A3B00 priority with a hypothetical actor change, 7A4FB0 swap value, 7A5020
value, 7A5080 held value, 7A5600 deselect -> 7A3D30 release -> 7AFB00 pool
return, 7ACF50/7A0F80 owner update, 4B9E60 node insert, 56A1F0 unlink, and
SAIData 4A7130 with the CRT _CIpow 96D890 taking its SSE2 body (CRT SSE2 flag
[B4B340] = 1, MXCSR 0x1F80), which the emulator computes exactly.

Seams (recorded in order): goal virtuals +4 +8 +20 +28 +2C +30 +38 +68 +7C
+80 (any other slot fails), actor virtuals +0 +4 +C +20, 7A10F0 (returns a
per-actor game object), 79F390 (unit check on that object), 7A1C10 (actor
travel time), 4A7CD0 (modifier match: case-scripted f32 sum/product
updates), 7A3860 affordability, 7A5200 finalize (sets +38 to a scripted
value), allocator 6F7810/6F7430, assertion 855D30, the x87 pow body 96D8ED
and ___libm_error_support 977111 (both reported as faults).
"""
from pathlib import Path
import json, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import GAME, Machine, PRECISIONS, f32, header, instructions, original, sha
from unicorn import x86_const as X

CATEGORIES = 17
NO_CATEGORY = 0x11
ENTRIES = {'add': (0x7a6930, 16), 'remove': (0x7a6a90, 20), 'reevaluate': (0x7a67b0, 4),
           'priority': (0x7a3b00, 8), 'swap': (0x7a4fb0, 12), 'value': (0x7a5020, 8),
           'held': (0x7a5080, 4), 'deselect': (0x7a5600, 0), 'weight': (0x4a7130, 4)}
FUNCTIONS = ['7a6930', '7a6a90', '7a67b0', '7a3b00', '7a4fb0', '7a5020', '7a5080', '7a5600', '7a3d30',
             '7afb00', '4a7130', '7a0f80', '7a6480']


def mix(*xs):
    h = 0x811c9dc5
    for x in xs:
        h ^= x & 0xffffffff
        h = (h * 0x01000193) & 0xffffffff
        h ^= h >> 13
    return h


def f64b(v):
    return struct.unpack('<Q', struct.pack('<d', v))[0]


def fval(bits):
    return struct.unpack('<f', struct.pack('<I', bits))[0]


def value(c, kind, *ids):
    k = mix(c['seed'], kind, *ids) % 61
    return f64b((k - c['bias']) / c['div'])


def travel_value(c, *ids):
    k = mix(c['seed'], 4, *ids) % 61
    return f64b(k / c['tdiv'])


def chance(c, kind, *ids):
    return mix(c['seed'], kind, *ids) % 100 < c['pct'][str(kind)]


def modifier_terms(c, m):
    addend = f32(((mix(c['seed'], 9, m) % 21) - 10) / 4)
    factor = f32((mix(c['seed'], 10, m) % 9 + 6) / 8)
    return addend, factor


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


class BaseMachine(Machine):
    def __init__(self, c):
        super().__init__(c['precision'])
        self.c = c
        self.asserted = None
        self.u.reg_write(X.UC_X86_REG_MXCSR, 0x1f80)
        self.w(0xb4b340, 1)
        self.seam(0x6f7430, lambda m: self.ret(0))
        self.seam(0x6f7810, lambda m: self.ret(self.alloc(max(self.arg(1), 4))))
        self.seam(0x855d30, self.s_fault('assertion 855D30'))
        self.seam(0x96d8ed, self.s_fault('x87 pow body'))
        self.seam(0x977111, self.s_fault('libm error'))
        self.seam(0x7a10f0, self.s_object)
        self.seam(0x79f390, self.s_unit_check)
        self.seam(0x7a1c10, self.s_travel)
        self.seam(0x4a7cd0, self.s_modifier)
        self.seam(0x7a3860, self.s_affordable)
        self.seam(0x7a5200, self.s_finalize)
        econ = self.econ = self.alloc(0x200)
        self.w(econ + 0x1c4, self.array(c['economy'])); self.w(econ + 0x1c8, len(c['economy']))
        player = self.player = self.alloc(0x80); self.w(player + 8, econ)
        data = self.data = self.alloc(0x400)
        self.w(data + 0x36c, c['norm']); self.w(data + 0x370, c['exponent'])
        self.modifiers = {}
        for t, mods in c['modifiers'].items():
            if mods is None:
                continue
            objs = [self.alloc(0x20) for _ in mods]
            for i, o in enumerate(objs):
                self.w(o + 0x18, objs[i + 1] if i + 1 < len(objs) else 0)
                self.modifiers[o] = mods[i]
            hdr = self.alloc(8); self.w(hdr, objs[0] if objs else 0)
            self.w(data + 0x4c + int(t) * 24 + 0x14, hdr)
        self.w(player + 0x1c, data)
        ctl = self.alloc(0x40); self.w(ctl + 4, player)
        avt = self.alloc(0x40)
        for slot in range(0, 0x40, 4):
            self.w(avt + slot, self.stub(self.unexpected('actor', slot)))
        for slot, fn in {0: self.a_destroy, 4: self.a_flag4, 0xc: self.a_dead, 0x20: self.a_serve}.items():
            self.w(avt + slot, self.stub(fn))
        self.actors = {}
        for aid, a in c['actors'].items():
            p = self.alloc(0x40); self.w(p, avt)
            self.w(p + 0x14, a['travel_cache']); self.w(p + 0x18, a['mask'] & 0xffffffff)
            self.w(p + 0x1c, a['mask'] >> 32); self.w(p + 0x30, self.alloc(16) if a['route'] else 0)
            self.actors[int(aid)] = p
        self.aids = {p: a for a, p in self.actors.items()}
        self.objects = {0x4d000000 + 0x10 * a: a for a in self.actors}
        gvt = self.alloc(0x100)
        for slot in range(0, 0x100, 4):
            self.w(gvt + slot, self.stub(self.unexpected('goal', slot)))
        slots = {0x4: self.g_type, 0x8: self.g_category, 0x20: self.g_location, 0x28: self.g_value,
                 0x2c: self.g_held, 0x30: self.g_count, 0x38: self.g_required, 0x68: self.g_remove,
                 0x7c: self.g_hook, 0x80: self.g_deselect}
        for slot, fn in slots.items():
            self.w(gvt + slot, self.stub(fn))
        self.goals = {}
        for gid, g in c['goals'].items():
            p = self.alloc(0x60); self.w(p, gvt); self.w(p + 4, ctl)
            self.w(p + 8, g['status']); self.w(p + 0x30, g['travel']); self.w(p + 0x34, g['current'])
            self.w(p + 0x38, g['base'])
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
        self.reserved = self.alloc(0x10)
        self.write_list(self.reserved, c['reserved'], len(c['reserved']))
        self.w(player + 0x38, self.r(self.reserved))
        self.running = self.alloc(0x10)
        self.w(self.running, self.array(c['economy'])); self.w(self.running + 4, len(c['economy']))
        for aid in self.actors:
            self.w(self.actors[aid] + 4, 1000 + self.memberships(aid))

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

    def walk(self, node):
        items = []
        while node:
            items.append(self.aid(self.r(node))); node = self.r(node + 4)
        return items

    def read_list(self, h):
        tail = self.r(h + 4)
        count = self.r(h + 8)
        return {'actors': self.walk(self.r(h)), 'tail': [self.aid(self.r(tail))] if tail else [],
                'count': count - (1 << 32) * (count >> 31)}

    def memberships(self, aid):
        heads = [self.r(p + 0xc) for p in self.goals.values()] + [self.r(p) for p in self.pools if p]
        heads.append(self.r(self.player + 0x38))
        return sum(self.walk(h).count(aid) for h in heads)

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

    def s_fault(self, name):
        def fn(m):
            self.asserted = name
            self.u.emu_stop()
        return fn

    def g_type(self, m):
        g = self.gid(); r = self.c['type'][str(g)]
        self.ev('goal_type', goal=g, result=r); self.ret(r)

    def g_category(self, m):
        g = self.gid(); r = self.c['category'][str(g)]
        self.ev('category', goal=g, result=r); self.ret(r)

    def g_location(self, m):
        g = self.gid(); r = 0x4c000000 + 0x10 * g
        self.ev('location', goal=g); self.ret(r)

    def g_value(self, m):
        g = self.gid(); a = self.aid(self.arg(1)); flag = self.arg(2) & 0xff
        r = value(self.c, 1, g, a or 0, flag)
        self.ev('value', goal=g, actor=a, check=flag, result=r); self.ret_st0_f64(r, 8)

    def g_held(self, m):
        g = self.gid(); a = self.aid(self.arg(1))
        r = value(self.c, 2, g, a or 0)
        self.ev('held_value', goal=g, actor=a, result=r); self.ret_st0_f64(r, 4)

    def g_count(self, m):
        g = self.gid(); r = self.read_list(self.goals[g] + 0xc)['count']
        self.ev('count', goal=g, result=r); self.ret(r)

    def g_required(self, m):
        g = self.gid(); r = self.c['required'][str(g)]
        self.ev('required_actor', goal=g, result=r); self.ret(self.aptr(r))

    def g_remove(self, m):
        g = self.gid(); a = self.aid(self.arg(1))
        self.ev('remove_actor', goal=g, actor=a, res=self.res(self.arg(2)), refresh=self.arg(3),
                keep=self.arg(4), extra=self.arg(5))
        h = self.goals[g] + 0xc
        if self.unlink_first(h, self.aptr(a)):
            self.w(h + 8, self.r(h + 8) - 1)
            if a is not None:
                self.w(self.actors[a] + 4, self.r(self.actors[a] + 4) - 1)
        if a is not None and self.r(self.actors[a] + 0x10) == self.goals[g]:
            self.w(self.actors[a] + 0x10, 0)
        self.ret(0, 20)

    def g_hook(self, m):
        g = self.gid(); add, remove = self.aid(self.arg(1)), self.aid(self.arg(2))
        r = value(self.c, 3, g, add or 0, remove or 0)
        self.ev('priority_hook', goal=g, add=add, remove=remove, result=r); self.ret_st0_f64(r, 8)

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

    def a_flag4(self, m):
        a = self.aid(self.ecx()); r = chance(self.c, 5, a)
        self.ev('actor_flag4', actor=a, result=r); self.ret(int(r))

    def a_serve(self, m):
        a = self.aid(self.ecx()); g = self.gids[self.arg(1)]; flag = self.arg(2) & 0xff
        r = chance(self.c, 7, a, g, flag)
        self.ev('actor_can_serve', actor=a, goal=g, flag=flag, result=r); self.ret(int(r), 8)

    def s_object(self, m):
        a = self.aid(self.ecx()); assert self.arg(1) == 0
        self.ret(0x4d000000 + 0x10 * a, 4)

    def s_unit_check(self, m):
        a = self.objects[self.arg(1)]; r = chance(self.c, 6, a)
        self.ev('unit_check', actor=a, result=r); self.ret(int(r))

    def s_travel(self, m):
        a = self.aid(self.ecx()); g = (self.arg(1) - 0x4c000000) // 0x10
        assert self.arg(1) == 0x4c000000 + 0x10 * g
        r = travel_value(self.c, a, g)
        self.ev('travel_time', actor=a, goal=g, result=r); self.ret_st0_f64(r, 4)

    def s_modifier(self, m):
        mid = self.modifiers[self.ecx()]; a = self.objects[self.arg(1)]
        ps, pp, pf = self.arg(2), self.arg(3), self.arg(4)
        flag_in = self.rb(pf)
        r = chance(self.c, 8, mid, a)
        if r:
            addend, factor = modifier_terms(self.c, mid)
            self.w(ps, f32(fval(self.r(ps)) + fval(addend)))
            if not self.rb(pf):
                self.wb(pf, 1)
                self.w(pp, f32(fval(self.r(pp)) * fval(factor)))
        self.ev('apply_modifier', modifier=mid, actor=a, matched_in=flag_in, result=r,
                sum=self.r(ps), product=self.r(pp))
        self.ret(0, 16)

    def s_affordable(self, m):
        g = self.gid(); r = g in self.c['affordable']
        self.ev('affordable', goal=g, res=self.res(self.arg(1)), relaxed=self.arg(2) & 0xff, result=r)
        self.ret(int(r), 8)

    def s_finalize(self, m):
        g = self.gid()
        self.ev('finalize', goal=g, res=self.res(self.arg(1)), draw=self.arg(2) & 0xff)
        self.w(self.goals[g] + 0x38, self.c['finalized'][str(g)])
        self.ret(0, 8)

    def state(self):
        goals = {}
        for g, p in sorted(self.goals.items()):
            goals[str(g)] = {'status': self.r(p + 8), 'current': self.r(p + 0x34), 'base': self.r(p + 0x38),
                             'travel': self.r(p + 0x30), **self.read_list(p + 0xc)}
        pools = [None if p == 0 else self.read_list(p) for p in self.pools]
        actors = {str(a): {'owner': self.gids.get(self.r(p + 0x10)), 'travel_cache': self.r(p + 0x14),
                           'route': self.r(p + 0x30) != 0} for a, p in sorted(self.actors.items())}
        for a, p in self.actors.items():
            assert self.r(p + 4) == 1000 + self.memberships(a), (a, self.r(p + 4), self.memberships(a))
        return {'goals': goals, 'pools': pools, 'actors': actors}

    def run(self):
        from unicorn import UcError
        c = self.c
        entry, pop = ENTRIES[c['entry']]
        goal = self.goals[c['goal']]
        a = c['args']
        args = {'add': lambda: (self.aptr(a[0]), self.resptr(a[1]), a[2], a[3]),
                'remove': lambda: (self.aptr(a[0]), self.resptr(a[1]), a[2], a[3], a[4]),
                'reevaluate': lambda: (self.resptr(a[0]),),
                'priority': lambda: (self.aptr(a[0]), self.aptr(a[1])),
                'swap': lambda: (self.aptr(a[0]), self.aptr(a[1]), a[2]),
                'value': lambda: (self.aptr(a[0]), a[1]),
                'held': lambda: (self.aptr(a[0]),),
                'deselect': lambda: (),
                'weight': lambda: (a[0],)}[c['entry']]()
        ecx = self.data if c['entry'] == 'weight' else goal
        try:
            self.call(entry, ecx=ecx, args=args, pop=pop)
        except (AssertionError, UcError) as e:
            if self.asserted:
                return {'fault': self.asserted, 'events': self.events}
            if self.null_accesses or isinstance(e, UcError):
                return {'fault': 'null dereference', 'events': self.events}
            raise
        out = {'events': self.events, **self.state()}
        if c['entry'] in ('priority', 'swap', 'value', 'held', 'weight'):
            out['result'] = self.st0()
        return out

    def resptr(self, name):
        return {'null': 0, 'running': self.running, 'economy': self.econ + 0x1c4}[name]


def make_case(rnd, name, entry, precision):
    n_goals = rnd.randrange(1, 4)
    n_actors = rnd.randrange(0, 9)
    used = rnd.sample(range(CATEGORIES), rnd.randrange(1, 3))
    gids = list(range(1, n_goals + 1))
    aids = list(range(1, n_actors + 1))
    actors = {}
    for a in aids:
        mask = 0
        for cat in used:
            if rnd.random() < 0.6:
                mask |= 1 << cat
        actors[str(a)] = {'mask': mask, 'owner': None, 'route': rnd.random() < 0.3,
                          'travel_cache': f32(rnd.choice([-1, 5, 12.5]))}
    goals = {}
    for g in gids:
        goals[str(g)] = {'status': rnd.choice([0, 1, 2, 2]),
                         'current': f32(rnd.choice([0, 3, 10, -2, rnd.uniform(-5, 40)])),
                         'base': f32(rnd.choice([0, 5, 12, -1, rnd.uniform(-5, 40)])),
                         'travel': f32(rnd.choice([0, 4, rnd.uniform(0, 60)])), 'actors': [], 'count': 0}
    for a in aids:
        if rnd.random() < 0.55:
            g = rnd.choice(gids)
            actors[str(a)]['owner'] = g
            goals[str(g)]['actors'].append(a)
    for g in gids:
        lst = goals[str(g)]['actors']
        rnd.shuffle(lst)
        if rnd.random() < 0.04:
            lst.insert(rnd.randrange(len(lst) + 1), None)
        goals[str(g)]['count'] = len(lst) + (rnd.choice([0, 0, 0, 0, 0, 0, 1, -1]) if lst else 0)
    pools = []
    for cat in range(CATEGORIES):
        members = [a for a in aids if actors[str(a)]['mask'] >> cat & 1 and actors[str(a)]['owner'] is None]
        rnd.shuffle(members)
        if cat not in used and not members and rnd.random() < 0.5:
            pools.append(None)
        else:
            pools.append({'actors': members, 'count': len(members)})
    if used and rnd.random() < 0.05:
        pools[rnd.choice(used)] = None
    types = {str(g): rnd.randrange(0, 23) for g in gids}
    modifiers = {}
    for t in set(types.values()):
        modifiers[str(t)] = rnd.choice([None, [], [rnd.randrange(1, 50) for _ in range(rnd.randrange(1, 4))]])
    target = rnd.choice(gids)
    in_list = [a for a in goals[str(target)]['actors'] if a is not None]
    others = [a for a in aids if a not in in_list]
    pick = lambda pool: rnd.choice(pool) if pool else None
    res = rnd.choice(['running', 'running', 'economy', 'null'])
    if entry == 'add':
        args = [pick(others) if rnd.random() < 0.85 else pick(aids), res, rnd.choice([0, 1, 1]), rnd.choice([0, 0, 1])]
    elif entry == 'remove':
        args = [pick(in_list) if rnd.random() < 0.85 else pick(aids), res, rnd.choice([0, 1, 1]),
                rnd.choice([0, 0, 1]), rnd.choice([0, 1])]
    elif entry == 'reevaluate':
        args = [rnd.choice(['running', 'economy'])]
    elif entry == 'priority':
        args = [pick(others) if rnd.random() < 0.6 else None, pick(in_list) if rnd.random() < 0.6 else None]
    elif entry == 'swap':
        args = [pick(aids) if rnd.random() < 0.7 else None, pick(in_list) if rnd.random() < 0.6 else None, rnd.choice([0, 1])]
    elif entry == 'value':
        args = [pick(aids), rnd.choice([0, 1])]
    elif entry == 'held':
        args = [pick(in_list) if rnd.random() < 0.8 else pick(aids)]
    elif entry == 'weight':
        args = [f32(rnd.choice([0, 1, 10, 50, 120, 400, 1000, rnd.uniform(0, 600), -20]))]
    else:
        args = []
    if entry in ('add', 'remove') and args[0] is None and aids and rnd.random() < 0.9:
        args[0] = rnd.choice(aids)
    if entry in ('value', 'held') and args[0] is None and aids and rnd.random() < 0.9:
        args[0] = rnd.choice(aids)
    c = {'name': name, 'entry': entry, 'precision': precision, 'goal': target, 'args': args,
         'seed': rnd.getrandbits(32), 'bias': rnd.choice([10, 20, 30]), 'div': rnd.choice([1.0, 4.0, 3.0, 7.0]),
         'tdiv': rnd.choice([1.0, 3.0, 7.0, 0.25]),
         'pct': {'5': rnd.choice([0, 20, 50]), '6': rnd.choice([0, 30, 60]), '7': rnd.choice([30, 80, 100]),
                 '8': rnd.choice([0, 40, 80])},
         'goals': goals, 'actors': actors, 'pools': pools,
         'reserved': [a for a in aids if rnd.random() < 0.15],
         'type': types, 'category': {str(g): rnd.choice(used + [NO_CATEGORY]) for g in gids},
         'required': {str(g): (pick(aids) if rnd.random() < 0.2 else None) for g in gids},
         'modifiers': modifiers,
         'finalized': {str(g): f32(rnd.choice([0, -3, 8, rnd.uniform(-5, 50)])) for g in gids},
         'affordable': [g for g in gids if rnd.random() < 0.75],
         'dead': [a for a in aids if rnd.random() < 0.15],
         'norm': f32(rnd.choice([20, 60, 150, 400, rnd.uniform(10, 500)])),
         'exponent': f32(rnd.choice([1.0, 2.0, 0.5, 1.5, rnd.uniform(0.25, 3)])),
         'economy': [f32(rnd.uniform(0, 300)) for _ in range(rnd.randrange(0, 4))]}
    return c


def cases():
    rnd = random.Random(0x7a3b00)
    precisions = list(PRECISIONS)
    rows = []
    plan = [('add', 90), ('remove', 90), ('reevaluate', 60), ('priority', 120), ('swap', 60),
            ('value', 60), ('held', 50), ('deselect', 50), ('weight', 60)]
    for entry, n in plan:
        for i in range(n):
            rows.append(make_case(rnd, '%s%d' % (entry, i), entry, precisions[i % 3]))
    return rows


def main():
    rows, visited, faults = [], {}, 0
    for c in cases():
        m = BaseMachine(c)
        out = m.run()
        m.verify_v14()
        visited.update(m.visited)
        faults += 'fault' in out
        rows.append({'input': c, 'output': out})
    covered = coverage(visited)
    doc = header(__file__, 'Unicorn execution of original SAI base goal virtuals with seamed goal/actor virtuals')
    doc.update({'cases': rows, 'faults': faults, 'coverage_instructions': covered,
                'instructions': instructions(visited)})
    (HERE / 'native-goal-base.json').write_text(json.dumps(doc, indent=1) + '\n', encoding='utf-8')
    print(len(rows), 'cases', faults, 'faults', covered)


if __name__ == '__main__':
    main()
