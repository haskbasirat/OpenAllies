"""Original KKC_LowPathSolver (0x53C170) and UAIStateMove continuation 0x531520.

Whole unchanged routines execute under Unicorn: the solver constructor
0x53BB90 (window 0x53A490, queue 0x53CDF0/0x53C870), Solve 0x53C170, the core
loop 0x53BE30, solver vtable 0xA36460 (0x53B0C0, 0x7C7840, 0x4F0550, 0x53A790),
goal vtable 0xA3634C (0x53A5D0, 0x53A5F0, 0x53B720, 0x539F00, 0x53B960,
0x53A610, 0x539EE0, 0x539F10, 0x4C9FA0, 0x4DB6B0, 0x53A5C0, 0x6A71F0), the
neighbour iterator 0x53BC80/0x53BB40/0x53B3C0, line test 0x53AF70, window
helpers 0x53B1F0/0x53AAC0/0x53A570/0x539E90/0x53A990/0x53A820/0x53A3A0,
goal helpers 0x53ADB0/0x53AD00/0x53AF10/0x53A660/0x53A6F0/0x53B490/0x53AB90/
0x53A930/0x43AA00/0x4325A0/0x53B960/0x53AE60, normalize 0x409230, facing
0x5522B0, context position 0x552250 (no-snap branch: descriptor +0x18 is 0),
epoch 0x552440, shared queue 0x53D2F0/0x53D2A0/0x53D090/0x53D150/0x53CE20/
0x53CDA0, path list 0x52CF40/0x52D180/0x52D000/0x52CA90/0x52CAD0/0x52D1E0/
0x52D080, stats 0x539F60, profiler 0x41C0E0/0x41C110 (slot 0x17 no-ops) and
0x531520 itself with 0x529C30.

Seams (recorded in order): tile predicate 0x54F370, first hit 0x553BA0,
sweep 0x553CE0, placement 0x4F3C40, exclusion 0x552340, overlap refresh
0x4D67F0, actor virtuals +0/+0xB4/+0x100/+0xC, descriptor constructor
0x554350, replacement 0x4CE6E0, stop 0x52D660, 0x4CFC80, owner timer
(+8)+4, owner virtual +0x15C, ActionGo 0x5300A0; allocation 0x6F7810/
0x6F7430/0x85C680/0x85C2B0, QueryPerformance timers 0x59AA70/0x59AB60 and
SVar lookup 0x4B2E60 (returns the case's explicit value). 0x552250 is only
observed; its body runs. PathLog (0xB3D43C) stays 0; 0x404F60 must not run.
"""
import copy, hashlib, json, math, random, struct, sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
import harness  # noqa: E402
from harness import Machine, NullDereference, f2b, b2f, PRECISION, HERE  # noqa: E402

DYN = [8, 12, 14, 15, 17, 18, 19, 20, 21]  # 0x531520/solver-written descriptor words
NATIVE_LIMIT = 0x1f9c


class Unexpected(Exception):
    pass


