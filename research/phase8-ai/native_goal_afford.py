"""Original SAI goal affordability 7A3860 over whole cost vectors, with
non-finite values.

7A3860 executes unchanged (seams as in native_goal_priority.py; the settlement
reserve is off, its maximum 0 as in all shipped egos). Each case has a
resource registry of 1 to 7 entries (word +18: 0 stockpiled, 1 upkeep, 2
limited), the kingdom's +1A0/+1B8 books and stock for each, and a goal cost
vector (count +20) no longer than the registry. Values include NaN and
infinities in the costs, the stock, the books and saving_time_limit, which
the x87 compares as unordered or ordered exactly as the original does. Each
case records the result byte for relaxed 0 and 1.
"""
from pathlib import Path
import copy, json, random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import f32, header, instructions, sha
from native_goal_priority import GoalMachine, record, GOAL_TYPES

NAN, INF, NINF = 0x7fc00000, 0x7f800000, 0xff800000


def cases():
    pri = [record() for _ in range(GOAL_TYPES)]
    base = {
        'name': 'base', 'entry': 'affordable', 'precision': 'Double53', 'draw': 0, 'relaxed': 0,
        'world_time': f32(120.0), 'debug_suppressed': 0, 'rng_state': 0x12345678,
        'resources': [],
        'ego': {'default_fuzziness': f32(200), 'default_inertia': f32(250), 'priorities': pri,
                'saving_time_limit': f32(180), 'reserved_amount': f32(0), 'reserved_time': f32(0),
                'reserved_max_settlements': 0},
        'settlements': 0, 'sai_count': 1, 'reserve_exempt': False, 'goal_type': 0, 'base_priority': f32(400),
        'goal': {'status': 2, 'fuzzy': f32(0), 'inertia_applied': f32(0), 'priority': f32(0), 'costs': []},
    }
    rows = []

    def res(word18, provided, consumed, available):
        return {'word18': word18, 'word1a0': provided, 'word1b8': consumed, 'available': available}

    def add(name, resources, costs, limit=f32(180)):
        for relaxed in (0, 1):
            c = copy.deepcopy(base)
            c.update(name=f'{name}_r{relaxed}', relaxed=relaxed, resources=resources)
            c['goal']['costs'] = costs
            c['ego']['saving_time_limit'] = limit
            rows.append(c)

    money = lambda a, p=f32(30), q=f32(10): res(0, p, q, a)
    upkeep = lambda a=f32(0): res(1, f32(5), f32(8), a)
    # Unordered and infinite values, one stockpiled resource.
    for label, cost in [('nan', NAN), ('inf', INF), ('ninf', NINF), ('zero', 0), ('300', f32(300))]:
        add(f'cost_{label}', [money(f32(100))], [cost])
        add(f'cost_{label}_stock_nan', [money(NAN)], [cost])
        add(f'cost_{label}_stock_inf', [money(INF)], [cost])
        add(f'cost_{label}_provided_nan', [money(f32(100), NAN)], [cost])
        add(f'cost_{label}_consumed_inf', [money(f32(100), f32(30), INF)], [cost])
        add(f'cost_{label}_provided_inf', [money(f32(100), INF)], [cost])
        add(f'cost_{label}_limit_nan', [money(f32(100))], [cost], NAN)
        add(f'cost_{label}_limit_inf', [money(f32(100))], [cost], INF)
        add(f'cost_{label}_limit_inf_flat', [money(f32(100), f32(10), f32(10))], [cost], INF)
    # Whole vectors: the registry of resources.tgi (money purchase, the rest
    # upkeep or limited), a cost shorter than the registry, two stockpiled.
    reg7 = lambda m: [money(m), upkeep(f32(50)), upkeep(), res(2, f32(40), f32(39), f32(0)), upkeep(), res(2, 0, 0, 0), upkeep()]
    add('reg7_fits', reg7(f32(500)), [f32(300), f32(1e9), f32(1e9), f32(1e9), 0, f32(5), 0])
    add('reg7_short', reg7(f32(500)), [f32(600), f32(1e9)])
    add('reg7_rate', reg7(f32(250)), [f32(300), 0, 0, 0, 0, 0, 0])
    add('reg7_empty', reg7(f32(0)), [])
    add('two_stockpiled', [money(f32(10)), upkeep(), res(0, f32(1), f32(0), f32(20))], [f32(5), f32(9e9), f32(25)])
    add('two_stockpiled_nan', [money(f32(10)), upkeep(), res(0, f32(1), f32(0), f32(20))], [NAN, f32(9e9), f32(25)])
    add('negative_stock', [money(f32(-50), f32(0), f32(0))], [0])
    add('negative_stock_rate', [money(f32(-50), f32(1), f32(0))], [0])
    rnd = random.Random(0x7a3860)
    pool = [NAN, INF, NINF, 0, 0x80000000]
    for i in range(60):
        n = rnd.randrange(1, 8)
        v = lambda lo, hi: rnd.choice(pool) if rnd.random() < 0.12 else f32(rnd.uniform(lo, hi))
        resources = [res(rnd.choice([0, 0, 1, 2]), v(-20, 80), v(-20, 80), v(-50, 700)) for _ in range(n)]
        costs = [rnd.choice([0, v(0, 600)]) for _ in range(rnd.randrange(0, n + 1))]
        add(f'random{i}', resources, costs, v(0, 300))
    return rows


def main():
    out, visited = [], {}
    for c in cases():
        m = GoalMachine(c); result = m.run()
        m.verify_v14(); visited.update(m.visited)
        out.append({'name': c['name'], 'relaxed': c['relaxed'], 'saving_time_limit': c['ego']['saving_time_limit'],
                    'resources': c['resources'], 'costs': c['goal']['costs'], 'affordable': result['affordable']})
    d = header(__file__, __doc__)
    d.update(instructions=instructions(visited), cases=out)
    p = HERE / 'native-goal-afford.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'affordable': sum(c['affordable'] for c in out),
                      'instructions': len(visited), 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
