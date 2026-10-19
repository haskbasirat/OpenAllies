"""Orders view/actor leaves over kingdoms, visibility, seen records and links,
run as ORIGINAL AA.exe code under Unicorn. Things and Kingdoms carry COPIES of
the real class vtables (4F9C10 runtime class -> constructor PRIMARY vtable);
only these entries are seams, recorded in call order and answered per case:

  Thing vt+70 (4EA1D0 / 4D3B80) and vt+74 (4EA0D0 / 4D3C50): the ported
    live_battle_visibility NativeVisibility::visible_to / explored_by;
  Kingdom vt+144 (4E52D0): the ported aa_runtime::relations::borrowed::kingdom;
  40EAA0 (aa-netcode can_process_40eaa0);
  the GetCurrentThreadId import [9F01E4] (compared with [B364D4]+48).

Native: view vt+78/7C/80/84 457B70/457BB0/457BF0/457C30, 4EAD70, 4EB340,
4CE890/4EAB20, 4EA350, 4EA430, 4EA3D0, 4EEF70, 4F0930, 40CFA0, 5C0E50,
5C0E10, 76AC60, 76AC70, 765E00, 40CEC0, Kingdom vt+134..140 4E5240/4E5250/
4E5280/4E52A0, view vt+50/54 45BF20/45BD40 (45BBD0, 4BB3C0), Thing vt+58
4EE3C0, vt+84 4EB8B0, vt+20 4CF120 / 969B10, vt+34 4D1F90 (96D710),
437650/436D40, 640B40 (4549C0, 4CE6C0), 5E05D0, 436600, and the 453470 head
(view+9C NULL returns before 42E640/43CD00).
Globals: B36550 Session (+E4, +F8), B36558 Node (+8 player, player+28),
B36534 GWorld (+275), B39AB0 SVar table (+1BC CheatFog), B3A5BC/B3A5C0
UVar PerformanceDisableDoodads, B3653C UI (+1E0), B364C0 ID manager
(+4 u16 generations, +20004 objects), B364D4 MainThread (+48).
The static section also proves B28158/B2815C/B3CC00/B3CC24 are never stored.
Default compares the fixture; --write regenerates it.
"""
from pathlib import Path
import sys, json, hashlib, struct
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase8-ai'))
import native_machine as nm
from capstone import Cs, CS_ARCH_X86, CS_MODE_32

THING_VT = {2: 0xa2a2a0, 3: 0xa273f0, 4: 0xa299d8, 6: 0xa28a00, 7: 0xa28b88, 8: 0xa29490, 9: 0xa28e90, 10: 0xa28d18}
DATA_VT = {'thing': 0xa20770, 'object': 0xa13318, 'actor': 0xa05540, 'doodad': 0xa0f6e0}
VIEW_VT = 0xa02798
KINGDOM_VT = 0xa299d8
MAIN_THREAD, OTHER_THREAD = 0x1111, 0x2222