class LowMachine(Machine):
    def __init__(self, case):
        super().__init__(case['precision'])
        self.c = case
        self.trace = []
        g = case['grid']
        self.width, self.height = g['width'], g['height']
        self.gworld = self.alloc(0x300)
        self.w32(0xb36534, self.gworld)
        self.grid = self.alloc(0x100)
        self.tiles = self.alloc(self.width * self.height * 0x30)
        self.w32(self.grid + 4, g['shift'], g['cell_size'])
        cs = b2f(g['cell_size'])
        self.w32(self.grid + 0xc, f2b(self.width * cs), f2b(self.height * cs))
        self.w32(self.grid + 0x1c, self.width, self.height)
        self.w32(self.grid + 0x28, self.tiles)
        self.w32(self.grid + 0x48, g['epoch'])
        for index, flags in g['flags']:
            self.w32(self.tiles + index * 0x30 + 0x20, flags)
        for index, stamp in g['stamps']:
            self.w32(self.tiles + index * 0x30 + 0x28, stamp)
        self.w32(self.gworld + 0x30, self.grid)
        self.w32(self.gworld + 0xe0, case.get('world_time', 0))
        if case['stats'] is not None:
            self.stats = self.alloc(0x34)
            self.w32(self.stats, *case['stats'])
        else:
            self.stats = 0
        self.w32(self.gworld + 4, self.stats)
        # Registry 0xB364C0: generations (u16) +4, objects +0x20004.
        self.registry = self.alloc(0x60010)
        self.w32(0xb364c0, self.registry)
        self.vtable = self.alloc(0x200)
        self.actor_info = {}
        for a in case['actors']:
            obj = self.alloc(0x100)
            a['object'] = obj
            self.w32(obj, self.vtable)
            self.w32(obj + 0x20, *a['position'])
            self.w32(obj + 0x2c, *a['facing'])
            self.w32(obj + 0x90, a['word90'])
            shape = self.alloc(0x20)
            self.w32(shape, a['shape'][0], a['shape'][1])
            self.w32(shape + 0x14, a['shape'][2])
            a['shape_ptr'] = shape
            self.actor_info[obj] = a
            idx = a['gtid'] & 0xffff
            gen = (a['gtid'] >> 16) if a['registered'] else ((a['gtid'] >> 16) ^ 0x5a5a) & 0xffff
            self.u.mem_write(self.registry + 4 + idx * 2, struct.pack('<H', gen))
            self.w32(self.registry + 0x20004 + idx * 4, obj)
        for off, handler in [(0, self.v0), (0xb4, self.vb4), (0x100, self.v100), (0xc, self.vshape)]:
            self.w32(self.vtable + off, self.stub(handler))
        self.w8(0xb3d43c, 0)
        self.w8(0xb3d43d, int(case['busy']))
        s = self.seams
        s[0x6f7810] = lambda m: m.ret(0, m.alloc(max(m.arg(0), 4)))
        s[0x6f7430] = lambda m: m.ret(0)
        s[0x85c680] = lambda m: m.ret(0, m.alloc(0x28))
        s[0x85c2b0] = lambda m: m.ret(4)
        s[0x59aa70] = lambda m: m.ret(0)
        s[0x59ab60] = lambda m: (m.note('timer'), m.ret(0))
        s[0x4b2e60] = self.svar
        # 0x53B720 logs 'destination too close' through 0x404F60 unconditionally.
        s[0x404f60] = lambda m: (m.note('log'), m.ret(0))
        s[0x411440] = self.unexpected
        s[0x54f370] = self.tile_blocked
        s[0x552250] = self.position
        s[0x553ba0] = self.first_hit
        s[0x553ce0] = self.sweep
        s[0x4f3c40] = self.placement
        s[0x552340] = self.excludes
        s[0x4d67f0] = lambda m: (m.note('refresh', m.ecx()), m.ret(0))
        self.context = None
        self.popped = []
        s[0x53bf90] = lambda m: m.popped.append(m.r32(m.solver + 0x24, 5))

    # ---- helpers -------------------------------------------------------
    def stub(self, handler):
        address = self.stub_next
        self.stub_next += 16
        self.seams[address] = handler
        return address

    def note(self, *row):
        self.trace.append(list(row))

    def unexpected(self, m):
        raise Unexpected(hex(m.u.reg_read(harness.UC_X86_REG_EIP)))

    def dyn(self, ctx):
        words = self.r32(ctx, 22)
        return [words[i] for i in DYN]

    def ctx_point(self, ctx):
        return b2f(self.r32(ctx + 0x4c)), b2f(self.r32(ctx + 0x50))

    def ignore_moving(self, ctx):
        return self.r8(ctx + 0x31) != 0

    def blockers(self, ctx):
        skip = self.ignore_moving(ctx)
        return [b for b in self.c['world']['circles'] if not (skip and b['moving'])]

    def hit_of(self, blocker):
        if blocker.get('actor') is not None:
            a = self.c['actors'][blocker['actor']]
            return 2, a['gtid']
        return 1, 0

    def tile_at(self, x, y):
        g = self.c['grid']
        cs = b2f(g['cell_size'])
        tx, ty = math.floor(x / cs), math.floor(y / cs)
        return tx, ty

    # ---- seams ---------------------------------------------------------
    def svar(self, m):
        name_ptr, kind = m.arg(0), m.arg(1)
        chars = []
        while True:
            ch = struct.unpack('<H', m.u.mem_read(name_ptr + 2 * len(chars), 2))[0]
            if ch == 0:
                break
            chars.append(chr(ch))
        name = ''.join(chars)
        expected_kind, value = self.c['svars'][name]
        assert kind == expected_kind, (name, kind)
        cell = m.alloc(0x40)
        m.w32(cell, value)
        m.w32(cell + 0x24, kind)
        m.note('svar', name, kind)
        m.ret(8, cell)

    def tile_blocked(self, m):
        tile, ctx = m.ecx(), m.arg(0)
        assert ctx == self.context, hex(ctx)
        index = (tile - self.tiles) // 0x30
        assert (tile - self.tiles) % 0x30 == 0 and 0 <= index < self.width * self.height
        x, y = index % self.width, index // self.width
        result = int([x, y] in self.c['world']['predicate_tiles'])
        m.note('tile', x, y, *self.dyn(ctx), result)
        m.ret(4, result)

    def position(self, m):
        # Observation only: 0x552250's original body runs (descriptor +0x18 = 0).
        assert m.ecx() == self.context
        m.note('pos', m.arg(0), m.arg(1))

    def first_hit(self, m):
        out, ctx = m.arg(0), m.arg(1)
        assert ctx == self.context and m.ecx() == self.grid
        x, y = self.ctx_point(ctx)
        kind, ref = 0, 0
        for b in self.blockers(ctx):
            if (x - b['x']) ** 2 + (y - b['y']) ** 2 < b['r'] ** 2:
                kind, ref = self.hit_of(b)
                break
        else:
            if list(self.tile_at(x, y)) in self.c['world']['predicate_tiles']:
                kind, ref = 1, 0
        m.note('hit', *self.dyn(ctx), kind, ref)
        m.w32(out, kind, ref, 0, 0)
        m.ret(8, out)

    def sweep(self, m):
        out, ctx, gx, gy = m.arg(0), m.arg(1), m.arg(2), m.arg(3)
        assert ctx == self.context and m.ecx() == self.grid
        ax, ay = self.ctx_point(ctx)
        bx, by = b2f(gx), b2f(gy)
        kind, ref = 0, 0
        for b in self.blockers(ctx):
            dx, dy = bx - ax, by - ay
            length2 = dx * dx + dy * dy
            t = 0.0 if length2 == 0 else max(0.0, min(1.0, ((b['x'] - ax) * dx + (b['y'] - ay) * dy) / length2))
            px, py = ax + t * dx, ay + t * dy
            if (px - b['x']) ** 2 + (py - b['y']) ** 2 < b['r'] ** 2:
                kind, ref = self.hit_of(b)
                break
        else:
            if list(self.tile_at(bx, by)) in self.c['world']['predicate_tiles']:
                kind, ref = 1, 0
        m.note('sweep', gx, gy, *self.dyn(ctx), kind, ref)
        m.w32(out, kind, ref, 0, 0)
        m.ret(16, out)

    def placement(self, m):
        actor, ctx = m.arg(0), m.arg(1)
        assert ctx == self.context and m.ecx() == self.gworld
        x, y = self.ctx_point(ctx)
        g = self.c['grid']
        cs = b2f(g['cell_size'])
        if x < 0 or y < 0 or x >= self.width * cs or y >= self.height * cs:
            result = 3
        elif any((x - b['x']) ** 2 + (y - b['y']) ** 2 < b['r'] ** 2 for b in self.c['world']['circles']):
            result = 4
        elif list(self.tile_at(x, y)) in self.c['world']['region_tiles']:
            result = 1
        else:
            result = 0
        m.note('place', actor, *self.dyn(ctx), result)
        m.ret(8, result)

    def excludes(self, m):
        ctx, actor = m.ecx(), m.arg(0)
        assert ctx == self.context
        result = int(self.actor_info[actor]['excluded'])
        m.note('excl', actor, *self.dyn(ctx), result)
        m.ret(4, result)

    def v0(self, m):
        a = self.actor_info[m.ecx()]
        m.note('v0', m.ecx(), a['reach'])
        m.ret_float(a['reach'], 'f32')

    def vb4(self, m):
        a = self.actor_info[m.ecx()]
        m.note('vb4', m.ecx(), int(a['vb4']))
        m.ret(0, int(a['vb4']))

    def v100(self, m):
        a = self.actor_info[m.ecx()]
        m.note('v100', m.ecx(), a['v100'])
        m.ret(0, a['v100'])

    def vshape(self, m):
        a = self.actor_info[m.ecx()]
        m.note('shape', m.ecx(), *a['shape'])
        m.ret(0, a['shape_ptr'])

    # ---- objects -------------------------------------------------------
    def build_path(self, collection, p):
        nodes = []
        for words in p['nodes']:
            n = self.alloc(0x28)
            self.w32(n, *words)
            nodes.append(n)
        for i, n in enumerate(nodes):
            self.w32(n + 0x20, nodes[i - 1] if i else 0)
            self.w32(n + 0x24, nodes[i + 1] if i + 1 < len(nodes) else 0)
        self.w32(collection + 4, nodes[0] if nodes else 0, nodes[-1] if nodes else 0,
                 p['kind1'] & 0xffffffff, p['kind2'] & 0xffffffff)
        self.w8(collection + 0x14, 0)
        self.w32(collection + 0x18, 0x13572468)

    def read_path(self, collection):
        head, tail, k1, k2 = self.r32(collection + 4, 4)
        nodes, prev, n = [], 0, head
        while n:
            assert self.r32(n + 0x20) == prev
            nodes.append(self.r32(n, 8))
            prev, n = n, self.r32(n + 0x24)
            assert len(nodes) < 10000
        assert tail == prev
        return {'nodes': nodes, 'kind1': k1, 'kind2': k2, 'dirty': self.r8(collection + 0x14),
                'word18': self.r32(collection + 0x18)}

    def construct(self):
        self.solver = self.alloc(0xd0)
        self.call(0x53bb90, self.solver, (self.grid, 2, 0x10, 0x10))
        assert self.fpu_top() == 0
        self.w32(self.gworld + 0x23c, self.solver)
        w = self.r32(self.solver + 0x44, 14)
        assert w[0] == self.grid and self.r32(self.solver + 0xc) == self.solver + 0x7c
        constructed = {'window': w[1:13], 'goal_window': self.r32(self.solver + 0x7c + 0x2c) == self.solver + 0x44}
        self.cells = w[13]
        q = self.solver + 0xb4
        self.w32(q + 0x10, self.c['queue_limit'])
        self.w32(q + 8, self.c['queue_count'])
        for index, stamp, direction in self.c['pre_cells']:
            self.w32(self.cells + index * 12 + 4, stamp, direction)
        return constructed

    def snapshot(self, path_addr):
        g = self.solver + 0x7c
        window = self.r32(self.solver + 0x44, 14)
        cells = self.r32(self.cells, 64 * 64 * 3)
        initial = {i: (s, d) for i, s, d in self.c['pre_cells']}
        changed = []
        for i in range(64 * 64):
            stamp, direction = cells[3 * i + 1], cells[3 * i + 2]
            if (stamp, direction) != initial.get(i, (0, 0)):
                changed.append([i, stamp, direction])
        stamps = []
        for i in range(self.width * self.height):
            v = self.r32(self.tiles + i * 0x30 + 0x28)
            if v:
                stamps.append([i, v])
        gw = self.r32(g, 14)
        return {
            'epoch': self.r32(self.grid + 0x48),
            'tile_stamps': stamps,
            'window': window[1:13],
            'changed_cells': changed,
            'goal': {'start_cell': gw[1:3], 'goal_cell': gw[3:5], 'path_linked': gw[5] != 0,
                     'context_linked': gw[6] != 0, 'start_bits': gw[7:9], 'goal_bits': gw[9:11],
                     'direct': (gw[12] & 0xff) != 0, 'examined': gw[13]},
            'search': self.r32(self.solver + 0x10, 12),
            'queue_count': self.r32(self.solver + 0xb4 + 8),
            'path': self.read_path(path_addr),
            'context': self.r32(self.context, 22),
            'busy': self.r8(0xb3d43d),
            'stats': self.r32(self.stats, 13) if self.stats else None,
        }

    def solve_case(self):
        constructed = self.construct()
        path = self.alloc(0x20)
        self.build_path(path, self.c['path'])
        self.context = self.alloc(0x58)
        self.w32(self.context, *self.c['context'])
        r = self.c['request']
        out = {'constructed': constructed}
        try:
            status = self.call(0x53c170, self.solver, (path, *r['start'], *r['goal'], self.context), count=50_000_000)
            assert self.final_esp == harness.STACK + 4 + 0x18
            assert self.fpu_top() == 0
            out['status'] = status
            out['fault'] = None
        except NullDereference as error:
            out['status'] = None
            out['fault'] = str(error)
        out.update(self.snapshot(path))
        out['events'] = self.trace
        out['popped'] = self.popped
        return out

    # ---- 0x531520 ------------------------------------------------------
    def continuation_case(self, entry='continuation'):
        constructed = self.construct()
        c = self.c['continuation']
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
        self.w32(owner_vt + 0x15c, self.stub(lambda m: (m.note('failed', m.arg(0)), m.ret(4))))
        # ResultPlan's attempts-exhausted owner virtual +0x124.
        self.w32(owner_vt + 0x124, self.stub(lambda m: (m.note('plan_failed'), m.ret(0))))
        self.w32(timer_vt + 4, self.stub(lambda m: (m.note('schedule', m.arg(0), m.arg(1)), m.ret(8))))
        self.w32(state + 0x10, c['target_id'], *c['destination'])
        self.w32(state + 0x54, c['waypoint_offset'])
        self.w32(state + 0x78, c['last_time'])
        if entry == 'result_plan':
            rp = self.c['result_plan']
            self.w32(state + 0x28, rp['attempts'] & 0xffffffff)
            self.w32(state + 0x2c, rp['phase'], rp['mode'])
            self.w32(state + 0x34, *rp['previous_destination'])
            regions = list(rp['regions'])

            def region(m):
                assert m.ecx() == self.gworld
                result = regions.pop(0)
                m.note('region', m.arg(0), m.arg(1), result)
                m.ret(8, result)
            self.seams[0x4f2220] = region
            self.seams[0x5313f0] = self.unexpected
            self.seams[0x53e1f0] = self.unexpected
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
        out = {'constructed': constructed}
        try:
            if entry == 'result_plan':
                self.call(0x5336b0, state, (0, c['argument']), count=50_000_000)
                assert self.final_esp == harness.STACK + 12
            else:
                self.call(0x531520, state, (c['argument'],), count=50_000_000)
                assert self.final_esp == harness.STACK + 8
            assert self.fpu_top() == 0
            out['fault'] = None
        except NullDereference as error:
            out['fault'] = str(error)
        if self.context is None:
            self.context = self.alloc(0x58)
        out.update(self.snapshot(state + 0x58))
        out['state'] = {'last_time': self.r32(state + 0x78), 'waypoint_offset': self.r32(state + 0x54),
                        'target_id': self.r32(state + 0x10), 'destination': self.r32(state + 0x14, 2),
                        'attempts': self.r32(state + 0x28), 'previous_destination': self.r32(state + 0x34, 2)}
        out['events'] = self.trace
        out['popped'] = self.popped
        return out


