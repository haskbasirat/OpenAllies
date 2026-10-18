"""Original SAI goal priority finalization 7A5200 and goal affordability 7A3860.

Both functions execute unchanged. Seams: goal virtual +4 (goal type), +48
(exempt from the settlement resource reserve), +78 (goal-specific base
priority, which writes goal+38 and may mutate any goal field), SAI 79E4B0 count
and the SAIDebug setting query 40EB50 (always reports debug output disabled;
the CSV dump is diagnostic I/O). The SAI RNG 45D400/85D840 executes natively on
the emulated SAI RNG word. Each case records the virtual-call order, final
goal fields, RNG state and 7A3860 result.
"""
from pathlib import Path
import copy, json, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import Machine, PRECISIONS, f32, header, instructions, sha
from unicorn.x86_const import UC_X86_REG_EAX

GOAL_TYPES = 23


class GoalMachine(Machine):
    def __init__(self, c):
        super().__init__(c['precision'])
        self.c = c
        self.mutations = copy.deepcopy(c.get('mutations', []))
        world = self.alloc(0x200); self.w(0xb36534, world); self.w(world + 0xe0, c['world_time'])
        app = self.alloc(0x200); self.w(0xb36550, app); self.wb(app + 0xf4, c['debug_suppressed'])
        sai = self.alloc(0x100); self.w(0xb36528, sai)
        self.rng = self.alloc(16); self.w(sai + 0x2c, self.rng); self.w(self.rng, c['rng_state'])
        n = len(c['resources'])
        defs = [self.alloc(0x40) for _ in range(n)]
        for d, res in zip(defs, c['resources']):
            self.w(d + 0x18, res['word18'])
        resmgr = self.alloc(0x100); self.w(0xb36530, resmgr); self.w(resmgr + 0x34, self.array(defs))
        ego = self.ego = self.alloc(0x400)
        e = c['ego']
        self.w(ego + 0x3c, e['default_fuzziness']); self.w(ego + 0x40, e['default_inertia'])
        for t, rec in enumerate(e['priorities']):
            for i, k in enumerate(['bonus', 'multiplier', 'minimum', 'fuzziness', 'inertia']):
                self.w(ego + 0x4c + t * 0x18 + 4 * i, rec[k])
        self.w(ego + 0x274, e['saving_time_limit']); self.w(ego + 0x278, e['reserved_amount'])
        self.w(ego + 0x27c, e['reserved_time']); self.w(ego + 0x280, e['reserved_max_settlements'])
        econ = self.alloc(0x200)
        self.w(econ + 0x1a0, self.array([r['word1a0'] for r in c['resources']]))
        self.w(econ + 0x1b8, self.array([r['word1b8'] for r in c['resources']]))
        p60 = self.alloc(0x40); p60_20 = self.alloc(0x40); self.w(p60 + 0x20, p60_20); self.w(p60_20 + 8, c['settlements'])
        player = self.alloc(0x100); self.w(player + 8, econ); self.w(player + 0x1c, ego); self.w(player + 0x60, p60)
        engine = self.alloc(0x40); self.w(engine + 4, player)
        vt = self.alloc(0x100)
        self.w(vt + 4, self.stub(self.v_type)); self.w(vt + 0x48, self.stub(self.v_exempt))
        self.w(vt + 0x4c, self.stub(self.v_describe)); self.w(vt + 0x78, self.stub(self.v_base))
        g = self.goal = self.alloc(0x100)
        self.w(g, vt); self.w(g + 4, engine)
        self.write_goal(c['goal'])
        avail_array = self.array([r['available'] for r in c['resources']])
        self.avail = self.alloc(16); self.w(self.avail, avail_array)
        self.seam(0x79e4b0, self.s_count)
        self.seam(0x40eb50, self.s_debug)

    def write_goal(self, g):
        b = self.goal
        self.w(b + 8, g['status']); self.w(b + 0x18, g['fuzzy']); self.w(b + 0x28, g['inertia_applied'])
        self.w(b + 0x38, g['priority']); self.w(b + 0x20, len(g['costs'])); self.w(b + 0x1c, self.array(g['costs']))
        self.goal_costs = list(g['costs'])

    def read_goal(self):
        b = self.goal
        n = self.r(b + 0x20)
        return {'status': self.r(b + 8), 'fuzzy': self.r(b + 0x18), 'inertia_applied': self.r(b + 0x28),
                'priority': self.r(b + 0x38), 'costs': [self.r(self.r(b + 0x1c) + 4 * i) for i in range(n)]}

    def event(self, name, result=None):
        self.events.append({'event': name, 'goal': self.read_goal(), 'result': result})
        if self.mutations and self.mutations[0]['event'] == name:
            m = self.mutations.pop(0)
            g = self.read_goal(); g.update(m['goal']); self.write_goal(g)

    def v_type(self, m):
        assert self.ecx() == self.goal
        self.event('goal_type', self.c['goal_type']); self.ret(self.c['goal_type'])

    def v_exempt(self, m):
        assert self.ecx() == self.goal
        self.event('reserve_exempt', self.c['reserve_exempt']); self.ret(int(self.c['reserve_exempt']))

    def v_describe(self, m):
        raise AssertionError('describe is reached only by the disabled diagnostic CSV path')

    def v_base(self, m):
        assert self.ecx() == self.goal
        g = self.read_goal(); g['priority'] = self.c['base_priority']; self.write_goal(g)
        self.event('base_priority'); self.ret(0)

    def s_count(self, m):
        assert self.ecx() == self.r(0xb36528)
        self.event('sai_count', self.c['sai_count']); self.ret(self.c['sai_count'])

    def s_debug(self, m):
        assert self.ecx() == 0xb39690
        self.events.append({'event': 'debug_setting_query'}); self.ret(0)

    def run(self):
        if self.c['entry'] == 'finalize':
            self.call(0x7a5200, ecx=self.goal, args=[self.avail, self.c['draw']])
            result = None
        else:
            result = self.call(0x7a3860, ecx=self.goal, args=[self.avail, self.c['relaxed']]) & 0xff
        assert not self.mutations, self.c['name']
        return {'goal': self.read_goal(), 'rng_state': self.r(self.rng), 'affordable': result, 'events': self.events}


