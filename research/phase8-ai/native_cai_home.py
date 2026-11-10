"""Original Mind home setter 52C280(home) executing unchanged on an emulated
regiment Mind (native_cai_regiment.py) with the stack clear 52A3C0 (native).

Emulated data read natively: the home's +8 and +14 words; Mind +4 actor
(the emulated organization object) +8 and +BC words; Mind +28; [B36528]
and its +20 byte.

Seams (recorded in order with the tracked statuses and the Mind stack): the
home virtual +174(actor), the pending-command release 52C0D0 on the Mind,
the strategic-AI reclassification 7A0C20(actor) on [B36528] (reached by a
tail jump), the state virtual +1C (leave), the Mind Updatable +4 (schedule)
and Mind virtuals +1A0/+1A4; allocator, free and loggers as in
native_cai_regiment.py.

Usage: native_cai_home.py (AA.exe is located by the shared harness relative
to this repository).
"""
from pathlib import Path
import json, random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import PRECISIONS, header, instructions, sha
from native_cai_regiment import RegimentMachine, coverage

TRACKED = ['repair', 'move', 'sleep', 'other']
SAI = 0xb36528


class HomeMachine(RegimentMachine):
    def __init__(self, c):
        super().__init__(c)
        self.w(self.org + 8, c['actor_flags8']); self.w(self.org + 0xbc, c['actor_bc'])
        self.home = 0
        if c['home'] is not None:
            h = c['home']
            self.home = self.add_object('home', h['gtid'], True, vtable_slots=[(0x174, self.h_174)])
            self.w(self.home + 8, h['flags8'])
        self.sai = 0
        if c['sai'] is not None:
            self.sai = self.alloc(0x40); self.wb(self.sai + 0x20, c['sai'])
        self.w(SAI, self.sai)
        self.seam(0x52c0d0, self.s_release)
        self.seam(0x7a0c20, self.s_notify)

    def h_174(self, m):
        assert self.ecx() == self.home
        r = self.c['home']['accepts']
        self.ev('home_accepts', actor=self.name_or_word(self.arg(1)), result=r); self.ret(int(r), 4)

    def s_release(self, m):
        assert self.ecx() == self.mind
        self.ev('release_pending'); self.ret(0)

    def s_notify(self, m):
        assert self.ecx() == self.sai
        self.ev('sai_notify', actor=self.name_or_word(self.arg(1))); self.ret(0, 4)

    def run(self):
        self.call(0x52c280, ecx=self.mind, args=(self.home,))
        return {'events': self.events, **self.snap(), 'home_word': self.r(self.mind + 0x28),
                'actor_flags8': self.r(self.org + 8)}


def cases():
    rnd = random.Random(0x52c280)
    stacks = [['repair'], ['repair', 'sleep'], ['other', 'repair'], [], [None], ['sleep']]
    gtid = 0x00a10a10
    rows = []
    for i in range(500):
        home = None
        if rnd.random() < 0.9:
            home = {'gtid': rnd.choice([gtid, gtid, gtid, 0x00b20b20]), 'flags8': rnd.choice([0, 0, 0, 1, 0xfffffffe, 3]),
                    'accepts': rnd.random() < 0.8}
        c = {'name': 'home%d' % i, 'precision': PRECISIONS_LIST[i % 3], 'home': home,
             'actor_flags8': rnd.choice([0, 4, 0x80000001, rnd.getrandbits(32)]),
             'actor_bc': rnd.choice([0, 0, 0x5000]),
             'sai': rnd.choice([None, 0, 1, 1, 0x80]),
             'states': {'repair': rnd.choice([0, 1, 2]), 'move': rnd.choice([0, 1, 2]),
                        'sleep': rnd.choice([0, 1, 2]), 'other': rnd.choice([1, 2])},
             'stack': rnd.choice(stacks),
             'words': [rnd.getrandbits(32), rnd.choice([0, 0, gtid, 0x00c30c30]), rnd.getrandbits(32),
                       rnd.getrandbits(32)],
             'can_activate': {}, 'leave_clears': rnd.random() < 0.3, 'tracked': TRACKED}
        rows.append(c)
    return rows


PRECISIONS_LIST = list(PRECISIONS)


def main():
    out, visited = [], {}
    for c in cases():
        m = HomeMachine(c)
        result = m.run()
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': result})
    cov = coverage(visited, {'set_home_52c280': (0x52c280, 0x52c317), 'clear_52a3c0': (0x52a3c0, 0x52a418)})
    d = header(__file__, __doc__)
    d.update(instructions=instructions(visited), coverage=cov, cases=out)
    p = HERE / 'native-cai-home.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'instructions': len(visited),
                      'notified': sum(any(e['event'] == 'sai_notify' for e in r['output']['events']) for r in out),
                      'coverage': {k: [v['visited'], v['instructions'], v['unvisited']] for k, v in cov.items()},
                      'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