# ---------------------------------------------------------------------------
# Scenario generation
# ---------------------------------------------------------------------------

def unit(rng):
    a = rng.uniform(-math.pi, math.pi)
    return [f2b(math.cos(a)), f2b(math.sin(a))]


def node(x, y, dx, dy, kind):
    return [f2b(x), f2b(y), f2b(dx), f2b(dy), kind, 0, f2b(x), f2b(y)]


def base_case(rng, name, **o):
    shift, cs = o.get('cell', (1, 2.0))
    width, height = o.get('size', (28, 26))
    flags = []
    density = o.get('static_density', 0.0)
    for y in range(height):
        for x in range(width):
            v = 0
            if rng.random() < density:
                v = rng.choice([1, 1, 3, 0x401])
            elif rng.random() < 0.05:
                v = rng.choice([0x400, 2, 0x20])
            if v:
                flags.append([y * width + x, v])
    world_w, world_h = width * cs, height * cs
    sx = o.get('start', (rng.uniform(0.4, 0.6) * world_w, rng.uniform(0.4, 0.6) * world_h))
    if 'goal' in o:
        gx, gy = o['goal']
    else:
        a = rng.uniform(-math.pi, math.pi)
        d = o.get('distance', rng.uniform(1.0, 12.0)) * cs
        gx, gy = sx[0] + math.cos(a) * d, sx[1] + math.sin(a) * d
    case = {
        'name': name,
        'grid': {'width': width, 'height': height, 'shift': shift, 'cell_size': f2b(cs),
                 'epoch': o.get('epoch', rng.randrange(1, 5000)), 'flags': flags,
                 'stamps': o.get('stamps', [])},
        'settings': {'examined_max': o.get('examined_max', 512), 'ignore_moving_depth': o.get('ignore', 3),
                     'accuracy_depth': o.get('accuracy', 5)},
        'queue_limit': o.get('queue_limit', NATIVE_LIMIT),
        'queue_count': o.get('queue_count', 0),
        'pre_cells': o.get('pre_cells', []),
        'busy': o.get('busy', False),
        'stats': o.get('stats', None),
        'request': {'start': [f2b(sx[0]), f2b(sx[1])], 'goal': [f2b(gx), f2b(gy)]},
        'path': o.get('path', {'nodes': [], 'kind1': 0, 'kind2': 0}),
        'actors': [],
        'world': {'circles': [], 'predicate_tiles': [], 'region_tiles': []},
    }
    case['svars'] = {
        'LowPathingExaminedMax': [3, case['settings']['examined_max'] & 0xffffffff],
        'LowPathingIgnoreMovingDepth': [3, case['settings']['ignore_moving_depth'] & 0xffffffff],
        'LowPathingAccuracyDepth': [3, case['settings']['accuracy_depth'] & 0xffffffff],
        'LowPathingWaitMinimum': [2, o.get('wait', f2b(2.0))],
    }
    # Source actor (descriptor +0x1C) with its own virtuals.
    source = o.get('source', True)
    words = [rng.getrandbits(32) for _ in range(22)]
    words[6] = 0  # no source definition: 0x552250 takes its plain store branch
    words[12] = (words[12] & ~0xff00) | ((rng.random() < 0.3) << 8)
    words[7] = 0
    case['context'] = words
    case['source'] = source
    for i in range(o.get('circles', 0)):
        add_circle(rng, case, sx, (gx, gy), cs, **o.get('circle', {}))
    for t, r, moving, actor in o.get('explicit_circles', []):
        circle = {'x': sx[0] + (gx - sx[0]) * t, 'y': sx[1] + (gy - sx[1]) * t, 'r': r * cs, 'moving': moving,
                  'actor': None}
        if actor:
            case['actors'].append(make_actor(rng, case, (circle['x'], circle['y']), cs, role='blocker'))
            circle['actor'] = len(case['actors']) - 1
        case['world']['circles'].append(circle)
    for a in case['actors']:
        if a['role'] == 'blocker':
            a.update(o.get('blocker', {}))
    start_tile = (int(sx[0]) >> shift, int(sx[1]) >> shift)
    for dx, dy in o.get('wall_tiles', []):
        x, y = start_tile[0] + dx, start_tile[1] + dy
        if 0 <= x < width and 0 <= y < height:
            index = y * width + x
            flags = [f for f in flags if f[0] != index]
            flags.append([index, 1])
            case['grid']['flags'] = flags
    if o.get('region_all'):
        case['world']['region_tiles'] = [[x, y] for y in range(height) for x in range(width)]
    for i in range(o.get('predicate', 0)):
        t = [rng.randrange(1, width - 1), rng.randrange(1, height - 1)]
        if t not in case['world']['predicate_tiles']:
            case['world']['predicate_tiles'].append(t)
    if source:
        case['actors'].append(make_actor(rng, case, sx, cs, role='source'))
    return case


