"""Original KKC_CAIStateRepair::Think 5239A0 (regiment state cai_repair at
Mind+160, vtable A32BA4 slot +C) executing unchanged on an emulated regiment
Mind (native_cai_regiment.py) at x87 precision Single24, Double53 and
Extended64, together with 529E60, 529C30, 529C20, the member iterators
437940/4364C0, the repair action constructor 51E870 with 4D9E20, 4D9BF0 and
4D9B10, and the move action constructor 4CA760 (all native).

Emulated data read natively: state+10/+18 bytes and +14 (target GTID); the
target's +7C component and its +28/+2C members; the chosen object's +60, +8
and +100 words; the organization component +28/+2C members (each with a +80
object whose +1C byte the flagged iterator requires set; case key 'skip'), +78 slot offsets and +BC/+C0
formation origin (binary32); Mind+2B4/+2B8; the null point; the action
registry [B36530]+298.

Seams (recorded in order with the tracked statuses and the Mind stack): Mind
virtuals +140 +B8 +168, organization virtuals +34(3) and +108 (owner),
member virtual +38(definition), state virtuals +C +1C, 4D1490 on target
members, 4D0940 on the chosen object, 44DD70 on the owner, 51FBC0(object) on
the Mind, 520320 (move order, 5 words), 5617C0 on the chosen object's +60
word, 4D9B60(member) on an action (records its fields) and 4CA2D0 (action
release); allocator, free and loggers as in native_cai_regiment.py.
"""
from pathlib import Path
import json, random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import PRECISIONS, f32, header, instructions, sha
from native_cai_regiment import RegimentMachine, UnitOrderMixin, coverage

TRACKED = ['repair', 'move', 'sleep', 'other']
OWNER = 0x0eee0000


class RepairMachine(UnitOrderMixin, RegimentMachine):
    def __init__(self, c):
        super().__init__(c, mind_virtuals=[(0x140, 'mind_virtual_140', 1), (0xb8, 'mind_virtual_b8', 0),
                                           (0x168, 'mind_virtual_168', 0)],
                         org_virtuals=[(0x34, self.o_34), (0x108, self.o_owner)])
        self.install_unit_orders(c['definitions'], c['issue'], OWNER)
        s = self.states['repair']
        self.wb(s + 0x10, c['byte10']); self.w(s + 0x14, c['target_gtid']); self.wb(s + 0x18, c['byte18'])
        self.w(self.mind + 0x2b4, c['sleep_point'][0]); self.w(self.mind + 0x2b8, c['sleep_point'][1])
        self.w(0xb279c0, c['null_point'][0]); self.w(0xb279c4, c['null_point'][1])
        self.w(self.org + 0x20, c['org_position'][0]); self.w(self.org + 0x24, c['org_position'][1])
        self.word60 = {}
        for key, o in c['objects'].items():
            p = self.add_object(key, o['gtid'], o['registered'])
            self.w(p + 0x20, o['position'][0]); self.w(p + 0x24, o['position'][1])
            self.w(p + 0x60, o['word60']); self.w(p + 8, o['flags8']); self.w(p + 0x100, o['word100'])
            if o['word60']:
                self.word60[o['word60']] = key
        target = self.objects['target']
        if c['target_members'] is not None:
            comp = self.alloc(0x100)
            words = [0 if k is None else self.objects[k] for k in c['target_members']]
            self.w(comp + 0x28, self.array(words)); self.w(comp + 0x2c, len(words))
            self.w(target + 0x7c, comp)
        words = []
        for i, mem in enumerate(c['members']):
            if mem is None:
                words.append(0); continue
            p = self.add_object('m%d' % i, (0xe0 + i) << 16 | (0xe00 + i), True, vtable_slots=[(0x38, self.mb_38)])
            c80 = self.alloc(0x40); self.wb(c80 + 0x1c, mem['skip']); self.w(p + 0x80, c80)
            words.append(p)
        self.w(self.comp + 0x28, self.array(words)); self.w(self.comp + 0x2c, len(words))
        self.w(self.comp + 0x78, self.array([w for pt in c['slots'] for w in pt]))
        self.w(self.comp + 0xbc, c['origin'][0]); self.w(self.comp + 0xc0, c['origin'][1])
        for address, fn in {0x4d1490: self.s_pred('member_4d1490', 'check_4d1490'),
                            0x4d0940: self.s_pred('object_4d0940', 'check_4d0940'),
                            0x44dd70: self.s_44dd70, 0x51fbc0: self.s_51fbc0, 0x520320: self.s_move_order,
                            0x5617c0: self.s_5617c0}.items():
            self.seam(address, fn)

    def o_34(self, m):
        assert self.ecx() == self.org
        r = self.c['org_34']
        self.ev('organization_virtual_34', value=self.arg(1), result=r); self.ret(int(r), 4)

    def mb_38(self, m):
        k = self.name(self.ecx()); r = self.c['members'][int(k[1:])]['accepts']
        self.ev('member_virtual_38', member=k, definition=self.arg(1), result=r); self.ret(int(r), 4)

    def s_pred(self, event, key):
        def fn(m):
            k = self.name(self.ecx()); r = self.c['objects'][k][key]
            self.ev(event, object=k, result=r); self.ret(int(r))
        return fn

    def s_44dd70(self, m):
        assert self.ecx() == OWNER
        r = self.c['owner_44dd70']
        self.ev('owner_44dd70', result=r); self.ret(int(r))

    def s_51fbc0(self, m):
        assert self.ecx() == self.mind
        k = self.name(self.arg(1)); r = self.c['in_range']
        self.ev('target_in_range', object=k, result=r); self.ret(int(r), 4)

    def s_move_order(self, m):
        assert self.ecx() == self.states['move']
        args = [self.arg(i) for i in range(1, 6)]
        args[1] = self.name(args[1])
        self.ev('move_order', args=args); self.ret(0, 0x14)

    def s_5617c0(self, m):
        k = self.word60[self.ecx()]; r = self.c['objects'][k]['check_5617c0']
        self.ev('check_5617c0', object=k, result=r); self.ret(int(r))

    def run(self):
        fault = self.run_think(0x5239a0, 'repair')
        if fault:
            return fault
        s = self.states['repair']
        return {'events': self.events, **self.snap(), 'byte10': self.rb(s + 0x10)}


