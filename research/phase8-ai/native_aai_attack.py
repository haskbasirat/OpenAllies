"""Original KKC_AAIStateAttack::Think 51B8C0 (aircraft state aai_attack at
Mind+E8) executing unchanged on an emulated aircraft Mind (native_uai_harness.py)
with 529E60, 529C30, 516720 and the inline helpers 401060, 401290, 445250,
45E790 and 46B000, at Single24, Double53 and Extended64.

Mind fields read: +30 (group), +A8/+DC (guard and escort state status),
+1D0 byte. Seams (recorded in order): 55E660 (airborne) and 55E1B0 on the flight
component (actor +BC), 5685B0 on the Mind+30 group, the group object's Mind
(+70) virtual +4C, 4D4FF0 on the actor, the attack-state helpers 518C40,
51A920, 51B0C0, 51B4C0 and 51B290 (writes the case's binary32 out value; a
case may script the +18/+30 words the real helpers would leave), 4F0E50 on
the world (ground height), the actor virtuals +18/+104 and the target
virtuals +18/+104 (80-bit ST0), 516BA0 and the Think of the move state,
5176A0 on the join state and the Updatable +8 scheduler.
"""
from pathlib import Path
import math, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_uai_harness import AIRCRAFT_STATES, MindMachine, PRECISIONS, f32, produce
from unicorn.x86_const import UC_X86_REG_EIP

ENTRY, END = 0x51b8c0, 0x51bfed


def x87_of(v):
    m, e = math.frexp(abs(v))
    return {'significand': int(m * (1 << 64)) & ((1 << 64) - 1), 'sign_exponent': ((v < 0) << 15) | (e - 1 + 16383)}


class AirAttackMachine(MindMachine):
    def __init__(self, c):
        super().__init__(c, AIRCRAFT_STATES)
        a = self.actor
        for off, v in zip((0x20, 0x24, 0x28, 0x2c, 0x30, 0x34), c['actor']['pos'] + c['actor']['dir']):
            self.w(a + off, v)
        self.wb(self.mind + 0x1d0, c['mind_b1d0'])
        self.flight = self.alloc(0x100); self.w(a + 0xbc, self.flight)
        self.vslot(a, 0x18, self.v_actor('actor_virtual_18', 'actor18'))
        self.vslot(a, 0x104, self.v_actor('actor_virtual_104', 'actor104'))
        self.objects = {}
        for key, o in c['objects'].items():
            p = self.objects[key] = self.obj(key, o['gtid'], registered=False)
            if o['registered']:
                self.register(o['gtid'], p)
            for off in (0x7c, 0xbc, 0x6c):
                self.w(p + off, o['w%x' % off])
            for off, v in zip((0x20, 0x24, 0x2c, 0x30, 0x34), o['pos'] + o['dir']):
                self.w(p + off, v)
            self.vslot(p, 0x18, self.v_object('object_virtual_18', key, 'v18'))
            self.vslot(p, 0x104, self.v_object('object_virtual_104', key, 'v104'))
            mind = self.obj(key + '.mind'); self.w(p + 0x70, mind)
            self.vslot(mind, 0x4c, self.v_mind4c(key))
        at = self.states['attack']
        for off in (0x10, 0x18, 0x1c, 0x24, 0x28, 0x2c, 0x30, 0x38):
            self.w(at + off, c['attack']['w%x' % off])
        self.wb(at + 0x14, c['attack']['b14']); self.wb(at + 0x20, c['attack']['b20'])
        for addr, fn in [(0x55e660, self.s_airborne), (0x55e1b0, self.s_reset28), (0x5685b0, self.s_group),
                         (0x4d4ff0, self.s_relation), (0x518c40, self.s_helper('attack_518c40')),
                         (0x51a920, self.s_helper('attack_51a920')), (0x51b0c0, self.s_helper('attack_51b0c0')),
                         (0x51b4c0, self.s_helper('attack_51b4c0')), (0x51b290, self.s_51b290),
                         (0x4f0e50, self.s_ground), (0x516ba0, self.s_move_to), (0x5176a0, self.s_join)]:
            self.seam(addr, fn)

    def x87_ret(self, v, pop=0):
        scratch = self.alloc(16)
        self.u.mem_write(scratch, struct.pack('<QH', v['significand'], v['sign_exponent']))
        stub = self.alloc(16)
        code = b'\xdb\x2d' + struct.pack('<I', scratch) + (b'\xc2' + struct.pack('<H', pop) if pop else b'\xc3')
        self.u.mem_write(stub, code)
        self.u.reg_write(UC_X86_REG_EIP, stub)

    def v_actor(self, event, key):
        def fn(m):
            assert self.ecx() == self.actor
            self.ev(event); self.x87_ret(self.c['actor'][key])
        return fn

    def v_object(self, event, key, field):
        def fn(m):
            assert self.ecx() == self.objects[key]
            self.ev(event, object=key); self.x87_ret(self.c['objects'][key][field])
        return fn

    def v_mind4c(self, key):
        def fn(m):
            r = self.c['objects'][key]['mind4c']
            self.ev('object_mind_4c', object=key, result=r); self.ret(self.objects[r] if r else 0)
        return fn

    def s_airborne(self, m):
        assert self.ecx() == self.flight
        r = self.c['airborne']; self.ev('flight_airborne', result=r); self.ret(int(r))

    def s_reset28(self, m):
        assert self.ecx() == self.flight + 0x28
        self.ev('flight_reset_28'); self.ret(0)

    def s_group(self, m):
        assert self.ecx() == self.c['words'][3]
        r = self.c['group_object']; self.ev('group_5685b0', result=r); self.ret(self.objects[r] if r else 0)

    def s_relation(self, m):
        assert self.ecx() == self.actor and self.arg(2) == 1 and self.arg(3) == 0
        k = self.name(self.arg(1)); r = self.c['objects'][k]['relation'] if k else 0
        self.ev('relation', object=k, result=r); self.ret(r & 0xffffffff, 12)

    def s_helper(self, name):
        def fn(m):
            at = self.states['attack']
            assert self.ecx() == at
            self.ev(name)
            after = self.c['after'].get(name)
            if after:
                for off, v in after.items():
                    self.w(at + int(off, 16), v)
            self.ret(0)
        return fn

    def s_51b290(self, m):
        assert self.ecx() == self.states['attack']
        ok, out = self.c['b290']
        self.ev('attack_51b290', result=ok)
        self.w(self.arg(1), out); self.ret(int(ok), 4)

    def s_ground(self, m):
        assert self.ecx() == self.r(0xb36534) and self.arg(2) == 0x123
        xy = [self.r(self.arg(1)), self.r(self.arg(1) + 4)]
        self.ev('ground_height', xy=xy); self.x87_ret(self.c['ground'], 8)

    def s_move_to(self, m):
        assert self.ecx() == self.mind + 0x34
        self.ev('move_to_point', point=[self.arg(1), self.arg(2), self.arg(3)]); self.ret(0, 12)

    def s_join(self, m):
        assert self.ecx() == self.states['join']
        self.ev('join_activate', mode=self.arg(1)); self.ret(1, 4)

    def run(self):
        r = self.run_think(ENTRY, 'attack')
        if 'fault' not in r:
            at = self.states['attack']
            r['attack'] = {'w%x' % off: self.r(at + off) for off in (0x10, 0x18, 0x24, 0x28, 0x2c, 0x30)}
            r['attack'].update(b14=self.rb(at + 0x14), b20=self.rb(at + 0x20))
            r['flight'] = {'b10': self.rb(self.flight + 0x10), 'w18': self.r(self.flight + 0x18),
                           'w80': self.r(self.flight + 0x80)}
        return r


