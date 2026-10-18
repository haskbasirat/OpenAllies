"""Component virtual leaves of the actor creation (the 4CF480 classes):
InitCreate (+4), InitExisting (+8), PostCreate (+C), +14 and the requirement
(+20) of every component class, run NATIVELY on fixture components (the
class's real vtable at +0, its real Updatable vtable at +8, owner actor at
+4) with their callees seamed and recorded.

Seams (other owners' subsystems): Updatable queue helpers 50AC60 / 50AC00 /
50ACC0 / 50B220, Properties 581420 / 5817A0, Body 561E50 / 565070 /
4CF110 / 561D40, Settlement 586BC0 / 584CF0, Supply 58AE10, Performer
57DCF0 / 57E430, Organization 57C450 / 4D4CF0 / 57B390 / 577F30 / 5709D0,
Mover 576B20, Experience 45DB90 / 5716B0, definition getters with modifier
45D5B0 / 45D730 / 45D4C0 / 45D6C0 (ST0 results), actor state 4D64F0 and
4D1050, zones 516640 (zone ctor inside the native 4D69A0) / 515F40 / 515940,
the GateZoneRadius SVar 4BE0D0 / 4BE0F0, the HQ index 4E2640, message
40C230 / 4B9F40, initial properties 45EED0, city 5644F0, Mind InitExisting
529EC0 / 517C60 (aa-mind), Wall grid 554DD0 / 554330.
Usage: python native_component_leaves.py -> native-component-leaves.json
"""
import json, struct, sys, time
from pathlib import Path
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from actor_machine import ActorMachine, P, VISITED, CTORS, H, f32, nm, Stop
from unicorn.x86_const import *

VT = {0: 0xa3b0e8, 1: 0xa3e280, 2: 0xa3b4d8, 3: 0xa3ce88, '4org': 0xa335a0, '4air': 0xa32710, '4elem': 0xa35810,
      5: 0xa3df40, 6: 0xa3e3dc, 7: 0xa3d434, 8: 0xa3c6f0, 9: 0xa3c690, 10: 0xa3c14c, 11: 0xa400c0, 12: 0xa40198,
      13: 0xa3fac8, 14: 0xa3eea0, 15: 0xa3b950, 16: 0xa3e6d0, 17: 0xa3c974, 18: 0xa3b2a8, 19: 0xa3cd00,
      20: 0xa3bbd4, 21: 0xa3eb58, 22: 0xa3c568, 23: 0xa3af78, 24: 0xa40100, 25: 0xa3cc6c, 26: 0xa3c9e4,
      27: 0xac7ed0, 28: 0xa3e144, 29: 0xa3b6f8, 30: 0xa3ca4c}
UPD = {0: 0xa3b0c0, 1: 0xa3e254, 2: 0xa3b4b0, 3: 0xa3ce5c, 5: 0xa3df18, 6: 0xa3e3b4, 7: 0xa3d40c, 8: 0xa3c6c8,
       9: 0xa3c668, 10: 0xa3c124, 11: 0xa40098, 12: 0xa40170, 13: 0xa3fa9c, 14: 0xa3ee78, 15: 0xa3b924,
       16: 0xa3e6a4, 18: 0xa3b27c, 19: 0xa3ccd8, 20: 0xa3bbac, 21: 0xa3eb30, 22: 0xa3c540, 23: 0xa3af48,
       25: 0xa3cc44, 26: 0xa3c9bc, 27: 0xac7ea8, 28: 0xa3e11c, 29: 0xa3b6d0, 30: 0xa3ca24}
VIRT = {4: ('InitCreate', 1, 4), 8: ('InitExisting', 0, 0), 0xc: ('PostCreate', 2, 8), 0x14: ('vf14', 0, 0),
        0x20: ('Requirement', 0, 0)}
