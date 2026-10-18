"""Original KKC_UAIStateUnload::Think 52F440 (element state uai_unload at
Mind+34) executing unchanged on an emulated element Mind with 529E60, 529C30,
52A340 (pop), 4D09A0 (home GTID lookup), 52EAE0 (join state activation) and
52A810/52A420/52A290 (mode 2 activation of the join state)
(native_uai_harness.py), at Single24, Double53 and Extended64.

Seams (recorded in order): 58BCB0 on the actor transport component (+C0; a
case may rewrite its +3C/+40 counts there; a null component is a native null
`this`, recorded as a fault), 52DF60 on the move state (movement-owned
order; all seven arguments recorded), the state virtuals +4/+18/+1C, Mind
virtuals +1A0/+1A4 and the Updatable +8 virtual +4 scheduler.
"""
from pathlib import Path
import random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_uai_harness import ELEMENT_STATES, DISCARD, MindMachine, PRECISIONS, f32, produce

ENTRY, END = 0x52f440, 0x52f558
JOIN_ACTIVATE, JOIN_ACTIVATE_END = 0x52eae0, 0x52eb4c


class UnloadMachine(MindMachine):
    def __init__(self, c):
        super().__init__(c, ELEMENT_STATES, DISCARD)
        self.objects = {}
        for key, o in c['objects'].items():
            p = self.objects[key] = self.obj(key, o['gtid'], registered=False)
            if o['registered']:
                self.register(o['gtid'], p)
        a = self.actor
        self.w(self.r(a + 4) + 0x41c, c['interval'])
        self.w(a + 0x104, c['actor_flags'])
        if c['locomotion']:
            self.w(a + 0x6c, self.obj('locomotion'))
        self.transport = 0
        if c['transport'] is not None:
            t = self.transport = self.obj('transport')
            tc = c['transport']
            self.w(t + 0x3c, tc['capacity'] & 0xffffffff); self.w(t + 0x40, tc['loaded'] & 0xffffffff)
            self.wb(t + 0x4c, int(tc['byte4c'])); self.w(t + 0x50, tc['point'][0]); self.w(t + 0x54, tc['point'][1])
            self.w(a + 0xc0, t)
        self.w(self.states['unload'] + 0x10, c['due'])
        self.seam(0x58bcb0, self.s_update)
        self.seam(0x52df60, self.s_move)

    def s_update(self, m):
        if self.ecx() == 0:
            return self.fault(0x58bcb0)
        assert self.ecx() == self.transport
        self.ev('transport_update')
        after = self.c['transport'].get('after')
        if after is not None:
            self.w(self.transport + 0x40, after & 0xffffffff)
        self.ret(0)

    def s_move(self, m):
        assert self.ecx() == self.states['move']
        self.ev('move_activate', args=[self.arg(i) for i in range(1, 8)])
        self.ret(int(self.c['move_result']), 0x1c)

    def run(self):
        r = self.run_think(ENTRY, 'unload')
        if 'fault' not in r:
            r['due'] = self.r(self.states['unload'] + 0x10)
            r['join_target'] = self.r(self.states['join'] + 0x10)
        return r


def cases():
    rnd = random.Random(0x52f440)
    rows = []
    for i in range(330):
        objects = {}
        for k in range(rnd.randrange(1, 3)):
            objects['o%d' % k] = {'gtid': (0x40 + k) << 16 | (0x400 + k), 'registered': rnd.random() < 0.8}
        home = rnd.choice([o['gtid'] for o in objects.values()] * 3 + [0, 0x7ff00400])
        time = rnd.choice([10.0, 100.0, 1234.5678, 1.0 / 3.0])
        due = rnd.choice([time - 1.0, time - 1.0, time - 1.0, time, time + 1.0, 0.0])
        cap = rnd.choice([0, 2, 3, -1])
        loaded = rnd.choice([cap - 1, cap, cap, cap + 1, 0])
        c = {'name': 'unload%d' % i, 'precision': list(PRECISIONS)[i % 3], 'objects': objects,
             'time': f32(time), 'due': f32(due),
             'interval': f32(rnd.choice([3.0, 0.1, 1.0 / 3.0, 7.25, 1e-3, 1e7 / 3.0])),
             'actor_flags': rnd.choice([0, 0, 0, 0x40, 0x41, 0x80]),
             'locomotion': rnd.random() < 0.8,
             'transport': None if rnd.random() < 0.06 else {
                 'capacity': cap, 'loaded': loaded, 'after': rnd.choice([None, None, cap, cap - 1]),
                 'byte4c': rnd.random() < 0.5, 'point': [f32(rnd.uniform(-50, 50)), f32(rnd.uniform(-50, 50))]},
             'null_point': [f32(rnd.choice([0.0, -1.0])), f32(rnd.choice([0.0, 3.0e38]))],
             'move_result': rnd.random() < 0.5, 'can_activate': rnd.random() < 0.8,
             'words': [rnd.getrandbits(32), home, 0, 0],
             'status': {k: rnd.choice([0, 0, 1, 2]) for k in ELEMENT_STATES},
             'stack': rnd.choice([['unload'], ['unload', 'sleep'], ['unload', 'move', 'sleep'], ['sleep', 'unload'],
                                  ['unload'], ['unload', 'join'], [], [None]])}
        c['status']['unload'] = rnd.choice([1, 2, 2])
        rows.append(c)
    return rows


def main():
    produce(__file__, __doc__, 'native-uai-unload.json', cases(), UnloadMachine,
            {'52F440': (ENTRY, END), '52EAE0': (JOIN_ACTIVATE, JOIN_ACTIVATE_END)})


if __name__ == '__main__':
    main()