def record(**k):
    base = {'bonus': f32(0), 'multiplier': f32(1), 'minimum': f32(0), 'fuzziness': f32(-1), 'inertia': f32(-1)}
    base.update(k)
    return base


def cases():
    pri = [record() for _ in range(GOAL_TYPES)]
    base = {
        'name': 'base', 'entry': 'finalize', 'precision': 'Single24', 'draw': 1, 'relaxed': 0,
        'world_time': f32(120.0), 'debug_suppressed': 0, 'rng_state': 0x12345678,
        'resources': [{'word18': 0, 'word1a0': f32(30), 'word1b8': f32(10), 'available': f32(500)},
                      {'word18': 0, 'word1a0': f32(5), 'word1b8': f32(8), 'available': f32(100)},
                      {'word18': 1, 'word1a0': f32(0), 'word1b8': f32(0), 'available': f32(0)}],
        'ego': {'default_fuzziness': f32(200), 'default_inertia': f32(250), 'priorities': pri,
                'saving_time_limit': f32(180), 'reserved_amount': f32(0), 'reserved_time': f32(0),
                'reserved_max_settlements': 0},
        'settlements': 1, 'sai_count': 1, 'reserve_exempt': False, 'goal_type': 0, 'base_priority': f32(400),
        'goal': {'status': 0, 'fuzzy': f32(17), 'inertia_applied': f32(99), 'priority': f32(-5),
                 'costs': [f32(300), f32(50), f32(1e9)]},
    }
    rows = []

    def add(name, change):
        c = copy.deepcopy(base); c['name'] = name; change(c); rows.append(c)

    # Status x draw x base sign.
    for status in [0xffffffff, 0, 1, 2, 3, 7]:
        for draw in [0, 1]:
            for bp in [-10.0, 0.0, -0.0, 1e-30, 400.0]:
                add(f'status{status:x}_draw{draw}_base{bp}', lambda c, s=status, d=draw, b=bp: (
                    c['goal'].update(status=s), c.update(draw=d, base_priority=f32(b))))
    # Type record use and default fallback.
    for t in [0, 4, 12, 22]:
        for fz, ine in [(-1, -1), (0, 0), (35.5, 77.25), (-0.0, -0.0), (-1e-20, 3)]:
            for status in [1, 2]:
                add(f'type{t}_fz{fz}_in{ine}_s{status}', lambda c, t=t, fz=fz, ine=ine, s=status: (
                    c.update(goal_type=t), c['goal'].update(status=s),
                    c['ego']['priorities'].__setitem__(t, record(multiplier=f32(1.5), bonus=f32(-20), minimum=f32(100),
                                                                fuzziness=f32(fz), inertia=f32(ine)))))
    # Multiplier/bonus/minimum interplay, including exact-minimum and zero results.
    for mult, bonus, minimum in [(1, 0, 0), (0, 0, 0), (2, -800, 0), (1, 0, 400), (1, -0.5, 400), (0.5, 200, 400),
                                 (-1, 1000, 0), (3, 1, 1201), (3, 1, 1200.99), (1.1, 0.1, 440.2)]:
        add(f'mbm_{mult}_{bonus}_{minimum}', lambda c, m=mult, b=bonus, n=minimum: c['ego']['priorities'].__setitem__(
            0, record(multiplier=f32(m), bonus=f32(b), minimum=f32(n))))
    # Affordability: reserve conditions and per-resource comparisons.
    for time, rtime in [(10, 30), (30, 30), (31, 30)]:
        for settlements, rmax in [(0, 2), (2, 2), (3, 2)]:
            for count in [0, 1]:
                for exempt in [False, True]:
                    add(f'reserve_t{time}_{rtime}_s{settlements}_{rmax}_c{count}_e{exempt}', lambda c, t=time, rt=rtime, s=settlements, rm=rmax, n=count, e=exempt: (
                        c.update(world_time=f32(t), settlements=s, sai_count=n, reserve_exempt=e),
                        c['ego'].update(reserved_time=f32(rt), reserved_max_settlements=rm, reserved_amount=f32(250))))
    for avail, cost, a0, b0 in [(300, 300, 0, 0), (299.99, 300, 0, 0), (0, 300, 120, 20), (0, 300, 120, 20.1),
                                (0, 300, 20, 120), (-50, -60, 0, 0), (1e-3, 0, 0, 0), (0, 0, 0, 1)]:
        for limit in [180, 1, 0]:
            add(f'afford_{avail}_{cost}_{a0}_{b0}_{limit}', lambda c, a=avail, k=cost, a0=a0, b0=b0, l=limit: (
                c['resources'][0].update(available=f32(a), word1a0=f32(a0), word1b8=f32(b0)),
                c['goal']['costs'].__setitem__(0, f32(k)), c['ego'].update(saving_time_limit=f32(l))))
    add('no_resources', lambda c: (c.update(resources=[]), c['goal'].update(costs=[])))
    add('debug_suppressed_flag', lambda c: c.update(debug_suppressed=1))
    # Base-priority virtual mutates fields read afterwards.
    add('base_changes_status_to_2', lambda c: c.update(mutations=[{'event': 'base_priority', 'goal': {'status': 2, 'fuzzy': f32(3.25)}}]))
    add('base_changes_status_negative', lambda c: c.update(mutations=[{'event': 'base_priority', 'goal': {'status': 0xffffffff}}]))
    add('type_call_changes_priority', lambda c: c.update(mutations=[{'event': 'goal_type', 'goal': {'priority': f32(1234.5)}}]))
    add('exempt_changes_costs', lambda c: (c.update(world_time=f32(0), reserve_exempt=False),
                                           c['ego'].update(reserved_time=f32(10), reserved_max_settlements=5, reserved_amount=f32(1)),
                                           c.update(mutations=[{'event': 'sai_count', 'goal': {'priority': f32(7)}}])))
    # RNG edge words: negative signed draws and draws rounding up to 1.0 at PC24.
    for seed in [0, 1, 0x7fffffff, 0x80000000, 0xffffffff, 0x8e4b12a5, 0xdeadbeef, 0x41c64e6d]:
        add(f'rng_{seed:08x}', lambda c, s=seed: c.update(rng_state=s))
    rnd = random.Random(0x7a5200)
    for i in range(48):
        def change(c, i=i, rnd=random.Random(rnd.random())):
            c['goal'].update(status=rnd.choice([0, 1, 2, 2, 3]), fuzzy=f32(rnd.uniform(-50, 300)),
                             priority=f32(rnd.uniform(-10, 10)), costs=[f32(rnd.uniform(0, 600)) for _ in range(3)])
            t = rnd.randrange(GOAL_TYPES)
            c.update(goal_type=t, base_priority=f32(rnd.uniform(-100, 3000)), rng_state=rnd.getrandbits(32),
                     draw=rnd.randrange(2), world_time=f32(rnd.uniform(0, 900)), settlements=rnd.randrange(5),
                     sai_count=rnd.randrange(3), reserve_exempt=bool(rnd.randrange(2)))
            c['ego'].update(default_fuzziness=f32(rnd.uniform(0, 400)), default_inertia=f32(rnd.uniform(0, 400)),
                            saving_time_limit=f32(rnd.uniform(0, 300)), reserved_amount=f32(rnd.uniform(0, 300)),
                            reserved_time=f32(rnd.uniform(0, 900)), reserved_max_settlements=rnd.randrange(5))
            c['ego']['priorities'][t] = record(multiplier=f32(rnd.uniform(-0.5, 3)), bonus=f32(rnd.uniform(-500, 500)),
                                               minimum=f32(rnd.uniform(-100, 600)), fuzziness=f32(rnd.uniform(-50, 300)),
                                               inertia=f32(rnd.uniform(-50, 400)))
            for r in c['resources']:
                r.update(word1a0=f32(rnd.uniform(-20, 80)), word1b8=f32(rnd.uniform(-20, 80)), available=f32(rnd.uniform(-10, 700)))
        add(f'random{i}', change)
    # Direct affordability predicate, both relaxed values.
    direct = []
    for c in rows:
        if c['name'].startswith(('afford_', 'reserve_', 'random', 'no_resources', 'exempt_changes')):
            for relaxed in [0, 1]:
                d = copy.deepcopy(c); d.update(entry='affordable', relaxed=relaxed, name=c['name'] + f'_direct_relaxed{relaxed}')
                d['mutations'] = [m for m in d.get('mutations', []) if m['event'] in ('sai_count', 'reserve_exempt')]
                direct.append(d)
    for p in PRECISIONS:
        for c in rows + direct:
            yield {**copy.deepcopy(c), 'name': c['name'] + '_' + p, 'precision': p}


def main():
    out, visited = [], {}
    for c in cases():
        try:
            m = GoalMachine(c); result = m.run()
        except Exception:
            print(c['name']); raise
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': result})
    d = header(__file__, __doc__)
    d.update(instructions=instructions(visited), cases=out)
    p = HERE / 'native-goal-priority.json'
    p.write_text(json.dumps(d, indent=1) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'instructions': len(visited), 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
