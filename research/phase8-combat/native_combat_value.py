"""Native oracle for combat value: 0x45E090 (ability value), 0x4BBF90 (thing
data mean values) and the modifier power getters 0x501A30/0x501AA0.

All three x87 precisions. The CRT pow (_CIpow 0x96D890 -> SSE2 body) runs
NATIVELY (no hook): [0xB4B340] = 1, MXCSR 0x1F80; its SSE2 arithmetic is exact
in Unicorn for these finite inputs (see native_crt_pow.py). Modifier getters
0x501D50/0x501DB0/0x501A30/0x501AA0/0x501560 and the definition getters run
natively. Seams (recorded): SVar lookup 0x4B2E60 (returns the case's float
cell), manager +4C stream virtual +14 (log enabled), CSV writer 0x856F00,
allocator 0x6F7810 / free 0x6F7430, assert 0x855D30, debug log 0x404F60.
"""
from pathlib import Path
import random
import struct
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import harness as h  # noqa: E402
from unicorn import x86_const as X  # noqa: E402

VALUE, MEANS = 0x45E090, 0x4BBF90
POWER_MELEE, POWER_RANGED = 0x501A30, 0x501AA0
LOOKUP, CSV, ALLOC, FREE, ASSERT, LOG = 0x4B2E60, 0x856F00, 0x6F7810, 0x6F7430, 0x855D30, 0x404F60
MANAGER_ROOT, GUARDS, SUPPRESS = 0xB39F58, 0xB3A66C, 0xB3B4F8


def f32(value):
    return struct.unpack('<I', struct.pack('<f', value))[0]


def r32(rng, lo, hi):
    return f32(rng.uniform(lo, hi))


class Oracle(h.Machine):
    def __init__(self, precision):
        super().__init__(precision)
        self.u.mem_map(0, 0x1000)
        self.u.reg_write(X.UC_X86_REG_MXCSR, 0x1F80)
        self.w(0xB4B340, 1)
        self.settings_table, self.settings = h.install_settings(self, {})
        self.events = []
        self.svars = {}
        self.hooks[LOOKUP] = self._lookup
        self.hooks[CSV] = self._csv
        self.hooks[ALLOC] = lambda m: m.ret(0, m.alloc(max(m.arg(0), 4)))
        self.hooks[FREE] = lambda m: m.ret(0)
        self.hooks[ASSERT] = self._assert
        self.hooks[LOG] = self._log
        self.assert_result = 0
        self.log_flag = 0
        self.manager = self.alloc(0x100)
        self.w(MANAGER_ROOT, self.manager)
        self.w(self.manager + 0x4C, self.vtable({0x14: lambda m: m.ret(0, m.log_flag)}))

    def alloc(self, size, align=16):
        ptr = super().alloc(size, align)
        self.u.mem_write(ptr, bytes(size))
        return ptr

    def setting(self, offset, bits):
        self.w(self.settings + offset, bits)
        self.w(self.settings_table + offset, self.settings + offset)

    def wstr(self, address):
        out = []
        while True:
            c = struct.unpack('<H', self.u.mem_read(address + 2 * len(out), 2))[0]
            if not c:
                return ''.join(out)
            out.append(chr(c))

    def _lookup(self, m):
        name = self.wstr(m.arg(0))
        self.events.append({'lookup': name, 'kind': m.arg(1)})
        m.ret(8, self.svars[name])

    def _csv(self, m):
        sp = m.reg(X.UC_X86_REG_ESP)
        doubles = struct.unpack('<17Q', m.u.mem_read(sp + 16, 17 * 8))
        self.events.append({'csv': [f'{v:016x}' for v in doubles], 'name': m.arg(2),
                            'stream': m.arg(0) - self.manager})
        m.ret(0)

    def _assert(self, m):
        self.events.append({'assert': self.wstr(m.arg(0)), 'line': m.arg(2)})
        m.ret(0, self.assert_result)

    def _log(self, m):
        sp = m.reg(X.UC_X86_REG_ESP)
        self.events.append({'log_line': m.arg(2), 'value': f'{struct.unpack("<Q", m.u.mem_read(sp + 24, 8))[0]:016x}'})
        m.ret(0)

    def st0_bits32(self):
        st = self.st0_ext()
        mant, top = st['mantissa'], st['sign_exponent']
        sign, exp = top >> 15, top & 0x7FFF
        if mant == 0:
            return sign << 31
        e = exp - 16383
        assert -126 <= e <= 127 and mant & ((1 << 40) - 1) == 0, (hex(mant), e)
        bits = (sign << 31) | ((e + 127) << 23) | ((mant >> 40) & 0x7FFFFF)
        return bits