def make_actor(rng, case, point, cs, role):
    gtid = (rng.randrange(1, 0x7fff) << 16) | rng.randrange(8, 0xfff0)
    return {
        'role': role, 'gtid': gtid, 'registered': rng.random() < 0.9,
        'position': [f2b(point[0]), f2b(point[1])], 'facing': unit(rng),
        'word90': rng.choice([0, 0x1234, 0x5678]) if role == 'blocker' else 0x1000,
        'vb4': rng.random() < 0.8, 'v100': rng.choice([0, 1, 7]), 'excluded': rng.random() < 0.15,
        'reach': f2b(rng.uniform(0.0, 1.2) * cs),
        'shape': [rng.choice([1, 2, 3, 4]), f2b(rng.uniform(0.2, 1.5) * cs), f2b(rng.uniform(0.2, 1.5) * cs)],
    }


def add_circle(rng, case, start, goal, cs, near_goal=False, actor_p=0.6, moving_p=0.4, radius=(0.3, 2.5)):
    if near_goal:
        cx, cy = goal[0] + rng.uniform(-0.6, 0.6) * cs, goal[1] + rng.uniform(-0.6, 0.6) * cs
    else:
        t = rng.uniform(0.15, 0.95)
        cx = start[0] + (goal[0] - start[0]) * t + rng.uniform(-2, 2) * cs
        cy = start[1] + (goal[1] - start[1]) * t + rng.uniform(-2, 2) * cs
    r = rng.uniform(*radius) * cs
    if (cx - start[0]) ** 2 + (cy - start[1]) ** 2 < (r + 0.05 * cs) ** 2:
        return
    circle = {'x': cx, 'y': cy, 'r': r, 'moving': rng.random() < moving_p, 'actor': None}
    if rng.random() < actor_p:
        a = make_actor(rng, case, (cx, cy), cs, role='blocker')
        case['actors'].append(a)
        circle['actor'] = len(case['actors']) - 1
    case['world']['circles'].append(circle)