class World:
    def __init__(self, c, pc, image):
        m = self.m = nm.Machine(pc)
        if image == 'v14':
            old = (nm.original.image_size + 4095) & ~4095
            new = (nm.modified.image_size + 4095) & ~4095
            if new > old:
                m.u.mem_map(nm.original.base + old, new - old)
            for vs, r, size, off in nm.modified.sections:
                if size:
                    m.u.mem_write(nm.modified.base + r, nm.modified.b[off:off + size])
        self.c = c
        self.trace = []
        self.names = {}
        self.ids = m.alloc(0x20004 + 4 * 0x400)
        m.w(0xb364c0, self.ids)
        self.kingdoms = {}
        for name, k in c.get('kingdoms', {}).items():
            self.kingdoms[name] = m.alloc(0x300)
            self.names[self.kingdoms[name]] = name
        self.things = {}
        for name in c.get('things', {}):
            self.things[name] = m.alloc(0x300)
            self.names[self.things[name]] = name
        for name, k in c.get('kingdoms', {}).items():
            self.kingdom(name, k)
        for name, t in c.get('things', {}).items():
            self.thing(name, t)
        self.globals()

    def ptr(self, name):
        if name is None:
            return 0
        return self.kingdoms[name] if name in self.kingdoms else self.things[name]

    def name(self, p):
        return self.names.get(p) if p else None

    def vtable(self, real, seams, size=0x200):
        m = self.m
        vt = m.alloc(size)
        m.u.mem_write(vt, bytes(m.u.mem_read(real, size)))
        for slot, fn in seams.items():
            m.w(vt + slot, m.stub(fn))
        return vt

    def data(self, kind, category, mask, byte_16d):
        m = self.m
        d = m.alloc(0x200)
        m.w(d, DATA_VT[kind])
        cat = m.alloc(0x40)
        m.w(cat + 0xc, category)
        m.w(d + 0x18, cat)
        m.w(d + 0x174, mask)
        m.wb(d + 0x16d, byte_16d)
        return d

    def kingdom(self, name, k):
        m = self.m
        p = self.kingdoms[name]

        def relation(mm, name=name):
            other = self.name(mm.arg(1))
            self.trace.append({'call': 'relation_144', 'kingdom': name, 'other': other})
            mm.ret(k['relations'].get(str(other), 0), 4)
        m.w(p, self.vtable(KINGDOM_VT, {0x144: relation}))
        m.w(p + 4, self.data('actor', 24, 0, 0))
        m.w(p + 8, 0)
        m.w(p + 0x190, k.get('w190', 0))
        m.w(p + 0x18c, k['w18c'])

    def thing(self, name, t):
        m = self.m
        p = self.things[name]
        cls = t['class']

        def answer(kind):
            def seam(mm, name=name):
                k = self.name(mm.arg(1))
                self.trace.append({'call': kind, 'thing': name, 'kingdom': k, 'deep': mm.arg(2) & 0xff})
                mm.ret(int(t.get(kind, {}).get(str(k), False)), 8)
            return seam
        m.w(p, self.vtable(THING_VT[cls], {0x70: answer('visible_70'), 0x74: answer('explored_74')}))
        kind = t.get('data', 'actor' if cls in (3, 4, 8) else 'object')
        m.w(p + 4, self.data(kind, t.get('category', 2 if kind == 'actor' else 7), t.get('mask', 0), t.get('byte_16d', 0)))
        m.w(p + 8, t.get('flags', 0))
        seen = t.get('seen', {})
        m.w(p + 0x58, t.get('seen_count', 4 if seen else 0))
        slots = m.alloc(4 * 8)
        for kname, record in seen.items():
            r = m.alloc(0x40)
            m.w(r + 0x14, self.ptr(record['owner']))
            m.w(r + 0x34, record['flags'])
            m.w(slots + 4 * self.c['kingdoms'][kname]['w18c'], r)
        m.w(p + 0x5c, slots)
        m.w(p + 0xe8, self.ptr(t.get('owner')))
        for slot in t.get('components', []):
            m.w(p + slot, m.alloc(0x60))
        if 'element' in t:
            e = m.alloc(0x40)
            m.w(e + 0x10, self.ptr(t['element']))
            m.w(p + 0x80, e)
        bits = t.get('action_bits', 0)
        m.w(p + 0xf0, bits & 0xffffffff)
        m.w(p + 0xf4, bits >> 32)
        m.w(p + 0x16c, self.handle(t.get('link_16c')))
        if 'structure_34' in t:
            s = m.alloc(0x40)
            m.w(s + 0x34, self.handle(t['structure_34']))
            m.w(p + 0x94, s)
        if 'leader' in t:
            o = m.alloc(0x60)
            m.w(o + 0x40, self.ptr(t['leader']))
            m.w(p + 0x7c, o)

    def handle(self, spec):
        """None, or [slot, generation, target or None, stale]."""
        if spec is None:
            return 0
        slot, gen, target, stale = spec
        m = self.m
        m.u.mem_write(self.ids + 4 + 2 * slot, struct.pack('<H', (gen + (1 if stale else 0)) & 0xffff))
        m.w(self.ids + 0x20004 + 4 * slot, self.ptr(target))
        return (gen << 16) | slot

    def globals(self):
        m, c = self.m, self.c
        s = c['session']
        session = m.alloc(0x200)
        m.w(session + 0xe4, s['state'])
        m.w(session + 0xf8, s.get('phase', 0))
        m.w(0xb36550, session)
        node = m.alloc(0x40)
        player = m.alloc(0x40) if s['local_player'] else 0
        if player:
            m.w(player + 0x28, self.ptr(s['kingdom']))
        m.w(node + 8, player)
        m.w(0xb36558, node)
        gworld = m.alloc(0x300)
        m.wb(gworld + 0x275, c.get('world_275', 0))
        m.w(0xb36534, gworld)
        svars = m.alloc(0x200)
        cheat = m.alloc(4)
        m.wb(cheat, c.get('cheat_fog', 0))
        m.w(svars + 0x1bc, cheat)
        m.w(0xb39ab0, svars)
        perf = m.alloc(4)
        m.wb(perf, c.get('perf_doodads', 0))
        m.w(0xb3a5bc, perf)
        m.w(0xb3a5c0, m.r(0xb3a5c0) | 1)
        ui = m.alloc(0x300)
        m.w(ui + 0x1e0, self.ptr(c.get('ui_kingdom')))
        m.w(0xb3653c, ui)
        main = m.alloc(0x100)
        m.w(main + 0x48, MAIN_THREAD)
        m.w(0xb364d4, main)

        def thread(mm):
            self.trace.append({'call': 'current_thread'})
            mm.ret(MAIN_THREAD if c.get('thread', 'main') == 'main' else OTHER_THREAD, 0)
        m.w(0x9f01e4, m.stub(thread))

        def can_process(mm):
            self.trace.append({'call': 'can_process_40eaa0'})
            mm.ret(int(c.get('can_process', False)), 0)
        m.seam(0x40eaa0, can_process)

    def view(self, thing, flags=0):
        m = self.m
        v = m.alloc(0x200)
        m.w(v, VIEW_VT)
        p = self.ptr(thing)
        m.w(v + 0x14, p)
        m.w(v + 0x10, m.r(p + 4) if p else 0)
        m.w(v + 0x54, flags)
        return v

    def published(self, names):
        """Views at Thing+C, named by their Thing."""
        owner = {}
        for name in names:
            v = self.view(name)
            owner[v] = name
            self.m.w(self.ptr(name) + 0xc, v)
        return owner