def modifier_block(m, scalars):
    block = m.alloc(0x80)
    for offset, bits in scalars.items():
        m.w(block + offset, bits)
    return block


def power_cases(rng):
    cases = []
    simple = [f32(v) for v in (0.0, 1.5, 10.0, -3.0, 7.25, 1e-3, 123456.7)]
    for precision in h.PRECISIONS:
        m = Oracle(precision)
        combos = [(b, s4, sk, c) for b in simple for s4 in (f32(0), f32(2.5), f32(-4))
                  for sk in (f32(0), f32(1.75), f32(-20)) for c in (f32(0), f32(5), f32(-1), f32(1e6))]
        combos += [(r32(rng, -50, 50), r32(rng, -5, 5), r32(rng, -5, 5), r32(rng, -2, 60)) for _ in range(150)]
        for ranged in (False, True):
            for base, s4, sk, clamp in combos:
                m.setting(0x1A4, clamp)
                block = modifier_block(m, {4: s4, 8: sk if not ranged else f32(9.5),
                                           0xC: sk if ranged else f32(9.5)})
                m.call(POWER_RANGED if ranged else POWER_MELEE, ecx=block, args=(base,), pop=4)
                cases.append({'precision': precision, 'ranged': ranged, 'base': base, 's4': s4,
                              'kind': sk, 'clamp': clamp, 'result': m.st0_bits32()})
    return cases


def value_case(m, rng, index):
    # Inputs ---------------------------------------------------------------
    pick = rng.random
    defn = {
        'flags_54': rng.choice((0, 0, 8, 0x80, 0x88, 8)),
        'range_24': rng.choice((f32(0), r32(rng, 0.5, 60), r32(rng, 0.5, 60))),
        'area_38': rng.choice((f32(0), r32(rng, 0, 20))),
        'projectile_3c': rng.random() < 0.5,
        'power_64': rng.choice((f32(0), f32(-1), r32(rng, 0.5, 100), r32(rng, 0.5, 100), r32(rng, 0.5, 100))),
    }
    form = rng.choice(('absent', 'empty', 'entry', 'entry'))
    local = {'value_04': r32(rng, 0, 3), 'recharge_18': r32(rng, 0.1, 10), 'form': form,
             'first_entry_10': r32(rng, 0, 4) if form == 'entry' else None}
    means = {'attack': r32(rng, 5, 40), 'defense': r32(rng, 0, 10), 'movement': r32(rng, 1, 10),
             'lifetime': r32(rng, 0.5, 20), 'offense': r32(rng, 1, 30)}
    unit = {'defense': r32(rng, 0, 4.5), 'health': rng.choice((f32(0), r32(rng, 1, 500), r32(rng, 1, 500))),
            'speed': rng.choice((f32(0), r32(rng, 0, 12), r32(rng, 0, 12))),
            'additive_a': r32(rng, -2, 5), 'additive_b': r32(rng, -2, 5)}
    settings = {'damage_minimum': r32(rng, 0.1, 5), 'range_power': r32(rng, 0.2, 1.5),
                'area_power': r32(rng, 0, 2), 'offense_weight': r32(rng, 0, 2),
                'defense_weight': r32(rng, 0, 2), 'movement_weight': r32(rng, 0, 2),
                'movement_default': r32(rng, 0.1, 1.5), 'clamp': r32(rng, 0, 5)}
    svars = {'CombatValueRangedMinimum': r32(rng, 0, 30), 'CombatValueDefenseWeightRanged': r32(rng, 0, 2)}
    modifier = None
    if pick() < 0.5:
        modifier = {4: r32(rng, -1, 3), 8: r32(rng, -2, 5), 0xC: r32(rng, -2, 5),
                    0x28: r32(rng, -5, 10), 0x2C: r32(rng, -5, 10), 0x64: r32(rng, 0.5, 2)}
    log = pick() < 0.4
    # Memory ---------------------------------------------------------------
    m.events = []
    m.w(GUARDS, 0)
    m.log_flag = int(log)
    for offset, key in ((0xA4, 'damage_minimum'), (0x144, 'range_power'), (0x148, 'area_power'),
                        (0x14C, 'offense_weight'), (0x150, 'defense_weight'), (0x154, 'movement_weight'),
                        (0x158, 'movement_default'), (0x1A4, 'clamp')):
        m.setting(offset, settings[key])
    m.svars = {}
    for name, bits in svars.items():
        cell = m.alloc(4)
        m.w(cell, bits)
        m.svars[name] = cell
    mgr = m.manager
    for offset, key in ((0x78, 'attack'), (0x80, 'defense'), (0x84, 'movement'), (0x8C, 'lifetime'),
                        (0x90, 'offense')):
        m.w(mgr + offset, means[key])
    d = m.alloc(0x80)
    m.w(d + 0x10, 0x5150 + index)
    m.w(d + 0x24, defn['range_24'])
    m.w(d + 0x38, defn['area_38'])
    m.w(d + 0x3C, int(defn['projectile_3c']))
    m.w(d + 0x54, defn['flags_54'])
    m.w(d + 0x64, defn['power_64'])
    link = m.alloc(0x60)
    m.w(link, d)
    m.w(link + 4, local['value_04'])
    m.w(link + 0x18, local['recharge_18'])
    if form != 'absent':
        p, q, data, elem = m.alloc(4), m.alloc(8), m.alloc(4), m.alloc(0x20)
        m.w(link + 0x3C, p)
        m.w(link + 0x40, 1)
        m.w(p, q)
        m.w(q, data)
        m.w(q + 4, 1 if form == 'entry' else 0)
        m.w(data, elem)
        m.w(elem + 0x10, local['first_entry_10'] or 0)
    mod_ptr = modifier_block(m, modifier) if modifier else 0
    m.call(VALUE, args=(link, unit['defense'], unit['health'], unit['speed'], unit['additive_a'],
                        unit['additive_b'], mod_ptr), pop=0)
    result = m.st0_bits32()
    return {'precision': m.precision, 'definition': defn, 'local': local, 'means': means, 'unit': unit,
            'settings': settings, 'svars': svars,
            'modifier': {f'{k:#x}': v for k, v in modifier.items()} if modifier else None,
            'log_enabled': log, 'result': result, 'events': m.events}


