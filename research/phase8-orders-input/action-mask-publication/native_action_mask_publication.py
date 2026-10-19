"""Oracle producer for the action-mask publication 0x4CFCC0 branches 2 and 3.

Runs ORIGINAL code in the original AA.exe and in the pinned v14 image at
PC24/PC53/PC64 (research/phase8-ai/native_machine.py, page-0 guard, every
executed instruction byte-checked against both images). Four suites:

1. masks:    0x4ECD70(this=VWorldState) (player selection command masks) with
             0x96D710 (__allshl) natively. Selected things use the REAL
             primary vtables GActor A273F0, GKingdom A299D8, GGroup A29490
             and GObject A2A2A0 (vt+40 887B50, vt+B4 6A71D0 / 6A71F0,
             vt+134 4D1E90 / 4E5240 / 969B10, vt+20 969B10 run natively;
             vt+20 0x4CF120 is a recorded seam returning the case's raw EAX),
             or a synthetic vtable whose +20/+40/+B4/+134 are recorded seams
             (so the vt+40-true and research 0x5845A0 paths are covered).
             0x5845A0 is a recorded seam (a NULL research pointer runs the
             original prologue and faults on page 0).
2. owners:   0x76C7F0(this=Session, owner) with 0x765E00 and 0x4ECD70
             natively over the Session+BC player list (player +28 kingdom,
             +50 VWorldState), including a NULL GWorld [B36534].
3. selected: 0x50C8E0(actor) with 0x4D1E80 (vf108), 0x4F5290 (and its
             0x76AC60, 0x76AC70, 0x765E00, 0x4F48E0, 0x4BEDF0, 0x4012C0,
             wcscmp 0x96CB83 and the CString releases), 0x50C7D0 natively.
             0x437EB0 (UIWorldState command masks) is a recorded seam: it is
             proved separately (research/phase8-orders-input/command-masks).
4. publish:  0x4CFCC0(this=actor) end to end: branch 1 0x529D70 (its jump
             target 0x4D1350 is a recorded seam), branch 2 0x76C7F0 ->
             0x4ECD70 and branch 3 0x50C8E0 natively as above.

Laboratory globals: [B36530] GameData (+298 the 39 ActorCommand definition
pointers, +3C byte; other bytes noise), [B36534] GWorld (+D8 phase, +28 map
settings (+A8 mode), +48 word, +130 notifier cookie, +148/+14C kingdom
slots), [B36538] VWorld (+13C UIWorldState, whose +08 head node is
{view, next}), [B36550] Session (+E4 application state, +BC player node list
{player, next}), [B36558] net (+8 local player: +24 name CString, +28
kingdom, +50 VWorldState). VWorldState: +0 kingdom, +4 head / +8 tail
{thing, next}, +10..+1C mask words (pre-filled with garbage), +20 group
(+18 byte). Views: +54 flags. CStrings: header {manager, length, capacity,
refcount=1} with a manager vtable whose +10 returns itself and +4 records a
free.

Events (in native order): vt40/vt20/vtb4/vt134 virtual calls named by
their call site, 5845a0, vf108 (0x4D1E80 entries), 765e00, 4ecd70, 76c7f0,
50c8e0, 4f5290 (each returned kingdom), 50c7d0 (view and AL result),
4d1350 and 437eb0. Rows whose native run faults are kept with `fault`
(events up to the fault) and must be errors in Rust.

  python native_action_mask_publication.py --write   # create (refuses to overwrite)
  python native_action_mask_publication.py           # replay: byte-identical check
"""
from pathlib import Path
import hashlib, json, random, struct, sys
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'research/phase8-ai'))
import native_machine as nm  # noqa: E402
from unicorn import UcError  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ESP, UC_X86_REG_EAX  # noqa: E402

OUT = HERE / 'native-action-mask-publication.json'
FUNCTIONS = {'4CFCC0': (0x4cfcc0, 0x4cfd2c), '529D70': (0x529d70, 0x529d78), '76C7F0': (0x76c7f0, 0x76c830),
             '765E00': (0x765e00, 0x765e10), '4ECD70': (0x4ecd70, 0x4ecefc), '96D710': (0x96d710, 0x96d72f),
             '50C8E0': (0x50c8e0, 0x50c965), '50C7D0': (0x50c7d0, 0x50c7f5), '4F5290': (0x4f5290, 0x4f5365),
             '76AC60': (0x76ac60, 0x76ac69), '76AC70': (0x76ac70, 0x76ac8d), '4F48E0': (0x4f48e0, 0x4f4978),
             '4BEDF0': (0x4bedf0, 0x4bee16), '4012C0': (0x4012c0, 0x401321), '96CB83': (0x96cb83, 0x96cbb5),
             '4D1E80': (0x4d1e80, 0x4d1e87), '4D1E90': (0x4d1e90, 0x4d1ead), '4E5240': (0x4e5240, 0x4e524e),
             '887B50': (0x887b50, 0x887b55), '6A71D0': (0x6a71d0, 0x6a71d3), '6A71F0': (0x6a71f0, 0x6a71f3),
             '969B10': (0x969b10, 0x969b15)}
