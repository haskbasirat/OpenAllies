"""Original regiment move Think 0x5276B0 with its member dispatch.

Whole unchanged routines execute: Think 0x5276B0, path planning 0x51EA20,
move tries 0x526940, destination 0x51E9B0/0x51D840/0x51D7F0, arrival
0x52A1F0, member dispatch 0x525740 with 0x579A90, 0x57BF30, member path
sends 0x523130/0x523240/0x5233F0, slot path 0x52CBD0/0x445280, translated
path 0x52D230, normalize 0x409230, node helpers 0x52CA90/0x52D180/0x52D0F0/
0x52D270/0x5189F0/0x52CD90 and the member iterator 0x4364C0.

Seams (recorded with arguments): 0x529E60 Think base, 0x52A000 current-state
tests, 0x51FBC0 target-in-range, 0x52CAE0 path region test, 0x4CEDE0 leader
moving, 0x44DEA0 group handle, 0x76B9A0, 0x554350 leader query constructor,
0x53E1F0 Solve (writes the supplied path), 0x4D64F0, 0x4EEBE0, 0x57BEF0,
0x554AD0 placement, 0x4CA760/0x4D9B60/0x4CA2D0 member order, 0x85D840 RNG,
group helpers 0x4D1900/0x4DDEE0/0x4DE6C0, Mind/actor/member virtuals,
allocation and the path node pool, diagnostics.
"""
import copy, math, random, struct, sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
from harness import Machine, NullDereference, write_document, f2b, b2f, PRECISION  # noqa: E402

PATH_VTABLE = 0xa34738


