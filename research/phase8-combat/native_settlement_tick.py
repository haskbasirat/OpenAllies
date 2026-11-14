"""Original Settlement component Tick 587290(controller) executing unchanged
under Unicorn at Single24, Double53 and Extended64, with 529C30 and 471A40
running natively.

Seams, recorded in call order with their arguments (objects as fixture ids):
586150 (periodic), 56B590 (task start), 568040 (idle), 56A6B0 (admits),
CreateFullWall 587210, piece creation 586DB0, UpgradeFullWall 585C60, piece
upgrade 585BF0 and the owner virtuals +108 of Settlement +14 and +4 (their
results carry the +1F0 value). The fixture also records Settlement +48.

usage: native_settlement_tick.py [AA.exe]
The optional argument must be the same original image the shared machine
checks (its SHA-256 is asserted).
"""
from pathlib import Path
import json, random, struct, sys

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / 'phase8-ai'))
import native_machine as nm
from native_machine import Machine, PRECISIONS, f32, header, instructions, sha

ENTRY, END = 0x587290, 0x587596
NAN = 0x7fc00000


class TickMachine(Machine):
    def __init__(self, c):
        super().__init__(c['precision'])
        self.c = c
        self.ids = {}
        self.addr = {}
        r = self.resp = c['responses']
        gw = self.alloc(0x100)
        self.w(0xb36534, gw)
        self.w(gw + 0xe0, c['time'])
        # definitions: pieces' definitions, then the settlement's
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
        kinds = self.alloc(0x14 * max(1, len(c['slot_kinds'])))
        for i, k in enumerate(c['slot_kinds']):
            self.w(kinds + 0x14 * i, k)
        self.w(d + 0x47c, kinds)
        self.wb(d + 0x4a8, c['byte_4a8'])
        self.wb(d + 0x4a9, c['byte_4a9'])
        # owners (virtual +108 results carry +1F0)
        def owner(key, value):
            o = self.alloc(0x200)
            self.w(o + 0x1f0, value)
            return o
        self.owner14 = owner('component14', c['owner_1f0']['component14'])
        self.owner_actor = owner('actor', c['owner_1f0']['actor'])
        # settlement actor
        a = self.obj('actor', 0x200)
        vt = self.alloc(0x200)
        self.w(a, vt)
        self.w(vt + 0x108, self.stub(lambda m: (self.ev('owner_1f0', 'actor'), self.ret(self.owner_actor))))
        self.w(a + 4, d)
        self.w(a + 0x100, c['actor_flags'])
        holder = self.alloc(0x40)
        hvt = self.alloc(0x200)
        self.w(holder, hvt)
        self.w(hvt + 0x108, self.stub(lambda m: (self.ev('owner_1f0', 'component14'), self.ret(self.owner14))))
        # members and pieces
        for key, x in c['actors'].items():
            p = self.obj(key, 0x200)
            self.w(p + 4, self.ref(x['def']))
            self.w(p + 0x100, x['flags'])
            if x['health'] is not None:
                hr = self.alloc(0x20)
                self.w(hr + 0x10, x['health'][0])
                self.w(hr + 0x14, x['health'][1])
                self.w(p + 0x60, hr)
        s = self.s = self.alloc(0x100)
        self.w(s + 4, a)
        self.w(s + 0x10, c['style_index'])
        self.w(s + 0x14, holder)
        self.w(s + 0x18, self.array([self.ref(k) for k in c['members']]))
        self.w(s + 0x1c, len(c['members']))
        self.w(s + 0x24, self.array([self.ref(k) for k in c['pieces']]))
        self.w(s + 0x28, len(c['pieces']))
        self.w(s + 0x48, c['last'])
        self.w(s + 0x78, c['rotation'] & 0xffffffff)
        self.wb(s + 0x7c, c['forwards'])
        self.controller = self.alloc(0x80)
        ctl = self.controller

        def seam(address, name, pop, nargs, value=lambda m: 0, ecx=None):
            def fn(m):
                expected = {'controller': ctl, 'settlement': s}[ecx]
                assert self.ecx() == expected, (name, hex(self.ecx()))
                args = [self.arg(i) for i in range(1, nargs + 1)]
                self.ev(name, *args)
                self.ret(value(args), pop)
            self.seam(address, fn)
        seam(0x586150, 'periodic', 4, 1, ecx='settlement')
        self.seam(0x56b590, self.s_start)
        seam(0x568040, 'idle', 0, 0, lambda a: int(r['idle']), ecx='controller')
        seam(0x56a6b0, 'admits', 4, 1, lambda a: int(r['admits'][str(a[0])]), ecx='controller')
        seam(0x587210, 'create_full_wall', 4, 1, lambda a: int(r['create_full_wall']), ecx='settlement')
        seam(0x586db0, 'create_piece', 0xc, 3, lambda a: self.ref(r['create_piece']), ecx='settlement')
        seam(0x585c60, 'upgrade_full_wall', 0, 0, lambda a: int(r['upgrade_full_wall']), ecx='settlement')
        seam(0x585bf0, 'upgrade_piece', 4, 1, lambda a: int(r['upgrade_piece']), ecx='settlement')

    def s_start(self, m):
        assert self.ecx() == self.controller
        self.ev('start', self.arg(1), self.id(self.arg(2)))
        self.ret(0, 8)

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
        self.call(ENTRY, self.s, [self.controller])
        assert ((self.u.reg_read(nm.UC_X86_REG_FPSW) >> 11) & 7) == 0
        return {'events': self.events, 'last': self.r(self.s + 0x48)}


