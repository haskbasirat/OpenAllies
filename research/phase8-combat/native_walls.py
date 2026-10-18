"""Native oracle for the wall / denizen / membership seams (round 4):
0x585160, 0x585AE0, 0x586370, 0x585A20, 0x56ACB0, 0x569540 (+0x569300) and
0x570670. Original code runs; recorded seams: 0x4D64F0, 0x42D490 (actor
virtuals +24/+108), 0x4D1350, 0x517FA0, 0x5859C0, 0x4D7960, 0x473300,
component +70 virtuals +40/+18C, link virtual 0, 0x585280, 0x567650,
0x4CF0F0, 0x7A0C20, heap. GTIDs resolve through the real registry layout.
"""
from pathlib import Path
import random
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import harness as h  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX  # noqa: E402


class Oracle(h.Machine):
    def __init__(self):
        super().__init__('Double53')
        self.u.mem_map(0, 0x1000)
        self.events = []
        self.ids = {}
        self.registry = self.alloc(0x40000)
        self.w(h.REGISTRY_ROOT, self.registry)
        self.slot = 0x10
        self.manager = self.alloc(0x10)
        self.w(self.manager, self.vtable({0x10: lambda m: m.ret(0, m.reg(UC_X86_REG_ECX))}))
        self.w(0xB39D8C, 0xABCD0001)
        rec = lambda name, pop, value=0, args=0: (lambda m: (m.events.append(
            {name: m.label(m.reg(UC_X86_REG_ECX)), 'args': [m.label(m.arg(i)) for i in range(args)]}),
            m.ret(pop, value)))
        self.hooks.update({
            0x4D64F0: lambda m: (m.events.append({'state': m.label(m.reg(UC_X86_REG_ECX)), 'value': m.arg(0),
                                                  'enabled': m.arg(1) & 0xFF}), m.ret(8)),
            0x42D490: lambda m: (m.events.append({'announce': m.arg(1)}), m.ret(0)),
            0x4D1350: rec('notify', 0),
            0x517FA0: lambda m: (m.events.append({'flag13': m.label(m.reg(UC_X86_REG_ECX)), 'bit': m.arg(0),
                                                  'value': m.arg(1) & 0xFF}), m.ret(8)),
            0x5859C0: rec('refresh_segments', 0, 1),
            0x4D7960: rec('begin_death', 0),
            0x473300: lambda m: (m.events.append({'link_node_dtor': True}), m.ret(0)),
            0x6F7430: lambda m: m.ret(0),
            0x6F7810: lambda m: m.ret(0, m.alloc(max(m.arg(0), 4))),
            0x585280: lambda m: (m.events.append({'within_wall': m.label(m.arg(0))}),
                                 m.ret(4, int(m.label(m.arg(0)) in m.within))),
            0x567650: rec('detach_b0', 0),
            0x4CF0F0: rec('refresh_member', 0),
            0x7A0C20: lambda m: (m.events.append({'observer': m.label(m.arg(0))}), m.ret(4)),
        })

    def alloc(self, size, align=16):
        ptr = super().alloc(size, align)
        self.u.mem_write(ptr, bytes(size))
        return ptr

    def label(self, address):
        return self.ids.get(address, address)

    def actor(self, name, flags8=0, flags100=0):
        a = self.alloc(0x200)
        self.ids[a] = name
        self.w(a + 8, flags8)
        self.w(a + 0x100, flags100)

        def name_v(m):
            header = m.alloc(0x20)
            m.w(header, m.manager)
            m.w(header + 0xC, 1000)
            m.w(m.arg(0), header + 0x10)
            m.ret(4, m.arg(0))
        self.w(a, self.vtable({0x24: name_v, 0x108: lambda m: m.ret(0, 0x5150)}))
        return a

    def gtid(self, actor):
        self.slot += 1
        self.u.mem_write(self.registry + 4 + self.slot * 2, (3).to_bytes(2, 'little'))
        if actor:
            self.w(self.registry + 0x20004 + self.slot * 4, actor)
        return (3 << 16) | self.slot


