"""Original KKC_AAIStateGuard::Think 51C900 (aircraft state aai_guard at
Mind+9C) executing unchanged on an emulated aircraft Mind with 529E60,
529C30, 529C20, 432E40, the GTID list predicates 51A550/517F30 and the
target setter 51C540 (native_uai_harness.py), at Single24, Double53 and
Extended64.

Seams (recorded in order): 4D4FF0 on the actor, 569300 on the guarded
object's denizen component (returns the case's GTID list), 51B790 on the
attack state (not ported; a case may script the attack status and +18 word
it leaves, as the real activation would), 5176A0 on the join state (not
ported), the attack state Think, state leave (+1C) and the Updatable +8
scheduler.
"""
from pathlib import Path
import random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_uai_harness import AIRCRAFT_STATES, MindMachine, PRECISIONS, f32, produce

ENTRY, END = 0x51c900, 0x51cc0c
SETTER, SETTER_END = 0x51c540, 0x51c5df


class AirGuardMachine(MindMachine):
    def __init__(self, c):
        super().__init__(c, AIRCRAFT_STATES)
        a = self.actor
        self.w(a + 0x20, c['pos'][0]); self.w(a + 0x24, c['pos'][1])
        self.objects = {}
        for key, o in c['objects'].items():
            p = self.objects[key] = self.obj(key, o['gtid'], registered=False)
            if o['registered']:
                self.register(o['gtid'], p)
            self.w(p + 8, o['flags']); self.w(p + 0x20, o['pos'][0]); self.w(p + 0x24, o['pos'][1])
        self.lists = {}
        for key, o in c['objects'].items():
            if o['threats'] is not None:
                comp = self.alloc(0x40); self.w(self.objects[key] + 0x88, comp)
                h = self.alloc(8); head = 0
                for g in reversed(o['threats']):
                    node = self.alloc(8); self.w(node, g); self.w(node + 4, head); head = node
                self.w(h, head); self.lists[comp] = (key, h)
        g = self.states['guard']
        self.w(g + 0x10, c['guarded']); self.w(g + 0x14, c['current'])
        at = self.states['attack']
        self.w(at + 0x18, c['attack18']); self.w(at + 0x30, c['attack30'])
        self.wb(self.states['move'] + 0x48, c['move48'])
        for addr, fn in [(0x4d4ff0, self.s_relation), (0x569300, self.s_threats), (0x51b790, self.s_attack),
                         (0x5176a0, self.s_join)]:
            self.seam(addr, fn)

    def s_relation(self, m):
        assert self.ecx() == self.actor and self.arg(2) == 1 and self.arg(3) == 0
        k = self.name(self.arg(1)); r = self.c['objects'][k]['relation'] if k else 0
        self.ev('relation', object=k, result=r); self.ret(r & 0xffffffff, 12)

    def s_threats(self, m):
        key, h = self.lists[self.ecx()]
        self.ev('component_threats', object=key); self.ret(h)

    def s_attack(self, m):
        at = self.states['attack']
        assert self.ecx() == at
        self.ev('attack_activate', a=self.arg(1), object=self.name(self.arg(2)), c=self.arg(3))
        after = self.c['attack_after']
        if after is not None:
            self.w(at + 0xc, after['status'])
            if after['word18'] == 'object':
                self.w(at + 0x18, self.r(self.arg(2) + 0x14))
        self.ret(1, 12)

    def s_join(self, m):
        assert self.ecx() == self.states['join']
        self.ev('join_activate', mode=self.arg(1)); self.ret(1, 4)

    def run(self):
        r = self.run_think(ENTRY, 'guard')
        if 'fault' not in r:
            r['current'] = self.r(self.states['guard'] + 0x14)
            r['attack18'] = self.r(self.states['attack'] + 0x18)
        return r


def cases():
    rnd = random.Random(0x51c900)
    rows = []
    for i in range(900):
        keys = ['o%d' % k for k in range(rnd.randrange(1, 6))]
        objects = {}
        for k, key in enumerate(keys):
            objects[key] = {'gtid': (0x80 + k) << 16 | (0x800 + k), 'registered': rnd.random() < 0.85,
                            'flags': 1 if rnd.random() < 0.1 else 0,
                            'pos': [f32(rnd.uniform(-60, 60)), f32(rnd.uniform(-60, 60))],
                            'relation': rnd.choice([0, 0, 0, 1]), 'threats': None}
        gt = lambda: rnd.choice([o['gtid'] for o in objects.values()] * 2 + [0x7ff00800])
        guarded = rnd.choice(keys)
        if rnd.random() < 0.8:
            objects[guarded]['threats'] = [gt() for _ in range(rnd.randrange(0, 5))]
        status = {k: rnd.choice([0, 0, 1, 2]) for k in AIRCRAFT_STATES}
        status['guard'] = rnd.choice([1, 2, 2]); status['attack'] = rnd.choice([0, 1, 2, 2])
        c = {'name': 'airguard%d' % i, 'precision': list(PRECISIONS)[i % 3], 'objects': objects,
             'guarded': rnd.choice([objects[guarded]['gtid']] * 6 + [0, 0x7ff00800]),
             'current': rnd.choice([0, gt(), gt()]), 'attack18': rnd.choice([0, gt(), gt()]),
             'attack30': rnd.choice([0, 4, 1, 2, 4]), 'move48': rnd.choice([0, 1]),
             'pos': [f32(rnd.uniform(-60, 60)), f32(rnd.uniform(-60, 60))],
             'attack_after': rnd.choice([None, {'status': 2, 'word18': 'object'}, {'status': 2, 'word18': None},
                                         {'status': 1, 'word18': None}]),
             'status': status, 'stack': rnd.choice([['guard'], ['guard', 'move'], ['attack', 'guard']])}
        rows.append(c)
    # Nearest-threat ties (o2 is below o1's 25 except after PC24 rounding).
    near = [3.0 - 3 * 2.0 ** -22, 4.0 + 2.0 ** -21]
    for order in (['o1', 'o2'], ['o2', 'o1']):
        for prec in PRECISIONS:
            objects = {'o0': {'gtid': 0x00800800, 'registered': True, 'flags': 0, 'pos': [f32(50.0), f32(50.0)],
                              'relation': 0, 'threats': None},
                       'o1': {'gtid': 0x00810801, 'registered': True, 'flags': 0, 'pos': [f32(3.0), f32(4.0)],
                              'relation': 0, 'threats': None},
                       'o2': {'gtid': 0x00820802, 'registered': True, 'flags': 0, 'pos': [f32(near[0]), f32(near[1])],
                              'relation': 0, 'threats': None}}
            objects['o0']['threats'] = [objects[k]['gtid'] for k in order]
            status = {k: 0 for k in AIRCRAFT_STATES}; status['guard'] = 2
            rows.append({'name': 'airguard_near_%s_%s' % ('_'.join(order), prec), 'precision': prec,
                         'objects': objects, 'guarded': 0x00800800, 'current': 0, 'attack18': 0, 'attack30': 0,
                         'move48': 1, 'pos': [f32(0.0), f32(0.0)], 'attack_after': {'status': 2, 'word18': 'object'},
                         'status': status, 'stack': ['guard']})
    return rows


def main():
    produce(__file__, __doc__, 'native-aai-guard.json', cases(), AirGuardMachine,
            {'51C900': (ENTRY, END), '51C540': (SETTER, SETTER_END)})


if __name__ == '__main__':
    main()
