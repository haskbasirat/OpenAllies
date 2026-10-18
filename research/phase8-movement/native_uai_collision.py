"""UAIStateMove collision avoidance 0x534A40 (ResultAvoidStatic) and the
moving-thing collision result 0x536AD0, both reaching 0x531520.

Whole unchanged routines execute under Unicorn: 0x534A40 with its option
helper 0x533140 and distance helper 0x52E210 (shape extents 0x46DAB0/
0x46DC50/0x46DCB0), closest point 0x46E4F0/0x46E150, normalize 0x409230,
Manhattan test 0x51D510, leader test 0x4CEFC0, handle 0x4CF1B0, path list
0x52CF40/0x52CA90/0x52D180/0x52D000, 0x529C30; 0x536AD0 with 0x46EB30/
0x46E0E0 (shape constructor), 0x4CF210/0x4EA0F0 (placed-shape descriptors);
tile descriptor 0x52F3E0. 0x531520 and the whole low-path solver run natively
inside them through native_low_path.LowMachine (imported, not modified).

Seams (recorded in order): 0x4CEDE0 moving state (the native body's first
read [ecx+0x7C] faults on a null actor), 0x531170 forward probe, 0x5312C0
explicit-origin probe, 0x46EF30 shape overlap, 0x46F100 shape range, 0x4D6940
ignored-thing append, actor virtuals +0/+4/+0xC/+0x10/+0x18/+0x20/+0x16C,
Mind virtuals +0x44/+0x68 (owner and +0x70 minds), ActionGo 0x5300A0, stop
0x52D660, owner timer (+8)+4, every 0x531520/low-path seam of LowMachine and
the allocator 0x6F7810/0x6F7430. 0x404F90 (diagnostic log) is only observed:
its body runs and returns because log level 0xB38B7C is 0 (BSS).
"""
import copy, hashlib, json, math, random, struct, sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
import harness  # noqa: E402
from harness import NullDereference, f2b, b2f, PRECISION, HERE  # noqa: E402
from native_low_path import LowMachine, base_case, make_actor, node  # noqa: E402

FLT_MAX = 0x7f7fffff


def unit_bits(x, y):
    length = math.hypot(x, y)
    if length == 0:
        return [0, 0]
    return [f2b(x / length), f2b(y / length)]


def box_shape(kind, w, l):
    """Native 6-word shape: kind, radius, minX, minY, maxX, maxY (0x46E0E0 layout)."""
    if kind in (2, 3):
        hx, hy = w / 2, l / 2
        return [kind, f2b(math.hypot(hx, hy)), f2b(-hx), f2b(-hy), f2b(hx), f2b(hy)]
    return [kind, f2b(w / 2), 0, 0, 0, 0]


