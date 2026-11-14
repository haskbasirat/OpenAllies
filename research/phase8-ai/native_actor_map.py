"""Original SAI actor-by-region list upkeep executing unchanged under Unicorn:
7A9940, 79E030, 7BD310, 7BD2A0, 7B99F0, 7BC330, 7BC470 and 7ADFF0, with the
list routines 4D0900 (push at the head) and 4F1430 (remove the first equal
node), 4CF1B0 and 4D03C0 native on real {head, tail} lists.

Seams, recorded in call order with their arguments (objects as fixture ids):
4CF010 (the region at the actor), 79F390, 7BCDB0, 7BB120 and the actor
virtual +108; the allocator 6F7810 (bump), 6F7430 (free) and the vector
reserve 4849F0 (reallocate and copy) are unrecorded. The fixture records
every list of every region table and owner, head first, after the call
(each tail pointer is checked) and every record's +14.

usage: native_actor_map.py [AA.exe]
The optional argument must be the same original image the shared machine
checks (its SHA-256 is asserted).
"""
from pathlib import Path
import json, random, sys

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import native_machine as nm
from native_machine import Machine, PRECISIONS, header, instructions, sha

OPS = {'remove_from_region': (0x7a9940, False), 'refresh': (0x79e030, False), 'region_move': (0x7bd310, True),
       'remove_28': (0x7bd2a0, True), 'remove_34': (0x7b99f0, True), 'owner_list': (0x7adff0, True),
       'add_28': (0x7bc330, True), 'add_34': (0x7bc470, True)}
RANGES = {'7A9940': (0x7a9940, 0x7a9999), '79E030': (0x79e030, 0x79e077), '7BD310': (0x7bd310, 0x7bd359),
          '7BD2A0': (0x7bd2a0, 0x7bd305), '7B99F0': (0x7b99f0, 0x7b9a5c), '7ADFF0': (0x7adff0, 0x7ae06d),
          '7BC330': (0x7bc330, 0x7bc46a), '7BC470': (0x7bc470, 0x7bc58a)}


class MapMachine(Machine):
    def __init__(self, c):
        super().__init__(c['precision'])
        self.c = c
        self.ids, self.addr, self.lists = {}, {}, {}
        self.seam(0x6f7810, lambda m: self.ret(self.alloc(max(self.arg(1), 4))))
        self.seam(0x6f7430, lambda m: self.ret(0))
        for key, k in c['kingdoms'].items():
            p = self.obj(key, 0x200)
            self.w(p + 0x188, k['index'])
        for key, a in c['actors'].items():
            p = self.obj(key, 0x200)
            vt = self.alloc(0x200)
            self.w(p, vt)
            kingdom = a['kingdom']
            self.w(vt + 0x108, self.stub(lambda m, kingdom=kingdom: (self.ev('kingdom', self.id(self.ecx())),
                                                                     self.ret(self.ref(kingdom)))))
            self.w(p + 0x14, a['handle'])
            self.w(p + 0x140, a['word_140'])
        for key, o in c['owners'].items():
            p = self.obj(key, 0x80)
            self.make_list('owner:' + key, o['list'], p + 0x4c)
        self.records = {}
        for key, a in c['actors'].items():
            if a['record'] is not None:
                rec = self.records[key] = self.alloc(0x40)
                self.w(rec + 0xc, self.ref(a['record']))
                self.w(rec + 0x14, 0x12345678)
                self.w(self.addr[key] + 0x154, rec)
        for key in c['watchers']:
            self.obj(key, 0x40)
        for key, r in c['regions'].items():
            p = self.obj(key, 0x80)
            self.w(p + 0x1c, self.array([self.ref(w) for w in r['watchers']]))
            self.w(p + 0x20, len(r['watchers']))
            for table, off in (('at28', 0x28), ('at34', 0x34)):
                ptrs = []
                for i, values in enumerate(r[table]):
                    if values is None:
                        ptrs.append(0)
                    else:
                        lp = self.alloc(8)
                        self.make_list('%s:%s:%d' % (key, table, i), values, lp)
                        ptrs.append(lp)
                spare, null_data = r['spare'][table]
                if null_data and not ptrs:
                    self.w(p + off, 0)
                else:
                    self.w(p + off, self.array(ptrs + [0] * spare))
                self.w(p + off + 4, len(r[table]))
                self.w(p + off + 8, len(r[table]) + spare)
        self.seam(0x4cf010, lambda m: (self.ev('region_of', self.id(self.ecx())),
                                       self.ret(self.ref(c['region_of'].get(self.id(self.ecx()))))))
        self.seam(0x79f390, lambda m: (self.ev('test_79f390', self.id(self.arg(1))),
                                       self.ret(int(c['test_79f390']))))
        self.seam(0x7bcdb0, lambda m: (self.ev('watcher_update', self.id(self.ecx()), self.id(self.arg(1))),
                                       self.ret(0, 4)))
        self.seam(0x7bb120, lambda m: (self.ev('watcher_added', self.id(self.ecx()), self.id(self.arg(1))),
                                       self.ret(0, 4)))

        def reserve(m):
            v, n = self.ecx(), self.arg(1)
            old, count = self.r(v), self.r(v + 4)
            new = self.alloc(4 * max(n, 1))
            for i in range(count):
                self.w(new + 4 * i, self.r(old + 4 * i))
            self.w(v, new)
            self.w(v + 8, n)
            self.ret(0, 4)
        self.seam(0x4849f0, reserve)

    def make_list(self, key, values, at):
        nodes = [self.alloc(8) for _ in values]
        for i, (n, v) in enumerate(zip(nodes, values)):
            self.w(n, v)
            self.w(n + 4, nodes[i + 1] if i + 1 < len(nodes) else 0)
        self.w(at, nodes[0] if nodes else 0)
        self.w(at + 4, nodes[-1] if nodes else 0)
        self.lists[key] = at

    def walk(self, at, key):
        values, n, last = [], self.r(at), 0
        while n:
            values.append(self.r(n))
            last, n = n, self.r(n + 4)
        assert self.r(at + 4) == last, key
        return values

    def dump(self):
        out = {}
        for key in self.c['owners']:
            out['owner:' + key] = self.walk(self.addr[key] + 0x4c, key)
        for key in self.c['regions']:
            p = self.addr[key]
            for table, off in (('at28', 0x28), ('at34', 0x34)):
                for i in range(self.r(p + off + 4)):
                    lp = self.r(self.r(p + off) + 4 * i)
                    if lp:
                        out['%s:%s:%d' % (key, table, i)] = self.walk(lp, key)
        return out

    def obj(self, key, size):
        p = self.alloc(size)
        self.ids[p] = key
        self.addr[key] = p
        return p

    def ref(self, key):
        return 0 if key is None else self.addr[key]

    def id(self, a):
        return self.ids.get(a, a)

    def ev(self, name, *args):
        self.events.append([name] + list(args))

    def run(self):
        c = self.c
        entry, this = OPS[c['operation']]
        a = c['args']
        args = [self.ref(a[0]), self.ref(a[1]) if isinstance(a[1], str) or a[1] is None else a[1]]
        self.call(entry, self.ref(c['this']) if this else 0, args)
        return {'events': self.events, 'lists': self.dump(),
                'records': {k: self.r(r + 0x14) for k, r in self.records.items()}}


