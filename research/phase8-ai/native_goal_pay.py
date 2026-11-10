"""Original SAI goal payment 7A39C0 (with 7A3980 and 7A3860 executing unchanged).

Seams as in native_goal_priority.py. Each case records the result byte, the
running resource vector after the call and the engine +0x20 saving byte. The
Rust table aa-play ai::goals::tests::PAY_7A39C0 is these cases, in order.
"""
from pathlib import Path
import copy, json, random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import f32, header, instructions, sha
from native_goal_priority import GoalMachine, record, GOAL_TYPES


class PayMachine(GoalMachine):
    def __init__(self, c):
        super().__init__(c)
        self.engine = self.r(self.goal + 4)
        self.wb(self.engine + 0x20, c['saving'])

    def run(self):
        result = self.call(0x7a39c0, ecx=self.goal, args=[self.avail]) & 0xff
        arr = self.r(self.avail)
        n = len(self.c['resources'])
        return {'paid': result, 'saving': self.rb(self.engine + 0x20),
                'available': [self.r(arr + 4 * i) for i in range(n)]}


def cases():
    pri = [record() for _ in range(GOAL_TYPES)]
    base = {
        'name': 'base', 'entry': 'pay', 'precision': 'Double53', 'draw': 0, 'relaxed': 0,
        'world_time': f32(120.0), 'debug_suppressed': 0, 'rng_state': 0x12345678, 'saving': 0,
        'resources': [{'word18': 0, 'word1a0': f32(30), 'word1b8': f32(10), 'available': f32(500)},
                      {'word18': 1, 'word1a0': f32(5), 'word1b8': f32(8), 'available': f32(100)},
                      {'word18': 1, 'word1a0': f32(0), 'word1b8': f32(0), 'available': f32(0)}],
        'ego': {'default_fuzziness': f32(200), 'default_inertia': f32(250), 'priorities': pri,
                'saving_time_limit': f32(180), 'reserved_amount': f32(0), 'reserved_time': f32(0),
                'reserved_max_settlements': 0},
        'settlements': 0, 'sai_count': 1, 'reserve_exempt': False, 'goal_type': 0, 'base_priority': f32(400),
        'goal': {'status': 2, 'fuzzy': f32(0), 'inertia_applied': f32(0), 'priority': f32(0),
                 'costs': [f32(300), f32(50), f32(1e9)]},
    }
    rows = []

    def add(name, change):
        c = copy.deepcopy(base); c['name'] = name; change(c); rows.append(c)

    for saving in [0, 1]:
        for avail, cost, a0, b0, limit in [(500, 300, 30, 10, 180), (300, 300, 0, 0, 180), (299.99, 300, 0, 0, 180),
                                           (0, 300, 120, 20, 180), (0, 300, 120, 20.1, 180), (0, 300, 20, 120, 180),
                                           (100, 0, 0, 0, 180), (0, 0, 0, 1, 180), (250, 300, 17, 0, 180),
                                           (250, 300, 16.6666, 0, 180), (250, 300, 50, 33.3333, 180), (0.1, 0.2, 0, 0, 0),
                                           (1234.567, 1234.5, 0, 0, 60), (16.7, 100, 27.77, 0, 180), (90, 100, 3.3333335, 0, 180)]:
            add(f'pay_s{saving}_{avail}_{cost}_{a0}_{b0}_{limit}', lambda c, s=saving, a=avail, k=cost, a0=a0, b0=b0, l=limit: (
                c.update(saving=s),
                c['resources'][0].update(available=f32(a), word1a0=f32(a0), word1b8=f32(b0)),
                c['goal']['costs'].__setitem__(0, f32(k)), c['ego'].update(saving_time_limit=f32(l))))
    # An upkeep cost is neither tested nor paid; a stockpiled second slot is.
    for word18 in [0, 1, 2]:
        for saving in [0, 1]:
            add(f'second_w{word18}_s{saving}', lambda c, w=word18, s=saving: (
                c.update(saving=s), c['resources'][1].update(word18=w, available=f32(40)),
                c['goal']['costs'].__setitem__(1, f32(50))))
    rnd = random.Random(0x7a39c0)
    for i in range(40):
        def change(c, rnd=random.Random(rnd.random())):
            c.update(saving=rnd.randrange(2))
            c['goal']['costs'] = [f32(rnd.choice([0.0, rnd.uniform(0, 600)])) for _ in range(3)]
            c['ego'].update(saving_time_limit=f32(rnd.uniform(0, 300)))
            for r in c['resources']:
                r.update(word18=rnd.choice([0, 0, 1, 2]), word1a0=f32(rnd.uniform(-20, 80)), word1b8=f32(rnd.uniform(-20, 80)),
                         available=f32(rnd.uniform(0, 700)))
        add(f'random{i}', change)
    for c in rows:
        yield c


def main():
    out, visited = [], {}
    for c in cases():
        m = PayMachine(c); result = m.run()
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': {'name': c['name'], 'saving': c['saving'],
                              'saving_time_limit': c['ego']['saving_time_limit'],
                              'resources': c['resources'], 'costs': c['goal']['costs']}, 'output': result})
    d = header(__file__, __doc__)
    d.update(instructions=instructions(visited), cases=out)
    p = HERE / 'native-goal-pay.json'
    p.write_text(json.dumps(d, indent=1) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'instructions': len(visited), 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
