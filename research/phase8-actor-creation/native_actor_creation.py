"""Fresh ACTOR creation oracle: the ORIGINAL KKC_GWorld::CreateGThing 4F9C10
on an A25B28 actor creation block (class 3), run natively through
  4D6BB0 (GActor ctor over 4EA5E0/4EF2A0, 4D4A10 nonblocking),
  4F9C10's actor steps (mode 1/3, +50, Organization layout),
  vf E4 = 4D6F10 InitCreate (4E9FC0, 4D21A0 assembly with the 4CF480 factory
          and 4CFB00 requirement validation, 4F4A20 / numbering, Character
          start / 4CFC80, trailer creation through 4FA030 (recursive), the
          component InitCreate loop, construction registration),
  vf E8 = 4D7230 InitExisting (4EBE50, 4D1FD0 required object, +138, the
          component InitExisting / vf+14 loops, +158 vf 128, the component
          refresh calls, settlement zone 4D69A0, SAI, TrackInfo updatable,
          creation FX),
  vf EC = 4CF300 PostCreate (4EBA40, component PostCreate, Kingdom add,
          city creation),
  4F9510 (actor branch 4D62F0, 4F4C40), 50C220, B0 -> 42A590,
  4F5490 on failure;
and the entry points 4FA030 (CreateGActor) and 4FA150 (CreateOrganization).

Component constructors are seams (recorded; they build fixture components
whose virtuals +4 InitCreate, +8 InitExisting, +C PostCreate and +14 are
recorded stubs with per-case results; +20 is the REAL requirement virtual of
that class). Every other seam is listed in SEAM_DOC. Usage:
  python native_actor_creation.py           -> native-actor-creation.json
"""
import json, struct, sys, time, copy
from pathlib import Path
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from actor_machine import (ActorMachine, P, V, VISITED, NATIVE, CTORS, CTOR_KIND, REQUIREMENT, SLOT,
                           DIRECTIONS, PCS, SHAPE_A, SHAPE_B, ACTORDATA_VT, H, ROOT, f32, nm, Stop)
from unicorn.x86_const import *

SEAM_DOC = {
    '6F7810': 'operator new (bump allocator, 0xA5 fill); 6F7430 delete (recorded)',
    '856370': 'CString allocator object',
    'ctor': '4CF480 component constructors (34): record the kind, build a fixture component '
            '(+0 vtable, +4 actor); vtable +4/+8/+C/+14 are recorded stubs returning the case '
            'result, +20 is the class\'s native requirement virtual',
    '4D1050': 'Structure requirement virtual (589090) callee',
    '4D49B0': 'actor state group lookup (aa_actors::state::group) -> case value',
    '4D64F0': 'actor state set (aa_actors::state::set): records (state, enabled) and applies the bit',
    '4D0720': 'Character view lookup (45E5E0/45E690) -> fixture view {+48 first animation, +4C count}',
    '563D50': 'Character start animation (on the Character component)',
    '563BA0': 'Character default stance (on the Character component, 4CFC80 tail)',
    '4D7F40': 'actor SetName (map-string name from the 4F4A20 name list entry)',
    '4E4980': 'Kingdom numbered-name counter (+35C) and 4CF1A0 (+170 = number)',
    '562200': 'trailer Body health copy (on the child Body)',
    '54BFC0': 'construction manager [B36520] registration (owner, definition)',
    '57C450': 'Organization component layout (+2C)',
    '855D30': 'assertion dialog (latches B3CC40/42/43/44)',
    '4154D0/5C9180/5043F0/4F7B70': 'instance-name map (non-empty names only)',
    '43B7D0/43D160': 'presentation scene transform / add',
    '553830/5536B0/5539B0': 'spatial index remove / insert (flag bit 26) / move',
    '46EE00/4F4590/4F0E50': 'snap, placement test, terrain height',
    '554350/555620/4EA150/4EBC70/4EB500': 'blocking query, tile footprint, visibility, ground paint',
    '555830/46B000': 'required-object search (GMap) / required-object placement test',
    '4D4CF0/4D1350/4D67F0/4D4130': 'actor refresh calls of 4D7230 (organization, detection memory, ...)',
    '5035E0/5817A0': 'global property test / Properties add (4D7230)',
    '5788B0/585650/57E430/569E40': 'Organization / Settlement / Performer / Denizen refresh (4D7230)',
    '514CD0/516640/515F40': 'zone allowed test / zone ctor / zone placement (4D69A0, 4D7230)',
    '7A0C20': 'SAI notification ([B36528])',
    '50AB10': 'TrackInfo updatable constructor (+144)',
    '429080': 'creation FX of a definition action list (42A590)',
    '4E6180': 'Kingdom add_actor (aa_economy::kingdom_registry::add_actor)',
    '5644F0': 'city creation (4CF300)',
    '4D4A90': 'actor state re-entry on registration (4D62F0)',
    '4CFEA0': 'spatial grid registration (4D62F0 tail)',
    '4F5490': 'destroy a created object whose initialization failed',
    '404ED0/404EA0/404F60': 'debug logs',
    '57AD30/57A730/4C2BC0/578290': 'CreateOrganization member spawn / count / first slot / slot test',
    '77DC50/77DA70': 'profiler scope / RNG sync log (no state)',
}

VSTUBS = {0x4: ('InitCreate', 4, 1), 0x8: ('InitExisting', 0, 0), 0xc: ('PostCreate', 8, 2), 0x14: ('vf14', 0, 0)}


def kname(kind):
    return str(kind)