LEAF_NATIVE = {0x438ac0, 0x563fd0, 0x562310, 0x57db90, 0x56f250, 0x568750, 0x58ca40, 0x586f70, 0x575ac0,
               0x571cc0, 0x574610, 0x6a71d0, 0x562290, 0x576700, 0x51d1b0, 0x57e000, 0x57c870, 0x5704d0,
               0x56f280, 0x5687e0, 0x58b0e0, 0x585940, 0x564d80, 0x581f30, 0x5717f0, 0x562e80, 0x5757c0,
               0x566db0, 0x55e600, 0x571cd0, 0x93e8e0, 0x4d1e10, 0x581880, 0x564be0, 0x4b2830, 0x579dd0,
               0x568070, 0x58d3b0, 0x563fe0, 0x580f70, 0x7c53e0, 0x55ea50, 0x5b6830, 0x589090, 0x4c9fa0,
               0x581370, 0x6b2450, 0x7c7840, 0x529c30, 0x561b40, 0x4364c0, 0x4d69a0, 0x4849f0, 0x514cd0,
               0x4bb000, 0x432e90, 0x96d710, 0x85d840, 0x4f02b0, 0x401560, 0x4d1e80, 0x45eed0}


class LeafMachine(ActorMachine):
    pass


def build(c):
    m = LeafMachine(c.get('pc', 'Double53'))
    import actor_machine
    actor_machine.NATIVE |= LEAF_NATIVE
    g = m.gworld
    m.w(g + 0xd4, c.get('rng', 0x2468ace))
    m.w(g + 0xe0, c.get('time', f32(37.5)))
    m.w(g + 0x2c, c.get('version', 0x1a))
    gmap = m.alloc(0x100, 0)
    m.w(gmap + 8, c.get('tile_size', f32(8.0)))
    m.w(gmap + 4, 3)
    m.w(g + 0x30, gmap)
    tm = m.alloc(0x100, 0)
    m.w(tm + 0x74, c.get('thing_manager_74', f32(1.5)))
    m.w(0xb39f58, tm)
    m.w(0xb36528, 0)
    # UnitSupplyEnabled SVar (561B40 static B3D654 latch B3D658 set)
    sv = m.alloc(4, 0)
    m.wb(sv, c.get('unit_supply', 1))
    m.w(0xb3d654, sv)
    m.w(0xb3d658, 1)
    # GateZoneRadius static (58D3B0): latch B3D6CC
    m.w(0xb3d6cc, c.get('gate_latch', 1))
    d = m.alloc(0x700, 0)
    m.labels[d] = 'def'
    for off, v in c.get('def', {}).items():
        if off == '0x4ec_targets':
            # +4EC list {+0 object (its +184 word), +4 next}
            head = 0
            for val in reversed(v):
                obj = m.alloc(0x200, 0)
                m.w(obj + 0x184, val)
                node = m.alloc(8, 0)
                m.w(node, obj)
                m.w(node + 4, head)
                head = node
            m.w(d + 0x4ec, head)
            continue
        off = int(off, 16)
        if isinstance(v, list):
            # a singly linked list {value at +vo, next at +no}
            vo, no, values = v
            head = 0
            for val in reversed(values):
                node = m.alloc(0x40, 0)
                m.w(node + vo, val)
                m.w(node + no, head)
                head = node
            m.w(d + off, head)
        elif off in (0x348, 0x42c, 0x42d, 0x1b0):
            m.wb(d + off, v)
        else:
            m.w(d + off, v)
    a = m.alloc(0x180, 0xa5)
    m.actor = a
    m.labels[a] = 'actor'
    act = c.get('actor', {})
    m.w(a, 0xa273f0)
    m.w(a + 4, d)
    m.w(a + 8, act.get('flags', 0x00800000))
    m.w(a + 0x14, 0x0001002a)
    m.w(a + 0x20, act.get('x', f32(33.5)))
    m.w(a + 0x24, act.get('y', f32(-12.75)))
    m.w(a + 0x28, f32(7.25))
    m.w(a + 0x100, act.get('low', 0))
    m.w(a + 0x104, act.get('high', 0))
    m.w(a + 0x138, act.get('w138', 0))
    m.w(a + 0x148, 0)
    m.w(a + 0x14c, 0)
    m.w(a + 0x150, 9)
    m.w(a + 0xe8, 0)
    for s in range(31):
        m.w(a + 0x60 + 4 * s, 0)
    for s in act.get('components', []):
        o = m.alloc(0x40, 0)
        m.labels[o] = 'other%d' % s
        if s == 3:
            m.w(o + 0x24, act.get('mover_24', f32(0.25)))
        m.w(a + 0x60 + 4 * s, o)
    if act.get('owner'):
        k = m.alloc(0x400, 0)
        m.labels[k] = 'kingdom'
        if act['owner'].get('hq_slots') is not None:
            hq = m.alloc(0x60, 0)
            m.w(hq + 0x40, act['owner']['hq_slots'])
            m.w(k + 0x234, hq)
        m.w(a + 0xe8, k)
    kind = c['kind']
    size = CTORS[kind][0]
    comp = m.alloc(size, 0xa5)
    m.comp = comp
    m.labels[comp] = 'comp'
    m.labels[comp + 8] = 'updatable'
    m.w(comp, VT[kind])
    m.w(comp + 4, a)
    if kind in UPD:
        m.w(comp + 8, UPD[kind])
        m.w(comp + 0xc, 0)
    defaults = {10: {'0x18': 0, '0x38': 0}, 7: {'0x2c': 0, '0x70': 0}}
    for off, v in {**defaults.get(kind, {}), **c.get('comp', {})}.items():
        off = int(off, 16)
        if isinstance(v, list):
            vo, no, values = v
            head = 0
            for val in reversed(values):
                node = m.alloc(0x40, 0xa5)
                m.w(node + vo, val)
                m.w(node + no, head)
                head = node
            m.w(comp + off, head)
        elif isinstance(v, dict) and 'byte' in v:
            m.wb(comp + off, v['byte'])
        else:
            m.w(comp + off, v)
    if c.get('members'):
        arr = m.alloc(16, 0)
        for k in (0, 2):
            mem = m.alloc(0x180, 0)
            el = m.alloc(0x40, 0)
            m.labels[el] = 'element%d' % k
            m.w(mem + 0x80, el)
            m.w(arr + 4 * k, mem)
        m.w(comp + 0x28, arr)
    slot = 4 if isinstance(kind, str) else kind
    m.w(a + 0x60 + 4 * slot, comp)
    p = m.alloc(0x60, 0)
    m.labels[p] = 'params'
    m.w(p, 0xa25b28)
    pr = c.get('params', {})
    m.w(p + 0x34, pr.get('w34', 0xffffffff))
    m.w(p + 0x38, pr.get('mode', 0))
    m.w(p + 0x40, pr.get('w40', 0))
    m.w(p + 0x48, pr.get('w48', 0))
    m.params = 0 if pr.get('null') else p
    install(m, c)
    return m


