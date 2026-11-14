"""Original element target choice under Unicorn, at Single24, Double53 and
Extended64, on the emulated element Mind of research/phase8-ai
(native_uai_harness.py):

* 52F010 (decision record rank) runs unchanged with 455000, called directly
  on two scripted 7-word records and a bypass byte (equal priorities, score
  differences around the 1e-4 threshold, distance ties, NaN, infinities and
  denormals).
* 534700 (best target of a spatial query) runs unchanged with 52F010 and
  455000. Seams (recorded in order): 558720 (query construction, not
  recorded), 558150 / 5580F0 (query begin, arguments recorded), 558750
  (next candidate, scripted order), 557FD0 (query end), candidate virtual
  +1C(filter), Mind virtuals +A0 and +1AC, 530E00 on the Mind, actor
  virtual +16C and 5340C0 (scripted in-place record writes; the record it
  receives is recorded).
* 535910 (engage state choice) runs unchanged with 529C30 and 577F20 (the
  organization component radius); 534700 is a seam recording its seven
  arguments and returning a scripted object.

Writes native-member-choice-fixtures.json next to this script.
"""
from pathlib import Path
import json, math, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / 'phase8-ai'))
from native_uai_harness import ELEMENT_STATES, DISCARD, MindMachine, PRECISIONS, f32, body_instructions  # noqa: E402
from native_machine import header, instructions, sha  # noqa: E402

RANK, RANK_END = 0x52f010, 0x52f0d6
NEAR, NEAR_END = 0x455000, 0x45502d
CHOOSE, CHOOSE_END = 0x534700, 0x534961
ENGAGE, ENGAGE_END = 0x535910, 0x5359c3
NAN, INF, NINF = 0x7fc00000, 0x7f800000, 0xff800000


