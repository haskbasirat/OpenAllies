"""Original CAI engage target list: 521060, 523DD0, 524C10, 51EFB0, 51F040
(+5C2CD0), 523EA0 and the sorted insertion 51FE60, plus 52A000.

All execute unchanged on an emulated regiment Mind (engage state at +1CC,
chasing state at +120 with its target word at +130, organization actor at
+4) and emulated target actors resolved through the GTID registry.

Seams (recorded in order): Mind virtuals +A0 (can target), +F0 (engage),
+1AC (priority); target virtual +130; attacker registration 4D3990 and
unregistration 4D0CD0; organization relation 4D4FF0; 4D4420; component
57A3C0, 579BA0 and 57A500; allocator 6F7810/6F7430; diagnostic log 404F90.
"""
from pathlib import Path
import copy, json, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import Machine, header, instructions, sha


class EngageMachine(Machine):
    def __init__(self, c):
        super().__init__('Extended64')
        self.c = copy.deepcopy(c)
        self.seam(0x6f7810, lambda m: self.ret(self.alloc(max(self.arg(1), 4))))
        self.seam(0x6f7430, lambda m: self.ret(0))
        self.seam(0x404f90, lambda m: self.ret(0))
        reg = self.alloc(0x20004 + 0x10000 * 4 + 16); self.w(0xb364c0, reg); self.reg = reg
        org = self.org = self.alloc(0x200)
        self.w(org + 0x14, c['org_gtid'])
        self.comp = 0
        if c['component']:
            self.comp = self.alloc(0x40); self.w(org + 0x7c, self.comp)
        entries = self.alloc(0x40)
        for i, e in enumerate(c['org_entries']):
            self.w(entries + 4 * i, e)
        self.w(org + 0x148, entries); self.w(org + 0x14c, len(c['org_entries']))
        mind = self.mind = self.alloc(0x300)
        mvt = self.alloc(0x200); self.w(mind, mvt)
        self.w(mvt + 0xa0, self.stub(lambda m: self.v_bool('can_target', 'can_target', 4)))
        self.w(mvt + 0xf0, self.stub(lambda m: self.v_bool('engage', 'engage', 4)))
        self.w(mvt + 0x1ac, self.stub(self.v_priority))
        self.w(mind + 4, org)
        self.engage = mind + 0x1cc; self.chasing = mind + 0x120
        for st in (self.engage, self.chasing):
            self.w(st + 4, mind)
        other = self.alloc(0x40)
        head = 0
        cur = {'engage': self.engage, 'chasing': self.chasing, 'other': other, None: 0}[c['current']]
        if c['current'] is not None:
            head = self.alloc(8); self.w(head, cur)
        self.w(mind + 0x10, head)
        self.w(mind + 0x130, c['chasing_target'])
        # Targets.
        tvt = self.alloc(0x200); self.w(tvt + 0x130, self.stub(self.v_check130))
        self.actors = {}
        for key, a in c['actors'].items():
            p = self.alloc(0x100); self.w(p, tvt)
            self.w(p + 8, a.get('flags', 0)); self.w(p + 0x14, a['gtid']); self.w(p + 0x90, a.get('word90', 0))
            self.actors[key] = p
            if a.get('registered', True):
                idx, gen = a['gtid'] & 0xffff, a['gtid'] >> 16
                self.u.mem_write(reg + 4 + 2 * idx, struct.pack('<H', gen)); self.w(reg + 0x20004 + 4 * idx, p)
        for key, a in c['actors'].items():
            if a.get('membership') is not None:
                ms = a['membership']
                mp = self.alloc(0x40); self.w(self.actors[key] + 0x80, mp)
                self.w(mp + 0x10, 0 if ms['org'] is None else self.actors[ms['org']]); self.wb(mp + 0x1c, ms['attached'])
        self.names = {p: k for k, p in self.actors.items()}
        self.names[org] = 'org'
        # Engage list.
        prev = 0
        nodes = []
        for n in c['list']:
            node = self.alloc(0x10); self.w(node, n['gtid']); self.w(node + 0xc, n['priority'] & 0xffffffff)
            self.w(node + 4, prev)
            if prev:
                self.w(prev + 8, node)
            prev = node; nodes.append(node)
        self.w(self.engage + 0x10, nodes[0] if nodes else 0); self.w(self.engage + 0x14, nodes[-1] if nodes else 0)
        self.nodes = nodes
        self.seam(0x4d3990, lambda m: self.s_actor_arg('register_attacker', 4))
        self.seam(0x4d0cd0, lambda m: self.s_actor_arg('unregister_attacker', 4))
        self.seam(0x57a3c0, self.s_comp_added)
        self.seam(0x579ba0, self.s_comp_update)
        self.seam(0x4d4ff0, self.s_relation)
        self.seam(0x4d4420, self.s_related)
        self.seam(0x57a500, self.s_accepts)

    def name(self, p):
        return self.names.get(p, hex(p))

    def lookup(self, table, key, default):
        return self.c.get(table, {}).get(key, default)

    def v_bool(self, table, event, pop):
        assert self.ecx() == self.mind
        t = self.name(self.arg(1)); r = self.lookup(table, t, True)
        self.events.append({'event': event, 'target': t, 'result': r}); self.ret(int(r), pop)

    def v_priority(self, m):
        assert self.ecx() == self.mind
        t = self.name(self.arg(1)); r = self.lookup('priority', t, 0)
        self.events.append({'event': 'priority', 'target': t, 'result': r}); self.ret(r & 0xffffffff, 4)

    def v_check130(self, m):
        t = self.name(self.ecx()); assert self.arg(2) == 1
        r = self.lookup('check130', t, False)
        self.events.append({'event': 'check130', 'target': t, 'entry': self.arg(1), 'result': r}); self.ret(int(r), 8)

    def s_actor_arg(self, event, pop):
        assert self.arg(1) == self.org
        self.events.append({'event': event, 'target': self.name(self.ecx())}); self.ret(0, pop)

    def s_comp_added(self, m):
        assert self.ecx() == self.comp
        self.events.append({'event': 'component_target_added', 'target': self.name(self.arg(1))}); self.ret(0, 4)

    def s_comp_update(self, m):
        if self.ecx() == 0:
            # 579BA0 dereferences its null component: native fault.
            self.null_accesses.append((0x579ba0, 0, 4)); self.u.emu_stop(); return
        assert self.ecx() == self.comp
        self.events.append({'event': 'component_update'}); self.ret(0)

    def s_relation(self, m):
        assert self.ecx() == self.org and self.arg(2) == 1 and self.arg(3) == 0
        t = self.name(self.arg(1)); r = self.lookup('relation', t, 0)
        self.events.append({'event': 'relation', 'target': t, 'result': r}); self.ret(r & 0xffffffff, 12)

    def s_related(self, m):
        a, b = self.name(self.ecx()), self.name(self.arg(1))
        r = self.lookup('related', a + '>' + b, False)
        self.events.append({'event': 'related', 'a': a, 'b': b, 'result': r}); self.ret(int(r), 4)

    def s_accepts(self, m):
        assert self.ecx() == self.comp and self.arg(2) == 1 and self.arg(3) == 1
        t = self.name(self.arg(1)); r = self.lookup('accepts', t, True)
        self.events.append({'event': 'component_accepts', 'target': t, 'result': r}); self.ret(int(r), 12)

    def read_list(self):
        out, node, prev = [], self.r(self.engage + 0x10), 0
        while node:
            assert self.r(node + 4) == prev
            out.append({'gtid': self.r(node), 'priority': struct.unpack('<i', struct.pack('<I', self.r(node + 0xc)))[0]})
            prev = node; node = self.r(node + 8)
        assert self.r(self.engage + 0x14) == prev
        return out

    def run(self):
        e, c = self.c['entry'], self.c
        target = 0 if c.get('target') is None else self.actors[c['target']]
        from unicorn import UcError
        try:
            if e == 'can_add':
                result = self.call(0x521060, ecx=self.engage, args=[target]) & 0xff
            elif e == 'add':
                result = self.call(0x523dd0, ecx=self.engage, args=[target, int(c['check'])]) & 0xff
            elif e == 'mind_add':
                result = self.call(0x524c10, ecx=self.mind, args=[target]) & 0xff
            elif e == 'remove':
                it = self.alloc(4); self.w(it, self.nodes[c['index']])
                self.call(0x51efb0, ecx=self.engage, args=[it]); result = None
            elif e == 'clear':
                self.call(0x51f040, ecx=self.engage); result = None
            elif e == 'refresh':
                self.call(0x523ea0, ecx=self.engage); result = None
        except (AssertionError, UcError):
            if not self.null_accesses:
                raise
            return {'fault': 'null dereference', 'events': self.events}
        return {'result': result, 'list': self.read_list(), 'events': self.events}


