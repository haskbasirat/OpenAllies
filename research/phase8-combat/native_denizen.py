"""Original DenizenComponent damage response under Unicorn (component +88).

Executes unchanged 56AE70 (OnDamaged: resolve the record source), 569A80
(intruder within the denizen range: definition +364/+368 nonzero, attached
organization members via the real 4364C0 iterator, binary32 squared distance
against the extended squared +368 range), 568D80 (OnIntruder: attached member
redirect to its organization, 4D0380, hostility, flags8 bit14, 517F30
membership test, 4CF1B0 handle and 4D1970/4D0900 head insertion into the +24
intruder list) and 529C30.
Seams: allocator 6F7810, owner virtual +108 (allegiance), intruder virtuals
+84 (4D0380 relation) and +140 (hostility), 404F90 debug log.
"""
import copy
import struct
from harness import Machine, PRECISIONS, f32, install_settings, install_world, report, REGISTRY_ROOT
from unicorn.x86_const import UC_X86_REG_ECX


class Run(Machine):
    def __init__(self, c, precision):
        super().__init__(precision)
        self.c = copy.deepcopy(c)
        c = self.c
        self.registry = self.alloc(0x60010)
        self.w(REGISTRY_ROOT, self.registry)
        install_settings(self, {})
        install_world(self)
        self.tags = {}
        self.actors = {}
        for tag in c['actors']:
            a = self.alloc(0x200)
            self.w(a + 4, self.alloc(0x600))
            self.w(a + 0x14, 0x10000 * int(tag) + int(tag))
            self.tags[a] = int(tag)
            self.actors[int(tag)] = a
        for tag, v in c['actors'].items():
            self.setup(int(tag), v)
        self.comp = self.alloc(0x40)
        owner = self.actors[1]
        self.w(self.comp + 4, owner)
        d = self.r(owner + 4)
        self.w(d + 0x364, c['range364'])
        self.w(d + 0x368, c['range368'])
        head = 0
        for gtid in reversed(c['intruders']):
            node = self.alloc(8)
            self.w(node, gtid)
            self.w(node + 4, head)
            head = node
        self.w(self.comp + 0x24, head)
        tail = head
        while tail and self.r(tail + 4):
            tail = self.r(tail + 4)
        self.w(self.comp + 0x28, tail)
        self.hooks.update({0x6f7810: lambda m: m.ret(0, m.alloc(max(m.arg(), 16))),
                           0x404f90: self.log})

    def setup(self, tag, v):
        a = self.actors[tag]
        self.w(a + 8, v.get('flags8', 0))
        self.w(a + 0x20, v.get('xy', [0, 0])[0])
        self.w(a + 0x24, v.get('xy', [0, 0])[1])
        if v.get('body'):
            self.w(a + 0x60, self.alloc(0x40))
        if v.get('members') is not None:
            org = self.alloc(0x100)
            self.w(a + 0x7c, org)
            members = v['members']
            arr = self.alloc(4 * max(1, len(members)))
            self.w(org + 0x28, arr)
            self.w(org + 0x2c, len(members))
            for i, m in enumerate(members):
                self.w(arr + 4 * i, self.actors[m] if m else 0)
        if v.get('membership') is not None:
            ms = self.alloc(0x40)
            self.w(a + 0x80, ms)
            org = v['membership']['organization']
            self.w(ms + 0x10, self.actors[org] if org else 0)
            self.wb(ms + 0x1c, int(v['membership']['attached']))
        if v.get('registered', True):
            gtid = 0x10000 * tag + tag
            slot = gtid & 0xffff
            self.u.mem_write(self.registry + 4 + slot * 2, struct.pack('<H', gtid >> 16))
            self.w(self.registry + 0x20004 + slot * 4, a)

        def allegiance(m, v=v, tag=tag):
            self.events.append({'Allegiance': tag})
            owner = v.get('allegiance', 0)
            self.ret(0, self.actors.get(owner, 0))

        def relation(m, v=v, tag=tag):
            self.events.append({'Relation84': [tag, self.tags.get(self.arg(), 0)]})
            self.ret(4, int(v.get('relation', True)))

        def hostile(m, v=v, tag=tag):
            self.events.append({'Hostile140': [tag, self.tags.get(self.arg(), 0)]})
            self.ret(4, int(v.get('hostile', True)))
        self.w(a, self.vtable({0x108: allegiance, 0x84: relation, 0x140: hostile}))

    def log(self, m):
        self.events.append('AddingIntruder')
        self.ret(0)

    def run(self):
        record = self.alloc(0x20)
        self.w(record, self.c['source'])
        self.call(0x56ae70, self.comp, (record,))
        out = []
        node = self.r(self.comp + 0x24)
        while node:
            out.append(self.r(node))
            node = self.r(node + 4)
        tail = self.r(self.comp + 0x28)
        return {'intruders': out, 'tail': self.r(tail) if tail else None, 'events': self.events}


