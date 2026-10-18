"""Original OrganizationComponent member removal 57CD70 under Unicorn.

Executes unchanged 57CD70 with 45EC60 (member definition +2D8 property list),
578540 (owner-granted property detach), 45D580(0) (definition +2C0 morale
value), the MoraleDeathFactor loss (4CF110 called, result unused), 578140,
4D1080, 4D1050, the 4364C0 leader reselection (first attached, else first
detached member), 57C800 follow-leader (578E90, 4515F0) and 529C30.
Seams: 4E1270 slot record release, 4CEF70 property removal, 570670
membership detach, 57A640, 5797E0 slot refresh, 4D4CF0, 4CF0F0, 4D64F0,
4D1350, 51F9F0/51F920, Mind +5C/+64/+7C/+110/+114, 57C680, and the
organization-owner removal leaves 4F5490 (destroy), 4D7960 (BeginDeath) and
4F6760 (removal request, aa-world::World::request_removal).
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
        install_settings(self, {0xd4: c['death_factor']})
        self.world = install_world(self, c['mode'], f32(10))
        self.tags = {}
        self.org = self.alloc(0x100)
        self.owner = self.actor(1)
        self.w(self.org + 4, self.owner)
        o = c['owner']
        self.w(self.owner + 8, o['flags8'])
        self.w(self.owner + 0x104, o['flags104'])
        self.w(self.owner + 0x20, f32(1))
        self.w(self.owner + 0x24, f32(2))
        props = self.alloc(0x100)
        self.w(self.owner + 0x78, props)
        entries = o['properties']
        arr = self.alloc(4 * max(1, len(entries)))
        self.w(props + 0x10, arr)
        self.w(props + 0x14, len(entries))
        for i, (kind, prop, disabled) in enumerate(entries):
            e = self.alloc(0x20)
            self.w(arr + 4 * i, e)
            self.w(e + 8, kind)
            self.w(e + 0xc, prop)
            self.wb(e + 0x10, disabled)
        mind = self.alloc(0x40)
        m = c['mind']
        self.w(mind, self.vtable({0x5c: self.flag('Mind5C', m and m['declines']),
                                  0x64: self.flag('Mind64', m and m['busy64']),
                                  0x7c: self.flag('Mind7C', m and m['busy7c']),
                                  0x110: self.flag('PrimaryLost', 0), 0x114: self.flag('LeaderChanged', 0)}))
        self.w(self.owner + 0x70, mind if m is not None else 0)
        if m is not None:
            self.w(mind + 0x28, m['word28'])
        for k, off in [('morale', 0x5c), ('maximum', 0x60), ('breakpoint', 0xb8), ('d8', 0xd8)]:
            self.w(self.org + off, c['org'][k])
        self.w(self.org + 0x64, c['org']['living'])
        # definition
        d = self.alloc(0x100)
        self.w(self.org + 0x18, d)
        self.w(d + 0x18, c['def18'])
        slots = c['slots']
        slot_index = self.alloc(4 * len(slots))
        slot_defs = self.alloc(16 * len(slots))
        self.w(d + 0x28, slot_index)
        self.w(d + 0x1c, slot_defs)
        members = self.alloc(4 * len(slots))
        self.w(self.org + 0x28, members)
        self.w(self.org + 0x2c, len(slots))
        recs = self.alloc(4 * len(slots))
        self.w(self.org + 0x34, recs)
        arr50 = self.alloc(4 * len(slots))
        self.w(self.org + 0x50, arr50)
        self.members = {}
        for i, s in enumerate(slots):
            self.w(slot_index + 4 * i, i)
            self.wb(slot_defs + 16 * i + 0xe, int(s['living']))
            self.w(arr50 + 4 * i, 0x5050)
            if s.get('record') is not None:
                rec = self.alloc(0x20)
                self.w(recs + 4 * i, rec)
                gtid = 0x60000 + i
                self.w(rec + 0xc, gtid)
                if s['record'] == 'live':
                    slot = gtid & 0xffff
                    self.u.mem_write(self.registry + 4 + slot * 2, struct.pack('<H', gtid >> 16))
                    self.w(self.registry + 0x20004 + slot * 4, rec)
                self.tags[rec] = 'record:%d' % i
            if s.get('member') is None:
                continue
            mv = s['member']
            a = self.actor(10 + i)
            self.members[i] = a
            self.w(members + 4 * i, a)
            self.w(a + 8, mv.get('flags8', 0))
            self.w(a + 0x100, mv.get('flags100', 0))
            self.w(a + 0x104, mv.get('flags104', 0))
            self.w(a + 0x20, mv.get('xy', [f32(5), f32(6)])[0])
            self.w(a + 0x24, mv.get('xy', [f32(5), f32(6)])[1])
            md = self.r(a + 4)
            self.w(md + 0x2c0, mv.get('morale_value', f32(4)))
            head = 0
            for value in reversed(mv.get('def_properties', [])):
                node = self.alloc(8)
                self.w(node, value)
                self.w(node + 4, head)
                head = node
            self.w(md + 0x2d8, head)
            self.w(a + 0x78, self.alloc(0x40) if mv.get('properties', True) else 0)
            self.w(a + 0xa4, 0xa4 if mv.get('a4') else 0)
            ms = self.alloc(0x40)
            self.w(a + 0x80, ms)
            self.w(ms + 0x10, self.owner)
            self.w(ms + 0x18, i)
            self.wb(ms + 0x1c, int(mv.get('attached', True)))
            if mv.get('comp6c') is not None:
                comp = self.alloc(0x40)
                self.w(a + 0x6c, comp)
                self.w(comp + 0x28, mv['comp6c'])
        if c['leader'] is not None:
            self.w(self.org + 0x40, self.members[c['leader']])
        if c['primary'] is not None:
            self.w(self.org + 0x44, self.members[c['primary']])
        self.hooks.update({
            0x4e1270: self.seam('RecruitRelease', 0, lambda: [self.tag(self.reg(UC_X86_REG_ECX))]),
            0x4cef70: self.seam('RemoveProperty', 8, lambda: [self.tag(self.reg(UC_X86_REG_ECX)), self.arg(),
                                                               self.tag(self.arg(1))]),
            0x570670: self.seam('Detach', 8, lambda: [self.tag(self.arg()), self.arg(1)]),
            0x57a640: self.seam('Recompute6C', 0, lambda: None),
            0x5797e0: self.seam('SlotRefresh', 4, lambda: self.arg()),
            0x4d4cf0: self.seam('Update4CF0', 0, lambda: None),
            0x4cf0f0: self.seam('Update0F0', 0, lambda: None),
            0x4d1350: self.seam('Notify', 0, lambda: self.tag(self.reg(UC_X86_REG_ECX))),
            0x4d64f0: self.seam('SetState', 8, lambda: [self.arg(), bool(self.arg(1) & 0xff)]),
            0x51f9f0: self.seam('MindExhausted', 0, lambda: None),
            0x51f920: self.seam('MindWavering', 0, lambda: None),
            0x57c680: self.seam('MoveToLeader', 8, lambda: [self.arg(), self.arg(1)]),
            0x4f5490: self.seam('Destroy', 4, lambda: self.tag(self.arg())),
            0x4d7960: self.seam('BeginDeath', 0, lambda: self.tag(self.reg(UC_X86_REG_ECX))),
            0x4f6760: self.seam('RequestRemoval', 8, lambda: [self.tag(self.arg()), self.arg(1)]),
        })

    def actor(self, tag):
        a = self.alloc(0x200)
        self.w(a + 4, self.alloc(0x600))
        self.tags[a] = tag
        return a

    def tag(self, p):
        return self.tags.get(p, 0 if p == 0 else -1)

    def flag(self, name, value):
        def handler(m):
            self.events.append({name: bool(value)})
            self.ret(0, int(bool(value)))
        return handler

    def seam(self, name, pop, args):
        def handler(m):
            self.events.append({name: args()})
            self.ret(pop)
        return handler

    def run(self):
        member = self.members.get(self.c['remove']) if self.c['remove'] is not None else None
        self.call(0x57cd70, self.org, (self.c['slot'], member or 0))
        return {'events': self.events, 'morale': self.r(self.org + 0x5c), 'living': self.r(self.org + 0x64),
                'leader': self.tag(self.r(self.org + 0x40)), 'primary': self.tag(self.r(self.org + 0x44)),
                'members': [self.tag(self.r(self.r(self.org + 0x28) + 4 * i)) for i in range(len(self.c['slots']))],
                'arr50': [self.r(self.r(self.org + 0x50) + 4 * i) for i in range(len(self.c['slots']))]}


def base(name, **k):
    c = dict(name=name, mode=0, death_factor=f32(1.5), def18=0,
             owner=dict(flags8=0, flags104=0, properties=[]),
             mind=dict(declines=False, busy64=False, busy7c=False, word28=0),
             org=dict(morale=f32(50), maximum=f32(100), breakpoint=f32(20), d8=f32(7), living=3),
             slots=[dict(living=True, member=dict()), dict(living=True, member=dict()),
                    dict(living=False, member=dict())],
             leader=0, primary=1, remove=2, slot=2)
    c.update(k)
    return c


def cases():
    rows = []
    rows.append(base('plain_member'))
    rows.append(base('living_slot', remove=1, slot=1, primary=0))
    rows.append(base('null_member', remove=None))
    rows.append(base('owner_destructed', owner=dict(flags8=1, flags104=0, properties=[])))
    rows.append(base('mode6', mode=6))
    rows.append(base('flag104', owner=dict(flags8=0, flags104=4, properties=[])))
    rows.append(base('mind_busy', mind=dict(declines=False, busy64=False, busy7c=False, word28=1)))
    for morale, value, mind in [(20.0, 4.0, dict(declines=False, busy64=False, busy7c=False, word28=0)),
                                (5.0, 4.0, dict(declines=False, busy64=False, busy7c=False, word28=0)),
                                (25.0, 4.0, dict(declines=True, busy64=False, busy7c=False, word28=0)),
                                (25.0, 4.0, dict(declines=False, busy64=False, busy7c=False, word28=0)),
                                (7.3, 0.1, dict(declines=False, busy64=False, busy7c=False, word28=0))]:
        slots = [dict(living=True, member=dict()), dict(living=True, member=dict()),
                 dict(living=False, member=dict(morale_value=f32(value)))]
        rows.append(base(f'morale_{morale}_{value}_{mind["declines"]}', mind=mind, slots=slots,
                         org=dict(morale=f32(morale), maximum=f32(100), breakpoint=f32(20), d8=f32(7), living=3)))
    rows.append(base('no_mind_morale_zero', mind=None, slots=[dict(living=True, member=dict()), dict(living=True, member=dict()),
                     dict(living=False, member=dict(morale_value=f32(60)))]))
    for value in [7.0, 8.0]:
        rows.append(base(f'comp6c_{value}', slots=[dict(living=True, member=dict()), dict(living=True, member=dict()),
                         dict(living=False, member=dict(comp6c=f32(value)))]))
    for flags100, flags104, mind in [(0, 0, dict(declines=False, busy64=False, busy7c=False, word28=0)),
                                     (0x4000000, 0, dict(declines=False, busy64=False, busy7c=False, word28=0)),
                                     (0, 0x10, dict(declines=False, busy64=False, busy7c=False, word28=0)),
                                     (0x2000000, 0, dict(declines=False, busy64=False, busy7c=False, word28=0)),
                                     (0, 0, None)]:
        slots = [dict(living=True, member=dict()), dict(living=True, member=dict(flags100=flags100, flags104=flags104)),
                 dict(living=False, member=dict())]
        rows.append(base(f'primary_{flags100}_{flags104}_{mind is None}', slots=slots, mind=mind, remove=1, slot=1,
                         primary=1))
    for name, slots, mind in [
        ('attached_next', [dict(living=True, member=dict()), dict(living=True, member=dict(attached=False)),
                           dict(living=True, member=dict(xy=[f32(9), f32(9)]))],
         dict(declines=False, busy64=True, busy7c=False, word28=0)),
        ('detached_only', [dict(living=True, member=dict()), dict(living=True, member=dict(attached=False)),
                           dict(living=True)], dict(declines=False, busy64=False, busy7c=False, word28=0)),
        ('mind_free_no_follow', [dict(living=True, member=dict()), dict(living=True, member=dict()),
                                 dict(living=True)], dict(declines=False, busy64=False, busy7c=False, word28=0))]:
        rows.append(base(f'leader_{name}', slots=slots, mind=mind, leader=0, primary=None, remove=0, slot=0))
    for flags8 in [0, 2, 0x10, 0x12]:
        for flags100 in [0, 0x2000000]:
            slots = [dict(living=True, member=dict(flags8=flags8, flags100=flags100)), dict(living=True),
                     dict(living=False)]
            rows.append(base(f'last_member_{flags8}_{flags100}', slots=slots, leader=0, primary=None,
                             remove=0, slot=0))
    rows.append(base('last_member_owner_destructed', owner=dict(flags8=1, flags104=0, properties=[]),
                     slots=[dict(living=True, member=dict()), dict(living=True), dict(living=False)],
                     leader=0, primary=None, remove=0, slot=0))
    rows.append(base('def18_negative', def18=-1, primary=None, leader=0))
    rows.append(base('properties', owner=dict(flags8=0, flags104=0,
                                              properties=[[3, 0x111, 0], [7, 0x222, 0], [7, 0x333, 1], [5, 0x444, 0]]),
                     slots=[dict(living=True, member=dict()), dict(living=True, member=dict()),
                            dict(living=False, member=dict(def_properties=[0xa1, 0xa2]))]))
    rows.append(base('properties_member_a4', owner=dict(flags8=0, flags104=0,
                                                        properties=[[3, 0x111, 0], [7, 0x222, 0]]),
                     slots=[dict(living=True, member=dict()), dict(living=True, member=dict()),
                            dict(living=False, member=dict(a4=True))]))
    rows.append(base('properties_member_without', owner=dict(flags8=0, flags104=0, properties=[[3, 0x111, 0]]),
                     slots=[dict(living=True, member=dict()), dict(living=True, member=dict()),
                            dict(living=False, member=dict(properties=False))]))
    for rec in ['live', 'stale']:
        rows.append(base(f'record_{rec}', slots=[dict(living=True, member=dict()), dict(living=True, member=dict()),
                                                 dict(living=False, record=rec, member=dict())]))
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
    report('native-member-removal-fixtures.json', __doc__, out, visited)


if __name__ == '__main__':
    main()
