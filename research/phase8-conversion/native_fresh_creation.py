"""Fresh creation oracle: the ORIGINAL KKC_GWorld::CreateGThing 4F9C10 path
for GObject (class 2) and DynamicDoodad (class 9) run natively down to the
world-subsystem seams, and CreateGActor 4FA030 / CreateOrganization 4FA150.

Natively executed (NATIVE): 4F9C10; constructors 4EA5E0, 4EF2A0, 4EE9F0,
4453A0, 4DCB30, 4DC920, 491ED0, component constructors 590160 / 58FE10 /
58F880 / 50AB90; RNG 4F02B0 / 85D840 / 49B900 / 49B440 on GWorld+D4;
InitCreate 4E9FC0 / 4EE460 with the GTID manager 4EFDB0 / 4EFB70 / 4F0190
on a fixture table; 4DCBA0; component virtual +4 (5904A0, 58F960) with
50ACC0 / 50B250; InitExisting 4DC730 / 4EBE50 / 4EE700 / 4EFA40 / 4EF6A0 /
4EE4C0 / 4CF210 / 4EA0F0 / 4EB6E0 / 4F1F70 / 4EA3D0; PostCreate 4EBA40 /
4D1E10; registration 4F9510 / 4BEDF0 / 4F4C40; 50C220 / 50C270; 42A590;
_ftol 96CC4C; CString helpers. Any other function entry reached stops the
case as `Unexpected`.

Seams (recorded with their arguments; each is another owner's subsystem):
  77DC50 / 77DA70  debug profiler scope and RNG sync log (no state)
  4F4590           placement test (spatial) -> fixture reason code
  553830 / 5536B0  spatial index insert (active / inactive shape)
  5539B0           spatial index move (facing changed in 4EE9F0)
  46EE00           half-cell snap (spatial)
  4F0E50           terrain height -> fixture value in ST0
  554350 / 555620  blocking query (4EB6E0 with flag bit 10)
  4EA150           tile footprint notification
  4EBC70           visibility refresh (vision)
  4EB500           ground paint (terrain)
  43B7D0 / 43D160  presentation scene transform / add (GWorld+130 -> +4)
  429080           creation FX of definition+BC
  GWorld+1E8 vf C  deferred-queue add (returns a node); 4F1A70 queue unlink
  4F5490           destroy a created object whose initialization failed
  404EA0 / 404F60  debug logs; 5A1040 map warning; 4B9F40 text lookup
Fixture objects (GWorld, GMap, settings, definition, parameter block) record
every read offset (`reads`), so each input is one the original code consumed.
"""
import json, struct, sys, time
from pathlib import Path
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import Machine, P, VISITED, STACK, STOP, Throw, Stop, f32, DEF_VTABLES
from native_conversion import Conv, units
from unicorn import UC_HOOK_MEM_READ
from unicorn.x86_const import *

ROOT = HERE.parents[1]
FUNCS = set()
for line in (ROOT.parent / 'reconstruction/research/native-aa/functions.tsv').read_text().splitlines()[1:]:
    FUNCS.add(int(line.split('\t')[0], 16))