VTABLES = {'gactor': 0xA273F0, 'gkingdom': 0xA299D8, 'ggroup': 0xA29490, 'object': 0xA2A2A0}
# Call sites (return addresses) of the thing virtuals, by name.
SITES = {0x4ece00: 'vt40', 0x4ece95: 'vt20', 0x4ecea3: 'vtb4', 0x4eceb4: 'vt134'}
VF108_SITES = {0x4cfce6, 0x4cfcfb, 0x50c90f, 0x50c92d}
EAX_TRUE = [1, 0x101, 0xFFFFFF01, 0x80]
EAX_FALSE = [0, 0x100, 0xFFFFFF00]
GARBAGE = [0xA5A5A5A5, 0x5A5A5A5A, 0xDEADBEEF, 0x01234567]


class Lab(nm.Machine):
    def __init__(self, precision, image):
        super().__init__(precision)
        if image is nm.modified:
            u = self.u
            u.mem_unmap(nm.original.base, (nm.original.image_size + 0xfff) & ~0xfff)
            u.mem_map(nm.modified.base, (nm.modified.image_size + 0xfff) & ~0xfff)
            for vs, rva, raw, off in nm.modified.sections:
                u.mem_write(nm.modified.base + rva, nm.modified.b[off:off + raw])
        self.image = image
        self.trap = self.stub(self._trap)
        self.sv = {}
        for name, fn in [('20', self._sv20), ('40', self._sv40), ('b4', self._svb4), ('134', self._sv134)]:
            self.sv[name] = self.stub(fn)
        self.synthetic_vt = self.array([self.trap] * 0x80)
        for off, name in [(0x20, '20'), (0x40, '40'), (0xB4, 'b4'), (0x134, '134')]:
            self.w(self.synthetic_vt + off, self.sv[name])
        self.manager_vt = self.array([self.trap, self.stub(self._free), self.trap, self.trap, self.stub(self._clone)])
        self.manager = self.alloc(0x10)
        self.w(self.manager, self.manager_vt)
        self.seam(0x4CF120, self._v20_native)
        self.seam(0x5845A0, self._research)
        self.seam(0x4D1350, self._rebuild)
        self.seam(0x437EB0, self._hud)
        for a in (0x887B50, 0x6A71D0, 0x6A71F0, 0x4D1E90, 0x4E5240, 0x969B10):
            self.observe(a, self._native_virtual)
        self.observe(0x4D1E80, self._vf108)
        self.observe(0x765E00, lambda m: m.events.append(['765e00', m.name(m.ecx())]))
        self.observe(0x4ECD70, lambda m: m.events.append(['4ecd70', m.name(m.ecx())]))
        self.observe(0x76C7F0, self._owner_views)
        self.observe(0x50C8E0, lambda m: m.events.append(['50c8e0', m.name(m.arg(1))]))
        self.observe(0x50C91E, lambda m: m.events.append(['4f5290', m.name(m.u.reg_read(UC_X86_REG_EAX))]))
        self.observe(0x50C93A, lambda m: m.events.append(['4f5290', m.name(m.u.reg_read(UC_X86_REG_EAX))]))
        self.observe(0x50C7D0, lambda m: m.pending.append(m.name(m.arg(1))))
        self.observe(0x50C955, self._list_result)
        self.mark = self.heap
        self.case = None

    # --- bookkeeping -------------------------------------------------------
    def name(self, address):
        if address == 0:
            return None
        return self.labels.get(address, hex(address))

    def _trap(self, m):
        raise RuntimeError('trap: unexpected virtual slot at %s' % hex(self.r(self.u.reg_read(UC_X86_REG_ESP))))

    def _free(self, m):
        m.events.append(['free', m.name(m.arg(1) + 0x10)])
        m.ret(0, 4)

    def _clone(self, m):
        m.ret(m.ecx())

    def _site(self):
        return SITES.get(self.r(self.u.reg_read(UC_X86_REG_ESP)))

    def _record_virtual(self, thing):
        site = self._site()
        assert site is not None, hex(self.r(self.u.reg_read(UC_X86_REG_ESP)))
        if site == 'vt40':
            self.events.append(['vt40', self.name(thing), self.name(self.arg(1)), self.arg(2)])
        elif site == 'vt20':
            self.events.append(['vt20', self.name(thing), self.arg(1)])
        elif site == 'vtb4':
            self.events.append(['vtb4', self.name(thing)])
        else:
            self.events.append(['vt134', self.name(thing), self.name(self.arg(1))])
        return site

    def _native_virtual(self, m):
        m._record_virtual(m.ecx())

    def _vf108(self, m):
        ret = m.r(m.u.reg_read(UC_X86_REG_ESP))
        assert ret in VF108_SITES, hex(ret)
        m.events.append(['vf108', m.name(m.ecx())])

    def _owner_views(self, m):
        assert m.ecx() == m.session
        m.events.append(['76c7f0', m.name(m.arg(1))])

    def _list_result(self, m):
        m.events.append(['50c7d0', m.pending.pop(), m.u.reg_read(UC_X86_REG_EAX) & 0xff])

    def _spec(self, thing):
        return self.case['things'][self.labels[thing]]

    def _v20_native(self, m):  # 0x4CF120 thiscall(this, byte) on a real vtable
        thing = m.ecx()
        m._record_virtual(thing)
        m.ret(m._spec(thing)['v20_eax'], 4)

    def _sv20(self, m):
        thing = m.ecx()
        m._record_virtual(thing)
        m.ret(m._spec(thing)['v20_eax'], 4)

    def _sv40(self, m):
        thing = m.ecx()
        m._record_virtual(thing)
        spec = m._spec(thing)
        index = m.arg(2)
        m.ret(spec['v40_eax'][index], 8)

    def _svb4(self, m):
        thing = m.ecx()
        m._record_virtual(thing)
        m.ret(m._spec(thing)['vb4_eax'])

    def _sv134(self, m):
        thing = m.ecx()
        m._record_virtual(thing)
        spec = m._spec(thing)
        kingdom = m.name(m.arg(1))
        m.ret(spec['v134_eax'].get(str(kingdom), 0), 4)

    def _research(self, m):
        research = m.ecx()
        if research == 0:
            return  # the original prologue runs and reads [0+4]
        m.events.append(['5845a0', m.name(research)])
        m.ret(m.case['researches'][m.labels[research]])

    def _rebuild(self, m):
        m.events.append(['4d1350', m.name(m.ecx())])
        m.ret(0)

    def _hud(self, m):
        assert m.ecx() == m.uiworld
        m.events.append(['437eb0'])
        m.ret(0)

    # --- memory ------------------------------------------------------------
    def cstring(self, text, label):
        units = [ord(c) for c in text]
        h = self.alloc(0x10 + 2 * (len(units) + 1))
        self.w(h, self.manager)
        self.w(h + 4, len(units))
        self.w(h + 8, len(units))
        self.w(h + 0xc, 1)
        for i, ch in enumerate(units):
            self.u.mem_write(h + 0x10 + 2 * i, struct.pack('<H', ch))
        self.labels[h + 0x10] = label
        self.strings.append(h)
        return h + 0x10

    def build(self, c):
        self.case = c
        used = self.heap - self.mark
        if used:
            self.u.mem_write(self.mark, b'\0' * used)
        self.heap = self.mark
        for g in (0xB36530, 0xB36534, 0xB36538, 0xB36550, 0xB36558):
            self.w(g, 0)
        self.labels, self.addr, self.events, self.pending, self.strings = {}, {}, [], [], []
        # objects first so references resolve
        for label in c['kingdoms']:
            self.addr[label] = self.alloc(0x300)
        for label in c['things']:
            if label not in self.addr:
                self.addr[label] = self.alloc(0x300)
        for label in c['views']:
            self.addr[label] = self.alloc(0x80)
        for label in c['states']:
            self.addr[label] = self.alloc(0x40)
        for label in c['researches']:
            self.addr[label] = self.alloc(0x20)
        for p in c['players']:
            self.addr[p['label']] = self.alloc(0x80)
        for label, a in self.addr.items():
            self.labels[a] = label
        ref = lambda label: self.addr[label] if label is not None else 0
        for label, name in c['kingdoms'].items():
            self.w(self.addr[label] + 0x18, self.cstring(name, label + '.name'))
        for label, t in c['things'].items():
            a = self.addr[label]
            kind = t['type']
            self.w(a, self.synthetic_vt if kind == 'synthetic' else VTABLES[kind])
            if 'e8' in t:
                self.w(a + 0xe8, ref(t['e8']))
            if t.get('research') is not None or kind == 'synthetic':
                self.w(a + 0xb4, ref(t.get('research')))
            if 'byte4c' in t:
                self.wb(a + 0x4c, t['byte4c'])
            if 'view' in t:
                self.w(a + 0xc, ref(t['view']))
            if t.get('element') is not None:
                e = self.alloc(0x20)
                org = t['element']
                if org is not None and org.get('organization') is not None:
                    o = ref(org['organization'])
                    comp = self.alloc(0x20)
                    self.w(comp + 4, ref(org['owner']))
                    self.w(o + 0x7c, comp)
                    self.w(e + 0x10, o)
                self.w(a + 0x80, e)
        for label, v in c['views'].items():
            self.w(self.addr[label] + 0x54, v['flags'])
        for label, s in c['states'].items():
            a = self.addr[label]
            self.w(a, ref(s['kingdom']))
            nodes = []
            for t in s['nodes']:
                n = self.alloc(8)
                self.w(n, ref(t))
                nodes.append(n)
            for i, n in enumerate(nodes):
                self.w(n + 4, nodes[i + 1] if i + 1 < len(nodes) else 0)
            self.w(a + 4, nodes[0] if nodes else 0)
            self.w(a + 8, 0 if s.get('tail_null') or not nodes else nodes[-1])
            for i, g in enumerate(GARBAGE):
                self.w(a + 0x10 + 4 * i, g)
            if s['group'] is not None:
                g = self.alloc(0x20)
                self.wb(g + 0x18, s['group'])
                self.w(a + 0x20, g)
        if c['defs'] is not None:
            rng = random.Random(len(self.case['name']))
            defs = []
            for b3c in c['defs']:
                d = self.alloc(0x40)
                self.u.mem_write(d, bytes(rng.randrange(256) for _ in range(0x40)))
                self.wb(d + 0x3c, b3c)
                defs.append(d)
            game = self.alloc(0x300)
            self.w(game + 0x298, self.array(defs))
            self.w(0xB36530, game)
        w = c['world']
        if w is not None:
            world = self.alloc(0x200)
            self.labels[world] = 'world'
            self.w(world + 0xd8, w['phase'])
            self.w(world + 0x48, w['w48'])
            if w['settings'] is not None:
                s = self.alloc(0x100)
                self.w(s + 0xa8, w['settings'])
                self.w(world + 0x28, s)
            slots = [ref(k) for k in w['kingdom_slots']]
            self.w(world + 0x148, self.array(slots))
            self.w(world + 0x14c, len(slots))
            self.w(world + 0x130, self.alloc(0x10))
            self.w(0xB36534, world)
        session = self.session = self.alloc(0x100)
        self.labels[session] = 'session'
        self.w(session + 0xe4, c['app_state'])
        nodes = []
        for p in c['players']:
            a = self.addr[p['label']]
            self.w(a + 0x24, self.cstring(p['name'], p['label'] + '.name'))
            self.w(a + 0x28, ref(p['kingdom']))
            self.w(a + 0x50, ref(p['view_state']))
            n = self.alloc(8)
            self.w(n, a)
            nodes.append(n)
        for i, n in enumerate(nodes):
            self.w(n + 4, nodes[i + 1] if i + 1 < len(nodes) else 0)
        self.w(session + 0xbc, nodes[0] if nodes else 0)
        self.w(0xB36550, session)
        net = self.alloc(0x20)
        self.w(net + 8, ref(c['local_player']))
        self.w(0xB36558, net)
        self.uiworld = 0
        if c['vworld'] is not None:
            vworld = self.alloc(0x200)
            self.w(0xB36538, vworld)
            if c['vworld']['ui'] is not None:
                ui = self.uiworld = self.alloc(0x80)
                cells = []
                for v in c['vworld']['ui']:
                    n = self.alloc(8)
                    self.w(n, ref(v))
                    cells.append(n)
                for i, n in enumerate(cells):
                    self.w(n + 4, cells[i + 1] if i + 1 < len(cells) else 0)
                self.w(ui + 8, cells[0] if cells else 0)
                self.w(ui + 0xc, cells[-1] if cells else 0)
                self.w(vworld + 0x13c, ui)

    def run(self):
        c = self.case
        call = c['call']
        try:
            if c['suite'] == 'masks':
                self.call(0x4ECD70, ecx=self.addr[call['state']], count=400_000)
            elif c['suite'] == 'owners':
                owner = self.addr[call['owner']] if call['owner'] is not None else 0
                self.call(0x76C7F0, ecx=self.session, args=(owner,), pop=4, count=400_000)
            elif c['suite'] == 'selected':
                self.call(0x50C8E0, ecx=0x13579BDF, args=(self.addr[call['actor']],), pop=4, count=400_000)
            else:
                self.call(0x4CFCC0, ecx=self.addr[call['actor']], count=400_000)
            fault = None
        except (AssertionError, UcError, RuntimeError) as e:
            nulls = [list(map(hex, a)) for a in self.null_accesses]
            self.null_accesses = []
            fault = {'null': nulls, 'error': type(e).__name__}
        masks = {label: [self.r(self.addr[label] + 0x10 + 4 * i) for i in range(4)] for label in c['states']}
        refs = [self.r(h + 0xc) for h in self.strings]
        return {'fault': fault, 'events': self.events, 'masks': masks, 'string_refcounts': refs}