def cases():
    rnd = random.Random(0x5239a0)
    precisions = list(PRECISIONS)
    stacks = [['repair'], ['repair'], ['repair', 'sleep'], ['other', 'repair'], [], ['repair', None]]
    pt = lambda: [f32(rnd.uniform(-60, 60)), f32(rnd.uniform(-60, 60))]
    rows = []
    for i in range(700):
        def obj(gtid, registered=True):
            return {'gtid': gtid, 'registered': registered, 'position': pt(),
                    'word60': rnd.choice([0] + [0x6000 + gtid % 0x100] * 14),
                    'flags8': rnd.choice([0x4000] * 8 + [0, 0xffffbfff, 0x4001]),
                    'word100': rnd.choice([0, 0, 0x200000]), 'check_4d1490': rnd.random() < 0.6,
                    'check_4d0940': rnd.random() < 0.3, 'check_5617c0': rnd.random() < 0.3}
        objects = {'target': obj(0x00f00f00, rnd.random() < 0.95)}
        tmembers = None
        if rnd.random() < 0.4:
            tmembers = []
            for k in range(rnd.choice([0, 1, 2, 2, 3, 3])):
                if rnd.random() < 0.15:
                    tmembers.append(None); continue
                key = 't%d' % k
                objects[key] = obj((0xf1 + k) << 16 | (0xf10 + k))
                tmembers.append(key)
            if rnd.random() < 0.2 and tmembers:
                # Equal distances: the first strictly nearer member wins.
                for k in tmembers:
                    if k is not None:
                        objects[k]['position'] = objects[tmembers[0] or 'target']['position']
        members = [None if rnd.random() < 0.1 else {'skip': rnd.choice([0, 0, 0, 1]), 'accepts': rnd.random() < 0.5}
                   for _ in range(rnd.choice([0, 1, 2, 3, 3]))]
        c = {'name': 'repair%d' % i, 'precision': precisions[i % 3], 'objects': objects,
             'target_gtid': rnd.choice([0x00f00f00] * 19 + [0x7fff0008]), 'target_members': tmembers,
             'members': members, 'slots': [pt() for _ in range(max(len(members), 1))],
             'origin': [f32(rnd.uniform(-1e3, 1e3)), f32(rnd.choice([0.1, 1e7, rnd.uniform(-1e3, 1e3)]))],
             'sleep_point': pt(), 'null_point': rnd.choice([[0, 0], [f32(3.5), f32(-2)]]),
             'org_position': pt(), 'byte10': rnd.choice([0, 1, 2]), 'byte18': rnd.choice([0, 1, 5]),
             'org_34': rnd.random() < 0.92, 'owner_44dd70': rnd.random() < 0.2, 'in_range': rnd.random() < 0.6,
             'definitions': {'3': rnd.choice([0, 0x7ab0003]), '5': rnd.choice([0, 0x7ab0005])},
             'issue': [rnd.random() < 0.5 for _ in range(8)],
             'states': {'repair': rnd.choice([0, 1, 2, 2]), 'move': rnd.choice([0, 1, 2]),
                        'sleep': rnd.choice([0, 1, 2]), 'other': rnd.choice([1, 2])},
             'stack': rnd.choice(stacks), 'words': [rnd.getrandbits(32) for _ in range(4)],
             'can_activate': {}, 'tracked': TRACKED}
        rows.append(c)
    return rows


def main():
    out, visited = [], {}
    for c in cases():
        m = RepairMachine(c)
        result = m.run()
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': result})
    cov = coverage(visited, {'think_5239a0': (0x5239a0, 0x523d42), 'repair_action_51e870': (0x51e870, 0x51e8ce)})
    d = header(__file__, __doc__)
    d.update(instructions=instructions(visited), coverage=cov, cases=out)
    p = HERE / 'native-cai-repair.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'faults': sum('fault' in r['output'] for r in out),
                      'instructions': len(visited),
                      'coverage': {k: [v['visited'], v['instructions'], v['unvisited']] for k, v in cov.items()},
                      'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
