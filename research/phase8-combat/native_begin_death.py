"""Native oracle for Actor BeginDeath 0x4D7960.

The original function runs unchanged; every callee is a recorded seam:
kill credit 4D57C0, zone release 514CE0 (514CD0 and 58CB30 run natively),
wall check 5853B0, announcement 42D490 (with actor virtuals +24/+108),
state 4D64F0, siege refresh 4D6660, component virtual +34, 568B70,
wall member 5859C0/584D50 (virtuals +158/+15C), 4EE8E0, 4D1350, 4D37E0,
virtual +50, start dying 562380 and removal request 4F6760. The +70 GTID goes
through the real registry lookup.
"""
from pathlib import Path
import random
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import harness as h  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX  # noqa: E402

COMPONENT_OFFSETS = [0x60 + 4 * i for i in range(31)]


class Oracle(h.Machine):
    def __init__(self):
        super().__init__('Double53')
        self.u.mem_map(0, 0x1000)
        self.events = []
        self.registry = self.alloc(0x40000)
        self.w(h.REGISTRY_ROOT, self.registry)
        self.world = self.alloc(0x100)
        self.w(h.WORLD_ROOT, self.world)
        self.w(0xB39D8C, 0xABCD0001)
        self.manager = self.alloc(0x10)
        self.w(self.manager, self.vtable({0x10: lambda m: m.ret(0, m.reg(UC_X86_REG_ECX))}))
        hooks = {
            0x4D57C0: lambda m: (m.events.append({'credit': True}), m.ret(0, m.credited)),
            0x514CE0: lambda m: (m.events.append({'zone_release': m.entries.index(m.reg(UC_X86_REG_ECX))}), m.ret(0)),
            0x5853B0: lambda m: (m.events.append({'wall_check': True}), m.ret(0, m.wall_check)),
            0x42D490: lambda m: (m.events.append({'announce': m.arg(1), 'manager': m.arg(0) == 0xABCD0001}),
                                 m.ret(0)),
            0x4D64F0: lambda m: (m.events.append({'state': m.arg(0), 'enabled': m.arg(1) & 0xFF}), m.ret(8)),
            0x4D6660: lambda m: (m.events.append({'siege': True}), m.ret(0)),
            0x568B70: lambda m: (m.events.append({'component88_death': m.arg(0) == m.actor}), m.ret(4)),
            0x5859C0: lambda m: (m.events.append({'wall_5859c0': True}), m.ret(0)),
            0x584D50: lambda m: (m.events.append({'wall_584d50': m.arg(0)}), m.ret(4)),
            0x4EE8E0: lambda m: (m.events.append({'stop': m.arg(0)}), m.ret(4)),
            0x4D1350: lambda m: (m.events.append({'notify': True}), m.ret(0)),
            0x4D37E0: lambda m: (m.events.append({'refresh_37e0': True}), m.ret(0)),
            0x562380: lambda m: (m.events.append({'start_dying': True}), m.ret(0)),
            0x4F6760: lambda m: (m.events.append({'removal': m.arg(0) == m.actor}), m.ret(8, m.removed)),
        }
        self.hooks.update(hooks)

    def alloc(self, size, align=16):
        ptr = super().alloc(size, align)
        self.u.mem_write(ptr, bytes(size))
        return ptr

    def cstring(self, text):
        raw = text.encode('utf-16le')
        header = self.alloc(0x10 + len(raw) + 2)
        self.w(header, self.manager)
        self.w(header + 4, len(text))
        self.w(header + 8, len(text))
        self.w(header + 0xC, 1000)
        self.u.mem_write(header + 0x10, raw + b'\0\0')
        return header + 0x10


