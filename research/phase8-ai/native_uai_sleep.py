"""Original KKC_UAIStateSleep::Think 52F950 (element state uai_sleep at
Mind+68) executing unchanged on an emulated element Mind with 529E60,
529C30, 52A000, the Tai stack-action constructors 51E800/51E850 and the
action base constructor/destructor 4D9E20/4CA2D0 (action registry
[B36530]+298) (native_uai_harness.py), at Single24, Double53, Extended64.

Seams (recorded in order): the Updatable +8 virtual +4 scheduler, 4D1820 on
the actor (pending point; writes the scripted point), 52DF60 on the move
state (movement-owned order; seven arguments recorded), Mind virtual +FC,
the flag predicates 4D1080/4D10A0/4D0980 (on the leader or the actor),
leader virtual +0 (ST0 from an exact binary64), 409230 (vector normalise:
records the vector it receives, writes the scripted result and returns a
length through ST0 that the Think discards), 4D9B60(actor) on the stack
action (records its vtable, definition, flags and +18..+24 words) and
52BF30 on the Mind (resume pending command).
"""
from pathlib import Path
import math, random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_uai_harness import ELEMENT_STATES, DISCARD, MindMachine, PRECISIONS, f32, from_f32, produce

ENTRY, END = 0x52f950, 0x52fc1f
UNPACK_CTOR, UNPACK_CTOR_END = 0x51e800, 0x51e84a
PACK_CTOR, PACK_CTOR_END = 0x51e850, 0x51e864


class SleepMachine(MindMachine):
    def __init__(self, c):
        super().__init__(c, ELEMENT_STATES, DISCARD)
        self.mind_virtual(0xfc, 'mind_virtual_fc')
        a = self.actor
        self.w(a + 0x110, c['word110']); self.w(a + 0x100, c['flags100']); self.w(a + 0xac, c['word_ac'])
        defn = self.r(a + 4); r = self.alloc(0x100); self.w(defn + 0x534, r); self.w(r + 0x54, c['reach'])
        self.leader = 0
        if c['membership'] is not None:
            ms = self.obj('membership'); self.w(a + 0x80, ms)
            m = c['membership']
            self.wb(ms + 0x1c, int(m['detached'])); self.w(ms + 0x18, m['index'])
            if m['organization'] is not None:
                o = m['organization']
                org = self.obj('organization'); self.w(ms + 0x10, org)
                comp = self.alloc(0x100); self.w(org + 0x7c, comp)
                table = self.alloc(8 * 8)
                for k, (x, y) in enumerate(o['table']):
                    self.w(table + 8 * k, x); self.w(table + 8 * k + 4, y)
                self.w(comp + 0x78, table)
                if o['leader'] is not None:
                    ld = self.leader = self.obj('leader'); self.w(comp + 0x44, ld)
                    self.w(ld + 0xac, o['leader']['word_ac'])
                    self.w(ld + 0x20, o['leader']['position'][0]); self.w(ld + 0x24, o['leader']['position'][1])
                    self.vslot(ld, 0, self.v_leader0)
        self.w(self.states['sleep'] + 0x10, c['due'])
        reg = self.alloc(0x400); table = self.alloc(0x100); self.w(reg + 0x298, table); self.w(0xb36530, reg)
        for ordinal, value in c['definitions'].items():
            self.w(table + 4 * int(ordinal), value)
        for addr, fn in [(0x4d1820, self.s_pending), (0x52df60, self.s_move), (0x4d1080, self.flag('4d1080')),
                         (0x4d10a0, self.flag('4d10a0')), (0x4d0980, self.flag('4d0980')),
                         (0x409230, self.s_normalize), (0x4d9b60, self.s_issue), (0x52bf30, self.s_resume)]:
            self.seam(addr, fn)

    def s_pending(self, m):
        assert self.ecx() == self.actor
        p = self.c['pending_point']
        self.ev('actor_pending_point', result=p)
        if p is not None:
            self.w(self.arg(1), p[0]); self.w(self.arg(1) + 4, p[1])
        self.ret(int(p is not None), 4)

    def s_move(self, m):
        assert self.ecx() == self.states['move']
        self.ev('move_activate', args=[self.arg(i) for i in range(1, 8)])
        self.ret(int(self.c['move_result']), 0x1c)

    def flag(self, name):
        def fn(m):
            who = self.name(self.ecx())
            assert who in ('leader', 'actor')
            r = self.c['flags'][who][name]
            self.ev('object_' + name, object=who, result=r); self.ret(int(r))
        return fn

    def v_leader0(self, m):
        assert self.ecx() == self.leader
        v = self.c['leader_v0']; self.ev('leader_virtual_0', result=v); self.x87(v, 0)

    def s_normalize(self, m):
        p = self.ecx()
        before = [self.r(p), self.r(p + 4)]
        after = self.c['normalized']
        self.ev('normalize', vector=before, result=after)
        self.w(p, after[0]); self.w(p + 4, after[1])
        self.x87(self.c['normalized_length'], 0)

    def s_issue(self, m):
        a = self.ecx()
        assert self.arg(1) == self.actor
        r = self.c['issue']
        self.ev('issue_action', vtable=self.r(a), definition=self.r(a + 8), flags=self.r(a + 0xc),
                words=[self.r(a + 0x18 + 4 * i) for i in range(4)], result=r)
        self.ret(int(r), 4)

    def s_resume(self, m):
        assert self.ecx() == self.mind
        r = self.c['resume']; self.ev('resume_pending', result=r); self.ret(int(r))

    def run(self):
        r = self.run_think(ENTRY, 'sleep')
        if 'fault' not in r:
            r['due'] = self.r(self.states['sleep'] + 0x10)
        return r


