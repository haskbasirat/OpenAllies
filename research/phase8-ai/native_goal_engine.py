"""Original SAI goal-engine passes 7A7700 (list maintenance) and 7A93B0
(priority-ordered selection with resource reservation).

Both execute unchanged on an emulated controller (+4 player whose +8 economy
holds the resource array at +1C4/+1C8, +8 array of 23 bucket pointers with
three {head, tail} goal lists, +14/+18/+1C goal array) including the CRT
qsort 96CE20 with comparator 7A30B0, array growth 4849F0, list compaction
7A32F0/7A3140, the eh vector iterators and the temporary list destructor.

Seams (recorded in order): goal virtuals +0 (destroy), +4 (type), +14
(expired), +60 (re-evaluate), +80 (deselect); 7A9150 and 7A7E30 (goal
evaluate/assign, recorded with the running resource array, goal order and
index; they apply case-scripted field changes); fiber yield 79DF70;
allocator 6F7810/6F7430. Scripted effects let the seams change status,
current/base priority and costs exactly where the real goals would.
"""
from pathlib import Path
import copy, json, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import Machine, PRECISIONS, f32, header, instructions, sha

BUCKETS = 23


class EngineMachine(Machine):
    def __init__(self, c):
        super().__init__(c['precision'])
        self.c = copy.deepcopy(c)
        self.effects = self.c.get('effects', [])
        sai = self.alloc(0x100); self.w(0xb36528, sai); self.sai = sai
        self.seam(0x79df70, self.s_yield)
        self.seam(0x6f7810, lambda m: self.ret(self.alloc(max(self.arg(1), 4))))
        self.seam(0x6f7430, lambda m: self.ret(0))
        econ = self.alloc(0x200)
        self.w(econ + 0x1c4, self.array(c['economy'])); self.w(econ + 0x1c8, len(c['economy'])); self.w(econ + 0x1cc, len(c['economy']))
        player = self.alloc(0x40); self.w(player + 8, econ)
        ctl = self.ctl = self.alloc(0x40); self.w(ctl + 4, player)
        self.buckets = [self.alloc(0x18) for _ in range(BUCKETS)]
        self.w(ctl + 8, self.array(self.buckets))
        vt = self.alloc(0x100)
        self.w(vt + 0, self.stub(self.v_destroy)); self.w(vt + 4, self.stub(self.v_type))
        self.w(vt + 0x14, self.stub(self.v_expired)); self.w(vt + 0x60, self.stub(self.v_reevaluate))
        self.w(vt + 0x80, self.stub(self.v_deselect))
        self.goals = {}
        for gid, g in c['goals'].items():
            p = self.alloc(0x60); self.w(p, vt); self.goals[int(gid)] = p
        self.ids = {p: gid for gid, p in self.goals.items()}
        self.write_goals()
        for b, lists in enumerate(c['buckets']):
            for l, items in enumerate(lists):
                head = tail = 0
                for gid in reversed(items):
                    node = self.alloc(8); self.w(node, 0 if gid is None else self.goals[gid]); self.w(node + 4, head); head = node
                    if not tail:
                        tail = node
                self.w(self.buckets[b] + 8 * l, head); self.w(self.buckets[b] + 8 * l + 4, tail)
        self.seam(0x7a9150, lambda m: self.s_pass('evaluate'))
        self.seam(0x7a7e30, lambda m: self.s_pass('assign'))
        self.destroyed = []

    def write_goals(self):
        for gid, g in self.c['goals'].items():
            p = self.goals[int(gid)]
            self.w(p + 8, g['status']); self.w(p + 0x34, g['current']); self.w(p + 0x38, g['base'])
            self.w(p + 0x1c, self.array(g['costs'])); self.w(p + 0x20, len(g['costs']))

    def read_goal(self, gid):
        p = self.goals[gid]
        n = self.r(p + 0x20)
        return {'status': self.r(p + 8), 'current': self.r(p + 0x34), 'base': self.r(p + 0x38),
                'costs': [self.r(self.r(p + 0x1c) + 4 * i) for i in range(n)]}

    def snapshot_goals(self):
        return {str(g): self.read_goal(g) for g in sorted(self.goals) if g not in self.destroyed}

    def lists(self):
        out = []
        for b in self.buckets:
            row = []
            for l in range(3):
                items, node = [], self.r(b + 8 * l)
                while node:
                    g = self.r(node); items.append(None if g == 0 else self.ids[g]); node = self.r(node + 4)
                row.append(items)
            out.append(row)
        return out

    def event(self, name, gid, extra=None):
        self.events.append({'event': name, 'goal': gid, **(extra or {})})
        for e in self.effects:
            if not e.get('used') and e['event'] == name and e['goal'] == gid:
                e['used'] = True
                g = self.read_goal(gid); g.update(e['set'])
                self.c['goals'][str(gid)] = g
                p = self.goals[gid]
                self.w(p + 8, g['status']); self.w(p + 0x34, g['current']); self.w(p + 0x38, g['base'])
                self.w(p + 0x1c, self.array(g['costs'])); self.w(p + 0x20, len(g['costs']))
                break

    def gid(self):
        return self.ids[self.ecx()]

    def v_destroy(self, m):
        g = self.gid(); assert self.arg(1) == 1
        self.event('destroy', g); self.destroyed.append(g); self.ret(0, 4)

    def v_type(self, m):
        g = self.gid(); t = self.c['types'][str(g)]
        self.event('type', g, {'result': t}); self.ret(t)

    def v_expired(self, m):
        g = self.gid(); r = g in self.c.get('expired', [])
        self.event('expired', g, {'result': r}); self.ret(int(r))

    def v_reevaluate(self, m):
        g = self.gid()
        arr = self.arg(1)
        assert arr == self.r(self.r(self.ctl + 4) + 8) + 0x1c4
        self.event('reevaluate', g); self.ret(0, 4)

    def v_deselect(self, m):
        self.event('deselect', self.gid()); self.ret(0)

    def s_pass(self, name):
        g = self.gid()
        res, arr, index = self.arg(1), self.arg(2), self.arg(3)
        running = [self.r(self.r(res) + 4 * i) for i in range(self.r(res + 4))]
        order = [self.ids[self.r(self.r(arr) + 4 * i)] for i in range(self.r(arr + 4))]
        assert arr == self.ctl + 0x14
        self.event(name, g, {'running': running, 'order': order, 'index': index})
        self.ret(0, 12)

    def s_yield(self, m):
        assert self.ecx() == self.sai and self.arg(1) == 0
        self.events.append({'event': 'yield'}); self.ret(0, 4)

    def run(self):
        entry = {'reevaluate': 0x7a7700, 'select': 0x7a93b0}[self.c['entry']]
        from unicorn import UcError
        try:
            self.call(entry, ecx=self.ctl)
        except (AssertionError, UcError) as e:
            if not self.null_accesses and not isinstance(e, UcError):
                raise
            return {'fault': 'null dereference', 'events': self.events}
        return {'events': self.events, 'lists': self.lists(), 'goals': self.snapshot_goals(),
                'destroyed': self.destroyed, 'array_count': self.r(self.ctl + 0x18)}