def build(spec):
    m = ActorMachine(spec['pc'])
    g, gd = m.gworld, m.gamedata
    m.spec = spec
    m.w(g + 0xd4, spec['rng'])
    m.w(g + 0xd8, spec.get('world_d8', 0))
    m.w(g + 0xe0, spec['begin'])
    m.w(g + 0x2c, spec.get('world_version', 0x1a))
    gmap = m.alloc(0x100, 0)
    m.w(gmap + 4, spec['tile_shift'])
    m.w(gmap + 8, spec['tile_size'])
    m.w(g + 0x30, gmap)
    m.gmap = gmap
    m.labels[gmap] = 'gmap'
    m.labels[g] = 'gworld'
    settings = m.alloc(0x100, 0)
    m.w(settings + 0xa8, spec['map_mode'])
    m.w(g + 0x28, settings)
    scene = m.alloc(0x40, 0)
    m.w(scene + 4, 0x5ce4e if spec.get('scene', True) else 0)
    m.w(g + 0x130, scene)
    m.labels[scene] = 'scene'
    tab = m.alloc(2048 * 8)
    i = PCS[spec['direction_table_pc']]
    m.u.mem_write(tab, DIRECTIONS[i * 2048 * 8:(i + 1) * 2048 * 8])
    dobj = m.alloc(16, 0)
    m.w(dobj + 4, 2048)
    m.w(dobj + 8, tab)
    m.w(0xb39af8, dobj)
    idm = spec['idmgr']
    m.w(m.idmgr + 0x60004, idm['count'])
    m.w(m.idmgr + 0x60008, idm['cursor'])
    m.w(m.idmgr + 0x6000c, idm['highest'])
    for slot, gen, live in idm['slots']:
        m.set_generation(slot, gen)
        if live:
            m.w(m.idmgr + 0x20004 + 4 * slot, 0x77770000 + slot)
    # game data [B36530]: registered actor states (4D62F0), global property (4D7230)
    states = m.alloc(4 * 64, 0)
    for k in range(64):
        m.w(states + 4 * k, 0x5e000000 + k)
    m.w(gd + 0x2c0 + 0x24, states)
    m.w(gd + 0x2e4, states)
    m.w(gd + 0x2e8, spec.get('registered_states', 0))
    holder = m.alloc(0x40, 0)
    cell = m.alloc(8, 0)
    m.w(cell, spec.get('global_property', 0))
    m.w(holder + 0x24, cell)
    m.w(gd + 0x464, holder)
    # [B39AB0] settings (TrackInfo period), [B39F58] Thing manager (+74)
    s39 = m.alloc(0x40, 0)
    f1c = m.alloc(4, 0)
    i28 = m.alloc(4, 0)
    m.w(f1c, spec.get('track_scale', f32(10.0)))
    m.w(i28, spec.get('track_divisor', 4))
    m.w(s39 + 0x1c, f1c)
    m.w(s39 + 0x28, i28)
    m.w(0xb39ab0, s39)
    tm = m.alloc(0x100, 0)
    m.w(tm + 0x74, spec.get('thing_manager_74', f32(1.5)))
    m.w(0xb39f58, tm)
    ui = 0
    if spec.get('sai'):
        ui = m.alloc(0x40, 0)
        m.wb(ui + 0x20, spec.get('sai_watching', 0))
        m.labels[ui] = 'ui'
    m.w(0xb36528, ui)
    # GWorld+268 name-assignment vector {data, count +26C, capacity +270}
    pre = spec.get('name_vector', [])
    vec = m.alloc(4 * max(1, len(pre)), 0)
    m.pre_names = pre
    m.w(g + 0x268, vec if pre else 0)
    m.w(g + 0x26c, len(pre))
    m.w(g + 0x270, len(pre))
    # CRT SSE2 availability flag (set by the CRT startup on every SSE2 CPU):
    # floor 96D5E0 takes its SSE2 path (MXCSR default 0x1F80).
    m.w(0xb4b340, 1)
    m.u.reg_write(UC_X86_REG_MXCSR, 0x1f80)
    m.w(0xb36520, 0xc0de0520)
    for latch in (0xb3cc40, 0xb3cc42, 0xb3cc43, 0xb3cc44):
        m.u.mem_write(latch, bytes([spec.get('latches', {}).get(hex(latch), 0)]))
    # definitions
    m.defs = {}
    m.defn = None
    for name, d in spec['definitions'].items():
        m.defs[name] = make_definition(m, name, d)
    for name, d in spec['definitions'].items():
        link_definition(m, name, d)
    m.defn = m.defs[spec['definition']]
    # owner kingdom, required objects
    m.owner = 0
    if spec.get('owner'):
        m.owner = m.alloc(0x400, 0)
        m.labels[m.owner] = 'kingdom'
    for gtid, o in spec.get('objects', {}).items():
        gtid = int(gtid, 16)
        obj = m.alloc(0x200, 0) if o.get('live', True) else 0
        if obj:
            vt = m.alloc(0x200, 0)
            m.w(obj, vt)
            m.w(vt + 0x128, m.stub(lambda m, gtid=gtid: (m.record('object vf128', object=hex(gtid),
                                                                   actor=m.label(m.arg(0))), m.ret(4))))
            od = m.alloc(0x40, 0)
            ovt = m.alloc(0x40, 0)
            m.w(ovt + 0x24, 0x462e50)
            m.w(od, ovt)
            m.wb(od + 0x34, o.get('settlement', 0))
            m.w(obj + 4, od)
            m.w(obj + 0x20, o.get('x', f32(1.0)))
            m.w(obj + 0x24, o.get('y', f32(2.0)))
            m.labels[obj] = 'object %s' % hex(gtid)
        m.register(gtid, obj)
        if not obj:
            m.set_generation(gtid & 0xffff, o.get('generation', (gtid >> 16) + 1))
    for k, (dname, index) in enumerate(m.pre_names):
        pair = m.alloc(8, 0)
        m.w(pair, m.r(m.defs[dname] + 0x1ac))
        m.w(pair + 4, index)
        m.w(m.r(g + 0x268) + 4 * k, pair)
    m.params = make_params(m, spec['params'], m.defn)
    install_seams(m, spec)
    for name, (b, size) in {'gworld': (g, 0x800), 'gmap': (gmap, 0x100), 'definition': (m.defn, 0x600),
                            'params': (m.params, 0x54)}.items():
        m.probe(name, b, size)
    return m


