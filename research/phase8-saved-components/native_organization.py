"""Original Organization component (slot 7, 0xDC bytes): constructor 579BD0
then saved Load 57D130 on the same object, under Unicorn (native_machine.py).

Native code executed: 579BD0 (4DB020, 4C2290, 4C21E0, 45D7F0, 45D4F0),
57D130, 50BCD0/50B250 (Updatable table at GWorld+284), 8589B0, layout Load
4C3180 with 4C3050/4C2E60, 4C1DB0, 430240/45F180/45D8A0/4849F0, 578300 with
45D580/4CF110, 4C2910 (4012C0), 4FC200, 57CBA0, 45EC60, 4E13F0, 59E300,
4C2D90, 4C0BA0/4C09C0, 57BFF0/57BF50/577F30.
Seams (each call recorded in order with symbolic arguments; see SEAM_DOC):
stream read, 41D290, allocation, registries 463FA0/4BDBA0/48D500, throws
41DFB0/858A00/858AD0/4F0110, logs 404EA0/404F30/5A1040, nested record load
4FAE80, assert dialog 855D30, and the world callbacks of 57CBA0/57BF50/578300
listed in SEAM_DOC. The 579A00 seam applies its Organization+40 store (leader)
because later native branches read +40; nothing else is emulated.
"""
from pathlib import Path
import sys, json, struct, random, copy
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import Machine, P, V, VISITED, H, f32, f64, Throw
from unicorn.x86_const import UC_X86_REG_ECX

W = lambda *v: struct.pack('<' + 'I' * len(v), *[x & 0xffffffff for x in v])


def S(text):
    return struct.pack('<H', len(text)) + text.encode('utf-16-le')


SEAM_DOC = {
    'owner+108': 'owner GActor virtual +108 (Kingdom)',
    'IsGActor': 'loaded thing virtual +B4 (4FC200 assert check, while B3CC47 is 0)',
    '855D30': 'assert dialog "thing->IsGActor()" (returns 0)',
    '4FAE80': 'nested GObject record load (DYNAMIC_THINGS record format), consumes the record',
    '5705A0': 'member Element: set owning organization (owner, slot)',
    '561760': 'member Body notified of owner',
    '4CED10': 'owner experience source token',
    '4CEBB0': 'member: set experience source (0, token)',
    '5788B0': 'Organization combat value recompute',
    'Mind+118': 'owner Mind virtual +118 (old D8, new D8)',
    '4D1080': 'member predicate used for state 0x22',
    '4D64F0': 'owner actor state set (state, enabled)',
    '579A00': 'leader change (seam stores Organization+40 = member)',
    '524B80': 'owner Mind: member added',
    '57C800': 'Organization follow leader for member',
    '4D4CF0': 'owner refresh', '4CF0F0': 'owner refresh', '4D1350': 'owner action publication',
    '5784C0': 'Organization member bookkeeping',
    '4CEF10': 'actor property/effect apply (property, kind, actor)',
    '4CEF70': 'actor property/effect remove (property, actor)',
    '4CEF80': 'actor property/effect present (property, actor)',
    '5147C0': 'owner +148 object +10 notification',
    '501C90': 'Modifier apply (morale value); ST0 result',
    '45EEF0': 'ActorData has Property',
    '4E2A60': 'Kingdom+1FC recruit single find (ActorData, 0)',
    '4E1200': 'recruit item pair with member',
    '4D49B0': 'owner state group 4 descriptor (+20, +24 words)',
    '57BEF0': 'formation refresh', '4D1900': 'owner +164 object', '4B2830': 'object refresh',
}

ACTOR_DATA = {
    # name: (flags174, morale_2c0)
    'regiment_a': (0x0, f32(0.0)),
    'inf': (0x100, f32(0.1)),
    'mg': (0x100, f32(1 / 3)),
    'co': (0x100, f32(20.2)),
    'cart': (0x0, f32(1.0)),
    'spare': (0x100, f32(0.2)),
}
PROPERTIES = ['prop_inf', 'prop_mg', 'prop_co']
LAYOUTS = {
    # name: entries [(property, required, locked, morale_line)], groups, primary, formations, f44, f48, property_50
    'lay3': ([('prop_inf', 1, 0, 1), ('prop_mg', 0, 0, 1), ('prop_co', 1, 1, 0)], [0, 0, 1, 2], 3, 2, 0, 1, 0x7701),
    'lay3b': ([('prop_inf', 0, 0, 0), ('prop_mg', 0, 0, 0), ('prop_co', 0, 0, 0)], [0, 1, 2], -1, 1, 0, 0, 0),
    'lay2': ([('prop_inf', 0, 0, 1), ('prop_mg', 0, 0, 1)], [0, 1], 0, 0, 0, 0, 0x7702),
    'lay1': ([('prop_inf', 0, 0, 1)], [0, 0], -1, 3, 2, 0, 0),
}
FORMATION_DEFS = [(0, 0), (1, 0x7711), (2, 0x7712)]  # (+1A byte, +24 property) by formation index
DAMAGE_TYPES = ['dt_bullet', 'dt_shell']


