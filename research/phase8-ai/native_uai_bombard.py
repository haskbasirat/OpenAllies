"""Original KKC_UAIStateBombard::Think 5353E0 (element state uai_bombard at
Mind+10C) executing unchanged on an emulated element Mind with 529E60,
529C30, 529C20, 4DB0F0, 432E40, 4CF1B0 and the member iterator 4364C0
(native_uai_harness.py), at Single24, Double53 and Extended64.

Seams (recorded in order): actor virtuals +108 (owner) and +24 (name, a
refcounted fake CString released by the Think through the fake string
manager's virtual +4, not recorded), target virtual +5C, 42D470 (cdecl
report), 42A530 on world+30 and 54F3C0 on its zone, Mind virtuals +140 +144
+124, 4D4FF0 and 4D5180 on the actor, 57DD90 on the performer (+74), 52DF60
on the move state (movement-owned; seven arguments), the move state Think,
state leave (+1C), 5340C0 (records target/point/by_point and the record it
receives, writes the scripted record) and 5307A0 (records the record and the
point). State+14 is a fake CString whose length (data-0xC) is a case input.
"""
from pathlib import Path
import random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_uai_harness import ELEMENT_STATES, DISCARD, MindMachine, PRECISIONS, f32, produce

ENTRY, END = 0x5353e0, 0x53589b
REPORTER = 0x0bad9d8c