def mean_case(m, rng, index):
    things = []
    count = rng.randint(3, 24)
    pool = [r32(rng, 0, 30) for _ in range(6)]
    for _ in range(count):
        flags = rng.choice((0x29, 0x29, 0x29, 0x4029, 0x20129, 0x20029, 0x4000, 0x09, 0x21))
        abilities = []
        for _ in range(rng.randint(0, 3)):
            form = rng.choice(('absent', 'entry'))
            abilities.append({'power_64': rng.choice((f32(0), rng.choice(pool), r32(rng, 0.5, 60))),
                              'value_04': r32(rng, 0, 3), 'recharge_18': r32(rng, 0.1, 10),
                              'first_entry_10': r32(rng, 0, 4) if form == 'entry' else None})
        things.append({'flags_174': flags,
                       'extent': {'count_468': rng.choice((0, 1, 2, -1)), 'base_478': r32(rng, 0, 20),
                                  'nested_3c': r32(rng, 0, 20)},
                       'abilities': abilities,
                       'defense_288': rng.choice((rng.choice(pool), r32(rng, 0, 12))),
                       'movement_228': r32(rng, 0, 12), 'health_284': r32(rng, 1, 600)})
    discard = rng.choice((f32(0), f32(0.05), f32(0.1), f32(0.2), f32(0.25)))
    suppressed = rng.random() < 0.3
    if index % 7 == 3:
        for t in things:
            t['defense_288'] = r32(rng, 60, 90)
    m.events = []
    m.setting(0x15C, discard)
    m.w(SUPPRESS, int(suppressed))
    mgr = m.alloc(0x100)
    for off in range(0x74, 0x94, 4):
        m.w(mgr + off, 0xDEADBEEF)
    holder = m.alloc(0x10)
    m.w(mgr + 0x68, holder)
    prev = holder
    first = True
    for t in things:
        d = m.alloc(0x500)
        m.w(d + 0x174, t['flags_174'])
        m.w(d + 0x228, t['movement_228'])
        m.w(d + 0x284, t['health_284'])
        m.w(d + 0x288, t['defense_288'])
        e = t['extent']
        m.w(d + 0x468, e['count_468'] & 0xFFFFFFFF)
        m.w(d + 0x478, e['base_478'])
        a, b, c = m.alloc(0x10), m.alloc(0x20), m.alloc(0x40)
        m.w(d + 0x464, a)
        m.w(a + 8, b)
        m.w(b + 0x1C, c)
        m.w(c + 0x3C, e['nested_3c'])
        arr = m.alloc(4 * max(1, len(t['abilities'])))
        m.w(d + 0x1EC, arr)
        m.w(d + 0x1F0, len(t['abilities']))
        for i, ab in enumerate(t['abilities']):
            ad = m.alloc(0x80)
            m.w(ad + 0x64, ab['power_64'])
            link = m.alloc(0x60)
            m.w(link, ad)
            m.w(link + 4, ab['value_04'])
            m.w(link + 0x18, ab['recharge_18'])
            if ab['first_entry_10'] is not None:
                p, q, data, elem = m.alloc(4), m.alloc(8), m.alloc(4), m.alloc(0x20)
                m.w(link + 0x3C, p)
                m.w(link + 0x40, 1)
                m.w(p, q)
                m.w(q, data)
                m.w(q + 4, 1)
                m.w(data, elem)
                m.w(elem + 0x10, ab['first_entry_10'])
            m.w(arr + 4 * i, link)
        node = m.alloc(0x10)
        m.w(node, d)
        m.w(prev + 8, node)
        prev = node
        first = False
    del first
    m.w(prev + 8, 0)
    m.call(MEANS, ecx=mgr)
    out = {k: m.r(mgr + off) for k, off in (('maximum_74', 0x74), ('attack', 0x78), ('attack_rate', 0x7C),
                                              ('defense', 0x80), ('movement', 0x84), ('health', 0x88),
                                              ('lifetime', 0x8C), ('offense', 0x90))}
    return {'precision': m.precision, 'things': things, 'discard': discard, 'suppressed': suppressed,
            'means': out, 'events': m.events}