def base_spec():
    return {
        'precision': 'Double53', 'world_version': 27, 'time_e0': f32(12.5), 'time_e4': f32(100.0),
        'owner': {'gtid': 0x10a0, 'def': 'regiment_a', 'position': [f32(40.0), f32(-8.5)], 'mind': True,
                  'kingdom': {'singles': [0x9901, 0, 0x9903]}, 'modifier': None, 'objects_10': None,
                  'breakpoint_308': f32(0.25), 'flags174': 0},
        'default_layout': ('lay3', ['inf', 'mg', 'co']),
        'table': {'max': 50, 'occupied': [3]},
        'updatable': (1, 7),
        'version': 10,
        'layout_stream': (0, 'lay3', None, ['inf', 'mg', 'co']),
        'member_count': None,
        'slots': [(0, 'm0'), (1, 'm1'), (-1, None), (3, 'm3')],
        'members': {
            'm0': {'def': 'inf', 'gtid': 0x2001, 'actor': True, 'body': True, 'speed': f32(5.0), 'experience': True,
                   'd1080': False, 'record_len': 9, 'list_2d8': [0x5501]},
            'm1': {'def': 'inf', 'gtid': 0x2002, 'actor': True, 'body': False, 'speed': f32(7.0), 'experience': False,
                   'd1080': True, 'record_len': 4, 'list_2d8': []},
            'm3': {'def': 'co', 'gtid': 0x2004, 'actor': True, 'body': True, 'speed': f32(3.0), 'experience': True,
                   'd1080': True, 'record_len': 13, 'list_2d8': [0x5503, 0x5504]},
        },
        'recruit_words': None,   # v2..6 (default: -1 per slot)
        'recruit_names': None,   # v>=7 (default: '' per slot)
        'recruit_lookup': {},    # ActorData name -> item handle (4E2A60)
        'items': {},             # item handle -> (+C, +10)
        'leader': 0,
        'morale_multiplier': f32(0.75),
        'formation_70': 1, 'formation_74': 0,
        'legacy_word': 0x11223344,
        'offsets': None,
        'words_50': None,
        'w84': f32(0.9), 'b8c': 1, 'b8d': 0,
        'damage': [0x31, 0x32, '', 0x33, 0x34, 0x35, 0x36, 0x37],
        'w88': f32(0.5), 'wb4': f32(50.0), 'bc': [f32(41.0), f32(-9.0)], 'd0': 0x4d0, 'd4': 0x4d4,
        'state_group': [(0, 1), (0, 2)],
        'modifier_results': [],
        'has_property': {},
        'cef80': True, 'd1900': 0x6601, 'ced10': 0x4ced,
        'is_actor': {},
    }


def payload(spec):
    v = spec['version']
    uv, uid = spec['updatable']
    p = W(uv) + (W(uid) if uv >= 1 else b'')
    p += W(v)
    lv, lname, lcount, lparams = spec['layout_stream']
    p += W(lv) + S(lname) + W(len(lparams) if lcount is None else lcount) + b''.join(S(x) for x in lparams)
    slots = spec['slots']
    p += W(len(slots) if spec['member_count'] is None else spec['member_count'])
    for idx, label in slots:
        p += W(idx)
        if idx >= 0:
            n = spec['members'][label]['record_len'] if label else 6
            p += bytes((0x40 + i) & 0xff for i in range(n))
    n_slots = len(slots)
    if 2 <= v <= 6:
        words = spec['recruit_words'] if spec['recruit_words'] is not None else [-1] * n_slots
        p += W(*words)
    if v >= 7:
        names = spec['recruit_names'] if spec['recruit_names'] is not None else [''] * n_slots
        p += b''.join(S(x) for x in names)
    p += W(spec['leader']) + W(spec['morale_multiplier']) + W(spec['formation_70'])
    if v >= 3:
        p += W(spec['formation_74'])
    if v == 0:
        p += W(spec['legacy_word'])
    offsets = spec['offsets'] if spec['offsets'] is not None else [[f32(i + 0.5), f32(-i - 0.25)] for i in range(n_slots)]
    for x, y in offsets:
        p += W(x, y)
    if v >= 1:
        words = spec['words_50'] if spec['words_50'] is not None else [0x500 + i for i in range(n_slots)]
        p += W(*words)
    if v >= 4:
        d = spec['damage']
        wv = spec['world_version']
        p += W(spec['w84']) + bytes([spec['b8c'], spec['b8d']]) + W(d[0], d[1]) + S(d[2]) + W(d[3], d[4])
        if wv >= 0x12:
            p += W(d[5])
        elif wv >= 0xf:
            p += bytes([d[5] & 0xff])
        if wv >= 0x14:
            p += W(d[6])
    if v >= 5:
        p += W(spec['w88'])
    if v >= 6:
        p += W(spec['wb4'])
    if v >= 8:
        p += W(*spec['bc'])
    if v >= 9:
        p += W(spec['d0'])
    if v >= 10:
        p += W(spec['d4'])
    return p + spec.get('extra', b'')