def cases():
    rnd = random.Random(0x51b8c0)
    rows = []
    for i in range(1200):
        keys = ['o%d' % k for k in range(rnd.randrange(1, 4))]
        objects = {}
        for k, key in enumerate(keys):
            objects[key] = {'gtid': (0x90 + k) << 16 | (0x900 + k), 'registered': rnd.random() < 0.85,
                            'w7c': rnd.choice([0, 0, 1]), 'wbc': rnd.choice([0, 1]), 'w6c': rnd.choice([0, 1]),
                            'pos': [f32(rnd.uniform(-80, 80)), f32(rnd.uniform(-80, 80))],
                            'dir': [f32(rnd.uniform(-1, 1)), f32(rnd.uniform(-1, 1)), f32(rnd.uniform(-1, 1))],
                            'v18': x87_of(rnd.uniform(1, 20)), 'v104': x87_of(rnd.uniform(1, 30)),
                            'relation': rnd.choice([0, 0, 1]), 'mind4c': None}
        for key in keys:
            objects[key]['mind4c'] = rnd.choice([None] + keys)
        gt = lambda: rnd.choice([o['gtid'] for o in objects.values()] * 2 + [0, 0x7ff00900])
        status = {k: rnd.choice([0, 0, 1, 2]) for k in AIRCRAFT_STATES}
        status['attack'] = rnd.choice([1, 2, 2])
        helper_after = {}
        for name in ('attack_518c40', 'attack_51a920', 'attack_51b0c0'):
            if rnd.random() < 0.3:
                helper_after[name] = {'0x18': gt()} if name == 'attack_518c40' else {'0x30': rnd.choice([0, 1, 2, 4])}
        c = {'name': 'airattack%d' % i, 'precision': list(PRECISIONS)[i % 3], 'objects': objects,
             'airborne': rnd.random() < 0.4, 'group_object': rnd.choice(keys * 4 + [None]),
             'words': [0, 0, 0, 0x5000 + rnd.randrange(16)], 'mind_b1d0': rnd.choice([0, 1, 1]),
             'actor': {'pos': [f32(rnd.uniform(-80, 80)), f32(rnd.uniform(-80, 80)), f32(rnd.uniform(0, 60))],
                       'dir': [f32(rnd.uniform(-1, 1)), f32(rnd.uniform(-1, 1)), f32(rnd.uniform(-1, 1))],
                       'actor18': x87_of(rnd.uniform(1, 20)), 'actor104': x87_of(rnd.uniform(0.5, 40))},
             'attack': {'w10': gt(), 'w18': gt(), 'w1c': f32(rnd.uniform(0, 30)),
                        'w24': f32(rnd.uniform(-80, 80)), 'w28': f32(rnd.uniform(-80, 80)), 'w2c': 0,
                        'w30': rnd.choice([0, 1, 1, 2, 3, 4, 5]), 'w38': f32(rnd.uniform(5, 40)),
                        'b14': rnd.choice([0, 1]), 'b20': rnd.choice([0, 1, 1, 1])},
             'b290': [rnd.random() < 0.8, f32(rnd.choice([0.25, 0.5, 2.0, rnd.uniform(0, 3)]))],
             'ground': x87_of(rnd.uniform(-5, 50)), 'after': helper_after,
             'status': status, 'stack': rnd.choice([['attack'], ['attack', 'move'], ['guard', 'attack']])}
        rows.append(c)
    return rows


def main():
    produce(__file__, __doc__, 'native-aai-attack.json', cases(), AirAttackMachine, {'51B8C0': (ENTRY, END)})


if __name__ == '__main__':
    main()
