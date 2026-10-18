"""Original KKC_CAIStateSiege::Think 525930 (regiment state cai_siege at
Mind+F8, vtable A32CC8 slot +C) executing unchanged on an emulated regiment
Mind (native_cai_regiment.py) at x87 precision Single24, Double53 and
Extended64, together with 529E60, 529C30, 529C20, the siege site check
51D130, the engage-state helper 51E000 and the GTID list operations 763B90
(clear), 4CF1B0 (handle) and 4D19A0/4D0900 (push back).

Emulated data read natively: state+10 (target GTID), +14 (site GTID),
+18/+1C GTID list, +20/+21 bytes, +24 (binary32 stamp); the object +20/+24
positions and +90 words, organization +20/+24 and its +148/+14C entry array
(entry +8 binary32), [B39F58]+74 (binary32 siege range), the SVar table
[B39AB0] +C4/+C8 -> binary32, the world [B36534]+E0 (binary32 time), the
kill target Mind+D0 and the null point [B279C0]/[B279C4].

Seams (recorded in order with the tracked statuses and the Mind stack): Mind
virtuals +140 +44 (info struct: +4 word, +14 byte) +F0, organization
virtuals +0 (binary32 ST0) and +16C, list object virtual +0 (binary32 ST0),
state virtuals +C +1C, 4D11D0 (target site), 57A500 (queue), 4CEDA0 on the
site (anchor object), 520440 (move order), 51D920 (kill activation),
523700(info, &list, site) on the siege state (appends the case's GTIDs to
the list natively), 4F3D90(entry) on the world, 51FBC0(object) on the Mind;
allocator, free and loggers as in native_cai_regiment.py. Some cases model a kill
activation callback that sets the target's +90 word (kill_activate_sets_word90).
A null info
struct is a native null dereference (fault case).
"""
from pathlib import Path
import json, random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import PRECISIONS, f32, header, instructions, sha
from native_cai_regiment import RegimentMachine, coverage

TRACKED = ['siege', 'kill', 'move', 'sleep', 'other']