class OrganizationMachine(Machine):
    def __init__(self, spec, data):
        super().__init__(data, spec['precision'])
        self.spec = spec
        o = spec['owner']
        world = self.alloc(0x400)
        self.w(0xb36534, world)
        self.w(world + 0x2c, spec['world_version'])
        self.w(world + 0xe0, spec['time_e0'])
        self.w(world + 0xe4, spec['time_e4'])
        table = self.alloc(16)
        slots = self.alloc(4 * (spec['table']['max'] + 2))
        for i in range(spec['table']['max'] + 2):
            self.w(slots + 4 * i, 0xdead0000 + i if i in spec['table']['occupied'] else 0)
        self.w(table, spec['table']['max'])
        self.w(table + 4, slots)
        self.w(world + 0x284, table)
        self.table_slots = slots
        for g in (0xb3ab00, 0xb39f58, 0xb3aaf8):
            self.w(g, self.alloc(0x40))
        self.wb(0xb3cc47, 0)
        # Properties, ActorData, LayoutData, formations, damage types
        self.props = {}
        for name in PROPERTIES:
            pr = self.alloc(0x40)
            self.w(pr + 8, self.cstring(name))
            self.props[name] = pr
            self.labels[pr] = 'property:' + name
        self.defs = {}
        for name, (flags, morale) in ACTOR_DATA.items():
            d = self.alloc(0x600)
            self.w(d + 8, self.cstring(name))
            self.w(d + 0x174, flags)
            self.w(d + 0x2c0, morale)
            self.defs[name] = d
            self.labels[d] = 'actor:' + name
        self.formations = []
        for byte_1a, prop in FORMATION_DEFS:
            fd = self.alloc(0x40)
            self.wb(fd + 0x1a, byte_1a)
            self.w(fd + 0x24, prop)
            f = self.alloc(0x20)
            self.w(f + 0xc, fd)
            self.labels[f] = 'formation:%d' % len(self.formations)
            self.formations.append(f)
        self.layouts = {}
        for name, (entries, groups, primary, nform, f44, f48, prop50) in LAYOUTS.items():
            d = self.alloc(0x80)
            self.w(d + 8, self.cstring(name))
            self.w(d + 0x18, primary)
            e = self.alloc(16 * len(entries))
            for i, (prop, req, locked, morale) in enumerate(entries):
                self.w(e + 16 * i, self.cstring(prop))
                self.w(e + 16 * i + 4, self.props[prop])
                self.w(e + 16 * i + 8, 0)
                self.u.mem_write(e + 16 * i + 0xc, bytes([req, locked, morale, 0]))
            self.w(d + 0x1c, e)
            self.w(d + 0x20, len(entries))
            g = self.alloc(4 * max(1, len(groups)))
            for i, x in enumerate(groups):
                self.w(g + 4 * i, x)
            self.w(d + 0x28, g)
            self.w(d + 0x2c, len(groups))
            fa = self.alloc(4 * max(1, nform))
            for i in range(nform):
                self.w(fa + 4 * i, self.formations[i])
            self.w(d + 0x34, fa)
            self.w(d + 0x38, nform)
            self.w(d + 0x44, f44)
            self.w(d + 0x48, f48)
            self.w(d + 0x50, prop50)
            self.layouts[name] = d
            self.labels[d] = 'layout:' + name
        self.damage = {}
        for name in DAMAGE_TYPES:
            dt = self.alloc(0x20)
            self.damage[name] = dt
            self.labels[dt] = 'damage:' + name
        # default layout object at owner ActorData+2F0
        dname, dparams = spec['default_layout']
        dl = self.alloc(0x18)
        self.w(dl, 0xa232b8)
        self.w(dl + 4, 1)
        self.w(dl + 8, self.layouts[dname])
        pa = self.alloc(4 * max(1, len(dparams)))
        for i, x in enumerate(dparams):
            self.w(pa + 4 * i, self.defs[x] if x else 0)
        self.w(dl + 0xc, pa)
        self.w(dl + 0x10, len(dparams))
        self.w(dl + 0x14, len(dparams))
        od = self.defs[o['def']]
        self.w(od + 0x2f0, dl)
        self.w(od + 0x308, o['breakpoint_308'])
        self.w(od + 0x174, o['flags174'])
        # owner actor
        self.kingdom = 0
        if o['kingdom'] is not None:
            k = self.alloc(0x400)
            singles = o['kingdom']['singles']
            arr = self.alloc(4 * max(1, len(singles)))
            for i, h in enumerate(singles):
                self.w(arr + 4 * i, self.item(h) if h else 0)
            self.w(k + 0x1fc + 0x18, arr)
            self.w(k + 0x1fc + 0x1c, len(singles))
            self.kingdom = k
            self.labels[k] = 'kingdom'

        def kingdom_virtual(m):
            m.record('owner+108')
            m.ret(0, m.kingdom)
        self.owner = self.alloc(0x200)
        self.labels[self.owner] = 'owner'
        self.w(self.owner, self.vtable({0x108: kingdom_virtual}))
        self.w(self.owner + 4, od)
        self.w(self.owner + 0x14, o['gtid'])
        self.w(self.owner + 0x20, o['position'][0])
        self.w(self.owner + 0x24, o['position'][1])
        if o['modifier'] is not None:
            props = self.alloc(0x40)
            self.w(self.owner + 0x78, props)
            self.labels[props + 0x1c] = 'modifier:owner'
        if o['objects_10'] is not None:
            objs = self.alloc(0x40)
            self.w(objs + 0x10, o['objects_10'])
            self.w(self.owner + 0x148, objs)
            self.w(self.owner + 0x14c, 5)
        else:
            self.w(self.owner + 0x14c, 4)
        self.mind = 0
        if o['mind']:
            def mind118(m):
                m.record('Mind+118', old=m.arg(0), new=m.arg(1))
                m.ret(8)
            self.mind = self.alloc(0x40)
            self.labels[self.mind] = 'owner.Mind'
            self.w(self.mind, self.vtable({0x118: mind118}))
            self.w(self.owner + 0x70, self.mind)
        # members
        self.members = {}
        for label, ms in spec['members'].items():
            a = self.alloc(0x200)
            self.labels[a] = label

            def is_actor(m, label=label):
                value = m.spec['members'][label]['actor']
                m.record('IsGActor', member=label, result=value)
                m.ret(0, int(value))
            self.w(a, self.vtable({0xb4: is_actor}))
            self.w(a + 4, self.defs[ms['def']])
            self.w(a + 0x14, ms['gtid'])
            if ms['body']:
                body = self.alloc(0x40)
                self.labels[body] = label + '.Body'
                self.w(a + 0x60, body)
            if ms['speed'] is not None:
                mover = self.alloc(0x40)
                self.w(mover + 0x28, ms['speed'])
                self.w(a + 0x6c, mover)
            el = self.alloc(0x40)
            self.labels[el] = label + '.Element'
            self.w(a + 0x80, el)
            if ms['experience']:
                self.w(a + 0xa4, self.alloc(0x20))
            self.members[label] = a
        # 45EC60 lists on member ActorData +2D8 (definition data; last writer wins per def)
        for label, ms in spec['members'].items():
            d = self.defs[ms['def']]
            head = 0
            for value in reversed(ms['list_2d8']):
                n = self.alloc(8)
                self.w(n, value)
                self.w(n + 4, head)
                head = n
            self.w(d + 0x2d8, head)
        self.pending_records = [label for idx, label in spec['slots'] if idx >= 0]
        self.state_group = list(spec['state_group'])
        self.modifier_results = list(spec['modifier_results'])
        self._seams()

    def item(self, h):
        if not hasattr(self, 'items'):
            self.items = {}
        if h not in self.items:
            it = self.alloc(0x20)
            c, ten = self.spec['items'].get(h, (0, 0))
            self.w(it + 0xc, c)
            self.w(it + 0x10, ten)
            self.items[h] = it
            self.labels[it] = 'item:%#x' % h
        return self.items[h]

    def label(self, p):
        if p == 0:
            return None
        return self.labels.get(p, '%#x' % p)

    def _seams(self):
        m = self
        ecx = lambda: m.u.reg_read(UC_X86_REG_ECX)

        def lookup(table, kind, pop):
            def h(m):
                name = m.cstring_text(m.arg(0))
                p = table.get(name, 0)
                m.record(kind, ids=name, found=bool(p))
                m.ret(pop, p)
            return h
        m.seam(0x463fa0, lookup(m.layouts, '463FA0', 4))
        m.seam(0x4bdba0, lookup(m.defs, '4BDBA0', 8))
        m.seam(0x48d500, lookup(m.damage, '48D500', 4))

        def nested(m):  # 4FAE80(ctx, &out, owner, flag) thiscall on GWorld
            assert m.arg(0) == m.ctx and m.arg(2) == m.owner and m.arg(3) == 0
            label = m.pending_records.pop(0)
            n = m.spec['members'][label]['record_len'] if label else 6
            m.take(n)
            if label == 'null':
                label = None
            m.record('4FAE80', member=label, bytes=n)
            m.w(m.arg(1), m.members[label] if label else 0)
            m.ret(0x10, m.members[label] if label else 0)
        m.seam(0x4fae80, nested)
        m.seam(0x855d30, lambda m: (m.record('855D30'), m.ret(0, 0)))

        def simple(name, pop=0, result=None, effect=None, args=()):
            def h(m):
                e = m.record(name, this=m.label(ecx()))
                for i, a in enumerate(args):
                    v = m.arg(i)
                    e[a] = m.label(v) if a in ('member', 'actor', 'owner', 'object', 'item') else                         (v & 0xff if a == 'enabled' else v)  # bool argument: low byte only
                if effect:
                    effect(m)
                r = result(m) if callable(result) else result
                if r is not None:
                    e['result'] = r if not isinstance(r, bool) else r
                m.ret(pop, int(r) if r is not None else None)
            return h
        m.seam(0x5705a0, simple('5705A0', 8, args=('owner', 'slot')))
        m.seam(0x561760, simple('561760', 4, args=('owner',)))
        m.seam(0x4ced10, simple('4CED10', 0, result=lambda m: m.spec['ced10']))
        m.seam(0x4cebb0, simple('4CEBB0', 8, args=('zero', 'token')))
        m.seam(0x5788b0, simple('5788B0', 0))
        m.seam(0x4d1080, simple('4D1080', 0, result=lambda m: m.spec['members'][m.label(ecx())]['d1080']))
        m.seam(0x4d64f0, simple('4D64F0', 8, args=('state', 'enabled')))

        def leader_effect(m):
            m.w(ecx() + 0x40, m.arg(0))
        m.seam(0x579a00, simple('579A00', 4, effect=leader_effect, args=('member',)))
        m.seam(0x524b80, simple('524B80', 4, args=('member',)))
        m.seam(0x57c800, simple('57C800', 4, args=('member',)))
        m.seam(0x4d4cf0, simple('4D4CF0', 0))
        m.seam(0x4cf0f0, simple('4CF0F0', 0))
        m.seam(0x4d1350, simple('4D1350', 0))
        m.seam(0x5784c0, simple('5784C0', 4, args=('member',)))
        m.seam(0x4cef10, simple('4CEF10', 0xc, args=('property', 'kind', 'actor')))
        m.seam(0x4cef70, simple('4CEF70', 8, args=('property', 'actor')))
        m.seam(0x4cef80, simple('4CEF80', 8, result=lambda m: m.spec['cef80'], args=('property', 'actor')))
        m.seam(0x5147c0, simple('5147C0', 0))
        m.seam(0x57bef0, simple('57BEF0', 0))
        m.seam(0x4d1900, simple('4D1900', 0, result=lambda m: m.spec['d1900']))
        m.seam(0x4b2830, simple('4B2830', 0))

        def has_property(m):  # 45EEF0(Property) thiscall on ActorData
            actor, prop = m.label(ecx()), m.label(m.arg(0))
            r = m.spec['has_property'].get('%s/%s' % (actor, prop), True)
            m.record('45EEF0', actor=actor, property=prop, result=r)
            m.ret(4, int(r))
        m.seam(0x45eef0, has_property)

        def recruit_find(m):  # 4E2A60(ActorData, 0) thiscall on Kingdom+1FC
            if m.kingdom == 0:
                # 4E2A69 divides by [ecx+0C] = [0x208]: native access violation.
                m.outcome = {'kind': 'NullDereference', 'pc': '0x4e2a69'}
                raise Throw()
            assert ecx() == m.kingdom + 0x1fc and m.arg(1) == 0
            name = m.label(m.arg(0))[len('actor:'):]
            h = m.spec['recruit_lookup'].get(name, 0)
            m.record('4E2A60', actor=name, result=h)
            m.ret(8, m.item(h) if h else 0)
        m.seam(0x4e2a60, recruit_find)
        m.seam(0x4e1200, simple('4E1200', 4, args=('member',)))

        def group(m):  # 4D49B0(group) thiscall on owner
            assert ecx() == m.owner
            g = m.arg(0)
            r = m.state_group.pop(0) if m.state_group else None
            m.record('4D49B0', group=g, result=r)
            if r is None:
                m.ret(4, 0)
                return
            d = m.alloc(0x30)
            m.w(d + 0x20, r[0])
            m.w(d + 0x24, r[1])
            m.ret(4, d)
        m.seam(0x4d49b0, group)

        def modifier(m):  # 501C90(value) thiscall on Modifier, result ST0
            value = m.arg(0)
            r = m.modifier_results.pop(0)
            m.record('501C90', modifier=m.label(ecx()), value=value, result_f64=r)
            m.fld_stub(r)
        m.seam(0x501c90, modifier)

    # ------------------------------------------------------------- results
    def vec(self, base, stride=4):
        ptr, count = self.r(base), self.r(base + 4)
        n = count if count < 0x80000000 else 0
        return count, [self.r(ptr + stride * i) if stride == 4 else [self.r(ptr + 8 * i), self.r(ptr + 8 * i + 4)]
                       for i in range(n)]

    def state(self, o):
        out = {}
        words = {}
        for off in [0x40, 0x44, 0x70, 0x74, 0x98]:
            words['%#x' % off] = self.label(self.r(o + off))
        for off in [0x48, 0x5c, 0x60, 0x64, 0x68, 0x6c, 0x84, 0x88, 0x90, 0x94, 0x9c, 0xa0, 0xa4, 0xa8, 0xac,
                    0xb4, 0xb8, 0xbc, 0xc0, 0xc4, 0xc8, 0xcc, 0xd0, 0xd4, 0xd8]:
            words['%#x' % off] = self.r(o + off)
        out['words'] = words
        out['bytes'] = {'%#x' % off: self.b(o + off) for off in [0x4c, 0x4d, 0x8c, 0x8d, 0x8e, 0xb0]}
        out['scheduled_head'] = self.r(o + 0xc)
        out['layout_definition'] = self.label(self.r(o + 0x18))
        n, params = self.vec(o + 0x1c)
        out['layout_parameters'] = [self.label(x) for x in params]
        n, members = self.vec(o + 0x28)
        out['member_count'] = n
        out['members'] = [self.label(x) for x in members]
        n, items = self.vec(o + 0x34)
        out['recruit_items'] = [self.label(x) for x in items]
        out['recruit_count'] = n
        out['words_50'] = self.vec(o + 0x50)[1]
        out['offsets_78'] = self.vec(o + 0x78, 8)[1]
        return out

    def run(self):
        o = self.alloc(0xdc, 0xcd)
        self.org = o
        self.labels[o] = 'org'
        self.call(0x579bd0, o, [self.owner])
        after_ctor = self.state(o)
        self.call(0x57d130, o, [self.ctx])
        lists = {}
        for name, d in self.defs.items():
            values, node = [], self.r(d + 0x2d8)
            while node:
                values.append(self.r(node))
                node = self.r(node + 4)
            lists[name] = values
        out = {'after_ctor': after_ctor, 'consumed': self.at, 'reads': self.reads, 'calls': self.calls,
               'outcome': self.outcome, 'definition_lists_2d8': lists}
        if self.outcome is None or self.outcome['kind'] != 'NullDereference':
            out['state'] = self.state(o)
            uid = self.spec['updatable'][1]
            if 0 <= uid <= self.spec['table']['max'] + 1:
                v = self.r(self.table_slots + 4 * uid)
                out['table_slot'] = 'org+8' if v == o + 8 else v
        return out


