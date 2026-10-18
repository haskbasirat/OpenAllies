"""GWorld::Load 4FC600 after the PostLoad pass: 4FD3F3..4FD471 under Unicorn
(native_machine.py) on synthetic GWorld / session / kingdom / actor objects.

Executed natively: the map-settings gate (+28 -> +A8 in {1, 4}), 4F5290
(application state [B36550]+E4, GWorld +48, 76AC60 / 76AC70 / 765E00 on
[B36558]+8, 4F48E0 with 4BEDF0, 4012C0 and wcscmp 96CB83), 4F1C20 (map
centre x 0.5 in the x87, the +284 actor scan with 4CE6C0, +1F8), the
GWorld +22C/+230 stores. Seams (recorded): 59E520 (scripted), 4F1E10 (the
economy agent's world start), the 404F60 log (its two doubles recorded as
64-bit patterns).
"""
from pathlib import Path
import sys, json, struct, copy
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import Machine, P, V, VISITED, H, f32, STACK, G_WORLD
from unicorn.x86_const import UC_X86_REG_ESP, UC_X86_REG_EIP, UC_X86_REG_EDI, UC_X86_REG_ECX

BEGIN, END = 0x4fd3f3, 0x4fd471


class TailMachine(Machine):
    def __init__(self, c):
        super().__init__(b'', c.get('precision', 'Double53'))
        self.c = c
        w = self.alloc(0x400)
        self.world = w
        self.w(G_WORLD, w)
        settings = self.alloc(0x100)
        self.w(settings + 0xa8, c['mode'])
        self.w(w + 0x28, settings)
        gmap = self.alloc(0x40)
        self.w(gmap + 0xc, c['map'][0])
        self.w(gmap + 0x10, c['map'][1])
        self.w(w + 0x30, gmap)
        self.w(w + 0x48, c['world_48'])
        app = self.alloc(0x100)
        self.w(app + 0xe4, c['app_state'])
        self.w(0xb36550, app)
        self.addr = {}
        self.label = {}
        for k in c['kingdoms']:
            a = self.alloc(0x400)
            self.addr[k['label']] = a
            self.label[a] = k['label']
        for o in c['actors']:
            a = self.alloc(0x200)
            self.addr[o['label']] = a
            self.label[a] = o['label']
        for o in c['actors']:
            a = self.addr[o['label']]
            d = self.alloc(0x200)
            self.w(d + 0x1b4, o.get('p', 0) & 0xffffffff)
            self.w(a + 4, d)
            self.w(a + 8, o.get('flags', 0))
            self.w(a + 0x20, o['xy'][0])
            self.w(a + 0x24, o['xy'][1])
            if o.get('settlement'):
                self.w(a + 0x98, self.alloc(0x10))
            if o.get('organization_component'):
                self.w(a + 0x7c, self.alloc(0x10))
            if o.get('element_org'):
                e = self.alloc(0x20)
                self.w(e + 0x10, self.addr[o['element_org']])
                self.w(a + 0x80, e)
        for k in c['kingdoms']:
            a = self.addr[k['label']]
            self.w(a + 0x18, self.cstring(k['name']))
            nodes = []
            for x in k.get('actors', []):
                n = self.alloc(8)
                self.w(n, self.addr[x])
                nodes.append(n)
            for i, n in enumerate(nodes):
                self.w(n + 4, nodes[i + 1] if i + 1 < len(nodes) else 0)
            self.w(a + 0x284, nodes[0] if nodes else 0)
            self.w(a + 0x1f8, self.addr[k['f8']] if k.get('f8') else 0)
        slots = c['world_kingdoms']
        vec = self.alloc(4 * max(1, len(slots)))
        for i, k in enumerate(slots):
            self.w(vec + 4 * i, self.addr[k] if k else 0)
        self.w(w + 0x148, vec)
        self.w(w + 0x14c, len(slots))
        net = self.alloc(0x40)
        if c['player'] is not None:
            pl = self.alloc(0x40)
            self.w(pl + 0x24, self.cstring(c['player']['name']))
            self.w(pl + 0x28, self.addr[c['player']['kingdom']] if c['player'].get('kingdom') else 0)
            self.w(net + 8, pl)
        self.w(0xb36558, net)

        def flag(m):  # 59E520 thiscall on the map settings
            assert m.u.reg_read(UC_X86_REG_ECX) == settings
            m.record('59E520', result=m.c['flag_59e520'])
            m.ret(0, int(m.c['flag_59e520']))
        self.seam(0x59e520, flag)

        def start(m):  # 4F1E10 thiscall on GWorld
            assert m.u.reg_read(UC_X86_REG_ECX) == w
            m.record('world_start')
            m.ret(0)
        self.seam(0x4f1e10, start)

        def log(m):  # 404F60(B38B70, file, line, function, fmt, double x, double y)
            fmt = m.read_wide(m.arg(4))
            m.record('404F60', line=m.arg(2), function=m.read_wide(m.arg(3)), format=fmt,
                     x='%08x%08x' % (m.arg(6), m.arg(5)), y='%08x%08x' % (m.arg(8), m.arg(7)))
            m.ret(0)
        self.seam(0x404f60, log)

    def run(self):
        frame = STACK - 0x400
        self.u.reg_write(UC_X86_REG_ESP, frame)
        self.u.reg_write(UC_X86_REG_EDI, self.world)
        self.u.emu_start(BEGIN, END, count=2_000_000)
        if self.null_accesses:
            self.outcome = {'kind': 'NullDereference', 'pc': self.null_accesses[0][0]}
        else:
            assert self.u.reg_read(UC_X86_REG_EIP) == END, hex(self.u.reg_read(UC_X86_REG_EIP))
        return {'outcome': self.outcome, 'calls': self.calls,
                'look_at': [self.r(self.world + 0x22c), self.r(self.world + 0x230)]}


