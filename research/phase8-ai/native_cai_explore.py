"""Original KKC_CAIStateExplore::Think 525D70 (regiment state cai_explore at
Mind+220, vtable A32BEC slot +C) executing unchanged on an emulated regiment
Mind (native_cai_regiment.py) at x87 precision Single24, Double53 and
Extended64, together with 529E60, 529C30, 529C20, the group resolver 4D1900,
the arrival test 51F200 (binary32 distance with fsqrt) and the Mind
activation 52A810 mode 0.

Emulated data read natively: organization +14/+20/+24/+164, the group's
+194 GTID, the leader object's +70 Mind and its +238..+248 explore target,
state+10/+14 (explore point) and +18..+28 (target: x, y, score, count,
word), the region map [B36534]+30 (+C/+10 binary32 size), the SVar table
[B39AB0] +74/+78 (candidate counts) and +A0 -> binary32 arrival radius, the
null point.

Seams (recorded in order with the tracked statuses and the Mind stack):
state virtuals +C +18 +1C (some cases model the real exits, which clear the
leaving state's status: leave_clears), 4DCE60(organization) and
4DCF00(&state+18) on the group, 4282A0(x, y) and 552640(&state+10) on the
region map, the candidate generators 524170/5242C0(&candidate, best count)
on the state (they write the case's candidate fields, or nothing),
4F46E0(organization, &candidate, 4.0) on the world, 520440 (move order) and
51E9B0(x, y) on the move state; allocator, free and loggers as in
native_cai_regiment.py. Some cases model a 4DCF00 callback that leaves the
move state active (share_sets_move) so the 51E9B0 branch is exercised. A
null leader Mind is a native null dereference (fault case).
"""
from pathlib import Path
import json, random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import PRECISIONS, f32, header, instructions, sha
from native_cai_regiment import RegimentMachine, coverage

TRACKED = ['explore', 'move', 'sleep', 'other']


class ExploreMachine(RegimentMachine):
    def __init__(self, c):
        super().__init__(c)
        s = self.states['explore']
        self.w(s + 0x10, c['point'][0]); self.w(s + 0x14, c['point'][1])
        for i, v in enumerate(c['target']):
            self.w(s + 0x18 + 4 * i, v)
        self.w(self.org + 0x20, c['org_position'][0]); self.w(self.org + 0x24, c['org_position'][1])
        self.w(self.org + 0x164, c['group_gtid'])
        world = self.world = self.alloc(0x100); self.w(0xb36534, world)
        regions = self.regions = self.alloc(0x40); self.w(world + 0x30, regions)
        self.w(regions + 0xc, c['size'][0]); self.w(regions + 0x10, c['size'][1])
        table = self.alloc(0x100); self.w(0xb39ab0, table)
        for off, key in [(0x74, 'count_a'), (0x78, 'count_b'), (0xa0, 'radius')]:
            p = self.alloc(4); self.w(p, c[key]); self.w(table + off, p)
        self.w(0xb279c0, c['null_point'][0]); self.w(0xb279c4, c['null_point'][1])
        g = self.add_object('group', 0x00b20b20, c['group_registered'])
        self.w(g + 0x194, c['leader_gtid'])
        leader = self.add_object('leader', 0x00b30b30, c['leader_registered'])
        if c['leader_mind'] is not None:
            lm = self.alloc(0x300)
            for i, v in enumerate(c['leader_mind']):
                self.w(lm + 0x238 + 4 * i, v)
            self.w(leader + 0x70, lm)
        self.cands = {'a': list(c['candidates_a']), 'b': list(c['candidates_b'])}
        self.accept = list(c['accept'])
        self.contains = list(c['contains'])
        for address, fn in {0x4dce60: self.s_leads, 0x4dcf00: self.s_share, 0x4282a0: self.s_contains,
                            0x552640: self.s_clamp, 0x524170: self.s_candidate('a'),
                            0x5242c0: self.s_candidate('b'), 0x4f46e0: self.s_4f46e0,
                            0x520440: self.s_move_order, 0x51e9b0: self.s_retarget}.items():
            self.seam(address, fn)

    def s_leads(self, m):
        assert self.ecx() == self.objects['group'] and self.arg(1) == self.org
        r = self.c['leads']
        self.ev('group_4dce60', result=r); self.ret(int(r), 4)

    def s_share(self, m):
        assert self.ecx() == self.objects['group'] and self.arg(1) == self.states['explore'] + 0x18
        self.ev('group_4dcf00', target=[self.r(self.arg(1) + 4 * i) for i in range(5)])
        if self.c['share_sets_move']:
            self.w(self.states['move'] + 0xc, 2)
        self.ret(0, 4)

    def s_contains(self, m):
        assert self.ecx() == self.regions
        r = self.contains.pop(0)
        self.ev('region_contains', point=[self.arg(1), self.arg(2)], result=r); self.ret(int(r), 8)

    def s_clamp(self, m):
        assert self.ecx() == self.regions and self.arg(1) == self.states['explore'] + 0x10
        p = self.c['clamped']
        self.w(self.arg(1), p[0]); self.w(self.arg(1) + 4, p[1])
        self.ev('region_clamp', result=p); self.ret(0, 4)

    def s_candidate(self, which):
        def fn(m):
            assert self.ecx() == self.states['explore']
            out = self.arg(1); best = self.arg(2)
            initial = [self.r(out + 4 * i) for i in range(5)]
            spec = self.cands[which].pop(0)
            if spec is not None:
                for i, v in enumerate(spec):
                    self.w(out + 4 * i, v)
            self.ev('candidate_' + which, best_count=best, initial=initial, result=spec); self.ret(0, 8)
        return fn

    def s_4f46e0(self, m):
        assert self.ecx() == self.world and self.arg(1) == self.org
        cand = self.arg(2)
        r = self.accept.pop(0)
        self.ev('world_4f46e0', candidate=[self.r(cand + 4 * i) for i in range(5)], radius=self.arg(3), result=r)
        self.ret(int(r), 12)

    def s_move_order(self, m):
        assert self.ecx() == self.states['move']
        self.ev('move_order', args=[self.arg(i) for i in range(1, 7)]); self.ret(0, 0x18)

    def s_retarget(self, m):
        assert self.ecx() == self.states['move']
        self.ev('move_retarget', point=[self.arg(1), self.arg(2)]); self.ret(0, 8)

    def run(self):
        fault = self.run_think(0x525d70, 'explore')
        if fault:
            return fault
        s = self.states['explore']
        return {'events': self.events, **self.snap(), 'point': [self.r(s + 0x10), self.r(s + 0x14)],
                'target': [self.r(s + 0x18 + 4 * i) for i in range(5)]}


