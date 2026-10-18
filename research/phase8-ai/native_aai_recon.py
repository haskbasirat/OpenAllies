"""Original KKC_AAIStateRecon::Think 5180B0 (aircraft state aai_recon at
Mind+B4) executing unchanged on an emulated aircraft Mind with 529E60
(native_uai_harness.py).

Seams (recorded in order): 5176A0 on the aircraft join state (Mind+12C, not
ported), the aircraft move state Think (Mind+34 virtual +C) and the
Updatable +8 virtual +4 scheduler.
"""
from pathlib import Path
import random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_uai_harness import AIRCRAFT_STATES, MindMachine, produce

ENTRY, END = 0x5180b0, 0x518105


class ReconMachine(MindMachine):
    def __init__(self, c):
        super().__init__(c, AIRCRAFT_STATES)
        self.wb(self.states['recon'] + 0x10, int(c['done']))
        self.seam(0x5176a0, self.s_join)

    def s_join(self, m):
        assert self.ecx() == self.states['join']
        self.ev('join_activate', mode=self.arg(1)); self.ret(int(self.c['join_result']), 4)

    def run(self):
        return self.run_think(ENTRY, 'recon')


def cases():
    rnd = random.Random(0x5180b0)
    rows = []
    for i in range(40):
        c = {'name': 'recon%d' % i, 'precision': 'Double53', 'done': rnd.random() < 0.5,
             'join_result': rnd.random() < 0.5,
             'status': {k: rnd.choice([0, 0, 1, 2]) for k in AIRCRAFT_STATES},
             'stack': rnd.choice([['recon'], ['recon', 'move'], ['move', 'recon']])}
        c['status']['recon'] = rnd.choice([0, 1, 2, 2])
        rows.append(c)
    return rows


def main():
    produce(__file__, __doc__, 'native-aai-recon.json', cases(), ReconMachine, {'5180B0': (ENTRY, END)})


if __name__ == '__main__':
    main()