class UaiMachine(LowMachine):
    def __init__(self, case):
        super().__init__(case)
        u = case['uai']
        for a in case['actors']:
            obj = a['object']
            self.w32(a['shape_ptr'], *a['shape6'])
            soft = self.alloc(0x20)
            self.w32(soft, *a['soft6'])
            a['soft_ptr'] = soft
            self.w32(obj + 8, a['flags'])
            self.w32(obj + 0x14, a['gtid'])
        for off, handler in [(4, self.v4), (0x10, self.vsoft), (0x18, self.v18), (0x20, self.v20),
                             (0x16c, self.v16c)]:
            self.w32(self.vtable + off, self.stub(handler))
        self.w32(self.gworld + 0xe8, u['step'])
        # Minds: shared vtable, +0x44 and +0x68 seams.
        self.mind_vt = self.alloc(0x100)
        self.w32(self.mind_vt + 0x44, self.stub(self.v44))
        self.w32(self.mind_vt + 0x68, self.stub(self.v68))
        self.minds = {}
        for key, mind in u['minds'].items():
            if key == 'owner':
                continue  # the UAI owner object is built by build_state
            obj = self.alloc(0x80)
            self.w32(obj, self.mind_vt)
            mind['object'] = obj
            self.minds[obj] = mind
        mind_obj = lambda key: 0 if key is None else u['minds'][key]['object']  # noqa: E731
        # Organization: actor+0x80 -> component, +0x10 -> thing; thing +0x70 mind,
        # +0x7C organization component, whose +0x40 is the leader actor.
        for a in case['actors']:
            a['mind_object'] = mind_obj(a.get('mind'))
            self.w32(a['object'] + 0x70, a['mind_object'])
            org = a.get('org')
            if org is None:
                continue
            membership = self.alloc(0x40)
            self.w32(a['object'] + 0x80, membership)
            if org.get('thing') is None:
                continue
            t = org['thing']
            thing = self.alloc(0x100)
            t['object'] = thing
            self.w32(membership + 0x10, thing)
            self.w32(thing + 0x70, mind_obj(t.get('mind')))
            if t.get('component', True):
                component = self.alloc(0x80)
                t['component_object'] = component
                leader = t.get('leader')
                if leader == 'self':
                    leader_obj = a['object']
                elif leader is None:
                    leader_obj = 0
                else:
                    leader_obj = case['actors'][leader]['object']
                self.w32(component + 0x40, leader_obj)
                self.w32(thing + 0x7c, component)
            else:
                self.w32(thing + 0x7c, 0)
        self.fprobes = list(u['forward'])
        self.oprobes = list(u['origin'])
        self.overlaps = list(u['overlap'])
        self.ranges = list(u['in_range'])
        s = self.seams
        s[0x4cede0] = self.moving
        s[0x531170] = self.fprobe
        s[0x5312c0] = self.oprobe
        s[0x46ef30] = self.overlap
        s[0x46f100] = self.in_range
        s[0x4d6940] = self.ignore
        s[0x404f90] = self.log

    # ---- seams ---------------------------------------------------------
    def actor(self, obj):
        return self.actor_info[obj]

    def v4(self, m):
        a = self.actor(m.ecx())
        m.note('v4', m.ecx(), a['v4'])
        m.ret_float(a['v4'], 'f32')

    def vshape(self, m):
        a = self.actor(m.ecx())
        if a.get('shape_null'):
            m.note('shape', m.ecx())
            m.ret(0, 0)
            return
        m.note('shape', m.ecx(), *a['shape6'])
        m.ret(0, a['shape_ptr'])

    def vsoft(self, m):
        a = self.actor(m.ecx())
        m.note('soft', m.ecx(), *a['soft6'])
        m.ret(0, a['soft_ptr'])

    def v18(self, m):
        a = self.actor(m.ecx())
        m.note('v18', m.ecx(), a['v18'])
        m.ret_float(a['v18'], 'f32')

    def v20(self, m):
        a = self.actor(m.ecx())
        m.note('v20', m.ecx(), m.arg(0), int(a['v20']))
        m.ret(4, int(a['v20']))

    def v16c(self, m):
        a = self.actor(m.ecx())
        other = m.arg(0)
        result = int(other in a['v16c'])
        m.note('v16c', m.ecx(), other, result)
        m.ret(4, result)

    def v44(self, m):
        mind = self.minds[m.ecx()]
        kind = mind['v44']
        if isinstance(kind, list):  # successive results of repeated calls
            kind = kind[mind.setdefault('v44_calls', 0)]
            mind['v44_calls'] += 1
        if kind == 'null':
            m.note('v44', m.ecx(), 0, 0, 0)
            m.ret(0, 0)
            return
        result = m.alloc(0x10)
        if kind == 'point':
            point = m.alloc(8)
            m.w32(point, *mind['point'])
            m.w32(result + 4, point)
            m.note('v44', m.ecx(), 2, *mind['point'])
        else:
            m.note('v44', m.ecx(), 1, 0, 0)
        m.ret(0, result)

    def v68(self, m):
        mind = self.minds[m.ecx()]
        m.note('v68', m.ecx(), int(mind['v68']))
        m.ret(0, int(mind['v68']))

    def moving(self, m):
        if m.ecx() == 0:
            raise NullDereference('0x7c')
        a = self.actor(m.ecx())
        m.note('moving', m.ecx(), int(a['moving']))
        m.ret(0, int(a['moving']))

    def fprobe(self, m):
        assert m.ecx() == self.state
        dx, dy, dist, out = m.arg(0), m.arg(1), m.arg(2), m.arg(3)
        assert m.r32(out, 4) == [0, 0, 0, 0]
        rule = self.c['uai'].get('forward_rule')
        if rule == 'clear_doubled':
            # Hit everything except the fallback probe at 2 * (step * speed).
            u = self.c['uai']
            doubled = f2b(2.0 * b2f(f2b(b2f(u['step']) * b2f(u['speed']))))
            result = [0, 0, 0, 0] if dist == doubled else [1, 0, 0, 0]
        else:
            result = self.fprobes.pop(0)
        m.w32(out, *result)
        m.note('fprobe', dx, dy, dist, *result)
        m.ret(0x10, int(result[0] != 0))

    def oprobe(self, m):
        assert m.ecx() == self.state
        ox, oy, dx, dy, dist, out = (m.arg(i) for i in range(6))
        assert m.r32(out, 4) == [0, 0, 0, 0]
        result = self.oprobes.pop(0)
        m.w32(out, *result)
        m.note('oprobe', ox, oy, dx, dy, dist, *result)
        m.ret(0x18, int(result[0] != 0))

    def descriptor(self, d):
        words = m_words = self.r32(d, 6)
        shape = self.r32(m_words[0], 6) if m_words[0] else [None] * 6
        return shape + words[1:]

    def overlap(self, m):
        a, b = self.descriptor(m.ecx()), self.descriptor(m.arg(0))
        result = int(self.overlaps.pop(0))
        m.note('overlap', *a, *b, result)
        m.ret(4, result)

    def in_range(self, m):
        a, distance, b, inverted = self.descriptor(m.ecx()), m.arg(0), self.descriptor(m.arg(1)), m.arg(2)
        result = int(self.ranges.pop(0))
        m.note('range', *a, distance, *b, inverted, result)
        m.ret(0xc, result)

    def ignore(self, m):
        m.note('ignore', m.ecx(), m.arg(0), m.arg(1))
        m.ret(8)

    def log(self, m):
        # Observation only: 0x404F90's body runs (level 0 -> no output).
        m.note('log', m.arg(2))

    # ---- entry ---------------------------------------------------------
    def build_state(self):
        c = self.c['continuation']
        u = self.c['uai']
        state = self.alloc(0x100)
        owner = self.alloc(0x100)
        owner_vt = self.alloc(0x200)
        timer_vt = self.alloc(0x20)
        actor = self.c['actors'][c['actor']]['object']
        self.w32(state + 4, owner)
        self.w32(owner, owner_vt)
        self.w32(owner + 4, actor)
        self.w32(owner + 8, timer_vt)
        self.w32(owner + 0x54, c['owner_word_54'])
        self.owner = owner
        self.minds[owner] = u['owner']
        self.w32(owner_vt + 0x15c, self.stub(lambda m: (m.note('failed', m.arg(0)), m.ret(4))))
        self.w32(owner_vt + 0x68, self.stub(self.v68))
        self.w32(timer_vt + 4, self.stub(lambda m: (m.note('schedule', m.arg(0), m.arg(1)), m.ret(8))))
        self.w32(state + 0x10, c['target_id'], *c['destination'])
        self.w32(state + 0x24, u['speed'])
        self.w32(state + 0x2c, u['phase'], u['mode'])
        self.w32(state + 0x4c, *u['cv'])
        self.w32(state + 0x54, c['waypoint_offset'])
        self.w32(state + 0x78, c['last_time'])
        self.build_path(state + 0x58, self.c['path'])
        self.context_words = list(self.c['context'])
        self.state = state

        def context(m):
            out, who, strict = m.ecx(), m.arg(0), m.arg(1)
            assert who == actor and strict == 0
            m.w32(out, *self.context_words)
            self.context = out
            m.note('context')
            m.ret(8, out)

        def replace(m):
            obj = m.ecx()
            result = c['replacement'].get(str(obj), obj)
            m.note('replace', obj, result)
            m.ret(0, result)

        def action_go(m):
            assert m.ecx() == state
            m.note('action_go', m.arg(0), self.r32(state + 0x54), self.read_path(state + 0x58))
            m.ret(4)

        self.seams[0x554350] = context
        self.seams[0x4ce6e0] = replace
        self.seams[0x52d660] = lambda m: (m.note('stop'), m.ret(0))
        self.seams[0x4cfc80] = lambda m: (m.note('anim', m.ecx()), m.ret(0))
        self.seams[0x5300a0] = action_go
        return state

    def uai_case(self):
        constructed = self.construct()
        u = self.c['uai']
        state = self.build_state()
        out = {'constructed': constructed, 'descriptor_in': None, 'descriptor_out': None, 'fault_eip': None}
        try:
            if u['entry'] == 'static':
                desc = self.alloc(0x18)
                d = u['descriptor']
                if d['source'] == 'tile':
                    # KKC_GMap::KKC_GMap 0x556570 builds the +0x30 tile shape with
                    # 0x46EB60(cell size); run it natively at the case precision.
                    self.w32(self.grid + 0x30, *([0xdeadbeef] * 6))
                    self.call(0x46eb60, self.grid + 0x30, (self.c['grid']['cell_size'],))
                    assert self.final_esp == harness.STACK + 8 and self.fpu_top() == 0
                    out['tile_shape'] = self.r32(self.grid + 0x30, 6)
                    self.call(0x52f3e0, self.grid, (desc, *d['tile']))
                    assert self.final_esp == harness.STACK + 0x10 and self.fpu_top() == 0
                else:
                    obj = self.c['actors'][d['actor']]['object']
                    self.call(0x4cf210, obj, (desc,))
                    assert self.final_esp == harness.STACK + 8 and self.fpu_top() == 0
                self.w32(desc + 0x14, d.get('alignment', 0))
                out['descriptor_in'] = self.r32(desc, 6)
                other = 0 if u['other'] is None else self.c['actors'][u['other']]['object']
                self.call(0x534a40, state, (desc, *u['cv'], other), count=50_000_000)
                assert self.final_esp == harness.STACK + 0x14
                out['descriptor_out'] = self.r32(desc, 6)
            else:
                target = self.c['actors'][u['other']]['object']
                self.call(0x536ad0, state, (target, u['distance']), count=50_000_000)
                assert self.final_esp == harness.STACK + 0xc
            assert self.fpu_top() == 0
            out['fault'] = None
        except NullDereference as error:
            out['fault'] = str(error)
            out['fault_eip'] = self.u.reg_read(harness.UC_X86_REG_EIP)
        if self.context is None:
            self.context = self.alloc(0x58)
        out.update(self.snapshot(state + 0x58))
        out['state'] = {'last_time': self.r32(state + 0x78), 'waypoint_offset': self.r32(state + 0x54)}
        out['events'] = self.trace
        out['popped'] = self.popped
        out['unused'] = {'forward': len(self.fprobes), 'origin': len(self.oprobes),
                         'overlap': len(self.overlaps), 'in_range': len(self.ranges)}
        return out