def solve_cases():
    rng = random.Random(0x53c170)
    rows = []

    def add(name, **o):
        rows.append(base_case(rng, name, **o))

    # Open ground, both cells inside, various distances and lattices.
    for i, cell in enumerate([(1, 2.0), (0, 1.0), (2, 4.0), (1, 3.0), (0, 0.75), (1, 2.0)]):
        add(f'open_{i}', cell=cell, distance=rng.uniform(1.5, 7.0), examined_max=160)
    add('far_goal_clipped', distance=40.0)
    add('far_goal_clipped_diagonal', goal=(2.0, 2.0), start=(30.3, 28.7))
    add('same_cell_direct', distance=0.05)
    add('same_cell_blocked_sweep', distance=0.05, explicit_circles=[(0.6, 0.01, False, True)])
    add('same_cell_blocked_moving_ignored', distance=0.05, explicit_circles=[(0.6, 0.01, True, False)])
    add('same_cell_goal_in_circle', distance=0.05, explicit_circles=[(1.0, 0.012, False, False)])
    add('too_close', distance=0.004)
    add('start_outside_window_edge', start=(55.5, 20.0), distance=6.0)
    add('start_edge_low', start=(0.6, 1.1), distance=5.0)
    for i in range(6):
        add(f'static_walls_{i}', static_density=0.25, distance=rng.uniform(3.0, 9.0),
            examined_max=512 if i < 2 else 160)
    for i in range(5):
        add(f'circles_{i}', circles=rng.randrange(2, 7), distance=rng.uniform(3.0, 10.0))
    for i in range(4):
        add(f'moving_ignored_{i}', circles=5, circle={'moving_p': 0.9}, ignore=rng.choice([0, 1, 2]),
            distance=rng.uniform(4.0, 9.0))
    for i in range(3):
        add(f'accuracy_point_{i}', circles=4, accuracy=rng.choice([0, 1, 2]), distance=rng.uniform(3.0, 8.0))
    add('accuracy_all', circles=4, accuracy=100, ignore=100, distance=6.0, examined_max=160)
    for i in range(4):
        add(f'goal_in_actor_{i}', circles=3, circle={'near_goal': True, 'actor_p': 1.0, 'moving_p': 0.0,
                                                      'radius': (0.6, 1.8)}, distance=rng.uniform(3.0, 9.0))
    for i in range(4):
        add(f'goal_displaced_{i}', circles=2, circle={'near_goal': True, 'actor_p': 1.0, 'moving_p': 0.0,
                                                       'radius': (0.5, 1.5)}, distance=rng.uniform(3.0, 9.0),
            blocker={'vb4': True, 'v100': 1, 'word90': 0x2222, 'excluded': False, 'registered': True})
    add('goal_displaced_exclusion', circles=1, circle={'near_goal': True, 'actor_p': 1.0, 'moving_p': 0.0,
                                                       'radius': (0.8, 1.2)}, distance=5.0,
        blocker={'vb4': True, 'v100': 3, 'word90': 0x2222, 'excluded': True, 'registered': True})
    add('goal_displaced_stale_gtid', circles=1, circle={'near_goal': True, 'actor_p': 1.0, 'moving_p': 0.0,
                                                        'radius': (0.8, 1.2)}, distance=5.0,
        blocker={'vb4': True, 'v100': 3, 'word90': 0x2222, 'excluded': False, 'registered': False})
    add('adjust_exhausted', region_all=True, distance=3.0)
    add('goal_in_static_circle', circles=1, circle={'near_goal': True, 'actor_p': 0.0, 'moving_p': 0.0,
                                                     'radius': (1.5, 2.5)}, distance=6.0)
    add('goal_unreachable_line', circles=6, circle={'near_goal': True, 'actor_p': 0.0, 'radius': (3.0, 4.0)},
        distance=3.0)
    for i in range(3):
        add(f'predicate_tiles_{i}', predicate=40, distance=rng.uniform(3.0, 9.0))
    add('examined_one', examined_max=1, distance=6.0)
    add('examined_zero', examined_max=0, distance=6.0)
    add('examined_small', examined_max=12, distance=9.0, circles=3)
    add('examined_medium', examined_max=60, static_density=0.2, distance=12.0)
    add('depth_limit_partial', distance=15.0, examined_max=4000)
    add('depth_limit_blocked', static_density=0.3, distance=15.0, examined_max=400)
    add('queue_limit_one', queue_limit=1, distance=6.0)
    add('queue_limit_two', queue_limit=2, distance=6.0)
    add('queue_limit_unbounded', queue_limit=0, distance=6.0)
    add('busy_rejected', busy=True, queue_count=17, stats=[rng.getrandbits(16) for _ in range(13)])
    add('stats_success', stats=[0] * 13, distance=5.0)
    add('stats_failure', stats=[3] * 13, distance=0.004)
    add('stats_partial', stats=[0x7fffffff, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16], examined_max=8,
        distance=9.0)
    add('epoch_rollover', epoch=0x7ffffffe, stamps=[[3, 77], [40, 0x7ffffffe]], distance=5.0)
    add('stale_window_stamp', epoch=99, pre_cells=[[i, 100, 4] for i in range(2000, 2200)], distance=5.0,
        examined_max=200)
    add('stale_window_band', epoch=500, pre_cells=[[i, 501, 6] for i in range(2380, 2440)]
        + [[i, 7, 3] for i in range(100, 140)], distance=6.0)
    add('path_prefix_kind0', distance=5.0,
        path={'nodes': [node(1, 2, 1, 0, 0), node(3, 4, 0, 1, 0), node(5, 6, 1, 0, 1), node(7, 8, 0, 1, 2)],
              'kind1': 1, 'kind2': 1})
    add('path_kind1_first', distance=5.0,
        path={'nodes': [node(1, 2, 1, 0, 1), node(3, 4, 0, 1, 0)], 'kind1': 1, 'kind2': 0})
    add('no_source', source=False, circles=3, distance=6.0)
    add('small_map', size=(18, 17), distance=5.0)
    add('wide_map', size=(60, 22), cell=(1, 2.0), distance=8.0, circles=4, examined_max=200)
    # Non-dyadic cell sizes: centers, offsets and differences need more than
    # 24 significand bits, so every binary32 store point is observable.
    for i, cell in enumerate([(1, 2.3), (0, 1.7), (2, 3.3), (1, 2.9), (0, 0.9), (1, 1.3)]):
        add(f'odd_cell_{i}', cell=cell, distance=rng.uniform(2.0, 9.0), circles=rng.randrange(0, 4),
            static_density=rng.choice([0.0, 0.1]), examined_max=200)
    # Lattice-aligned goals: heuristic and priority ties exercise the shared
    # heap's tie order and the strict closest-cell comparison.
    for i, (cell, limit, density) in enumerate([((1, 2.0), 24, 0.0), ((1, 2.0), 60, 0.2), ((0, 1.0), 40, 0.1),
                                                ((2, 4.0), 512, 0.25), ((1, 2.3), 30, 0.1), ((1, 2.0), 512, 0.3),
                                                ((1, 2.0), 9, 0.0), ((0, 1.7), 50, 0.15)]):
        sub = cell[1] / 4
        sx = (round(rng.uniform(24, 30) * 8) / 8, round(rng.uniform(22, 28) * 8) / 8)
        k, m = rng.randrange(-28, 29), rng.randrange(-28, 29)
        add(f'lattice_{i}', cell=cell, start=(sx[0] * cell[1] / 2, sx[1] * cell[1] / 2), static_density=density,
            goal=(sx[0] * cell[1] / 2 + k * sub, sx[1] * cell[1] / 2 + m * sub), examined_max=limit)
    wall_axis = [(2, d) for d in range(-2, 3)]
    wall_diag = [(2, 0), (0, 2), (1, 1), (2, 1), (1, 2), (3, -1), (-1, 3)]
    for i, (offset, wall, limit) in enumerate([((16, 0), wall_axis, 30), ((16, 0), wall_axis, 90),
                                               ((16, 0), wall_axis, 400), ((0, -14), [(d, -2) for d in range(-2, 3)], 60),
                                               ((10, 10), wall_diag, 40), ((10, 10), wall_diag, 300),
                                               ((-12, 0), [(-2, d) for d in range(-3, 4)], 120),
                                               ((8, 8), [], 5)]):
        start = (28.25, 26.75)
        add(f'tie_wall_{i}', start=start, goal=(start[0] + offset[0] * 0.5, start[1] + offset[1] * 0.5),
            wall_tiles=wall, examined_max=limit)
    # Unreachable goal half a sub-cell between two rows inside a ring of
    # blocked tiles: the two frontier cells straddling the goal row tie at the
    # final closest-cell minimum (strict comparison 0x53BFBE..0x53BFC6).
    ring = [(2, -1), (2, 0), (2, 1), (3, -1), (3, 1), (4, -1), (4, 0), (4, 1)]
    for i, (limit, half) in enumerate([(80, 0.25), (150, 0.25), (120, -0.25), (60, 0.25)]):
        start = (28.25, 26.75)
        add(f'tie_ring_{i}', start=start, goal=(start[0] + 6.0, start[1] + half), wall_tiles=ring,
            examined_max=limit)
    for i in range(10):
        add(f'generated_{i}', cell=rng.choice([(1, 2.0), (0, 1.0), (2, 4.0), (1, 2.5)]),
            static_density=rng.choice([0.0, 0.1, 0.2]), circles=rng.randrange(0, 6),
            predicate=rng.randrange(0, 20), distance=rng.uniform(0.5, 14.0),
            examined_max=rng.choice([200, 128, 40]), ignore=rng.choice([3, 1, 6]),
            accuracy=rng.choice([5, 2, 8]))
    return [r for r in rows if r is not None]