NATIVE = {
    0x4f9c10, 0x4ea5e0, 0x4ef2a0, 0x4ee9f0, 0x4453a0, 0x4dcb30, 0x4dc920, 0x491ed0, 0x590160,
    0x58fe10, 0x58f880, 0x50ab90, 0x49b900, 0x49b440, 0x85d840, 0x4f02b0, 0x4e9fc0, 0x4ee460,
    0x4efdb0, 0x4efb70, 0x4f0190, 0x4dcba0, 0x5904a0, 0x58f960, 0x50acc0, 0x50b250, 0x4dc730,
    0x4ebe50, 0x4ee700, 0x4efa40, 0x4ef6a0, 0x4f0460, 0x4ee4c0, 0x4cf210, 0x4ea0f0, 0x4ea010,
    0x4ea020, 0x4eb6e0, 0x4f1f70, 0x4ea3d0, 0x4eba40, 0x4d1e10, 0x4f9510, 0x4bedf0, 0x4f4c40,
    0x50c220, 0x50c270, 0x42a590, 0x96cc4c, 0x405b80, 0x4012c0, 0x4047a0, 0x6a71d0, 0x6a71f0,
    0x4c9fa0, 0x4f0500, 0x462e50, 0x4d1e40, 0x4fa030, 0x4fa150, 0x401560, 0x4fa113, 0x4fa10d, 0x4fa2c4, 0x4fa2bb,
}
DIRECTIONS = (ROOT / 'research/phase5-actors/direction-tables.bin').read_bytes()
PCS = {'Single24': 0, 'Double53': 1, 'Extended64': 2}
SHAPE_A, SHAPE_B = 0x38, 0x50


class Fresh(Conv):
    def __init__(self, precision='Double53'):
        super().__init__(precision)
        self.guard = False
        self.trace = []
        self.unexpected = []
        self.mem_reads = {}
        self.idmgr = self.alloc(0x60020, 0)
        self.bitmap = self.alloc(0x2000, 0)
        self.w(self.idmgr + 0x60010, self.bitmap)
        self.w(self.idmgr + 0x60008, 1)
        self.generations = self.idmgr
        self.w(0xb364c0, self.idmgr)
        self.gworld = self.alloc(0x800, 0)
        self.w(0xb36534, self.gworld)
        self.world = self.gworld
        self.nodes = []
        self.obj = None
        self.defn = 0
        for addr, name, pop, nargs in ((0x77dc50, '77DC50', 8, 2), (0x77da70, '77DA70', 0xc, 3)):
            self.leaf(addr, name, pop, nargs, record=False)

    def _hook(self, u, pc, size, user):
        if self.guard and pc in FUNCS:
            self.trace.append(pc)
            if pc not in self.seams and pc not in NATIVE:
                self.unexpected.append(hex(pc))
                self.outcome = {'kind': 'Unexpected', 'pc': hex(pc)}
                raise Stop()
        super()._hook(u, pc, size, user)

    def probe(self, name, base, size):
        seen = self.mem_reads.setdefault(name, set())

        def h(u, access, address, sz, value, _):
            seen.add(address - base)
        self.u.hook_add(UC_HOOK_MEM_READ, h, begin=base, end=base + size - 1)

    def leaf(self, address, name, pop, nargs=0, ret=0, record=True):
        def h(m):
            args = [m.arg(i) for i in range(nargs)]
            if record:
                m.record(name, ecx=m.u.reg_read(UC_X86_REG_ECX), args=args)
            m.ret(pop, ret(m, args) if callable(ret) else ret)
        self.seam(address, h)

    def ret_float(self, bits, pop):
        code = self.alloc(32)
        data = code + 16
        self.w(data, bits)
        self.u.mem_write(code, bytes([0xd9, 0x05]) + struct.pack('<I', data) + bytes([0xc2]) + struct.pack('<H', pop))
        self.u.reg_write(UC_X86_REG_EIP, code)

    def shape_name(self, p):
        return {self.defn + SHAPE_A: 'A', self.defn + SHAPE_B: 'B'}.get(p, hex(p))

    def descriptor(self, p):
        """A 6-word index descriptor {shape, facing x, y, position x, y, 0}."""
        w = [self.r(p + 4 * i) for i in range(6)]
        return [self.shape_name(w[0])] + w[1:]