def cases():
    rnd = random.Random(0x52f950)
    c32 = from_f32(0x38d1b717)
    crafted = [2.0 ** e + k * 2.0 ** (e - 23) + 2.0 ** (e - 24) + c32 for e in (4, 8, 16, 20) for k in (0, 1, 2)]
    rows = []
    for i in range(720):
        time = rnd.choice([10.0, 100.0, 1234.5678, 1.0 / 3.0, 7777.7])
        table = [(f32(rnd.choice([0.0, 1.5, -3.25, rnd.uniform(-20, 20)])), f32(rnd.choice([0.0, 2.0, rnd.uniform(-20, 20)])))
                 for _ in range(4)]
        leader = rnd.choice([None, {}, {}, {}, {}, {}])
        if leader is not None:
            leader.update(word_ac=rnd.choice([0, 1, 1, 1]), position=[f32(rnd.uniform(-500, 500)), f32(rnd.uniform(-500, 500))])
        org = rnd.choice([None, {}, {}, {}, {}, {}])
        if org is not None:
            org.update(table=table, leader=leader)
        ms = rnd.choice([None, {}, {}, {}, {}, {}])
        if ms is not None:
            ms.update(detached=rnd.random() < 0.3, index=rnd.randrange(0, 4), organization=org)
        nx, ny = rnd.uniform(-1, 1), rnd.uniform(-1, 1)
        c = {'name': 'sleep%d' % i, 'precision': list(PRECISIONS)[i % 3],
             'time': f32(time), 'due': f32(rnd.choice([time - 1.0, time - 1.0, time, time + 1.0, 0.0])),
             'word110': rnd.choice([0, 0, 0x1234]), 'flags100': rnd.choice([0, 0, 0x1000000, 0x200]),
             'pending_point': rnd.choice([None, [f32(rnd.uniform(-90, 90)), f32(rnd.uniform(-90, 90))]]),
             'move_result': rnd.random() < 0.5, 'word_ac': rnd.choice([0, 1, 1, 1]),
             'reach': f32(rnd.choice([0.0, 1.5, 4.0, 1.0 / 3.0, 12.0])),
             'leader_v0': rnd.choice([2.0, 1.0 / 3.0, 5.5, 25.0, rnd.uniform(0, 30)] + crafted),
             'normalized': [f32(nx), f32(ny)], 'normalized_length': rnd.choice([1.0, 0.5]),
             'membership': ms, 'definitions': {'11': rnd.choice([0, 0x7ab0000]), '12': rnd.choice([0, 0x7ac0000])},
             'flags': {'leader': {'4d1080': rnd.random() < 0.6, '4d10a0': rnd.random() < 0.4, '4d0980': rnd.random() < 0.4},
                       'actor': {'4d1080': False, '4d10a0': rnd.random() < 0.7, '4d0980': False}},
             'issue': rnd.random() < 0.5, 'resume': rnd.random() < 0.5,
             'null_point': [f32(rnd.choice([0.0, -1.0])), f32(rnd.choice([0.0, 3.0e38]))],
             'status': {k: rnd.choice([0, 0, 1, 2]) for k in ELEMENT_STATES},
             'stack': rnd.choice([['sleep'], ['sleep'], ['sleep', 'move'], ['move', 'sleep']])}
        c['status']['sleep'] = rnd.choice([1, 2, 2])
        rows.append(c)
    # The leader-offset x87 path at every precision (crafted values put the
    # reach sum just past binary32 ties so PC53 double rounding shows).
    base = next(c for c in rows if c['membership'] and c['membership']['organization']
                and c['membership']['organization']['leader'])
    for i, v in enumerate(crafted + [1.0 / 3.0, 25.0 + 2 ** -30]):
        for k, prec in enumerate(PRECISIONS):
            c = dict(base, name='sleep_leader%d_%s' % (i, prec), precision=prec, leader_v0=v,
                     reach=f32(rnd.choice([0.0, 0.0, 1.0 / 3.0])), due=f32(0.0), word110=0, word_ac=1,
                     flags={'leader': {'4d1080': True, '4d10a0': False, '4d0980': False},
                            'actor': {'4d1080': False, '4d10a0': True, '4d0980': False}})
            ms = dict(base['membership'], index=i % 4)
            org = dict(ms['organization'], leader=dict(ms['organization']['leader'], word_ac=1))
            # Offsets shorter than the reach so the scaling runs.
            org['table'] = [(f32(0.25), f32(-0.5))] * 4
            ms['organization'] = org; c['membership'] = ms
            rows.append(c)
    return rows


def main():
    produce(__file__, __doc__, 'native-uai-sleep.json', cases(), SleepMachine,
            {'52F950': (ENTRY, END), '51E800': (UNPACK_CTOR, UNPACK_CTOR_END), '51E850': (PACK_CTOR, PACK_CTOR_END)})


if __name__ == '__main__':
    main()
