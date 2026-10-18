"""Original KKC_UAIStateJoin::Think 531AD0 (element state uai_join at
Mind+164) executing unchanged on an emulated element Mind with 529E60 and
529C30 (native_uai_harness.py).

Seams (recorded in order): Mind virtual +140 (GTID by value), the embedded
move state Think (Mind+7C virtual +C; a case may rewrite the join GTID there
to exercise the native reread), 576AD0 on the actor locomotion (+6C; a null
locomotion is a native null `this`, recorded as a fault).
"""
from pathlib import Path
import copy, random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_uai_harness import ELEMENT_STATES, DISCARD, MindMachine, PRECISIONS, produce

ENTRY, END = 0x531ad0, 0x531b76


class JoinMachine(MindMachine):
    def __init__(self, c):
        super().__init__(c, ELEMENT_STATES, DISCARD)
        self.mind_virtual(0x140, 'mind_virtual_140', 1)
        self.objects = {}
        for key, o in c['objects'].items():
            p = self.objects[key] = self.obj(key, o['gtid'], registered=False)
            if o['registered']:
                self.register(o['gtid'], p)
        for key, o in c['objects'].items():
            if o.get('null_entry'):
                self.register(o['gtid'], 0)
        a = self.actor
        perf = self.obj('performer'); loco = self.obj('locomotion')
        ptr = {'performer': perf, 'locomotion': loco, None: 0}
        self.w(a + 0xe4, ptr[c['current_performer']]); self.w(a + 0x6c, ptr[c['locomotion']])
        self.w(self.states['join'] + 0x10, c['target'])
        self.seam(0x576ad0, self.s_follow)
        self.w(self.svt + 0xc, self.stub(self.v_think_join))

    def v_think_join(self, m):
        k = self.state_names[self.ecx()]
        self.ev('embedded_think', state=k)
        if k == 'move' and self.c.get('retarget') is not None:
            self.w(self.states['join'] + 0x10, self.c['retarget'])
        self.ret(0)

    def s_follow(self, m):
        if self.ecx() == 0:
            return self.fault(0x576ad0)
        assert self.names[self.ecx()] == 'locomotion'
        self.ev('locomotion_follow', object=self.name(self.arg(1)), delay=self.arg(2)); self.ret(0, 8)

    def run(self):
        r = self.run_think(ENTRY, 'join')
        if 'fault' not in r:
            r['target'] = self.r(self.states['join'] + 0x10)
        return r


def cases():
    rnd = random.Random(0x531ad0)
    rows = []
    for i in range(240):
        objects = {}
        for k in range(rnd.randrange(1, 4)):
            objects['o%d' % k] = {'gtid': (0x20 + k) << 16 | (0x200 + k), 'registered': rnd.random() < 0.8,
                                  'null_entry': False}
        if rnd.random() < 0.15:
            k = rnd.choice(list(objects)); objects[k]['registered'] = False; objects[k]['null_entry'] = True
        gt = lambda: rnd.choice([o['gtid'] for o in objects.values()] + [0, 0x7ff00202])
        c = {'name': 'join%d' % i, 'precision': 'Double53', 'objects': objects, 'target': gt(),
             'retarget': rnd.choice([None, None, None, gt()]),
             'current_performer': rnd.choice([None, 'performer', 'locomotion', 'locomotion']),
             'locomotion': rnd.choice([None, 'locomotion', 'locomotion', 'locomotion']),
             'status': {k: rnd.choice([0, 0, 1, 2]) for k in ELEMENT_STATES},
             'stack': rnd.choice([['join'], ['join', 'sleep'], ['sleep', 'join']])}
        c['status']['join'] = rnd.choice([1, 2, 2])
        rows.append(c)
    return rows


def main():
    produce(__file__, __doc__, 'native-uai-join.json', cases(), JoinMachine, {'531AD0': (ENTRY, END)})


if __name__ == '__main__':
    main()