def build(spec):
    """One creation world + definition + parameter block from a case spec."""
    m = Fresh(spec['pc'])
    g = m.gworld
    m.w(g + 0xd4, spec['rng'])
    m.w(g + 0xd8, spec['world_d8'])
    m.w(g + 0xe0, spec['begin'])
    gmap = m.alloc(0x100, 0)
    m.w(gmap + 4, spec['tile_shift'])
    m.w(gmap + 8, spec['tile_size'])
    m.w(g + 0x30, gmap)
    settings = m.alloc(0x100, 0)
    m.w(settings + 0xa8, spec['map_mode'])
    m.w(g + 0x28, settings)
    scene = m.alloc(0x40, 0)
    m.w(scene + 4, 0x5ce4e if spec['scene'] else 0)
    m.w(g + 0x130, scene)
    app = m.alloc(0x200, 0)
    m.w(0xb36550, app)
    qvt = m.alloc(0x20, 0)
    m.w(g + 0x1e8, qvt)

    def queue_add(m):  # GWorld+1E8 vf C (this, updatable, delay, data) ret C -> node
        upd, delay, data = m.arg(0), m.arg(1), m.arg(2)
        node = m.alloc(0x14, 0)
        m.nodes.append(node)
        m.record('queue add', updatable=m.labels.get(upd, hex(upd)), delay=delay, data=data,
                 node=len(m.nodes) - 1)
        m.ret(0xc, node)
    m.w(qvt + 0xc, m.stub(queue_add))

    def queue_unlink(m):  # 4F1A70 thiscall(GWorld, node) ret 4
        node = m.arg(0)
        m.record('queue unlink', node=m.nodes.index(node) if node in m.nodes else hex(node))
        m.ret(4)
    m.seam(0x4f1a70, queue_unlink)
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
    d = m.alloc(0x200, 0)
    m.defn = d
    vt = {2: 'ObjectData', 9: 'DynamicDoodadData'}[spec['class']]
    m.w(d, DEF_VTABLES[vt])
    m.w(d + 8, m.cstring('def_' + str(spec['class'])))
    m.w(d + 0x10, m.cstring('display_' + str(spec['class'])))
    cat = m.alloc(0x40, 0)
    m.w(cat + 0x1c, spec['class'])
    m.w(d + 0x18, cat)
    m.w(d + 0x1c, spec['flags'])
    m.w(d + 0x24, spec['radius_factor'])
    m.w(d + 0x28, spec['scale'][0])
    m.w(d + 0x2c, spec['scale'][1])
    m.wb(d + 0x34, spec['city'])
    for base, sh in ((SHAPE_A, spec['shape_a']), (SHAPE_B, spec['shape_b'])):
        for k, v in enumerate(sh):
            m.w(d + base + 4 * k, v)
    m.wb(d + 0x108, spec['variations'])
    weights = spec['aspects']
    arr = m.alloc(4 * max(1, len(weights)), 0)
    for k, wbits in enumerate(weights):
        a = m.alloc(0x40, 0)
        m.w(a + 0x2c, wbits)
        m.w(arr + 4 * k, a)
    m.w(d + 0x10c, arr)
    m.w(d + 0x110, len(weights))
    m.w(d + 0x128, spec['total_weight'])
    m.comp_defs = {}
    for slot, period in enumerate(spec['components']):
        if period is not None:
            cd = m.alloc(0x40, 0)
            m.w(cd + 0x10, period)
            m.w(d + 0x174 + 4 * slot, cd)
            m.comp_defs[slot] = cd
    p = m.alloc(0x28, 0)
    m.w(p, 0xa25b1c)
    m.w(p + 4, d)
    for k, v in enumerate(spec['position'] + spec['direction']):
        m.w(p + 8 + 4 * k, v)
    m.w(p + 0x1c, m.cstring(spec['name']))
    m.w(p + 0x20, spec['variant'])
    m.w(p + 0x24, spec['requested_id'])
    m.params = p
    m.leaf(0x43b7d0, 'scene transform', 4, 1)
    m.leaf(0x43d160, 'scene add', 4, 1)
    m.leaf(0x4f4590, 'placement test', 0x14, 5, ret=spec['placement'])
    # 5536B0 inserts the object into the spatial index and sets its flag bit 26
    # (55372F..55373C); 553830 removes it and clears the bit (5538AF..5538BC).
    # The seams apply exactly that flag write; the index itself is the
    # spatial owner's.
    for addr, name, setbit in ((0x553830, 'index remove', False), (0x5536b0, 'index insert', True)):
        def h(m, name=name, setbit=setbit):
            obj = m.arg(0)
            m.record(name, object=obj == (m.obj or 0), a=m.descriptor(m.arg(1)), b=m.descriptor(m.arg(2)))
            f = m.r(obj + 8)
            m.w(obj + 8, (f | 0x4000000) if setbit else (f & 0xfbffffff))
            m.ret(0xc)
        m.seam(addr, h)

    def move(m):  # 5539B0 thiscall(GMap, obj, &newA, &oldA, &newB, &oldB) ret 14
        m.record('index move', object=m.arg(0) == (m.obj or 0),
                 descriptors=[m.descriptor(m.arg(i)) for i in range(1, 5)])
        m.ret(0x14)
    m.seam(0x5539b0, move)

    def snap(m):  # 46EE00 thiscall(shape, &xy, tile size) ret 8
        xy = m.arg(0)
        m.record('snap', shape=m.shape_name(m.u.reg_read(UC_X86_REG_ECX)), xy=[m.r(xy), m.r(xy + 4)],
                 tile=m.arg(1))
        if spec['snap'] is not None:
            m.w(xy, spec['snap'][0])
            m.w(xy + 4, spec['snap'][1])
        m.ret(8)
    m.seam(0x46ee00, snap)

    def height(m):  # 4F0E50 thiscall(GWorld, &xy, flags) ret 8 -> ST0
        xy = m.arg(0)
        m.record('terrain height', xy=[m.r(xy), m.r(xy + 4)], flags=m.arg(1))
        m.ret_float(spec['height'], 8)
    m.seam(0x4f0e50, height)
    m.leaf(0x554350, 'blocking query shape', 8, 2)
    m.leaf(0x555620, 'blocking query', 0xc, 3)
    m.leaf(0x4ea150, 'tile footprint', 0, 0)
    m.leaf(0x4ebc70, 'visibility refresh', 0, 0)
    m.leaf(0x4eb500, 'ground paint', 4, 1)
    m.leaf(0x429080, 'creation fx', 0x14, 5)
    m.leaf(0x4f5490, 'destroy', 4, 1)

    def assertion(m):  # 855D30(expression, file, line) cdecl -> new latch value
        m.record('assert dialog', expression=m.read_wide(m.arg(0)), line=m.arg(2))
        m.ret(0, spec.get('assert_result', 0))
    m.seam(0x855d30, assertion)
    for latch, key in ((0xb3cc43, 'latch_43'), (0xb3cc44, 'latch_44')):
        m.wb(latch, spec.get(key, 0))

    def log(name):
        def h(m):
            m.record(name, line=m.arg(2), format=m.read_wide(m.arg(4)))
            m.ret(0)
        return h
    m.seam(0x404ea0, log('log 404EA0'))
    m.seam(0x404f60, log('log 404F60'))
    for name, (b, size) in {'gworld': (g, 0x800), 'gmap': (gmap, 0x100), 'settings': (settings, 0x100),
                            'definition': (d, 0x200), 'params': (p, 0x28)}.items():
        m.probe(name, b, size)
    return m


