"""Original Settlement component wall piece helpers executing unchanged under
Unicorn: the piece style level 5856F0, the piece upgrade 585BF0,
UpgradeFullWall 585C60 (with 585BF0 and 5856F0 running natively inside) and
CreateFullWall 587210, with 529C30 and 471A40 native.

Seams, recorded in call order with their arguments (objects as fixture ids):
566360 (component upgrade, on the piece's +9C component), 586DB0 (piece
creation) and the debug report 404F30 (its line argument).

usage: native_settlement_walls.py [AA.exe]
The optional argument must be the same original image the shared machine
checks (its SHA-256 is asserted).
"""
from pathlib import Path
import json, random, sys

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / 'phase8-ai'))
import native_machine as nm
from native_machine import Machine, PRECISIONS, header, instructions, sha

ENTRIES = {'level': 0x5856f0, 'upgrade_piece': 0x585bf0, 'upgrade_full_wall': 0x585c60,
           'create_full_wall': 0x587210}
RANGES = {'5856F0': (0x5856f0, 0x585798), '585BF0': (0x585bf0, 0x585c5a), '585C60': (0x585c60, 0x585cb7),
          '587210': (0x587210, 0x58728f)}


class WallMachine(Machine):
    def __init__(self, c):
        super().__init__(c['precision'])
        self.c = c
        self.ids = {}
        self.addr = {}
        for key in c['piece_defs']:
            self.obj(key, 0x40)
        d = self.obj('settlement_def', 0x500)
        styles = self.alloc(12 * len(c['styles']) + 12)
        for i, st in enumerate(c['styles']):
            p = self.obj('style%d' % i, 0x40)
            for k, key in enumerate(st):
                self.w(p + 0x18 + 4 * k, self.ref(key))
            self.w(styles + 12 * i + 8, p)
        self.w(d + 0x464, styles)
        kinds = self.alloc(0x14 * max(1, len(c['slot_kinds'])))
        for i, k in enumerate(c['slot_kinds']):
            self.w(kinds + 0x14 * i, k)
        self.w(d + 0x47c, kinds)
        self.wb(d + 0x4a9, c['byte_4a9'])
        a = self.obj('actor', 0x200)
        self.w(a + 4, d)
        for key, x in c['actors'].items():
            p = self.obj(key, 0x200)
            self.w(p + 4, self.ref(x['def']))
            if x['component']:
                comp = self.alloc(0x40)
                self.ids[comp] = key + ':9c'
                self.w(p + 0x9c, comp)
        s = self.s = self.alloc(0x100)
        self.w(s + 4, a)
        self.w(s + 0x10, c['style_index'] & 0xffffffff)
        self.w(s + 0x24, self.array([self.ref(k) for k in c['pieces']]))
        self.w(s + 0x28, len(c['pieces']))
        r = c['responses']
        created = iter(r['create_piece'])
        self.seam(0x566360, lambda m: (self.ev('upgrade', self.id(self.ecx()), self.id(self.arg(1))),
                                       self.ret(0, 4)))

        def create(m):
            assert self.ecx() == s
            self.ev('create_piece', self.arg(1), self.arg(2), self.arg(3))
            self.ret(self.ref(next(created)), 0xc)
        self.seam(0x586db0, create)
        self.seam(0x404f30, lambda m: (self.ev('report', self.arg(3)), self.ret(0)))

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
        args = [self.c['arg'] & 0xffffffff] if op != 'upgrade_full_wall' else []
        v = self.call(ENTRIES[op], self.s, args)
        v = v & 0xffffffff if op == 'level' else v & 0xff
        return {'events': self.events, 'result': v}


def cases():
    rnd = random.Random(0x585c60)
    rows = []
    ops = ['level', 'upgrade_piece', 'upgrade_full_wall', 'create_full_wall']
    for i in range(1200):
        piece_defs = ['pd%d' % k for k in range(4)]
        n_styles = rnd.randrange(1, 5)
        styles = [[rnd.choice(piece_defs + [None]) for _ in range(3)] for _ in range(n_styles + 1)]
        n = rnd.randrange(0, 6)
        actors = {}
        pieces = []
        for k in range(n):
            if rnd.random() < 0.7:
                key = 'piece%d' % k
                actors[key] = {'def': rnd.choice(piece_defs + [None]), 'component': rnd.random() < 0.8}
                pieces.append(key)
            else:
                pieces.append(None)
        op = ops[i % 4]
        c = {'name': '%s%d' % (op, i), 'precision': list(PRECISIONS)[i % 3], 'operation': op,
             'piece_defs': piece_defs, 'styles': styles,
             'slot_kinds': [rnd.choice([0, 1, 2, 3]) for _ in range(n)],
             'byte_4a9': rnd.choice([0, 1]), 'actors': actors, 'pieces': pieces,
             'style_index': rnd.choice([0, 0, n_styles - 1, rnd.randrange(n_styles), -1]),
             'arg': rnd.choice([0, 1, 2, n - 1, n, -1, rnd.randrange(-1, n + 2)]) if op != 'create_full_wall'
             else rnd.choice([0, 1]),
             'responses': {'create_piece': [rnd.choice(['new%d' % k, None]) for k in range(n)]}}
        for k in range(n):
            actors['new%d' % k] = {'def': None, 'component': False}
        rows.append(c)
    return rows


def main():
    if len(sys.argv) > 1:
        assert sha(Path(sys.argv[1]).read_bytes()) == sha(nm.original.b), 'not the original image'
    out, visited = [], {}
    for c in cases():
        try:
            m = WallMachine(c)
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
    p = HERE / 'native-settlement-walls-fixtures.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'coverage': d['coverage'], 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