def continuation_cases():
    rng = random.Random(0x531520)
    rows = []

    def add(name, solve=None, **c):
        case = base_case(rng, name, **(solve or {}))
        actor = make_actor(rng, case, [b2f(case['request']['start'][0]), b2f(case['request']['start'][1])],
                           b2f(case['grid']['cell_size']), role='mover')
        case['actors'].append(actor)
        target = make_actor(rng, case, (1.0, 1.0), 1.0, role='target')
        target['registered'] = True
        case['actors'].append(target)
        defaults = {
            'actor': len(case['actors']) - 2, 'target': len(case['actors']) - 1,
            'target_valid': True, 'replacement': True,
            'owner_word_54': 0, 'argument': 0, 'last_time': 0, 'world_time': f2b(10.0),
            'waypoint_offset': f2b(0.75), 'use_path_goal': False,
        }
        defaults.update(c)
        case['continuation'] = defaults
        case['world_time'] = defaults['world_time']
        rows.append(case)

    add('first_solve_empty_path', solve={'distance': 6.0})
    add('first_solve_path_goal', solve={'distance': 6.0, 'path': {'nodes': [], 'kind1': 0, 'kind2': 0}},
        use_path_goal=True)
    add('negative_last_time', last_time=f2b(-1.0), solve={'distance': 6.0})
    add('wait_too_soon', last_time=f2b(9.0), solve={'distance': 6.0})
    add('wait_exact_minimum', last_time=f2b(8.0), solve={'distance': 6.0})
    add('wait_just_under', last_time=f2b(8.000001), solve={'distance': 6.0})
    add('wait_custom_minimum', last_time=f2b(9.5), solve={'distance': 6.0, 'wait': f2b(0.25)})
    add('argument_flag', argument=0x101, solve={'distance': 6.0})
    add('argument_high_bytes_only', argument=0x12345600, solve={'distance': 6.0})
    add('owner_mode_two', owner_word_54=2, solve={'distance': 6.0})
    add('target_invalid', target_valid=False, solve={'distance': 6.0})
    add('target_no_replacement', replacement=False, solve={'distance': 6.0})
    add('solve_failure', solve={'distance': 0.004})
    add('solve_failure_kind0_prefix', use_path_goal=True,
        solve={'distance': 0.004, 'path': {'nodes': [], 'kind1': 0, 'kind2': 0}})
    add('partial_route', solve={'distance': 14.0, 'examined_max': 30})
    add('direct_line', solve={'distance': 0.2})
    add('queue_limit_null_tail', solve={'distance': 6.0, 'queue_limit': 1})
    add('obstacles', solve={'distance': 8.0, 'circles': 4, 'static_density': 0.15})
    add('stats_present', solve={'distance': 6.0, 'stats': [0] * 13})
    for i in range(4):
        add(f'generated_{i}', solve={'distance': rng.uniform(1.0, 10.0), 'circles': rng.randrange(0, 5)},
            last_time=rng.choice([0, f2b(1.0), f2b(9.9)]), argument=rng.choice([0, 1]),
            owner_word_54=rng.choice([0, 2]))
    return rows


