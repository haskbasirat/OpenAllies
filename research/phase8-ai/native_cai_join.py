"""Original KKC_CAIStateJoin::Think 521850 (regiment state cai_join at
Mind+24C, vtable A32EC4 slot +C) executing unchanged on an emulated regiment
Mind (native_cai_regiment.py) with 529E60, 529C30, the GTID resolver 432E40,
the organization member iterator 4364C0 (skip flag clear: null entries are
skipped, count +2C reread) and the Tai helper 5217C0 (every member joins).

Emulated data read natively: the organization +148/+14C entry array, the
organization component +28/+2C member array, member +14 GTID and +70 Mind,
state+10 (target GTID), +14/+15 bytes, +18/+1C GTID list and the null point
[B279C0]/[B279C4].

Seams (recorded in order with the tracked statuses and the Mind stack): Mind
virtuals +140 +144 +16C, Updatable +8 virtual +4 (schedule), state virtual
+C (move Think), target virtual +130(entry, 0), member Mind virtual +F4
(target), 520320 (move order, 5 words) on the move state; loggers 404F60 and
404F90 (diagnostics, not recorded); allocator and free as in
native_cai_regiment.py. A null member Mind is a native null dereference
(fault case).
"""
from pathlib import Path
import json, random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import f32, header, instructions, sha
from native_cai_regiment import RegimentMachine, coverage

TRACKED = ['join', 'move', 'sleep', 'other']


class JoinMachine(RegimentMachine):
    def __init__(self, c):
        super().__init__(c, mind_virtuals=[(0x140, 'mind_virtual_140', 1), (0x144, 'mind_virtual_144', 0),
                                           (0x16c, 'mind_virtual_16c', 0)])
        s = self.states['join']
        self.w(s + 0x10, c['target_gtid']); self.wb(s + 0x14, c['arrived']); self.wb(s + 0x15, c['gathering'])
        self.w(s + 0x18, self.array(c['listed'])); self.w(s + 0x1c, c['listed_count'])
        self.w(self.org + 0x14c, c['count']); self.w(self.org + 0x148, self.array(c['entries']))
        self.w(0xb279c0, c['null_point'][0]); self.w(0xb279c4, c['null_point'][1])
        t = c['target']
        self.add_object('target', t['gtid'], t['registered'], vtable_slots=[(0x130, self.t_130)])
        mmvt = self.alloc(0x200)
        for slot in range(0, 0x200, 4):
            self.w(mmvt + slot, self.stub(self.unexpected('member mind', slot)))
        self.w(mmvt + 0xf4, self.stub(self.mm_f4))
        self.member_minds = {}
        words = []
        for i, mem in enumerate(c['members']):
            if mem is None:
                words.append(0); continue
            p = self.add_object('m%d' % i, mem['gtid'], True)
            if mem['mind']:
                mm = self.alloc(0x40); self.w(mm, mmvt); self.w(p + 0x70, mm); self.member_minds[mm] = 'm%d' % i
            words.append(p)
        self.w(self.comp + 0x28, self.array(words)); self.w(self.comp + 0x2c, c['member_count'])
        self.seam(0x520320, self.s_move_order)

    def t_130(self, m):
        r = self.c['target']['v130']
        self.ev('target_virtual_130', entry=self.arg(1), flag=self.arg(2), result=r); self.ret(int(r), 8)

    def mm_f4(self, m):
        self.ev('member_mind_virtual_f4', member=self.member_minds[self.ecx()], target=self.name(self.arg(1)))
        self.ret(0, 4)

    def s_move_order(self, m):
        assert self.ecx() == self.states['move']
        args = [self.arg(i) for i in range(1, 6)]
        args[1] = self.name(args[1])
        self.ev('move_order', args=args); self.ret(0, 0x14)

    def run(self):
        fault = self.run_think(0x521850, 'join')
        if fault:
            return fault
        return {'events': self.events, **self.snap()}


def cases():
    rnd = random.Random(0x521850)
    stacks = [['join'], ['join'], ['join', 'sleep'], ['other', 'join'], [], ['join', None]]
    rows = []
    for i in range(400):
        members = []
        for k in range(rnd.randrange(0, 5)):
            if rnd.random() < 0.2:
                members.append(None); continue
            members.append({'gtid': (0x80 + k) << 16 | (0x800 + k), 'mind': rnd.random() < 0.98})
        gtids = [m['gtid'] for m in members if m is not None]
        listed = [g for g in gtids if rnd.random() < 0.5] + rnd.choice([[], [0], [0x7fff0003]])
        rnd.shuffle(listed)
        target = {'gtid': 0x00900900, 'registered': rnd.random() < 0.95, 'v130': rnd.random() < 0.3}
        c = {'name': 'join%d' % i, 'precision': 'Double53', 'target': target,
             'target_gtid': rnd.choice([0x00900900] * 8 + [0x7fff0004, 0]),
             'arrived': rnd.choice([0, 1, 1]), 'gathering': rnd.choice([0, 1]),
             'listed': listed, 'listed_count': len(listed) if rnd.random() < 0.9 else rnd.choice([0, 0xffffffff]),
             'count': rnd.choice([0, 4, 5, 6, 0xffffffff]),
             'entries': [rnd.getrandbits(32) if rnd.random() < 0.8 else 0 for _ in range(6)],
             'members': members, 'member_count': len(members) if rnd.random() < 0.9 else 0,
             'null_point': rnd.choice([[0, 0], [f32(3.5), f32(-2)]]),
             'states': {'join': rnd.choice([0, 1, 2, 2]), 'move': rnd.choice([0, 0, 2]),
                        'sleep': rnd.choice([0, 1, 2]), 'other': rnd.choice([1, 2])},
             'stack': rnd.choice(stacks), 'words': [rnd.getrandbits(32) for _ in range(4)],
             'can_activate': {}, 'tracked': TRACKED}
        rows.append(c)
    return rows


def main():
    out, visited = [], {}
    for c in cases():
        m = JoinMachine(c)
        result = m.run()
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': result})
    cov = coverage(visited, {'think_521850': (0x521850, 0x521aed), 'join_all_5217c0': (0x5217c0, 0x52184a)})
    d = header(__file__, __doc__)
    d.update(instructions=instructions(visited), coverage=cov, cases=out)
    p = HERE / 'native-cai-join.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'faults': sum('fault' in r['output'] for r in out),
                      'instructions': len(visited),
                      'coverage': {k: [v['visited'], v['instructions'], v['unvisited']] for k, v in cov.items()},
                      'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