def variants():
    rows = []

    def add(name, **changes):
        s = base_spec()
        for k, v in changes.items():
            if k == 'owner':
                s['owner'].update(v)
            elif k == 'members':
                for label, mv in v.items():
                    s['members'].setdefault(label, {}).update(mv)
            else:
                s[k] = v
        rows.append((name, s))

    for pc in ['Single24', 'Double53', 'Extended64']:
        for version in range(11):
            add('version%d/%s' % (version, pc), version=version, precision=pc,
                recruit_words=[0, -1, 2, 7] if 2 <= version <= 6 else None)
    for wv in [0xe, 0xf, 0x11, 0x12, 0x13, 0x14, 21, 28]:
        add('world_version_%d' % wv, world_version=wv, damage=[1, 2, 'dt_shell', 4, 5, 0x1ff, 7, 8])
    # updatable
    add('updatable_v0', updatable=(0, 0))
    add('updatable_negative', updatable=(1, -5))
    add('updatable_duplicate', updatable=(1, 3))
    add('updatable_above_table', updatable=(1, 51))
    add('updatable_at_table_max', updatable=(1, 50))
    add('updatable_bad_version', updatable=(2, 0))
    add('bad_version', version=11)
    # layout stream
    add('layout_version1', layout_stream=(1, 'lay3', None, []))
    add('layout_unknown', layout_stream=(0, 'nope', None, []))
    add('layout_empty_name', layout_stream=(0, '', None, []))
    add('layout_count_mismatch', layout_stream=(0, 'lay3', None, ['inf', 'mg']))
    add('layout_unknown_param', layout_stream=(0, 'lay3', None, ['inf', 'zzz', 'co']))
    add('layout_null_param_unrequired', layout_stream=(0, 'lay3', None, ['inf', '', 'co']))
    add('layout_other_same_count', layout_stream=(0, 'lay3b', None, ['inf', 'mg', 'co']))
    add('layout_other_count', layout_stream=(0, 'lay2', None, ['inf', 'mg']))
    add('verify_locked_changed', layout_stream=(0, 'lay3', None, ['inf', 'mg', 'spare']))
    add('verify_required_missing', layout_stream=(0, 'lay3', None, ['', 'mg', 'co']))
    add('verify_required_missing_locked_null', default_layout=('lay3', ['inf', 'mg', None]),
        layout_stream=(0, 'lay3', None, ['inf', 'mg', '']))
    add('verify_default_removed', default_layout=('lay3', ['inf', 'mg', 'co']),
        layout_stream=(0, 'lay3', None, ['inf', '', 'co']))
    add('verify_both_null', default_layout=('lay3', ['inf', None, 'co']),
        layout_stream=(0, 'lay3', None, ['inf', '', 'co']))
    add('verify_missing_property', has_property={'actor:mg/property:prop_mg': False})
    add('verify_other_param', layout_stream=(0, 'lay3', None, ['inf', 'spare', 'co']))
    # member records
    add('member_count_word', member_count=5)
    add('member_load_null', slots=[(0, 'm0'), (1, 'null'), (-1, None), (3, 'm3')],
        members={'null': {'def': 'inf', 'gtid': 0, 'actor': True, 'body': False, 'speed': None,
                          'experience': True, 'd1080': False, 'record_len': 5, 'list_2d8': []}})
    add('member_not_actor', members={'m1': {'actor': False}})
    add('member_no_element', members={'m1': {'def': 'cart'}})
    add('member_slots_permuted', slots=[(3, 'm3'), (0, 'm0'), (-1, None), (1, 'm1')], leader=3)
    add('member_all_empty', slots=[(-1, None)] * 4, leader=-1)
    add('member_duplicate_slot', slots=[(0, 'm0'), (0, 'm1'), (-1, None), (3, 'm3')])
    add('member_speeds_nan_equal', members={'m0': {'speed': 0x7fc00000}, 'm1': {'speed': f32(3.0)},
                                            'm3': {'speed': f32(3.0)}})
    add('member_no_movers', members={'m0': {'speed': None}, 'm1': {'speed': None}, 'm3': {'speed': None}})
    add('owner_without_mind', owner={'mind': False})
    add('owner_objects_10', owner={'objects_10': 0x5150})
    add('owner_modifier', owner={'modifier': 1}, modifier_results=[f64(2.3 / 3)])
    add('owner_modifier_big', owner={'modifier': 1}, modifier_results=[f64(1e6 / 7)], morale_multiplier=f32(1 / 3))
    # recruit items, versions 2..6
    for w in ([0, 1, 2, 3], [-1, -1, -1, -1], [5, 3, 1, 0x7fffffff]):
        add('recruit_words_%s' % '_'.join(map(str, w)), version=6, recruit_words=w)
    add('recruit_words_no_kingdom', version=6, recruit_words=[0, -1, -1, -1], owner={'kingdom': None})
    add('recruit_words_no_kingdom_negative', version=6, recruit_words=[-1, -1, -1, -1], owner={'kingdom': None})
    # recruit items, version >= 7
    paired = {'items': {0x9a01: (0x2001, 0x10a0), 0x9a03: (0x2004, 0x10a0), 0x9a02: (0, 0x10a0)},
              'recruit_lookup': {'inf': 0x9a01, 'co': 0x9a03, 'mg': 0x9a02}}
    add('recruit_names_paired', recruit_names=['inf', '', '', 'co'], **paired)
    add('recruit_names_null_member', recruit_names=['', '', 'mg', ''], **paired)
    add('recruit_names_unknown', recruit_names=['inf', 'zzz', '', ''], **paired)
    add('recruit_names_not_found', recruit_names=['', '', '', 'spare'], **paired)
    add('recruit_names_not_paired', recruit_names=['', 'mg', '', ''], **paired)
    add('recruit_names_wrong_actor', recruit_names=['co', '', '', ''], **paired)
    add('recruit_names_wrong_org', recruit_names=['inf', '', '', ''],
        items={0x9a01: (0x2001, 0x10a1)}, recruit_lookup={'inf': 0x9a01})
    add('recruit_names_no_kingdom', recruit_names=['inf', '', '', ''], owner={'kingdom': None}, **paired)
    # leader, formations, timers
    for leader in (-1, 2, 3, 4, 0x7fffffff, -0x80000000):
        add('leader_%d' % leader, leader=leader)
    for f70, f74 in ((-1, -1), (0, 1), (1, 0), (2, 0), (5, 0)):
        add('formation_%d_%d' % (f70, f74), formation_70=f70, formation_74=f74)
    add('formation_v2', version=2, formation_70=0)
    for sg in ([None], [(0, 0), (0, 0)], [(1, 2), None], [(2, 2), (2, 1)], [(3, 0), (3, 3)], [(4, 1), (7, 2)],
               [(0, 1), (0, 0)]):
        add('state_group_%s' % '_'.join(map(str, sg)).replace(' ', ''), state_group=sg)
    add('state_group_cef80_false', state_group=[(2, 1), (2, 1)], cef80=False, d1900=0)
    add('state_group_formation_null', formation_70=-1, state_group=[(0, 0), (0, 2)], cef80=False)
    add('state_group_formation_null_property', formation_70=-1, state_group=[(0, 0), (0, 2)],
        default_layout=('lay2', ['inf', 'mg']), layout_stream=(0, 'lay2', None, ['inf', 'mg']),
        slots=[(0, 'm0'), (1, 'm1')], formation_74=-1)
    for b4 in (f32(100.0), f32(100.5), f32(-1.0), 0x7fc00000, 0x80000000, f32(1e30)):
        add('timer_b4_%08x' % b4, wb4=b4)
    add('timer_v5_ctor_time', version=5, time_e0=f32(150.0))
    add('damage_type_unknown', damage=[1, 2, 'zz', 4, 5, 6, 7])
    add('damage_type_known', damage=[1, 2, 'dt_bullet', 4, 5, 6, 7])
    for mult in (f32(1 / 3), f32(0.0), 0x80000000, f32(-2.5), f32(1e-30), f32(3e38)):
        add('morale_mult_%08x' % mult, morale_multiplier=mult)
    # Subnormal 59E300 products: the only place where PC24 double rounding
    # (fmul at 24 bits, then the binary32 subnormal store) differs.
    for pc in ['Single24', 'Double53', 'Extended64']:
        for mult in (0x00000003, 0x00012345, 0x007fffff, 0x00400001, 0x00000001, 0x80000007,
                     0x0000ca46, 0x00013522, 0x80013cc4):
            add('morale_subnormal_%08x/%s' % (mult, pc), precision=pc, morale_multiplier=mult)
    add('one_slot_layout', default_layout=('lay1', ['spare']), layout_stream=(0, 'lay1', None, ['inf']),
        slots=[(0, 'm0'), (1, 'm1')], leader=1, version=7)
    return rows


