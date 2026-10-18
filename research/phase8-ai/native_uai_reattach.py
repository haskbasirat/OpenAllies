"""Original KKC_UAIStateReattach::Think 534000 (element state uai_reattach
at Mind+178) executing unchanged on an emulated element Mind with 529E60 and
529C30 (native_uai_harness.py).

Seams (recorded in order): Mind virtual +144, 5709D0 on the membership (a
case scripts the detached byte +1C it leaves, or removes the actor
membership to reach the native null read), 532930 on the move state
(movement-owned order; all six arguments recorded), the embedded move state
Think (Mind+7C virtual +C).
"""
from pathlib import Path
import random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_uai_harness import ELEMENT_STATES, DISCARD, MindMachine, f32, produce

ENTRY, END = 0x534000, 0x5340b3


class ReattachMachine(MindMachine):
    def __init__(self, c):
        super().__init__(c, ELEMENT_STATES, DISCARD)
        self.mind_virtual(0x144, 'mind_virtual_144')
        a = self.actor
        if c['membership'] is not None:
            ms = self.ms = self.obj('membership')
            self.wb(ms + 0x1c, int(c['membership']['detached']))
            if c['membership']['organization']:
                self.w(ms + 0x10, self.obj('organization'))
            self.w(a + 0x80, ms)
        self.seam(0x5709d0, self.s_refresh)
        self.seam(0x532930, self.s_move)

    def s_refresh(self, m):
        assert self.ecx() == self.ms
        self.ev('membership_refresh')
        after = self.c['membership']['after']
        if after == 'removed':
            self.w(self.actor + 0x80, 0)
        else:
            self.wb(self.ms + 0x1c, int(after))
        self.ret(0)

    def s_move(self, m):
        assert self.ecx() == self.states['move']
        self.ev('move_to_object', mode=self.arg(1), object=self.name(self.arg(2)), a3=self.arg(3), a4=self.arg(4),
                point=[self.arg(5), self.arg(6)])
        self.ret(int(self.c['move_result']), 0x18)

    def run(self):
        return self.run_think(ENTRY, 'reattach')


def cases():
    rnd = random.Random(0x534000)
    rows = []
    for i in range(160):
        ms = rnd.choice([None, {}, {}, {}, {}])
        if ms is not None:
            ms.update(detached=rnd.random() < 0.2, organization=rnd.random() < 0.85,
                      after=rnd.choice([False, False, False, True, 'removed']))
        c = {'name': 'reattach%d' % i, 'precision': 'Double53', 'membership': ms,
             'null_point': [f32(rnd.choice([0.0, 3.0e38, -1.0])), f32(rnd.choice([0.0, 3.0e38, 2.0]))],
             'move_result': rnd.random() < 0.5,
             'status': {k: rnd.choice([0, 0, 1, 2]) for k in ELEMENT_STATES},
             'stack': rnd.choice([['reattach'], ['reattach', 'sleep'], ['move', 'reattach']])}
        c['status']['reattach'] = rnd.choice([1, 2, 2])
        rows.append(c)
    return rows


def main():
    produce(__file__, __doc__, 'native-uai-reattach.json', cases(), ReattachMachine, {'534000': (ENTRY, END)})


if __name__ == '__main__':
    main()
