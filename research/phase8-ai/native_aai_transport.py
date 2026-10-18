"""Original KKC_AAIStateTransport::Think 5185D0 (aircraft state
aai_transport at Mind+198) executing unchanged on an emulated aircraft Mind
with 529E60 and 529C30 (native_uai_harness.py). The actor flight component
(+BC), base (+C0), actor vector (+2C/+30/+34) and the move state +34 word
are emulated data; their final words are compared.

Seams (recorded in order): 517B60 on the transport state (not ported), actor
virtual +104 (ST0 from an exact binary64), 516740 on the aircraft move state
(movement-owned; both 3-word arguments recorded), 5176A0 on the aircraft join
state (not ported), the move state Think and the Updatable +8 scheduler. A
null base is a native null read (fault).
"""
from pathlib import Path
import random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_uai_harness import AIRCRAFT_STATES, MindMachine, f32, produce

ENTRY, END = 0x5185d0, 0x518711


class TransportMachine(MindMachine):
    def __init__(self, c):
        super().__init__(c, AIRCRAFT_STATES)
        a = self.actor
        self.flight = self.alloc(0x100); self.w(a + 0xbc, self.flight)
        self.w(self.flight + 0x14, 0x11111111); self.w(self.flight + 0x84, 0x22222222)
        self.w(self.states['move'] + 0x34, 0x33333333)
        for k, off in enumerate((0x2c, 0x30, 0x34)):
            self.w(a + off, c['vector'][k])
        if c['base'] is not None:
            b = self.obj('base'); self.w(a + 0xc0, b)
            self.wb(b + 0x4c, int(c['base']['released']))
            self.w(b + 0x50, c['base']['point'][0]); self.w(b + 0x54, c['base']['point'][1])
        self.wb(self.states['transport'] + 0x11, int(c['pending']))
        self.vslot(a, 0x104, self.v_104)
        self.seam(0x517b60, self.s_step)
        self.seam(0x516740, self.s_fly)
        self.seam(0x5176a0, self.s_join)

    def v_104(self, m):
        assert self.ecx() == self.actor
        v = self.c['v104']; self.ev('actor_virtual_104', result=v); self.x87(v, 0)

    def s_step(self, m):
        assert self.ecx() == self.states['transport']
        r = self.c['step']; self.ev('transport_step', result=r); self.ret(int(r))

    def s_fly(self, m):
        assert self.ecx() == self.states['move']
        self.ev('move_fly_to', position=[self.arg(i) for i in (1, 2, 3)], direction=[self.arg(i) for i in (4, 5, 6)])
        self.ret(1, 0x18)

    def s_join(self, m):
        assert self.ecx() == self.states['join']
        self.ev('join_activate', mode=self.arg(1)); self.ret(1, 4)

    def run(self):
        r = self.run_think(ENTRY, 'transport')
        if 'fault' not in r:
            r['pending'] = self.rb(self.states['transport'] + 0x11)
            r['move34'] = self.r(self.states['move'] + 0x34)
            r['flight'] = [self.r(self.flight + 0x14), self.r(self.flight + 0x84)]
        return r


def cases():
    rnd = random.Random(0x5185d0)
    rows = []
    for i in range(120):
        c = {'name': 'transport%d' % i, 'precision': 'Double53', 'pending': rnd.random() < 0.7,
             'step': rnd.random() < 0.3, 'v104': rnd.choice([1.0 / 3.0, 250.0, 2.0 ** -30 * 3, 1e10 / 7, 0.1]),
             'base': rnd.choice([None, {}, {}, {}, {}, {}]),
             'vector': [f32(rnd.choice([0.0, -0.0, 1.5, -2.25, rnd.uniform(-1, 1)])) for _ in range(3)],
             'status': {k: rnd.choice([0, 0, 1, 2]) for k in AIRCRAFT_STATES},
             'stack': rnd.choice([['transport'], ['transport', 'move']])}
        if c['base'] is not None:
            c['base'] = {'released': rnd.random() < 0.6, 'point': [f32(rnd.uniform(-90, 90)), f32(rnd.uniform(-90, 90))]}
        c['status']['transport'] = rnd.choice([0, 1, 2, 2])
        rows.append(c)
    return rows


def main():
    produce(__file__, __doc__, 'native-aai-transport.json', cases(), TransportMachine, {'5185D0': (ENTRY, END)})


if __name__ == '__main__':
    main()
