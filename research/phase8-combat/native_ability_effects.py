"""Native oracle for ability-definition effect loading (round 4).

Original code executed in Unicorn (byte-pinned like every phase8 oracle):
* enum registrations: effect types 497AF0, FXC types 497A40 (add 4976B0),
  modifier types 4C1060 and attributes 4C11E0 (add 4C0F50), heal types inside
  the table constructor 4CC1D0 (add 4CAC40). The add functions are recorded
  seams (their map insertion needs the CString heap manager).
* effect construction 4CA880(kind) and every kind's LoadData (vtable +10).
* modifier loading 4C4030 (4FE840 construction, ADD 4FEC70, PERIODIC_DAMAGE
  501350, FLAG, RESIST_DAMAGE LoadData and the ADD-amount-0 rule).
* EnchantmentProperty LoadData 4C4F90 (base 4C42D0 is a recorded seam) with
  the permanent-VFX search 428A30/428360 over FXC objects, the VFX lifetime
  field written by 494F30, and CreateVFX 42C5E0's +64 duration byte.
Seams (recorded, never executed): TGI node getters 5B54D0/5B5130/5B5380/
5B4AE0/5B4410, target-filter parse 4C5BF0, DataManager find 48D500 and
4BDB70, enum map find 497220 (ASCII case-insensitive per 405030/96CAD6),
473CB0 cost list, 47D8D0 property ctor, 4B4E60 registration, fatal 410C40
(stops the run as the native exception), logs 404F60/404ED0, heap.
"""
from pathlib import Path
import struct
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import harness as h  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX, UC_X86_REG_ESP, UC_X86_REG_EIP  # noqa: E402

WORLD_DATA = 0xB36530


def f32(v):
    return struct.unpack('<I', struct.pack('<f', v))[0]


