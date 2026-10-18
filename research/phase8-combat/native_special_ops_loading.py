"""Native oracle for the round-5 definition loading used by special operations.

Original code executed in Unicorn (byte-pinned through harness.py):
* modifier loading 4C4030 for every modifier type, with the real 4FE840
  construction and each LoadData: MULTIPLY 4FED70, ENCHANTMENT_IMMUNITY
  5010A0, ABILITY_EFFECT 501150 (nested 4CA880 effect and its LoadData),
  PERIODIC_HEAL 4FEE90, REVERSE_ABILITY 501410, RESOURCE_PRODUCTION 4FEEC0
  (vector 4849F0/45F180), all over the base reader 4FEAF0.
* SummonEffect LoadData 4CC890 with the real deferred object 4CAA00/473AD0
  (list [B36530]+824), then its resolver (vtable 4CCD60 -> 4CC730).
* ModifierProperty required_properties resolution 4C4E10 with the real
  tokenizer 424FA0/412480.
Seams (recorded, never executed): the round-4 TGI node getters, 48D500/
4BDB70 finds, 497220 enum maps, 463FA0 Property find, 427BD0 Thing find
(fake ThingData: +8 name, virtual +38 actor, +174 mask, +184 transport,
transport +1C flags), CString manager 856370/405B80, fatal 410C40, logs, heap.
"""
from pathlib import Path
import struct
import sys

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import harness as h  # noqa: E402
import native_ability_effects as nae  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX, UC_X86_REG_EAX  # noqa: E402

f32 = nae.f32
RESOURCES = ['gold', 'oil', 'steel']


class Oracle(nae.Oracle):
    def __init__(self):
        super().__init__()
        for offset in (0x3A8, 0x468):
            self.w(self.world + offset, self.alloc(0x40))
            self.registries[self.r(self.world + offset)] = offset
        # ATL IAtlStringMgr: Allocate +0 (nAllocLength, nCharSize), Free +4,
        # Reallocate +8 (data, nAllocLength, nCharSize), GetNilString +C,
        # Clone +10. CStringData: +0 manager, +4 length, +8 alloc, +C refs.
        def allocate(mm, n=None, size=None):
            n = mm.arg(0) if n is None else n
            size = mm.arg(1) if size is None else size
            header = mm.alloc(0x10 + (n + 1) * size)
            mm.w(header, mm.manager)
            mm.w(header + 8, n)
            mm.w(header + 0xC, 1)
            return header

        def reallocate(mm):
            old, n, size = mm.arg(0), mm.arg(1), mm.arg(2)
            new = allocate(mm, n, size)
            length = mm.r(old + 4)
            mm.u.mem_write(new + 0x10, bytes(mm.u.mem_read(old + 0x10, min(length + 1, n + 1) * size)))
            mm.w(new + 4, length)
            mm.ret(12, new)
        self.w(self.manager, self.vtable({0x0: lambda mm: mm.ret(8, allocate(mm)),
                                          0x4: lambda mm: mm.ret(4),
                                          0x8: reallocate,
                                          0xC: lambda mm: mm.ret(0, mm.nil),
                                          0x10: lambda mm: mm.ret(0, mm.reg(UC_X86_REG_ECX))}))
        # Field getters: a missing field throws "Node doesn't contain field."
        # (5B5163 and the sibling getters); modelled as a fatal stop.
        self.hooks[0x5B5130] = lambda m: m._getter(lambda v: m.ret_f32(4, v))
        self.hooks[0x5B4AE0] = lambda m: m._getter(lambda v: m.ret_f32(4, v))
        self.hooks[0x5B5380] = lambda m: m._getter(lambda v: m.ret(4, int(v)))
        self.hooks[0x5B54D0] = lambda m: m._getter(lambda v: m.ret(4, m.cstring(v)))
        self.hooks[0x463FA0] = self._property_find
        self.hooks[0x427BD0] = self._thing_find
        self.things = {}
        names = self.alloc(4 * len(RESOURCES))
        for i, name in enumerate(RESOURCES):
            rdef = self.alloc(0x20)
            self.w(rdef + 8, self.r(self.cstring(name)))
            self.w(names + 4 * i, rdef)
        self.w(self.world + 0x34, names)
        self.resource_count = len(RESOURCES)

    def _child(self, m):
        """5B4410(name, required): a missing required sub-node throws the TGI
        exception "SubNode not found." (5B46DE); modelled as a fatal stop."""
        node = m.reg(UC_X86_REG_ECX)
        name = m.wide(m.arg(0))
        child = m.nodes[node][1].get(name.lower(), 0)
        m.records.append({'child': name, 'required': m.arg(1) & 0xFF, 'present': bool(child)})
        if not child and m.arg(1) & 0xFF:
            m.error = f'SubNode not found. ({name})'
            m.u.emu_stop()
            return
        m.ret(8, child)

    def _getter(self, done):
        node = self.reg(UC_X86_REG_ECX)
        name = self.wide(self.arg(0))
        self.records.append({'get': name})
        fields = self.nodes[node][0]
        if name.lower() not in fields:
            self.error = f"Node doesn't contain field. ({name})"
            self.u.emu_stop()
            return
        done(fields[name.lower()])

    def set_resource_count(self, count):
        self.w(self.world + 0x38, count)

    def _property_find(self, m):
        # 463FA0 receives the CString data pointer itself.
        name = m.wide(m.arg(0))
        value = m.known.get(0x464, {}).get(name.lower(), 0)
        m.records.append({'property_find': name, 'found': bool(value)})
        m.ret(4, value)

    def _thing_find(self, m):
        # 427BD0 receives the CString data pointer itself.
        name = m.wide(m.arg(0))
        value = m.things.get(name.lower(), 0)
        m.records.append({'thing_find': name, 'found': bool(value)})
        m.ret(4, value)

    def thing(self, name, actor, mask=0, transport_flags=None):
        t = self.alloc(0x200)
        self.w(t + 8, self.r(self.cstring(name)))
        self.w(t, self.vtable({0x38: lambda mm, a=actor: mm.ret(0, int(a))}))
        self.w(t + 0x174, mask)
        if transport_flags is not None:
            tr = self.alloc(0x40)
            self.w(tr + 0x1C, transport_flags)
            self.w(t + 0x184, tr)
        self.things[name.lower()] = t
        return t


