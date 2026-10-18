"""Native oracle for ModifierState Add/RemovePropertyModifiers (round 6):
502B30 cases 3/4/6/7/8/9 and 502C60, executed natively with the real
modifier vtables (4FE840 classes): immunity push (483A20) and its +20 hook
4FE6F0 (purge of hostile enchantments of that type), ability-effect lists
502AD0/502700/5021E0 and 501EF0, periodic damage records 502560/5015C0 and
501820, heals (766DE0) and 5018C0, reverse abilities 502640/4C86D0 and
5023B0/4CE080, resource production 501F60 (50B530/45F180).
Recorded seams: 581810 (property removal; the hook removes the selected
entry from the list as 581810 does), actor virtual +138 (allied), 4BE3F0
(ability main power), 4CF0F0 (actor refresh), 404F90 (log), heap.
"""
from pathlib import Path
import random
import struct
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import harness as h  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX  # noqa: E402

f32 = h.f32
VT = {3: 0xA2D6C4, 4: 0xA2D6E8, 6: 0xA2D73C, 7: 0xA2D75C, 8: 0xA2D77C, 9: 0xA2D7D0}
ENCHANT_VT, PLAIN_VT = 0xA2EBE0, 0xA29678
RESOURCES = 3


class Oracle(h.Machine):
    def __init__(self):
        super().__init__('Double53')
        self.u.mem_map(0, 0x1000)
        self.hooks[0x6F7810] = lambda m: m.ret(0, m.alloc(max(m.arg(0), 4)))
        self.hooks[0x6F7430] = lambda m: m.ret(0)
        self.registry = self.alloc(0x40000)
        self.w(h.REGISTRY_ROOT, self.registry)
        self.slot = 0x20
        self.data = self.alloc(0x1000)
        self.w(0xB36530, self.data)
        self.w(self.data + 0x38, RESOURCES)
        self.events = []
        self.names = {}

    def alloc(self, size, align=16):
        ptr = super().alloc(size, align)
        self.u.mem_write(ptr, bytes(size))
        return ptr

    def gtid(self, obj):
        self.slot += 1
        self.u.mem_write(self.registry + 4 + self.slot * 2, (5).to_bytes(2, 'little'))
        if obj:
            self.w(self.registry + 0x20004 + self.slot * 4, obj)
        return (5 << 16) | self.slot

    def obj(self, name, size=0x100):
        o = self.alloc(size)
        self.w(o + 0x14, self.gtid(o))
        self.names[o] = name
        return o

    def label(self, v):
        return self.names.get(v, v) if v else 0


def vec(m, at):
    data, count = m.r(at), m.r(at + 4)
    return [m.r(data + 4 * i) for i in range(count)] if data else []


def snapshot(m, state):
    immunity = [m.label(v) for v in vec(m, state + 0x68)]
    pdata, pcount = m.r(state + 0x80), m.r(state + 0x84)
    production = [m.r(pdata + 4 * i) for i in range(pcount)] if pdata else []
    lists = []
    ldata, lcount = m.r(state + 0x8C), m.r(state + 0x90)
    for i in range(lcount):
        node, items = m.r(ldata + 8 * i), []
        while node:
            items.append(m.label(m.r(node)))
            node = m.r(node + 4)
        lists.append(items)
    damage = [[m.label(m.r(r)), m.r(r + 4), m.r(r + 8)] for r in vec(m, state + 0x98)]
    heals = [m.label(v) for v in vec(m, state + 0xA4)]
    reverse = [[m.label(m.r(a + 4)), m.r(a + 0x20), m.r(a + 8)] for a in vec(m, state + 0xB0)]
    return {'immunity': immunity, 'production': production, 'ability_effects': lists,
            'periodic_damage': damage, 'periodic_heals': heals, 'reverse_abilities': reverse}