PERFORMER = 0x57E430


def random_ability(rng):
    defn = {
        'flags_54': rng.choice((0, 0, 8, 0x80, 0x88, 8)),
        'range_24': rng.choice((f32(0), r32(rng, 0.5, 60), r32(rng, 0.5, 60))),
        'area_38': rng.choice((f32(0), r32(rng, 0, 20))),
        'projectile_3c': rng.random() < 0.5,
        'power_64': rng.choice((f32(0), r32(rng, 0.5, 100), r32(rng, 0.5, 100), r32(rng, 0.5, 100))),
    }
    form = rng.choice(('absent', 'empty', 'entry', 'entry'))
    local = {'value_04': r32(rng, 0, 3), 'recharge_18': r32(rng, 0.1, 10), 'form': form,
             'first_entry_10': r32(rng, 0, 4) if form == 'entry' else None}
    return defn, local


def build_link(m, defn, local, tag):
    d = m.alloc(0x80)
    m.w(d + 0x10, tag)
    m.w(d + 0x24, defn['range_24'])
    m.w(d + 0x38, defn['area_38'])
    m.w(d + 0x3C, int(defn['projectile_3c']))
    m.w(d + 0x54, defn['flags_54'])
    m.w(d + 0x64, defn['power_64'])
    link = m.alloc(0x60)
    m.w(link, d)
    m.w(link + 4, local['value_04'])
    m.w(link + 0x18, local['recharge_18'])
    if local['form'] != 'absent':
        p, q, data, elem = m.alloc(4), m.alloc(8), m.alloc(4), m.alloc(0x20)
        m.w(link + 0x3C, p)
        m.w(link + 0x40, 1)
        m.w(p, q)
        m.w(q, data)
        m.w(q + 4, 1 if local['form'] == 'entry' else 0)
        m.w(data, elem)
        m.w(elem + 0x10, local['first_entry_10'] or 0)
    return link