class TileMachine(harness.Machine):
    def run(self, case):
        grid = self.alloc(0x100)
        self.w32(grid + 4, case['shift'], case['cell_size'])
        out = self.alloc(0x18)
        self.w32(out, *([0xdeadbeef] * 6))
        self.call(0x52f3e0, grid, (out, *case['tile']))
        assert self.final_esp == harness.STACK + 0x10 and self.fpu_top() == 0
        words = self.r32(out, 6)
        assert words[0] == grid + 0x30
        return {'words': words[1:]}


# ---------------------------------------------------------------------------
# Scenario generation
# ---------------------------------------------------------------------------

def extend_actor(rng, a, cs, **o):
    kind = o.get('kind', rng.choice([0, 1, 1, 2, 3, 3, 4]))
    w, l = rng.uniform(0.4, 2.4) * cs, rng.uniform(0.4, 2.4) * cs
    if kind == 2:
        l = w
    a['shape6'] = o.get('shape6', box_shape(kind, w, l))
    a['shape'] = [a['shape6'][0], a['shape6'][1], a['shape6'][5]]
    sk = rng.choice([1, 2, 3])
    sw, sl = rng.uniform(0.3, 2.0) * cs, rng.uniform(0.3, 2.0) * cs
    a['soft6'] = box_shape(sk, sw, sw if sk == 2 else sl)
    a['flags'] = o.get('flags', rng.choice([0, 0, 1 << 21, 0x400]))
    a['moving'] = o.get('moving', rng.random() < 0.5)
    a['v4'] = f2b(o.get('v4', rng.uniform(0.2, 1.5) * cs))
    a['v18'] = f2b(o.get('v18', rng.uniform(0.5, 6.0)))
    a['v20'] = o.get('v20', rng.random() < 0.5)
    a['v16c'] = []
    a['mind'] = o.get('mind')
    a['org'] = o.get('org')
    return a


