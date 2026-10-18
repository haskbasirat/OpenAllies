"""Original KKC_AAIStateLanding::Think 519750 (aircraft state aai_landing
at Mind+174) executing unchanged on an emulated aircraft Mind with 529E60,
529C30, 529EE0, 516720 and the frame math 517EE0/517DF0
(native_uai_harness.py), at Single24, Double53 and Extended64. The actor
flight component (+BC) and the move state +34 word are emulated data; their
final words are compared.

Seams (recorded in order): object virtual +174(actor), 4D7960 on the actor,
516740 on the aircraft move state (movement-owned; both vectors recorded),
the move state Think and the Updatable +8 scheduler.
"""
from pathlib import Path
import copy, math, random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_uai_harness import AIRCRAFT_STATES, MindMachine, PRECISIONS, f32, produce

ENTRY, END = 0x519750, 0x519f30
HELPERS = {'517EE0': (0x517ee0, 0x517f28), '517DF0': (0x517df0, 0x517ed7)}
FLIGHT_WORDS = (0x14, 0x18, 0x34, 0x80, 0x84, 0x8c)


class LandingMachine(MindMachine):
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
            self.wb(bd + 0x3a8, o['path2'])
            self.vslot(p, 0x174, self.v_accepts)
        self.w(self.states['landing'] + 0x10, c['base'])
        for addr, fn in [(0x4d7960, self.s_lost), (0x516740, self.s_fly)]:
            self.seam(addr, fn)

    def v_accepts(self, m):
        k = self.name(self.ecx()); assert self.arg(1) == self.actor
        r = self.c['objects'][k]['accepts']; self.ev('object_accepts', object=k, result=r); self.ret(int(r), 4)

    def s_lost(self, m):
        assert self.ecx() == self.actor
        self.ev('actor_4d7960'); self.ret(0)

    def s_fly(self, m):
        assert self.ecx() == self.states['move']
        self.ev('move_fly_to', position=[self.arg(i) for i in (1, 2, 3)], direction=[self.arg(i) for i in (4, 5, 6)])
        self.ret(1, 0x18)

    def run(self):
        r = self.run_think(ENTRY, 'landing')
        if 'fault' not in r:
            r['flight'] = {'%x' % off: self.r(self.flight + off) for off in FLIGHT_WORDS}
            r['move34'] = self.r(self.states['move'] + 0x34)
        return r


def leg(rnd):
    a = rnd.uniform(-math.pi, math.pi)
    return ([f32(rnd.uniform(-30, 30)), f32(rnd.uniform(-30, 30)), f32(rnd.choice([0.0, 4.0, rnd.uniform(0, 40)]))],
            [f32(math.cos(a)), f32(math.sin(a))],
            f32(rnd.choice([0.0, 0.0, -0.0, -3.0, 12.0, rnd.uniform(0, 60)])))


def cases():
    rnd = random.Random(0x519750)
    rows = []
    for i in range(900):
        ang = rnd.uniform(-math.pi, math.pi)
        bdef = {}
        for (pt, hd, run) in [(0x38c, 0x398, 0x3a0), (0x374, 0x380, 0x388), (0x3c4, 0x3d0, 0x3d8), (0x3ac, 0x3b8, 0x3c0)]:
            q, h, r = leg(rnd)
            for k in range(3):
                bdef['%x' % (pt + 4 * k)] = q[k]
            bdef['%x' % hd] = h[0]; bdef['%x' % (hd + 4)] = h[1]; bdef['%x' % run] = r
        objects = {'b0': {'gtid': 0x00c00c00, 'registered': rnd.random() < 0.95,
                          'flags': 1 if rnd.random() < 0.05 else 0, 'accepts': rnd.random() < 0.9,
                          'pos': [f32(rnd.uniform(-2000, 2000)) for _ in range(2)] + [f32(rnd.uniform(0, 50))],
                          'heading': [f32(math.cos(ang)), f32(math.sin(ang))], 'definition': bdef,
                          'path2': rnd.choice([0, 1])}}
        c = {'name': 'landing%d' % i, 'precision': list(PRECISIONS)[i % 3], 'objects': objects,
             'base': rnd.choice([0x00c00c00] * 9 + [0x7ff00c00]),
             'flight90': rnd.choice([0, 1, 1, 1]), 'flight': {'34': rnd.choice([0, 0, 0x40000000])},
             'definition': {'40c': f32(rnd.choice([100.0, 150.0, 37.5])), '410': f32(rnd.choice([30.0, 45.0, 1.0 / 3.0]))},
             'status': {k: rnd.choice([0, 0, 1, 2]) for k in AIRCRAFT_STATES},
             'stack': rnd.choice([['landing'], ['landing', 'move']])}
        c['status']['landing'] = rnd.choice([1, 2, 2])
        rows.append(c)
    flying = [c for c in rows if c['flight90'] and c['base'] == 0x00c00c00 and c['objects']['b0']['registered']
              and not c['objects']['b0']['flags'] and c['objects']['b0']['accepts']]
    for k, base in enumerate(flying[:40]):
        for prec in PRECISIONS:
            c = copy.deepcopy(base); c.update(name='landing_same%d_%s' % (k, prec), precision=prec)
            rows.append(c)
    return rows


def main():
    produce(__file__, __doc__, 'native-aai-landing.json', cases(), LandingMachine, {'519750': (ENTRY, END), **HELPERS})


if __name__ == '__main__':
    main()