class Oracle(h.Machine):
    def __init__(self):
        super().__init__('Double53')
        self.u.mem_map(0, 0x1000)
        self.records = []
        self.error = None
        self.nodes = {}
        self.enums = {}
        self.registries = {}
        self.hooks[0x6F7810] = lambda m: m.ret(0, m.alloc(max(m.arg(0), 4)))
        self.hooks[0x6F7430] = lambda m: m.ret(0)
        self.hooks[0x410C40] = self._fatal
        self.hooks[0x404F60] = self._log
        self.hooks[0x404ED0] = self._log
        self.hooks[0x5B54D0] = self._string
        self.hooks[0x5B5130] = lambda m: m.ret_f32(4, m._field(m.arg(0)))
        self.hooks[0x5B4AE0] = lambda m: m.ret_f32(4, m._field(m.arg(0)))
        self.hooks[0x5B5380] = lambda m: m.ret(4, int(m._field(m.arg(0))))
        self.hooks[0x5B4410] = self._child
        self.hooks[0x4C5BF0] = self._filter
        self.hooks[0x48D500] = self._find
        self.hooks[0x4BDB70] = self._find
        self.hooks[0x497220] = self._map_find
        self.hooks[0x473CB0] = self._costs
        self.hooks[0x47D8D0] = self._property_ctor
        self.hooks[0x4C42D0] = self._property_base
        self.hooks[0x4B4E60] = self._register
        self.manager = self.alloc(0x10)
        self.nil = self.alloc(0x20)
        self.w(self.nil, self.manager)
        self.w(self.nil + 0xC, 1000)
        self.w(self.manager, self.vtable({0x10: lambda m: m.ret(0, m.reg(UC_X86_REG_ECX)),
                                          0xC: lambda m: m.ret(0, m.nil)}))
        self.hooks[0x856370] = lambda m: m.ret(0, m.manager)
        self.hooks[0x405B80] = self._assign
        self.world = self.alloc(0x1000)
        self.w(WORLD_DATA, self.world)
        for offset in (0x3A4, 0x3AC, 0x464, 0xC):
            self.w(self.world + offset, self.alloc(0x40))
            self.registries[self.r(self.world + offset)] = offset

    def alloc(self, size, align=16):
        ptr = super().alloc(size, align)
        self.u.mem_write(ptr, bytes(size))
        return ptr

    def wide(self, p):
        out = bytearray()
        while bytes(self.u.mem_read(p, 2)) != b'\0\0':
            out += bytes(self.u.mem_read(p, 2))
            p += 2
        return out.decode('utf-16le')

    def cstring(self, text):
        raw = text.encode('utf-16le')
        header = self.alloc(0x10 + len(raw) + 2)
        self.w(header, self.manager)
        self.w(header + 4, len(text))
        self.w(header + 8, len(text))
        self.w(header + 0xC, 1000)
        self.u.mem_write(header + 0x10, raw + b'\0\0')
        cell = self.alloc(4)
        self.w(cell, header + 0x10)
        return cell

    def node(self, fields, children=None):
        handle = self.alloc(0x60)
        self.nodes[handle] = ({k.lower(): v for k, v in fields.items()},
                              {k.lower(): v for k, v in (children or {}).items()})
        return handle

    def _field(self, name_ptr):
        node = self.reg(UC_X86_REG_ECX)
        name = self.wide(name_ptr)
        fields = self.nodes[node][0]
        self.records.append({'get': name})
        if name.lower() not in fields:
            raise KeyError(f'field {name} not provided')
        return fields[name.lower()]

    def _string(self, m):
        m.ret(4, m.cstring(m._field(m.arg(0))))

    def _child(self, m):
        node = m.reg(UC_X86_REG_ECX)
        name = m.wide(m.arg(0))
        child = m.nodes[node][1].get(name.lower(), 0)
        m.records.append({'child': name, 'required': m.arg(1), 'present': bool(child)})
        m.ret(8, child)

    def _filter(self, m):
        target = m.reg(UC_X86_REG_ECX)
        text = m.wide(m.r(m.arg(0)))
        m.records.append({'filter': text, 'offset': target - m.object})
        m.w(target, 0x11111111)
        m.w(target + 4, 0x22222222)
        m.ret(4)

    def _find(self, m):
        registry = m.reg(UC_X86_REG_ECX)
        name = m.wide(m.arg(0))
        offset = m.registries.get(registry)
        known = m.known.get(offset, {})
        value = known.get(name.lower(), 0)
        m.records.append({'find': hex(offset) if offset is not None else hex(registry), 'name': name,
                          'found': bool(value)})
        m.ret(4, value)

    def _map_find(self, m):
        table = m.reg(UC_X86_REG_ECX)
        key = m.wide(m.r(m.arg(0)))
        if table - 0x40 in m.nodes:
            # TGI node field table (node+40): presence test of an authored field.
            present = key.lower() in m.nodes[table - 0x40][0]
            m.records.append({'field_present': key, 'present': present})
            if not present:
                return m.ret(4, 0)
            entry = m.alloc(8)
            m.w(entry, 1)
            node = m.alloc(8)
            m.w(node, entry)
            return m.ret(4, node)
        entries = m.maps[table]
        m.records.append({'enum_find': hex(table), 'key': key})
        for value, name in entries:
            if name.lower() == key.lower():
                entry = m.alloc(8)
                m.w(entry, value)
                node = m.alloc(8)
                m.w(node, entry)
                return m.ret(4, node)
        return m.ret(4, 0)

    def _costs(self, m):
        m.records.append({'costs': m.arg(0) - m.object, 'node': m.arg(1)})
        m.ret(0)

    def _property_ctor(self, m):
        prop = m.reg(UC_X86_REG_ECX)
        m.records.append({'property_ctor': m.wide(m.arg(0))})
        m.w(prop, 0xA0C834)
        m.ret(4, prop)

    def _property_base(self, m):
        m.records.append({'property_base_load': m.arg(0)})
        m.ret(4)

    def _register(self, m):
        m.records.append({'register': hex(m.registries.get(m.reg(UC_X86_REG_ECX), 0))})
        m.ret(4)

    def _assign(self, m):
        dest = m.reg(UC_X86_REG_ECX)
        m.w(dest, m.r(m.arg(0)))
        m.ret(4, dest)

    def _fatal(self, m):
        m.error = m.wide(m.arg(0))
        m.u.emu_stop()

    def _log(self, m):
        m.records.append({'log': True})
        m.ret(0)

    def run(self, address, ecx, args, pop):
        self.error = None
        sp = self.STACK - 0x200
        self.w(sp, self.STOP)
        for i, value in enumerate(args):
            self.w(sp + 4 + 4 * i, value)
        self.u.reg_write(UC_X86_REG_ESP, sp)
        self.u.reg_write(UC_X86_REG_ECX, ecx)
        self.u.emu_start(address, self.STOP, count=200000)
        if self.error is None:
            assert self.reg(UC_X86_REG_EIP) == self.STOP, hex(self.reg(UC_X86_REG_EIP))
            assert self.reg(UC_X86_REG_ESP) == sp + 4 + pop
        return self.error