def main():
    out = []
    for name, spec in variants():
        data = payload(spec)
        m = OrganizationMachine(spec, data)
        native = m.run()
        row = copy.deepcopy(spec)
        out.append({'name': name, 'input': row, 'payload': data.hex(), 'native': native})
    # truncation sweep over one complete version-10 payload
    spec = base_spec()
    spec['recruit_names'] = ['inf', '', '', 'co']
    spec['items'] = {0x9a01: (0x2001, 0x10a0), 0x9a03: (0x2004, 0x10a0)}
    spec['recruit_lookup'] = {'inf': 0x9a01, 'co': 0x9a03}
    spec['damage'] = [1, 2, 'dt_bullet', 4, 5, 6, 7]
    full = payload(spec)
    for cut in range(0, len(full) + 1, 5):
        m = OrganizationMachine(spec, full[:cut])
        out.append({'name': 'truncated_%d' % cut, 'input': copy.deepcopy(spec), 'payload': full[:cut].hex(),
                    'native': m.run()})
    for a, b in VISITED.items():
        assert bytes.fromhex(b) == P.range(a, len(bytes.fromhex(b))) == V.range(a, len(bytes.fromhex(b)))
    doc = {'schema': 1, 'passed': True, 'producer_sha256': H(Path(__file__).read_bytes()),
           'machine_sha256': H((HERE / 'native_machine.py').read_bytes()),
           'original_sha256': H(P.b), 'v14_sha256': H(V.b), 'scope': __doc__, 'seams': SEAM_DOC,
           'definitions': {'actor_data': {k: list(v) for k, v in ACTOR_DATA.items()}, 'properties': PROPERTIES,
                           'layouts': {k: {'entries': v[0], 'groups': v[1], 'primary': v[2], 'formations': v[3],
                                           'formation_44': v[4], 'formation_48': v[5], 'property_50': v[6]}
                                       for k, v in LAYOUTS.items()},
                           'formations': FORMATION_DEFS, 'damage_types': DAMAGE_TYPES},
           'visited_instructions': len(VISITED), 'cases': out}
    path = HERE / 'native-organization.json'
    path.write_text(json.dumps(doc, indent=1, default=lambda x: x.hex() if isinstance(x, bytes) else x))
    kinds = {}
    for row in out:
        k = (row['native']['outcome'] or {}).get('kind', 'ok')
        kinds[k] = kinds.get(k, 0) + 1
    print(json.dumps({'cases': len(out), 'outcomes': kinds, 'sha256': H(path.read_bytes())}))


if __name__ == '__main__':
    main()