def wall(m, rng, owner_flags, actor_flags, segments, required):
    w = m.alloc(0x40)
    actor = m.actor('wall_actor', flags100=actor_flags)
    m.w(w + 4, actor)
    definition = m.alloc(0x500)
    m.w(actor + 4, definition)
    table = m.alloc(0x40)
    m.w(definition + 0x464, table)
    m.w(w + 0x10, 2)
    m.w(table + 2 * 12, required & 0xFFFFFFFF)
    owner = m.actor('owner', flags100=owner_flags) if owner_flags is not None else 0
    m.w(w + 0x14, owner)
    arr = m.alloc(4 * max(1, len(segments)))
    for i, s in enumerate(segments):
        if s == 'owner':
            m.w(arr + 4 * i, owner)
        elif s is not None:
            m.w(arr + 4 * i, m.actor(f'segment{i}'))
    m.w(w + 0x18, arr)
    m.w(w + 0x1C, len(segments))
    return w


def main():
    rng = random.Random(0x586370)
    m = Oracle()
    complete = []
    for count in (0, 1, 3, -1):
        for pattern in range(6):
            flags = [rng.choice((None, 0x400, 0x401, 0, 0x8)) for _ in range(max(count, 0))]
            w = m.alloc(0x40)
            arr = m.alloc(4 * max(1, len(flags)))
            for i, f in enumerate(flags):
                if f is not None:
                    a = m.alloc(0x40)
                    m.w(a + 8, f)
                    m.w(arr + 4 * i, a)
            m.w(w + 0x24, arr)
            m.w(w + 0x28, count & 0xFFFFFFFF)
            complete.append({'count': count, 'flags': flags, 'result': m.call(0x585160, ecx=w, pop=0) & 0xFF})
    sieges = []
    for i in range(40):
        owner_flags = rng.choice((0, 0x100000, 0x40000000, 0x40100000))
        actor_flags = rng.choice((0, 0x100000))
        segments = [rng.choice((None, 'seg', 'owner')) for _ in range(rng.randint(0, 4))]
        required = rng.choice((0, 2, 5, -1))
        m.events = []
        w = wall(m, rng, owner_flags, actor_flags, segments, required)
        m.call(0x586370, ecx=w, pop=0)
        sieges.append({'owner_flags': owner_flags, 'actor_flags': actor_flags, 'segments': segments,
                       'required': required, 'events': m.events})
    transfers = []
    for i in range(30):
        m.events = []
        comp = m.alloc(0x60)
        owner = m.actor('denizen_owner')
        m.w(comp + 4, owner)
        links = []
        refcounts = []
        head = 0
        spec = [[rng.choice(('accept', 'reject', 'no70', 'dead')) for _ in range(rng.randint(0, 3))]
                for _ in range(rng.randint(0, 3))]
        for li, link in reversed(list(enumerate(spec))):
            obj = m.alloc(0x20)
            m.w(obj, m.vtable({0: lambda mm, li=li: (mm.events.append({'link_delete': li}), mm.ret(4))}))
            refs = rng.choice((1, 2))
            refcounts.insert(0, refs)
            m.w(obj + 4, refs)
            gl = 0
            for ai, kind in reversed(list(enumerate(link))):
                if kind == 'dead':
                    g = m.gtid(0)
                else:
                    a = m.actor(f'l{li}a{ai}')
                    if kind != 'no70':
                        c70 = m.alloc(0x20)
                        m.w(c70, m.vtable({
                            0x40: lambda mm, n=f'l{li}a{ai}': (mm.events.append({'c70_40': n, 'arg': mm.arg(0)}),
                                                              mm.ret(4)),
                            0x18C: lambda mm, n=f'l{li}a{ai}', ok=(kind == 'accept'): (
                                mm.events.append({'c70_18c': n, 'owner': mm.label(mm.arg(0))}), mm.ret(4, int(ok)))}))
                        m.w(a + 0x70, c70)
                    g = m.gtid(a)
                gn = m.alloc(8)
                m.w(gn, g)
                m.w(gn + 4, gl)
                gl = gn
            m.w(obj + 0xC, gl)
            node = m.alloc(8)
            m.w(node, obj)
            m.w(node + 4, head)
            head = node
            links.append(obj)
        m.w(comp + 0x40, head)
        chain = 0
        for _ in range(rng.randint(0, 2)):
            n = m.alloc(0x20)
            m.w(n, 0x1111)
            m.w(n + 8, 0x2222)
            m.w(n + 0x10, chain)
            chain = n
        m.w(comp + 0x18, chain)
        if i % 3 == 0:
            w = m.alloc(0x40)
            m.ids[w] = 'wall'
            o = m.actor('wall_owner')
            m.w(w + 0x14, o)
            m.w(o + 0x88, comp)
            m.call(0x585A20, ecx=w, args=(0,), pop=4)
            entry = 'wall'
        else:
            m.call(0x56ACB0, ecx=comp, args=(0,), pop=4)
            entry = 'component88'
        cleared = chain == 0 or (m.r(chain) == 0 and m.r(chain + 8) == 0)
        transfers.append({'entry': entry, 'links': spec, 'refcounts': refcounts, 'events': m.events, 'list_cleared': m.r(comp + 0x40) == 0,
                          'chain_cleared': cleared})
    intruders = []
    for i in range(30):
        m.events = []
        comp = m.alloc(0x40)
        spec = [rng.choice(('alive', 'dying', 'dead')) for _ in range(rng.randint(0, 4))]
        m.within = set()
        head = 0
        names = []
        for k, kind in reversed(list(enumerate(spec))):
            if kind == 'dead':
                g = m.gtid(0)
            else:
                a = m.actor(f'i{k}', flags8=1 if kind == 'dying' else 0)
                if rng.random() < 0.4:
                    m.within.add(f'i{k}')
                g = m.gtid(a)
            node = m.alloc(8)
            m.w(node, g)
            m.w(node + 4, head)
            head = node
        m.w(comp + 0x24, head)
        wall_actor = m.actor('wall')
        m.w(wall_actor + 0x98, m.alloc(0x40))
        al = m.call(0x569540, ecx=comp, args=(wall_actor,), pop=4) & 0xFF
        intruders.append({'list': spec, 'within': sorted(m.within), 'result': al, 'events': m.events,
                          'remaining': m.r(comp + 0x24) != 0})
    detaches = []
    for i in range(16):
        m.events = []
        member = m.actor('member', flags8=rng.choice((0, 1)))
        if rng.random() < 0.5:
            m.w(member + 0xB0, m.alloc(0x20))
            m.ids[m.r(member + 0xB0)] = 'b0'
        membership = m.alloc(0x40)
        m.w(membership + 4, member)
        m.w(membership + 0x10, 0x55)
        m.w(membership + 0x18, 3)
        m.wb(membership + 0x1C, 1)
        observer = rng.random() < 0.6
        m.w(0xB36528, m.alloc(0x10) if observer else 0)
        m.call(0x570670, ecx=membership, args=(7, 8), pop=8)
        detaches.append({'flags8': m.r(member + 8), 'has_b0': m.r(member + 0xB0) != 0, 'observer': observer,
                         'events': m.events, 'cleared': [m.r(membership + 0x10), m.r(membership + 0x18),
                                                         m.rb(membership + 0x1C)]})
    h.report('native-wall-fixtures.json', 'Wall/denizen/membership seams (round 4).', complete, m.visited,
             extra={'sieges': sieges, 'transfers': transfers, 'intruders': intruders, 'detaches': detaches})


if __name__ == '__main__':
    main()