class ChoiceMachine(MindMachine):
    def __init__(self, c):
        super().__init__(c, ELEMENT_STATES, DISCARD)
        a = self.actor
        self.objects = {}
        for key, o in c.get('objects', {}).items():
            p = self.objects[key] = self.obj(key, o['gtid'])
            self.w(p + 0x20, o['pos'][0]); self.w(p + 0x24, o['pos'][1])
            self.vslot(p, 0x1c, self.v_matches)
        self.names[0] = 0
        self.w(a + 0x20, c.get('actor_pos', [0, 0])[0]); self.w(a + 0x24, c.get('actor_pos', [0, 0])[1])
        if c.get('performer', True):
            self.w(a + 0x74, self.alloc(0x40))
        self.org = 0
        if c.get('membership') is not None:
            ms = self.alloc(0x40); self.w(a + 0x80, ms)
            m = c['membership']
            if m['organization']:
                org = self.org = self.obj('organization')
                self.w(org + 0x20, m['pos'][0]); self.w(org + 0x24, m['pos'][1])
                comp = self.comp = self.alloc(0x40); self.w(org + 0x7c, comp)
                x = self.alloc(0x10); y = self.alloc(0x400)
                self.w(comp + 4, x); self.w(x + 4, y); self.w(y + 0x300, m['radius'])
                self.w(ms + 0x10, org)
        self.query = None
        self.next = 0
        self.w(self.mvt + 0xa0, self.stub(self.v_considers))
        self.w(self.mvt + 0x1ac, self.stub(self.v_priority))
        self.vslot(a, 0x16c, self.v_accepts)
        self.seam(0x558720, self.s_init)
        self.seam(0x558150, self.s_begin_org)
        self.seam(0x5580f0, self.s_begin_circle)
        self.seam(0x558750, self.s_next)
        self.seam(0x557fd0, self.s_end)
        self.seam(0x530e00, self.s_unreachable)
        self.seam(0x5340c0, self.s_decide)
        if c['entry'] == 'engage' and c['seam_choose']:
            self.seam(CHOOSE, self.s_choose)

    def rec(self, p):
        w = [self.r(p + 4 * k) for k in range(7)]
        w[4] = self.name(w[4]); w[6] = struct.unpack('<i', struct.pack('<I', w[6]))[0]
        return w

    # seams ------------------------------------------------------------------
    def s_init(self, m):
        self.query = self.ecx(); self.ret(0)

    def s_begin_org(self, m):
        assert self.ecx() == self.query and self.arg(1) == self.comp
        self.ev('query_begin', organization=self.name(self.org)); self.ret(0, 4)

    def s_begin_circle(self, m):
        assert self.ecx() == self.query
        self.ev('query_begin', point=[self.arg(1), self.arg(2)], radius=self.arg(3)); self.ret(0, 12)

    def s_next(self, m):
        assert self.ecx() == self.query
        order = self.c['candidates']
        if self.next < len(order):
            p = self.objects[order[self.next]['key']]; self.next += 1
        else:
            p = 0
        self.ev('next', result=self.name(p)); self.ret(p)

    def s_end(self, m):
        assert self.ecx() == self.query
        self.ev('query_end'); self.ret(0)

    def current(self):
        return self.c['candidates'][self.next - 1]

    def v_matches(self, m):
        k = self.current(); assert self.ecx() == self.objects[k['key']]
        assert self.arg(1) == self.objects[self.c['filter']]
        self.ev('matches', object=k['key'], result=k['matches']); self.ret(int(k['matches']), 4)

    def v_considers(self, m):
        k = self.current(); assert self.ecx() == self.mind and self.arg(1) == self.objects[k['key']]
        self.ev('considers', object=k['key'], result=k['considers']); self.ret(int(k['considers']), 4)

    def s_unreachable(self, m):
        k = self.current(); assert self.ecx() == self.mind and self.arg(1) == self.objects[k['key']]
        self.ev('unreachable', object=k['key'], result=k['unreachable']); self.ret(int(k['unreachable']), 4)

    def v_accepts(self, m):
        k = self.current(); assert self.ecx() == self.actor and self.arg(1) == self.objects[k['key']]
        self.ev('accepts', object=k['key'], result=k['accepts']); self.ret(int(k['accepts']), 4)

    def v_priority(self, m):
        k = self.current(); assert self.ecx() == self.mind and self.arg(1) == self.objects[k['key']]
        self.ev('priority', object=k['key'], result=k['priority']); self.ret(k['priority'] & 0xffffffff, 4)

    def s_decide(self, m):
        k = self.current(); obj = self.objects[k['key']]
        assert self.ecx() == self.mind and self.arg(1) == obj and self.arg(2) == 0 and self.arg(4) == 0
        p = self.arg(3); before = self.rec(p)
        self.w(p + 0x10, obj)
        for i, v in k['writes']:
            self.w(p + 4 * i, v & 0xffffffff)
        self.ev('decide', object=k['key'], before=before); self.ret(k['status'], 16)

    def s_choose(self, m):
        assert self.ecx() == self.mind
        args = [self.arg(i) for i in range(1, 8)]
        self.ev('choose', point=args[0:2], radius=args[2], out=args[3], minimum=args[4], filter=args[5],
                preferred=self.name(args[6]))
        r = self.c['choose_result']
        self.ret(0 if r is None else self.objects[r], 0x1c)

    def run(self):
        c = self.c
        if c['entry'] == 'rank':
            pa, pb = self.alloc(32), self.alloc(32)
            for k in range(7):
                self.w(pa + 4 * k, c['a'][k] & 0xffffffff); self.w(pb + 4 * k, c['b'][k] & 0xffffffff)
            r = self.call(RANK, ecx=0, args=[pa, pb, c['flag']])
            return {'result': struct.unpack('<i', struct.pack('<I', r))[0]}
        if c['entry'] == 'choose':
            out = 0
            if c['out']:
                out = self.alloc(32)
                for k in range(7):
                    self.w(out + 4 * k, 0x5a5a5a5a)
            filt = self.objects[c['filter']] if c['filter'] else 0
            pref = self.objects[c['preferred']] if c['preferred'] else 0
            r = self.call(CHOOSE, ecx=self.mind, args=[c['point'][0], c['point'][1], c['radius'], out,
                                                       c['minimum'] & 0xffffffff, filt, pref])
            return {'result': self.name(r), 'out': self.rec(out) if out else None, 'events': self.events}
        target = self.objects[c['target']] if c['target'] else 0
        r = self.call(ENGAGE, ecx=self.states['engage'], args=[target, 0])
        return {'result': self.name(r), 'events': self.events}


SCORES = [f32(10.0), f32(10.0), f32(10.00005), f32(10.0002), f32(9.9999), f32(-1.0), f32(0.0), 0x80000000,
          f32(1e-4), f32(2e-4), f32(5e-5), 1, 0x00400000, NAN, INF, NINF, f32(3.0), f32(1e20)]
DISTS = [f32(100.0), f32(100.0), f32(100.0001), f32(100.0002), f32(99.9999), 0x7f7fffff, f32(0.0),
         f32(1e-4), f32(1.5e-4), f32(5e-5), 1, NAN, INF, f32(4.0), f32(4.00009)]


def record(rnd, ability=None):
    return [rnd.choice([0, 0x1111, 0x2222]) if ability is None else ability, rnd.choice([1, 2]),
            rnd.choice([0, 1, 3]), rnd.choice(SCORES), rnd.choice([0, 0x3333]), rnd.choice(DISTS),
            rnd.choice([0, 1, 2, 2, 3, -1])]


def tiny_pair(rnd):
    """Score pairs whose difference lands near 1e-4 and needs rounding."""
    b = rnd.choice([rnd.uniform(-1e-3, 1e-3), rnd.uniform(-3, 3), rnd.uniform(-1e-9, 1e-9)])
    d = rnd.choice([1e-4, -1e-4, 1e-4 * (1 + 2 ** -23), 1e-4 * (1 - 2 ** -23), 0.99999e-4, 1.00001e-4])
    return f32(b + d), f32(b)


