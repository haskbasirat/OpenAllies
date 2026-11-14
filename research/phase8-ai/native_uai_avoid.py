"""Original element kill-state hand-over: 5358A0 (hand an object to the kill
state, on the move or engage state), 5351C0 (kill state activation on an
object) and 533D90 (kill state target change), executing unchanged on an
emulated element Mind (native_uai_harness.py) with 529C00, 529C20, 529C30
and the Mind activation 52A810 (52A470 / 52A420 / 52A290 stack operations),
at Single24, Double53 and Extended64.

Seams (recorded in order): 4D4FF0 on the actor (relation, scripted result),
4D0D70 on the former target and 4D3A20 on the new target (argument: the
actor), 57DEA0 / 57DD90 on the actor +74 performer (57DD90 with a null
performer is a native null `this`, recorded as a fault; the scratch words it
receives are recorded), 532930 on the move state (all six arguments), actor
virtual +108 (writes scripted scratch words; returns the scratch or another
point) and object virtual +88 (records the point words it is given), the
state virtuals +4/+18/+1C, Mind virtuals +1A0/+1A4 and the Updatable +8
virtual +4 scheduler.
"""
from pathlib import Path
import random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_uai_harness import ELEMENT_STATES, DISCARD, MindMachine, PRECISIONS, f32, produce

HAND, HAND_END = 0x5358a0, 0x53590f
SELECT, SELECT_END = 0x5351c0, 0x53523c
TARGET, TARGET_END = 0x533d90, 0x533ef7
ACTOR_GTID = 0x00400040


class AvoidMachine(MindMachine):
    def __init__(self, c):
        super().__init__({**c, 'actor_gtid': ACTOR_GTID}, ELEMENT_STATES, DISCARD)
        self.objects = {'actor': self.actor}
        self.names[self.actor] = 'actor'
        if c['actor_registered']:
            self.register(ACTOR_GTID, self.actor)
        for key, o in c['objects'].items():
            p = self.objects[key] = self.obj(key, o['gtid'], registered=False)
            if o['registered']:
                self.register(o['gtid'], p)
        for p in self.objects.values():
            self.vslot(p, 0x88, self.v_in_reach)
        self.vslot(self.actor, 0x108, self.v_scratch)
        a = self.actor
        if c['locomotion'] is not None:
            loco = self.alloc(0x80); self.wb(loco + 0x56, c['locomotion']); self.w(a + 0x6c, loco)
        self.performer = 0
        if c['performer']:
            self.performer = self.alloc(0x80); self.w(a + 0x74, self.performer)
        self.w(self.states['kill'] + 0x10, c['kill_target'])
        self.other = self.alloc(16); self.w(self.other, f32(7.5)); self.w(self.other + 4, f32(-2.25))
        self.seam(0x4d4ff0, self.s_relation)
        self.seam(0x4d0d70, self.s_released)
        self.seam(0x4d3a20, self.s_acquired)
        self.seam(0x57dea0, self.s_halt)
        self.seam(0x57dd90, self.s_approach)
        self.seam(0x532930, self.s_move)

    def s_relation(self, m):
        assert self.ecx() == self.actor
        r = self.c['relation']
        self.ev('relation', object=self.name(self.arg(1)), a=self.arg(2), b=self.arg(3), result=r)
        self.ret(r & 0xffffffff, 12)

    def s_released(self, m):
        assert self.arg(1) == self.actor
        self.ev('released', object=self.name(self.ecx())); self.ret(0, 4)

    def s_acquired(self, m):
        assert self.arg(1) == self.actor
        self.ev('acquired', object=self.name(self.ecx())); self.ret(0, 4)

    def s_halt(self, m):
        assert self.ecx() == self.performer and self.performer
        self.ev('halt'); self.ret(0)

    def s_approach(self, m):
        if self.ecx() == 0:
            return self.fault('57DD90 on a null performer')
        assert self.ecx() == self.performer
        p = self.arg(3)
        self.ev('approach', mode=self.arg(1), object=self.name(self.arg(2)), point=[self.r(p), self.r(p + 4)])
        self.ret(0, 12)

    def s_move(self, m):
        assert self.ecx() == self.states['move']
        self.ev('move_to_object', mode=self.arg(1), object=self.name(self.arg(2)), a3=self.arg(3), a4=self.arg(4),
                point=[self.arg(5), self.arg(6)])
        self.ret(0, 0x18)

    def v_scratch(self, m):
        assert self.ecx() == self.actor
        p = self.arg(1)
        before = [self.r(p), self.r(p + 4)]
        s = self.c['scratch']
        if s['write'] is not None:
            self.w(p, s['write'][0]); self.w(p + 4, s['write'][1])
        self.ev('position_ref', before=before)
        self.ret(self.other if s['other'] else p, 4)

    def v_in_reach(self, m):
        p = self.arg(1); r = self.c['in_reach']
        self.ev('in_reach', object=self.name(self.ecx()), point=[self.r(p), self.r(p + 4)], result=r)
        self.ret(int(r), 4)

    def run(self):
        c = self.c
        obj = 0 if c['object'] is None else self.objects[c['object']]
        try:
            if c['entry'] == '5358A0':
                r = self.call(HAND, ecx=self.states[c['this']], args=[obj])
            elif c['entry'] == '5351C0':
                r = self.call(SELECT, ecx=self.states['kill'], args=[c['mode'], obj])
            else:
                r = self.call(TARGET, ecx=self.states['kill'], args=[obj])
        except Exception as e:
            if self.null_accesses:
                return {'fault': 'null dereference', 'events': self.events}
            raise
        if self.null_accesses:
            return {'fault': 'null dereference', 'events': self.events}
        out = dict(events=self.events, **self.snap())
        out['result'] = r & 0xff
        out['kill_target'] = self.r(self.states['kill'] + 0x10)
        return out