def cases():
    rnd = random.Random(0x525d70)
    precisions = list(PRECISIONS)
    stacks = [['explore'], ['explore'], ['explore', 'sleep'], ['other', 'explore'], [], ['explore', None]]
    rows = []
    for i in range(900):
        size = [f32(rnd.choice([100.0, 256.0, 512.0])), f32(rnd.choice([100.0, 256.0, 512.0]))]
        coord = lambda: f32(rnd.choice([-1.0, -0.0, 0.0, 50.0, 99.5, 100.0, 300.0, rnd.uniform(-20, 520)]))
        cand = lambda: None if rnd.random() < 0.1 else [coord(), coord(), f32(rnd.choice([-10000.0, 0.0, 1.5, 7.0, rnd.uniform(-5, 50)])),
                                                         rnd.choice([0, 0, 1, 2, 3, 0xffffffff]), rnd.getrandbits(32)]
        count_a = rnd.choice([0, 1, 2, 3, 0xffffffff])
        count_b = rnd.choice([0, 1, 2, 3])
        org = [f32(rnd.uniform(0, 300)), f32(rnd.uniform(0, 300))]
        target = [coord(), coord(), f32(rnd.uniform(-5, 50)), rnd.choice([0, 1, 2]), rnd.getrandbits(32)]
        if rnd.random() < 0.3:
            # Near the organization so the arrival radius test decides.
            target[0] = f32(from_f32(org[0]) + rnd.uniform(-8, 8)); target[1] = f32(from_f32(org[1]) + rnd.uniform(-8, 8))
        leader_target = [coord(), coord(), f32(rnd.uniform(-5, 50)), rnd.choice([0, 1, 2]), rnd.getrandbits(32)]
        if rnd.random() < 0.3:
            leader_target[0], leader_target[1] = target[0], target[1]
        c = {'name': 'explore%d' % i, 'precision': precisions[i % 3],
             'point': [coord(), coord()], 'target': target, 'org_position': org,
             'group_gtid': rnd.choice([0x00b20b20, 0x00b20b20, 0, 0x7fff000a]),
             'group_registered': rnd.random() < 0.9, 'leader_gtid': rnd.choice([0x00b30b30] * 5 + [0, 0x7fff000b]),
             'leader_registered': rnd.random() < 0.9,
             'leader_mind': None if rnd.random() < 0.03 else leader_target,
             'leads': rnd.random() < 0.5, 'size': size,
             'count_a': count_a, 'count_b': count_b, 'radius': f32(rnd.choice([0.0, 2.0, 5.0, 10.0])),
             'candidates_a': [cand() for _ in range(count_a if count_a < 0x80000000 else 0)],
             'candidates_b': [cand() for _ in range(count_b)],
             'accept': [rnd.random() < 0.7 for _ in range(4)],
             'contains': [rnd.random() < 0.8, rnd.random() < 0.7],
             'clamped': [f32(rnd.uniform(0, 100)), f32(rnd.uniform(0, 100))],
             'null_point': rnd.choice([[0, 0], [f32(3.5), f32(-2)]]), 'leave_clears': rnd.random() < 0.6,
             'share_sets_move': rnd.random() < 0.3,
             'states': {'explore': rnd.choice([0, 1, 2, 2]), 'move': rnd.choice([0, 1, 2, 2]),
                        'sleep': rnd.choice([0, 1, 2]), 'other': rnd.choice([1, 2])},
             'stack': rnd.choice(stacks), 'words': [rnd.getrandbits(32) for _ in range(4)],
             'can_activate': {}, 'tracked': TRACKED}
        rows.append(c)
    return rows


from native_machine import from_f32


def main():
    out, visited = [], {}
    for c in cases():
        m = ExploreMachine(c)
        result = m.run()
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': result})
    cov = coverage(visited, {'think_525d70': (0x525d70, 0x526260), 'arrived_51f200': (0x51f200, 0x51f244)})
    d = header(__file__, __doc__)
    d.update(instructions=instructions(visited), coverage=cov, cases=out)
    p = HERE / 'native-cai-explore.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'faults': sum('fault' in r['output'] for r in out),
                      'instructions': len(visited),
                      'coverage': {k: [v['visited'], v['instructions'], v['unvisited']] for k, v in cov.items()},
                      'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