def uai_template(rng, name, entry, solve=None, **o):
    solve = dict(solve or {})
    solve.setdefault('distance', rng.uniform(3.0, 9.0))
    case = base_case(rng, name, **solve)
    cs = b2f(case['grid']['cell_size'])
    for a in case['actors']:
        extend_actor(rng, a, cs)
    sx, sy = b2f(case['request']['start'][0]), b2f(case['request']['start'][1])
    gx, gy = b2f(case['request']['goal'][0]), b2f(case['request']['goal'][1])
    mover = make_actor(rng, case, (sx, sy), cs, role='mover')
    heading = math.atan2(gy - sy, gx - sx) + o.get('facing_turn', rng.uniform(-0.6, 0.6))
    mover['facing'] = [f2b(math.cos(heading)), f2b(math.sin(heading))]
    extend_actor(rng, mover, cs, **o.get('mover', {}))
    case['actors'].append(mover)
    target = make_actor(rng, case, (1.0, 1.0), 1.0, role='target')
    target['registered'] = True
    extend_actor(rng, target, cs)
    case['actors'].append(target)
    # Obstacle actor between start and goal.
    t = o.get('obstacle_t', rng.uniform(0.25, 0.75))
    ox = sx + (gx - sx) * t + rng.uniform(-1.0, 1.0) * cs * o.get('lateral', 1.0)
    oy = sy + (gy - sy) * t + rng.uniform(-1.0, 1.0) * cs * o.get('lateral', 1.0)
    obstacle = make_actor(rng, case, (ox, oy), cs, role='obstacle')
    extend_actor(rng, obstacle, cs, **o.get('obstacle', {}))
    case['actors'].append(obstacle)
    other = None
    if o.get('other', rng.random() < 0.5):
        other_actor = make_actor(rng, case, (ox + cs, oy - cs), cs, role='other')
        extend_actor(rng, other_actor, cs, **o.get('other_actor', {}))
        case['actors'].append(other_actor)
        other = len(case['actors']) - 1
    case['continuation'] = {
        'actor': len(case['actors']) - (4 if other is not None else 3), 'target': None,
        'target_valid': True, 'replacement': True, 'owner_word_54': o.get('owner_word_54', 0),
        'argument': 0, 'last_time': o.get('last_time', 0), 'world_time': f2b(10.0),
        'waypoint_offset': f2b(0.75), 'use_path_goal': False,
    }
    case['continuation']['target'] = case['continuation']['actor'] + 1
    case['world_time'] = case['continuation']['world_time']
    # Route: optional kind-0 prefix, then a kind-1 waypoint at the goal.
    d = unit_bits(gx - sx, gy - sy)
    nodes = []
    for i in range(o.get('prefix', rng.choice([0, 0, 1, 2]))):
        f = (i + 1) * 0.2
        nodes.append(node(sx + (gx - sx) * f, sy + (gy - sy) * f, b2f(d[0]), b2f(d[1]), 0))
    if o.get('route', True):
        nodes.append(node(gx, gy, b2f(d[0]), b2f(d[1]), o.get('head_kind', 1)))
        nodes.append(node(gx + 3.0, gy + 1.0, 0.0, 1.0, 2))
    case['path'] = {'nodes': nodes, 'kind1': sum(n[4] == 1 for n in nodes), 'kind2': sum(n[4] == 2 for n in nodes)}
    cv = o.get('cv')
    if cv is None:
        a = math.atan2(oy - sy, ox - sx) + rng.uniform(-0.4, 0.4)
        cv = [f2b(math.cos(a)), f2b(math.sin(a))]
    minds = {'owner': {'v44': 'nopoint', 'v68': o.get('owner_v68', rng.random() < 0.3)}}
    minds.update(o.get('minds', {}))
    case['uai'] = {
        'entry': entry, 'cv': cv, 'step': f2b(o.get('step', rng.choice([0.05, 0.1, 0.125, 0.2]))),
        'speed': f2b(o.get('speed', rng.uniform(1.0, 8.0))), 'phase': 1, 'mode': o.get('mode', rng.choice([0, 1, 3])),
        'other': other, 'distance': f2b(o.get('distance', rng.uniform(0.5, 6.0) * cs)),
        'minds': minds, 'owner': minds['owner'],
        'forward': o.get('forward', [probe(rng, o.get('hit_p', 0.4)) for _ in range(6)]),
        'origin': o.get('origin', [probe(rng, o.get('hit_p', 0.4)) for _ in range(4)]),
        'overlap': o.get('overlap', [rng.random() < 0.6]), 'in_range': o.get('in_range', [rng.random() < 0.5]),
        'descriptor': o.get('descriptor', {'source': 'actor', 'actor': len(case['actors']) - (2 if other is not None else 1)}),
    }
    case['obstacle_index'] = case['actors'].index(obstacle)
    case['mover_index'] = case['continuation']['actor']
    for hook in o.get('hooks', []):
        hook(case)
    return case


def probe(rng, p):
    if rng.random() < p:
        return [rng.choice([1, 2, 3]), rng.getrandbits(32), rng.getrandbits(32), rng.getrandbits(32)]
    return [0, 0, 0, 0]


