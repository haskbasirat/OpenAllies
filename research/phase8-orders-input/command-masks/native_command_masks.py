"""Oracle producer for the UIWorldState command masks 0x437EB0, the research
availability test 0x5845A0 and the selectable actors' virtual +0x154.

Runs ORIGINAL code in the original AA.exe and in the pinned v14 image at
PC24/PC53/PC64 (research/phase8-ai/native_machine.py, page-0 guard, every
executed instruction byte-checked against both images). Three suites:

1. masks: 0x437EB0(this=UIWorldState) with 0x45B7A0, 0x96D710 (__allshl) and
   0x4364C0 (Organization member iterator, filter byte 0) executed natively.
   Laboratory memory: UIWorldState +08 head node / +0C tail node ({view,
   next}), +20..+2C pre-filled with garbage, +40 group (NULL or +18 byte),
   +4C kingdom word; [B36530]+298 the 39 definition pointers (+3C byte; the
   other bytes are noise; NULL entries in fault rows); views (+14 thing or
   NULL); things are either synthetic actors (+7C Organization, +100/+104
   state words, +B4 Research pointer; synthetic vtable whose +B4, +154, +20
   and +134 are Python seams returning raw EAX, every other slot traps) or
   class-2 Objects with the REAL primary vtable A2A2A0 (+B4 = 6A71F0 and
   +20 = 969B10 run natively); Organizations (+28 member array, +2C count,
   +8C byte). Seams: 5C0E30 (local kingdom, recorded per call) and 5845A0
   (research availability, recorded with its research object).
2. research: 0x5845A0(this=Research component) natively; component+4 owner,
   owner+4 ActorData, ActorData+558 head-first item list, component+10 list;
   0x5844A0 is a seam returning raw EAX per item (recorded with its args).
3. v154: actor vt+154(kingdom, index, selected) dispatched through the REAL
   vtables GActor A273F0, GGroup A29490 and GKingdom A299D8 (all 0x4CE680),
   so 0x4CE680, 0x4CF120 (vt+20, including Element +10 parent tail calls
   through the parent's real vtable), 0x4D1F90 (vt+34) and 0x96D710 run
   natively. Seams: 0x40EAA0 (this must be B39690) and 0x76AC90 (this must
   be [B36550]). The same object's vt+20(1) and vt+34(index) are also called
   directly (fresh calls) so the Rust 4CE680 kernel can replay the
   composition with the measured leaf results.

  python native_command_masks.py --write   # create (refuses to overwrite)
  python native_command_masks.py           # replay: byte-identical check
"""
from pathlib import Path
import hashlib, json, random, sys
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'research/phase8-ai'))
import native_machine as nm  # noqa: E402
from unicorn import UcError  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ESP  # noqa: E402

OUT = HERE / 'native-command-masks.json'
CONTROLS = ROOT / 'research/skirmish-20261016-player-orders/command-ui/native-actual-controls.json'
FUNCTIONS = {'437EB0': (0x437eb0, 0x438172), '45B7A0': (0x45b7a0, 0x45b7c1), '4364C0': (0x4364c0, 0x436509),
             '96D710': (0x96d710, 0x96d72f), '5845A0': (0x5845a0, 0x5845ee), '4CE680': (0x4ce680, 0x4ce6be),
             '4CF120': (0x4cf120, 0x4cf19d), '4D1F90': (0x4d1f90, 0x4d1fc1), '6A71F0': (0x6a71f0, 0x6a71f3),
             '969B10': (0x969b10, 0x969b15)}