def cases():
    rows = []
    base = dict(mode=0, map=[f32(4096.0), f32(2048.0)], world_48=0, app_state=2, flag_59e520=False,
                kingdoms=[], actors=[], world_kingdoms=[], player=None)

    def add(name, **kw):
        c = copy.deepcopy(base)
        c.update(kw)
        c['name'] = name
        rows.append(c)

    K = lambda label, name, actors=(), f8=None: {'label': label, 'name': name, 'actors': list(actors), 'f8': f8}
    A = lambda label, x, y, p=0, **kw: {'label': label, 'xy': [f32(x), f32(y)], 'p': p, **kw}
    two = [K('axis', 'Axis'), K('allies', 'Allies', ['s1', 'f1', 'o1', 'skip'])]
    actors = [A('s1', 10, 20, 5, settlement=True), A('f1', 30, 40, 9, flags=1 << 29),
              A('o1', 50, 60, 3, organization_component=True), A('skip', 70, 80, 99, flags=1 << 22, settlement=True)]
    add('app_state_3', app_state=3, player={'name': 'Allies'}, kingdoms=two, actors=actors,
        world_kingdoms=[None, 'axis', 'allies'])
    add('by_name_settlement_wins', player={'name': 'Allies'}, kingdoms=two, actors=actors,
        world_kingdoms=[None, 'axis', 'allies'])
    add('by_name_flagged_wins', player={'name': 'Allies'},
        kingdoms=[K('allies', 'Allies', ['f1', 'f2', 'o1'])],
        actors=[A('f1', 30, 40, 9, flags=1 << 29), A('f2', 31, 41, 9, flags=1 << 29), A('o1', 50, 60, 30)],
        world_kingdoms=['allies'])
    add('by_name_organization', player={'name': 'Allies'},
        kingdoms=[K('allies', 'Allies', ['m1', 'm2', 'neg'])],
        actors=[A('m1', 1, 2, 0, element_org='org'), A('org', 500, 600, 4, organization_component=True),
                A('m2', 3, 4, 2), A('neg', 5, 6, -5)],
        world_kingdoms=['allies'])
    add('by_name_f8', player={'name': 'Allies'},
        kingdoms=[K('allies', 'Allies', ['neg', 'skip'], f8='cap')],
        actors=[A('neg', 5, 6, -1), A('skip', 7, 8, 50, flags=(1 << 22) | (1 << 29)), A('cap', 777, -3.5)],
        world_kingdoms=['allies'])
    add('by_name_centre', player={'name': 'Allies'}, kingdoms=[K('allies', 'Allies')], world_kingdoms=['allies'])
    add('by_name_missing', player={'name': 'Nobody'}, kingdoms=two, actors=actors, world_kingdoms=['axis', 'allies'])
    add('by_name_case_differs', player={'name': 'allies'}, kingdoms=two, actors=actors, world_kingdoms=['axis', 'allies'])
    add('mode_nonzero_by_name', world_48=1, mode=2, player={'name': 'Axis'}, kingdoms=two, actors=actors,
        world_kingdoms=['axis', 'allies'])
    add('player_kingdom', world_48=1, mode=0, player={'name': 'Axis', 'kingdom': 'allies'}, kingdoms=two,
        actors=actors, world_kingdoms=['axis', 'allies'])
    add('player_no_kingdom', world_48=1, mode=0, player={'name': 'Axis'}, kingdoms=two, actors=actors,
        world_kingdoms=['axis', 'allies'])
    add('no_player_76ac70', world_48=1, mode=0, player=None, kingdoms=two, actors=actors,
        world_kingdoms=['axis', 'allies'])
    add('no_player_by_name', world_48=0, player=None, kingdoms=two, actors=actors, world_kingdoms=['axis', 'allies'])
    add('world_start_mode1', mode=1, flag_59e520=True, player={'name': 'Axis'}, kingdoms=two, actors=actors,
        world_kingdoms=['axis', 'allies'])
    add('world_start_mode4_flag_clear', mode=4, flag_59e520=False, player={'name': 'Axis'}, kingdoms=two,
        actors=actors, world_kingdoms=['axis', 'allies'])
    add('world_start_mode4', mode=4, flag_59e520=True, app_state=3)
    add('odd_map_sizes', app_state=3, map=[0x00000003, 0x7f7fffff])
    add('odd_map_sizes_2', app_state=3, map=[0x00800001, 0x3f800001])
    for base_name in ('odd_map_sizes', 'odd_map_sizes_2', 'by_name_centre'):
        b = next(r for r in rows if r['name'] == base_name)
        for pc in ('Single24', 'Extended64'):
            rows.append({**copy.deepcopy(b), 'name': base_name + '_' + pc, 'precision': pc})
    return rows


def main():
    out = []
    for c in cases():
        m = TailMachine(c)
        out.append({'input': c, 'native': m.run()})
    for a, b in VISITED.items():
        assert bytes.fromhex(b) == P.range(a, len(bytes.fromhex(b))) == V.range(a, len(bytes.fromhex(b)))
    doc = {'schema': 1, 'passed': True, 'producer_sha256': H(Path(__file__).read_bytes()),
           'machine_sha256': H((HERE / 'native_machine.py').read_bytes()),
           'original_sha256': H(P.b), 'v14_sha256': H(V.b), 'scope': __doc__, 'cases': out}
    path = HERE / 'native-load-tail.json'
    path.write_text(json.dumps(doc, indent=1), encoding='utf-8')
    for row in out:
        n = row['native']
        xy = [struct.unpack('<f', struct.pack('<I', v))[0] for v in n['look_at']]
        print(row['input']['name'], n['outcome'], [x['seam'] for x in n['calls']], xy)
    print(json.dumps({'cases': len(out), 'sha256': H(path.read_bytes())}))


if __name__ == '__main__':
    main()
