"""Original Settlement component piece creation 586DB0, the health copy
5850F0 and the 4CEF70 broadcast 585040 executing unchanged under Unicorn at
Single24, Double53 and Extended64, with 529C30, 471A40, 4C8C20 / 4C8C70 and
4CF1B0 native.

Seams, recorded in call order with their arguments (objects as fixture ids):
4F9C10 (the whole 0x54-byte creation block is dumped), 587800, 561E50,
4CEF70, the actor virtual +108 and the string allocator 856370.

usage: native_settlement_pieces.py [AA.exe]
The optional argument must be the same original image the shared machine
checks (its SHA-256 is asserted).
"""
from pathlib import Path
import json, random, sys

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / 'phase8-ai'))
import native_machine as nm
from native_machine import Machine, PRECISIONS, f32, header, instructions, sha

ENTRIES = {'create': 0x586db0, 'share': 0x5850f0, 'notify': 0x585040}
RANGES = {'586DB0': (0x586db0, 0x586f58), '5850F0': (0x5850f0, 0x585153), '585040': (0x585040, 0x5850e1)}


class PieceMachine(Machine):
    def __init__(self, c):
        super().__init__(c['precision'])
        self.c = c
        self.ids = {}
        self.addr = {}
        self.w(0xb36534, self.alloc(0x100))
        for key in c['piece_defs']:
            self.obj(key, 0x40)
        d = self.obj('settlement_def', 0x500)
        styles = self.alloc(12 * len(c['styles']))
        for i, st in enumerate(c['styles']):
            p = self.obj('style%d' % i, 0x40)
            for k, key in enumerate(st):
                self.w(p + 0x18 + 4 * k, self.ref(key))
            self.w(styles + 12 * i + 8, p)
        self.w(d + 0x464, styles)
        entries = self.alloc(0x14 * max(1, len(c['slots'])))
        for i, e in enumerate(c['slots']):
            for k, v in enumerate([e['kind']] + e['offset'] + e['facing']):
                self.w(entries + 0x14 * i + 4 * k, v)
        self.w(d + 0x47c, entries)
        self.wb(d + 0x4a8, c['byte_4a8'])
        self.wb(d + 0x4a9, c['byte_4a9'])
        # string allocator for 4C8C20 and the block's +1C release
        sav = self.alloc(0x20)
        self.str_alloc = self.alloc(0x10)
        self.w(self.str_alloc, sav)
        self.w(sav + 4, self.stub(lambda m: (self.ev('string_free'), self.ret(0, 4))))
        h = self.alloc(0x20)
        self.w(h, self.str_alloc)
        self.w(h + 0xc, 1000)
        self.empty = h + 0x10
        self.w(sav + 0xc, self.stub(lambda m: self.ret(h)))
        self.seam(0x856370, lambda m: self.ret(self.str_alloc))
        a = self.obj('actor', 0x200)
        vt = self.alloc(0x200)
        self.w(a, vt)
        self.w(vt + 0x108, self.stub(lambda m: (self.ev('kingdom'), self.ret(c['kingdom']))))
        self.w(a + 4, d)
        self.w(a + 0x14, c['actor_handle'])
        for i in range(3):
            self.w(a + 0x20 + 4 * i, c['position'][i])
        for key, x in c['objects'].items():
            p = self.obj(key, 0x200)
            self.w(p + 4, self.ref(x.get('def')))
            self.w(p + 0x78, x['word_78'])
            if x['body'] is not None:
                b = self.alloc(0x40)
                self.ids[b] = key + ':body'
                self.w(b + 0x10, x['body'])
                self.w(p + 0x60, b)
        s = self.s = self.alloc(0x100)
        self.w(s + 4, a)
        self.w(s + 0x10, c['style_index'])
        self.w(s + 0x14, self.ref(c['holder']))
        self.w(s + 0x18, self.array([self.ref(k) for k in c['members']]))
        self.w(s + 0x1c, len(c['members']))
        self.w(s + 0x24, self.array([self.ref(k) for k in c['pieces']]))
        self.w(s + 0x28, len(c['pieces']))

        def create(m):
            p = self.arg(1)
            assert self.r(p) == 0xa25b28
            w = [self.r(p + 4 * i) for i in range(0x14)]
            name = 'empty' if w[7] == self.empty else w[7]
            self.ev('create', [self.id(w[1])] + w[2:7] + [name] + w[8:10] + [w[10], self.id(w[11])] + w[12:20] +
                    [self.rb(p + 0x50), self.rb(p + 0x51), self.rb(p + 0x52)])
            self.ret(self.ref(c['created']), 4)
        self.seam(0x4f9c10, create)
        self.seam(0x587800, lambda m: (self.ev('attach_piece', self.arg(1), self.id(self.arg(2)), self.arg(3)),
                                       self.ret(0, 0xc)))
        self.seam(0x561e50, lambda m: (self.ev('set_health', self.id(self.ecx()), self.arg(1), self.arg(2),
                                               self.arg(3), self.arg(4)), self.ret(0, 0x10)))
        self.seam(0x4cef70, lambda m: (self.ev('notify', self.id(self.ecx()), self.arg(1), self.id(self.arg(2))),
                                       self.ret(0, 8)))

    def obj(self, key, size):
        p = self.alloc(size)
        self.ids[p] = key
        self.addr[key] = p
        return p

    def ref(self, key):
        return 0 if key is None else self.addr[key]

    def id(self, a):
        return self.ids.get(a, a)

    def ev(self, name, *args):
        self.events.append([name] + list(args))

    def run(self):
        op = self.c['operation']
        if op == 'create':
            a = self.c['args']
            v = self.call(ENTRIES[op], self.s, [a[0] & 0xffffffff, a[1], a[2]])
            return {'events': self.events, 'result': self.id(v)}
        if op == 'share':
            self.call(ENTRIES[op], self.s, [self.ref(self.c['piece'])])
            return {'events': self.events}
        event = self.alloc(0x20)
        self.w(event + 0xc, self.c['event_value'])
        self.wb(event + 0x10, self.c['event_handled'])
        self.call(ENTRIES[op], self.s, [event])
        return {'events': self.events}


