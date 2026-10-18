"""Original KKC_AAIStateTakeoff::Think 519150 (aircraft state aai_takeoff
at Mind+15C) executing unchanged on an emulated aircraft Mind with 529E60,
529C30, 52A340 (pop), 529EE0, the move-state setters 516720/5166F0 and the
frame math 517EE0/517DF0 (native_uai_harness.py), at Single24, Double53 and
Extended64. The actor flight component (+BC) and the move state +34 word are
emulated data; their final words are compared.

Seams (recorded in order): object virtual +174(actor), 4D7960 on the actor,
actor virtual +104 (ST0 from an exact binary64), 428430 (binary32 vector
inequality, computed exactly, not recorded), 516740 on the aircraft move
state (movement-owned; both vectors recorded), the move state Think, state
virtuals, Mind virtuals +1A0/+1A4 and the Updatable +8 scheduler.
"""
from pathlib import Path
import math, random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_uai_harness import AIRCRAFT_STATES, MindMachine, PRECISIONS, f32, from_f32, produce

ENTRY, END = 0x519150, 0x51974b
HELPERS = {'517EE0': (0x517ee0, 0x517f28), '517DF0': (0x517df0, 0x517ed7), '516720': (0x516720, 0x51673f),
           '5166F0': (0x5166f0, 0x516716)}
FLIGHT_WORDS = (0x14, 0x18, 0x34, 0x80, 0x84, 0x8c)


class TakeoffMachine(MindMachine):
    def __init__(self, c):
        super().__init__(c, AIRCRAFT_STATES)
        a = self.actor
        d = self.r(a + 4)
        for off, v in c['definition'].items():
            self.w(d + int(off, 16), v)
        self.flight = self.alloc(0x100); self.w(a + 0xbc, self.flight)
        for off in FLIGHT_WORDS:
            self.w(self.flight + off, c['flight'].get('%x' % off, 0x11110000 + off))
        self.wb(self.flight + 0x10, 0x5a); self.wb(self.flight + 0x90, c['flight90'])
        self.w(self.states['move'] + 0x34, 0x33333333)
        self.objects = {}
        for key, o in c['objects'].items():
            p = self.objects[key] = self.obj(key, o['gtid'], registered=False)
            if o['registered']:
                self.register(o['gtid'], p)
            self.w(p + 8, o['flags'])
            for k, v in enumerate(o['pos'] + o['heading']):
                self.w(p + 0x20 + 4 * k, v)
            bd = self.r(p + 4)
            for off, v in o['definition'].items():
                self.w(bd + int(off, 16), v)
            self.vslot(p, 0x174, self.v_accepts)
        s = self.states['takeoff']
        self.wb(s + 0x10, int(c['launched'])); self.w(s + 0x14, c['base'])
        self.vslot(a, 0x104, self.v_104)
        for addr, fn in [(0x4d7960, self.s_lost), (0x428430, self.s_differ), (0x516740, self.s_fly)]:
            self.seam(addr, fn)

    def v_accepts(self, m):
        k = self.name(self.ecx()); assert self.arg(1) == self.actor
        r = self.c['objects'][k]['accepts']; self.ev('object_accepts', object=k, result=r); self.ret(int(r), 4)

    def v_104(self, m):
        assert self.ecx() == self.actor
        v = self.c['v104']; self.ev('actor_virtual_104', result=v); self.x87(v, 0)

    def s_lost(self, m):
        assert self.ecx() == self.actor
        self.ev('actor_4d7960'); self.ret(0)

    def s_differ(self, m):
        a, b = self.ecx(), self.arg(1)
        va = [from_f32(self.r(a + 4 * k)) for k in range(3)]
        vb = [from_f32(self.r(b + 4 * k)) for k in range(3)]
        self.ret(int(not all(x == y for x, y in zip(va, vb))), 4)

    def s_fly(self, m):
        assert self.ecx() == self.states['move']
        self.ev('move_fly_to', position=[self.arg(i) for i in (1, 2, 3)], direction=[self.arg(i) for i in (4, 5, 6)])
        self.ret(1, 0x18)

    def run(self):
        r = self.run_think(ENTRY, 'takeoff')
        if 'fault' not in r:
            r['launched'] = self.rb(self.states['takeoff'] + 0x10)
            r['flight'] = {'%x' % off: self.r(self.flight + off) for off in FLIGHT_WORDS}
            r['flight10'] = self.rb(self.flight + 0x10)
            r['move34'] = self.r(self.states['move'] + 0x34)
        return r