class Think(Machine):
    def __init__(self, case):
        super().__init__(case['precision'])
        self.c = case
        self.responses = list(case['responses'])
        self.live = {}
        S = self.seams
        S[0x6f7810] = self.allocate
        S[0x6f7430] = self.free
        S[0x85c680] = self.node_alloc
        S[0x85c2b0] = self.node_free
        S[0x404f60] = lambda m: m.ret(0)
        S[0x529e60] = lambda m: (m.event('ThinkBase'), m.ret(0))
        S[0x52a000] = self.current
        S[0x51fbc0] = lambda m: (m.event('TargetInRange', self.ident(m.arg())), m.ret(4, self.flag('TargetInRange')))
        S[0x52cae0] = lambda m: (m.event('PathRegionOk', m.arg(0), m.arg(1)), m.ret(8, self.flag('PathRegionOk')))
        S[0x4cede0] = lambda m: (m.event('LeaderMoving'), m.ret(0, self.flag('LeaderMoving')))
        S[0x44dea0] = lambda m: (m.event('GroupHandleValid'), m.ret(0, self.flag('GroupHandleValid')))
        S[0x76b9a0] = lambda m: (m.event('Owner76b9a0'), m.ret(4, self.flag('Owner76b9a0')))
        S[0x554350] = self.prepare_query
        S[0x53e1f0] = self.solve
        S[0x4d64f0] = lambda m: (m.event('ActorState', m.arg(0), m.arg(1)), m.ret(8))
        S[0x4eebe0] = lambda m: (m.event('SetOrganizationFacing', *self.r32(m.arg(), 2)), m.ret(4))
        S[0x57bef0] = self.compute_slots
        S[0x554ad0] = self.place
        S[0x4ca760] = self.order_construct
        S[0x4d9b60] = lambda m: (m.event('IssueOrder', self.ident(m.arg())), m.ret(4))
        S[0x4ca2d0] = lambda m: m.ret(0)
        S[0x85d840] = self.random
        S[0x4ddee0] = lambda m: (m.event('GroupActive'), m.ret(4, self.flag('GroupActive')))
        S[0x4de6c0] = self.group_offset
        self.build()

    # ---- helpers ----
    def response(self, name):
        r = self.responses.pop(0)
        assert r['name'] == name, (r, name)
        return r

    def flag(self, name):
        return int(self.response(name)['value'])

    def allocate(self, m):
        n = m.arg()
        p = m.alloc(n)
        self.live[p] = n
        m.ret(0, p)

    def free(self, m):
        p = m.arg()
        assert p in self.live, hex(p)
        del self.live[p]
        m.ret(0)

    def node_alloc(self, m):
        p = m.alloc(0x28)
        self.live[p] = 0x28
        m.ret(0, p)

    def node_free(self, m):
        p = m.arg()
        assert p in self.live, hex(p)
        del self.live[p]
        m.ret(4)

    def stub(self, handler):
        a = self.stub_next
        self.stub_next += 16
        self.seams[a] = handler
        return a

    def register(self, handle, obj):
        slot = handle & 0xffff
        self.u.mem_write(self.registry + 4 + slot * 2, struct.pack('<H', handle >> 16))
        self.w32(self.registry + 0x20004 + slot * 4, obj)

    def ident(self, ptr):
        return self.ids.get(ptr, 0xffffffff if ptr else 0)

    def write_path(self, collection, nodes, k1, k2):
        self.w32(collection, PATH_VTABLE)
        ptrs = []
        for words in nodes:
            n = self.alloc(0x28)
            self.live[n] = 0x28
            self.w32(n, *words)
            ptrs.append(n)
        for i, n in enumerate(ptrs):
            self.w32(n + 0x20, ptrs[i - 1] if i else 0)
            self.w32(n + 0x24, ptrs[i + 1] if i + 1 < len(ptrs) else 0)
        self.w32(collection + 4, ptrs[0] if ptrs else 0, ptrs[-1] if ptrs else 0, k1 & 0xffffffff, k2 & 0xffffffff)

    def read_path(self, collection):
        nodes, n = [], self.r32(collection + 4)
        while n:
            nodes.append(self.r32(n, 8))
            n = self.r32(n + 0x24)
        k1, k2 = self.r32(collection + 0xc, 2)
        s = lambda v: v - (1 << 32) if v >> 31 else v
        return {'nodes': nodes, 'kind1_count': s(k1), 'kind2_count': s(k2),
                'dirty': bool(self.r8(collection + 0x14)), 'word18': self.r32(collection + 0x18)}

    # ---- seams ----
    def current(self, m):
        ecx = m.ecx()
        name = 'IsCurrent' if ecx == self.state else 'State220Current'
        assert ecx in (self.state, self.mind + 0x220), hex(ecx)
        m.event(name)
        m.ret(0, self.flag(name))

    def prepare_query(self, m):
        assert m.arg(0) == self.leader_ptr and m.arg(1) == 0
        if m.arg(0) == 0:
            # 0x554350 dereferences the actor immediately.
            raise NullDereference('0x554350 null leader')
        m.event('PrepareLeaderQuery')
        self.u.mem_write(m.ecx() + 0x20, b'\0' * 0x20)
        m.ret(8, m.ecx())

    def solve(self, m):
        path, fx, fy, tx, ty, query, mode = [m.arg(i) for i in range(7)]
        assert path == self.state + 0x34 and m.ecx() == self.solver
        m.event('Solve', fx, fy, tx, ty, self.ident(self.r32(query + 0x20)), self.r32(query + 0x28),
                self.r32(query + 0x38), mode)
        r = self.response('Solve')
        if r['value']:
            self.write_path(path, r['nodes'], r['kind1_count'], r['kind2_count'])
        m.ret(0x1c, int(r['value']))

    def compute_slots(self, m):
        m.event('ComputeSlots')
        r = self.response('ComputeSlots')
        for i, p in enumerate(r['slots']):
            self.w32(self.slots + 8 * i, *p)
        m.ret(0)

    def place(self, m):
        point, direction = m.arg(0), m.arg(1)
        m.event('PlaceNear', *self.r32(point, 2), *self.r32(direction, 2), m.arg(3), m.arg(4))
        self.w32(point, *self.response('PlaceNear')['point'])
        m.ret(0x14)

    def order_construct(self, m):
        m.event('MemberOrder', *[m.arg(i) for i in range(5)])
        m.ret(0x14, m.ecx())

    def random(self, m):
        v = self.response('Random')['value']
        m.event('Random', v)
        m.ret(0, v)

    def group_offset(self, m):
        m.event('GroupOffset')
        out = m.arg(0)
        self.w32(out, *self.response('GroupOffset')['offset'])
        m.ret(0xc, out)

    def member_path(self, name, index):
        def handler(m):
            path, speed, ax, ay = m.arg(0), m.arg(1), m.arg(2), m.arg(3)
            p = self.read_path(path)
            m.event(name, index, speed, ax, ay, *[w for n in p['nodes'] for w in n], p['kind1_count'] & 0xffffffff,
                    p['kind2_count'] & 0xffffffff)
            m.ret(16)
        return handler

    # ---- world ----
    def build(self):
        c = self.c
        self.ids = {}
        self.registry = self.alloc(0x40100)
        self.w32(0xb364c0, self.registry)
        world = self.alloc(0x300)
        gmap = self.alloc(0x40)
        self.solver = self.alloc(0x40)
        self.w32(0xb36534, world)
        self.w32(world + 0x30, gmap)
        self.w32(world + 0xe0, c['world_time'])
        self.w32(world + 0x238, self.solver)
        self.w32(gmap + 0xc, f2b(256.0), f2b(192.0))
        sync = self.alloc(0x40)
        self.w32(0xb3655c, sync)
        self.w32(0xb36560, sync)
        cache = self.alloc(0x400)
        self.w32(0xb39ab0, cache)
        self.w32(cache + 0x190, cache + 0x300)
        self.w32(cache + 0x300, c['group_speed_minimum'])
        self.w32(0xb3cf68, self.r32(0xb3cf68) | 1)
        self.w32(cache + 0x308, c['min_seconds'])
        self.w32(0xb3cf64, cache + 0x308)
        self.mind = self.alloc(0x400)
        self.actor = self.alloc(0x200)
        mvt = self.alloc(0x200)
        self.w32(self.mind, mvt)
        self.w32(self.mind + 4, self.actor)
        tvt = self.alloc(0x10)
        self.w32(self.mind + 8, tvt)
        self.w32(tvt + 4, self.stub(lambda m: (m.event('Schedule', m.arg(0), m.arg(1)), m.ret(8))))
        self.w32(mvt + 0x98, self.stub(lambda m: (m.event('MindV98'), m.ret(0, self.response('MindV98')['value']))))
        self.w32(mvt + 0xb8, self.stub(lambda m: (m.event('MindB8'), m.ret(0))))
        self.w32(mvt + 0x124, self.stub(lambda m: (m.event('MoveFailed'), m.ret(0))))
        self.w32(mvt + 0x140, self.stub(lambda m: (m.event('TargetLost', m.arg()), m.ret(4))))
        self.w32(mvt + 0x144, self.stub(lambda m: (m.event('Finished'), m.ret(0))))
        self.w32(mvt + 0xf4, self.stub(lambda m: (m.event('MindF4', self.ident(m.arg())), m.ret(4))))
        self.state = self.mind + 0x68
        self.w32(self.state, self.alloc(0x40))
        self.w32(self.state + 4, self.mind)
        s = c['state']
        self.w32(self.state + 0xc, 2, s['target_id'], *s['path_destination_bits'], *s['destination_bits'],
                 *s['auxiliary_bits'], s['speed_bits'], s['tries'] & 0xffffffff)
        self.write_path(self.state + 0x34, s['path']['nodes'], s['path']['kind1_count'], s['path']['kind2_count'])
        self.w8(self.state + 0x48, 0)
        self.w32(self.state + 0x4c, 0)
        self.w32(self.state + 0x50, s['formation_word'], s['start_time_bits'])
        a = c['actor']
        avt = self.alloc(0x200)
        self.w32(self.actor, avt)
        self.w32(avt + 0x108, self.stub(lambda m: (m.event('OwnerV108'), m.ret(0, self.response('OwnerV108')['value']))))
        self.w32(avt + 0x138, self.stub(lambda m: (m.event('ActorV138', m.arg()), m.ret(4, self.flag('ActorV138')))))
        self.w32(self.actor + 0x14, 0x10001)
        self.ids[self.actor] = 0x10001
        self.register(0x10001, self.actor)
        self.w32(self.actor + 0x20, *a['position'])
        self.w32(self.actor + 0x2c, *a['facing'])
        self.w32(self.actor + 0x100, a['property_flags'])
        self.w32(self.actor + 0x164, a['group_handle'])
        if a['group_valid']:
            self.register(a['group_handle'], self.alloc(0x40))
        org = self.alloc(0x200)
        self.w32(self.actor + 0x7c, org)
        self.w32(org + 4, self.actor)
        o = c['organization']
        self.slots = self.alloc(8 * 16)
        for i, p in enumerate(o['slots']):
            self.w32(self.slots + 8 * i, *p)
        self.w32(org + 0x78, self.slots)
        if o['formation'] is not None:
            f = self.alloc(0x20)
            fd = self.alloc(0x40)
            self.w8(fd + 0x18, o['formation'][0])
            self.w8(fd + 0x19, o['formation'][1])
            self.w32(f + 0xc, fd)
            self.w32(org + 0x74, f)
            self.formation_ptr = f
        else:
            self.formation_ptr = 0
        if s['formation_word'] == 1:
            self.w32(self.state + 0x50, self.formation_ptr)
        members = self.alloc(4 * 16)
        self.member_ptrs = []
        self.leader_ptr = 0
        for i, mem in enumerate(o['members']):
            if mem is None:
                self.member_ptrs.append(0)
                continue
            p = self.alloc(0x200)
            el = self.alloc(0x40)
            mind = self.alloc(0x40)
            vt = self.alloc(0x200)
            self.w32(mind, vt)
            self.w32(vt + 0x100, self.stub(self.member_path('MemberMovePath', i)))
            self.w32(vt + 0x10c, self.stub(self.member_path('MemberFormationPath', i)))
            self.w32(p + 0x70, mind)
            self.w32(p + 0x80, el)
            self.w32(p + 4, self.alloc(0x40))
            self.w32(p + 8, mem['flags'])
            self.w32(p + 0x14, 0x20010 + i)
            self.w32(p + 0x20, *mem['position'])
            self.w32(el + 0x10, self.actor if mem['in_org'] else self.alloc(0x10))
            self.w32(el + 0x18, mem['slot'])
            self.w8(el + 0x1c, mem['attached'])
            self.ids[p] = i
            self.register(0x20010 + i, p)
            self.w32(members + 4 * i, p)
            self.member_ptrs.append(p)
        self.w32(org + 0x28, members)
        self.w32(org + 0x2c, len(o['members']))
        if o['leader'] is not None:
            self.leader_ptr = self.member_ptrs[o['leader']]
            self.w32(org + 0x40, self.leader_ptr)
        pending = c['pending']
        nodes = []
        for gtid in pending:
            n = self.alloc(8)
            self.live[n] = 8
            self.w32(n, gtid)
            nodes.append(n)
        for i, n in enumerate(nodes):
            self.w32(n + 4, nodes[i + 1] if i + 1 < len(nodes) else 0)
        self.w32(self.mind + 0x2bc, nodes[0] if nodes else 0, nodes[-1] if nodes else 0)
        t = c['target']
        if t is not None:
            tp = self.alloc(0x200)
            tvt2 = self.alloc(0x200)
            self.w32(tp, tvt2)
            self.w32(tvt2 + 0x108, self.stub(lambda m: (m.event('TargetV108'), m.ret(0, 0x7777))))
            self.w32(tp + 0x14, t['id'])
            self.w32(tp + 0x20, *t['position'])
            self.w32(tp + 0xd0, t['word_d0'])
            self.ids[tp] = t['id']
            if t['valid']:
                self.register(t['id'], tp)
        if c['state']['target_id'] == 0x10001:
            pass

    def run(self):
        self.call(0x5276b0, self.state)
        assert self.fpu_top() == 0
        assert not self.responses, self.responses
        s = self.state
        t = self.r32(s + 0x30)
        return {'state': {'target_id': self.r32(s + 0x10), 'path_destination_bits': self.r32(s + 0x14, 2),
                          'destination_bits': self.r32(s + 0x1c, 2), 'speed_bits': self.r32(s + 0x2c),
                          'tries': t - (1 << 32) if t >> 31 else t, 'path': self.read_path(s + 0x34),
                          'formation_word_is_current': self.r32(s + 0x50) == self.formation_ptr,
                          'formation_word_zero': self.r32(s + 0x50) == 0},
                'pending_left': self.r32(self.mind + 0x2bc) != 0, 'events': self.events}


