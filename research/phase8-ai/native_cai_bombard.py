"""Original KKC_CAIStateBombard::Think 5207F0 (regiment state cai_bombard at
Mind+D4, vtable A3348C slot +C) executing unchanged on an emulated regiment
Mind (native_cai_regiment.py) at x87 precision Single24, Double53 and
Extended64, together with 529E60, 529C30, 529C20, the member iterator
4364C0, the GTID handle 4CF1B0, the attack action constructor 51E7A0 with
4D9E20 and 4D9BF0, and the move action constructor 4CA760 (all native).

Emulated data read natively: state+10 (target GTID), +14 (CString whose
length word at data-0xC is read), +18/+1C (binary32 point), +20 byte; the
target's +8 flags, +14 GTID and +20/+24 position; the organization component
+28/+2C members, +78 slot offsets and +BC/+C0 origin; Mind+2B4/+2B8; the
null point; [B36534]+30 (region map) and [B39D8C] (alert context).

Seams (recorded in order with the tracked statuses and the Mind stack): Mind
virtuals +140 +144, organization virtuals +24 (name, fake CString) and +108
(owner, no stack argument), target virtual +5C(owner, 1), member virtual
+34(1), state virtuals +C +1C, 42D470 (cdecl alert: context, 0x26, owner,
&position, name), 42A530(x, y) on the region map and 54F3C0(owner) on its
result, 4D4FF0(target, 3, 1) and 4D5180(&point, 1) on the organization,
520320/520440 (move orders), 4D9B60(member) on an action (records its
fields) and 4CA2D0 (action release); allocator, free and loggers as in
native_cai_regiment.py.
"""
from pathlib import Path
import json, random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import PRECISIONS, f32, header, instructions, sha
from native_cai_regiment import RegimentMachine, UnitOrderMixin, coverage

TRACKED = ['bombard', 'move', 'sleep', 'other']
OWNER = 0x0eee0000
REGIONS = 0x0bad3000
CONTEXT = 0x0bad8d00


class BombardMachine(UnitOrderMixin, RegimentMachine):
    def __init__(self, c):
        super().__init__(c, mind_virtuals=[(0x140, 'mind_virtual_140', 1), (0x144, 'mind_virtual_144', 0)],
                         org_virtuals=self.org_unit_virtuals())
        self.install_unit_orders(c['definitions'], c['issue'], OWNER)
        s = self.states['bombard']
        self.w(s + 0x10, c['target_gtid'])
        self.w(s + 0x14, self.cstring('label', refs=3)); self.w(self.r(s + 0x14) - 0xc, c['label_length'])
        self.w(s + 0x18, c['point'][0]); self.w(s + 0x1c, c['point'][1]); self.wb(s + 0x20, c['byte20'])
        self.w(self.mind + 0x2b4, c['sleep_point'][0]); self.w(self.mind + 0x2b8, c['sleep_point'][1])
        self.w(0xb279c0, c['null_point'][0]); self.w(0xb279c4, c['null_point'][1])
        world = self.alloc(0x100); self.w(0xb36534, world); self.w(world + 0x30, REGIONS)
        self.w(0xb39d8c, CONTEXT)
        t = c['target']
        p = self.add_object('target', t['gtid'], t['registered'], vtable_slots=[(0x5c, self.t_5c)])
        self.w(p + 8, t['flags8']); self.w(p + 0x20, t['position'][0]); self.w(p + 0x24, t['position'][1])
        words = []
        for i, mem in enumerate(c['members']):
            if mem is None:
                words.append(0); continue
            words.append(self.add_object('m%d' % i, (0xe0 + i) << 16 | (0xe00 + i), True,
                                         vtable_slots=[(0x34, self.mb_34)]))
        self.w(self.comp + 0x28, self.array(words)); self.w(self.comp + 0x2c, len(words))
        self.w(self.comp + 0x78, self.array([w for pt in c['slots'] for w in pt]))
        self.w(self.comp + 0xbc, c['origin'][0]); self.w(self.comp + 0xc0, c['origin'][1])
        for address, fn in {0x42d470: self.s_alert, 0x42a530: self.s_region, 0x54f3c0: self.s_region_check,
                            0x4d4ff0: self.s_relation, 0x4d5180: self.s_point_relation,
                            0x520320: self.s_move_object, 0x520440: self.s_move_point}.items():
            self.seam(address, fn)

    def t_5c(self, m):
        r = self.c['target']['v5c']
        self.ev('target_virtual_5c', owner=self.arg(1), flag=self.arg(2), result=r); self.ret(int(r), 8)

    def mb_34(self, m):
        k = self.name(self.ecx()); r = self.c['members'][int(k[1:])]['accepts']
        self.ev('member_virtual_34', member=k, value=self.arg(1), result=r); self.ret(int(r), 4)

    def s_alert(self, m):
        assert self.arg(1) == CONTEXT
        pos = self.arg(4)
        self.ev('post_alert', kind=self.arg(2), owner=self.arg(3), position=[self.r(pos), self.r(pos + 4)],
                argument=self.strings[self.arg(5)])
        self.ret(0)

    def s_region(self, m):
        assert self.ecx() == REGIONS
        self.ev('region_at', point=[self.arg(1), self.arg(2)]); self.ret(0x0bad4000, 8)

    def s_region_check(self, m):
        assert self.ecx() == 0x0bad4000
        r = self.c['region_check']
        self.ev('region_54f3c0', owner=self.arg(1), result=r); self.ret(int(r), 4)

    def s_relation(self, m):
        assert self.ecx() == self.org
        r = self.c['relation']
        self.ev('relation', object=self.name(self.arg(1)), a=self.arg(2), b=self.arg(3), result=r)
        self.ret(r & 0xffffffff, 12)

    def s_point_relation(self, m):
        assert self.ecx() == self.org and self.arg(1) == self.states['bombard'] + 0x18
        r = self.c['point_relation']
        self.ev('point_relation', b=self.arg(2), result=r); self.ret(r & 0xffffffff, 8)

    def s_move_object(self, m):
        assert self.ecx() == self.states['move']
        args = [self.arg(i) for i in range(1, 6)]
        args[1] = self.name(args[1])
        self.ev('move_order_object', args=args); self.ret(0, 0x14)

    def s_move_point(self, m):
        assert self.ecx() == self.states['move']
        self.ev('move_order_point', args=[self.arg(i) for i in range(1, 7)]); self.ret(0, 0x18)

    def run(self):
        fault = self.run_think(0x5207f0, 'bombard')
        if fault:
            return fault
        return {'events': self.events, **self.snap(), 'byte20': self.rb(self.states['bombard'] + 0x20)}


