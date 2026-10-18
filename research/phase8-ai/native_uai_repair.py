"""Original KKC_UAIStateRepair::Think 5359D0 (element state uai_repair at
Mind+148) executing unchanged on an emulated element Mind with 529E60 and
529C30 (native_uai_harness.py), at Single24, Double53 and Extended64.

Seams (recorded in order): 5617C0 (body whole), Mind virtuals +168 +B8 +140,
4D4FF0 (relation), 5340C0 (decision; checks that the Think initialised the
record's ability = 0 and status = 3 words and writes back the scripted
ability/status), 57F290 on the actor performer (+74), 50B250 on Mind+8,
52D660 on the move state (stop action, movement-owned), the move state
Think, 4CF110 (player), 4C8270 (ability value, returned through ST0 from an
exact binary64), 576AD0 on the actor locomotion (+6C). A null ability or
locomotion reaching a callee is a native null dereference (fault).
"""
from pathlib import Path
import random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_uai_harness import ELEMENT_STATES, DISCARD, MindMachine, PRECISIONS, produce

ENTRY, END = 0x5359d0, 0x535b92


class RepairMachine(MindMachine):
    def __init__(self, c):
        super().__init__(c, ELEMENT_STATES, DISCARD)
        self.mind_virtual(0x168, 'mind_virtual_168'); self.mind_virtual(0xb8, 'mind_virtual_b8')
        self.mind_virtual(0x140, 'mind_virtual_140', 1)
        self.objects = {}
        for key, o in c['objects'].items():
            p = self.objects[key] = self.obj(key, o['gtid'], registered=False)
            if o['registered']:
                self.register(o['gtid'], p)
            if o['body']:
                b = self.obj(key + '_body'); self.w(p + 0x60, b)
        for key in ('a0', 'a1'):
            self.objects[key] = self.obj(key)
        a = self.actor
        self.performer = self.obj('performer'); self.w(a + 0x74, self.performer)
        loco = self.obj('locomotion')
        ptr = {'performer': self.performer, 'locomotion': loco, None: 0}
        self.w(a + 0xe4, ptr[c['current_performer']]); self.w(a + 0x6c, ptr[c['locomotion']])
        self.player = self.obj('player')
        s = self.states['repair']
        self.wb(s + 0x10, int(c['notify'])); self.w(s + 0x14, c['target']); self.wb(s + 0x18, int(c['keep']))
        for addr, fn in [(0x5617c0, self.s_whole), (0x4d4ff0, self.s_relation), (0x5340c0, self.s_choose),
                         (0x57f290, self.s_use), (0x50b250, self.s_cancel), (0x52d660, self.s_stop),
                         (0x4cf110, self.s_player), (0x4c8270, self.s_value), (0x576ad0, self.s_follow)]:
            self.seam(addr, fn)

    def s_whole(self, m):
        assert self.names[self.ecx()].endswith('_body')
        r = self.c['whole']; self.ev('body_whole', result=r); self.ret(int(r))

    def s_relation(self, m):
        assert self.ecx() == self.actor and self.arg(2) == 1 and self.arg(3) == 0
        r = self.c['relation']; self.ev('relation', target=self.name(self.arg(1)), result=r); self.ret(r & 0xffffffff, 12)

    def s_choose(self, m):
        assert self.ecx() == self.mind and self.arg(2) == 0 and self.arg(4) == 0
        rec = self.arg(3)
        assert self.r(rec) == 0 and self.r(rec + 8) == 3
        ab, st = self.c['choice']
        self.w(rec, 0 if ab is None else self.objects[ab]); self.w(rec + 8, st)
        self.ev('choose', target=self.name(self.arg(1))); self.ret(st, 16)

    def s_use(self, m):
        assert self.ecx() == self.performer and self.arg(2) == 1 and self.arg(4) == 0
        if self.arg(1) == 0:
            return self.fault(0x57f290)
        r = self.c['use']
        self.ev('performer_use', ability=self.name(self.arg(1)), target=self.name(self.arg(3)), result=r)
        self.ret(int(r), 16)

    def s_cancel(self, m):
        assert self.ecx() == self.mind + 8
        self.ev('cancel_updates'); self.ret(0)

    def s_stop(self, m):
        assert self.ecx() == self.states['move']
        self.ev('move_stop'); self.ret(0)

    def s_player(self, m):
        assert self.ecx() == self.actor
        self.ev('player'); self.ret(self.player)

    def s_value(self, m):
        if self.ecx() == 0:
            return self.fault(0x4c8270)
        assert self.arg(1) == self.player
        v = self.c['value']; self.ev('ability_value', ability=self.name(self.ecx()), result=v); self.x87(v, 4)

    def s_follow(self, m):
        if self.ecx() == 0:
            return self.fault(0x576ad0)
        assert self.names[self.ecx()] == 'locomotion'
        self.ev('locomotion_follow', object=self.name(self.arg(1)), delay=self.arg(2)); self.ret(0, 8)

    def run(self):
        r = self.run_think(ENTRY, 'repair')
        if 'fault' not in r:
            r['notify'] = self.rb(self.states['repair'] + 0x10)
        return r