class BombardMachine(MindMachine):
    def __init__(self, c):
        super().__init__(c, ELEMENT_STATES, DISCARD)
        self.mind_virtual(0x140, 'mind_virtual_140', 1)
        self.mind_virtual(0x144, 'mind_virtual_144'); self.mind_virtual(0x124, 'mind_virtual_124')
        a = self.actor
        self.w(a + 0x20, c['pos'][0]); self.w(a + 0x24, c['pos'][1])
        self.perf = 0
        if c['performer'] is not None:
            p = self.perf = self.obj('performer'); self.w(a + 0x74, p); self.w(p + 4, a)
            if c['performer']['current']:
                self.w(a + 0xe4, p)
            self.w(p + 0x10, c['performer']['stage'])
        if c['locomotion']:
            self.w(a + 0x6c, self.obj('locomotion'))
        if c['organization']:
            ms = self.obj('membership'); self.w(a + 0x80, ms); self.w(ms + 0x10, self.obj('organization'))
        self.objects = {}
        for key, o in c['objects'].items():
            p = self.objects[key] = self.obj(key, o['gtid'], registered=False)
            if o['registered']:
                self.register(o['gtid'], p)
            self.w(p + 0x20, o['pos'][0]); self.w(p + 0x24, o['pos'][1]); self.w(p + 0x6c, o['word6c'])
            self.vslot(p, 0x5c, self.v_5c)
        for key, o in c['objects'].items():
            if o['members'] is not None:
                comp = self.alloc(0x40)
                arr = self.array([0 if m is None else self.objects[m] for m in o['members']])
                self.w(comp + 0x28, arr); self.w(comp + 0x2c, len(o['members']))
                self.w(self.objects[key] + 0x7c, comp)
        s = self.states['bombard']
        self.w(s + 0x10, c['target'])
        hdr = self.alloc(0x20); self.w(hdr + 4, c['area_length']); self.w(s + 0x14, hdr + 0x10)
        self.w(s + 0x18, c['point'][0]); self.w(s + 0x1c, c['point'][1])
        self.w(0xb39d8c, REPORTER)
        self.zone_mgr = self.alloc(0x40); self.w(self.world + 0x30, self.zone_mgr)
        self.zone = self.alloc(0x40)
        mgr = self.alloc(0x10); mvt = self.alloc(0x10); self.w(mgr, mvt)
        self.w(mvt + 4, self.stub(lambda m: self.ret(0, 4)))
        self.string_mgr = mgr; self.strings = {}
        self.vslot(a, 0x24, self.v_name); self.vslot(a, 0x108, self.v_owner)
        for addr, fn in [(0x42d470, self.s_report), (0x42a530, self.s_zone), (0x54f3c0, self.s_zone_check),
                         (0x4d4ff0, self.s_relation), (0x4d5180, self.s_point_relation), (0x57dd90, self.s_approach),
                         (0x52df60, self.s_move), (0x5340c0, self.s_choose), (0x5307a0, self.s_apply)]:
            self.seam(addr, fn)

    def cstring(self, text, refs=1):
        hdr = self.alloc(0x20)
        self.w(hdr, self.string_mgr); self.w(hdr + 0xc, refs)
        self.strings[hdr + 0x10] = text
        return hdr + 0x10

    def v_name(self, m):
        assert self.ecx() == self.actor
        dst = self.arg(1); self.w(dst, self.cstring('actor name'))
        self.ev('actor_name'); self.ret(dst, 4)

    def v_owner(self, m):
        assert self.ecx() == self.actor
        r = self.c['owner']; self.ev('actor_owner', result=r); self.ret(r)

    def v_5c(self, m):
        k = self.name(self.ecx()); r = self.c['objects'][k]['v5c']
        self.ev('target_virtual_5c', target=k, owner=self.arg(1), flag=self.arg(2), result=r); self.ret(int(r), 8)

    def s_report(self, m):
        assert self.arg(1) == REPORTER
        p = self.arg(4)
        self.ev('report', kind=self.arg(2), owner=self.arg(3), point=[self.r(p), self.r(p + 4)],
                name=self.strings[self.arg(5)])
        self.ret(0)

    def s_zone(self, m):
        assert self.ecx() == self.zone_mgr
        self.ev('zone_at', point=[self.arg(1), self.arg(2)]); self.ret(self.zone, 8)

    def s_zone_check(self, m):
        assert self.ecx() == self.zone
        r = self.c['zone_check']; self.ev('zone_check', owner=self.arg(1), result=r); self.ret(int(r), 4)

    def s_relation(self, m):
        assert self.ecx() == self.actor
        r = self.c['relation']
        self.ev('relation', target=self.name(self.arg(1)), a=self.arg(2), b=self.arg(3), result=r); self.ret(r & 0xffffffff, 12)

    def s_point_relation(self, m):
        assert self.ecx() == self.actor
        p = self.arg(1); r = self.c['point_relation']
        self.ev('point_relation', point=[self.r(p), self.r(p + 4)], flag=self.arg(2), result=r); self.ret(r & 0xffffffff, 8)

    def s_approach(self, m):
        assert self.ecx() == self.perf
        p = self.arg(3)
        self.ev('performer_approach', mode=self.arg(1), object=self.name(self.arg(2)), point=[self.r(p), self.r(p + 4)])
        self.ret(0, 12)

    def s_move(self, m):
        assert self.ecx() == self.states['move']
        self.ev('move_activate', args=[self.arg(i) for i in range(1, 8)])
        self.ret(1, 0x1c)

    def rec(self, p):
        w = [self.r(p + 4 * k) for k in range(7)]
        w[0] = self.name(w[0]); w[4] = self.name(w[4])
        return w

    def s_choose(self, m):
        assert self.ecx() == self.mind
        pt = self.arg(2)
        self.ev('choose', target=self.name(self.arg(1)), point=None if pt == 0 else [self.r(pt), self.r(pt + 4)],
                by_point=self.arg(4), before=self.rec(self.arg(3)))
        out = self.arg(3); r = self.c['record']
        for k, v in enumerate(r):
            self.w(out + 4 * k, self.objects[v] if isinstance(v, str) else v)
        self.ret(r[2], 16)

    def s_apply(self, m):
        assert self.ecx() == self.mind
        pt = self.arg(2)
        self.ev('apply', record=self.rec(self.arg(1)), point=None if pt == 0 else [self.r(pt), self.r(pt + 4)])
        self.ret(0, 8)

    def run(self):
        return self.run_think(ENTRY, 'bombard')