# --- cases ------------------------------------------------------------------
def eax(rng, value):
    return rng.choice(EAX_TRUE if value else EAX_FALSE)


def blank(name, suite):
    return {'name': name, 'suite': suite, 'kingdoms': {}, 'things': {}, 'views': {}, 'states': {},
            'researches': {}, 'players': [], 'world': {'phase': 0, 'w48': 1, 'settings': 0, 'kingdom_slots': []},
            'app_state': 2, 'local_player': None, 'vworld': {'ui': []}, 'defs': [0] * 39, 'call': {}}


def thing(rng, kind, kingdoms, owner=None, research=None):
    t = {'type': kind, 'v20_eax': eax(rng, rng.random() < 0.6)}
    if kind in ('gactor', 'ggroup', 'synthetic'):
        t['e8'] = owner if owner is not None else rng.choice(kingdoms + [None])
    if kind == 'synthetic':
        t['v40_eax'] = [eax(rng, rng.random() < 0.3) for _ in range(39)]
        t['vb4_eax'] = eax(rng, rng.random() < 0.6)
        t['v134_eax'] = {k: eax(rng, rng.random() < 0.5) for k in kingdoms}
        t['research'] = research
    return t


def masks_case(rng, i, kind_pool, faults=False):
    c = blank('masks%03d' % i, 'masks')
    kingdoms = ['K1', 'K2']
    c['kingdoms'] = {'K1': 'Axis', 'K2': 'Allies'}
    for k in kingdoms:
        c['things'][k] = {'type': 'gkingdom', 'v20_eax': eax(rng, rng.random() < 0.6)}
    n = rng.choice([1, 1, 2, 2, 3, 4])
    labels = []
    for j in range(n):
        kind = rng.choice(kind_pool)
        if kind == 'gkingdom':
            labels.append(rng.choice(kingdoms))
            continue
        label = 'A%d' % (j + 1)
        research = None
        if kind == 'synthetic' and rng.random() < 0.7:
            research = 'R%d' % (j + 1)
            c['researches'][research] = eax(rng, rng.random() < 0.5)
        c['things'][label] = thing(rng, kind, kingdoms, research=research)
        labels.append(label)
    if rng.random() < 0.25 and len(labels) > 1:
        labels[-1] = labels[0]  # head and tail name the same thing
    if rng.random() < 0.15:
        labels.append(labels[0])
    c['states']['S1'] = {'kingdom': rng.choice(kingdoms + [None, 'K1']), 'nodes': labels,
                         'group': rng.choice([None, 0, 1, 0x80]), 'tail_null': False}
    c['defs'] = [rng.choice([0, 0, 1, 0x80]) for _ in range(39)]
    c['call'] = {'state': 'S1'}
    return c