def candidates(rnd, keys):
    """A scripted 558750 order (repeats allowed) with per-visit answers."""
    order = [rnd.choice(keys) for _ in range(rnd.randrange(0, 9))] if keys else []
    cands = []
    for key in order:
        mode = rnd.choice(['full', 'full', 'full', 'fail', 'partial'])
        if mode == 'full':
            writes = [[0, rnd.choice([0, 0x1111, 0x2222, 0x2222])], [1, rnd.choice([1, 2])], [2, rnd.choice([0, 1, 3])],
                      [3, rnd.choice(SCORES[:13])], [5, rnd.choice(DISTS[:11])], [6, rnd.choice([0, 1, 2, 3])]]
        elif mode == 'fail':
            writes = [[2, 3]]
        else:
            writes = [[3, rnd.choice(SCORES[:13])]]
        cands.append({'key': key, 'matches': rnd.random() < 0.8, 'considers': rnd.random() < 0.85,
                      'unreachable': rnd.random() < 0.15, 'accepts': rnd.random() < 0.75,
                      'priority': rnd.choice([0, 1, 2, 2, 3, 3, 4, -1]), 'writes': writes,
                      'status': rnd.choice([0, 1, 3])})
    return cands


def cases():
    rnd = random.Random(0x52f010)
    rows = []
    precisions = list(PRECISIONS)
    for i in range(900):
        a, b = record(rnd), record(rnd)
        if i % 3 == 0:
            a[0], b[0] = 0x1111, 0x2222
            b[6] = a[6]
        if i % 5 == 0:
            a[3], b[3] = tiny_pair(rnd)
        if i % 7 == 0:
            a[5], b[5] = tiny_pair(rnd)
        rows.append({'name': 'rank%d' % i, 'entry': 'rank', 'precision': precisions[i % 3], 'a': a, 'b': b,
                     'flag': rnd.choice([0, 1, 0x100, 0x101, 0xff])})
    for i in range(600):
        objects = {}
        for k in range(rnd.randrange(0, 7)):
            objects['o%d' % k] = {'gtid': (0x50 + k) << 16 | (0x500 + k), 'pos': [f32(rnd.uniform(0, 50)), f32(rnd.uniform(0, 50))]}
        keys = list(objects)
        cands = candidates(rnd, keys)
        ms = rnd.choice([None, None, {'organization': False}, {'organization': True}])
        if ms and ms['organization']:
            ms.update(pos=[f32(rnd.uniform(0, 50)), f32(rnd.uniform(0, 50))], radius=f32(rnd.uniform(1, 40)))
        rows.append({'name': 'choose%d' % i, 'entry': 'choose', 'precision': precisions[i % 3], 'objects': objects,
                     'candidates': cands, 'membership': ms, 'performer': rnd.random() < 0.92,
                     'point': [f32(rnd.uniform(0, 50)), f32(rnd.uniform(0, 50))], 'radius': f32(rnd.uniform(1, 40)),
                     'out': rnd.random() < 0.6, 'minimum': rnd.choice([0, 1, 2, 2, 3, -1]),
                     'filter': rnd.choice([None, None] + keys), 'preferred': rnd.choice([None, None] + keys)})
    for i in range(240):
        objects = {'o%d' % k: {'gtid': (0x50 + k) << 16 | (0x500 + k),
                               'pos': [f32(rnd.choice([rnd.uniform(-60, 60), 3.0, 0.1])), f32(rnd.uniform(-60, 60))]} for k in range(2)}
        ms = rnd.choice([None, {'organization': False}, {'organization': True}, {'organization': True}])
        if ms and ms['organization']:
            ms.update(pos=[f32(rnd.uniform(0, 50)), f32(rnd.uniform(0, 50))], radius=f32(rnd.uniform(1, 40)))
        rows.append({'name': 'engage%d' % i, 'entry': 'engage', 'precision': precisions[i % 3], 'objects': objects,
                     'membership': ms, 'actor_pos': [f32(rnd.uniform(-60, 60)), f32(rnd.choice([rnd.uniform(-60, 60), 0.1]))],
                     'target': rnd.choice([None, 'o0', 'o1', 'o0']), 'choose_result': rnd.choice([None, 'o0', 'o1']),
                     'seam_choose': i % 2 == 0, 'candidates': [] if i % 2 == 0 else candidates(rnd, list(objects)),
                     'performer': rnd.random() < 0.92})
    return rows


def main():
    out, visited = [], {}
    for c in cases():
        try:
            m = ChoiceMachine(c); result = m.run()
        except Exception:
            print(c['name']); raise
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': result})
    d = header(__file__, __doc__)
    ranges = {'52F010': (RANK, RANK_END), '455000': (NEAR, NEAR_END), '534700': (CHOOSE, CHOOSE_END),
              '535910': (ENGAGE, ENGAGE_END)}
    cover = {k: [len([a for a in visited if lo <= a < hi]), body_instructions(lo, hi)] for k, (lo, hi) in ranges.items()}
    d.update(instructions=instructions(visited), coverage=cover, cases=out)
    p = HERE / 'native-member-choice-fixtures.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'instructions': len(visited), 'coverage': cover, 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