def result_plan_cases():
    rng = random.Random(0x5336b0)
    rows = []

    def add(name, solve, regions=(5, 5), attempts=0, **c):
        case = base_case(rng, name, **solve)
        actor = make_actor(rng, case, [b2f(case['request']['start'][0]), b2f(case['request']['start'][1])],
                           b2f(case['grid']['cell_size']), role='mover')
        case['actors'].append(actor)
        target = make_actor(rng, case, (1.0, 1.0), 1.0, role='target')
        target['registered'] = True
        case['actors'].append(target)
        defaults = {
            'actor': len(case['actors']) - 2, 'target': len(case['actors']) - 1,
            'target_valid': True, 'replacement': True,
            'owner_word_54': 0, 'argument': 0, 'last_time': 0, 'world_time': f2b(10.0),
            'waypoint_offset': f2b(0.75), 'use_path_goal': False,
        }
        defaults.update(c)
        case['continuation'] = defaults
        case['world_time'] = defaults['world_time']
        case['result_plan'] = {'attempts': attempts, 'phase': 1, 'mode': 2, 'regions': list(regions),
                               'previous_destination': [f2b(99.0), f2b(98.0)]}
        rows.append(case)

    # Destinations within 3.0 of the actor (0x52E1D0) in the same region reach 0x531520.
    add('plan_continue_solve', {'distance': 1.2, 'circles': 1})
    add('plan_continue_solve_flags', {'distance': 1.3}, argument=0x01, owner_word_54=2)
    add('plan_continue_wait', {'distance': 1.2}, last_time=f2b(9.5))
    add('plan_continue_failure', {'distance': 0.004})
    add('plan_continue_direct', {'distance': 0.05})
    add('plan_continue_path_goal', {'distance': 1.1, 'path': {'nodes': [], 'kind1': 0, 'kind2': 0}},
        use_path_goal=True)
    add('plan_attempts_exhausted', {'distance': 1.2}, attempts=3, regions=())
    return rows


def run_case(case, kind):
    m = LowMachine(case)
    case['registry'] = []
    for a in case['actors']:
        idx = a['gtid'] & 0xffff
        gen = struct.unpack('<H', m.u.mem_read(m.registry + 4 + idx * 2, 2))[0]
        entry = [idx, gen, m.r32(m.registry + 0x20004 + idx * 4)]
        if entry not in case['registry']:
            case['registry'].append(entry)
    # Source actor pointer and the path-goal variant need allocated objects.
    src = [a for a in case['actors'] if a['role'] == 'source']
    if case['source'] and src:
        case['context'][7] = src[0]['object']
    if kind in ('continuation', 'result_plan'):
        c = case['continuation']
        target = case['actors'][c['target']]
        c['target_id'] = target['gtid'] if c['target_valid'] else (target['gtid'] ^ 0x10000)
        c['replacement'] = {str(target['object']): (case['actors'][c['actor']]['object'] if c['replacement']
                                                     else target['object'])}
        c['destination'] = case['request']['goal']
        if c['use_path_goal']:
            gx, gy = b2f(case['request']['goal'][0]), b2f(case['request']['goal'][1])
            case['path'] = {'nodes': [node(gx, gy, 1, 0, 1), node(gx + 3, gy + 1, 0, 1, 2)], 'kind1': 1,
                            'kind2': 1}
            c['destination'] = [f2b(gx + 50.0), f2b(gy - 50.0)] if kind == 'continuation' else \
                case['request']['goal']
        return m.continuation_case(kind)
    return m.solve_case()