def make_case(rnd, name, entry, n_goals=12, n_res=3, special=None):
    goals, types = {}, {}
    buckets = [[[], [], []] for _ in range(BUCKETS)]
    for gid in range(1, n_goals + 1):
        status = rnd.choice([0, 1, 1, 2, 2])
        t = rnd.choice([0, 1, 2, 4, 7, 12, 12, 21, 22])
        goals[str(gid)] = {'status': status, 'current': f32(rnd.choice([0, 5, 10, 10, 25, 40, -3, rnd.uniform(-10, 60)])),
                           'base': f32(rnd.choice([0, 5, 10, 10, 25, 40, -3, rnd.uniform(-10, 60)])),
                           'costs': [f32(rnd.choice([0, 10, 25, rnd.uniform(0, 100)])) for _ in range(rnd.randrange(0, n_res + 1))]}
        types[str(gid)] = t
        buckets[t][status].append(gid)
    for row in buckets:
        for l in row:
            rnd.shuffle(l)
    effects = []
    for gid in range(1, n_goals + 1):
        for ev in (['evaluate', 'assign'] if entry == 'select' else ['reevaluate', 'deselect']):
            if rnd.random() < 0.6:
                s = {}
                if rnd.random() < 0.7:
                    s['status'] = rnd.choice([0, 1, 2, 2, 2])
                if rnd.random() < 0.6:
                    s['current'] = f32(rnd.choice([0, 7, 15, 30, rnd.uniform(-20, 80)]))
                if rnd.random() < 0.3:
                    s['base'] = f32(rnd.choice([0, -1, 20, rnd.uniform(-20, 80)]))
                effects.append({'event': ev, 'goal': gid, 'set': s})
    c = {'name': name, 'entry': entry, 'precision': 'Single24', 'goals': goals, 'types': types, 'buckets': buckets,
         'economy': [f32(rnd.uniform(0, 300)) for _ in range(n_res)], 'effects': effects,
         'expired': [g for g in range(1, n_goals + 1) if rnd.random() < 0.15] if entry == 'reevaluate' else []}
    if special:
        special(c)
    return c


def cases():
    rnd = random.Random(0x7a93b0)
    rows = []
    rows.append(make_case(rnd, 'select_empty', 'select', 0))
    rows.append(make_case(rnd, 'reevaluate_empty', 'reevaluate', 0))
    for i in range(40):
        rows.append(make_case(rnd, f'select{i}', 'select', rnd.randrange(1, 25)))
    for i in range(40):
        rows.append(make_case(rnd, f'reevaluate{i}', 'reevaluate', rnd.randrange(1, 25)))
    # Equal priorities (qsort tie order) and a cost-heavy chain.
    def ties(c):
        for g in c['goals'].values():
            g['base'] = f32(10)
    rows.append(make_case(rnd, 'select_ties', 'select', 16, special=ties))
    def null_node(c):
        c['buckets'][3][1].insert(0, None)
    rows.append(make_case(rnd, 'select_null_node', 'select', 5, special=null_node))
    rows.append(make_case(rnd, 'reevaluate_null_node', 'reevaluate', 5, special=null_node))
    out = []
    for p in PRECISIONS:
        for c in rows:
            out.append({**copy.deepcopy(c), 'name': c['name'] + '_' + p, 'precision': p})
    return out


def main():
    out, visited = [], {}
    for c in cases():
        try:
            m = EngineMachine(c); result = m.run()
        except Exception:
            print(c['name']); raise
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': result})
    d = header(__file__, __doc__)
    d.update(instructions=instructions(visited), cases=out)
    p = HERE / 'native-goal-engine.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'faults': sum('fault' in r['output'] for r in out),
                      'instructions': len(visited), 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