def cases():
    rnd = random.Random(0x586db0)
    rows = []
    ops = ['create', 'create', 'share', 'notify']
    for i in range(1200):
        piece_defs = ['pd%d' % k for k in range(4)]
        styles = [[rnd.choice(piece_defs + [None]) for _ in range(3)] for _ in range(3)]
        n = rnd.randrange(0, 6)
        objects = {}

        def obj(key):
            objects[key] = {'def': rnd.choice(piece_defs), 'word_78': rnd.choice([0, 0x55]),
                            'body': rnd.choice([None, f32(rnd.uniform(0, 500)), f32(rnd.uniform(0, 500))])}
            return key
        pieces = [obj('piece%d' % k) if rnd.random() < 0.6 else None for k in range(n)]
        members = [obj('member%d' % k) for k in range(rnd.randrange(0, 4))]
        holder = rnd.choice([None, obj('holder')])
        obj('created')
        op = ops[i % 4]
        c = {'name': '%s%d' % (op, i), 'precision': list(PRECISIONS)[i % 3], 'operation': op,
             'piece_defs': piece_defs, 'styles': styles,
             'slots': [{'kind': rnd.choice([0, 1, 2, 3]),
                        'offset': [f32(rnd.uniform(-30, 30)), f32(rnd.uniform(-30, 30))],
                        'facing': [f32(rnd.uniform(-1, 1)), f32(rnd.uniform(-1, 1))]} for _ in range(n)],
             'byte_4a8': rnd.choice([0, 1, 1]), 'byte_4a9': rnd.choice([0, 1]),
             'kingdom': rnd.choice([0, 0x7000, 0x7100]), 'actor_handle': 0x00050033,
             'position': [f32(rnd.uniform(0, 4000)), f32(rnd.uniform(0, 4000)), f32(rnd.uniform(-5, 60))],
             'objects': objects, 'pieces': pieces, 'members': members, 'holder': holder,
             'style_index': rnd.choice([0, 0, 1, 2]), 'created': rnd.choice(['created', None]),
             'args': [rnd.choice([0, 1, n - 1, n, -1, rnd.randrange(-1, n + 1)]),
                      rnd.choice([0, 1, 0x100, 0x101, 0xff]), rnd.choice([0, 1])],
             'piece': rnd.choice([p for p in pieces if p is not None] or [None]),
             'event_value': rnd.getrandbits(32), 'event_handled': rnd.choice([0, 0, 1])}
        if op == 'share' and c['piece'] is None:
            c['piece'] = obj('lone')
        if op == 'share' and objects[c['piece']]['body'] is None:
            objects[c['piece']]['body'] = f32(77.5)
        rows.append(c)
    return rows


def main():
    if len(sys.argv) > 1:
        assert sha(Path(sys.argv[1]).read_bytes()) == sha(nm.original.b), 'not the original image'
    out, visited = [], {}
    for c in cases():
        try:
            m = PieceMachine(c)
            result = m.run()
        except Exception:
            print(c['name'])
            raise
        m.verify_v14()
        visited.update(m.visited)
        out.append({'input': c, 'output': result})
    d = header(__file__, __doc__)
    d.update(instructions=instructions(visited),
             coverage={k: len([a for a in visited if lo <= a < hi]) for k, (lo, hi) in RANGES.items()},
             cases=out)
    p = HERE / 'native-settlement-pieces-fixtures.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'coverage': d['coverage'], 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
