"""Native oracle for the special-operation activation and effect Apply
virtuals (round 5). Original code runs; recorded seams are listed per case.

* faction: KKC_Faction::UseAbility 4CDE00 with 4C8300 and 4C9EF0 executing;
  seams 4C8F20 (admission: context bytes, check_range; result per case),
  4C9550 (use: context bytes and time; result per case), 404F60 log.
* heal_modifier: 501B70 (modifier heal amount) with the [B39AB0]+1A4 cap.
* heal: HealEffect Apply 4CA0B0 (501B70 executes); seams 4C8330 (target),
  4CF110 (source modifier), 562BA0 (body heal: the {source, amount} record).
* enchant: EnchantmentEffect Apply 4CA130 with 581280/5032D0, 581700,
  501610 and 503140 executing; seams 4C8330, owner virtual +138 (allied),
  581630 (insertion: the 0x1C instance bytes).
* capture: CaptureEffect Apply 4CA260; seams 4C8330, 4D5230 (record bytes).
* describe: DescribeEconomyEffect Apply 4CC050 up to its seams 4C8330,
  kingdom virtuals +108/+24, 4E7200 (text), 4B9F40 (localize), 40BF70 (post).
* find: 5032D0 directly and the actor wrapper 4CEF80.
GTIDs resolve through the real registry layout ([B364C0]).
"""
from pathlib import Path
import random
import struct
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import harness as h  # noqa: E402
import native_special_ops_loading as nsl  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX  # noqa: E402

f32 = h.f32


class Oracle(nsl.Oracle):
    def __init__(self):
        super().__init__()
        self.events = []
        self.registry = self.alloc(0x40000)
        self.w(h.REGISTRY_ROOT, self.registry)
        self.slot = 0x20
        self.world_obj = h.install_world(self, time=f32(123.5))
        self.settings, self.storage = h.install_settings(self, {0x1A4: f32(25.0), 0x128: 0})

    def gtid(self, obj):
        self.slot += 1
        self.u.mem_write(self.registry + 4 + self.slot * 2, (5).to_bytes(2, 'little'))
        if obj:
            self.w(self.registry + 0x20004 + self.slot * 4, obj)
        return (5 << 16) | self.slot

    def handle(self, size=0x200):
        obj = self.alloc(size)
        self.w(obj + 0x14, self.gtid(obj))
        return obj


def words(m, address, count):
    return list(struct.unpack(f'<{count}I', bytes(m.u.mem_read(address, 4 * count))))


def faction_cases(m, rng):
    out = []
    for index in range(40):
        events = []
        count = rng.randint(0, 4)
        pool = []
        for d in range(6):
            obj = m.alloc(0x20)
            m.w(obj + 0x10, m.r(m.cstring(f'ability_{d}')))
            pool.append(obj)
        picks = [rng.randrange(6) for _ in range(count)]
        definitions = [pool[i] for i in picks]
        wanted_index = rng.choice(picks + [5]) if count else 5
        wanted = pool[wanted_index]
        target = rng.random() < 0.5
        can_use = rng.choice((0, 0, 4, 9))
        use_result = rng.choice((0, 1, 2, -1))
        faction = m.alloc(0x40)
        fdef = m.alloc(0x40)
        m.w(fdef + 0x10, m.r(m.cstring('rommel')))
        m.w(faction + 4, fdef)
        owner = m.handle()
        m.w(faction + 8, owner)
        abilities = m.alloc(4 * max(1, count))
        handles = []
        for d in definitions:
            ab = m.alloc(0x40)
            m.w(ab + 4, d)
            m.w(abilities + 4 * len(handles), ab)
            handles.append(ab)
        m.w(faction + 0x10, abilities)
        m.w(faction + 0x14, count)
        tgt = 0
        pos = [f32(rng.uniform(-300, 300)) for _ in range(3)]
        if target:
            tgt = m.handle()
            for k in range(3):
                m.w(tgt + 0x20 + 4 * k, pos[k])
        point = m.alloc(8)
        pt = [f32(rng.uniform(-300, 300)), f32(rng.uniform(-300, 300))]
        m.w(point, pt[0])
        m.w(point + 4, pt[1])

        def admission(mm, events=events, handles=handles, can_use=can_use, owner=owner, tgt=tgt):
            ctx = words(mm, mm.arg(0), 12)
            events.append({'can_use': handles.index(mm.reg(UC_X86_REG_ECX)), 'check_range': mm.arg(1),
                           'context': [ctx[0], ctx[1], int(ctx[2] == owner), int(ctx[3] == owner)] + ctx[4:7]
                           + [int(ctx[7] == tgt and tgt != 0), ctx[7] == 0] + ctx[8:]})
            mm.ret(8, can_use)

        def use(mm, events=events, handles=handles, use_result=use_result):
            ctx = words(mm, mm.arg(0), 12)
            events.append({'use': handles.index(mm.reg(UC_X86_REG_ECX)), 'time': mm.arg(1),
                           'context_kind': ctx[0], 'target_position': ctx[8:11]})
            mm.ret(8, use_result & 0xFFFFFFFF)

        def log(mm, events=events):
            events.append({'log': True})
            mm.ret(0)
        m.hooks[0x4C8F20] = admission
        m.hooks[0x4C9550] = use
        m.hooks[0x404F60] = log
        al = m.call(0x4CDE00, ecx=faction, args=(wanted, tgt, point), pop=12) & 0xFF
        out.append({'definitions': picks, 'wanted': wanted_index, 'target': target, 'target_position': pos,
                    'point': pt, 'can_use': can_use, 'use_result': use_result, 'returned': al, 'events': events})
    for a in (0x4C8F20, 0x4C9550, 0x404F60):
        del m.hooks[a]
    return out