def registrations(m):
    out = {}
    for name, function, add, table in (('effect', 0x497AF0, 0x4976B0, None), ('fxc', 0x497A40, 0x4976B0, None),
                                       ('modifier', 0x4C1060, 0x4C0F50, None),
                                       ('attribute', 0x4C11E0, 0x4C0F50, None),
                                       ('flag', 0x4C1110, 0x4C0F50, None)):
        rows = []

        def record(mm, rows=rows):
            rows.append([mm.arg(0), mm.wide(mm.arg(1)), mm.arg(2)])
            mm.ret(12)
        m.hooks[add] = record
        m.run(function, m.alloc(0x40), [], 0)
        del m.hooks[add]
        out[name] = rows
    rows = []

    def heal(mm, rows=rows):
        rows.append([mm.arg(0), mm.wide(mm.arg(1)), mm.arg(2)])
        mm.ret(12)
    m.hooks[0x4CAC40] = heal
    m.run(0x4CC1D0, m.alloc(0x40), [1], 4)
    del m.hooks[0x4CAC40]
    out['heal'] = rows
    return out


def effect_case(m, kind, fields, children=None, known=None, name=''):
    m.records = []
    m.known = known or {}
    obj_ptr = m.run(0x4CA880, 0, [kind], 0)
    assert obj_ptr is None
    from unicorn.x86_const import UC_X86_REG_EAX
    obj = m.reg(UC_X86_REG_EAX)
    m.object = obj
    words_before = [m.r(obj + 4 * i) for i in range(10)]
    vtable = m.r(obj)
    load = m.r(vtable + 0x10)
    node = m.node(fields, children)
    error = m.run(load, obj, [node], 4)
    words = [m.r(obj + 4 * i) for i in range(10)]
    return {'name': name, 'kind': kind, 'fields': fields, 'error': error, 'vtable': hex(vtable), 'load': hex(load),
            'constructed': words_before, 'words': words, 'hide_byte': m.rb(obj + 0x10), 'records': m.records}


def modifier_case(m, fields, known=None, name=''):
    """4C4030 on a property object (+8 IDS pointer, +1C modifier list)."""
    m.records = []
    m.known = known or {}
    prop = m.alloc(0x80)
    m.object = prop
    m.w(prop + 8, m.r(m.cstring('test_property')))
    node = m.node(fields)
    error = m.run(0x4C4030, prop, [node], 4)
    head = m.r(prop + 0x1C)
    out = {'name': name, 'fields': fields, 'error': error, 'records': m.records, 'added': bool(head)}
    if head:
        mod = m.r(head)
        out['modifier'] = [m.r(mod + 4 * i) for i in range(6)]
        out['vtable'] = hex(m.r(mod))
    return out


def vfx_case(m, permanent, duration, name=''):
    """494F30 VFX LoadData: +24 lifetime kind and +28 duration."""
    m.records = []
    vfx = m.alloc(0x80)
    vt = m.vtable({0x6C: lambda mm: mm.ret(0, mm.model_duration), 0x68: lambda mm: mm.ret(0, 0)})
    m.w(vfx, vt)
    m.hooks[0x4BB310] = lambda mm: mm.ret(4)
    node = m.node({'permanent': permanent, 'duration': duration, 'critical': 1, 'attached_scale': 0,
                   'never_discard_temporary': 0, 'flash_period': 0, 'floating_ui_vfx': 0, 'cutscene': 0,
                   'nightonly': 0, 'constant_screen_size': 0, 'show_in_cutscene': 1, 'force_real_time': 0})
    m.model_duration = 0
    error = m.run(0x494F30, vfx, [node], 4)
    return {'name': name, 'permanent': permanent, 'duration': duration, 'error': error,
            'lifetime': m.r(vfx + 0x24), 'cutscene_byte': m.rb(vfx + 0x39)}