def install(m, c):
    res = c.get('results', {})

    def lab(v):
        return m.label(v)

    def queue(name):
        def h(m):
            m.record(name, ecx=lab(m.ecx()), data=m.arg(0))
            m.ret(4)
        return h
    m.seam(0x50ac60, queue('50AC60'))
    m.seam(0x50ac00, queue('50AC00'))

    def after(m):
        m.record('50ACC0', ecx=lab(m.ecx()), delay=m.arg(0), data=m.arg(1))
        m.ret(8)
    m.seam(0x50acc0, after)

    def has(m):
        m.record('50B220', ecx=lab(m.ecx()), queue=m.arg(0), data=m.arg(1))
        m.ret(8, res.get('50B220', 0))
    m.seam(0x50b220, has)
    m.leaf(0x581420, '581420', 4, 1, extra=lambda m, e, a: e.update(ecx=lab(m.ecx())))
    m.leaf(0x5817a0, '5817A0', 0xc, 3, extra=lambda m, e, a: e.update(ecx=lab(m.ecx()), args=[a[0], a[1], lab(a[2])]))
    m.leaf(0x561e50, '561E50', 0x10, 4, extra=lambda m, e, a: e.update(ecx=lab(m.ecx())))
    m.leaf(0x565070, '565070', 0, 0, ret=res.get('565070', 0))
    m.leaf(0x4cf110, '4CF110', 0, 0, ret=res.get('4CF110', 0x4cf110))
    m.leaf(0x561d40, '561D40', 0xc, 3)
    m.leaf(0x586bc0, '586BC0', 4, 1, ret=res.get('586BC0', 1))
    m.leaf(0x584cf0, '584CF0', 0, 0)
    m.leaf(0x58ae10, '58AE10', 4, 1)
    m.leaf(0x57dcf0, '57DCF0', 4, 1)
    m.leaf(0x57e430, '57E430', 0, 0, extra=lambda m, e, a: e.update(ecx=lab(m.ecx())))
    m.leaf(0x57c450, '57C450', 4, 1)
    m.leaf(0x4d4cf0, '4D4CF0', 0, 0, extra=lambda m, e, a: e.update(ecx=lab(m.ecx())))
    m.leaf(0x57b390, '57B390', 0, 0)
    m.leaf(0x577f30, '577F30', 4, 1)
    m.leaf(0x5709d0, '5709D0', 0, 0, extra=lambda m, e, a: e.update(ecx=lab(m.ecx())))
    m.leaf(0x576b20, '576B20', 8, 2, extra=lambda m, e, a: e.update(ecx=lab(m.ecx())))
    m.leaf(0x45db90, '45DB90', 4, 1, ret=res.get('45DB90', 2))
    m.leaf(0x5716b0, '5716B0', 4, 1)
    m.leaf(0x4d64f0, '4D64F0', 8, 2, extra=lambda m, e, a: e.update(ecx=lab(m.ecx()), args=[a[0], a[1] & 0xff]))
    m.leaf(0x4d1050, '4D1050', 0, 0, ret=res.get('4D1050', 0), extra=lambda m, e, a: e.update(ecx=lab(m.ecx())))
    for addr, name, key in ((0x45d5b0, '45D5B0', 'element_range'), (0x45d730, '45D730', 'target_health'),
                            (0x45d4c0, '45D4C0', 'organization_range'), (0x45d6c0, '45D6C0', 'defender_range')):
        def getter(m, name=name, key=key):
            m.record(name, ecx=lab(m.ecx()), arg=m.arg(0))
            m.ret_float(res.get(key, f32(6.5)), 4)
        m.seam(addr, getter)

    def gate_lookup(m):  # 4BE0D0 thiscall(B3D6C8; B39AA8, &name) ret 8
        m.record('4BE0D0')
        m.ret(8)
    m.seam(0x4be0d0, gate_lookup)
    m.seam(0x4be0f0, lambda m: (m.record('4BE0F0'), m.ret_float(res.get('gate_radius', f32(12.0)), 0)))

    def zone_ctor(m):
        m.record('zone ctor', actor=lab(m.arg(0)), mask=m.arg(1) & 0xffff)
        m.ret(8, m.ecx())
    m.seam(0x516640, zone_ctor)

    def zone_place(m):
        idx = None
        zones = m.r(m.actor + 0x148)
        for k in range(m.r(m.actor + 0x14c)):
            if zones and m.r(zones + 4 * k) == m.ecx() and m.ecx():
                idx = k
        m.record('515F40', zone=idx, xyz=[m.arg(0), m.arg(1), m.arg(2)])
        m.ret(0xc)
    m.seam(0x515f40, zone_place)

    def zone_bit(m):
        idx = None
        zones = m.r(m.actor + 0x148)
        for k in range(m.r(m.actor + 0x14c)):
            if zones and m.r(zones + 4 * k) == m.ecx() and m.ecx():
                idx = k
        m.record('515940', zone=idx, args=[m.arg(0), m.arg(1)])
        m.ret(8)
    m.seam(0x515940, zone_bit)
    m.leaf(0x4e2640, '4E2640', 0, 0, ret=res.get('hq_index', 3), extra=lambda m, e, a: e.update(ecx=lab(m.ecx())))
    m.leaf(0x40c230, '40C230', 0, 4, extra=lambda m, e, a: e.update(args=[a[0], a[1], lab(a[2]), lab(a[3])]))

    def actor_name(m):  # GActor vf+24 4D99D0 thiscall(actor; &out) ret 4
        m.record('actor name', ecx=lab(m.ecx()))
        m.w(m.arg(0), m.cstring('actor name'))
        m.ret(4, m.arg(0))
    m.seam(0x4d99d0, actor_name)
    m.leaf(0x5644f0, '5644F0', 0, 3, extra=lambda m, e, a: e.update(args=[lab(a[0]), a[1], lab(a[2])]))
    m.leaf(0x529ec0, 'mind 529EC0', 0, 0, ret=res.get('mind', 1))
    m.leaf(0x517c60, 'mind 517C60', 0, 0, ret=res.get('mind', 1))
    m.leaf(0x58cc40, 'wall grid 58CC40', 0, 0, ret=1)
    m.seam(0x977111, lambda m: m.throw('LibmError'))