def performer_case(m, rng, index):
    means = {'attack': r32(rng, 5, 40), 'defense': r32(rng, 0, 10), 'movement': r32(rng, 1, 10),
             'lifetime': r32(rng, 0.5, 20), 'offense': r32(rng, 1, 30)}
    settings = {'damage_minimum': r32(rng, 0.1, 5), 'range_power': r32(rng, 0.2, 1.5),
                'area_power': r32(rng, 0, 2), 'offense_weight': r32(rng, 0, 2),
                'defense_weight': r32(rng, 0, 2), 'movement_weight': r32(rng, 0, 2),
                'movement_default': r32(rng, 0.1, 1.5), 'clamp': r32(rng, 0, 5)}
    svars = {'CombatValueRangedMinimum': r32(rng, 0, 30), 'CombatValueDefenseWeightRanged': r32(rng, 0, 2)}
    props = None
    modifier = None
    if rng.random() < 0.7:
        props = [r32(rng, -1, 3) for _ in range(rng.choice((0, 1, 3, 4, 5, 8, 9)))]
        modifier = {4: r32(rng, -1, 3), 8: r32(rng, -2, 5), 0xC: r32(rng, -2, 5),
                    0x28: r32(rng, -5, 10), 0x2C: r32(rng, -5, 10), 0x64: r32(rng, 0.5, 2)}
    body = (r32(rng, 0, 4.5), r32(rng, 1, 500)) if rng.random() < 0.85 else None
    speed = r32(rng, 0, 12) if rng.random() < 0.85 else None
    bonus = r32(rng, -1, 4)
    tables = []
    for _ in range(2):
        tables.append([{'enabled': rng.random() < 0.7, 'ability': random_ability(rng)}
                       for _ in range(rng.randint(0, 3))])
    secondary_present = rng.random() < 0.6
    log = rng.random() < 0.3
    m.events = []
    m.w(GUARDS, 0)
    m.log_flag = int(log)
    for offset, key in ((0xA4, 'damage_minimum'), (0x144, 'range_power'), (0x148, 'area_power'),
                        (0x14C, 'offense_weight'), (0x150, 'defense_weight'), (0x154, 'movement_weight'),
                        (0x158, 'movement_default'), (0x1A4, 'clamp')):
        m.setting(offset, settings[key])
    m.svars = {}
    for name, bits in svars.items():
        cell = m.alloc(4)
        m.w(cell, bits)
        m.svars[name] = cell
    for offset, key in ((0x78, 'attack'), (0x80, 'defense'), (0x84, 'movement'), (0x8C, 'lifetime'),
                        (0x90, 'offense')):
        m.w(m.manager + offset, means[key])
    actor = m.alloc(0x100)
    adef = m.alloc(0x220)
    m.w(actor + 4, adef)
    m.w(adef + 0x210, bonus)
    if body:
        b = m.alloc(0x40)
        m.w(b + 0x18, body[0])
        m.w(b + 0x14, body[1])
        m.w(actor + 0x60, b)
    if speed is not None:
        c = m.alloc(0x40)
        m.w(c + 0x28, speed)
        m.w(actor + 0x6C, c)
    if props is not None:
        comp = m.alloc(0x100)
        data = m.alloc(4 * max(1, len(props)))
        for i, v in enumerate(props):
            entry, pdef = m.alloc(0x20), m.alloc(0x40)
            m.w(pdef + 0x28, v)
            m.w(entry + 0xC, pdef)
            m.w(data + 4 * i, entry)
        m.w(comp + 0x10, data)
        m.w(comp + 0x14, len(props))
        for offset, bits in modifier.items():
            m.w(comp + 0x1C + offset, bits)
        m.w(actor + 0x78, comp)

    def table(rows, tag):
        data = m.alloc(4 * max(1, len(rows)))
        for i, row in enumerate(rows):
            entry = m.alloc(0x20)
            m.w(entry + 8, build_link(m, *row['ability'], tag + i))
            m.wb(entry + 0x18, int(row['enabled']))
            m.w(data + 4 * i, entry)
        return data
    performer = m.alloc(0x80)
    m.w(performer + 4, actor)
    m.w(performer + 0x14, table(tables[0], 0x100))
    m.w(performer + 0x18, len(tables[0]))
    m.w(performer + 0x60, 0x7F7FFFFF)
    if secondary_present:
        sec = m.alloc(0x40)
        m.w(sec + 0x10, table(tables[1], 0x200))
        m.w(sec + 0x14, len(tables[1]))
        m.w(actor + 0xB0, sec)
    m.call(PERFORMER, ecx=performer)
    return {'precision': m.precision, 'means': means, 'settings': settings, 'svars': svars,
            'property_values': props,
            'modifier': {f'{k:#x}': v for k, v in modifier.items()} if modifier else None,
            'body': body, 'speed': speed, 'bonus_210': bonus,
            'abilities': [{'enabled': r['enabled'], 'definition': r['ability'][0], 'local': r['ability'][1]}
                          for r in tables[0]],
            'secondary': [{'enabled': r['enabled'], 'definition': r['ability'][0], 'local': r['ability'][1]}
                          for r in tables[1]] if secondary_present else None,
            'log_enabled': log, 'value': m.r(performer + 0x60), 'events': m.events}