def make_definition(m, name, d):
    a = m.alloc(0x600, 0)
    m.labels[a] = 'def:' + name
    m.w(a, ACTORDATA_VT)
    m.w(a + 8, m.cstring(name))
    m.w(a + 0x10, m.cstring('display_' + name))
    cat = m.alloc(0x40, 0)
    m.w(cat + 0x1c, d.get('class', 3))
    m.w(cat + 0xc, d.get('category_c', 0))
    m.w(a + 0x18, cat)
    m.w(a + 0x1c, d['flags'])
    m.w(a + 0x24, d.get('height', f32(1.25)))
    m.w(a + 0x28, d['scale'][0])
    m.w(a + 0x2c, d['scale'][1])
    m.wb(a + 0x34, d.get('settlement', 0))
    for base, sh in ((SHAPE_A, d.get('shape_a', [1, f32(0.5), 0, 0, 0, 0])),
                     (SHAPE_B, d.get('shape_b', [1, f32(0.75), 0, 0, 0, 0]))):
        for k, v in enumerate(sh):
            m.w(a + base + 4 * k, v)
    m.wb(a + 0x108, d.get('variations', 0))
    weights = d.get('aspects', [])
    arr = m.alloc(4 * max(1, len(weights)), 0)
    for k, wbits in enumerate(weights):
        asp = m.alloc(0x40, 0)
        m.w(asp + 0x2c, wbits)
        m.w(arr + 4 * k, asp)
    m.w(a + 0x10c, arr)
    m.w(a + 0x110, len(weights))
    m.w(a + 0x128, d.get('total_weight', 0))
    m.w(a + 0x174, d['mask'])
    m.wb(a + 0x1b0, d.get('numbered', 0))
    if d.get('names'):
        lst = m.alloc(0x40, 0)
        entries = m.alloc(4 * len(d['names']), 0)
        for k, text in enumerate(d['names']):
            m.w(entries + 4 * k, m.cstring(text))
        m.w(lst + 0x18, entries)
        m.w(lst + 0x1c, len(d['names']))
        m.w(a + 0x1ac, lst)
        m.labels[lst] = 'names:' + name
    tracks = d.get('tracks', [])
    if tracks:
        arr = m.alloc(0x24 * len(tracks), 0)
        for k, (kind, length) in enumerate(tracks):
            m.w(arr + 0x24 * k, kind)
            m.w(arr + 0x24 * k + 0x20, length)
        m.w(a + 0x25c, arr)
        m.w(a + 0x260, len(tracks))
    for k, v in enumerate(d.get('node_offset', [0, 0, 0])):
        m.w(a + 0x270 + 4 * k, v)
    m.w(a + 0x5b4, d.get('zone_radius', 0))
    if d.get('required_object'):
        m.w(a + 0x4e0, 0x4e04e0)
    return a


def link_definition(m, name, d):
    a = m.defs[name]
    if d.get('trailer'):
        m.w(a + 0x268, m.defs[d['trailer']])
    if d.get('layout'):
        lay = m.alloc(0x40, 0)
        data = m.alloc(0x40, 0)
        m.w(lay + 8, data)
        m.w(data + 0x2c, d['layout'])
        m.w(a + 0x2f0, lay)
        m.labels[lay] = 'layout:' + name


def make_params(m, p, defn):
    a = m.alloc(0x60, 0)
    m.labels[a] = 'params'
    m.w(a, 0xa25b28)
    m.w(a + 4, defn)
    for k, v in enumerate(p['position'] + p['direction']):
        m.w(a + 8 + 4 * k, v)
    m.w(a + 0x1c, m.cstring(p.get('name', '')))
    m.w(a + 0x20, p.get('variant', 0xffffffff))
    m.w(a + 0x24, p.get('requested_id', 0))
    m.w(a + 0x28, m.owner if p.get('owner') else 0)
    if p.get('layout') is not None:
        lay = m.alloc(0x40, 0)
        m.labels[lay] = 'params layout'
        m.w(a + 0x2c, lay)
    m.w(a + 0x30, p.get('word_30', 0))
    m.w(a + 0x34, p.get('word_34', 0xffffffff))
    m.w(a + 0x38, p.get('mode', 0))
    m.w(a + 0x3c, p.get('word_3c', 0))
    m.w(a + 0x40, p.get('word_40', 0))
    m.w(a + 0x44, p.get('word_44', 0))
    m.w(a + 0x48, p.get('word_48', 0))
    m.wb(a + 0x50, p.get('byte_50', 0))
    m.wb(a + 0x51, p.get('byte_51', 0))
    m.wb(a + 0x52, p.get('byte_52', 0))
    return a


def actor_label(m, a):
    if a in m.actors:
        return 'actor%d' % m.actors.index(a)
    return m.label(a)


def component_owner(m, c):
    for idx, kind, addr in m.components:
        if addr == c:
            return idx, kind
    return None, None