def masks_cases(rng):
    out = []
    pools = [['gactor', 'ggroup', 'gkingdom', 'object'], ['synthetic'], ['gactor', 'synthetic', 'object', 'gkingdom']]
    for i in range(96):
        out.append(masks_case(rng, i, pools[i % 3]))
    # directed rows
    c = blank('masks_empty', 'masks')
    c['states']['S1'] = {'kingdom': None, 'nodes': [], 'group': 1, 'tail_null': False}
    c['call'] = {'state': 'S1'}
    out.append(c)
    c = blank('masks_empty_no_defs', 'masks')
    c['defs'] = None
    c['states']['S1'] = {'kingdom': None, 'nodes': [], 'group': None, 'tail_null': False}
    c['call'] = {'state': 'S1'}
    out.append(c)
    # two distinct eligible own actors with a group: bits 0x25 / 0x26
    for byte in (0, 1):
        c = blank('masks_group_%d' % byte, 'masks')
        c['kingdoms'] = {'K1': 'Axis'}
        c['things']['K1'] = {'type': 'gkingdom', 'v20_eax': 1}
        c['things']['A1'] = {'type': 'gactor', 'e8': 'K1', 'v20_eax': 1}
        c['things']['A2'] = {'type': 'gactor', 'e8': 'K1', 'v20_eax': 0x101}
        c['things']['A3'] = {'type': 'gactor', 'e8': 'K1', 'v20_eax': 1}
        c['states']['S1'] = {'kingdom': 'K1', 'nodes': ['A1', 'A2', 'A3'], 'group': byte, 'tail_null': False}
        c['call'] = {'state': 'S1'}
        out.append(c)
    # synthetic research path at index 0x23 with and without research
    for avail in (1, 0):
        c = blank('masks_research_%d' % avail, 'masks')
        c['things']['A1'] = {'type': 'synthetic', 'e8': None, 'v20_eax': 0, 'v40_eax': [1] * 39, 'vb4_eax': 0,
                             'v134_eax': {}, 'research': 'R1'}
        c['researches']['R1'] = avail
        c['states']['S1'] = {'kingdom': None, 'nodes': ['A1'], 'group': None, 'tail_null': False}
        c['call'] = {'state': 'S1'}
        out.append(c)
    # faults: inconsistent tail, missing definitions with distinct things,
    # NULL research at index 0x23, NULL thing in a node
    c = blank('fault_tail_null', 'masks')
    c['things']['A1'] = {'type': 'gactor', 'e8': None, 'v20_eax': 1}
    c['states']['S1'] = {'kingdom': None, 'nodes': ['A1'], 'group': None, 'tail_null': True}
    c['call'] = {'state': 'S1'}
    out.append(c)
    c = blank('fault_no_defs', 'masks')
    c['defs'] = None
    c['things']['A1'] = {'type': 'gactor', 'e8': None, 'v20_eax': 1}
    c['things']['A2'] = {'type': 'gactor', 'e8': None, 'v20_eax': 1}
    c['states']['S1'] = {'kingdom': None, 'nodes': ['A1', 'A2'], 'group': None, 'tail_null': False}
    c['call'] = {'state': 'S1'}
    out.append(c)
    c = blank('fault_null_research', 'masks')
    c['things']['A1'] = {'type': 'synthetic', 'e8': None, 'v20_eax': 0, 'v40_eax': [1] * 39, 'vb4_eax': 0,
                         'v134_eax': {}, 'research': None}
    c['states']['S1'] = {'kingdom': None, 'nodes': ['A1'], 'group': None, 'tail_null': False}
    c['call'] = {'state': 'S1'}
    out.append(c)
    c = blank('fault_null_thing', 'masks')
    c['things']['A1'] = {'type': 'gactor', 'e8': None, 'v20_eax': 1}
    c['states']['S1'] = {'kingdom': None, 'nodes': ['A1', None], 'group': None, 'tail_null': False}
    c['call'] = {'state': 'S1'}
    out.append(c)
    return out