def cases():
    rnd = random.Random(0x5207f0)
    precisions = list(PRECISIONS)
    stacks = [['bombard'], ['bombard'], ['bombard', 'sleep'], ['other', 'bombard'], [], ['bombard', None]]
    pt = lambda: [f32(rnd.uniform(-60, 60)), f32(rnd.uniform(-60, 60))]
    rows = []
    for i in range(800):
        members = [None if rnd.random() < 0.1 else {'accepts': rnd.random() < 0.6}
                   for _ in range(rnd.choice([0, 1, 2, 3]))]
        target = {'gtid': 0x00a10a10, 'registered': rnd.random() < 0.9, 'position': pt(),
                  'flags8': rnd.choice([0, 0, 0x20000000, 0x20000001, 0xdfffffff]), 'v5c': rnd.random() < 0.6}
        c = {'name': 'bombard%d' % i, 'precision': precisions[i % 3], 'target': target,
             'target_gtid': rnd.choice([0x00a10a10] * 5 + [0x7fff0009, 0]),
             'label_length': rnd.choice([0, 0, 5]), 'point': pt(), 'byte20': rnd.choice([0, 1]),
             'members': members, 'slots': [pt() for _ in range(max(len(members), 1))],
             'origin': [f32(rnd.uniform(-1e3, 1e3)), f32(rnd.choice([0.1, 1e7, rnd.uniform(-1e3, 1e3)]))],
             'sleep_point': pt(), 'null_point': rnd.choice([[0, 0], [f32(3.5), f32(-2)]]),
             'region_check': rnd.random() < 0.4, 'relation': rnd.choice([0, 0, 1, 1, 2, -1]),
             'point_relation': rnd.choice([0, 0, 1, 1, 2]),
             'definitions': {'1': rnd.choice([0, 0x7ab0001]), '5': rnd.choice([0, 0x7ab0005])},
             'issue': [rnd.random() < 0.5 for _ in range(8)],
             'states': {'bombard': rnd.choice([0, 1, 2, 2]), 'move': rnd.choice([0, 1, 2]),
                        'sleep': rnd.choice([0, 1, 2]), 'other': rnd.choice([1, 2])},
             'stack': rnd.choice(stacks), 'words': [rnd.getrandbits(32) for _ in range(4)],
             'can_activate': {}, 'tracked': TRACKED}
        rows.append(c)
    return rows


def main():
    out, visited = [], {}
    for c in cases():
        m = BombardMachine(c)
        result = m.run()
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': result})
    cov = coverage(visited, {'think_5207f0': (0x5207f0, 0x520c78), 'attack_action_51e7a0': (0x51e7a0, 0x51e7f2)})
    d = header(__file__, __doc__)
    d.update(instructions=instructions(visited), coverage=cov, cases=out)
    p = HERE / 'native-cai-bombard.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'faults': sum('fault' in r['output'] for r in out),
                      'instructions': len(visited),
                      'coverage': {k: [v['visited'], v['instructions'], v['unvisited']] for k, v in cov.items()},
                      'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