class SiegeMachine(RegimentMachine):
    def __init__(self, c):
        super().__init__(c, mind_virtuals=[(0x140, 'mind_virtual_140', 1)],
                         org_virtuals=[(0x16c, self.o_16c), (0x0, self.o_0)])
        mvt = self.r(self.mind)
        self.w(mvt + 0x44, self.stub(self.m_44)); self.w(mvt + 0xf0, self.stub(self.m_f0))
        self.accepts = list(c['accepts'])
        s = self.states['siege']
        self.w(s + 0x10, c['target_gtid']); self.w(s + 0x14, c['site_gtid'])
        self.wb(s + 0x20, c['byte20']); self.wb(s + 0x21, c['byte21']); self.w(s + 0x24, c['stamp'])
        self.list = s + 0x18
        for g in c['list']:
            self.push(g)
        self.w(self.states['kill'] + 0x10, c['kill_target'])
        self.w(self.org + 0x20, c['org_position'][0]); self.w(self.org + 0x24, c['org_position'][1])
        for key, o in c['objects'].items():
            p = self.add_object(key, o['gtid'], o['registered'], vtable_slots=[(0, self.ob_0)])
            self.w(p + 0x20, o['position'][0]); self.w(p + 0x24, o['position'][1]); self.w(p + 0x90, o['word90'])
        entry = 0
        if c['entry8'] is not None:
            entry = self.alloc(0x20); self.w(entry + 8, c['entry8'])
        self.entry = entry
        self.w(self.org + 0x14c, c['count']); self.w(self.org + 0x148, self.array([0, 0, 0, 0, entry, 0]))
        rng = self.alloc(0x100); self.w(0xb39f58, rng); self.w(rng + 0x74, c['range_base'])
        table = self.alloc(0x100); self.w(0xb39ab0, table)
        for off, key in [(0xc4, 'svar_c4'), (0xc8, 'svar_c8')]:
            p = self.alloc(4); self.w(p, c[key]); self.w(table + off, p)
        world = self.world = self.alloc(0x100); self.w(0xb36534, world); self.w(world + 0xe0, c['time'])
        self.w(0xb279c0, c['null_point'][0]); self.w(0xb279c4, c['null_point'][1])
        self.info = 0
        if not c['info_null']:
            self.info = self.alloc(0x20); self.w(self.info + 4, c['info_word4']); self.wb(self.info + 0x14, c['info_byte14'])
        for address, fn in {0x57a500: self.s_57a500, 0x4d11d0: self.s_4d11d0, 0x4ceda0: self.s_4ceda0,
                            0x520440: self.s_move_order, 0x51d920: self.s_kill_activate, 0x523700: self.s_fill,
                            0x4f3d90: self.s_4f3d90, 0x51fbc0: self.s_51fbc0}.items():
            self.seam(address, fn)

    def push(self, gtid):
        node = self.alloc(8); self.w(node, gtid)
        tail = self.r(self.list + 4)
        if tail:
            self.w(tail + 4, node)
        else:
            self.w(self.list, node)
        self.w(self.list + 4, node)

    def read_list(self):
        out, node = [], self.r(self.list)
        while node:
            out.append(self.r(node)); node = self.r(node + 4)
        return out

    def o_16c(self, m):
        assert self.ecx() == self.org
        self.ev('organization_virtual_16c', site=self.name(self.arg(1)), result=self.c['org_16c'])
        self.ret(int(self.c['org_16c']), 4)

    def o_0(self, m):
        assert self.ecx() == self.org
        self.ev('organization_virtual_0', result=self.c['org_reach']); self.ret_st0_f32(self.c['org_reach'])

    def ob_0(self, m):
        k = self.name(self.ecx())
        r = self.c['objects'][k]['reach']
        self.ev('object_virtual_0', object=k, result=r); self.ret_st0_f32(r)

    def m_44(self, m):
        assert self.ecx() == self.mind
        self.ev('mind_virtual_44', result=0 if self.info == 0 else 'info'); self.ret(self.info)

    def m_f0(self, m):
        assert self.ecx() == self.mind
        r = self.c['mind_f0']
        self.ev('mind_virtual_f0', object=self.name(self.arg(1)), result=r); self.ret(int(r), 4)

    def s_57a500(self, m):
        assert self.ecx() == self.comp
        r = self.accepts.pop(0)
        self.ev('component_57a500', object=self.name(self.arg(1)), flags=[self.arg(2), self.arg(3)], result=r)
        self.ret(int(r), 12)

    def s_4d11d0(self, m):
        k = self.name(self.ecx()); site = self.c['objects'][k].get('site')
        self.ev('object_4d11d0', object=k, result=site or 0)
        self.ret(0 if site is None else self.objects[site])

    def s_4ceda0(self, m):
        k = self.name(self.ecx()); a = self.c['anchor']
        self.ev('site_4ceda0', site=k, result=a or 0)
        self.ret(0 if a is None else self.objects[a])

    def s_move_order(self, m):
        assert self.ecx() == self.states['move']
        self.ev('move_order', args=[self.arg(i) for i in range(1, 7)]); self.ret(0, 0x18)

    def s_kill_activate(self, m):
        assert self.ecx() == self.states['kill']
        r = self.c['kill_activate']
        self.ev('kill_activate', mode=self.arg(1), target=self.name(self.arg(2)), result=r)
        if self.c['kill_activate_sets_word90']:
            self.w(self.objects['target'] + 0x90, 1)
        self.ret(int(r), 8)

    def s_fill(self, m):
        assert self.ecx() == self.states['siege'] and self.arg(2) == self.list and self.arg(1) == self.info
        self.ev('siege_fill', site=self.name(self.arg(3)), gtids=self.c['fill'])
        for g in self.c['fill']:
            self.push(g)
        self.ret(0, 12)

    def s_4f3d90(self, m):
        assert self.ecx() == self.world and self.arg(1) == self.entry
        self.ev('world_4f3d90'); self.ret(0, 4)

    def s_51fbc0(self, m):
        assert self.ecx() == self.mind
        k = self.name(self.arg(1)); r = self.c['objects'][k]['in_range']
        self.ev('target_in_range', object=k, result=r); self.ret(int(r), 4)

    def run(self):
        fault = self.run_think(0x525930, 'siege')
        if fault:
            return fault
        s = self.states['siege']
        return {'events': self.events, **self.snap(), 'list': self.read_list(),
                'bytes': [self.rb(s + 0x20), self.rb(s + 0x21)], 'stamp': self.r(s + 0x24),
                'info_byte14': None if self.info == 0 else self.rb(self.info + 0x14)}