def install_seams(m, spec):
    res = spec.get('results', {})

    def lab(v):
        return actor_label(m, v)

    def alloc(m):
        n = m.arg(0)
        p = m.alloc(n, 0xa5)
        if n == 0x180:
            m.actors.append(p)
            m.labels[p] = 'actor%d' % (len(m.actors) - 1)
            m.record('alloc actor', index=len(m.actors) - 1)
        m.ret(0, p)
    m.seam(0x6f7810, alloc)
    m.seam(0x6f7430, lambda m: m.ret(0))

    # ---- component constructors and their fixture virtuals
    def make_ctor(kind, size):
        def h(m):
            block, actor = m.ecx(), m.arg(0)
            idx = m.actors.index(actor) if actor in m.actors else None
            m.record('ctor', kind=kname(kind), actor=lab(actor))
            m.components.append((idx, kind, block))
            m.labels[block] = 'comp%s:%s' % (idx, kname(kind))
            vt = m.alloc(0x40, 0)
            for off, (name, pop, nargs) in VSTUBS.items():
                m.w(vt + off, m.stub(make_virtual(kind, name, pop, nargs)))
            m.w(vt + 0x20, REQUIREMENT[kind])
            m.w(block, vt)
            m.w(block + 4, actor)
            if kind == 0:
                # Body +10 current / +14 maximum health (read by 4D6F10's trailer
                # health copy 562200)
                m.w(block + 0x10, res.get('body_health', f32(80.0)))
                m.w(block + 0x14, res.get('body_maximum', f32(100.0)))
            org = spec.get('organization')
            if kind == 7 and org is not None:
                # Organization fixture for 4FA150: +18 layout data {+18 leader,
                # +28 slot -> group, +2C slot count}, +1C group array.
                data = m.alloc(0x40, 0)
                slots = m.alloc(4 * max(1, len(org['slot_groups'])), 0)
                groups = m.alloc(4 * 16, 0)
                m.w(block + 0x18, data)
                m.w(block + 0x1c, groups)
                m.w(data + 0x18, org['leader'])
                m.w(data + 0x28, slots)
                m.w(data + 0x2c, len(org['slot_groups']))
                for k, gi in enumerate(org['slot_groups']):
                    m.w(slots + 4 * k, gi)
                for gi in org['defined_groups']:
                    m.w(groups + 4 * gi, 0x6660000 + gi)
            m.ret(4, block)
        return h

    def make_virtual(kind, name, pop, nargs):
        def h(m):
            c = m.ecx()
            idx, _ = component_owner(m, c)
            args = [m.arg(i) for i in range(nargs)]
            m.record(name, actor='actor%s' % idx, kind=kname(kind),
                     args=[m.label(x) for x in args])
            r = res.get('%s:%s:%s' % (name, idx, kname(kind)), res.get('%s:%s' % (name, kname(kind)), 1))
            m.ret(pop, r)
        return h
    for kind, (size, addr) in CTORS.items():
        m.seam(addr, make_ctor(kind, size))
    m.leaf(0x4d1050, '4D1050', 0, 0, ret=res.get('4D1050', 0))

    # ---- actor state
    def group(m):
        m.record('state group', actor=lab(m.ecx()), group=m.arg(0))
        m.ret(4, res.get('group13', 0))
    m.seam(0x4d49b0, group)

    def set_state(m):
        a, state, en = m.ecx(), m.arg(0), m.arg(1) & 0xff
        m.record('set state', actor=lab(a), state=state, enabled=en)
        word = a + (0x100 if state < 32 else 0x104)
        bit = 1 << (state & 31)
        m.w(word, (m.r(word) | bit) if en else (m.r(word) & ~bit))
        m.ret(8)
    m.seam(0x4d64f0, set_state)

    # ---- character
    def view(m):
        a = m.ecx()
        m.record('character view', actor=lab(a))
        v = m.alloc(0x60, 0)
        first = m.alloc(4, 0)
        m.w(first, res.get('view_first', 1))
        m.w(v + 0x48, first)
        m.w(v + 0x4c, res.get('view_count', 1))
        m.ret(0, v)
    m.seam(0x4d0720, view)
    m.leaf(0x563d50, 'character start', 0, 0, extra=lambda m, e, a: e.update(ecx=m.label(m.ecx())))
    m.leaf(0x563ba0, 'default stance', 0, 0)

    def set_name(m):
        m.record('set name', actor=lab(m.ecx()), text=m.read_wide(m.r(m.arg(0))))
        m.ret(4)
    m.seam(0x4d7f40, set_name)

    def assign_number(m):
        k, a = m.ecx(), m.arg(0)
        n = res.get('number', 7)
        m.record('assign number', kingdom=m.label(k), actor=lab(a))
        m.w(a + 0x170, n)
        m.ret(4, n)
    m.seam(0x4e4980, assign_number)
    m.leaf(0x562200, 'copy health', 8, 2)
    m.leaf(0x54bfc0, 'register construction', 8, 2)

    def org_layout(m):
        m.record('organization layout', component=m.label(m.ecx()), layout=m.label(m.arg(0)))
        m.ret(4)
    m.seam(0x57c450, org_layout)

    def assertion(m):
        m.record('assert dialog', expression=m.read_wide(m.arg(0)), line=m.arg(2))
        m.ret(0, res.get('assert', 0))
    m.seam(0x855d30, assertion)

    def name_find(m):
        m.record('4154D0', text=m.read_wide(m.r(m.arg(0))))
        entry = 0
        if res.get('name_entry') is not None:
            entry = m.alloc(8, 0)
            m.w(entry + 4, int(res['name_entry'], 16))
        m.ret(4, entry)
    m.seam(0x4154d0, name_find)
    m.leaf(0x5c9180, '5C9180', 4, 1)
    m.leaf(0x5043f0, '5043F0', 4, 1, ret=0x5043f0)
    m.leaf(0x4f7b70, '4F7B70', 0xc, 3)

    # ---- scene / spatial (as in the class 2/9 oracle)
    m.leaf(0x43b7d0, 'scene transform', 4, 1)
    m.leaf(0x43d160, 'scene add', 4, 1)
    m.leaf(0x4f4590, 'placement test', 0x14, 5, ret=lambda m, a: res.get('placement', 0))
    # 4EF6A0 with a GACD block on a GActor: 4F3640(params, actor, actor, 0, 0, 0, 0)
    m.leaf(0x4f3640, 'actor placement test', 0x1c, 7, ret=lambda m, a: res.get('placement', 0),
           extra=lambda m, e, a: e.update(args=[m.label(a[0]), lab(a[1]), lab(a[2])] + a[3:]))

    def descriptor(p):
        w = [m.r(p + 4 * i) for i in range(6)]
        shape = {SHAPE_A: 'A', SHAPE_B: 'B'}.get(w[0] - m.r(m.ecx_obj + 4) if False else None)
        return w[1:5]

    def index(name, setbit):
        def h(m):
            obj = m.arg(0)
            m.record(name, object=lab(obj), a=shape_desc(m, m.arg(1)), b=shape_desc(m, m.arg(2)))
            f = m.r(obj + 8)
            m.w(obj + 8, (f | 0x4000000) if setbit else (f & 0xfbffffff))
            m.ret(0xc)
        return h
    m.seam(0x553830, index('index remove', False))
    m.seam(0x5536b0, index('index insert', True))

    def move(m):
        m.record('index move', object=lab(m.arg(0)), descriptors=[shape_desc(m, m.arg(i)) for i in range(1, 5)])
        m.ret(0x14)
    m.seam(0x5539b0, move)

    def snap(m):
        xy = m.arg(0)
        m.record('snap', xy=[m.r(xy), m.r(xy + 4)], tile=m.arg(1))
        if res.get('snap') is not None:
            m.w(xy, res['snap'][0])
            m.w(xy + 4, res['snap'][1])
        m.ret(8)
    m.seam(0x46ee00, snap)

    def height(m):
        xy = m.arg(0)
        m.record('terrain height', xy=[m.r(xy), m.r(xy + 4)], flags=m.arg(1))
        m.ret_float(res.get('height', f32(7.25)), 8)
    m.seam(0x4f0e50, height)
    m.leaf(0x554350, 'blocking query shape', 8, 2)
    m.leaf(0x555620, 'blocking query', 0xc, 3)
    m.leaf(0x4ea150, 'tile footprint', 0, 0, extra=lambda m, e, a: e.update(ecx=lab(m.ecx())))
    m.leaf(0x4ebc70, 'visibility refresh', 0, 0, extra=lambda m, e, a: e.update(ecx=lab(m.ecx())))
    m.leaf(0x4eb500, 'ground paint', 4, 1, extra=lambda m, e, a: e.update(ecx=lab(m.ecx())))
    # 42A590(list; object, flag) -> 429080(list; object, &position, object, &position, flag)
    m.leaf(0x429080, 'creation fx', 0x14, 5, extra=lambda m, e, a: (e.clear(), e.update(
        seam='creation fx', list=fx_list(m, m.ecx()), object=lab(a[0]), flag=a[4],
        pos=[m.r(a[1] + 4 * i) for i in range(3)], pos2=[m.r(a[3] + 4 * i) for i in range(3)])))
    m.leaf(0x4f5490, 'destroy', 4, 1, extra=lambda m, e, a: e.update(args=[lab(a[0])]))

    def log(name):
        def h(m):
            e = m.record(name, line=m.arg(2), format=m.read_wide(m.arg(4)))
            m.ret(0)
        return h
    m.seam(0x404ea0, log('log 404EA0'))
    m.seam(0x404f60, log('log 404F60'))

    # ---- 4D1FD0 required object
    def find_required(m):  # 555830 thiscall(GMap; def, x, y) ret C
        m.record('find required object', definition=m.label(m.arg(0)), xy=[m.arg(1), m.arg(2)])
        found = res.get('required_found')
        m.ret(0xc, m.objects_by_gtid(found) if found else 0)
    m.objects_by_gtid = lambda gtid: m.r(m.idmgr + 0x20004 + 4 * (int(gtid, 16) & 0xffff))
    m.seam(0x555830, find_required)
    m.leaf(0x46b000, 'required placement blocked', 4, 1, ret=lambda m, a: res.get('required_blocked', 0))

    # ---- 4D7230 refresh calls
    for addr, name in ((0x4d4cf0, '4D4CF0'), (0x4d1350, '4D1350'), (0x4d67f0, '4D67F0'), (0x4d4130, '4D4130')):
        m.leaf(addr, name, 0, 0, extra=lambda m, e, a: e.update(ecx=lab(m.ecx())))
    m.leaf(0x5035e0, '5035E0', 0, 2, ret=lambda m, a: res.get('5035E0', 1),
           extra=lambda m, e, a: e.update(args=[lab(a[0]), a[1]]))
    m.leaf(0x5817a0, 'properties add', 0xc, 3, extra=lambda m, e, a: e.update(args=[a[0], a[1], lab(a[2])]))
    for addr, name in ((0x5788b0, 'organization refresh'), (0x585650, 'settlement refresh'),
                       (0x57e430, 'performer refresh'), (0x569e40, 'denizen refresh')):
        m.leaf(addr, name, 0, 0)
    def zone_ctor(m):  # 516640 thiscall(block; actor, zone flags word) ret 8
        m.record('zone ctor', actor=lab(m.arg(0)), mask=m.arg(1) & 0xffff)
        m.ret(8, m.ecx())
    m.seam(0x516640, zone_ctor)

    def zone_place(m):  # 515F40 thiscall(zone or null; x, y, z) ret C
        m.record('zone place', zone=m.ecx() != 0, xyz=[m.arg(0), m.arg(1), m.arg(2)])
        m.ret(0xc)
    m.seam(0x515f40, zone_place)
    m.leaf(0x7a0c20, 'SAI notify', 4, 1, extra=lambda m, e, a: e.update(args=[lab(a[0])]))

    def track(m):  # 50AB10 thiscall(block; actor, period) ret 8
        m.record('track updatable', actor=lab(m.arg(0)), period=m.arg(1))
        m.ret(8, m.ecx())
    m.seam(0x50ab10, track)

    # ---- PostCreate / registration
    m.leaf(0x4e6180, 'kingdom add actor', 4, 1, extra=lambda m, e, a: e.update(args=[lab(a[0])]))
    m.leaf(0x5644f0, 'city create', 0, 3, extra=lambda m, e, a: e.update(args=[lab(a[0]), a[1], a[2]]))
    m.leaf(0x4d4a90, 'state reenter', 8, 2, extra=lambda m, e, a: e.update(ecx=lab(m.ecx())))
    m.leaf(0x4cfea0, 'grid register', 0, 0, extra=lambda m, e, a: e.update(ecx=lab(m.ecx())))

    # ---- entry points (4FA150)
    def spawn(m):  # 57AD30 thiscall(org; slot, flag) ret 8
        idx, kind = component_owner(m, m.ecx())
        m.record('57AD30', actor='actor%s' % idx, slot=m.arg(0), flag=m.arg(1) & 0xff)
        m.ret(8, 0)
    m.seam(0x57ad30, spawn)
    m.leaf(0x57a730, '57A730', 0, 0, ret=lambda m, a: res.get('member_count', 1),
           extra=lambda m, e, a: e.update(ecx=m.label(m.ecx())))
    m.seam(0x977111, lambda m: m.throw('LibmError'))
    m.leaf(0x79ea90, 'SAI 79EA90', 4, 1, ret=0, extra=lambda m, e, a: e.update(args=[m.label(a[0])]))
    m.leaf(0x4c2bc0, '4C2BC0', 0, 0, ret=lambda m, a: res.get('first_slot', 0))

    def slot_test(m):
        xy = m.arg(2)
        m.record('578290', layout=m.label(m.arg(0)), slot=m.arg(1), xy=[m.r(xy), m.r(xy + 4)],
                 facing=[m.arg(3), m.arg(4)])
        if res.get('slot_xy') is not None:
            m.w(xy, res['slot_xy'][0])
            m.w(xy + 4, res['slot_xy'][1])
        m.ret(0, res.get('slot_ok', 1))
    m.seam(0x578290, slot_test)


