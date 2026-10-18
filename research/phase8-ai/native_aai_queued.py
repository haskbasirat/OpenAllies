"""Original KKC_AAIStateQueued::Think 519F70 (aircraft state aai_queued at
Mind+188) executing unchanged on an emulated aircraft Mind with 529E60 and
529C30 (native_uai_harness.py). The actor flight component (+BC) and base
(+C0) are emulated data; their final words are compared.

Seams (recorded in order): actor virtual +104 (ST0 from an exact binary64),
518550 on the transport state (Mind+198, not ported), 4D7960 on the actor,
and the Updatable +8 virtual +4 scheduler.
"""
from pathlib import Path
import random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_uai_harness import AIRCRAFT_STATES, MindMachine, f32, produce

ENTRY, END = 0x519f70, 0x51a084


class QueuedMachine(MindMachine):
    def __init__(self, c):
        super().__init__(c, AIRCRAFT_STATES)
        a = self.actor
        self.flight = self.alloc(0x100); self.w(a + 0xbc, self.flight)
        for off in (0x8c, 0x18, 0x80):
            self.w(self.flight + off, 0x11111111)
        self.wb(self.flight + 0x10, 0x5a)
        self.w(self.r(a + 4) + 0x414, c['def414'])
        if c['base'] is not None:
            b = self.obj('base'); self.w(a + 0xc0, b)
            self.wb(b + 0x4c, int(c['base']['released']))
            self.w(b + 0x30, c['base']['point'][0]); self.w(b + 0x34, c['base']['point'][1])
        if c['home'] is not None:
            self.obj('home', c['home']['gtid'], registered=c['home']['registered'])
        self.vslot(a, 0x104, self.v_104)
        self.seam(0x518550, self.s_transport)
        self.seam(0x4d7960, self.s_home_lost)

    def v_104(self, m):
        assert self.ecx() == self.actor
        v = self.c['v104']; self.ev('actor_virtual_104', result=v); self.x87(v, 0)

    def s_transport(self, m):
        assert self.ecx() == self.states['transport']
        self.ev('transport_activate', mode=self.arg(1), point=[self.arg(2), self.arg(3)]); self.ret(1, 12)

    def s_home_lost(self, m):
        assert self.ecx() == self.actor
        self.ev('actor_4d7960'); self.ret(0)

    def run(self):
        r = self.run_think(ENTRY, 'queued')
        if 'fault' not in r:
            r['flight'] = {'8c': self.r(self.flight + 0x8c), '18': self.r(self.flight + 0x18),
                           '80': self.r(self.flight + 0x80), '10': self.rb(self.flight + 0x10)}
        return r


def cases():
    rnd = random.Random(0x519f70)
    rows = []
    for i in range(120):
        home = rnd.choice([None, {'gtid': 0x00700700, 'registered': True}, {'gtid': 0x00700700, 'registered': False}])
        c = {'name': 'queued%d' % i, 'precision': 'Double53',
             'base': rnd.choice([None, {}, {}, {}]), 'def414': f32(rnd.choice([120.0, 0.5, 33.25])),
             'v104': rnd.choice([1.0 / 3.0, 250.0, 2.0 ** -30 * 3, 1e10 / 7, 0.1, -5.5]),
             'home': home, 'words': [rnd.getrandbits(32), rnd.choice([0x00700700, 0x00700700, 0, 0x7f000700]), 0, 0],
             'status': {k: rnd.choice([0, 0, 1, 2]) for k in AIRCRAFT_STATES},
             'stack': rnd.choice([['queued'], ['queued', 'move']])}
        if c['base'] is not None:
            c['base'] = {'released': rnd.random() < 0.6, 'point': [f32(rnd.uniform(-90, 90)), f32(rnd.uniform(-90, 90))]}
        c['status']['queued'] = rnd.choice([0, 1, 2, 2])
        rows.append(c)
    return rows


def main():
    produce(__file__, __doc__, 'native-aai-queued.json', cases(), QueuedMachine, {'519F70': (ENTRY, END)})


if __name__ == '__main__':
    main()