def cases():
    rows = []
    g = lambda t: 0x10000 * t + t
    owner = dict(xy=[f32(0), f32(0)], allegiance=9)
    for name, actors, source, r364, r368, intruders in [
        ('inside', {'2': dict(xy=[f32(3), f32(4)], body=True, flags8=0x4000)}, g(2), 1.0, 5.0, []),
        ('edge', {'2': dict(xy=[f32(3), f32(4)], body=True, flags8=0x4000)}, g(2), 1.0, 5.0, [g(7)]),
        ('outside', {'2': dict(xy=[f32(3), f32(4.0001)], body=True, flags8=0x4000)}, g(2), 1.0, 5.0, []),
        ('fraction', {'2': dict(xy=[f32(0.1), f32(0.2)], body=True, flags8=0x4000)}, g(2), 1.0, 0.2236068, []),
        ('zero364', {'2': dict(xy=[f32(1), f32(1)], body=True, flags8=0x4000)}, g(2), 0.0, 5.0, []),
        ('zero368', {'2': dict(xy=[f32(1), f32(1)], body=True, flags8=0x4000)}, g(2), 1.0, 0.0, []),
        ('stale_source', {'2': dict(xy=[f32(1), f32(1)], body=True, flags8=0x4000, registered=False)}, g(2), 1.0, 5.0, []),
        ('zero_source', {}, 0, 1.0, 5.0, []),
        ('already_listed', {'2': dict(xy=[f32(1), f32(1)], body=True, flags8=0x4000)}, g(2), 1.0, 5.0, [g(5), g(2)]),
        ('not_targetable', {'2': dict(xy=[f32(1), f32(1)], body=True, flags8=0)}, g(2), 1.0, 5.0, []),
        ('flag_0x800', {'2': dict(xy=[f32(1), f32(1)], body=True, flags8=0x4800)}, g(2), 1.0, 5.0, []),
        ('no_body_no_org', {'2': dict(xy=[f32(1), f32(1)], flags8=0x4000)}, g(2), 1.0, 5.0, []),
        ('relation_false', {'2': dict(xy=[f32(1), f32(1)], body=True, flags8=0x4000, relation=False)}, g(2), 1.0, 5.0, []),
        ('not_hostile', {'2': dict(xy=[f32(1), f32(1)], body=True, flags8=0x4000, hostile=False)}, g(2), 1.0, 5.0, []),
        ('member_redirect', {'2': dict(xy=[f32(1), f32(1)], body=True, flags8=0x4000,
                                       membership=dict(organization=3, attached=True)),
                             '3': dict(xy=[f32(50), f32(50)], flags8=0x4000, members=[2])}, g(2), 1.0, 5.0, []),
        ('member_detached', {'2': dict(xy=[f32(1), f32(1)], body=True, flags8=0x4000,
                                       membership=dict(organization=3, attached=False)),
                             '3': dict(xy=[f32(50), f32(50)], flags8=0x4000, members=[2])}, g(2), 1.0, 5.0, []),
        ('org_source_member_inside', {'3': dict(xy=[f32(50), f32(50)], flags8=0x4000, members=[0, 4, 2]),
                                      '4': dict(xy=[f32(40), f32(1)], body=True, membership=dict(organization=3, attached=True)),
                                      '2': dict(xy=[f32(1), f32(1)], body=True, membership=dict(organization=3, attached=True))},
         g(3), 1.0, 5.0, []),
        ('org_source_detached_inside', {'3': dict(xy=[f32(50), f32(50)], flags8=0x4000, members=[2]),
                                        '2': dict(xy=[f32(1), f32(1)], body=True, membership=dict(organization=3, attached=False))},
         g(3), 1.0, 5.0, []),
        ('org_source_none_inside', {'3': dict(xy=[f32(1), f32(1)], flags8=0x4000, members=[4]),
                                    '4': dict(xy=[f32(40), f32(1)], body=True, membership=dict(organization=3, attached=True))},
         g(3), 1.0, 5.0, [])]:
        rows.append(dict(name=f'denizen_{name}', actors={'1': owner, **actors}, source=source,
                         range364=f32(r364), range368=f32(r368), intruders=intruders))
    return rows


def main():
    out = []
    visited = {}
    for precision in PRECISIONS:
        for c in cases():
            m = Run(c, precision)
            try:
                result = m.run()
            except Exception as exc:
                raise RuntimeError(f'{c["name"]} {precision}') from exc
            visited.update(m.visited)
            out.append({'input': {**c, 'precision': precision}, 'output': result})
    report('native-denizen-fixtures.json', __doc__, out, visited)


if __name__ == '__main__':
    main()