VIEW_CLASS = {'class_78': 0x457b70, 'class_7c': 0x457bb0, 'class_80': 0x457bf0, 'class_84': 0x457c30}


def run(c, pc, image):
    w = World(c, pc, image)
    m = w.m
    leaf = c['leaf']
    out = {}
    if leaf in VIEW_CLASS:
        out['value'] = m.call(VIEW_CLASS[leaf], w.view(c['thing']), []) & 0xff
    elif leaf in ('virtual_50', 'virtual_54'):
        v = w.view(c['thing'], c.get('view_flags', 0))
        out['value'] = m.call(0x45bf20 if leaf == 'virtual_50' else 0x45bd40, v, [1]) & 0xff
    elif leaf in ('vt58', 'vt84', 'vt20', 'vt34'):
        t = w.ptr(c['thing'])
        slot = {'vt58': 0x58, 'vt84': 0x84, 'vt20': 0x20, 'vt34': 0x34}[leaf]
        args = {'vt58': [w.ptr(c.get('kingdom')), 1], 'vt84': [w.ptr(c.get('kingdom'))], 'vt20': [c.get('arg', 1)],
                'vt34': [c.get('index', 0)]}[leaf]
        out['entry'] = hex(m.r(m.r(t) + slot))
        out['value'] = m.call(m.r(m.r(t) + slot), t, args) & 0xff
    elif leaf == '437650':
        owner = w.published(c['views'])
        v = next(k for k, n in owner.items() if n == c['thing'])
        r = m.call(0x437650, 0, [v])
        out['result'] = owner.get(r)
    elif leaf == '640b40':
        owner = w.published(c['views'])
        v = next(k for k, n in owner.items() if n == c['thing'])
        r = m.call(0x640b40, 0, [v])
        out['result'] = owner.get(r) if r else None
    elif leaf == '5e05d0':
        v = w.view(c['thing'])
        other = w.view(None)
        r = m.call(0x5e05d0, 0, [v, nm.f32(0.0), other, nm.f32(1.0)])
        out['live'] = r == other
    elif leaf == '436600':
        owner = w.published(c['views'])
        by = {n: k for k, n in owner.items()}
        head = 0
        for name in reversed(c['selection']):
            cell = m.alloc(8)
            m.w(cell, by[name])
            m.w(cell + 4, head)
            head = cell
        sel = m.alloc(0x20)
        m.w(sel + 8, head)
        r = m.call(0x436600, sel, [])
        out['result'] = owner.get(r) if r else None
    elif leaf == '453470':
        v = w.view(c['thing'])
        m.w(v + 0x150, c['link_150'])
        m.w(v + 0x9c, 0)
        m.call(0x453470, v, [])
        out['returned'] = True
    else:
        raise ValueError(leaf)
    out['trace'] = w.trace
    return out, m.visited