def run(c):
    m = build(c)
    m.calls, m.outcome = [], None
    m.null_accesses.clear()
    off = c['virtual']
    name, nargs, pop = VIRT[off]
    leaf = m.r(m.r(m.comp) + off)
    m.guard = True
    # The leaf itself counts as native (the vtable entry under test).
    import actor_machine
    actor_machine.NATIVE.add(leaf)
    args = {4: [m.params], 8: [], 0xc: [m.params, c.get('flag', 0)], 0x14: [], 0x20: []}[off]
    r = m.call(leaf, m.comp, args, pop)
    m.guard = False
    if not m.outcome and off != 0x20:
        r &= 0xff
    out = {'leaf': hex(leaf), 'returned': None if m.outcome else r, 'outcome': m.outcome, 'calls': m.calls,
           'rng_after': m.r(m.gworld + 0xd4), 'unexpected': m.unexpected}
    size = CTORS[c['kind']][0]
    out['comp'] = [m.r(m.comp + 4 * i) for i in range(size // 4)]
    if c['kind'] == 10:
        slots, node = [], m.r(m.comp + 0x18)
        while node and len(slots) < 16:
            slots.append(m.r(node + 8))
            node = m.r(node + 0x10)
        out['denizen_slot_health'] = slots
    out['actor'] = {'low': m.r(m.actor + 0x100), 'high': m.r(m.actor + 0x104),
                    'zones': m.r(m.actor + 0x14c), 'cap': m.r(m.actor + 0x150)}
    return out


def cases():
    out = []
    add = lambda label, **kw: out.append((label, kw))
    allk = list(VT)
    for k in allk:
        add('req_%s' % k, kind=k, virtual=0x20)
        add('postcreate_%s' % k, kind=k, virtual=0xc)
    add('req_13_owner_4d1050_true', kind=13, virtual=0x20, results={'4D1050': 1})
    # InitCreate (+4)
    for k in (1, 2, 3, '4org', '4air', '4elem', 13, 16, 17, 18, 21, 23, 24, 27, 28):
        add('ic_true_%s' % k, kind=k, virtual=4)
    for k in (6, 7, 8, 11, 15, 22, 25, 29):
        add('ic_periodic_%s' % k, kind=k, virtual=4)
    add('ic_body_plain', kind=0, virtual=4, def_={})
    add('ic_body_construction', kind=0, virtual=4, actor={'low': 1 << 21})
    add('ic_body_supply', kind=0, virtual=4, **{'def': {'0x2a8': f32(0.5)}})
    add('ic_body_supply_scheduled', kind=0, virtual=4, results={'50B220': 1}, **{'def': {'0x2a8': f32(0.5)}})
    add('ic_body_supply_org', kind=0, virtual=4, actor={'components': [7]}, **{'def': {'0x2a8': f32(0.5)}})
    add('ic_body_supply_w40', kind=0, virtual=4, params={'w40': 5}, **{'def': {'0x2a8': f32(0.5)}})
    add('ic_body_supply_w48', kind=0, virtual=4, params={'w48': 5}, **{'def': {'0x2a8': f32(0.5)}})
    add('ic_body_supply_null_params', kind=0, virtual=4, params={'null': 1}, **{'def': {'0x2a8': f32(0.5)}})
    add('ic_body_supply_svar_off', kind=0, virtual=4, unit_supply=0, **{'def': {'0x2a8': f32(0.5)}})
    add('ic_performer_no_props', kind=5, virtual=4)
    add('ic_performer_props', kind=5, virtual=4, actor={'components': [6]})
    add('ic_defender_props', kind=20, virtual=4, actor={'components': [6]})
    add('ic_economy_off', kind=9, virtual=4)
    add('ic_economy_on', kind=9, virtual=4, **{'def': {'0x348': 1}})
    add('ic_denizen_basic', kind=10, virtual=4, **{'def': {'0x3a4': f32(2.5)}})
    add('ic_denizen_health_list', kind=10, virtual=4, comp={'0x18': [4, 0x10, [0x111, 0x222]]},
        **{'def': {'0x364': f32(1.0), '0x3a4': f32(-0.0)}})
    add('ic_denizen_mode4', kind=10, virtual=4, params={'mode': 4}, comp={'0x18': [4, 0x10, [0x111]]},
        **{'def': {'0x35c': 1}})
    add('ic_denizen_construction', kind=10, virtual=4, actor={'low': 1 << 21}, comp={'0x18': [4, 0x10, [0x111]]})
    add('ic_denizen_null_params', kind=10, virtual=4, params={'null': 1}, comp={'0x18': [4, 0x10, [0x111]]})
    add('ic_wall_off', kind=12, virtual=4, **{'def': {'0x42c': 1}})
    add('ic_wall_on', kind=12, virtual=4, **{'def': {'0x42c': 1, '0x42d': 1}})
    add('ic_settlement', kind=14, virtual=4, comp={'0x28': 7})
    add('ic_settlement_fail', kind=14, virtual=4, comp={'0x28': 7}, results={'586BC0': 0})
    add('ic_mobile_plain', kind=19, virtual=4)
    add('ic_mobile_534', kind=19, virtual=4, **{'def': {'0x534': 1}})
    add('ic_mobile_state26', kind=19, virtual=4, actor={'low': 1 << 26})
    add('ic_mobile_state25', kind=19, virtual=4, actor={'low': 1 << 25})
    add('ic_flare', kind=26, virtual=4)
    add('ic_hq_assigned', kind=30, virtual=4, comp={'0x10': 2}, actor={'owner': {'hq_slots': 4}})
    add('ic_hq_allocates', kind=30, virtual=4, comp={'0x10': 0xffffffff}, actor={'owner': {'hq_slots': 4}})
    add('ic_hq_no_badge', kind=30, virtual=4, comp={'0x10': 0xffffffff}, actor={'owner': {'hq_slots': 4}},
        results={'hq_index': 0xffffffff})
    add('ic_hq_no_owner', kind=30, virtual=4, comp={'0x10': 0xffffffff})
    add('ic_hq_owner_no_slots', kind=30, virtual=4, comp={'0x10': 0xffffffff}, actor={'owner': {}})
    # InitExisting (+8)
    for k in (1, 2, 6, 13, 21, 24, 28, 29, 30):
        add('ie_true_%s' % k, kind=k, virtual=8)
    for v in (0x15, 0x16):
        add('ie_mover_flag_v%x' % v, kind=3, virtual=8, version=v, comp={'0x54': 0x00010000})
        add('ie_mover_scheduled_v%x' % v, kind=3, virtual=8, version=v, comp={'0x54': 0x00010000, '0xc': 1})
        add('ie_construction_v%x' % v, kind=15, virtual=8, version=v)
        add('ie_construction_sched_v%x' % v, kind=15, virtual=8, version=v, comp={'0xc': 1})
        add('ie_flare_v%x' % v, kind=26, virtual=8, version=v)
        add('ie_element_v%x' % v, kind=8, virtual=8, version=v)
        add('ie_org_v%x' % v, kind=7, virtual=8, version=v, comp={'0x2c': 0, '0x70': 0})
        add('ie_settlement_v%x' % v, kind=14, virtual=8, version=v)
        add('ie_denizen_v%x' % v, kind=10, virtual=8, version=v, comp={'0x38': 0x10},
            **{'def': {'0x34c': [8, 0xc, [0x2, 0x40]], '0x354': [4, 0x1c, [0x100]], '0x364': f32(2.0)}})
    add('ie_mover_noflag', kind=3, virtual=8, comp={'0x54': 0})
    add('ie_org_layout_set', kind=7, virtual=8, comp={'0x2c': 0x777, '0x70': 0})
    add('ie_economy', kind=9, virtual=8, **{'def': {'0x348': 1}})
    add('ie_economy_sched', kind=9, virtual=8, comp={'0xc': 1}, **{'def': {'0x348': 1}})
    add('ie_supply', kind=11, virtual=8)
    add('ie_supply_sched_zones', kind=11, virtual=8, comp={'0xc': 1})
    add('ie_recruit', kind=16, virtual=8, comp={'0x10': {'byte': 0}, '0x11': {'byte': 0}})
    add('ie_recruit_list', kind=16, virtual=8, comp={'0x10': {'byte': 0}, '0x11': {'byte': 0}},
        **{'def': {'0x4ec_targets': [0, 0x100, 0]}})
    add('ie_recruit_only_184', kind=16, virtual=8, comp={'0x10': {'byte': 0}, '0x11': {'byte': 0}},
        **{'def': {'0x4ec_targets': [0x100]}})
    add('ie_experience_same', kind=17, virtual=8, comp={'0x8': 500, '0xc': 2})
    add('ie_experience_changed', kind=17, virtual=8, comp={'0x8': 500, '0xc': 1})
    add('ie_builder', kind=18, virtual=8)
    add('ie_mobile_no', kind=19, virtual=8)
    add('ie_mobile_yes', kind=19, virtual=8, results={'4D1050': 1}, **{'def': {'0x554': f32(4.5)}})
    add('ie_defender_org', kind=20, virtual=8, **{'def': {'0x174': 1 << 7}})
    add('ie_defender_plain', kind=20, virtual=8, actor={'components': [23, 5]})
    add('ie_aircraft_slow', kind=23, virtual=8, actor={'components': [3], 'mover_24': f32(0.01)})
    add('ie_aircraft_fast', kind=23, virtual=8, actor={'components': [3], 'mover_24': f32(3.0)})
    add('ie_metagame', kind=27, virtual=8)
    add('ie_performer', kind=5, virtual=8)
    add('ie_mind_elem', kind='4elem', virtual=8)
    add('ie_mind_org_false', kind='4org', virtual=8, results={'mind': 0})
    add('ie_mind_air', kind='4air', virtual=8)
    add('ie_body_full', kind=0, virtual=8, comp={'0x10': f32(50.0), '0x14': f32(50.0)})
    add('ie_body_damaged', kind=0, virtual=8, comp={'0x10': f32(20.0), '0x14': f32(50.0)})
    add('ie_body_construction', kind=0, virtual=8, actor={'low': 1 << 21}, comp={'0x10': f32(20.0), '0x14': f32(50.0)})
    add('ie_body_construction_comp', kind=0, virtual=8, actor={'components': [15]},
        comp={'0x10': f32(50.0), '0x14': f32(50.0)}, results={'565070': 0x565070})
    add('ie_wall', kind=12, virtual=8)
    # PostCreate (+C) with arguments
    add('pc_properties', kind=6, virtual=0xc, **{'def': {'0x1d4': [0, 4, [0x31, 0x32, 0x33]]}})
    add('pc_properties_flag1', kind=6, virtual=0xc, flag=1, **{'def': {'0x1d4': [0, 4, [0x31]]}})
    add('pc_properties_empty', kind=6, virtual=0xc)
    add('pc_city_flag0', kind=29, virtual=0xc, params={'w34': 9})
    add('pc_city_flag1', kind=29, virtual=0xc, flag=1, params={'w34': 9})
    # +14
    for k in allk:
        if k not in (7, 10, 12, 22, 29):
            add('vf14_%s' % k, kind=k, virtual=0x14)
    add('vf14_org', kind=7, virtual=0x14, comp={'0x28': 0, '0x2c': 0})
    add('vf14_org_members', kind=7, virtual=0x14, comp={'0x28': 'members', '0x2c': 3})
    add('vf14_denizen_zero', kind=10, virtual=0x14, comp={'0x38': 0})
    add('vf14_denizen_range', kind=10, virtual=0x14, comp={'0x38': 0x20}, **{'def': {'0x364': f32(3.5)}})
    add('vf14_denizen_range_nobit', kind=10, virtual=0x14, comp={'0x38': 0}, **{'def': {'0x364': f32(3.5)}})
    add('vf14_denizen_dead', kind=10, virtual=0x14, actor={'flags': 0x00800001}, **{'def': {'0x364': f32(3.5)}})
    add('vf14_wall_off', kind=12, virtual=0x14)
    add('vf14_wall_gate', kind=12, virtual=0x14, **{'def': {'0x42c': 1, '0x42d': 1}})
    add('vf14_wall_gate_unlatched', kind=12, virtual=0x14, gate_latch=0, **{'def': {'0x42c': 1, '0x42d': 1}})
    add('vf14_city', kind=29, virtual=0x14, **{'def': {'0x5b4': f32(16.0)}})
    # Precision variants of the leaves with x87 arithmetic.
    for pcn in ('Single24', 'Extended64'):
        add('ie_mobile_odd_' + pcn, kind=19, virtual=8, pc=pcn, time=f32(1234567.1), results={'4D1050': 1},
            **{'def': {'0x554': f32(0.3333333)}})
        add('ie_settlement_odd_' + pcn, kind=14, virtual=8, pc=pcn, tile_size=f32(7.3), thing_manager_74=f32(1e-3))
    add('ie_mobile_odd_Double53', kind=19, virtual=8, time=f32(1234567.1), results={'4D1050': 1},
        **{'def': {'0x554': f32(0.3333333)}})
    add('ie_settlement_odd_Double53', kind=14, virtual=8, tile_size=f32(7.3), thing_manager_74=f32(1e-3))
    return out


def main():
    rows = []
    t = time.time()
    for label, c in cases():
        if 'def_' in c:
            c.setdefault('def', c.pop('def_'))
        if c.get('comp', {}).get('0x28') == 'members':
            c['comp']['0x28'] = 0
            c['members'] = True
        res = run(c)
        assert res['outcome'] is None, (label, res['outcome'], res['unexpected'])
        rows.append({'label': label, 'case': c, 'result': res})
        print(label, res['leaf'], res['returned'], len(res['calls']), flush=True)
    doc = {'image': 'AA.exe (executed windows identical in runtime-expanded-v14/AA.exe)',
           'harness_sha256': {n: H((HERE / n).read_bytes()) for n in ('actor_machine.py', 'native_component_leaves.py')},
           'base_machine_sha256': H(Path(nm.__file__).read_bytes()), 'cases': rows,
           'visited_instructions': len(VISITED)}
    (HERE / 'native-component-leaves.json').write_text(json.dumps(doc, indent=1))
    print(len(rows), 'cases %.1fs' % (time.time() - t))


if __name__ == '__main__':
    main()