def cases():
    rnd = random.Random(0x587290)
    rows = []
    for i in range(1500):
        time = f32(rnd.choice([12345.678, 10.5, 3.25, 99999.0, 1.0e6 / 3.0]))
        tf = struct.unpack('<f', struct.pack('<I', time))[0]
        last = rnd.choice([f32(tf - 10.0), time, 0, f32(tf - 20.0), f32(tf - 9.999), f32(tf - 10.001)])
        if rnd.random() < 0.3:
            last = (last + rnd.randrange(-3, 4)) & 0xffffffff
        piece_defs = ['pd%d' % k for k in range(4)]
        styles = [[rnd.choice(piece_defs + [None]) for _ in range(3)] for _ in range(3)]
        n_members = rnd.randrange(0, 5)
        n_pieces = rnd.randrange(0, 6)
        actors = {}

        def actor(key):
            full = f32(rnd.choice([100.0, 50.0, 1.0]))
            fv = struct.unpack('<f', struct.pack('<I', full))[0]
            hp = rnd.choice([None, [full, full], [f32(fv * 0.5), full], [f32(fv * 2), full],
                             [f32(fv * 0.99), full], [NAN, full]])
            actors[key] = {'def': rnd.choice(piece_defs), 'health': hp,
                           'flags': rnd.choice([0, 0, 0x200000, 0x20000000, 0x20200000, 0x40])}
            return key
        members = [actor('member%d' % k) for k in range(n_members)]
        pieces = [actor('piece%d' % k) if rnd.random() < 0.6 else None for k in range(n_pieces)]
        actor('new_piece')
        owners = [0, f32(0.5), f32(-0.25), NAN]
        c = {'name': 'tick%d' % i, 'precision': list(PRECISIONS)[i % 3], 'time': time, 'last': last,
             'piece_defs': piece_defs, 'styles': styles,
             'slot_kinds': [rnd.choice([0, 1, 2, 3, 0, 2]) for _ in range(n_pieces)],
             'byte_4a8': rnd.choice([0, 0, 1]), 'byte_4a9': rnd.choice([0, 1, 1]),
             'owner_1f0': {'component14': rnd.choice(owners), 'actor': rnd.choice(owners)},
             'actor_flags': rnd.choice([0, 0, 0, 0x100000, 0x40000000, 0x200000]),
             'actors': actors, 'members': members, 'pieces': pieces,
             'style_index': rnd.choice([0, 0, 1, 2]),
             'rotation': rnd.choice([0, 1, 2, 5, -1, 7]), 'forwards': rnd.choice([0, 1]),
             'responses': {'idle': rnd.random() < 0.8, 'admits': {'6': rnd.random() < 0.7, '7': rnd.random() < 0.7},
                           'create_full_wall': rnd.random() < 0.5,
                           'create_piece': rnd.choice(['new_piece', None]),
                           'upgrade_full_wall': rnd.random() < 0.5, 'upgrade_piece': rnd.random() < 0.6}}
        rows.append(c)
    return rows


def main():
    if len(sys.argv) > 1:
        assert sha(Path(sys.argv[1]).read_bytes()) == sha(nm.original.b), 'not the original image'
    out, visited = [], {}
    for c in cases():
        try:
            m = TickMachine(c)
            result = m.run()
        except Exception:
            print(c['name'])
            raise
        m.verify_v14()
        visited.update(m.visited)
        out.append({'input': c, 'output': result})
    d = header(__file__, __doc__)
    d.update(instructions=instructions(visited),
             coverage={'587290': len([a for a in visited if ENTRY <= a < END])}, cases=out)
    p = HERE / 'native-settlement-tick-fixtures.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'coverage': d['coverage'], 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
