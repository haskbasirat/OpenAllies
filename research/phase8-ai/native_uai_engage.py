"""Original KKC_UAIStateEngage::Think 536E10 (element state uai_engage at
Mind+12C) executing unchanged on an emulated element Mind with 529E60,
529C30, 529C20, 578D00, 5358A0 and 52E830 (engage reaction delay: SVar
EngageReactionTimeRange cache [B3D2D8] pre-initialised, world RNG 4F02B0 +
inline 85D840) (native_uai_harness.py), at Single24, Double53, Extended64.

Seams (recorded in order): 5210D0 on the organization Mind's engage state
(org+7C -> +4 -> +70, +1CC), 535910 (engage choice, not ported), 5351C0 on
the kill state (not ported), 52E9D0 (return to slot, movement-owned), the
kill state Think (Mind+F8 virtual +C), state leave (+1C) and the Updatable
+8 virtual +4 scheduler.
"""
from pathlib import Path
import random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_uai_harness import ELEMENT_STATES, DISCARD, MindMachine, PRECISIONS, f32, produce

ENTRY, END = 0x536e10, 0x536f3d
HAND, HAND_END = 0x5358a0, 0x53590f
REACT, REACT_END = 0x52e830, 0x52e8f5


class EngageMachine(MindMachine):
    def __init__(self, c):
        super().__init__(c, ELEMENT_STATES, DISCARD)
        self.objects = {}
        for key, o in c['objects'].items():
            p = self.objects[key] = self.obj(key, o['gtid'], registered=False)
            if o['registered']:
                self.register(o['gtid'], p)
        a = self.actor
        self.orgmind = 0
        if c['membership'] is not None:
            ms = self.obj('membership'); self.w(a + 0x80, ms)
            if c['membership']['organization']:
                org = self.obj('organization'); self.w(ms + 0x10, org)
                comp = self.alloc(0x100); self.w(org + 0x7c, comp)
                inner = self.alloc(0x100); self.w(comp + 4, inner)
                self.orgmind = self.alloc(0x400); self.w(inner + 0x70, self.orgmind)
        self.w(0xb3d2dc, self.r(0xb3d2dc) | 1)
        rng = self.alloc(4); self.w(rng, c['range']); self.w(0xb3d2d8, rng)
        e = self.states['engage']
        self.w(e + 0x10, c['target']); self.wb(e + 0x18, int(c['reselect']))
        self.w(self.states['kill'] + 0x10, c['kill_target'])
        self.seam(0x5210d0, self.s_keeps)
        self.seam(0x535910, self.s_select)
        self.seam(0x5351c0, self.s_kill_select)
        self.seam(0x52e9d0, self.s_return)

    def s_keeps(self, m):
        assert self.ecx() == self.orgmind + 0x1cc
        r = self.c['keeps']
        self.ev('regiment_engage_keeps', target=self.name(self.arg(1)), result=r); self.ret(int(r), 4)

    def s_select(self, m):
        assert self.ecx() == self.states['engage'] and self.arg(2) == 0
        r = self.c['select']
        self.ev('engage_select', target=self.name(self.arg(1)), result=r)
        self.ret(0 if r is None else self.objects[r], 8)

    def s_kill_select(self, m):
        assert self.ecx() == self.states['kill']
        self.ev('kill_select', mode=self.arg(1), object=self.name(self.arg(2))); self.ret(int(self.c['kill_result']), 8)

    def s_return(self, m):
        assert self.ecx() == self.states['engage']
        self.ev('return_to_slot'); self.ret(0)

    def run(self):
        r = self.run_think(ENTRY, 'engage')
        if 'fault' not in r:
            r['reselect'] = self.rb(self.states['engage'] + 0x18)
            r['rng'] = self.r(self.world + 0xd4)
        return r


def cases():
    rnd = random.Random(0x536e10)
    rows = []
    for i in range(600):
        objects = {}
        for k in range(rnd.randrange(1, 4)):
            objects['o%d' % k] = {'gtid': (0x50 + k) << 16 | (0x500 + k), 'registered': rnd.random() < 0.85}
        gt = lambda: rnd.choice([o['gtid'] for o in objects.values()] * 2 + [0, 0x7ff00501])
        ms = rnd.choice([None, {'organization': True}, {'organization': True}, {'organization': False}])
        c = {'name': 'engage%d' % i, 'precision': list(PRECISIONS)[i % 3], 'objects': objects,
             'membership': ms, 'target': gt(), 'reselect': rnd.random() < 0.3, 'kill_target': gt(),
             'keeps': rnd.random() < 0.6, 'select': rnd.choice([None] + list(objects) * 2),
             'kill_result': rnd.random() < 0.5, 'rng': rnd.getrandbits(32),
             'range': f32(rnd.choice([0.5, 2.0, 1.0 / 3.0, 1e-5, 0.0, 7.0, 1e-4])),
             'status': {k: rnd.choice([0, 0, 1, 2]) for k in ELEMENT_STATES},
             'stack': rnd.choice([['engage'], ['engage', 'sleep'], ['kill', 'engage']])}
        c['status']['engage'] = rnd.choice([1, 2, 2])
        c['status']['kill'] = rnd.choice([0, 1, 2, 2])
        c['status']['move'] = rnd.choice([0, 1, 2, 2])
        rows.append(c)
    return rows


def main():
    produce(__file__, __doc__, 'native-uai-engage.json', cases(), EngageMachine,
            {'536E10': (ENTRY, END), '5358A0': (HAND, HAND_END), '52E830': (REACT, REACT_END)})


if __name__ == '__main__':
    main()