VTABLES = {'GActor': 0xA273F0, 'GGroup': 0xA29490, 'GKingdom': 0xA299D8, 'Object': 0xA2A2A0}
PINNED_SLOTS = [0x20, 0x34, 0xB4, 0x134, 0x150, 0x154]
KINGDOMS = [0, 0x13579BDF, 0x2468ACE1]
EAX_TRUE = [1, 1, 0x101, 0xFFFFFF01, 0x80]
EAX_FALSE = [0, 0, 0x100, 0xFFFFFF00]
SESSION = 0x0BADC0DE


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
        self.vt = self.array([self.trap] * 0x80)
        for off, fn in [(0xB4, self._b4), (0x154, self._v154), (0x20, self._v20), (0x134, self._v134)]:
            self.w(self.vt + off, self.stub(fn))
        self.seam(0x5C0E30, self._kingdom)
        self.seam(0x5845A0, self._research)
        self.seam(0x5844A0, self._blocked)
        self.seam(0x40EAA0, self._gate)
        self.seam(0x76AC90, self._phase)
        self.observe(0x4CF120, lambda m: m.events.append(['4cf120', m.name(m.ecx()), m.arg(1)]))
        self.observe(0x4D1F90, lambda m: m.events.append(['4d1f90', m.name(m.ecx()), m.arg(1)]))
        self.game = self.alloc(0x300)
        self.w(0xB36530, self.game)
        self.w(0xB3653C, 0x0C0FFEE0)
        self.mark = self.heap
        self.case = None
        self.names = {}

    def alloc(self, size, align=16):
        p = super().alloc(size, align)
        self.u.mem_write(p, bytes(max(size, 4)))
        return p

    def name(self, p):
        return self.names.get(p, hex(p))

    def reset(self, case):
        self.heap = self.mark
        self.case = case
        self.events = []
        self.names = {}

    def run(self, entry, ecx, args, pop):
        try:
            al = self.call(entry, ecx=ecx, args=args, pop=pop, count=200_000)
        except (AssertionError, UcError) as e:
            nulls = [list(map(hex, a)) for a in self.null_accesses]
            self.null_accesses = []
            return None, {'null': nulls, 'error': type(e).__name__}
        return al, None

    # seams ----------------------------------------------------------------
    def _trap(self, m):
        raise AssertionError('unexpected virtual call')

    def _thing(self, m):
        return m.case['things'][m.names[m.ecx()]]

    def _b4(self, m):
        t = m.names[m.ecx()]
        m.events.append(['b4', t])
        m.ret(m.case['things'][t]['b4'], pop=0)

    def _v154(self, m):
        t = m.names[m.ecx()]
        k, index, selected = m.arg(1), m.arg(2), m.arg(3)
        m.events.append(['154', t, k, index, selected])
        thing = m.case['things'][t]
        m.ret(thing['eax154'][1] if index in thing['can154'] else thing['eax154'][0], pop=0xC)

    def _v20(self, m):
        t = m.names[m.ecx()]
        m.events.append(['20', t, m.arg(1)])
        m.ret(m.case['things'][t]['v20'], pop=4)

    def _v134(self, m):
        t = m.names[m.ecx()]
        m.events.append(['134', t, m.arg(1)])
        thing = m.case['things'][t]
        m.ret(thing['v134'][str(m.arg(1))], pop=4)

    def _kingdom(self, m):
        m.events.append(['5c0e30'])
        m.ret(m.case['kingdom_5c0e30'], pop=0)

    def _research(self, m):
        r = m.names[m.ecx()]
        m.events.append(['5845a0', r])
        m.ret(m.case['research'][r], pop=0)

    def _blocked(self, m):
        assert m.ecx() == m.component
        item, notify = m.names[m.arg(1)], m.arg(2)
        m.events.append(['5844a0', item, notify])
        m.ret(m.case['blocked'][item], pop=8)

    def _gate(self, m):
        assert m.ecx() == 0xB39690
        m.events.append(['40eaa0'])
        m.ret(m.case['gate'], pop=0)

    def _phase(self, m):
        assert m.ecx() == SESSION
        m.events.append(['76ac90'])
        m.ret(m.case['phase'], pop=0)

    # suite 1: 0x437EB0 -------------------------------------------------------
    def masks(self, case):
        self.reset(case)
        defs = []
        for d in case['definitions']:
            if d is None:
                defs.append(0)
                continue
            p = self.alloc(0x48)
            self.u.mem_write(p, bytes(case['noise'][:0x48]))
            self.wb(p + 0x3C, d)
            defs.append(p)
        self.w(self.game + 0x298, self.array(defs))
        things = {}
        for name, t in case['things'].items():
            p = things[name] = self.alloc(0x200)
            self.names[p] = name
        researches = {}
        for name in case['research']:
            p = researches[name] = self.alloc(0x20)
            self.names[p] = name
        for name, t in case['things'].items():
            p = things[name]
            if t['kind'] == 'object':
                self.w(p, VTABLES['Object'])
                continue
            self.w(p, self.vt)
            if t['org'] is not None:
                o = t['org']
                org = self.alloc(0x100)
                self.wb(org + 0x8C, o['b8c'])
                self.w(org + 0x28, self.array([things[x] if x else 0 for x in o['members']]))
                self.w(org + 0x2C, len(o['members']))
                self.w(p + 0x7C, org)
            self.w(p + 0x100, t['state'][0])
            self.w(p + 0x104, t['state'][1])
            self.w(p + 0xB4, researches[t['research']] if t['research'] else 0)
            self.w(p + 0xE8, 0x5A5A00E8)
            self.w(p + 0xF0, 0x5A5A00F0)
        views = []
        for v in case['views']:
            p = self.alloc(0x40)
            self.w(p + 0x14, things[v] if v else 0)
            views.append(p)
        uiw = self.alloc(0x60)
        head, tail = 0, 0
        for i in reversed(case['selected']):
            node = self.alloc(8)
            self.w(node, views[i])
            self.w(node + 4, head)
            if tail == 0:
                tail = node
            head = node
        self.w(uiw + 8, head)
        self.w(uiw + 0xC, tail)
        for i, w in enumerate(case['garbage']):
            self.w(uiw + 0x20 + 4 * i, w)
        group = 0
        if case['group18'] is not None:
            group = self.alloc(0x24)
            self.wb(group + 0x18, case['group18'])
        self.w(uiw + 0x40, group)
        self.w(uiw + 0x4C, case['kingdom_4c'])
        al, fault = self.run(0x437EB0, uiw, (), 0)
        out = {'fault': fault, 'events': self.events}
        if fault is None:
            out['masks'] = [self.r(uiw + 0x20 + 4 * i) for i in range(4)]
        return out

    # suite 2: 0x5845A0 -------------------------------------------------------
    def research(self, case):
        self.reset(case)
        items = {}
        for name in case['blocked']:
            p = items[name] = self.alloc(0x30)
            self.names[p] = name

        def chain(names):
            head = 0
            for n in reversed(names):
                node = self.alloc(8)
                self.w(node, items[n])
                self.w(node + 4, head)
                head = node
            return head
        definition = self.alloc(0x600)
        self.w(definition + 0x558, chain(case['items_558']))
        owner = self.alloc(0x40)
        self.w(owner + 4, definition)
        self.component = self.alloc(0x18)
        self.w(self.component + 4, owner)
        self.w(self.component + 0x10, chain(case['listed_10']))
        seam = self.seams.pop(0x5845A0)
        try:
            al, fault = self.run(0x5845A0, self.component, (), 0)
        finally:
            self.seams[0x5845A0] = seam
        return {'fault': fault, 'events': self.events, 'al': None if fault else al & 0xFF}

    # suite 3: vt+154 through the real vtables ---------------------------------
    def build_actors(self, case):
        actors = {}
        for name in case['actors']:
            p = actors[name] = self.alloc(0x200)
            self.names[p] = name
        for name, a in case['actors'].items():
            p = actors[name]
            self.w(p, VTABLES[a['vtable']])
            self.w(p + 8, a['flags8'])
            if a['element'] is not None:
                e = self.alloc(0x20)
                self.w(e + 0x10, actors[a['element']] if a['element'] else 0)
                self.w(p + 0x80, e)
            for off, key in [(0x6C, 'mover'), (0xAC, 'mobile'), (0xA0, 'recruit'), (0x88, 'denizen')]:
                if a[key]:
                    self.w(p + off, self.alloc(0x10))
            self.w(p + 0xE8, KINGDOMS[a['owner']] if a['owner'] < len(KINGDOMS) else a['owner'])
            self.w(p + 0xF0, a['bits'] & 0xFFFFFFFF)
            self.w(p + 0xF4, a['bits'] >> 32)
        self.w(0xB36550, SESSION)
        return actors

    def v154(self, case):
        self.reset(case)
        actors = self.build_actors(case)
        p = actors['a']
        vt = VTABLES[case['actors']['a']['vtable']]
        k, index, selected = case['args']
        out = {}
        al, fault = self.run(self.r(vt + 0x154), p, (k, index, selected), 0xC)
        out['call'] = {'fault': fault, 'events': self.events, 'al': None if fault else al & 0xFF}
        for slot, args, key in [(0x20, (1,), 'vt20'), (0x34, (index,), 'vt34')]:
            self.events = []
            al, fault = self.run(self.r(vt + slot), p, args, 4)
            out[key] = {'fault': fault, 'events': self.events, 'al': None if fault else al & 0xFF}
        return out