def shape_desc(m, p):
    w = [m.r(p + 4 * i) for i in range(6)]
    names = {}
    for nm_, d in m.defs.items():
        names[d + SHAPE_A] = 'A'
        names[d + SHAPE_B] = 'B'
    return [names.get(w[0], hex(w[0]))] + w[1:5]


def fx_list(m, p):
    for nm_, d in m.defs.items():
        for off in (0xbc, 0xe4):
            if p == d + off:
                return '%s+%X' % (nm_, off)
    return hex(p)


def snapshot_actor(m, a):
    words = [m.r(a + 4 * i) for i in range(0x180 // 4)]
    comps = []
    for slot in range(31):
        c = words[0x60 // 4 + slot]
        if c:
            idx, kind = component_owner(m, c)
            comps.append([slot, kname(kind)])
    # pointer-valued words become labels
    out = []
    for i, w in enumerate(words):
        off = 4 * i
        if 0x60 <= off < 0xdc:
            out.append(None)
        elif w in m.labels or w in m.actors:
            out.append(actor_label(m, w))
        elif off in (0x18, 0xdc, 0xe0):
            out.append({'string': m.cstring_units(w) if w else None})
        else:
            out.append(w)
    return {'words': out, 'components': comps}


def run(spec):
    m = build(spec)
    m.reset_case() if hasattr(m, 'reset_case') else None
    m.calls, m.outcome = [], None
    m.null_accesses.clear()
    m.guard = True
    entry = {'create': 0x4f9c10, 'actor': 0x4fa030, 'organization': 0x4fa150}[spec.get('entry', 'create')]
    r = m.call(entry, m.gworld, [m.params], 4)
    m.guard = False
    out = {'returned': None if m.outcome else (actor_label(m, r) if r else 0), 'outcome': m.outcome,
           'calls': m.calls, 'rng_after': m.r(m.gworld + 0xd4),
           'reads': {k: sorted(v) for k, v in m.mem_reads.items()},
           'params_after': [m.r(m.params + 4 * i) for i in range(0x54 // 4)],
           'unexpected': m.unexpected}
    out['actors'] = [snapshot_actor(m, a) for a in m.actors]
    lst, n = [], m.r(m.gworld + 8)
    while n and len(lst) < 16:
        lst.append(actor_label(m, m.r(n)))
        n = m.r(n + 8)
    out['world_list_8'] = lst
    cities, n = [], m.r(m.gworld + 0x260)
    while n and len(cities) < 16:
        cities.append(m.r(n))
        n = m.r(n + 4)
    out['available_cities'] = cities
    names = []
    for k in range(m.r(m.gworld + 0x26c)):
        e = m.r(m.r(m.gworld + 0x268) + 4 * k)
        names.append([m.label(m.r(e)), m.r(e + 4)])
    out['name_assignments'] = names
    idm = m.idmgr
    out['idmgr'] = {'count': m.r(idm + 0x60004), 'cursor': m.r(idm + 0x60008), 'highest': m.r(idm + 0x6000c)}
    out['latches'] = {hex(a): m.rb(a) for a in (0xb3cc40, 0xb3cc42, 0xb3cc43, 0xb3cc44)}
    return out


# ------------------------------------------------------------------ cases
def defn(**kw):
    d = dict(flags=0x00800000, scale=[f32(0.9), f32(1.1)], mask=0, height=f32(1.25), variations=0,
             aspects=[], total_weight=0, numbered=0)
    d.update(kw)
    return d


def base(**kw):
    s = dict(pc='Double53', rng=0x1234567, begin=f32(12.5), tile_shift=3, tile_size=f32(8.0), map_mode=0,
             direction_table_pc='Double53', idmgr={'count': 0, 'cursor': 40, 'highest': 41, 'slots': []},
             definition='unit', definitions={'unit': defn()},
             params={'position': [f32(33.5), f32(-12.75), f32(0.0)], 'direction': [f32(0.6), f32(-0.8)],
                     'owner': True}, owner=True, results={})
    for k, v in kw.items():
        if k in ('params', 'results') and isinstance(v, dict):
            s[k] = {**s[k], **v}
        else:
            s[k] = v
    return s


INFANTRY = (1 << 0) | (1 << 2) | (1 << 3) | (1 << 5) | (1 << 6) | (1 << 8) | (1 << 17) | (1 << 22)


def cases():
    out = []
    add = lambda label, **kw: out.append((label, base(**kw)))
    for pc in PCS:
        add('empty_mask_' + pc, pc=pc)
        add('infantry_' + pc, pc=pc, definitions={'unit': defn(mask=INFANTRY, variations=1,
                                                              aspects=[f32(1.0), f32(2.0)], total_weight=f32(3.0))})
    add('no_owner', params={'owner': False})
    add('requested_id', params={'requested_id': (3 << 16) | 77}, idmgr={'count': 0, 'cursor': 40, 'highest': 41,
                                                                       'slots': [(77, 3, 0)]})
    add('ids_full', idmgr={'count': 0x8000, 'cursor': 3, 'highest': 9, 'slots': []})
    add('mode1', params={'mode': 1})
    add('mode3_body', params={'mode': 3}, definitions={'unit': defn(mask=INFANTRY)})
    add('byte50', params={'byte_50': 1})
    add('nonblocking_blocking_flag', params={'byte_52': 1}, definitions={'unit': defn(flags=0x00800400)})
    add('nonblocking_plain', params={'byte_52': 1})
    add('group13_active', results={'group13': 5})
    add('init_create_false_slot5', definitions={'unit': defn(mask=INFANTRY)}, results={'InitCreate:5': 0})
    add('init_existing_false_slot3', definitions={'unit': defn(mask=INFANTRY)}, results={'InitExisting:3': 0})
    add('mind_init_existing_false', definitions={'unit': defn(mask=INFANTRY)}, results={'InitExisting:4elem': 0})
    add('requirement_missing', definitions={'unit': defn(mask=(1 << 18))})
    add('organization_mind', definitions={'unit': defn(mask=(1 << 7) | (1 << 0))})
    add('aircraft_mind_missing_aircraft', definitions={'unit': defn(mask=(1 << 23) | (1 << 0))})
    add('aircraft_mind', definitions={'unit': defn(mask=(1 << 23) | (1 << 0) | (1 << 5))})
    add('all_components', definitions={'unit': defn(mask=0x7fffffff, zone_radius=f32(5.0))})
    add('construction_state', definitions={'unit': defn(mask=(1 << 15) | (1 << 0) | (1 << 9) | (1 << 8))},
        params={'mode': 1}, results={})
    add('numbered_owner', definitions={'unit': defn(numbered=1)})
    add('numbered_no_owner', definitions={'unit': defn(numbered=1)}, params={'owner': False})
    add('names_list', definitions={'unit': defn(names=['Alpha', 'Bravo', 'Charlie'])})
    add('names_reason4', definitions={'unit': defn(names=['Alpha', 'Bravo'], numbered=1)}, params={'mode': 4})
    add('character_start', definitions={'unit': defn(mask=INFANTRY)}, params={'byte_51': 1})
    add('character_start_no_view', definitions={'unit': defn(mask=INFANTRY)}, params={'byte_51': 1},
        results={'view_count': 0})
    add('character_start_null_first', definitions={'unit': defn(mask=INFANTRY)}, params={'byte_51': 1},
        results={'view_first': 0})
    add('trailer', definitions={'unit': defn(mask=INFANTRY, trailer='limber', node_offset=[f32(0.5), f32(-2.0), f32(0.25)]),
                                'limber': defn(mask=1, node_offset=[f32(0.0), f32(1.5), f32(0.0)])})
    add('trailer_no_body', definitions={'unit': defn(mask=INFANTRY & ~1, trailer='limber'),
                                        'limber': defn(mask=1)})
    add('required_object_none_found', definitions={'unit': defn(required_object=1)})
    add('required_object_found', definitions={'unit': defn(required_object=1)},
        objects={'0x30009': {'x': f32(4.0), 'y': f32(5.0)}}, results={'required_found': '0x30009'})
    add('required_object_saved_live', definitions={'unit': defn(required_object=1)},
        objects={'0x30009': {}}, params={'word_30': 0x30009})
    add('required_object_saved_blocked', definitions={'unit': defn(required_object=1)},
        objects={'0x30009': {}}, params={'word_30': 0x30009}, results={'required_blocked': 1})
    add('required_object_saved_dead', definitions={'unit': defn(required_object=1)},
        objects={'0x30009': {'live': False}}, params={'word_30': 0x30009})
    add('required_object_map_mode1', definitions={'unit': defn(required_object=1)},
        objects={'0x30009': {}}, params={'word_30': 0x30009}, map_mode=1)
    add('word30_object_no_requirement', objects={'0x30009': {}}, params={'word_30': 0x30009})
    add('params_40_live', objects={'0x2000a': {}}, params={'word_40': 0x2000a})
    add('params_44_fallback', objects={'0x2000b': {}}, params={'word_40': 0x5000c, 'word_44': 0x2000b})
    add('global_property', definitions={'unit': defn(mask=INFANTRY)}, global_property=0x61)
    add('global_property_test_false', definitions={'unit': defn(mask=INFANTRY)}, global_property=0x61,
        results={'5035E0': 0})
    add('settlement_byte', definitions={'unit': defn(settlement=1)})
    add('settlement_byte_zone_refused', definitions={'unit': defn(settlement=1, flags=0x00800001)},
        results={'zone_allowed': 0})
    add('sai', sai=1)
    add('sai_flag0', sai=1, definitions={'unit': defn(flags=0x00800001)})
    add('tracks', definitions={'unit': defn(tracks=[(1, f32(2.5)), (0, f32(9.0)), (1, f32(4.0))])})
    add('tracks_none_type1', definitions={'unit': defn(tracks=[(0, f32(2.5))])})
    add('fx_construction_state', definitions={'unit': defn(mask=(1 << 15) | 1)}, params={'mode': 1})
    add('fx_params_mismatch', params={'requested_id': (3 << 16) | 77, 'word_3c': 5},
        idmgr={'count': 0, 'cursor': 40, 'highest': 41, 'slots': [(77, 3, 0)]})
    add('kingdom_postcreate_flag', definitions={'unit': defn(mask=(1 << 29), zone_radius=0)})
    add('city_zone_radius', definitions={'unit': defn(mask=(1 << 29), zone_radius=f32(3.0))})
    add('registered_states', registered_states=40, params={'mode': 1})
    add('placement_fails', results={'placement': 2})
    add('placement_fails_mode2', results={'placement': 2}, map_mode=2)
    add('map_mode_1', map_mode=1)
    add('no_scene', scene=False)
    add('random_facing', definitions={'unit': defn(flags=0x00808000)})
    add('organization_layout', definitions={'unit': defn(mask=(1 << 7) | 1, layout=3)},
        params={'layout': 1})
    add('organization_layout_mode1_assert', definitions={'unit': defn(mask=(1 << 7) | 1)},
        params={'layout': 1, 'mode': 1})
    add('name_nonempty', params={'name': 'hero'})
    add('name_nonempty_replaces_entry', params={'name': 'hero'}, results={'name_entry': '0x50007'},
        objects={'0x50007': {'live': False}})
    add('name_instance_used', params={'name': 'hero'}, results={'name_entry': '0x50007'},
        objects={'0x50007': {}})
    add('names_collision', definitions={'unit': defn(names=['Alpha', 'Bravo', 'Charlie'])},
        name_vector=[('unit', 1)])
    add('names_collision_wrap', definitions={'unit': defn(names=['Alpha', 'Bravo', 'Charlie'])},
        name_vector=[('unit', 1), ('unit', 0), ('unit', 2)])
    add('names_collision_two', definitions={'unit': defn(names=['Alpha', 'Bravo', 'Charlie'])},
        name_vector=[('unit', 0), ('unit', 1)])
    add('structure_requirement', definitions={'unit': defn(mask=(1 << 13) | 1)})
    add('sai_watching_components', sai=1, sai_watching=1, definitions={'unit': defn(mask=INFANTRY)})
    add('trailer_child_refused', definitions={'unit': defn(mask=INFANTRY, trailer='limber'),
                                              'limber': defn(mask=1)},
        results={'placement': 0, 'InitCreate:1:0': 0})
    add('entry_gactor', entry='actor')
    org = {'leader': 1, 'slot_groups': [0, 1, 2, 1], 'defined_groups': [0, 1]}
    add('entry_organization', entry='organization', organization=org,
        definitions={'unit': defn(mask=(1 << 7) | 1, layout=2)},
        results={'first_slot': 1, 'slot_xy': [f32(40.0), f32(-10.0)], 'member_count': 2})
    add('entry_organization_no_members', entry='organization', organization=org,
        definitions={'unit': defn(mask=(1 << 7) | 1, layout=2)}, results={'first_slot': 1, 'member_count': 0})
    add('entry_organization_slot_refused', entry='organization', organization=org,
        definitions={'unit': defn(mask=(1 << 7) | 1, layout=2)}, results={'first_slot': 1, 'slot_ok': 0})
    return out


def main():
    rows = []
    t = time.time()
    only = sys.argv[1:]
    for label, spec in cases():
        if only and label not in only:
            continue
        res = run(spec)
        rows.append({'label': label, 'spec': spec, 'result': res})
        print(label, res['returned'], res['outcome'], len(res['calls']), flush=True)
    doc = {'image': 'AA.exe (executed windows identical in runtime-expanded-v14/AA.exe)',
           'harness_sha256': {n: H((HERE / n).read_bytes()) for n in ('actor_machine.py', 'native_actor_creation.py')},
           'base_machine_sha256': H(Path(nm.__file__).read_bytes()),
           'native': sorted(hex(a) for a in NATIVE), 'seams': SEAM_DOC, 'cases': rows,
           'visited_instructions': len(VISITED)}
    if not only:
        (HERE / 'native-actor-creation.json').write_text(json.dumps(doc, indent=1))
    print(len(rows), 'cases %.1fs' % (time.time() - t))


if __name__ == '__main__':
    main()