def cases():
    rnd = random.Random(0x525930)
    precisions = list(PRECISIONS)
    stacks = [['siege'], ['siege'], ['siege', 'sleep'], ['other', 'siege'], ['kill', 'siege'], [], ['siege', None]]
    rows = []
    pt = lambda: [f32(rnd.uniform(-60, 60)), f32(rnd.uniform(-60, 60))]
    for i in range(900):
        objects = {'target': {'gtid': 0x00a00a00, 'registered': rnd.random() < 0.95, 'position': pt(),
                              'word90': rnd.choice([0] * 12 + [1]), 'site': rnd.choice(['s0'] * 12 + ['s1', None]),
                              'reach': f32(rnd.uniform(0, 10)), 'in_range': rnd.random() < 0.5},
                   's0': {'gtid': 0x00b00b00, 'registered': rnd.random() < 0.95, 'position': pt(), 'word90': 0,
                          'reach': f32(1), 'in_range': False},
                   's1': {'gtid': 0x00b10b01, 'registered': True, 'position': pt(), 'word90': 0,
                          'reach': f32(1), 'in_range': False},
                   'a0': {'gtid': 0x00c00c00, 'registered': False, 'position': pt(), 'word90': 0,
                          'reach': f32(1), 'in_range': False}}
        for k in range(rnd.randrange(0, 4)):
            objects['o%d' % k] = {'gtid': (0xd0 + k) << 16 | (0xd00 + k), 'registered': rnd.random() < 0.85,
                                  'position': pt(), 'word90': 0, 'reach': f32(rnd.uniform(0, 30)),
                                  'in_range': rnd.random() < 0.6}
        gt = [o['gtid'] for k, o in objects.items() if k.startswith('o')] + [0x7fff0005]
        org_position = pt()
        # Range so the approach decision is near the boundary for some cases.
        anchor = objects['a0']['position']
        dx = from_f32(anchor[0]) - from_f32(org_position[0]); dy = from_f32(anchor[1]) - from_f32(org_position[1])
        dist = (dx * dx + dy * dy) ** 0.5
        base = rnd.choice([dist - 1.0, dist * rnd.uniform(0.5, 1.5), dist, 5.0, 100.0])
        c = {'name': 'siege%d' % i, 'precision': precisions[i % 3], 'objects': objects,
             'target_gtid': rnd.choice([0x00a00a00] * 9 + [0x7fff0006]),
             'site_gtid': rnd.choice([0x00b00b00] * 14 + [0x00b10b01, 0x7fff0007, 0]),
             'anchor': rnd.choice(['a0'] * 9 + [None]),
             'accepts': [rnd.random() < 0.97], 'org_16c': rnd.random() < 0.97,
             'byte20': rnd.choice([0, 1]), 'byte21': rnd.choice([0, 1]),
             'stamp': f32(rnd.choice([0.0, 10.0, rnd.uniform(0, 100)])),
             'list': [rnd.choice(gt) for _ in range(rnd.randrange(0, 3))],
             'fill': [rnd.choice(gt) for _ in range(rnd.randrange(0, 3))],
             'kill_target': rnd.choice([0x00a00a00, 0x00a00a00, 0]),
             'org_position': org_position, 'count': rnd.choice([0, 5, 5, 5, 0xffffffff]),
             'entry8': rnd.choice([None, f32(0.0), f32(2.5), f32(rnd.uniform(0, 20))]),
             'range_base': f32(base - 0.5), 'svar_c4': f32(0.5), 'svar_c8': f32(rnd.choice([0.0, 5.0, 30.0])),
             'time': f32(rnd.choice([0.0, 10.0, 50.0, rnd.uniform(0, 100)])),
             'null_point': rnd.choice([[0, 0], [f32(3.5), f32(-2)]]),
             'info_null': rnd.random() < 0.02, 'info_word4': rnd.choice([0, 1, 7]),
             'info_byte14': rnd.choice([0, 1]), 'mind_f0': rnd.random() < 0.5,
             'org_reach': f32(rnd.uniform(0, 20)), 'kill_activate': rnd.random() < 0.7,
             'states': {'siege': rnd.choice([0, 1, 2, 2]), 'kill': rnd.choice([0, 2, 2]), 'move': rnd.choice([0, 2]),
                        'sleep': rnd.choice([0, 1, 2]), 'other': rnd.choice([1, 2])},
             'stack': rnd.choice(stacks), 'words': [rnd.getrandbits(32) for _ in range(4)],
             'can_activate': {}, 'tracked': TRACKED}
        if rnd.random() < 0.05:
            c['accepts'] = [False]
        # Modelled callback: the kill activation marks the target (+90), so
        # the later +90 reread takes the handle path.
        c['kill_activate_sets_word90'] = rnd.random() < 0.3
        rows.append(c)
    return rows


from native_machine import from_f32


def main():
    out, visited = [], {}
    for c in cases():
        m = SiegeMachine(c)
        result = m.run()
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': result})
    cov = coverage(visited, {'think_525930': (0x525930, 0x525c9d), 'engage_51e000': (0x51e000, 0x51e028)})
    d = header(__file__, __doc__)
    d.update(instructions=instructions(visited), coverage=cov, cases=out)
    p = HERE / 'native-cai-siege.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'faults': sum('fault' in r['output'] for r in out),
                      'instructions': len(visited),
                      'coverage': {k: [v['visited'], v['instructions'], v['unvisited']] for k, v in cov.items()},
                      'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