def cases():
    rnd = random.Random(0x5353e0)
    rows = []
    for i in range(900):
        keys = ['o%d' % k for k in range(rnd.randrange(1, 5))]
        objects = {}
        for k, key in enumerate(keys):
            objects[key] = {'gtid': (0x70 + k) << 16 | (0x700 + k), 'registered': rnd.random() < 0.85,
                            'pos': [f32(rnd.uniform(-100, 100)), f32(rnd.uniform(-100, 100))],
                            'word6c': rnd.choice([0, 1, 1]), 'v5c': rnd.random() < 0.6, 'members': None}
        tgt = rnd.choice(keys)
        if rnd.random() < 0.6:
            objects[tgt]['members'] = [rnd.choice(keys + [None]) for _ in range(rnd.randrange(0, 5))]
        gt = lambda: rnd.choice([objects[tgt]['gtid']] * 4 + [o['gtid'] for o in objects.values()] + [0, 0x7ff00700])
        c = {'name': 'bombard%d' % i, 'precision': list(PRECISIONS)[i % 3], 'objects': objects,
             'target': gt(), 'area_length': rnd.choice([0, 0, 5]),
             'point': [f32(rnd.uniform(-100, 100)), f32(rnd.uniform(-100, 100))],
             'pos': [f32(rnd.uniform(-100, 100)), f32(rnd.uniform(-100, 100))],
             'performer': rnd.choice([None] + [{'current': rnd.random() < 0.3, 'stage': rnd.choice([4, 4, 1])}] * 15),
             'locomotion': rnd.random() < 0.85, 'organization': rnd.random() < 0.5, 'owner': rnd.choice([2, 5]),
             'zone_check': rnd.random() < 0.4, 'relation': rnd.choice([0, 1, 1, 0, 3]),
             'point_relation': rnd.choice([0, 1, 1, 0, 2]),
             'record': [rnd.choice(keys), rnd.choice([1, 2]), rnd.choice([0, 1, 2, 3]), f32(rnd.uniform(0, 9)), 0,
                        f32(4.0), rnd.randrange(0, 6)],
             'null_point': [f32(rnd.choice([0.0, -1.0])), f32(rnd.choice([0.0, 3.0e38]))],
             'status': {k: rnd.choice([0, 0, 1, 2]) for k in ELEMENT_STATES},
             'stack': rnd.choice([['bombard'], ['bombard', 'sleep'], ['move', 'bombard']])}
        c['status']['bombard'] = rnd.choice([1, 2, 2])
        rows.append(c)
    # Nearest-member ties: o2's exact squared distance is just below o1's 25
    # but rounds to 25 at PC24 (kept: o1) and stays below at PC53/PC64.
    near = [3.0 - 3 * 2.0 ** -22, 4.0 + 2.0 ** -21]
    for order in (['o1', 'o2'], ['o2', 'o1'], ['o1', 'o2', 'o3']):
        for prec in PRECISIONS:
            objects = {'o0': {'gtid': 0x00700700, 'registered': True, 'pos': [f32(50.0), f32(50.0)], 'word6c': 1,
                              'v5c': True, 'members': order},
                       'o1': {'gtid': 0x00710701, 'registered': True, 'pos': [f32(3.0), f32(4.0)], 'word6c': 1,
                              'v5c': True, 'members': None},
                       'o2': {'gtid': 0x00720702, 'registered': True, 'pos': [f32(near[0]), f32(near[1])],
                              'word6c': 1, 'v5c': True, 'members': None},
                       'o3': {'gtid': 0x00730703, 'registered': True, 'pos': [f32(-4.0), f32(-3.0)], 'word6c': 1,
                              'v5c': True, 'members': None}}
            c = dict(rows[0], name='bombard_near_%s_%s' % ('_'.join(order), prec), precision=prec, objects=objects,
                     target=0x00700700, pos=[f32(0.0), f32(0.0)], performer={'current': False, 'stage': 4},
                     relation=0, record=['o1', 1, 2, f32(1.0), 0, f32(4.0), 0])
            rows.append(c)
    return rows


def main():
    produce(__file__, __doc__, 'native-uai-bombard.json', cases(), BombardMachine, {'5353E0': (ENTRY, END)})


if __name__ == '__main__':
    main()