def cases():
    rnd = random.Random(0x524c10)
    rows = []
    def actors_set(n_targets, with_members=True):
        actors = {}
        for i in range(n_targets):
            actors[f't{i}'] = {'gtid': (0x10 + i) << 16 | (0x100 + i), 'flags': 1 if rnd.random() < 0.1 else 0,
                               'word90': rnd.choice([0, 0, 5])}
        if with_members:
            for i in range(n_targets):
                if rnd.random() < 0.35:
                    org = rnd.choice([f't{j}' for j in range(n_targets) if j != i]) if n_targets > 1 else None
                    actors[f't{i}']['membership'] = {'org': org, 'attached': rnd.choice([0, 1, 1])}
        return actors
    for i in range(150):
        n = rnd.randrange(1, 8)
        actors = actors_set(n)
        keys = list(actors)
        if rnd.random() < 0.2:
            actors[keys[-1]]['registered'] = False
        lst = []
        for k in rnd.sample(keys, rnd.randrange(0, len(keys) + 1)):
            lst.append({'gtid': actors[k]['gtid'], 'priority': rnd.randrange(-2, 7)})
        if rnd.random() < 0.3:
            lst.append({'gtid': 0x7fff0042, 'priority': rnd.randrange(-2, 7)})  # stale node
        lst.sort(key=lambda x: x['priority'])
        entry = rnd.choice(['can_add', 'add', 'add', 'mind_add', 'mind_add', 'mind_add', 'remove', 'clear', 'refresh', 'refresh', 'refresh'])
        if entry == 'remove' and not lst:
            entry = 'clear'
        c = {'name': f'{entry}{i}', 'entry': entry, 'org_gtid': 0x00090009, 'component': rnd.random() < 0.85,
             'org_entries': [rnd.randrange(1, 9) for _ in range(rnd.choice([0, 4, 5, 6]))],
             'current': rnd.choice(['engage', 'engage', 'chasing', 'other', None]),
             'chasing_target': rnd.choice([0, actors[rnd.choice(keys)]['gtid'], 0x55550001]),
             'actors': actors, 'list': lst, 'target': rnd.choice(keys + [None] if entry == 'mind_add' else keys),
             'check': rnd.random() < 0.7, 'index': rnd.randrange(len(lst)) if lst else 0,
             'can_target': {k: rnd.random() < 0.8 for k in keys}, 'engage': {k: rnd.random() < 0.6 for k in keys},
             'priority': {k: rnd.randrange(-2, 7) for k in keys}, 'check130': {k: rnd.random() < 0.4 for k in keys},
             'relation': {k: rnd.choice([0, 0, 0, 1, 2]) for k in keys}, 'accepts': {k: rnd.random() < 0.8 for k in keys},
             'related': {f'{a}>{b}': rnd.random() < 0.3 for a in keys + ['org'] for b in keys + ['org']}}
        rows.append(c)
    return rows


def main():
    out, visited = [], {}
    for c in cases():
        try:
            m = EngageMachine(c); result = m.run()
        except Exception:
            print(c['name']); raise
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': result})
    d = header(__file__, __doc__)
    d.update(instructions=instructions(visited), cases=out)
    p = HERE / 'native-cai-engage.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'faults': sum('fault' in r['output'] for r in out),
                      'instructions': len(visited), 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