def cases():
    rnd = random.Random(0x7a9940)
    rows = []
    ops = list(OPS)
    for i in range(1500):
        op = ops[i % len(ops)]
        kingdoms = {'k%d' % j: {'index': j} for j in range(4)}
        kingdoms['k_far'] = {'index': 9}
        actors = {}
        handles = []
        for j in range(4):
            h = (0x10 + j) << 16 | (0x200 + j)
            handles.append(h)
            actors['a%d' % j] = {'handle': h, 'word_140': rnd.choice([0, 0, 0, 0, 1]),
                                 'kingdom': rnd.choice(['k0', 'k1', 'k2', 'k3', 'k_far', None]),
                                 'record': None}
        owners = {'o0': {'list': [rnd.choice(handles + [0, 0x99]) for _ in range(rnd.randrange(0, 5))]}}
        actors['a0']['record'] = rnd.choice(['o0', 'o0', None])
        watchers = ['w%d' % j for j in range(3)]

        def lists(n, nulls):
            return [None if nulls and rnd.random() < 0.2 else
                    [rnd.choice(handles + [0x77]) for _ in range(rnd.randrange(0, 5))] for _ in range(n)]
        nulls = op == 'remove_34'
        regions = {'r0': {'watchers': rnd.sample(watchers, rnd.randrange(0, 4)),
                          'at28': lists(rnd.randrange(0, 5), nulls), 'at34': lists(rnd.randrange(0, 5), False),
                          'spare': {t: [rnd.choice([0, 0, 1, 3]), rnd.random() < 0.5] for t in ('at28', 'at34')}}}
        region_of = {'a0': 'r0'} if op == 'remove_from_region' or rnd.random() < 0.8 else {}
        actor = rnd.choice(['a0', 'a0', 'a1'])
        if op == 'remove_from_region':
            region_of[actor] = 'r0'
        flag = rnd.choice([0, 1, 0x100, 0x101])
        kingdom_arg = rnd.choice(['k0', 'k1', 'k3', 'k_far', None])
        this = {'region_move': 'r0', 'remove_28': 'r0', 'remove_34': 'r0', 'owner_list': 'o0', 'add_28': 'r0',
                'add_34': 'r0'}.get(op)
        if op in ('remove_28', 'remove_34', 'remove_from_region', 'add_28', 'add_34'):
            args = [actor if not (op == 'remove_28' and rnd.random() < 0.1) else None,
                    rnd.choice(['k0', 'k1', 'k3', 'k_far', 'k_far', None]) if op.startswith('add') else kingdom_arg]
        elif op == 'owner_list':
            args = [actor if rnd.random() < 0.9 else None, flag]
        else:
            args = [actor, flag]
        c = {'name': '%s%d' % (op, i), 'precision': list(PRECISIONS)[i % 3], 'operation': op, 'this': this,
             'args': args, 'kingdoms': kingdoms, 'actors': actors, 'owners': owners, 'regions': regions,
             'watchers': watchers, 'region_of': region_of, 'test_79f390': rnd.random() < 0.5}
        rows.append(c)
    return rows


def main():
    if len(sys.argv) > 1:
        assert sha(Path(sys.argv[1]).read_bytes()) == sha(nm.original.b), 'not the original image'
    out, visited = [], {}
    for c in cases():
        try:
            m = MapMachine(c)
            result = m.run()
        except Exception:
            print(c['name'])
            raise
        m.verify_v14()
        visited.update(m.visited)
        out.append({'input': c, 'output': result})
    d = header(__file__, __doc__)
    d.update(instructions=instructions(visited),
             coverage={k: len([a for a in visited if lo <= a < hi]) for k, (lo, hi) in RANGES.items()},
             cases=out)
    p = HERE / 'native-actor-map.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'coverage': d['coverage'], 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
