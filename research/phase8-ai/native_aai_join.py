"""Original KKC_AAIStateJoin::Think 518D60 (aircraft state aai_join at
Mind+12C) executing unchanged on an emulated aircraft Mind with 529E60,
529C30 and 529EE0 (native_uai_harness.py), at Single24, Double53 and
Extended64.

Seams (recorded in order): 428430 (binary32 vector inequality, computed
exactly, not recorded), object virtual +10 (returns a struct whose +4 is the
case's binary32 limit), object virtual +174(actor), actor virtual +108
(owner), 4E1D90(actor) on the owner, 4D7960 on the actor, Mind virtual +40,
518260 on the landing state (not ported: writes the scripted approach point
and direction into the Think's buffers), 516740 on the aircraft move state
(movement-owned), 45D6F0(0) on the actor definition (ST0 from an exact
binary64), the move state Think and the Updatable +8 scheduler. A null join
object at the re-approach is a native null read (fault).
"""
from pathlib import Path
import random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_uai_harness import AIRCRAFT_STATES, MindMachine, PRECISIONS, f32, from_f32, produce

ENTRY, END = 0x518d60, 0x51911d
JOINABLE, JOINABLE_END = 0x529ee0, 0x529f0a


class AirJoinMachine(MindMachine):
    def __init__(self, c):
        super().__init__(c, AIRCRAFT_STATES)
        self.mind_virtual(0x40, 'mind_virtual_40', 1, fmt=self.name)
        a = self.actor
        for k in range(3):
            self.w(a + 0x20 + 4 * k, c['pos'][k])
        self.objects = {}
        for key, o in c['objects'].items():
            p = self.objects[key] = self.obj(key, o['gtid'], registered=False)
            if o['registered']:
                self.register(o['gtid'], p)
            self.w(p + 8, o['flags'])
            for k in range(3):
                self.w(p + 0x20 + 4 * k, o['pos'][k])
            self.wb(self.r(p + 4) + 0x370, int(o['def370']))
            lim = self.alloc(8); self.w(lim + 4, o['limit'])
            self.vslot(p, 0x10, lambda m, lim=lim: self.ret(lim))
            self.vslot(p, 0x174, self.v_accepts)
        s = self.states['join']
        self.w(s + 0x10, c['target']); self.wb(s + 0x2c, int(c['arrived']))
        for k in range(3):
            self.w(s + 0x14 + 4 * k, c['last'][k]); self.w(s + 0x20 + 4 * k, c['point'][k])
        self.owner = self.obj('owner')
        self.vslot(a, 0x108, self.v_owner)
        self.vslot(self.owner, 0, lambda m: None)
        for addr, fn in [(0x428430, self.s_differ), (0x4e1d90, self.s_base), (0x4d7960, self.s_lost),
                         (0x518260, self.s_approach), (0x516740, self.s_fly), (0x45d6f0, self.s_reach)]:
            self.seam(addr, fn)

    def v_accepts(self, m):
        k = self.name(self.ecx()); assert self.arg(1) == self.actor
        r = self.c['objects'][k]['accepts']; self.ev('object_accepts', object=k, result=r); self.ret(int(r), 4)

    def v_owner(self, m):
        assert self.ecx() == self.actor
        self.ev('actor_owner'); self.ret(self.owner)

    def s_differ(self, m):
        a, b = self.ecx(), self.arg(1)
        va = [from_f32(self.r(a + 4 * k)) for k in range(3)]
        vb = [from_f32(self.r(b + 4 * k)) for k in range(3)]
        self.ret(int(not all(x == y for x, y in zip(va, vb))), 4)

    def s_base(self, m):
        assert self.ecx() == self.owner and self.arg(1) == self.actor
        r = self.c['base']; self.ev('owner_base', result=r); self.ret(0 if r is None else self.objects[r], 4)

    def s_lost(self, m):
        assert self.ecx() == self.actor
        self.ev('actor_4d7960'); self.ret(0)

    def s_approach(self, m):
        s = self.states['join']
        assert self.ecx() == self.states['landing'] and self.arg(2) == s + 0x20
        pt, d = self.arg(2), self.arg(3)
        self.ev('landing_approach', object=self.name(self.arg(1)), point=[self.r(pt + 4 * k) for k in range(3)],
                direction=[self.r(d + 4 * k) for k in range(3)])
        for k in range(3):
            self.w(pt + 4 * k, self.c['approach'][0][k]); self.w(d + 4 * k, self.c['approach'][1][k])
        self.ret(1, 12)

    def s_fly(self, m):
        assert self.ecx() == self.states['move']
        self.ev('move_fly_to', position=[self.arg(i) for i in (1, 2, 3)], direction=[self.arg(i) for i in (4, 5, 6)])
        self.ret(1, 0x18)

    def s_reach(self, m):
        assert self.ecx() == self.r(self.actor + 4) and self.arg(1) == 0
        v = self.c['reach']; self.ev('definition_45d6f0', result=v); self.x87(v, 4)

    def run(self):
        r = self.run_think(ENTRY, 'join')
        if 'fault' not in r:
            s = self.states['join']
            r['target'] = self.r(s + 0x10); r['arrived'] = self.rb(s + 0x2c)
            r['last'] = [self.r(s + 0x14 + 4 * k) for k in range(3)]
            r['point'] = [self.r(s + 0x20 + 4 * k) for k in range(3)]
        return r