def static_cases():
    rng = random.Random(0x534a40)
    rows = []

    def add(name, **o):
        rows.append(uai_template(rng, name, 'static', **o))

    def leader_org(leader, thing_mind='group', component=True):
        return {'thing': {'mind': thing_mind, 'component': component, 'leader': leader}}

    def set_leader(case, **fields):
        leader = make_actor(rng, case, (b2f(case['actors'][case['mover_index']]['position'][0]) - 3.0,
                                        b2f(case['actors'][case['mover_index']]['position'][1]) + 1.5),
                            b2f(case['grid']['cell_size']), role='leader')
        extend_actor(rng, leader, b2f(case['grid']['cell_size']), **fields)
        case['actors'].append(leader)
        case['actors'][case['mover_index']]['org'] = leader_org(len(case['actors']) - 1)
        return leader

    # Plain shapes (no organization), every descriptor kind, both sources.
    for i in range(10):
        add(f'circle_{i}', obstacle={'kind': rng.choice([0, 1])}, hit_p=0.3)
    for i in range(14):
        add(f'box_{i}', obstacle={'kind': rng.choice([2, 3, 3, 4])}, hit_p=0.3)
    for i in range(6):
        add(f'tile_{i}', descriptor={'source': 'tile', 'tile': [rng.randrange(4, 20), rng.randrange(4, 20)]},
            hit_p=0.3)
    add('tile_alignment_cached', descriptor={'source': 'tile', 'tile': [10, 11], 'alignment': 2})
    add('tile_alignment_cached_other', descriptor={'source': 'tile', 'tile': [9, 12], 'alignment': 7})
    # Kind 4 (neither circle nor rectangle) with alignment 2/3: 0x46DC50/0x46DCB0 radius branch.
    for i, facing in enumerate([[0, f2b(1.0)], [f2b(-1.0), 0], [0x80000000, f2b(-1.0)]]):
        add(f'aligned_kind4_{i}', obstacle={'kind': 4},
            hooks=[lambda c, f=facing: c['actors'][c['obstacle_index']].update(facing=f)])
    for i, cached in enumerate([2, 3, 9]):
        add(f'cached_kind4_{i}', obstacle={'kind': 4},
            hooks=[lambda c, a=cached: c['uai']['descriptor'].update(alignment=a)])
    add('actor_alignment_cached', obstacle={'kind': 3},
        hooks=[lambda c: c['uai']['descriptor'].update(alignment=3)])
    # Axis-aligned boxes: alignment 2/3 and |axis| <= 1e-4 extent selection.
    for i, facing in enumerate([[0, f2b(1.0)], [f2b(1.0), 0], [0, f2b(-1.0)], [f2b(-1.0), 0x80000000]]):
        add(f'aligned_box_{i}', obstacle={'kind': rng.choice([2, 3])},
            hooks=[lambda c, f=facing: c['actors'][c['obstacle_index']].update(facing=f)])
    # Early continuation: empty route, zero collision vector (+/-0).
    add('empty_route', route=False, prefix=0)
    add('kind0_only_route', route=False, prefix=2)
    add('zero_vector', cv=[0, 0])
    add('negative_zero_vector', cv=[0x80000000, 0])
    add('zero_x_only', cv=[0, f2b(1.0)])
    add('zero_y_only', cv=[f2b(-1.0), 0x80000000])
    # Options exhausted: every probe hits; fallback forward probe variants.
    allhit = [[1, 0, 0, 0]] * 6
    add('fallback_clear', forward=[[1, 0, 0, 0]] * 4 + [[0, 0, 0, 0]] * 2, origin=[[1, 0, 0, 0]] * 4)
    add('fallback_hit_no_other', forward=allhit, origin=allhit, other=False)
    add('fallback_hit_mover_flag_v20_no_other', forward=allhit, origin=allhit, other=False,
        mover={'flags': 1 << 21, 'v20': True})
    add('fallback_hit_mover_flag_v20_other_flag_v20', forward=allhit, origin=allhit, other=True,
        mover={'flags': 1 << 21, 'v20': True}, other_actor={'flags': 1 << 21, 'v20': True})
    add('fallback_hit_mover_flag_v20_other_flag_no_v20', forward=allhit, origin=allhit, other=True,
        mover={'flags': 1 << 21, 'v20': True}, other_actor={'flags': 1 << 21, 'v20': False, 'mind': 'other',
                                                           'moving': False},
        minds={'other': {'v44': 'nopoint', 'v68': False}}, owner_v68=False)
    add('fallback_hit_mover_flag_no_v20', forward=allhit, origin=allhit, other=True,
        mover={'flags': 1 << 21, 'v20': False}, other_actor={'mind': 'other', 'moving': False},
        minds={'other': {'v44': 'nopoint', 'v68': False}}, owner_v68=False)
    add('fallback_hit_other_no_mind', forward=allhit, origin=allhit, other=True,
        mover={'flags': 0}, other_actor={'mind': None})
    add('fallback_hit_other_moving', forward=allhit, origin=allhit, other=True,
        mover={'flags': 0}, other_actor={'mind': 'other', 'moving': True},
        minds={'other': {'v44': 'nopoint', 'v68': False}})
    add('fallback_hit_other_mind_v68', forward=allhit, origin=allhit, other=True,
        mover={'flags': 0}, other_actor={'mind': 'other', 'moving': False},
        minds={'other': {'v44': 'nopoint', 'v68': True}})
    add('fallback_hit_v16c', forward=allhit, origin=allhit, other=True,
        mover={'flags': 0}, other_actor={'mind': 'other', 'moving': False},
        minds={'other': {'v44': 'nopoint', 'v68': False}},
        hooks=[lambda c: c['actors'][c['mover_index']]['v16c'].append('OTHER')])
    add('fallback_hit_owner_v68', forward=allhit, origin=allhit, other=True,
        mover={'flags': 0}, other_actor={'mind': 'other', 'moving': False},
        minds={'other': {'v44': 'nopoint', 'v68': False}}, owner_v68=True)
    for i in range(3):
        add(f'fallback_hit_ignore_{i}', forward=allhit, origin=allhit, other=True,
            mover={'flags': rng.choice([0, 0x400])}, other_actor={'mind': 'other', 'moving': False},
            minds={'other': {'v44': 'nopoint', 'v68': False}}, owner_v68=False)
    # Organization / leader following (0x4CEFC0, mode 3, Mind +0x44).
    add('org_without_thing', mover={'org': {'thing': None}}, mode=3)
    add('org_thing_null_component', mover={'org': leader_org(None, component=False)}, mode=3, prefix=0,
        minds={'group': {'v44': 'point', 'point': [f2b(1), f2b(2)], 'v68': False}})
    add('org_self_leader', prefix=0, mover={'org': leader_org('self')}, mode=3, minds={'group': {'v44': 'point',
                                                                                       'point': [f2b(1), f2b(2)],
                                                                                       'v68': False}})
    add('org_mode_not_3', prefix=0, mover={'org': leader_org(None)}, mode=1, minds={'group': {'v44': 'point',
                                                                                     'point': [f2b(1), f2b(2)],
                                                                                     'v68': False}})
    add('org_null_leader_head_kind0', mover={'org': leader_org(None)}, mode=3, route=True, head_kind=0, prefix=0,
        minds={'group': {'v44': 'point', 'point': [f2b(1), f2b(2)], 'v68': False}})
    add('org_empty_path_mode3', mover={'org': leader_org(None)}, mode=3, route=False, prefix=0,
        minds={'group': {'v44': 'point', 'point': [f2b(1), f2b(2)], 'v68': False}})
    add('org_thing_no_mind', prefix=0, mover={'org': leader_org(None, thing_mind=None)}, mode=3)
    add('org_without_thing_mode3', prefix=0, mover={'org': {'thing': None}}, mode=3)
    add('org_v44_null', prefix=0, mover={'org': leader_org(None)}, mode=3, minds={'group': {'v44': 'null', 'v68': False}})
    add('org_v44_nopoint', prefix=0, mover={'org': leader_org(None)}, mode=3, minds={'group': {'v44': 'nopoint', 'v68': False}})
    add('org_null_leader_follow', prefix=0, mover={'org': leader_org(None)}, mode=3,
        minds={'group': {'v44': 'point', 'point': [f2b(1), f2b(2)], 'v68': False}})
    group = {'group': {'v44': 'point', 'point': [f2b(3.0), f2b(4.0)], 'v68': False},
             'leadmind': {'v44': 'point', 'point': [f2b(40.5), f2b(33.25)], 'v68': False},
             'leadnull': {'v44': 'null', 'v68': False},
             'leadnopoint': {'v44': 'nopoint', 'v68': False}}
    for i, (fields, extra) in enumerate([
            ({'moving': True, 'mind': 'leadmind'}, {}),
            ({'moving': True, 'mind': None}, {}),
            ({'moving': True, 'mind': 'leadnopoint'}, {}),
            ({'moving': True, 'mind': 'leadnull'}, {}),
            ({'moving': False, 'mind': 'leadmind'}, {}),
            ({'moving': False, 'mind': None}, {'near': True}),
            ({'moving': True, 'mind': 'leadnopoint'}, {'near': True}),
            ({'moving': True, 'mind': 'leadmind'}, {'tiny': True}),
            ({'moving': False, 'mind': None}, {'cell': True})]):
        def hook(case, fields=fields, extra=extra):
            leader = set_leader(case, **fields)
            mover = case['actors'][case['mover_index']]
            if extra.get('near'):
                leader['position'] = [f2b(b2f(mover['position'][0]) + 0.00005), f2b(b2f(mover['position'][1]) - 0.0001)]
            if extra.get('tiny'):
                case['uai']['minds']['leadmind']['point'] = [f2b(b2f(leader['position'][0]) + 0.00003),
                                                             leader['position'][1]]
            if extra.get('cell'):
                leader['position'] = [f2b(b2f(mover['position'][0]) + 0.0002), mover['position'][1]]
        add(f'leader_follow_{i}', mode=3, prefix=0, minds=copy.deepcopy(group), hooks=[hook])
    # The second leader Mind +0x44 call differs from the first (0x534BC2 / 0x534BC5 reads).
    for i, second in enumerate(['null', 'nopoint']):
        minds = copy.deepcopy(group)
        minds['leadmind']['v44'] = ['point', second]
        add(f'leader_second_v44_{second}', mode=3, prefix=0, minds=minds,
            hooks=[lambda case: set_leader(case, moving=True, mind='leadmind')])
    for i in range(6):
        def hook(case):
            set_leader(case, moving=rng.random() < 0.5, mind=rng.choice([None, 'leadmind', 'leadnopoint']))
        add(f'leader_generated_{i}', mode=3, prefix=rng.choice([0, 0, 1]), minds=copy.deepcopy(group), hooks=[hook],
            obstacle={'kind': rng.choice([1, 2, 3, 4])}, hit_p=0.3)
    # Null obstacle shape: fault at the kind read 0x534C30.
    add('null_shape', hooks=[lambda c: c['actors'][c['obstacle_index']].update(shape_null=True)])
    add('null_shape_empty_route', route=False, prefix=0,
        hooks=[lambda c: c['actors'][c['obstacle_index']].update(shape_null=True)])
    # Clear probes: the first option succeeds (one or two inserted nodes).
    clear = [[0, 0, 0, 0]] * 6
    for i in range(10):
        add(f'option_clear_{i}', forward=clear, origin=clear if i % 2 else [[2, 5, 6, 7]] + clear,
            obstacle={'kind': rng.choice([0, 1, 2, 3, 4])}, lateral=rng.choice([0.2, 1.0, 2.0]))
    # Obstacle very close / overlapping the mover (inner-radius branch of 0x533140).
    for i in range(8):
        add(f'close_obstacle_{i}', obstacle_t=0.02, lateral=0.1, obstacle={'kind': rng.choice([1, 2, 3, 0, 4])},
            forward=[probe(rng, 0.3) for _ in range(6)], solve={'distance': rng.uniform(2.0, 6.0)})
    # Large step*speed minimum distance.
    for i in range(4):
        add(f'large_minimum_{i}', step=0.5, speed=rng.uniform(6.0, 12.0), hit_p=0.2)
    # Wait/failure inside 0x531520 after the early exit.
    add('continuation_waits', route=False, prefix=1, last_time=f2b(9.5))
    add('continuation_owner_mode', route=False, prefix=0, owner_word_54=2)
    for i in range(30):
        add(f'generated_{i}', hit_p=rng.choice([0.0, 0.2, 0.5, 0.9]), mode=rng.choice([0, 1, 3]),
            obstacle={'kind': rng.choice([0, 1, 2, 3, 4])}, lateral=rng.choice([0.0, 0.5, 1.0, 3.0]),
            facing_turn=rng.uniform(-2.5, 2.5))
    return rows