def K(w18c, **kw):
    return {'w18c': w18c, 'w190': 0, 'relations': {}, **kw}


KINGDOMS = {'me': K(0, relations={'me': 0, 'enemy': 3, 'ally': 1, 'neutral': 2, 'None': 2}),
            'enemy': K(1), 'ally': K(2), 'neutral': K(3)}
SESSION = {'state': 0, 'phase': 0, 'local_player': True, 'kingdom': 'me'}


def world(**kw):
    d = {'kingdoms': KINGDOMS, 'session': SESSION, 'things': {}}
    d.update(kw)
    return d


def cases():
    potential = 0x20000800
    for leaf in VIEW_CLASS:
        for owner in ['me', 'enemy', 'ally', None]:
            for cls in [3, 2, 8, 4, 6]:
                if cls in (2, 6) and owner != 'enemy':
                    continue
                for visible, explored, flags in [(True, False, 0), (False, False, 0), (False, True, 0), (True, False, 0x80),
                                                 (False, True, potential)]:
                    t = {'class': cls, 'owner': owner, 'flags': flags, 'visible_70': {'me': visible},
                         'explored_74': {'me': explored}}
                    yield {'leaf': leaf, 'thing': 'u', **world(things={'u': t})}
        yield {'leaf': leaf, 'thing': 'u', **world(things={'u': {'class': 3, 'owner': 'enemy', 'mask': 1 << 26}})}
        yield {'leaf': leaf, 'thing': 'u', **world(things={'u': {'class': 3, 'owner': 'enemy', 'flags': potential, 'byte_16d': 1,
                                                                 'explored_74': {'me': True}}})}
        yield {'leaf': leaf, 'thing': 'u', **world(things={'u': {'class': 9, 'data': 'doodad', 'flags': potential,
                                                                 'explored_74': {'me': True}}})}
        for owner in ['ally', 'enemy', 'me']:
            yield {'leaf': leaf, 'thing': 'u', **world(things={'u': {'class': 3, 'owner': 'neutral', 'explored_74': {'me': True},
                                                                     'seen': {'me': {'owner': owner, 'flags': 0}}}})}
        yield {'leaf': leaf, 'thing': 'u', **world(kingdoms=dict(KINGDOMS, me=K(0, w190=1, relations=KINGDOMS['me']['relations'])),
                                                   things={'u': {'class': 3, 'owner': 'enemy', 'explored_74': {'me': True},
                                                                 'seen': {'me': {'owner': 'ally', 'flags': 0}}}})}
        for s, ui in [(dict(SESSION, state=3), 'ally'), (dict(SESSION, local_player=False), 'ally'),
                      (dict(SESSION, local_player=False), None), (dict(SESSION, kingdom=None), 'ally')]:
            yield {'leaf': leaf, 'thing': 'u', 'ui_kingdom': ui,
                   **world(session=s, things={'u': {'class': 3, 'owner': 'ally', 'visible_70': {'me': True, 'ally': True}}})}
        for thread, cheat, w275, state in [('main', 1, 0, 0), ('main', 0, 1, 0), ('main', 0, 0, 0), ('other', 1, 1, 0)]:
            yield {'leaf': leaf, 'thing': 'u', 'thread': thread, 'cheat_fog': cheat, 'world_275': w275,
                   **world(session=dict(SESSION, state=state), things={'u': {'class': 3, 'owner': 'enemy'}})}
        yield {'leaf': leaf, 'thing': None, **world()}
    for leaf in ['virtual_50', 'virtual_54']:
        for visible, explored in [(True, False), (False, False), (False, True)]:
            for view_flags in [0, 1 << 20, 1 << 26]:
                for perf, cat, data, cls in [(0, 2, 'actor', 3), (1, 1, 'object', 2), (1, 0x26, 'object', 2),
                                             (1, 7, 'doodad', 9), (1, 2, 'actor', 3)]:
                    for tflags in [0, 0x400]:
                        if tflags and not perf:
                            continue
                        t = {'class': cls, 'data': data, 'category': cat, 'owner': 'enemy', 'flags': tflags,
                             'visible_70': {'me': visible}, 'explored_74': {'me': explored}}
                        yield {'leaf': leaf, 'thing': 'u', 'view_flags': view_flags, 'perf_doodads': perf,
                               **world(things={'u': t})}
        for thread, cheat, w275 in [('main', 1, 0), ('main', 0, 1), ('other', 1, 0), ('other', 0, 1)]:
            yield {'leaf': leaf, 'thing': 'u', 'thread': thread, 'cheat_fog': cheat, 'world_275': w275,
                   **world(things={'u': {'class': 3, 'owner': 'enemy', 'explored_74': {'me': True}}})}
        for s, ui in [(dict(SESSION, local_player=False), 'ally'), (dict(SESSION, state=3), None), (dict(SESSION, kingdom=None), None),
                      (dict(SESSION, kingdom=None), 'ally')]:
            yield {'leaf': leaf, 'thing': 'u', 'ui_kingdom': ui,
                   **world(session=s, things={'u': {'class': 3, 'owner': 'enemy', 'visible_70': {'ally': True},
                                                   'explored_74': {'ally': True, 'me': True}}})}
    for cls in THING_VT:
        for visible, explored, flags in [(True, False, 0), (False, False, 0), (False, True, 0), (False, True, potential)]:
            for kingdom in ['me', None]:
                yield {'leaf': 'vt58', 'thing': 'u', 'kingdom': kingdom,
                       **world(things={'u': {'class': cls, 'flags': flags, 'visible_70': {str(kingdom): visible},
                                             'explored_74': {str(kingdom): explored}}})}
    for cls in [3, 2, 8]:
        for kingdom, w190, seen, flags, visible in [
                ('me', 0, {}, 0x4000, False), ('me', 0, {}, 0, False), (None, 0, {}, 0x4000, False),
                ('me', 1, {'me': {'owner': 'me', 'flags': 0}}, 0x4000, False),
                ('me', 0, {'me': {'owner': 'me', 'flags': 0x4000}}, 0, False),
                ('me', 0, {'me': {'owner': 'me', 'flags': 0}}, 0x4000, False),
                ('me', 0, {'me': {'owner': 'me', 'flags': 0}}, 0x4000, True),
                ('me', 0, {'ally': {'owner': 'me', 'flags': 0x4000}}, 0, False)]:
            ks = dict(KINGDOMS, me=K(0, w190=w190, relations=KINGDOMS['me']['relations']))
            t = {'class': cls, 'flags': flags, 'seen': seen, 'visible_70': {'me': visible}, 'explored_74': {'me': True}}
            yield {'leaf': 'vt84', 'thing': 'u', 'kingdom': kingdom, **world(kingdoms=ks, things={'u': t})}
    for cls in [3, 8, 2]:
        for flags, comps, can, phase, arg in [(0, [], False, 0, 1), (4, [], False, 0, 1), (0, [0x6c], False, 0, 1),
                                              (0, [0x6c, 0xac], False, 0, 1), (0, [0x6c, 0xa0], False, 0, 1),
                                              (0, [0x6c, 0x88], False, 0, 1), (0, [0x6c], True, 7, 1), (0, [0x6c], True, 6, 1),
                                              (0, [0x6c], True, 7, 0), (0, [0x6c], False, 7, 1)]:
            t = {'class': cls, 'flags': flags, 'components': comps}
            yield {'leaf': 'vt20', 'thing': 'u', 'arg': arg, 'can_process': can,
                   **world(session=dict(SESSION, phase=phase), things={'u': t})}
        for parent in [[], [0x6c]]:
            yield {'leaf': 'vt20', 'thing': 'u', 'arg': 1, **world(things={
                'u': {'class': cls, 'components': [0x6c], 'element': 'p'}, 'p': {'class': 3, 'components': parent}})}
    for cls in [3, 2]:
        for bits in [0, 1, 0x8000000000000000, 0x100000000, 0xffffffffffffffff]:
            for index in [0, 1, 31, 32, 63, 64, 255, 256, 0xffffffff]:
                yield {'leaf': 'vt34', 'thing': 'u', 'index': index, **world(things={'u': {'class': cls, 'action_bits': bits}})}
    for spec in [None, [5, 9, 'p', False], [5, 9, 'p', True], [5, 9, None, False], [5, 9, 'q', False]]:
        things = {'u': {'class': 3, 'link_16c': spec}, 'p': {'class': 3}, 'q': {'class': 2}}
        yield {'leaf': '437650', 'thing': 'u', 'views': ['u', 'p'], **world(things=things)}
    yield {'leaf': '437650', 'thing': 'u', 'views': ['u'], **world(things={'u': {'class': 2}})}
    for spec in [None, [6, 3, 'p', False], [6, 3, 'p', True], [6, 3, None, False], [6, 3, 'q', False]]:
        things = {'u': {'class': 3, 'structure_34': spec}, 'p': {'class': 3}, 'q': {'class': 2}}
        yield {'leaf': '640b40', 'thing': 'u', 'views': ['u', 'p'], **world(things=things)}
        yield {'leaf': '5e05d0', 'thing': 'u', **world(things=things)}
    for visible, explored in [(True, False), (False, False), (False, True)]:
        yield {'leaf': '640b40', 'thing': 'u', 'views': ['u', 'org'],
               **world(things={'u': {'class': 3, 'element': 'org'},
                               'org': {'class': 3, 'visible_70': {'me': visible}, 'explored_74': {'me': explored}}})}
    yield {'leaf': '640b40', 'thing': 'u', 'views': ['u', 'org'],
           **world(things={'u': {'class': 3, 'element': 'org', 'components': [0x7c]}, 'org': {'class': 3}})}
    yield {'leaf': '640b40', 'thing': 'u', 'views': ['u'], **world(things={'u': {'class': 2}})}
    for link in [0, 0x10007]:
        yield {'leaf': '453470', 'thing': 'u', 'link_150': link, **world(things={'u': {'class': 3}})}
    for sel in [['a'], ['a', 'b'], ['b', 'a'], [], ['c', 'a'], ['d', 'b']]:
        things = {'a': {'class': 3, 'leader': 'la'}, 'b': {'class': 3, 'leader': 'lb'}, 'c': {'class': 2}, 'd': {'class': 3},
                  'la': {'class': 3}, 'lb': {'class': 3, 'components': [0xa4]}}
        yield {'leaf': '436600', 'views': ['a', 'b', 'c', 'd', 'la', 'lb'], 'selection': sel, **world(things=things)}