FINDINGS = {
    'implementation': 'crates/aa-navigation/src/low_path/{mod,window,goal,search,continuation}.rs',
    'test': 'crates/aa-navigation/tests/low_path_native.rs',
    'ported': {
        '0x53BB90': 'KKC_LowPathSolver constructor (window, goal vtable 0xA3634C, queue 0x53CDF0) -> LowPathSolver::new',
        '0x53A490': 'window constructor (64x64 sub-cells, both extents from the width) -> Window::new',
        '0x53C170': 'Solve: window placement, cell conversion (0x96CC4C truncation), goal fields, core loop, stats -> low_path::solve',
        '0x53BE30': 'core loop (template shared with 0x53D8D0; reuses search::NativeQueue)',
        '0x53B0C0': 'solver vtable[0] expand candidate',
        '0x7C7840': 'solver vtable[1] examine (always 3)',
        '0x4F0550': 'solver vtable[2] mark start through goal Mark',
        '0x53A790': 'solver vtable[3] queue exhausted: 0 at start, else 2',
        '0x53BC80/0x53BB40/0x53B3C0': 'neighbour iterator (8 at start, 5 around a diagonal, 3 around an orthogonal predecessor)',
        '0x53A660': 'limit: depth >= 32 or examined >= LowPathingExaminedMax',
        '0x53B1F0': 'window placement around the start tile, clamped, lattice offset',
        '0x53AAC0': 'sub-cell center (X partial stored to binary32)',
        '0x53A570': 'world position to sub-cell',
        '0x539E90 and 0x53A6F0 bounds': 'window bounds (upper extended sum exclusive)',
        '0x53A990/0x53A820/0x53A3A0': 'goal clipping into the window along the start segment',
        '0x53ADB0': 'octile heuristic to the exact goal',
        '0x53AD00': 'accumulated Euclidean cost between sub-cell centers',
        '0x53AF10': 'cell position (exact start/goal)',
        '0x53AF70': 'neighbour line test: sweep below AccuracyDepth, end-point query otherwise; byte +0x31 ignore-moving',
        '0x53B720': 'OnSolveStarted: epoch 0x552440, kind-0 prefix removal 0x52CF40, goal adjust, same-cell direct sweep, overlap refresh',
        '0x53B490': 'goal adjust: step back one sub-cell at a time, displace past a blocking actor',
        '0x53AB90/0x53A930/0x43AA00/0x4325A0': 'goal displacement past an actor',
        '0x53B960/0x53AE60': 'OnSolved path building (kind-0 waypoints, one per heading change) and direct line',
        '0x53A610/0x539EE0/0x539F10/0x539F00/0x53A5D0/0x53A5F0/0x53A5C0/0x4C9FA0/0x4DB6B0/0x6A71F0': 'goal vtable leaves',
        '0x409230': 'normalize',
        '0x5522B0': 'descriptor facing store',
        '0x539F60': 'LowPathStats update (diagnostic object; ported because it is state)',
        '0x52CF40/0x52D180/0x52D000/0x52CA90/0x52CAD0/0x52D1E0/0x52D080': 'path list operations used here',
        '0x531520': 'UAIStateMove continuation -> low_path::continue_current',
    },
    'goal_vtable_0xA3634C': ['+00 0x53A5D0 IsGoal', '+04 0x53A5F0 GetStart', '+08 0x53B720 OnSolveStarted',
                             '+0C 0x539F00 OnFailed', '+10 0x53B960 OnSolved', '+14 0x53A610 Mark',
                             '+18 0x539EE0 IsVisited', '+1C 0x539F10 IsBlocked (0x54F370)',
                             '+20 0x4C9FA0 start depth 0', '+24 0x4DB6B0 start cost +0.0',
                             '+28 0x6A71D0 returns 1 (unused here)', '+2C 0x53A5C0 direct flag',
                             '+30 0x6A71F0 returns 0 (fan from the predecessor)'],
    'vtable_bounds': '0xA36348 is a null word (no RTTI locator); entries 0xA3634C..0xA3637C; 0xA36380 starts the LowPathingExaminedMax string',
    'solver_vtable_0xA36460': ['0x53B0C0', '0x7C7840', '0x4F0550', '0x53A790'],
    'settings': {
        'LowPathingExaminedMax': 'int (type 3), 0x53A660; SVars.tgi line 96 = 512',
        'LowPathingIgnoreMovingDepth': 'int (type 3), 0x53BC80; SVars.tgi line 97 = 3',
        'LowPathingAccuracyDepth': 'int (type 3), 0x53BC80; SVars.tgi line 98 = 5',
        'LowPathingWaitMinimum': 'fixed (type 2), 0x531520; SVars.tgi line 95 = 2',
        'PathingTerrainChangePenalty': 'not read by the low-path family (only by 0x53CB70, the tile-search cost)',
        'note': 'Looked up lazily by name through 0x4B2E60 in registry 0xB39AA8; 0x5065D0 registers none of them (no code default). A missing Var reports "Requested non-existent Var" and the caller dereferences null.',
    },
    'timing': '0x41C0E0/0x41C110(0x17) are no-ops (only slots 1..4 act); 0x59AB60 stores QueryPerformanceCounter at solver+0xC8 and discards the milliseconds; nothing reads it during the solve and 0x59ABA0 is unreachable from 0x53C170. No search limit depends on wall-clock time.',
    'caps': {
        'queue limit 0x1F9C (0x53CDF0 stores +0x10)': 'Options::queue_limit / NATIVE_QUEUE_LIMIT; storage grows',
        'window 16x16 tiles, sub shift 2 (0x4F2A80..0x4F2A89)': 'WindowConfig::NATIVE, explicit; cells are a Vec',
        'depth 32 (0x53A6AA cmp 0x20)': 'kept as the native search rule DEPTH_LIMIT',
    },
    'quirks': ['window extents both use the width',
               'world epoch is shared with the tile search; window stamps persist across solves, so stale stamps (or a rollover to epoch 0) read as visited',
               'OnSolved(start) keeps the goal path/context links',
               'a nonzero status can leave a previously empty path empty: 0x531520 then reads a null tail at 0x531711; Rust returns EmptySolvedPath',
               'a queue-limit failure returns 2 even when the closest cell is the start'],
}


def main():
    groups = {'solve': solve_cases(), 'continuation': continuation_cases(), 'result-plan': result_plan_cases()}
    counts = {}
    for kind, rows in groups.items():
        out = []
        harness.VISITED.clear()
        for template in rows:
            for precision in PRECISION:
                case = copy.deepcopy(template)
                case['precision'] = precision
                case['output'] = run_case(case, kind.replace('-', '_'))
                out.append(case)
                o = case['output']
                print(kind, precision, case['name'], o.get('status'), o['fault'], len(o['events']),
                      o['goal']['examined'], flush=True)
        name = f'native-low-path-{kind}.json'
        doc = {'schema': 1, 'scope': __doc__, 'original_sha256': harness.ORIGINAL_SHA,
               'v14_sha256': harness.V14_SHA, 'producer_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
               'dynamic_context_words': DYN, 'case_count': len(out), 'cases': out,
               'instructions': [{'va': hex(a), 'bytes': b} for a, b in sorted(harness.VISITED.items())]}
        target = HERE / name
        target.write_text(json.dumps(doc, separators=(',', ':')) + '\n')
        digest = hashlib.sha256(target.read_bytes()).hexdigest()
        counts[name] = {'cases': len(out), 'instructions': len(harness.VISITED), 'sha256': digest}
        print(json.dumps({'file': name, **counts[name]}))
    findings = dict(FINDINGS, fixtures=counts,
                    producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                    original_sha256=harness.ORIGINAL_SHA, v14_sha256=harness.V14_SHA)
    (HERE / 'native-low-path-findings.json').write_text(json.dumps(findings, indent=1) + '\n')


if __name__ == '__main__':
    main()