# case generation ------------------------------------------------------------
def actual_non_group():
    run = json.loads(CONTROLS.read_text())['runs'][0]
    by = {a['ids']: a['ui_flags_3c_to_41_and_position_44'] for a in run['authored']}
    return [{'ids': d['ids'], 'b3c': by[d['ids']][0]} for d in run['definitions']]


ACTUAL = actual_non_group()


def eax(rng, truth):
    return rng.choice(EAX_TRUE if truth else EAX_FALSE)


def mask_cases(rng):
    cases = []
    for k in range(900):
        style = rng.choice(['random'] * 5 + ['retreat', 'research', 'single', 'tail'])
        nthings = rng.randint(1, 7)
        names = ['t%d' % i for i in range(nthings)]
        research = {}
        things = {}
        for n in names:
            if style != 'retreat' and rng.random() < 0.12:
                things[n] = {'kind': 'object'}
                continue
            org = None
            if rng.random() < (0.9 if style == 'retreat' else 0.45):
                members = [rng.choice(names + [None]) for _ in range(rng.randint(0, 4))]
                org = {'b8c': rng.choice([1, 0x80, 0xFF]) if rng.random() < (0.95 if style == 'retreat' else 0.5) else 0,
                       'members': members}
            state_lo = rng.randrange(1 << 32)
            if rng.random() < (0.95 if style == 'retreat' else 0.5):
                state_lo |= 0x10
            else:
                state_lo &= ~0x10
            r = None
            if rng.random() < (0.6 if style == 'research' else 0.3):
                r = 'r%d' % len(research)
                research[r] = eax(rng, rng.random() < 0.5)
            pool = list(range(0x27))
            can = set(rng.sample(pool, rng.randint(0, 12)))
            for special in (7, 8, 0x23):
                if rng.random() < 0.5:
                    can.add(special)
            if style == 'research' and rng.random() < 0.8:
                can.add(0x23)
            things[n] = {'kind': 'actor', 'b4': eax(rng, rng.random() < 0.88), 'org': org,
                         'state': [state_lo, rng.randrange(1 << 32)], 'research': r,
                         'can154': sorted(can), 'eax154': [eax(rng, False), eax(rng, True)],
                         'v20': eax(rng, rng.random() < 0.7),
                         'v134': {str(x): eax(rng, rng.random() < 0.5) for x in KINGDOMS}}
        nviews = rng.randint(1, 7)
        views = [rng.choice(names) if rng.random() > 0.1 else None for _ in range(nviews)]
        if k % 25 == 0:
            selected = []
        elif style == 'single':
            selected = [rng.randrange(nviews)]
        else:
            selected = rng.sample(range(nviews), rng.randint(1, nviews))
        if rng.random() < 0.3:
            definitions = [d['b3c'] for d in ACTUAL]
        else:
            definitions = [rng.choice([0, 0, 0, 1, 0x80, 0xFF]) for _ in range(0x27)]
        if rng.random() < 0.05:
            definitions[rng.randrange(0x27)] = None
        group18 = rng.choice([None, 0, 1, 0x80]) if rng.random() < 0.9 else None
        if style == 'tail':
            group18 = rng.choice([0, 1, 0x40])
        cases.append({'style': style, 'things': things, 'research': research, 'views': views, 'selected': selected,
                      'definitions': definitions, 'group18': group18,
                      'kingdom_4c': rng.choice(KINGDOMS) if style != 'tail' else rng.choice(KINGDOMS[1:]),
                      'kingdom_5c0e30': rng.choice(KINGDOMS),
                      'garbage': [rng.randrange(1 << 32) for _ in range(4)],
                      'noise': [rng.randrange(256) for _ in range(0x48)]})
    return cases