DEFINITION = 0x4618B0
UNCOMPUTED = 0xBF800000


def definition_case(m, rng, index):
    """0x4618B0 over a small definition graph (diagnostic stream disabled)."""
    means = {'attack': r32(rng, 5, 40), 'defense': r32(rng, 0, 10), 'movement': r32(rng, 1, 10),
             'lifetime': r32(rng, 0.5, 20), 'offense': r32(rng, 1, 30)}
    settings = {'damage_minimum': r32(rng, 0.1, 5), 'range_power': r32(rng, 0.2, 1.5),
                'area_power': r32(rng, 0, 2), 'offense_weight': r32(rng, 0, 2),
                'defense_weight': r32(rng, 0, 2), 'movement_weight': r32(rng, 0, 2),
                'movement_default': r32(rng, 0.1, 1.5), 'clamp': r32(rng, 0, 5)}
    svars = {'CombatValueRangedMinimum': r32(rng, 0, 30), 'CombatValueDefenseWeightRanged': r32(rng, 0, 2)}
    count = rng.randint(2, 7)
    things = []
    for i in range(count):
        flags = 0
        for bit, chance in ((5, 0.85), (0, 0.8), (3, 0.8), (6, 0.5), (7, 0.4), (10, 0.4)):
            if rng.random() < chance:
                flags |= 1 << bit
        things.append({
            'flags_174': flags,
            'list_1d4': [r32(rng, -1, 3) for _ in range(rng.choice((0, 1, 2, 3)))],
            'defense_288': r32(rng, 0, 4.5), 'health_284': rng.choice((f32(0), r32(rng, 1, 500))),
            'speed_228': r32(rng, 0, 12), 'bonus_210': r32(rng, -1, 4),
            'abilities': [random_ability(rng) for _ in range(rng.randint(0, 3))],
            'children_2f0': [rng.choice((None, rng.randrange(count), rng.randrange(count)))
                             for _ in range(rng.randint(0, 3))],
            'weighted_35c': [(rng.randrange(count), rng.randint(-2, 5)) for _ in range(rng.randint(0, 3))],
            'initial_1e4': rng.choice((UNCOMPUTED, UNCOMPUTED, r32(rng, 0, 50))),
        })
    m.events = []
    m.w(GUARDS, 0)
    m.log_flag = 0
    for offset, key in ((0xA4, 'damage_minimum'), (0x144, 'range_power'), (0x148, 'area_power'),
                        (0x14C, 'offense_weight'), (0x150, 'defense_weight'), (0x154, 'movement_weight'),
                        (0x158, 'movement_default'), (0x1A4, 'clamp')):
        m.setting(offset, settings[key])
    m.svars = {}
    for name, bits in svars.items():
        cell = m.alloc(4)
        m.w(cell, bits)
        m.svars[name] = cell
    for offset, key in ((0x78, 'attack'), (0x80, 'defense'), (0x84, 'movement'), (0x8C, 'lifetime'),
                        (0x90, 'offense')):
        m.w(m.manager + offset, means[key])
    defs = [m.alloc(0x380) for _ in things]
    for d, t in zip(defs, things):
        m.w(d + 0x174, t['flags_174'])
        m.w(d + 0x288, t['defense_288'])
        m.w(d + 0x284, t['health_284'])
        m.w(d + 0x228, t['speed_228'])
        m.w(d + 0x210, t['bonus_210'])
        m.w(d + 0x1E4, t['initial_1e4'])
        m.w(d + 0x30C, 0x12345678)
        m.w(d + 0x36C, 0x12345678)
        head = 0
        for v in reversed(t['list_1d4']):
            node, item = m.alloc(8), m.alloc(0x40)
            m.w(item + 0x28, v)
            m.w(node, item)
            m.w(node + 4, head)
            head = node
        m.w(d + 0x1D4, head)
        arr = m.alloc(4 * max(1, len(t['abilities'])))
        for i, ab in enumerate(t['abilities']):
            m.w(arr + 4 * i, build_link(m, ab[0], ab[1], 0x300 + i))
        m.w(d + 0x1EC, arr)
        m.w(d + 0x1F0, len(t['abilities']))
        holder, table = m.alloc(0x10), m.alloc(0x30)
        n = len(t['children_2f0'])
        indices, slots = m.alloc(4 * max(1, n)), m.alloc(4 * max(1, n))
        for i, child in enumerate(t['children_2f0']):
            slot = n - 1 - i
            m.w(indices + 4 * i, slot)
            m.w(slots + 4 * slot, 0 if child is None else defs[child])
        m.w(table + 0x28, indices)
        m.w(table + 0x2C, n)
        m.w(holder + 8, table)
        m.w(holder + 0xC, slots)
        m.w(d + 0x2F0, holder)
        head = 0
        for child, c in reversed(t['weighted_35c']):
            node = m.alloc(0x10)
            m.w(node, defs[child])
            m.w(node + 8, c & 0xFFFFFFFF)
            m.w(node + 0xC, head)
            head = node
        m.w(d + 0x35C, head)
    whole_list = index % 2 == 1
    if whole_list:
        # 0x4BB590 ComputeCVActorDataValues over the manager +68 list (all
        # definitions in order); SVar CombatValueCSV is an empty CString.
        m.w(0xB3B4F4, 0)
        text = m.alloc(0x20)
        m.w(text, 1000)
        m.w(text + 4, 0)
        m.w(text + 0xC, 1000)
        cell = m.alloc(0x20)
        m.w(cell + 0x10, text + 0x10)
        m.svars['CombatValueCSV'] = cell
        holder = m.alloc(0x10)
        prev = holder
        for d in defs:
            node = m.alloc(0x10)
            m.w(node, d)
            m.w(prev + 8, node)
            prev = node
        m.w(m.manager + 0x68, holder)
        m.call(0x4BB590, ecx=m.manager)
    else:
        m.call(DEFINITION, ecx=defs[0])
    out = [{'value_1e4': m.r(d + 0x1E4), 'children_30c': m.r(d + 0x30C), 'weighted_36c': m.r(d + 0x36C)}
           for d in defs]
    for t in things:
        t['abilities'] = [{'definition': a[0], 'local': a[1]} for a in t['abilities']]
    return {'precision': m.precision, 'means': means, 'settings': settings, 'svars': svars,
            'entry': 'list_4bb590' if whole_list else 'definition_4618b0',
            'things': things, 'out': out, 'events': m.events}


def main():
    rng = random.Random(0x45E090)
    visited = {}
    power = power_cases(rng)
    values, means, performers, definitions = [], [], [], []
    for precision in h.PRECISIONS:
        m = Oracle(precision)
        for i in range(60):
            performers.append(performer_case(m, rng, i))
        for i in range(60):
            definitions.append(definition_case(m, rng, i))
        for i in range(160):
            values.append(value_case(m, rng, i))
        for i in range(36):
            means.append(mean_case(m, rng, i))
        visited.update(m.visited)
    h.report('native-combat-value-fixtures.json',
             '0x45E090 ability combat value (native CRT pow), 0x4BBF90 mean values, 0x501A30/0x501AA0 '
             'modifier power; all precisions. F32 values are bit patterns.',
             values, visited, extra={'power_cases': power, 'mean_cases': means, 'performer_cases': performers,
                    'definition_cases': definitions})
    csv = sum(1 for c in values if any('csv' in e for e in c['events']))
    asserts = sum(1 for c in means if any('assert' in e for e in c['events']))
    print(f'value cases {len(values)} (csv {csv}), mean cases {len(means)} (asserts {asserts}), '
          f'power cases {len(power)}')


if __name__ == '__main__':
    main()