def dump_effect(m, obj):
    return [m.r(obj + 4 * i) for i in range(11)]


def modifier(m, fields, children=None, known=None, name='', count=None):
    m.records = []
    m.known = known or {}
    if count is not None:
        m.set_resource_count(count)
    prop = m.alloc(0x80)
    m.object = prop
    m.w(prop + 8, m.r(m.cstring('test_property')))
    node = m.node(fields, children)
    error = m.run(0x4C4030, prop, [node], 4)
    head = m.r(prop + 0x1C)
    out = {'name': name, 'fields': fields, 'error': error, 'records': m.records, 'added': bool(head)}
    if count is not None:
        out['resource_count'] = count
    if head:
        mod = m.r(head)
        out['vtable'] = hex(m.r(mod))
        out['words'] = [m.r(mod + 4 * i) for i in range(6)]
        if fields['type'].upper() == 'ABILITY_EFFECT' and m.r(mod + 0x10):
            effect = m.r(mod + 0x10)
            out['effect'] = dump_effect(m, effect)
            out['effect_hide_byte'] = m.rb(effect + 0x10)
        if fields['type'].upper() == 'RESOURCE_PRODUCTION':
            data, size = m.r(mod + 0xC), m.r(mod + 0x10)
            out['production'] = [m.r(data + 4 * i) for i in range(size)] if data else []
    return out