def research_cases(rng):
    cases = []
    for k in range(240):
        n = rng.randint(0, 6)
        names = ['i%d' % i for i in range(n + 2)]
        items = [rng.choice(names) for _ in range(n)]
        listed = rng.sample(names, rng.randint(0, min(4, len(names))))
        blocked = {x: rng.choice([0, 0, 0, 1, 2, 4, 0x100, 0xFFFFFFFF, 0x80000000]) for x in names}
        cases.append({'items_558': items, 'listed_10': listed, 'blocked': blocked})
    return cases


def v154_cases(rng):
    cases = []
    indices = list(range(0x27)) + [0x3F, 0x40, 0x41, 0x105, 0x1FF, 0xFFFFFFFF]
    for k in range(720):
        vtable = ['GActor', 'GGroup', 'GKingdom'][k % 3]
        chain = rng.choice([0, 0, 0, 1, 2])
        actors = {}
        names = ['a'] + ['p%d' % i for i in range(chain)]
        for i, n in enumerate(names):
            parent = names[i + 1] if i + 1 < len(names) else None
            element = parent if parent else (0 if rng.random() < 0.15 else None)
            bits = rng.randrange(1 << 64)
            if rng.random() < 0.3:
                bits = 0
            actors[n] = {'vtable': vtable if n == 'a' else rng.choice(['GActor', 'GGroup', 'GKingdom']),
                         'flags8': rng.randrange(1 << 32) & ~(4 if rng.random() < 0.8 else 0),
                         'element': element, 'mover': rng.random() < 0.7, 'mobile': rng.random() < 0.3,
                         'recruit': rng.random() < 0.25, 'denizen': rng.random() < 0.25,
                         'owner': rng.choice([0, 1, 1, 1, 2, 0x5A5A5A5A]), 'bits': bits}
        args = [rng.choice(KINGDOMS + KINGDOMS[1:2] * 2), rng.choice(indices), rng.choice([0, 1, 1, 1, 0x100, 0xFFFFFF01, 0x80])]
        cases.append({'actors': actors, 'args': args, 'gate': eax(rng, rng.random() < 0.5),
                      'phase': rng.choice([7, 7, 0, 6, 8, 0x107])})
    return cases