def cases():
    rnd = random.Random(0x5359d0)
    rows = []
    for i in range(600):
        objects = {}
        for k in range(rnd.randrange(1, 3)):
            objects['o%d' % k] = {'gtid': (0x60 + k) << 16 | (0x600 + k), 'registered': rnd.random() < 0.9,
                                  'body': rnd.random() < 0.85}
        gt = lambda: rnd.choice([o['gtid'] for o in objects.values()] * 4 + [0, 0x7ff00600])
        c = {'name': 'repair%d' % i, 'precision': list(PRECISIONS)[i % 3], 'objects': objects,
             'target': gt(), 'keep': rnd.random() < 0.6, 'notify': rnd.random() < 0.4,
             'whole': rnd.random() < 0.4, 'relation': rnd.choice([0, 0, 0, 1, 3]),
             'choice': [rnd.choice(['a0', 'a1', 'a1', None]), rnd.choice([0, 1, 2, 3, 4, 2])],
             'use': rnd.random() < 0.5,
             'current_performer': rnd.choice([None, 'performer', 'locomotion', 'locomotion']),
             'locomotion': rnd.choice([None, 'locomotion', 'locomotion', 'locomotion']),
             'value': rnd.choice([2.0, 1.0 / 3.0, 1e-4, 1e-4 + 2 ** -40, 0.5 + 2 ** -30, 10.0 / 7.0,
                                  rnd.uniform(0, 5), 3.0 + 2 ** -26 + 2 ** -50]),
             'status': {k: rnd.choice([0, 0, 1, 2]) for k in ELEMENT_STATES},
             'stack': rnd.choice([['repair'], ['repair', 'sleep'], ['move', 'repair']])}
        c['status']['repair'] = rnd.choice([1, 2, 2])
        rows.append(c)
    # The x87 locomotion-delay path at every precision. Values just around
    # binary32 halfway points after subtracting [A34770] expose the PC53
    # double rounding (ties-to-even after the 53-bit rounding).
    import math
    from native_uai_harness import from_f32
    c32 = from_f32(0x38d1b717)
    crafted = []
    for half in (1 + 2 ** -24, 3 + 2 ** -23, 0.75 + 2 ** -26, 5 + 3 * 2 ** -22):
        v = half + c32
        crafted += [v, math.nextafter(v, math.inf), math.nextafter(v, -math.inf)]
    # M + half-ulp(binary32) + c32 with M even at 24 bits and the binary64
    # rounding of v leaving a tiny positive excess: the exact difference is
    # just above the binary32 tie, but PC53 rounds it onto the tie first.
    for e in (16, 17, 18, 20):
        for k in (0, 2, 1):
            crafted.append(2.0 ** e + k * 2.0 ** (e - 23) + 2.0 ** (e - 24) + c32)
    values = [1.0 / 3.0, 1e-4 + 2 ** -40, 0.5 + 2 ** -30, 10.0 / 7.0, 3.0 + 2 ** -26 + 2 ** -50, 1e-4,
              2.0 ** -20 / 3.0, 1e-4 - 2 ** -45] + [rnd.uniform(0, 5) for _ in range(6)] + crafted
    for i, value in enumerate(values * 3):
        c = dict(rows[i], name='repair_delay%d' % i, precision=list(PRECISIONS)[i // len(values)], keep=True,
                 relation=0, choice=['a1', rnd.choice([2, 4, 0x7fffffff])], current_performer='locomotion',
                 locomotion='locomotion', value=value)
        c['objects'] = {k: dict(o, registered=True, body=True) for k, o in c['objects'].items()}
        c['target'] = c['objects']['o0']['gtid']
        rows.append(c)
    return rows


def main():
    produce(__file__, __doc__, 'native-uai-repair.json', cases(), RepairMachine, {'5359D0': (ENTRY, END)})


if __name__ == '__main__':
    main()