def heal_modifier_cases(m, rng):
    out = []
    values = [0.0, -0.0, 1.0, 5.0, 25.0, 30.0, -3.0, 0.1, 1e-30, 100.0, 24.999998]
    for amount in values:
        for bonus in (0.0, 2.5, -2.5, -40.0, 1e-7, 7.0):
            for cap in (25.0, 0.0, -1.0):
                mod = m.alloc(0x40)
                m.w(mod + 0x14, f32(bonus))
                m.w(m.storage + 0x1A4, f32(cap))
                m.call(0x501B70, ecx=mod, args=(f32(amount),), pop=4)
                out.append({'amount': f32(amount), 'bonus': f32(bonus), 'cap': f32(cap),
                            'result': m.st0_f32() if hasattr(m, 'st0_f32') else None})
    m.w(m.storage + 0x1A4, f32(25.0))
    return out


def target_hook(m, target):
    m.hooks[0x4C8330] = lambda mm: mm.ret(0, target)


def heal_cases(m, rng):
    out = []
    for index in range(30):
        events = []
        has_target = rng.random() < 0.85
        has_body = rng.random() < 0.85
        has_mod = rng.random() < 0.5
        amount = rng.choice((10.0, 40.0, 3.3, 0.0))
        scale = rng.choice((1.0, 0.5, 0.3333, 2.0))
        bonus = rng.choice((0.0, 5.0, -50.0))
        effect = m.alloc(0x20)
        m.w(effect + 0x14, f32(amount))
        tgt = 0
        if has_target:
            tgt = m.alloc(0x100)
            if has_body:
                m.w(tgt + 0x60, m.alloc(0x40))
        target_hook(m, tgt)
        ctx = m.alloc(0x30)
        source = m.alloc(0x40)
        m.w(ctx + 8, source)
        mod = 0
        if has_mod:
            mod = m.alloc(0x40)
            m.w(mod + 0x14, f32(bonus))
        m.hooks[0x4CF110] = lambda mm, mod=mod, events=events, source=source: (
            events.append({'modifier_of_source': mm.reg(UC_X86_REG_ECX) == source}), mm.ret(0, mod))
        m.hooks[0x562BA0] = lambda mm, events=events, tgt=tgt, source=source: (
            events.append({'heal': mm.reg(UC_X86_REG_ECX) == mm.r(tgt + 0x60),
                           'record': [int(mm.r(mm.arg(0)) == source), mm.r(mm.arg(0) + 4)]}), mm.ret(4))
        al = m.call(0x4CA0B0, ecx=effect, args=(0, ctx, f32(scale)), pop=12) & 0xFF
        out.append({'target': has_target, 'body': has_body, 'modifier': has_mod, 'amount': f32(amount),
                    'scale': f32(scale), 'bonus': f32(bonus), 'returned': al, 'events': events})
    for a in (0x4C8330, 0x4CF110, 0x562BA0):
        del m.hooks[a]
    return out