def run_create(spec):
    m = build(spec)
    m.reset_case()

    def alloc_obj(m):  # 6F7810: remember the object and component blocks
        n = m.arg(0)
        p = m.alloc(n, 0xa5)
        if m.obj is None and n in (0x60, 0x68):
            m.obj = p
            m.labels[p] = 'object'
        elif n == 0x1c:
            m.labels[p + 0xc] = 'component0'
        elif n == 0x30:
            m.labels[p + 0xc] = 'component1'
        m.ret(0, p)
    m.seam(0x6f7810, alloc_obj)
    m.guard = True
    entry = {'create': 0x4f9c10, 'actor': 0x4fa030, 'organization': 0x4fa150}[spec.get('entry', 'create')]
    r = m.call(entry, m.gworld, [m.params], 4)
    m.guard = False
    out = {'returned': None if m.outcome else ('object' if r and r == m.obj else r), 'outcome': m.outcome,
           'calls': m.calls, 'rng_after': m.r(m.gworld + 0xd4),
           'reads': {k: sorted(v) for k, v in m.mem_reads.items()},
           'params_after': [m.r(m.params + 4 * i) for i in range(10)],
           'trace': [hex(a) for a in m.trace]}
    if m.obj:
        o = m.obj
        out['object'] = [m.r(o + 4 * i) for i in range(0x68 // 4)]
        out['object_name'] = m.cstring_units(m.r(o + 0x18))
        comps = []
        for slot in range(2):
            c = m.r(o + 0x60 + 4 * slot) if spec['class'] == 9 else 0
            if c:
                n = 0x1c if slot == 0 else 0x30
                words = [m.r(c + 4 * i) for i in range(n // 4)]
                comps.append({'slot': slot, 'words': words, 'owner_is_object': words[1] == o,
                              'definition_is_slot_def': words[2] == m.comp_defs.get(slot),
                              'event_head_set': words[4] != 0})
            else:
                comps.append(None)
        out['components'] = comps
    lst, n = [], m.r(m.gworld + 8)
    while n and len(lst) < 16:
        lst.append('object' if m.r(n) == m.obj else hex(m.r(n)))
        n = m.r(n + 8)
    out['world_list_8'] = lst
    cities, n = [], m.r(m.gworld + 0x260)
    while n and len(cities) < 16:
        cities.append(m.r(n))
        n = m.r(n + 4)
    out['available_cities'] = cities
    out['available_cities_tail_is_head'] = m.r(m.gworld + 0x264) == m.r(m.gworld + 0x260)
    idm = m.idmgr
    out['idmgr'] = {'count': m.r(idm + 0x60004), 'cursor': m.r(idm + 0x60008), 'highest': m.r(idm + 0x6000c)}
    return out


def base(**kw):
    s = dict(pc='Double53', cls=9, flags=0, radius_factor=f32(0.5), scale=[f32(0.8), f32(1.2)], city=0,
             shape_a=[3, f32(2.0), 0, 0, f32(1.5), f32(2.5)], shape_b=[1, f32(1.0)], variations=1,
             aspects=[f32(1.0), f32(3.0)], total_weight=f32(4.0), components=[None, None],
             tile_shift=3, tile_size=f32(8.0), map_mode=0, world_d8=0, scene=True, rng=0x1234567,
             begin=f32(12.5), direction_table_pc='Double53',
             idmgr={'count': 0, 'cursor': 40, 'highest': 41, 'slots': [(5, 3, 0)]}, placement=0,
             snap=None, height=f32(7.25), position=[f32(33.5), f32(-12.75), f32(0.0)],
             direction=[f32(0.6), f32(-0.8)], name='', variant=1, requested_id=(3 << 16) | 5)
    s.update(kw)
    s['class'] = s.pop('cls')
    return s


def cases():
    out = []
    add = lambda label, **kw: out.append((label, base(**kw)))
    for pc in PCS:
        add('doodad_requested_' + pc, pc=pc)
        add('doodad_random_variant_' + pc, pc=pc, variant=0xffffffff)
        add('doodad_components_' + pc, pc=pc, components=[f32(5.0), f32(2.5)], variant=0xffffffff)
        add('object_class2_' + pc, pc=pc, cls=2, variant=0xffffffff, requested_id=0)
    add('allocate_id', requested_id=0)
    add('allocate_id_skips_live', requested_id=0, idmgr={'count': 2, 'cursor': 7, 'highest': 9,
                                                          'slots': [(7, 2, 1), (8, 1, 1), (9, 4, 0)]})
    add('allocate_id_wraps', requested_id=0, idmgr={'count': 1, 'cursor': 0xffff, 'highest': 0xffff,
                                                    'slots': [(0xffff, 1, 1)]})
    add('allocate_id_full', requested_id=0, idmgr={'count': 0x8000, 'cursor': 3, 'highest': 9, 'slots': []})
    add('allocate_id_highest_grows', requested_id=0, idmgr={'count': 0, 'cursor': 50, 'highest': 41, 'slots': []})
    add('requested_id_above_highest', requested_id=(2 << 16) | 77, idmgr={'count': 0, 'cursor': 40, 'highest': 41,
                                                                            'slots': [(77, 2, 0)]})
    add('variations_off', variations=0, variant=0xffffffff)
    add('variant_explicit_out_of_range', variant=7)
    add('component_slot0_only', components=[f32(1.0), None])
    add('component_slot1_only', components=[None, f32(0.75)])
    add('component_zero_period', components=[0, 0])
    add('flag_fixed_facing', flags=1 << 17)
    add('flag_random_facing', flags=1 << 15)
    add('flag_random_facing_table_pc24', flags=1 << 15, direction_table_pc='Single24')
    add('flag_3d', flags=1 << 12)
    add('invalid_direction', direction=[f32(3.0), f32(4.0)])
    add('flag_index_active', flags=1 << 26)
    add('flag_index_active_facing', flags=(1 << 26) | (1 << 15))
    add('flag_snap', flags=1 << 16, snap=[f32(36.0), f32(-12.0)])
    add('flag_height_123', flags=1 << 21)
    add('flag_bit10_blocking', flags=1 << 10)
    add('flag_bit8_world_list', flags=1 << 8)
    add('flag_bit29', flags=1 << 29)
    add('thickness_warning', flags=0x20000400, shape_a=[3, f32(4.0), 0, 0, f32(1.0), f32(1.5)])
    add('thickness_ok', flags=0x20000400, shape_a=[2, f32(4.0), 0, 0, f32(3.0), f32(2.5)])
    add('thickness_small_radius', flags=0x20000400, shape_a=[3, f32(1.0), 0, 0, f32(1.0), f32(1.5)])
    add('city_byte', city=1)
    for code in range(1, 9):
        add('placement_fail_%d' % code, placement=code)
    add('placement_fail_1_mode2', placement=1, map_mode=2)
    add('placement_fail_3_mode2', placement=3, map_mode=2)
    add('map_mode_1', map_mode=1)
    add('world_state_6', world_d8=6)
    add('no_scene', scene=False)
    add('negative_tile_coords', position=[f32(-5.5), f32(-100.25), f32(0.0)])
    add('large_scale', scale=[f32(100.0), f32(1e6)], radius_factor=f32(3.3))
    add('entry_4fa030_object', entry='actor')
    return out


def run_entry(spec):
    """4FA030 / 4FA150 with 4F9C10 seamed (fresh actor creation is another
    port): the created object is a fixture with vtable +B4 (IsGActor) and, for
    4FA150, an Organization component at +7C {+18 layout data (+18 leader
    slot, +28 slot -> group index, +2C slot count), +1C group array}."""
    m = Fresh()
    g = m.gworld
    d = m.alloc(0x400, 0)
    m.w(d, DEF_VTABLES[spec['def_vtable']])
    m.w(d + 0x174, spec['def_flags'])
    m.w(d + 8, m.cstring('actor_def'))
    p = m.alloc(0x60, 0)
    m.w(p, 0xa25b28)
    m.w(p + 4, d)
    for k, v in enumerate(spec['position'] + spec['direction']):
        m.w(p + 8 + 4 * k, v)
    m.wb(p + 0x51, spec['byte_51'])

    def layout(slot_count):
        lay = m.alloc(0x40, 0)
        data = m.alloc(0x40, 0)
        m.w(lay + 8, data)
        m.w(data + 0x2c, slot_count)
        return lay
    m.layouts = {}
    if spec['params_layout'] is not None:
        lay = layout(spec['params_layout'])
        m.w(p + 0x2c, lay)
        m.layouts[lay] = 'params+2C'
    if spec['def_layout'] is not None:
        lay = layout(spec['def_layout'])
        m.w(d + 0x2f0, lay)
        m.layouts[lay] = 'definition+2F0'
    obj = None
    m.org_component = None
    if spec['create_result']:
        obj = m.alloc(0x180, 0)
        vt = m.alloc(0x200, 0)
        is_actor = spec['is_gactor']
        m.w(vt + 0xb4, m.stub(lambda m: m.ret(0, 1 if is_actor else 0)))
        m.w(obj, vt)
        m.w(obj + 0x14, 0x00070042)
        if spec['organization'] is not None:
            org = spec['organization']
            oc = m.alloc(0x40, 0)
            data = m.alloc(0x40, 0)
            slots = m.alloc(4 * max(1, len(org['slot_groups'])), 0)
            groups = m.alloc(4 * 16, 0)
            m.w(oc + 0x18, data)
            m.w(oc + 0x1c, groups)
            m.w(data + 0x18, org['leader'])
            m.w(data + 0x28, slots)
            m.w(data + 0x2c, len(org['slot_groups']))
            for k, gi in enumerate(org['slot_groups']):
                m.w(slots + 4 * k, gi)
            for gi in org['defined_groups']:
                m.w(groups + 4 * gi, 0x6660000 + gi)
            m.w(obj + 0x7c, oc)
            m.org_component = oc

    def create(m):  # 4F9C10 seam thiscall(GWorld, params) ret 4
        m.record('4F9C10', params=m.arg(0) == p, world=m.u.reg_read(UC_X86_REG_ECX) == g,
                 position=[m.r(p + 8), m.r(p + 0xc), m.r(p + 0x10)])
        m.ret(4, obj or 0)
    m.seam(0x4f9c10, create)

    def first_slot(m):  # 4C2BC0 thiscall(layout)
        m.record('4C2BC0', layout=m.layouts.get(m.u.reg_read(UC_X86_REG_ECX)))
        m.ret(0, spec['first_slot'])
    m.seam(0x4c2bc0, first_slot)

    def slot_test(m):  # 578290(layout, slot, &xy, facing x, facing y) cdecl
        xy = m.arg(2)
        m.record('578290', layout=m.layouts.get(m.arg(0)), slot=m.arg(1), xy=[m.r(xy), m.r(xy + 4)],
                 facing=[m.arg(3), m.arg(4)])
        if spec['slot_xy'] is not None:
            m.w(xy, spec['slot_xy'][0])
            m.w(xy + 4, spec['slot_xy'][1])
        m.ret(0, spec['slot_ok'])
    m.seam(0x578290, slot_test)

    def spawn(m):  # 57AD30 thiscall(org, slot, flag) ret 8
        m.record('57AD30', org=m.u.reg_read(UC_X86_REG_ECX) == m.org_component, slot=m.arg(0), flag=m.arg(1))
        m.ret(8, 0)
    m.seam(0x57ad30, spawn)

    def member_count(m):  # 57A730 thiscall(org)
        m.record('57A730')
        m.ret(0, spec['member_count'])
    m.seam(0x57a730, member_count)
    m.leaf(0x4f5490, 'destroy', 4, 1)

    def log(m):
        m.record('log 404F60', line=m.arg(2), format=m.read_wide(m.arg(4)))
        m.ret(0)
    m.seam(0x404f60, log)

    def assertion(m):
        m.record('assert dialog', expression=m.read_wide(m.arg(0)), line=m.arg(2))
        m.ret(0, spec['assert_result'])
    m.seam(0x855d30, assertion)
    m.wb(0xb3cc43, spec['latch'])
    m.wb(0xb3cc44, spec['latch'])
    m.reset_case()
    m.guard = True
    NATIVE.add(0x4be740)
    entry = {'actor': 0x4fa030, 'organization': 0x4fa150}[spec['entry']]
    r = m.call(entry, g, [p], 4)
    m.guard = False
    return {'returned': None if m.outcome else ('object' if obj and r == obj else r), 'outcome': m.outcome,
            'calls': m.calls, 'latch_43': m.u.mem_read(0xb3cc43, 1)[0], 'latch_44': m.u.mem_read(0xb3cc44, 1)[0],
            'params_after': [m.r(p + 4 * i) for i in range(7)]}


def entry_cases():
    def e(**kw):
        s = dict(entry='organization', def_vtable='ActorData', def_flags=1 << 7, params_layout=None,
                 def_layout=3, first_slot=1, slot_ok=1, slot_xy=[f32(40.0), f32(-10.0)],
                 position=[f32(33.5), f32(-12.75), f32(2.0)], direction=[f32(0.6), f32(-0.8)], byte_51=1,
                 create_result=True, is_gactor=True, latch=0, assert_result=0, member_count=2,
                 organization={'leader': 1, 'slot_groups': [0, 1, 2, 1], 'defined_groups': [0, 1]})
        s.update(kw)
        return s
    return [
        ('actor_ok', e(entry='actor')),
        ('actor_null', e(entry='actor', create_result=False)),
        ('actor_not_gactor', e(entry='actor', is_gactor=False)),
        ('actor_not_gactor_latched', e(entry='actor', is_gactor=False, latch=1)),
        ('actor_not_gactor_dialog_sets_latch', e(entry='actor', is_gactor=False, assert_result=1)),
        ('org_full', e()),
        ('org_params_layout', e(params_layout=2)),
        ('org_layout_empty', e(def_layout=0)),
        ('org_slot_test_fails', e(slot_ok=0)),
        ('org_not_actordata', e(def_vtable='ObjectData')),
        ('org_no_flag7', e(def_flags=0)),
        ('org_create_null', e(create_result=False)),
        ('org_no_component', e(organization=None)),
        ('org_leader_negative', e(organization={'leader': 0xffffffff, 'slot_groups': [0, 1, 2],
                                                'defined_groups': [0, 2]})),
        ('org_leader_undefined', e(organization={'leader': 2, 'slot_groups': [0, 1, 2], 'defined_groups': [0, 1]})),
        ('org_no_members', e(member_count=0)),
        ('org_not_gactor', e(is_gactor=False)),
    ]


def main():
    rows = []
    t = time.time()
    for label, spec in cases():
        res = run_create(spec)
        assert res['outcome'] is None or res['outcome']['kind'] != 'Unexpected', (label, res['outcome'])
        rows.append({'label': label, 'spec': spec, 'result': res})
        print(label, res['returned'], res['outcome'], flush=True)
    entries = []
    for label, spec in entry_cases():
        res = run_entry(spec)
        assert res['outcome'] is None or res['outcome']['kind'] != 'Unexpected', (label, res['outcome'])
        entries.append({'label': label, 'spec': spec, 'result': res})
        print(label, res['returned'], res['outcome'], flush=True)
    doc = {'image': 'AA.exe (executed windows identical in runtime-expanded-v14/AA.exe)',
           'native': sorted(hex(a) for a in NATIVE), 'cases': rows, 'entries': entries, 'visited_instructions': len(VISITED)}
    (HERE / 'native-fresh-creation.json').write_text(json.dumps(doc, indent=1))
    print(len(rows), 'cases %.1fs' % (time.time() - t))


if __name__ == '__main__':
    main()