def session_players(rng, c, kingdoms, count):
    for j in range(count):
        label = 'P%d' % (j + 1)
        state = None
        if rng.random() < 0.8:
            state = 'S%d' % (j + 1)
            nodes = []
            for k in range(rng.choice([0, 1, 1, 2, 3])):
                a = 'A%d_%d' % (j + 1, k + 1)
                c['things'][a] = thing(rng, rng.choice(['gactor', 'gactor', 'object', 'ggroup']), kingdoms)
                nodes.append(a)
            c['states'][state] = {'kingdom': rng.choice(kingdoms + [None]), 'nodes': nodes,
                                  'group': rng.choice([None, 0, 1]), 'tail_null': False}
        c['players'].append({'label': label, 'name': rng.choice(['Axis', 'Allies', 'Observer']),
                             'kingdom': rng.choice(kingdoms + [None]), 'view_state': state})


def owners_cases(rng):
    out = []
    for i in range(48):
        c = blank('owners%03d' % i, 'owners')
        kingdoms = ['K1', 'K2', 'K3']
        c['kingdoms'] = {'K1': 'Axis', 'K2': 'Allies', 'K3': 'Comintern'}
        for k in kingdoms:
            c['things'][k] = {'type': 'gkingdom', 'v20_eax': eax(rng, rng.random() < 0.5)}
        session_players(rng, c, kingdoms, rng.choice([0, 1, 2, 3, 4]))
        if i % 8 == 7:
            c['world'] = None  # 765E00 returns NULL without a GWorld
        c['defs'] = [rng.choice([0, 1]) for _ in range(39)]
        c['call'] = {'owner': rng.choice(kingdoms + [None])}
        out.append(c)
    return out