def summon(m, fields, things, known=None, name='', campaign=False):
    m.records = []
    m.known = known or {}
    m.things = {}
    for spec in things:
        m.thing(**spec)
    m.w(m.world + 0x824, 0)
    m.run(0x4CA880, 0, [0], 0)
    effect = m.reg(UC_X86_REG_EAX)
    m.object = effect
    node = m.node(fields)
    error = m.run(m.r(m.r(effect) + 0x10), effect, [node], 4)
    out = {'name': name, 'fields': fields, 'things': things, 'load_error': error, 'load_records': m.records,
           'loaded': dump_effect(m, effect), 'hide_byte': m.rb(effect + 0x10), 'controllable': m.rb(effect + 0x28)}
    deferred = m.r(m.world + 0x824)
    out['deferred'] = bool(deferred)
    if error is None and deferred:
        out['deferred_vtable'] = hex(m.r(deferred))
        out['deferred_effect'] = m.r(deferred + 0x10) == effect
        m.records = []
        error = m.run(m.r(m.r(deferred)), deferred, [], 0)
        out['resolve_error'] = error
        out['resolve_records'] = m.records
        resolved = {}
        for key, slot in (('object', 0x14), ('dynamic', 0x18)):
            ptr = m.r(effect + slot)
            resolved[key] = next((n for n, p in m.things.items() if p == ptr), ptr)
        out['resolved'] = resolved
    return out


def required(m, text, known, name=''):
    m.records = []
    m.known = known
    prop = m.alloc(0x80)
    m.object = prop
    m.w(prop + 8, m.r(m.cstring('owner_property')))
    error = m.run(0x4C4E10, prop, [m.cstring(text)], 4)
    order = []
    node = m.r(prop + 0x38)
    while node:
        order.append(m.r(node))
        node = m.r(node + 4)
    inv = {v: k for k, v in known.get(0x464, {}).items()}
    return {'name': name, 'text': text, 'error': error, 'records': m.records,
            'list': [inv.get(v, v) for v in order], 'tail_set': m.r(prop + 0x3C) != 0}


def ability_required(m, text, known, name=''):
    """4BF990's own required_properties block (4C00AB..4C023D), executed in
    place with EBP = the Ability node and ESI = the AbilityData (+58 head,
    +5C tail); the run stops at the block exit 4C023D."""
    from unicorn.x86_const import UC_X86_REG_ESP, UC_X86_REG_EBP, UC_X86_REG_ESI, UC_X86_REG_EIP
    m.records = []
    m.known = known
    m.error = None
    data = m.alloc(0x80)
    m.object = data
    node = m.node({'required_properties': text})
    frame = m.STACK - 0x2000
    m.u.mem_write(frame, bytes(0x200))
    nil = m.nil + 0x10
    m.w(frame + 0x10, nil)
    m.w(frame + 0x88, nil)
    m.u.reg_write(UC_X86_REG_ESP, frame)
    m.u.reg_write(UC_X86_REG_EBP, node)
    m.u.reg_write(UC_X86_REG_ESI, data)
    m.hooks[0x4C023D] = lambda mm: mm.u.emu_stop()
    m.u.emu_start(0x4C00AB, 0xFFFFFFF0, count=400000)
    del m.hooks[0x4C023D]
    if m.error is None:
        assert m.reg(UC_X86_REG_EIP) == 0x4C023D, hex(m.reg(UC_X86_REG_EIP))
        assert m.reg(UC_X86_REG_ESP) == frame
    order = []
    cell = m.r(data + 0x58)
    while cell:
        order.append(m.r(cell))
        cell = m.r(cell + 4)
    inv = {v: k for k, v in known.get(0x464, {}).items()}
    return {'name': name, 'text': text, 'error': m.error, 'records': m.records,
            'list': [inv.get(v, v) for v in order], 'tail_set': m.r(data + 0x5C) != 0}


