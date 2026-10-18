"""Original KKC_AAIStateEscort::Think 51CC10 (aircraft state aai_escort at
Mind+D0) executing unchanged on an emulated aircraft Mind with 529E60,
529C20, the GTID handle check 401060 and the target setter 51C5E0
(native_uai_harness.py).

Seams (recorded in order): 4D0440 on group members (recently attacked),
516DD0 on the aircraft move state (movement-owned), 5684E0 on the group,
51B790 on the attack state (not ported; a case may script the attack status
and +18 word it leaves), 5176A0 on the join state (not ported), the attack
state Think, state leave (+1C) and the Updatable +8 scheduler. Groups are
emulated objects whose +0C is a GTID list; members' body (+60) +28 records
hold the attacker GTID and a binary32 value at +10. A member with a null body
that passes 4D0440 is a native null read (fault).
"""
from pathlib import Path
import random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_uai_harness import AIRCRAFT_STATES, MindMachine, f32, produce

ENTRY, END = 0x51cc10, 0x51cf3d
SETTER, SETTER_END = 0x51c5e0, 0x51c680


class EscortMachine(MindMachine):
    def __init__(self, c):
        super().__init__(c, AIRCRAFT_STATES)
        self.objects = {}
        for key, o in c['objects'].items():
            p = self.objects[key] = self.obj(key, o['gtid'], registered=False)
            if o['registered']:
                self.register(o['gtid'], p)
            if o['record'] is not None:
                body = self.alloc(0x80); self.w(p + 0x60, body)
                self.w(body + 0x28, o['record'][0]); self.w(body + 0x38, o['record'][1])
        self.groups = {}
        for key, members in c['groups'].items():
            g = self.groups[key] = self.obj(key)
            head = 0
            for gt in reversed(members):
                node = self.alloc(8); self.w(node, gt); self.w(node + 4, head); head = node
            self.w(g + 0xc, head)
        e = self.states['escort']
        self.w(e + 0x10, self.groups[c['group']] if c['group'] else 0); self.w(e + 0x14, c['current'])
        if c['words'][3]:
            self.w(self.mind + 0x30, self.groups['g2'])
        at = self.states['attack']
        self.w(at + 0x18, c['attack18']); self.w(at + 0x30, c['attack30'])
        self.wb(self.states['move'] + 0x48, c['move48'])
        for addr, fn in [(0x4d0440, self.s_recent), (0x516dd0, self.s_move_escort), (0x5684e0, self.s_group),
                         (0x51b790, self.s_attack), (0x5176a0, self.s_join)]:
            self.seam(addr, fn)

    def s_recent(self, m):
        k = self.name(self.ecx()); r = self.c['objects'][k]['recent']
        self.ev('recently_attacked', object=k, result=r); self.ret(int(r))

    def s_move_escort(self, m):
        assert self.ecx() == self.states['move']
        self.ev('move_escort', group=self.name(self.arg(1))); self.ret(0, 4)

    def s_group(self, m):
        k = self.name(self.ecx()); r = self.c['group_5684e0']
        self.ev('group_5684e0', group=k, result=r); self.ret(int(r))

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
        r = self.run_think(ENTRY, 'escort')
        if 'fault' not in r:
            r['current'] = self.r(self.states['escort'] + 0x14)
            r['attack18'] = self.r(self.states['attack'] + 0x18)
        return r


def cases():
    rnd = random.Random(0x51cc10)
    rows = []
    for i in range(900):
        keys = ['o%d' % k for k in range(rnd.randrange(1, 7))]
        objects = {}
        for k, key in enumerate(keys):
            objects[key] = {'gtid': (0x90 + k) << 16 | (0x900 + k), 'registered': rnd.random() < 0.85,
                            'recent': rnd.random() < 0.6, 'record': None}
        gt = lambda: rnd.choice([o['gtid'] for o in objects.values()] * 2 + [0, 0x7ff00900])
        for key in keys:
            if rnd.random() < 0.93:
                objects[key]['record'] = [gt(), f32(rnd.choice([0.0, 1.0, 2.5, 2.5, 7.0, rnd.uniform(0, 9)]))]
        groups = {'g1': [gt() for _ in range(rnd.randrange(0, 5))], 'g2': [gt() for _ in range(rnd.randrange(0, 4))]}
        status = {k: rnd.choice([0, 0, 1, 2]) for k in AIRCRAFT_STATES}
        status['escort'] = rnd.choice([1, 2, 2]); status['attack'] = rnd.choice([0, 1, 2, 2])
        c = {'name': 'escort%d' % i, 'precision': 'Double53', 'objects': objects, 'groups': groups,
             'group': rnd.choice(['g1'] * 9 + [None]), 'current': rnd.choice([0, 0, 0, gt()]),
             'words': [0, 0, 0, rnd.choice([0, 2])], 'attack18': rnd.choice([0, gt(), gt()]),
             'attack30': rnd.choice([0, 4, 1]), 'move48': rnd.choice([0, 1]), 'group_5684e0': rnd.random() < 0.5,
             'attack_after': rnd.choice([None, {'status': 2, 'word18': 'object'}, {'status': 2, 'word18': None}]),
             'status': status, 'stack': rnd.choice([['escort'], ['escort', 'move'], ['attack', 'escort']])}
        rows.append(c)
    return rows


def main():
    produce(__file__, __doc__, 'native-aai-escort.json', cases(), EscortMachine,
            {'51CC10': (ENTRY, END), '51C5E0': (SETTER, SETTER_END)})


if __name__ == '__main__':
    main()