def main():
    rng = random.Random(0x502B30)
    m = Oracle()
    types = [m.obj(f'ench_type{i}', 0x20) for i in range(3)]
    ability_types = []
    for i in range(4):
        t = m.obj(f'ability_type{i}', 0x20)
        m.w(t + 0xC, i * 2)          # registry index (sparse)
        ability_types.append(t)
    effects = [m.obj(f'effect{i}', 0x20) for i in range(4)]
    damage_types = [m.obj(f'damage{i}', 0x20) for i in range(2)]
    definitions = [m.obj(f'ability_def{i}', 0x80) for i in range(3)]
    m.hooks[0x4BE3F0] = lambda mm: (mm.ret_f32(4, f32(7.5)))
    m.hooks[0x404F90] = lambda mm: mm.ret(0)
    cases = []
    for scenario in range(120):
        events = []
        m.events = events
        actor = m.obj('actor', 0x200)
        allied_with = set()
        m.w(actor, m.vtable({0x138: lambda mm, s=allied_with, ev=events: (
            ev.append({'allied_query': mm.label(mm.arg(0))}), mm.ret(4, int(mm.arg(0) in s)))}))
        props = m.alloc(0x40)
        m.w(actor + 0x78, props)
        lst = m.alloc(4 * 64)
        m.w(props + 0x10, lst)
        m.w(props + 0x14, 0)
        state = m.alloc(0x100)
        m.hooks[0x4CF0F0] = lambda mm, ev=events: (ev.append({'refresh': mm.label(mm.reg(UC_X86_REG_ECX))}),
                                                   mm.ret(0))
        sources = [m.obj(f'source{i}') for i in range(2)]
        allegiances = [m.obj(f'kingdom{i}') for i in range(2)]
        if rng.random() < 0.6:
            allied_with.add(allegiances[0])

        def remove(mm, ev=events, props=props):
            prop, source = mm.arg(0), mm.arg(1)
            ev.append({'remove': mm.label(prop), 'source': mm.label(source)})
            data, count = mm.r(props + 0x10), mm.r(props + 0x14)
            for i in range(count):
                inst = mm.r(data + 4 * i)
                s = mm.r(inst + 0x14) & 0xFFFF
                resolved = mm.r(mm.registry + 0x20004 + s * 4) if mm.u.mem_read(
                    mm.registry + 4 + s * 2, 2) == (mm.r(inst + 0x14) >> 16).to_bytes(2, 'little') else 0
                if mm.r(inst + 0xC) == prop and resolved == source:
                    for j in range(i, count - 1):
                        mm.w(data + 4 * j, mm.r(data + 4 * (j + 1)))
                    mm.w(props + 0x14, count - 1)
                    break
            mm.ret(8, 1)
        m.hooks[0x581810] = remove

        # Existing enchantment instances on the actor (purge candidates).
        existing = []
        for i in range(rng.randint(0, 5)):
            pdef = m.obj(f'existing_prop{i}', 0x80)
            m.w(pdef + 0x64, rng.choice(types[:2] + [types[0], 0]))
            inst = m.alloc(0x20)
            m.w(inst, rng.choice((ENCHANT_VT, PLAIN_VT)))
            m.w(inst + 0xC, pdef)
            m.w(inst + 0x14, m.r(rng.choice(sources) + 0x14))
            m.w(inst + 0x18, m.r(rng.choice(allegiances) + 0x14) if rng.random() < 0.8 else 0)
            m.w(lst + 4 * i, inst)
            existing.append({'property': f'existing_prop{i}', 'type': m.label(m.r(pdef + 0x64)),
                             'enchantment': m.r(inst) == ENCHANT_VT,
                             'source': m.label(rng and m.r(m.registry + 0x20004 + (m.r(inst + 0x14) & 0xFFFF) * 4)),
                             'allegiance': m.label(m.r(m.registry + 0x20004 + (m.r(inst + 0x18) & 0xFFFF) * 4))
                             if m.r(inst + 0x18) else 0})
        m.w(props + 0x14, len(existing))
        # Property definitions with modifier lists and their instances.
        props_defs = []
        for p in range(rng.randint(1, 3)):
            pdef = m.obj(f'prop{p}', 0x80)
            m.w(pdef + 0x64, rng.choice(types + [0]))
            mods = []
            for k in range(rng.randint(1, 4)):
                kind = rng.choice((3, 4, 6, 7, 8, 9))
                mod = m.alloc(0x40)
                m.names[mod] = f'prop{p}_mod{k}'
                m.w(mod, VT[kind])
                m.w(mod + 8, kind)
                spec = {'kind': kind, 'name': f'prop{p}_mod{k}'}
                if kind == 3:
                    t = rng.choice(types[:2] + [types[0], 0])
                    m.w(mod + 0xC, t)
                    spec['type'] = m.label(t)
                elif kind == 4:
                    at = rng.choice(ability_types)
                    ef = rng.choice(effects)
                    m.w(mod + 0xC, at)
                    m.w(mod + 0x10, ef)
                    spec['ability_type_index'] = m.r(at + 0xC)
                    spec['effect'] = m.label(ef)
                elif kind == 6:
                    dt = rng.choice(damage_types)
                    m.w(mod + 0xC, dt)
                    m.w(mod + 0x10, f32(rng.choice((5.0, 12.5))))
                    spec['damage_type'] = m.label(dt)
                    spec['amount'] = m.r(mod + 0x10)
                elif kind == 7:
                    m.w(mod + 0xC, f32(rng.choice((2.0, 3.25))))
                    spec['amount'] = m.r(mod + 0xC)
                elif kind == 8:
                    d = rng.choice(definitions)
                    m.w(mod + 0xC, d)
                    spec['definition'] = m.label(d)
                else:
                    values = [f32(rng.choice((1.5, -0.25, 1e-3, 1e8, 3.0))) for _ in range(RESOURCES)]
                    arr = m.alloc(4 * RESOURCES)
                    for i, v in enumerate(values):
                        m.w(arr + 4 * i, v)
                    m.w(mod + 0xC, arr)
                    m.w(mod + 0x10, RESOURCES)
                    m.w(mod + 0x14, RESOURCES)
                    spec['values'] = values
                mods.append((mod, spec))
            head = 0
            for mod, _ in reversed(mods):
                node = m.alloc(8)
                m.w(node, mod)
                m.w(node + 4, head)
                head = node
            m.w(pdef + 0x1C, head)
            enchant = rng.random() < 0.6
            inst = m.alloc(0x20)
            m.w(inst, ENCHANT_VT if enchant else PLAIN_VT)
            m.w(inst + 0xC, pdef)
            src = rng.choice(sources + [0])
            m.w(inst + 0x14, m.r(src + 0x14) if src else 0)
            al = rng.choice(allegiances + [0])
            m.w(inst + 0x18, m.r(al + 0x14) if al else 0)
            props_defs.append({'property': f'prop{p}', 'type': m.label(m.r(pdef + 0x64)), 'enchantment': enchant,
                               'source': m.label(src), 'allegiance': m.label(al), 'modifiers': [s for _, s in mods],
                               'instance': inst})
        steps = []
        order = [('add', i) for i in range(len(props_defs))]
        removes = [('remove', i) for i in range(len(props_defs))]
        rng.shuffle(removes)
        order += removes[:rng.randint(0, len(removes))]
        for op, i in order:
            events.clear()
            inst = props_defs[i]['instance']
            with_actor = rng.random() < 0.85
            if op == 'add':
                m.call(0x502B30, ecx=state, args=(inst, props + 0x10, actor if with_actor else 0), pop=12)
            else:
                m.call(0x502C60, ecx=state, args=(inst, props + 0x10, actor if with_actor else 0), pop=12)
            remaining = [m.label(m.r(m.r(lst + 4 * k) + 0xC)) for k in range(m.r(props + 0x14))]
            steps.append({'op': op, 'property': i, 'actor': with_actor, 'events': list(events),
                          'state': snapshot(m, state), 'actor_properties': remaining})
        for p in props_defs:
            del p['instance']
        cases.append({'existing': existing, 'properties': props_defs, 'allied': [m.label(a) for a in allied_with],
                      'steps': steps})
    h.report('native-modifier-tables-fixtures.json', 'ModifierState Add/RemovePropertyModifiers (round 6).',
             cases, m.visited)


if __name__ == '__main__':
    main()