def enchant_cases(m, rng):
    out = []
    for index in range(40):
        events = []
        has_target = rng.random() < 0.9
        has_props = rng.random() < 0.9
        existing = rng.choice(('none', 'same', 'same_other_source', 'other'))
        allied = rng.random() < 0.5
        immunity = rng.choice(('none', 'match', 'wildcard', 'other'))
        enchantment_type = 0x6100
        prop = m.alloc(0x80)
        m.w(prop + 0x64, enchantment_type)
        effect = m.alloc(0x20)
        m.w(effect, 0xA25DE4)
        m.w(effect + 0x14, prop)
        tgt = 0
        props = 0
        if has_target:
            tgt = m.handle()
            if has_props:
                props = m.alloc(0x100)
                m.w(tgt + 0x78, props)
                m.w(props + 4, tgt)
                m.w(tgt, m.vtable({0x138: lambda mm, allied=allied, events=events: (
                    events.append({'allied_query': mm.arg(0)}), mm.ret(4, int(allied)))}))
                # +10 instance list (5032D0) and +1C modifier state (+68/+6C).
                instances = m.alloc(0x10)
                count = 0
                if existing != 'none':
                    inst = m.alloc(0x20)
                    m.w(inst + 0xC, prop if existing.startswith('same') else m.alloc(0x10))
                    m.w(inst + 0x14, m.gtid(m.alloc(0x10)))
                    m.w(instances, inst)
                    count = 1
                m.w(props + 0x10, instances)
                m.w(props + 0x14, count)
                types = m.alloc(0x10)
                tcount = 0
                if immunity != 'none':
                    m.w(types, {'match': enchantment_type, 'wildcard': 0, 'other': 0x6200}[immunity])
                    tcount = 1
                m.w(props + 0x1C + 0x68, types)
                m.w(props + 0x1C + 0x6C, tcount)
        target_hook(m, tgt)
        ctx = m.alloc(0x30)
        source = m.handle()
        allegiance = m.handle()
        m.w(ctx + 8, source)
        m.w(ctx + 0xC, allegiance)

        def insert(mm, events=events, source=source, allegiance=allegiance, prop=prop):
            inst = mm.arg(0)
            w = words(mm, inst, 7)
            events.append({'insert': True, 'vtable': hex(w[0]), 'time': w[1], 'word8': w[2],
                           'property': w[3] == prop, 'byte10': mm.rb(inst + 0x10),
                           'source_gtid': w[5] == mm.r(source + 0x14),
                           'allegiance_gtid': w[6] == mm.r(allegiance + 0x14)})
            mm.ret(4)
        m.hooks[0x581630] = insert
        al = m.call(0x4CA130, ecx=effect, args=(0, ctx, f32(1.0)), pop=12) & 0xFF
        out.append({'target': has_target, 'properties': has_props, 'existing': existing, 'allied': allied,
                    'immunity': immunity, 'returned': al, 'events': events,
                    'allied_query_allegiance': any(e.get('allied_query') == allegiance for e in events)})
    for a in (0x4C8330, 0x581630):
        del m.hooks[a]
    return out


def capture_cases(m, rng):
    out = []
    for has_target, result in ((True, 1), (True, 0), (False, 1)):
        events = []
        tgt = m.handle() if has_target else 0
        target_hook(m, tgt)
        ctx = m.alloc(0x30)
        source = m.handle()
        m.w(ctx + 8, source)
        m.hooks[0x4D5230] = lambda mm, events=events, result=result, tgt=tgt, source=source: (
            events.append({'attempt': mm.reg(UC_X86_REG_ECX) == tgt, 'source': mm.arg(0) == source,
                           'record': words(mm, mm.arg(1), 7)}), mm.ret(8, result))
        al = m.call(0x4CA260, ecx=m.alloc(0x20), args=(0, ctx, f32(1.0)), pop=12) & 0xFF
        out.append({'target': has_target, 'attempt_result': result, 'returned': al, 'events': events})
    for a in (0x4C8330, 0x4D5230):
        del m.hooks[a]
    return out


def describe_cases(m, rng):
    out = []
    for has_target, has_kingdom in ((True, True), (True, False), (False, True)):
        events = []
        tgt = 0
        kingdom = 0
        if has_target:
            tgt = m.alloc(0x100)
            if has_kingdom:
                kingdom = m.alloc(0x100)

                def name_v(mm):
                    cell = mm.arg(0)
                    mm.w(cell, mm.r(mm.cstring('Germany')))
                    mm.ret(4, cell)
                m.w(kingdom, m.vtable({0x24: name_v}))
            m.w(tgt, m.vtable({0x108: lambda mm, k=kingdom: mm.ret(0, k)}))
        target_hook(m, tgt)
        ctx = m.alloc(0x30)
        allegiance = m.alloc(0x40)
        m.w(ctx + 0xC, allegiance)

        def describe(mm, events=events, kingdom=kingdom):
            cell = mm.arg(0)
            mm.w(cell, mm.r(mm.cstring('<economy>')))
            events.append({'describe': mm.reg(UC_X86_REG_ECX) == kingdom})
            mm.ret(4, cell)

        def localize(mm, events=events):
            cell = mm.arg(0)
            mm.w(cell, mm.r(mm.cstring('localized')))
            # 4B9F40 pops (out, key); the key's string arguments stay with the caller.
            events.append({'localize': True})
            mm.ret(8, cell)

        def post(mm, events=events, allegiance=allegiance):
            events.append({'post': mm.arg(1), 'allegiance': mm.arg(2) == allegiance, 'word': mm.arg(3)})
            mm.ret(0)
        m.hooks[0x4E7200] = describe
        m.hooks[0x4B9F40] = localize
        m.hooks[0x40BF70] = post
        m.hooks[0x404F60] = lambda mm, events=events: (events.append({'log': True}), mm.ret(0))
        al = m.call(0x4CC050, ecx=m.alloc(0x20), args=(0, ctx, f32(1.0)), pop=12) & 0xFF
        out.append({'target': has_target, 'kingdom': has_kingdom, 'returned': al, 'events': events})
    for a in (0x4C8330, 0x4E7200, 0x4B9F40, 0x40BF70, 0x404F60):
        del m.hooks[a]
    return out