def cases():
    rnd = random.Random(0x519150)
    rows = []
    for i in range(900):
        ang = rnd.uniform(-math.pi, math.pi)
        heading = [f32(math.cos(ang)), f32(math.sin(ang))]
        q1 = [f32(rnd.uniform(-20, 20)), f32(rnd.uniform(-20, 20)), f32(rnd.choice([0.0, 5.0, rnd.uniform(0, 30)]))]
        q2 = rnd.choice([list(q1), [f32(rnd.uniform(-40, 40)), f32(rnd.uniform(-40, 40)), f32(rnd.uniform(0, 60))]])
        a = rnd.uniform(-math.pi, math.pi); b = rnd.uniform(-math.pi, math.pi)
        bdef = {'374': q1[0], '378': q1[1], '37c': q1[2], '380': f32(math.cos(a)), '384': f32(math.sin(a)),
                '388': f32(rnd.choice([0.0, -1.0, 10.0, 25.5, rnd.uniform(0, 50)])),
                '38c': q2[0], '390': q2[1], '394': q2[2], '398': f32(math.cos(b)), '39c': f32(math.sin(b)),
                '3a0': f32(rnd.choice([0.0, -2.0, 15.0, rnd.uniform(0, 80)]))}
        objects = {'b0': {'gtid': 0x00b00b00, 'registered': rnd.random() < 0.95, 'flags': 1 if rnd.random() < 0.05 else 0,
                          'accepts': rnd.random() < 0.9, 'pos': [f32(rnd.uniform(-2000, 2000)) for _ in range(2)] + [f32(rnd.uniform(0, 50))],
                          'heading': heading, 'definition': bdef}}
        c = {'name': 'takeoff%d' % i, 'precision': list(PRECISIONS)[i % 3], 'objects': objects,
             'base': rnd.choice([0x00b00b00] * 9 + [0x7ff00b00]), 'launched': rnd.random() < 0.1,
             'flight90': rnd.choice([0, 1, 1, 1]), 'flight': {'34': rnd.choice([0, 0, 0x40000000])},
             'definition': {'40c': f32(rnd.choice([100.0, 150.0, 37.5])), '410': f32(rnd.choice([30.0, 45.0, 1.0 / 3.0])),
                            '414': f32(rnd.choice([300.0, 250.0]))},
             'v104': rnd.choice([60.0, 1.0 / 3.0, 120.5, rnd.uniform(10, 200)]),
             'status': {k: rnd.choice([0, 0, 1, 2]) for k in AIRCRAFT_STATES},
             'stack': rnd.choice([['takeoff'], ['takeoff', 'move'], ['move', 'takeoff'], []])}
        c['status']['takeoff'] = rnd.choice([1, 2, 2])
        rows.append(c)
    # The same flying inputs at every precision.
    import copy
    flying = [c for c in rows if not c['launched'] and c['flight90'] and c['base'] == 0x00b00b00
              and c['objects']['b0']['registered'] and not c['objects']['b0']['flags'] and c['objects']['b0']['accepts']]
    for k, base in enumerate(flying[:40]):
        for prec in PRECISIONS:
            c = copy.deepcopy(base); c.update(name='takeoff_same%d_%s' % (k, prec), precision=prec)
            rows.append(c)
    return rows


def main():
    produce(__file__, __doc__, 'native-aai-takeoff.json', cases(), TakeoffMachine,
            {'519150': (ENTRY, END), **HELPERS})


if __name__ == '__main__':
    main()