def case(m, rng, index):
    c = {
        'flags8': rng.choice((0, 0, 0x20000000, 0x10, 1)),
        'flags100': rng.choice((0, 0, 0x20000000, 0x01000000)),
        'zones': [rng.choice((None, 0, 0x42, 0x2, 0x40, 0x100)) for _ in range(rng.randint(0, 3))],
        'components': sorted(rng.sample(COMPONENT_OFFSETS, rng.randint(0, 8))),
        'c80_word10': rng.random() < 0.3,
        'c0_byte38': rng.random() < 0.5,
        'wall_owner': rng.random() < 0.5,
        'wall_check': rng.random() < 0.5,
        'virtual_100': rng.choice((None, 0, 1)),
        'virtual_20': rng.random() < 0.6,
        'linked88': rng.random() < 0.5,
        'wall_member': rng.random() < 0.3,
        'def_1b2': rng.random() < 0.3,
        'credited': rng.choice((0, 0x7777)),
        'removed': rng.random() < 0.5,
    }
    if index % 5 == 0:
        c['components'] = sorted(set(c['components']) | {0x90, 0x80, 0xC0, 0x94, 0x64, 0x60, 0x70, 0xA4})
    m.events = []
    actor = m.alloc(0x200)
    m.actor = actor
    m.credited = c['credited']
    m.wall_check = int(c['wall_check'])
    m.removed = int(c['removed'])
    definition = m.alloc(0x200)
    m.w(actor + 4, definition)
    m.wb(definition + 0x1B2, int(c['def_1b2']))
    m.w(actor + 8, c['flags8'])
    m.w(actor + 0x100, c['flags100'])
    entries = []
    table = m.alloc(4 * max(1, len(c['zones'])))
    for i, word in enumerate(c['zones']):
        if word is None:
            continue
        e = m.alloc(0x20)
        m.u.mem_write(e + 0x14, word.to_bytes(2, 'little'))
        m.w(table + 4 * i, e)
        entries.append(e)
    m.entries = entries
    m.w(actor + 0x148, table)
    m.w(actor + 0x14C, len(c['zones']))
    for offset in c['components']:
        comp = m.alloc(0x100)
        m.w(comp, m.vtable({0x34: lambda mm, o=offset: (mm.events.append(
            {'component_death': o, 'credited': mm.arg(0)}), mm.ret(4))}))
        m.w(actor + offset, comp)
        if offset == 0x80:
            m.w(comp + 0x10, int(c['c80_word10']))
        if offset == 0xC0:
            m.wb(comp + 0x38, int(c['c0_byte38']))
        if offset == 0x90 and c['wall_owner']:
            owner = m.alloc(0x100)
            m.w(owner + 0x98, m.alloc(0x40))
            m.w(comp + 0x10, owner)
        if offset == 0x70:
            slot = 0x100 + index
            m.u.mem_write(m.registry + 4 + slot * 2, (7).to_bytes(2, 'little'))
            target = m.alloc(0x100)
            m.w(m.registry + 0x20004 + slot * 4, target)
            m.w(comp + 0x28, (7 << 16) | slot)
            if c['linked88']:
                m.w(target + 0x88, m.alloc(0x40))
    v100 = None
    if c['virtual_100'] is not None:
        v100 = m.alloc(0x20)
        m.w(v100 + 8, c['virtual_100'])

    def name(mm):
        mm.events.append({'name': True})
        mm.w(mm.arg(0), mm.cstring('unit'))
        mm.ret(4, mm.arg(0))
    m.w(actor, m.vtable({
        0x20: lambda mm: mm.ret(4, int(c['virtual_20'])),
        0x24: name,
        0x50: lambda mm: (mm.events.append({'virtual_50': mm.arg(0)}), mm.ret(4)),
        0x100: lambda mm: mm.ret(0, v100 or 0),
        0x108: lambda mm: (mm.events.append({'v108': True}), mm.ret(0, 0x5150)),
        0x158: lambda mm: mm.ret(0, int(c['wall_member'])),
        0x15C: lambda mm: (lambda w: (mm.w(w + 0x98, mm.alloc(0x40)), mm.ret(0, w)))(mm.alloc(0x100)),
    }))
    m.call(0x4D7960, ecx=actor)
    return {'input': c, 'events': m.events, 'flags8_after': m.r(actor + 8)}


def main():
    rng = random.Random(0x4D7960)
    m = Oracle()
    cases = [case(m, rng, i) for i in range(160)]
    h.report('native-begin-death-fixtures.json', 'Actor BeginDeath 0x4D7960 (callees recorded).', cases,
             m.visited)


if __name__ == '__main__':
    main()