def find_cases(m, rng):
    out = []
    for index in range(40):
        count = rng.randint(0, 4)
        definitions = [rng.choice((0xA1, 0xA2)) for _ in range(count)]
        sources = []
        objs = [m.alloc(0x10) for _ in range(3)]
        lst = m.alloc(4 * max(1, count))
        insts = []
        for i, d in enumerate(definitions):
            inst = m.alloc(0x20)
            m.w(inst + 0xC, d)
            s = rng.choice((0, 1, 2, 'dead'))
            sources.append(s)
            m.w(inst + 0x14, m.gtid(0) if s == 'dead' else m.gtid(objs[s]))
            m.w(lst + 4 * i, inst)
            insts.append(inst)
        holder = m.alloc(0x10)
        m.w(holder, lst)
        m.w(holder + 4, count)
        definition = rng.choice((0xA1, 0xA2, 0xA3))
        source = rng.choice((None, 0, 1, 2))
        exclude = rng.choice([None] + list(range(count)))
        got = m.call(0x5032D0, args=(holder, definition, objs[source] if source is not None else 0,
                                     insts[exclude] if exclude is not None else 0), pop=0)
        out.append({'definitions': definitions, 'sources': sources, 'definition': definition, 'source': source,
                    'exclude': exclude, 'result': insts.index(got) if got else None})
    wrapper = []
    for has_props in (True, False):
        actor = m.alloc(0x100)
        if has_props:
            props = m.alloc(0x40)
            inst = m.alloc(0x20)
            m.w(inst + 0xC, 0xA1)
            lst = m.alloc(4)
            m.w(lst, inst)
            m.w(props + 0x10, lst)
            m.w(props + 0x14, 1)
            m.w(actor + 0x78, props)
        got = m.call(0x4CEF80, ecx=actor, args=(0xA1, 0), pop=8)
        wrapper.append({'properties': has_props, 'found': got != 0})
    return out, wrapper


def expiry_cases(m, rng):
    """Enchantment instance expiry 5031B0 (vtable A2EBE0 slot 0) natively."""
    out = []
    values = [0.0, 1.0, 29.999998, 30.0, 30.000002, 120.0, 1e7, 16777217.0, 0.1, 100.25]
    for now in values:
        for created in (0.0, 0.1, 30.0, 100.25, 1e7):
            for duration in (0.0, 30.0, 120.0, -1.0, 0.1):
                inst = m.alloc(0x20)
                prop = m.alloc(0x80)
                m.w(prop + 0x60, f32(duration))
                m.w(inst + 4, f32(created))
                m.w(inst + 0xC, prop)
                m.w(m.world_obj + 0xE0, f32(now))
                al = m.call(0x5031B0, ecx=inst, pop=0) & 0xFF
                out.append({'now': f32(now), 'created': f32(created), 'duration': f32(duration), 'expired': al})
    m.w(m.world_obj + 0xE0, f32(123.5))
    return out


def main():
    rng = random.Random(0x4CDE00)
    m = Oracle()
    faction = faction_cases(m, rng)
    heal_mod = heal_modifier_cases(m, rng)
    heal = heal_cases(m, rng)
    enchant = enchant_cases(m, rng)
    capture = capture_cases(m, rng)
    describe = describe_cases(m, rng)
    find, wrapper = find_cases(m, rng)
    expiry = expiry_cases(m, rng)
    h.report('native-special-ops-effects-fixtures.json',
             'Faction UseAbility 4CDE00 and the effect Apply virtuals (round 5); seams per producer docstring.',
             faction, m.visited, extra={'heal_modifier': heal_mod, 'heal': heal, 'enchant': enchant,
                                        'capture': capture, 'describe': describe, 'find': find,
                                        'find_wrapper': wrapper, 'expiry': expiry})
    print(len(faction), len(heal_mod), len(heal), len(enchant), len(capture), len(describe), len(find))


if __name__ == '__main__':
    main()