def fb(*v):
    return [f2b(x) for x in v]


def rand_nodes(rng, n, start):
    nodes = []
    x, y = start
    for _ in range(n):
        a = rng.uniform(-math.pi, math.pi)
        L = rng.uniform(1, 15)
        nx, ny = min(250.0, max(2.0, x + math.cos(a) * L)), min(185.0, max(2.0, y + math.sin(a) * L))
        nodes.append([f2b(nx), f2b(ny), f2b(math.cos(a)), f2b(math.sin(a)), rng.choice([0, 1, 1, 2]), 0, f2b(nx), f2b(ny)])
        x, y = nx, ny
    return nodes


def scenario(rng, i):
    pos = [rng.uniform(20, 200), rng.uniform(20, 150)]
    members = []
    n = rng.randint(1, 6)
    for k in range(n):
        if rng.random() < 0.15:
            members.append(None)
            continue
        members.append({'flags': rng.choice([0, 0, 1]), 'slot': k, 'attached': int(rng.random() < 0.75),
                        'in_org': rng.random() < 0.9,
                        'position': fb(pos[0] + rng.uniform(-6, 6), pos[1] + rng.uniform(-6, 6))})
    present = [k for k, mm in enumerate(members) if mm is not None]
    leader = rng.choice(present) if present and rng.random() < 0.9 else None
    if leader is not None:
        members[leader]['in_org'] = True
    path_n = rng.choice([0, 0, 1, 3, 5])
    dest = [rng.uniform(5, 250), rng.uniform(5, 185)]
    target_kind = rng.choice(['none', 'none', 'valid', 'stale', 'self'])
    target = None
    target_id = 0
    if target_kind in ('valid', 'stale'):
        target_id = 0x30007
        target = {'id': target_id, 'position': fb(rng.uniform(5, 250), rng.uniform(5, 185)),
                  'word_d0': rng.choice([0, 1]), 'valid': target_kind == 'valid'}
    elif target_kind == 'self':
        target_id = 0x10001
    formation = rng.choice([None, (1, 0), (0, 1), (0, 0), (1, 0)])
    return {
        'name': f'think_{i}', 'world_time': f2b(rng.choice([20.0, 40.0, 100.0])),
        'group_speed_minimum': f2b(1.0), 'min_seconds': f2b(10.0),
        'state': {'target_id': target_id, 'path_destination_bits': fb(*dest), 'destination_bits': fb(*dest),
                  'auxiliary_bits': fb(0.25, -0.5), 'speed_bits': f2b(rng.choice([0.5, 2.0, 3.5])),
                  'tries': rng.choice([0, 1, 2, 5]),
                  'path': {'nodes': rand_nodes(rng, path_n, pos), 'kind1_count': 1, 'kind2_count': 0},
                  'formation_word': rng.choice([0, 1]), 'start_time_bits': f2b(rng.choice([5.0, 25.0, 95.0]))},
        'actor': {'position': fb(*pos), 'facing': fb(0.6, 0.8), 'property_flags': rng.choice([0, 0x400000]),
                  'group_handle': 0x40009, 'group_valid': rng.random() < 0.3},
        'organization': {'members': members, 'leader': leader, 'formation': formation,
                         'slots': [fb(rng.uniform(-6, 6), rng.uniform(-6, 6)) for _ in range(8)]},
        'pending': [0x20010 + k for k in rng.sample(range(n), rng.randint(0, n))] + ([0x50005] if rng.random() < 0.2 else []),
        'target': target,
    }