def permanent_case(m, fxcs, name=''):
    """428A30 over a list of fake FXC objects: (is_create, vfx_lifetime, duration_byte, children)."""
    vt_create = 0x9FC24C

    def build(spec):
        head = 0
        for item in reversed(spec):
            fxc = m.alloc(0x80)
            if item['create']:
                m.w(fxc, vt_create)
                vfx = m.alloc(0x40)
                m.w(vfx + 0x24, item['lifetime'])
                m.w(fxc + 0x3C, vfx)
                m.wb(fxc + 0x64, item['duration_nonzero'])
            else:
                m.w(fxc, m.vtable({0x24: lambda mm: mm.ret(0, 0), 0x20: lambda mm: mm.ret(0, 0)}))
            m.w(fxc + 8, build(item.get('immediate', [])))
            m.w(fxc + 0x10, build(item.get('chain', [])))
            node = m.alloc(8)
            m.w(node, fxc)
            m.w(node + 4, head)
            head = node
        return head
    holder = m.alloc(8)
    m.w(holder, build(fxcs))
    from unicorn.x86_const import UC_X86_REG_EAX
    m.run(0x428A30, holder, [], 0)
    return {'name': name, 'fxcs': fxcs, 'result': m.reg(UC_X86_REG_EAX) & 0xFF}


def create_vfx_case(m, duration, name=''):
    """42C5E0: +3C VFX, +64 = duration != 0 (base 42C1B0 and lookups are seams)."""
    m.records = []
    fxc = m.alloc(0x80)
    m.w(fxc, 0x9FC24C)
    vfx = m.alloc(0x40)
    m.known = {0xC: {'smoke': vfx}}
    m.hooks[0x42C1B0] = lambda mm: mm.ret(8)
    m.hooks[0x42C070] = lambda mm: mm.ret(4, 0)
    m.maps[0xB3B0CC] = [[0, 'SELF']]
    node = m.node({'vfx_ids': 'smoke', 'duration': duration, 'at': 'SELF', 'attached': 1, 'location': '',
                   'translation': 0, 'orientation': 0})
    def stop(mm):
        mm.error = 'stopped after +64 store (0x42C6A5)'
        mm.u.emu_stop()
    m.hooks[0x42C6A5] = stop
    error = m.run(0x42C5E0, fxc, [node, 0], 8)
    del m.hooks[0x42C6A5]
    return {'name': name, 'duration': duration, 'error': error, 'vfx_bound': m.r(fxc + 0x3C) == vfx,
            'duration_byte': m.rb(fxc + 0x64), 'duration_bits': m.r(fxc + 0x60)}


