"""Original repair-state accept 51ECF0(mode, object, keep) (regiment state
cai_repair at Mind+160) executing unchanged on an emulated regiment Mind
(native_cai_regiment.py), together with 529C00, 529C30, 52A000, 44DD70,
4C57F0, 4D0940, 579F70 with the member iterator 4364C0, 4CF1B0 and the
activation 52A810 with its stack operations 52A470/52A420/52A290 (all
native).

Emulated data read natively: the object's +8, +14, +60, +7C (component with
+28/+2C members) and +100 words; repair state+14/+18; Mind +C; the
organization owner's +1F0 (binary32).

Seams (recorded in order with the tracked statuses and the Mind stack):
organization virtuals +34(3) and +108 (owner), 4D1490 on component members,
the state virtuals +4 (can activate), +18 (enter) and +1C (leave), the Mind
Updatable +4 (schedule) and Mind virtuals +1A0/+1A4; allocator, free and
loggers as in native_cai_regiment.py.

Usage: native_cai_repair_accept.py (AA.exe is located by the shared harness
relative to this repository).
"""
from pathlib import Path
import json, random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import PRECISIONS, f32, header, instructions, sha
from native_cai_regiment import RegimentMachine, coverage

TRACKED = ['repair', 'move', 'sleep', 'other']


class AcceptMachine(RegimentMachine):
    def __init__(self, c):
        super().__init__(c, org_virtuals=[(0x34, self.o_34), (0x108, self.o_owner)])
        s = self.states['repair']
        self.w(s + 0x14, c['state_target']); self.wb(s + 0x18, c['state_byte18'])
        self.w(self.mind + 0xc, c['mind_c'])
        owner = self.owner = self.alloc(0x400)
        self.w(owner + 0x1f0, c['owner_1f0'])
        self.target = 0
        if c['object'] is not None:
            o = c['object']
            p = self.target = self.add_object('target', o['gtid'], True)
            self.w(p + 8, o['flags8']); self.w(p + 0x60, o['word60']); self.w(p + 0x100, o['word100'])
            if o['members'] is not None:
                comp = self.alloc(0x100)
                words = []
                for i, mem in enumerate(o['members']):
                    if mem is None:
                        words.append(0); continue
                    words.append(self.add_object('m%d' % i, (0xe0 + i) << 16 | (0xe00 + i), True))
                self.w(comp + 0x28, self.array(words)); self.w(comp + 0x2c, len(words))
                self.w(p + 0x7c, comp)
        self.seam(0x4d1490, self.s_4d1490)

    def o_34(self, m):
        assert self.ecx() == self.org
        r = self.c['org_34']
        self.ev('organization_virtual_34', value=self.arg(1), result=r); self.ret(int(r), 4)

    def o_owner(self, m):
        assert self.ecx() == self.org
        self.ev('organization_owner'); self.ret(self.owner)

    def s_4d1490(self, m):
        k = self.name(self.ecx()); r = self.c['object']['members'][int(k[1:])]['check_4d1490']
        self.ev('member_4d1490', object=k, result=r); self.ret(int(r))

    def run(self):
        from unicorn import UcError
        s = self.states['repair']
        try:
            result = self.call(0x51ecf0, ecx=s, args=(self.c['mode'], self.target, self.c['keep']))
        except (AssertionError, UcError) as e:
            if self.null_accesses or isinstance(e, UcError):
                return {'fault': 'null dereference', 'events': self.events}
            raise
        return {'result': result & 0xff, 'events': self.events, **self.snap(),
                'state_target': self.r(s + 0x14), 'state_byte18': self.rb(s + 0x18)}


def cases():
    rnd = random.Random(0x51ecf0)
    stacks = [['repair'], ['repair'], ['repair', 'sleep'], ['other', 'repair'], [], [None], ['sleep']]
    gtid = 0x00f00f00
    rows = []
    for i in range(800):
        obj = None
        if rnd.random() < 0.95:
            members = None
            if rnd.random() < 0.4:
                members = [None if rnd.random() < 0.15 else {'check_4d1490': rnd.random() < 0.4}
                           for _ in range(rnd.choice([0, 1, 2, 3]))]
            obj = {'gtid': gtid, 'flags8': rnd.choice([0x4000] * 8 + [0, 0xffffbfff, 0x4001]),
                   'word60': rnd.choice([0, 0x6001, 0x6001, 0x6001]),
                   'word100': rnd.choice([0, 0, 0x200000, 0x20000000, 0x20200000, 0xdfdfffff]),
                   'members': members}
        c = {'name': 'repair_accept%d' % i, 'precision': PRECISIONS_LIST[i % 3], 'object': obj,
             'mode': rnd.choice([0, 1, 2, 3, 4]), 'keep': rnd.choice([0, 0, 1, 2]),
             'state_target': rnd.choice([gtid, gtid, 0x00e00e00, 0]),
             'state_byte18': rnd.choice([0, 0, 1, 2]), 'mind_c': rnd.choice([0, 0, 1, 0x3f800000]),
             'owner_1f0': rnd.choice([f32(0.0), f32(0.0), 0x80000000, f32(1.5), f32(-2.0), 0x00000001,
                                      0x80000001]),
             'org_34': rnd.random() < 0.9,
             'can_activate': {'repair': rnd.random() < 0.7},
             'states': {'repair': rnd.choice([0, 1, 2, 2]), 'move': rnd.choice([0, 1, 2]),
                        'sleep': rnd.choice([0, 1, 2]), 'other': rnd.choice([1, 2])},
             'stack': rnd.choice(stacks), 'words': [rnd.getrandbits(32) for _ in range(4)],
             'leave_clears': rnd.random() < 0.3, 'tracked': TRACKED}
        rows.append(c)
    return rows


PRECISIONS_LIST = list(PRECISIONS)


def main():
    out, visited = [], {}
    for c in cases():
        m = AcceptMachine(c)
        result = m.run()
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': result})
    cov = coverage(visited, {'accept_51ecf0': (0x51ecf0, 0x51ee1e), 'ready_member_579f70': (0x579f70, 0x579fcd),
                             'upkeep_44dd70': (0x44dd70, 0x44dd8c)})
    d = header(__file__, __doc__)
    d.update(instructions=instructions(visited), coverage=cov, cases=out)
    p = HERE / 'native-cai-repair-accept.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'faults': sum('fault' in r['output'] for r in out),
                      'accepted': sum(r['output'].get('result', 0) == 1 for r in out),
                      'instructions': len(visited),
                      'coverage': {k: [v['visited'], v['instructions'], v['unvisited']] for k, v in cov.items()},
                      'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