def selected_world(rng, c, kingdoms):
    c['world'] = {'phase': rng.choice([0, 1, 5, 6, 7]), 'w48': rng.choice([0, 1]),
                  'settings': rng.choice([None, 0, 0, 1, 4]), 'kingdom_slots': rng.sample(kingdoms + [None], 3)}
    c['app_state'] = rng.choice([2, 2, 3, 4])
    c['local_player'] = 'P1' if rng.random() < 0.85 else None


def selected_cases(rng):
    out = []
    for i in range(96):
        c = blank('selected%03d' % i, 'selected')
        kingdoms = ['K1', 'K2']
        c['kingdoms'] = {'K1': 'Axis', 'K2': 'Allies'}
        for k in kingdoms:
            c['things'][k] = {'type': 'gkingdom', 'v20_eax': 0}
        selected_world(rng, c, kingdoms)
        if c['world']['settings'] is None and rng.random() < 0.5:
            c['world']['settings'] = 0
        c['players'].append({'label': 'P1', 'name': rng.choice(['Axis', 'Allies', 'Nobody']),
                             'kingdom': rng.choice(kingdoms + [None]), 'view_state': None})
        view = 'V1' if rng.random() < 0.9 else None
        if view:
            c['views'][view] = {'flags': rng.choice([1 << 17, 0, (1 << 17) | 0x100004, 0xfffdffff, 0xffffffff])}
        c['things']['A1'] = {'type': 'gactor', 'e8': rng.choice(kingdoms + [None, 'K1']), 'v20_eax': 0, 'view': view}
        others = ['V2'] if rng.random() < 0.5 else []
        for o in others:
            c['views'][o] = {'flags': 1 << 17}
        ui = others + ([view] if view and rng.random() < 0.7 else [])
        rng.shuffle(ui)
        c['vworld'] = {'ui': ui}
        c['call'] = {'actor': 'A1'}
        out.append(c)
    # directed: every gate passes, by-name (w48 = 0) and by local player kingdom
    for w48, settings in ((0, 0), (1, 0), (1, 1), (1, None)):
        c = blank('selected_pass_%d_%s' % (w48, settings), 'selected')
        c['kingdoms'] = {'K1': 'Axis', 'K2': 'Allies'}
        for k in ('K1', 'K2'):
            c['things'][k] = {'type': 'gkingdom', 'v20_eax': 0}
        c['world'] = {'phase': 1, 'w48': w48, 'settings': settings, 'kingdom_slots': [None, 'K2', 'K1']}
        c['local_player'] = 'P1'
        c['players'].append({'label': 'P1', 'name': 'Axis', 'kingdom': 'K1', 'view_state': None})
        c['views']['V1'] = {'flags': 1 << 17}
        c['things']['A1'] = {'type': 'gactor', 'e8': 'K1', 'v20_eax': 0, 'view': 'V1'}
        c['vworld'] = {'ui': ['V1']}
        c['call'] = {'actor': 'A1'}
        out.append(c)
    # faults: a NULL UIWorldState / NULL local player on the by-name path
    c = blank('fault_null_uiworld', 'selected')
    c['kingdoms'] = {'K1': 'Axis'}
    c['things']['K1'] = {'type': 'gkingdom', 'v20_eax': 0}
    c['world'] = {'phase': 1, 'w48': 1, 'settings': 0, 'kingdom_slots': ['K1']}
    c['local_player'] = 'P1'
    c['players'].append({'label': 'P1', 'name': 'Axis', 'kingdom': 'K1', 'view_state': None})
    c['views']['V1'] = {'flags': 1 << 17}
    c['things']['A1'] = {'type': 'gactor', 'e8': 'K1', 'v20_eax': 0, 'view': 'V1'}
    c['vworld'] = {'ui': None}
    c['call'] = {'actor': 'A1'}
    out.append(c)
    c = blank('fault_null_local_player', 'selected')
    c['kingdoms'] = {'K1': 'Axis'}
    c['things']['K1'] = {'type': 'gkingdom', 'v20_eax': 0}
    c['world'] = {'phase': 1, 'w48': 0, 'settings': 0, 'kingdom_slots': ['K1']}
    c['local_player'] = None
    c['views']['V1'] = {'flags': 1 << 17}
    c['things']['A1'] = {'type': 'gactor', 'e8': 'K1', 'v20_eax': 0, 'view': 'V1'}
    c['vworld'] = {'ui': ['V1']}
    c['call'] = {'actor': 'A1'}
    out.append(c)
    return out