def vec(rnd, spread=200.0):
    return [f32(rnd.uniform(-spread, spread)) for _ in range(3)]


def cases():
    rnd = random.Random(0x518d60)
    rows = []
    for i in range(900):
        keys = ['o%d' % k for k in range(rnd.randrange(1, 4))]
        objects = {}
        for k, key in enumerate(keys):
            objects[key] = {'gtid': (0xa0 + k) << 16 | (0xa00 + k), 'registered': rnd.random() < 0.85,
                            'flags': 1 if rnd.random() < 0.1 else 0, 'pos': vec(rnd),
                            'def370': rnd.random() < 0.85, 'accepts': rnd.random() < 0.75,
                            'limit': f32(rnd.choice([5.0, 50.0, 400.0, 1.0 / 3.0]))}
        gt = lambda: rnd.choice([o['gtid'] for o in objects.values()] * 2 + [0, 0x7ff00a00])
        t = rnd.choice(keys)
        last = rnd.choice([list(objects[t]['pos']), vec(rnd), [f32(from_f32(x) + rnd.choice([0.0, 1.0, -3.0])) for x in objects[t]['pos']]])
        c = {'name': 'airjoin%d' % i, 'precision': list(PRECISIONS)[i % 3], 'objects': objects,
             'target': rnd.choice([objects[t]['gtid']] * 3 + [gt()]), 'arrived': rnd.random() < 0.5,
             'last': last, 'point': vec(rnd), 'pos': vec(rnd),
             'words': [0, rnd.choice([0, gt(), gt()]), 0, 0], 'base': rnd.choice([None] + keys),
             'approach': [vec(rnd), [f32(rnd.uniform(-1, 1)) for _ in range(3)]],
             'reach': rnd.choice([10.0, 100.0, 150.0 + 2 ** -30, 1.0 / 3.0, 250.0, rnd.uniform(0, 300)]),
             'status': {k: rnd.choice([0, 0, 1, 2]) for k in AIRCRAFT_STATES},
             'stack': rnd.choice([['join'], ['join', 'move']])}
        c['status']['join'] = rnd.choice([1, 2, 2])
        rows.append(c)
    # Reach ties: twice 45D6F0 exceeds the stored distance 100 only above
    # PC24 (PC24 rounds 100 + 2^-39 to 100).
    for reach in (50.0 + 2.0 ** -40, 50.0, 50.0 - 2.0 ** -40):
        for prec in PRECISIONS:
            o = {'gtid': 0x00a00a00, 'registered': True, 'flags': 0, 'pos': [f32(7.0)] * 3, 'def370': True,
                 'accepts': True, 'limit': f32(400.0)}
            status = {k: 0 for k in AIRCRAFT_STATES}; status['join'] = 2
            rows.append({'name': 'airjoin_reach_%r_%s' % (reach, prec), 'precision': prec, 'objects': {'o0': o},
                         'target': 0x00a00a00, 'arrived': False, 'last': [f32(7.0)] * 3,
                         'point': [f32(100.0), f32(0.0), f32(0.0)], 'pos': [f32(0.0)] * 3, 'words': [0, 0, 0, 0],
                         'base': None, 'approach': [[f32(0.0)] * 3, [f32(0.0)] * 3], 'reach': reach,
                         'status': status, 'stack': ['join']})
    return rows


def main():
    produce(__file__, __doc__, 'native-aai-join.json', cases(), AirJoinMachine,
            {'518D60': (ENTRY, END), '529EE0': (JOINABLE, JOINABLE_END)})


if __name__ == '__main__':
    main()