def thing_cases():
    rng = random.Random(0x536ad0)
    rows = []

    def add(name, **o):
        o.setdefault('other', True)
        case = uai_template(rng, name, 'thing', **o)
        # The moving thing is param_2; place it ahead of the mover.
        rows.append(case)

    for i in range(8):
        add(f'static_target_{i}', other_actor={'moving': False}, hit_p=0.3)
    allhit = [[1, 0, 0, 0]] * 6
    for i, (mind, owner) in enumerate([('other', False), (None, False), ('other', True)]):
        add(f'static_target_blocked_{i}', other_actor={'moving': False, 'mind': mind}, mover={'flags': 0},
            forward=allhit, origin=allhit, minds={'other': {'v44': 'nopoint', 'v68': False}}, owner_v68=owner)
    for i in range(2):
        add(f'static_target_fallback_clear_{i}', other_actor={'moving': False}, origin=allhit,
            hooks=[lambda c: c['uai'].update(forward_rule='clear_doubled')])
    add('static_target_null_shape', other_actor={'moving': False},
        hooks=[lambda c: c['actors'][c['uai']['other']].update(shape_null=True)])
    # Head-on / perpendicular: soft range in/out.
    for i, rng_result in enumerate([True, False, True, False]):
        def hook(case, flip=i):
            m = case['actors'][case['mover_index']]
            o = case['actors'][case['uai']['other']]
            f = [b2f(m['facing'][0]), b2f(m['facing'][1])]
            o['facing'] = [f2b(-f[0]), f2b(-f[1])] if flip < 2 else [f2b(-f[1]), f2b(f[0])]
        add(f'head_on_{i}', other_actor={'moving': True}, in_range=[rng_result], hooks=[hook])
    same = lambda case: case['actors'][case['uai']['other']].update(facing=case['actors'][case['mover_index']]['facing'])  # noqa: E731
    add('same_dir_no_overlap', other_actor={'moving': True}, overlap=[False], hooks=[same])
    add('same_dir_slower_mover', other_actor={'moving': True, 'v18': 5.0}, mover={'v18': 3.0}, overlap=[True],
        hooks=[same])
    add('same_dir_equal_speed', other_actor={'moving': True, 'v18': 4.0}, mover={'v18': 4.0}, overlap=[True],
        hooks=[same])
    add('same_dir_faster_soft_in', other_actor={'moving': True, 'v18': 2.0}, mover={'v18': 4.0}, overlap=[True],
        in_range=[True], hooks=[same])
    add('same_dir_faster_soft_out', other_actor={'moving': True, 'v18': 2.0}, mover={'v18': 4.0}, overlap=[True],
        in_range=[False], hooks=[same])
    add('same_dir_close_stop', other_actor={'moving': True, 'v18': 3.9}, mover={'v18': 4.0}, overlap=[True],
        in_range=[True], hooks=[same])
    add('same_dir_close_go', other_actor={'moving': True, 'v18': 3.9}, mover={'v18': 4.0}, overlap=[True],
        in_range=[False], hooks=[same])
    add('same_dir_close_boundary', other_actor={'moving': True, 'v18': 3.75}, mover={'v18': 4.0}, overlap=[True],
        in_range=[True], hooks=[same])
    for i in range(16):
        def hook(case):
            m = case['actors'][case['mover_index']]
            o = case['actors'][case['uai']['other']]
            a = math.atan2(b2f(m['facing'][1]), b2f(m['facing'][0])) + rng.uniform(-1.2, 1.2)
            o['facing'] = [f2b(math.cos(a)), f2b(math.sin(a))]
            o['v18'] = f2b(b2f(m['v18']) * rng.choice([0.5, 0.9, 0.97, 1.0, 1.2]))
        add(f'generated_{i}', other_actor={'moving': rng.random() < 0.85}, overlap=[rng.random() < 0.7],
            in_range=[rng.random() < 0.5], hooks=[hook], hit_p=0.3)
    return rows