def main():
    m = Oracle()
    enums = nae.registrations(m)
    m.maps = {0xB3CB6C: [[v, n] for v, n, _ in enums['heal']],
              0xB3B654: [[v, n] for v, n, _ in enums['modifier']],
              0xB3B69C: [[v, n] for v, n, _ in enums['attribute']],
              0xB3B678: [[v, n] for v, n, _ in enums['flag']],
              0xB3B060: [[v, n] for v, n, _ in enums['effect']]}
    attributes = [n for _, n, _ in enums['attribute']]
    D = {0x3A4: {'blast': 0x5000, 'piercing': 0x5004}, 0x3AC: {'ench_type_generic': 0x5100, 'ench_type_fear': 0x5104},
         0x3A8: {'artillery': 0x5200, 'null': 0x5204}, 0x468: {'gbr_supply_drop': 0x5300, 'courage': 0x5304},
         0x464: {'tank': 0x5400, 'infantry': 0x5404, 'property_blitzkrieg': 0x5408}, 0xC: {}}
    mods = []
    # MULTIPLY 4FED70: every attribute, then the amount range rule.
    for attribute in attributes + ['NOT_AN_ATTRIBUTE']:
        mods.append(modifier(m, {'type': 'MULTIPLY', 'attribute': attribute, 'amount': f32(0.2)}, known=D,
                             name=f'multiply_{attribute}'))
    for amount in (-1.0, -1.5, 5.0, 5.5, 0.0, -0.0, 2.0):
        for attribute in ('SPEED_MAX', attributes[18], attributes[23]):
            mods.append(modifier(m, {'type': 'multiply', 'attribute': attribute, 'amount': f32(amount)}, known=D,
                                 name=f'multiply_{attribute}_{amount}'))
    # ENCHANTMENT_IMMUNITY 5010A0.
    for etype in ('ench_type_fear', 'ENCH_TYPE_GENERIC', '', 'unknown'):
        mods.append(modifier(m, {'type': 'ENCHANTMENT_IMMUNITY', 'enchantment_type': etype}, known=D,
                             name=f'immunity_{etype}'))
    # ABILITY_EFFECT 501150 with a nested effect.
    morale = m.node({'type': 'MORALE_LOSS', 'targets': 'enemy', 'hide_from_tooltip': 1, 'amount': f32(0.5)})
    damage = m.node({'type': 'MAIN_DAMAGE', 'targets': '', 'hide_from_tooltip': 0, 'amount': f32(30),
                     'damage_type_IDS': 'blast'})
    bogus = m.node({'type': 'NOT_AN_EFFECT'})
    for atype, child, label in (('artillery', morale, 'morale'), ('ARTILLERY', damage, 'damage'),
                                ('unknown', morale, 'unknown_type'), ('', morale, 'empty_type'),
                                ('artillery', None, 'no_effect'), ('artillery', bogus, 'unknown_effect')):
        children = {'Effect': child} if child else {}
        mods.append(modifier(m, {'type': 'ABILITY_EFFECT', 'ability_type': atype}, children, known=D,
                             name=f'ability_effect_{label}'))
    # PERIODIC_HEAL 4FEE90.
    for amount in (5.0, -2.0, 0.0):
        mods.append(modifier(m, {'type': 'PERIODIC_HEAL', 'amount': f32(amount)}, known=D,
                             name=f'periodic_heal_{amount}'))
    # REVERSE_ABILITY 501410.
    for ability in ('gbr_supply_drop', 'COURAGE', '', 'unknown'):
        mods.append(modifier(m, {'type': 'REVERSE_ABILITY', 'ability_definition': ability}, known=D,
                             name=f'reverse_{ability}'))
    # RESOURCE_PRODUCTION 4FEEC0 over the Resource registry ([B36530]+10).
    for count in (0, 1, 3):
        production = m.node({name: f32(1.5 * (i + 1)) for i, name in enumerate(RESOURCES[:count])})
        mods.append(modifier(m, {'type': 'RESOURCE_PRODUCTION'}, {'Production': production}, known=D,
                             name=f'production_{count}', count=count))
    mods.append(modifier(m, {'type': 'RESOURCE_PRODUCTION'}, {}, known=D, name='production_missing', count=2))
    mods.append(modifier(m, {'type': 'RESOURCE_PRODUCTION'}, {'Production': m.node({'gold': f32(1)})}, known=D,
                         name='production_partial', count=2))
    m.set_resource_count(len(RESOURCES))
    # SummonEffect 4CC890 and the deferred resolution 4CC730.
    base = {'targets': '', 'hide_from_tooltip': 1, 'object_ids': 'supply', 'dynamic_campaign_object_ids': '',
            'property_ids': '', 'controllable': 0, 'unique_target_flags': ''}
    thing = lambda name, actor=True, mask=0, transport=None: {'name': name, 'actor': actor, 'mask': mask,
                                                              'transport_flags': transport}
    summons = []
    for label, fields, things in (
            ('object', {}, [thing('supply', False)]),
            ('actor', {'controllable': 1}, [thing('supply', True, 1 << 6)]),
            ('unknown_object', {}, []),
            ('empty_object', {'object_ids': ''}, [thing('supply')]),
            ('dynamic', {'dynamic_campaign_object_ids': 'camp'}, [thing('supply'), thing('camp')]),
            ('unknown_dynamic', {'dynamic_campaign_object_ids': 'camp'}, [thing('supply')]),
            ('flying_transport', {}, [thing('supply', True, 1 << 16, 0x1000)]),
            ('ground_transport', {}, [thing('supply', True, 1 << 16, 0x0)]),
            ('ground_transport_no_recruit', {}, [thing('supply', True, 0, 0x0)]),
            ('dynamic_ground_transport', {'dynamic_campaign_object_ids': 'camp'},
             [thing('supply', True, 1 << 16, 0x1000), thing('camp', True, 1 << 16, 0x400)]),
            ('property', {'property_ids': 'tank'}, [thing('supply', True, 1 << 6)]),
            ('property_unknown', {'property_ids': 'nope'}, [thing('supply', True, 1 << 6)]),
            ('property_non_actor', {'property_ids': 'tank'}, [thing('supply', False)]),
            ('property_no_component', {'property_ids': 'tank'}, [thing('supply', True, 0)]),
            ('property_dynamic_non_actor', {'property_ids': 'tank', 'dynamic_campaign_object_ids': 'camp'},
             [thing('supply', True, 1 << 6), thing('camp', False)]),
            ('property_dynamic_no_component', {'property_ids': 'TANK', 'dynamic_campaign_object_ids': 'camp'},
             [thing('supply', True, 1 << 6), thing('camp', True, 0)]),
            ('unique_flags', {'unique_target_flags': 'tank', 'targets': '+SELF'}, [thing('supply', False)])):
        summons.append(summon(m, {**base, **fields}, things, known=D, name=label))
    # ModifierProperty required_properties 4C4E10.
    reqs = []
    for text in ('', 'tank', 'tank,infantry', 'infantry,tank', ' tank , infantry ', 'tank,,infantry', ',tank',
                 'tank,', 'unknown', 'tank unknown', 'TANK', 'tank\tinfantry', 'tank\ninfantry', '"tank,infantry"',
                 "'tank'", 'tank;infantry', ',', ' ', 'tank infantry', 'tank　infantry', 'tank, ,infantry',
                 '"",tank', '"tank', "tank'x'", "'tank'infantry", '"tank"",infantry', ' , tank', "'',tank",
                 'tank,"infantry', '\t\ttank', 'tank ,infantry'):
        reqs.append(required(m, text, D, name=repr(text)))
    # Leading-character sweep: CRT iswspace (96CC0B) table for code units
    # 1..255 (a leading whitespace unit is skipped, any other one is kept).
    for code in range(1, 256):
        if chr(code) in ',"\'':
            continue
        reqs.append(required(m, chr(code) + 'tank', D, name=f'lead_{code:02x}'))
    ability_reqs = []
    for case in reqs:
        ability_reqs.append(ability_required(m, case['text'], D, name=case['name']))
    h.report('native-special-ops-loading-fixtures.json',
             'Round-5 modifier kinds, SummonEffect loading/resolution and required_properties (4C4E10).',
             mods, m.visited, extra={'enums': enums, 'summons': summons, 'required': reqs,
                    'ability_required': ability_reqs})
    print(len(mods), len(summons), len(reqs), len(ability_reqs))


if __name__ == '__main__':
    main()