def cases():
    rnd = random.Random(0x5351c0)
    rows = []
    for i in range(900):
        objects = {}
        for k in range(rnd.randrange(1, 4)):
            objects['o%d' % k] = {'gtid': (0x50 + k) << 16 | (0x500 + k), 'registered': rnd.random() < 0.85}
        gts = [o['gtid'] for o in objects.values()]
        entry = ['5358A0', '5351C0', '533D90'][i % 3]
        c = {'name': 'avoid%d' % i, 'precision': list(PRECISIONS)[(i // 3) % 3], 'entry': entry,
             'this': rnd.choice(['move', 'engage']), 'mode': rnd.choice([0, 0, 1, 2, 3, 4]),
             'objects': objects, 'actor_registered': rnd.random() < 0.7,
             'object': rnd.choice([None] + list(objects) * 3 + ['actor']),
             'kill_target': rnd.choice(gts * 2 + [0, 0, ACTOR_GTID, 0x7ff00501]),
             'locomotion': rnd.choice([None, 0, 0, 0, 1]), 'performer': rnd.random() < 0.85,
             'relation': rnd.choice([0, 0, 0, 1, 2, -1]), 'can_activate': rnd.random() < 0.7,
             'in_reach': rnd.random() < 0.5,
             'scratch': {'write': rnd.choice([None, [f32(3.0), f32(4.5)], [f32(-1.0), 0]]), 'other': rnd.random() < 0.3},
             'null_point': rnd.choice([[0, 0], [f32(1.5), f32(-8.0)]]),
             'status': {k: rnd.choice([0, 0, 1, 2]) for k in ELEMENT_STATES},
             'stack': rnd.choice([['move'], ['engage'], ['kill', 'engage'], ['engage', 'sleep'], ['move', 'deploy'], []])}
        c['status']['kill'] = rnd.choice([0, 1, 2, 2])
        c['status']['move'] = rnd.choice([0, 1, 2, 2])
        rows.append(c)
    return rows


def main():
    produce(__file__, __doc__, 'native-uai-avoid.json', cases(), AvoidMachine,
            {'5358A0': (HAND, HAND_END), '5351C0': (SELECT, SELECT_END), '533D90': (TARGET, TARGET_END)})


if __name__ == '__main__':
    main()