def tile_cases():
    rng = random.Random(0x52f3e0)
    rows = []
    for shift, cs in [(0, 1.0), (1, 2.0), (2, 4.0), (1, 2.3), (0, 0.7), (3, 8.0), (1, 1.9)]:
        for _ in range(3):
            rows.append({'name': f'tile_{shift}_{cs}', 'shift': shift, 'cell_size': f2b(cs),
                         'tile': [rng.randrange(0, 600), rng.randrange(0, 600)]})
    rows.append({'name': 'tile_large', 'shift': 2, 'cell_size': f2b(4.3), 'tile': [0x1fffff, 0x0abcde]})
    rows.append({'name': 'tile_negative', 'shift': 1, 'cell_size': f2b(2.1), 'tile': [-5, 3]})
    return rows


def run_uai(case):
    m = UaiMachine(case)
    case['registry'] = []
    for a in case['actors']:
        idx = a['gtid'] & 0xffff
        gen = struct.unpack('<H', m.u.mem_read(m.registry + 4 + idx * 2, 2))[0]
        entry = [idx, gen, m.r32(m.registry + 0x20004 + idx * 4)]
        if entry not in case['registry']:
            case['registry'].append(entry)
    src = [a for a in case['actors'] if a['role'] == 'source']
    if case['source'] and src:
        case['context'][7] = src[0]['object']
    c = case['continuation']
    target = case['actors'][c['target']]
    c['target_id'] = target['gtid']
    c['replacement'] = {str(target['object']): case['actors'][c['actor']]['object']}
    c['destination'] = case['request']['goal']
    mover = case['actors'][case['mover_index']]
    if case['uai']['other'] is not None:
        other_obj = case['actors'][case['uai']['other']]['object']
        mover['v16c'] = [other_obj] if mover['v16c'] else []
    out = m.uai_case()
    for a in case['actors']:
        a['mind_object'] = m.r32(a['object'] + 0x70)
        org = a.get('org')
        a['org_objects'] = None
        if org is not None:
            membership = m.r32(a['object'] + 0x80)
            thing = m.r32(membership + 0x10)
            a['org_objects'] = {'thing': thing,
                                'thing_mind': m.r32(thing + 0x70) if thing else 0,
                                'component': m.r32(thing + 0x7c) if thing else 0,
                                'leader': m.r32(m.r32(thing + 0x7c) + 0x40) if thing and m.r32(thing + 0x7c) else 0}
    case['owner_object'] = m.owner
    case['state_object'] = m.state
    case['mind_objects'] = {str(k): v['object'] for k, v in case['uai']['minds'].items() if 'object' in v}
    case['tile_shape_object'] = m.grid + 0x30
    return out