def publish_cases(rng):
    out = []
    for i in range(64):
        c = blank('publish%03d' % i, 'publish')
        kingdoms = ['K1', 'K2']
        c['kingdoms'] = {'K1': 'Axis', 'K2': 'Allies'}
        for k in kingdoms:
            c['things'][k] = {'type': 'gkingdom', 'v20_eax': eax(rng, rng.random() < 0.5)}
        selected_world(rng, c, kingdoms)
        c['world']['settings'] = rng.choice([0, 0, 1])
        session_players(rng, c, kingdoms, rng.choice([1, 2, 3]))
        c['local_player'] = 'P1' if rng.random() < 0.9 else None
        view = 'V1' if rng.random() < 0.8 else None
        if view:
            c['views'][view] = {'flags': rng.choice([1 << 17, 0, (1 << 17) | 0x100000])}
        element = None
        r = rng.random()
        if r < 0.3:
            c['things']['O1'] = {'type': 'gactor', 'e8': None, 'v20_eax': 0}
            c['things']['H1'] = {'type': 'gactor', 'e8': None, 'v20_eax': 0}
            element = {'organization': 'O1', 'owner': 'H1'}
        elif r < 0.6:
            element = {'organization': None}
        c['things']['X1'] = {'type': 'gactor', 'e8': rng.choice(kingdoms + [None]), 'v20_eax': 0, 'view': view,
                             'byte4c': rng.choice([0, 1, 2, 0x7f, 0x80, 0xff]), 'element': element}
        if c['states'] and rng.random() < 0.6:
            s = rng.choice(sorted(c['states']))
            c['states'][s]['nodes'].insert(0, 'X1')
        c['vworld'] = {'ui': [view] if view and rng.random() < 0.7 else []}
        c['defs'] = [rng.choice([0, 1]) for _ in range(39)]
        c['call'] = {'actor': 'X1'}
        out.append(c)
    return out