def main():
    m = Oracle()
    enums = registrations(m)
    heal = [[v, n] for v, n, _ in enums['heal']]
    m.maps = {0xB3CB6C: heal,
              0xB3B654: [[v, n] for v, n, _ in enums['modifier']],
              0xB3B69C: [[v, n] for v, n, _ in enums['attribute']],
              0xB3B678: [[v, n] for v, n, _ in enums['flag']]}
    D = {0x3A4: {'blast': 0x5000, 'piercing': 0x5004}, 0x3AC: {'ench_type_generic': 0x5100},
         0x464: {}, 0xC: {}}
    effects = []
    effects.append(effect_case(m, 1, {'targets': 'enemy', 'hide_from_tooltip': 0, 'amount': f32(0.75)}, name='morale_loss'))
    effects.append(effect_case(m, 2, {'targets': '', 'hide_from_tooltip': 1, 'amount': f32(5)}, name='morale_gain'))
    for ht in ('HEAL', 'REPAIR', 'repair', 'Heal', 'INVALID', 'BOGUS'):
        effects.append(effect_case(m, 3, {'targets': 'damaged', 'hide_from_tooltip': 0, 'amount': f32(40),
                                          'heal_type': ht}, name=f'heal_{ht}'))
    for kind in (4, 5):
        effects.append(effect_case(m, kind, {'targets': '', 'hide_from_tooltip': 0, 'amount': f32(30),
                                             'damage_type_IDS': 'blast'}, known=D, name=f'damage{kind}'))
        effects.append(effect_case(m, kind, {'targets': '', 'hide_from_tooltip': 0, 'amount': f32(30),
                                             'damage_type_IDS': 'unknown'}, known=D, name=f'damage{kind}_unknown'))
    for kind in (7, 9):
        effects.append(effect_case(m, kind, {'targets': 'capturable', 'hide_from_tooltip': 1}, name=f'plain{kind}'))
    child = m.node({'ids': 'at_stun_x'})

    def property_load(mm):
        mm.records.append({'property_load': mm.arg(0) == child, 'vtable': hex(mm.r(mm.reg(UC_X86_REG_ECX)))})
        mm.ret(4)
    m.hooks[0x4C4F90] = property_load
    effects.append(effect_case(m, 6, {'targets': '', 'hide_from_tooltip': 0}, {'EnchantmentProperty': child},
                               known=D, name='enchantment'))
    del m.hooks[0x4C4F90]
    costs = m.node({})
    effects.append(effect_case(m, 8, {'targets': '', 'hide_from_tooltip': 0}, {'ResourcesAmount': costs},
                               name='steal_resources'))
    effects.append(effect_case(m, 0, {'targets': '', 'hide_from_tooltip': 0, 'object_ids': '',
                                      'dynamic_campaign_object_ids': ''}, name='summon_empty'))
    modifiers = []
    for attribute, amount, repair in (('SUPPLY_RANGE', -100, 0), ('AV', 2, 0), ('ABILITY_HEAL', 5, 1), ('AV', 5, 1),
                                      ('AUTO_UPGRADE', 0, 0), ('TERRAIN_SPEED_CAP', 0, 0), ('AV', 0, 0),
                                      ('SPEED_MAX', 1, 0), ('supply_range', -0.0, 0), ('NOPE', 1, 0)):
        modifiers.append(modifier_case(m, {'type': 'ADD', 'attribute': attribute, 'amount': f32(amount),
                                           'repair': repair}, known=D, name=f'add_{attribute}_{amount}_{repair}'))
    modifiers.append(modifier_case(m, {'type': 'PERIODIC_DAMAGE', 'damage_type': 'blast', 'amount': f32(10)},
                                   known=D, name='periodic_damage'))
    modifiers.append(modifier_case(m, {'type': 'periodic_damage', 'damage_type': 'none', 'amount': f32(10)},
                                   known=D, name='periodic_damage_unknown'))
    modifiers.append(modifier_case(m, {'type': 'FLAG', 'modifier_flag': 'PARALYZE'}, known=D, name='flag'))
    modifiers.append(modifier_case(m, {'type': 'RESIST_DAMAGE', 'damage_type': 'piercing', 'amount': f32(0.2)},
                                   known=D, name='resist'))
    modifiers.append(modifier_case(m, {'type': 'BOGUS'}, known=D, name='unknown_type'))
    vfx = [vfx_case(m, p, f32(d), name=f'vfx_{p}_{d}') for p in (0, 1) for d in (0, 2.5, -0.0)]
    permanent = []
    C = lambda life, nz, **kw: {'create': True, 'lifetime': life, 'duration_nonzero': nz, **kw}
    N = lambda **kw: {'create': False, **kw}
    for name, spec in (('empty', []), ('permanent', [C(1, 0)]), ('timed', [C(1, 1)]), ('nonpermanent', [C(2, 0)]),
                       ('other_then_permanent', [N(), C(1, 0)]), ('chain', [N(chain=[C(1, 0)])]),
                       ('immediate', [N(immediate=[C(3, 0), C(1, 0)])]), ('none', [N(), C(0, 0), C(2, 1)])):
        permanent.append(permanent_case(m, spec, name=name))
    create = [create_vfx_case(m, f32(d), name=f'create_{d}') for d in (0, 1.5, -0.0)]
    enchant = []
    for etype, fx_ok in (('ench_type_generic', True), ('unknown_type', True), ('ench_type_generic', False)):
        m.records = []
        m.known = D
        prop = m.alloc(0x80)
        m.object = prop
        m.w(prop + 8, m.r(m.cstring('at_stun_x')))
        head = 0
        if fx_ok:
            fxc = m.alloc(0x80)
            m.w(fxc, 0x9FC24C)
            vfx_def = m.alloc(0x40)
            m.w(vfx_def + 0x24, 1)
            m.w(fxc + 0x3C, vfx_def)
            node_ = m.alloc(8)
            m.w(node_, fxc)
            head = node_
        m.w(prop + 0x44, head)
        node = m.node({'is_technology': 0, 'duration': f32(1), 'enchantment_type': etype})
        error = m.run(0x4C4F90, prop, [node], 4)
        enchant.append({'enchantment_type': etype, 'permanent_fx': fx_ok, 'error': error, 'records': m.records,
                        'is_technology': m.rb(prop + 0x5C), 'duration_bits': m.r(prop + 0x60),
                        'type_slot': m.r(prop + 0x64)})
    h.report('native-ability-effect-fixtures.json',
             'Ability effect/enchantment/modifier loading (round 4); seams listed in the producer docstring.',
             effects, m.visited,
             extra={'enums': enums, 'modifiers': modifiers, 'vfx': vfx, 'permanent': permanent,
                    'create_vfx': create, 'enchantment_property': enchant})
    print({k: len(v) for k, v in enums.items()}, len(effects), len(modifiers))


if __name__ == '__main__':
    main()