def produce():
    rng = random.Random(0x437EB0)
    masks, research, v154 = mask_cases(rng), research_cases(rng), v154_cases(rng)
    runs, visited = [], {}
    for image_name, image in (('original', nm.original), ('v14', nm.modified)):
        for precision in nm.PRECISIONS:
            m = Lab(precision, image)
            out = {'masks': [m.masks(c) for c in masks], 'research': [m.research(c) for c in research],
                   'v154': [m.v154(c) for c in v154]}
            m.verify_v14()
            visited.update(m.visited)
            runs.append({'image': image_name, 'precision': precision, 'rows': out})
    base = runs[0]['rows']
    for r in runs[1:]:
        assert r['rows'] == base, (r['image'], r['precision'])
    for k, v in VTABLES.items():
        for o in PINNED_SLOTS:
            assert nm.original.range(v + o, 4) == nm.modified.range(v + o, 4), (k, hex(o))
    from capstone import Cs, CS_ARCH_X86, CS_MODE_32
    md = Cs(CS_ARCH_X86, CS_MODE_32)
    coverage = {}
    for name, (lo, hi) in FUNCTIONS.items():
        every = [i.address for i in md.disasm(nm.original.range(lo, hi - lo), lo)]
        coverage[name] = {'executed': len([a for a in every if a in visited]), 'instructions': len(every),
                          'unexecuted': [hex(a) for a in every if a not in visited]}
    head = nm.header(__file__, 'Unicorn execution of original 0x437EB0 / 0x5845A0 / vt+154 with explicit seams (see producer docstring)')
    head.update({'runs': [{'image': r['image'], 'precision': r['precision']} for r in runs], 'identical_across_runs': True,
                 'vtable_pins': {k: {hex(o): hex(int.from_bytes(img.range(v + o, 4), 'little'))
                                     for o in PINNED_SLOTS} for k, v in VTABLES.items() for img in [nm.original]},
                 'definitions_actual': ACTUAL, 'kingdoms': KINGDOMS,
                 'masks': [{'case': c, 'native': r} for c, r in zip(masks, base['masks'])],
                 'research': [{'case': c, 'native': r} for c, r in zip(research, base['research'])],
                 'v154': [{'case': c, 'native': r} for c, r in zip(v154, base['v154'])],
                 'coverage': coverage, 'instructions': nm.instructions(visited)})
    return head


def main():
    data = json.dumps(produce(), indent=0, sort_keys=True).encode()
    if '--dry' in sys.argv:
        doc = json.loads(data)
        from collections import Counter
        print(doc['coverage'])
        print('mask faults', Counter(str(r['native']['fault']['null']) if r['native']['fault'] else None for r in doc['masks']))
        print('research', Counter(r['native']['al'] for r in doc['research']))
        print('v154', Counter((r['case']['actors']['a']['vtable'], r['native']['call']['al']) for r in doc['v154']))
        bits = Counter()
        for r in doc['masks']:
            if r['native']['fault'] is None:
                mk = r['native']['masks']
                for i in range(64):
                    if (mk[0] | mk[1] << 32) >> i & 1:
                        bits['any%d' % i] += 1
                    if (mk[2] | mk[3] << 32) >> i & 1:
                        bits['all%d' % i] += 1
        print(sorted(bits.items()))
        print(len(data), 'bytes')
        return
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