FINDINGS = {
    'implementation': 'crates/aa-navigation/src/uai_collision.rs',
    'test': 'crates/aa-navigation/tests/uai_collision_native.rs',
    'ported': {
        '0x534A40': 'KKC_UAIStateMove::ResultAvoidStatic -> uai_collision::avoid_static',
        '0x533140': 'avoid option attempt (closest point, push-out, probes, two kind-0 nodes, ActionGo)',
        '0x52E210': 'avoid option distance past the obstacle along an axis',
        '0x46DAB0/0x46DC50/0x46DCB0': 'descriptor alignment cache (+0x14) and extent selection',
        '0x46E4F0/0x46E150': 'closest point on a placed shape (kind 1 normalize, kinds 2/3 local clamp, else zero vector 0xB279C0)',
        '0x51D510': 'Manhattan distance < 0.0002',
        '0x4CEFC0': 'actor is its organization leader',
        '0x536AD0': 'collision with a thing at a distance -> uai_collision::avoid_thing',
        '0x46EB30/0x46E0E0': 'swept rectangle shape constructor (kind 2 when width == length, else 3)',
        '0x4CF210/0x4EA0F0': 'placed-shape descriptors from actor virtual +0xC / +0x10',
        '0x52F3E0': 'tile descriptor (GWorld+0x30 grid shape, facing (0,-1))',
        '0x46EB60': 'square shape (0x46E0E0(size, size)); KKC_GMap 0x556570 builds the GMap+0x30 tile shape from the cell size',
    },
    'seams': ['0x4CEDE0', '0x531170', '0x5312C0', '0x46EF30', '0x46F100', '0x4D6940', 'actor virtuals +0/+4/+0xC/+0x10/+0x18/+0x20/+0x16C',
              'Mind virtuals +0x44/+0x68', '0x5300A0', '0x52D660', 'owner timer +4', 'all 0x531520/low-path seams of native_low_path.LowMachine'],
    'diagnostics': '0x404F90 lines 0x86C/0x873/0x87E are observed (level 0xB38B7C = 0, no output) and have no Rust callback',
}


def run_job(job):
    """Worker: one (group, index, precision) case; returns it with the executed instructions."""
    group, index, precision = job
    harness.VISITED.clear()
    if group == 'tile':
        case = copy.deepcopy(tile_cases()[index])
        case['precision'] = precision
        m = TileMachine(precision)
        case['output'] = m.run({**case, 'tile': [t & 0xffffffff for t in case['tile']]})
    else:
        case = copy.deepcopy((static_cases() if group == 'static' else thing_cases())[index])
        case['precision'] = precision
        case['output'] = run_uai(case)
    return group, case, dict(harness.VISITED)


def main():
    import multiprocessing
    sizes = {'static': len(static_cases()), 'thing': len(thing_cases()), 'tile': len(tile_cases())}
    jobs = [(g, i, precision) for g, n in sizes.items() for i in range(n) for precision in PRECISION]
    out = {g: [] for g in sizes}
    visited = {}
    with multiprocessing.Pool(int(sys.argv[1]) if len(sys.argv) > 1 else 6) as pool:
        for group, case, seen in pool.imap(run_job, jobs):
            out[group].append(case)
            visited.update(seen)
            o = case['output']
            if group != 'tile':
                print(group, case['precision'], case['name'], o['fault'], len(o['events']), flush=True)
    for a, b in visited.items():
        assert harness.P.range(a, len(b) // 2).hex() == b == harness.V.range(a, len(b) // 2).hex(), hex(a)
    doc = {'schema': 1, 'scope': __doc__, 'original_sha256': harness.ORIGINAL_SHA, 'v14_sha256': harness.V14_SHA,
           'producer_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
           'case_count': sum(len(v) for v in out.values()), 'cases': {g: len(v) for g, v in out.items()},
           'static': out['static'], 'thing': out['thing'], 'tile': out['tile'], 'findings': FINDINGS,
           'instructions': [{'va': hex(a), 'bytes': b} for a, b in sorted(visited.items())]}
    target = HERE / 'native-uai-collision.json'
    target.write_text(json.dumps(doc, separators=(',', ':')) + '\n')
    digest = hashlib.sha256(target.read_bytes()).hexdigest()
    print(json.dumps({'file': target.name, 'cases': doc['cases'], 'instructions': len(visited), 'sha256': digest}))


if __name__ == '__main__':
    main()