def responses(rng, c):
    """A generous response stream; unused tails are trimmed after a dry run."""
    out = []
    names = ['IsCurrent', 'GroupActive', 'TargetInRange', 'ActorV138', 'PathRegionOk', 'LeaderMoving', 'GroupHandleValid',
             'MindV98', 'State220Current', 'OwnerV108', 'Owner76b9a0', 'Solve', 'ComputeSlots', 'PlaceNear', 'Random',
             'GroupOffset']
    return names


def respond(rng, name, c):
    if name == 'Solve':
        ok = rng.random() < 0.8
        n = rng.choice([0, 1, 2, 4]) if ok else 0
        nodes = rand_nodes(rng, n, [b2f(c['actor']['position'][0]), b2f(c['actor']['position'][1])])
        return {'name': name, 'value': ok, 'nodes': nodes, 'kind1_count': sum(1 for x in nodes if x[4] == 1),
                'kind2_count': sum(1 for x in nodes if x[4] == 2)}
    if name == 'ComputeSlots':
        return {'name': name, 'slots': [fb(rng.uniform(-6, 6), rng.uniform(-6, 6)) for _ in range(8)]}
    if name == 'PlaceNear':
        return {'name': name, 'point': fb(rng.uniform(5, 250), rng.uniform(5, 185))}
    if name == 'Random':
        return {'name': name, 'value': rng.getrandbits(32)}
    if name == 'GroupOffset':
        return {'name': name, 'offset': fb(rng.uniform(-5, 5), rng.uniform(-5, 5))}
    if name == 'MindV98':
        return {'name': name, 'value': rng.choice([0, 3, 0x1234])}
    if name == 'OwnerV108':
        return {'name': name, 'value': rng.choice([0, 0x6666])}
    return {'name': name, 'value': rng.random() < 0.5}