def cases():
    rng = random.Random(0x4CFCC0)
    return masks_cases(rng) + owners_cases(rng) + selected_cases(rng) + publish_cases(rng)


def produce():
    all_cases = cases()
    runs, visited = [], {}
    for image_name, image in (('original', nm.original), ('v14', nm.modified)):
        for precision in nm.PRECISIONS:
            m = Lab(precision, image)
            rows = []
            for case in all_cases:
                m.build(case)
                rows.append(m.run())
            m.verify_v14()
            visited.update(m.visited)
            runs.append({'image': image_name, 'precision': precision, 'rows': rows})
    base = runs[0]
    for r in runs[1:]:
        assert r['rows'] == base['rows'], (r['image'], r['precision'])
    coverage, unexecuted = {}, {}
    for name, (lo, hi) in FUNCTIONS.items():
        coverage[name] = sorted(hex(a) for a in visited if lo <= a < hi)
        code = nm.original.range(lo, hi - lo)
        from capstone import Cs, CS_ARCH_X86, CS_MODE_32
        all_insns = [i.address for i in Cs(CS_ARCH_X86, CS_MODE_32).disasm(code, lo) if i.mnemonic != 'int3']
        unexecuted[name] = [hex(a) for a in all_insns if a not in visited]
    head = nm.header(__file__, 'Unicorn execution of original 0x4CFCC0 / 0x76C7F0 / 0x4ECD70 / 0x50C8E0 with explicit seams '
                                '(see producer docstring)')
    head.update({'runs': [{'image': r['image'], 'precision': r['precision']} for r in runs],
                 'identical_across_runs': True, 'cases': all_cases, 'rows': base['rows'],
                 'coverage': coverage, 'unexecuted': unexecuted, 'instructions': nm.instructions(visited)})
    return head


def main():
    data = json.dumps(produce(), indent=1, sort_keys=True).encode()
    if '--write' in sys.argv:
        if OUT.exists():
            raise SystemExit(f'{OUT} exists; refusing to overwrite')
        OUT.write_bytes(data)
        print('wrote', OUT, hashlib.sha256(data).hexdigest())
    else:
        old = OUT.read_bytes()
        assert old == data, 'replay differs'
        print('replay identical', hashlib.sha256(data).hexdigest())


if __name__ == '__main__':
    main()