def static_globals():
    """Every absolute reference to the relation/default globals, decoded in
    instruction order from the enclosing int3-padded function start."""
    md = Cs(CS_ARCH_X86, CS_MODE_32)
    b = nm.original.b
    out = {}
    for target in [0xb28158, 0xb2815c, 0xb3cc00, 0xb3cc24]:
        refs = []
        pat = struct.pack('<I', target)
        i = b.find(pat)
        while i >= 0:
            for vs, r, raw, off in nm.original.sections:
                if off <= i < off + raw:
                    va = nm.original.base + r + i - off
                    start = i
                    while start > off and not (b[start - 1] == 0xcc and b[start - 2] == 0xcc):
                        start -= 1
                    for ins in md.disasm(b[start:i + 16], nm.original.base + r + start - off):
                        if ins.address <= va < ins.address + ins.size:
                            if ins.address + ins.size >= va + 4:
                                refs.append({'va': hex(ins.address), 'text': f'{ins.mnemonic} {ins.op_str}'})
                            break
            i = b.find(pat, i + 1)
        def writes(text):
            mnemonic, _, ops = text.partition(' ')
            return ops.startswith('dword ptr [' + hex(target)) and mnemonic not in ('cmp', 'test', 'push', 'call', 'jmp')
        assert not [r for r in refs if writes(r['text'])], refs
        try:
            initial = hex(struct.unpack('<I', nm.original.range(target, 4))[0])
        except ValueError:
            initial = 'uninitialized (zero-filled)'
        out[hex(target)] = {'initial': initial, 'references': refs}
    return out


def main():
    rows, pins = [], {}
    for c in cases():
        for pc in (nm.PRECISIONS if c['leaf'] == '5e05d0' else ['Double53']):
            pair = []
            for image in ['original', 'v14']:
                out, visited = run(c, pc, image)
                pair.append(out)
                pins.update(visited)
            assert pair[0] == pair[1], c
            rows.append({'precision': pc, 'input': c, 'output': pair[0]})
    for a, b in pins.items():
        assert b == nm.original.range(a, len(b)) == nm.modified.range(a, len(b))
    doc = {**nm.header(__file__, __doc__), 'rows': rows, 'instructions': nm.instructions(pins),
           'thing_vtables': {str(k): hex(v) for k, v in THING_VT.items()}, 'globals': static_globals()}
    path = HERE / 'native-view-order-world.json'
    if '--write' in sys.argv:
        path.write_text(json.dumps(doc, indent=1))
    else:
        assert json.loads(path.read_text()) == doc, 'fixture differs'
    print(json.dumps({'rows': len(rows), 'pins': len(pins), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}))


if __name__ == '__main__':
    main()