class Adaptive(Think):
    """Supplies responses on demand from a seeded generator (recorded)."""
    def __init__(self, case, rng):
        self.rng = rng
        self.used = []
        case = dict(case)
        case['responses'] = []
        super().__init__(case)

    def response(self, name):
        r = respond(self.rng, name, self.c)
        if name == 'OwnerV108' and self.used and self.used[-1]['name'] == 'OwnerV108':
            r = dict(self.used[-1])
        self.used.append(r)
        return r


def main():
    rng = random.Random(0x5276b0)
    templates = [scenario(rng, i) for i in range(220)]
    out = []
    for t in templates:
        # Record one adaptive native run (PC53), then replay its responses under every precision.
        seed = rng.getrandbits(32)
        probe = copy.deepcopy(t)
        probe['precision'] = 'Double53'
        probe['responses'] = []
        m = Adaptive(probe, random.Random(seed))
        try:
            m.run()
        except NullDereference:
            pass
        stream = m.used
        for precision in PRECISION:
            c = copy.deepcopy(t)
            c['precision'] = precision
            c['responses'] = copy.deepcopy(stream)
            mm = Think(c)
            try:
                c['output'] = mm.run()
            except NullDereference as error:
                c['output'] = {'null_dereference': str(error), 'events': mm.events}
            except AssertionError:
                # A precision-dependent branch consumed a different stream; record
                # the adaptive run for this precision instead.
                ma = Adaptive(dict(c), random.Random(seed))
                try:
                    out_a = ma.run()
                    c['responses'] = ma.used
                    c['output'] = out_a
                except NullDereference as error:
                    c['responses'] = ma.used
                    c['output'] = {'null_dereference': str(error), 'events': ma.events}
            out.append(c)
    write_document('native-company-think.json', __doc__, out)


if __name__ == '__main__':
    main()
