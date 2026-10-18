"""Original KKC_GActor::ChangeKingdom 4D8450 (actor virtual +160) under Unicorn.

Executes the unchanged 4D8450: ownable check, early return on the same
kingdom, visibility mask over the world kingdom table (virtual +70 with
argument 1, minus the new kingdom's own index unless its +190 is set), the
ordered component/economy hand-over, the recruit-single move between
kingdoms (with the duplicate-single destroy path), recursion into the outer,
organization and Mind-held actors, re-reveal (virtual +8C, 4ECB90) and the
+15C wall re-reveal (585FF0).

Every callee is a recorded seam (no native body runs): 404F30/404F60/404ED0
logs, 4BEDF0/401B00 names, 7A0A40 SAI, 4D5DE0/4CFD30 change snapshot,
589CC0/589050 component94, 565400/565700 construction, 4D19F0 +160 object,
4DFF30 +164 object, 4E2A60/4E2CB0/4E4C30 kingdom recruit singles (+1FC),
4E44E0/4E6180 kingdom remove/add actor, 574590 component D8, 5714D0
membership, 6F7430 free, 4F6760 removal request, 50C2B0 manager, 584F10 wall,
579F10 OrganizationComponent, 56ADA0 Denizen, 5643A0 +D4 object, 4D1350,
4CFEA0, 4ECB90 reveal, 585FF0 wall reveal and the other actors' +160.
"""
import copy
import struct
from harness import Machine, PRECISIONS, install_settings, install_world, report, REGISTRY_ROOT
from unicorn.x86_const import UC_X86_REG_ECX


class Run(Machine):
    def __init__(self, c, precision):
        super().__init__(precision)
        self.u.mem_map(0, 0x1000)
        self.c = copy.deepcopy(c)
        c = self.c
        self.registry = self.alloc(0x60010)
        self.w(REGISTRY_ROOT, self.registry)
        install_settings(self, {})
        self.world = install_world(self)
        self.tags = {}
        self.kingdoms = {}
        table = self.alloc(4 * 8)
        self.w(self.world + 0x154, table)
        self.w(self.world + 0x158, len(c['kingdoms']))
        self.w(self.world + 0x130, 0x1300)
        for i, k in enumerate(c['kingdoms']):
            if k is None:
                continue
            kp = self.alloc(0x400)
            self.tags[kp] = 'k%d' % i
            self.tags[kp + 0x1fc] = 'singles:k%d' % i
            self.kingdoms[i] = kp
            self.w(table + 4 * i, kp)
            self.w(kp + 0x18c, k['index'])
            self.w(kp + 0x190, int(k['own_visible']))
            self.w(kp, self.vtable({0x24: self.name_slot('k%d' % i)}))
        self.w(0xb36528, self.alloc(0x40) if c['sai'] is not None else 0)
        if c['sai'] is not None:
            self.wb(self.r(0xb36528) + 0x20, int(c['sai']))
        self.actor = self.make_actor('actor', c['actor'])
        self.others = {}
        for name in ('outer', 'org', 'mindobj', 'wallobj'):
            o = self.alloc(0x200)
            self.tags[o] = name
            self.others[name] = o
            self.w(o, self.vtable({0x160: self.recursive(name)}))
        a, v = self.actor, c['actor']
        self.w(a + 8, (0x800000 if v['ownable'] else 0))
        self.w(a + 0x14, 0x10001)
        self.definition = self.alloc(0x100)
        self.w(a + 4, self.definition)
        self.w(a + 0xe8, self.kingdoms.get(v['kingdom'], 0) if v['kingdom'] is not None else 0)
        self.w(a + 0x90, 0x90 if v['wall_link'] else 0)
        self.w(a + 0x140, 0x1400)
        self.w(a + 0x100, v['flags100'])
        for key, off in [('comp94', 0x94), ('comp9c', 0x9c), ('obj160', 0x160), ('compd8', 0xd8),
                         ('wall98', 0x98), ('orgcomp7c', 0x7c), ('c88', 0x88), ('objd4', 0xd4)]:
            if v.get(key):
                p = self.alloc(0x200)
                self.tags[p] = key
                self.w(a + off, p)
                if key == 'wall98':
                    pass
        if v.get('obj164') is not None:
            gtid = 0x50005
            self.w(a + 0x164, gtid)
            if v['obj164']:
                obj = self.alloc(0x40)
                self.tags[obj] = 'obj164'
                self.register(gtid, obj)
        if v.get('membership') is not None:
            ms = self.alloc(0x40)
            self.tags[ms] = 'membership'
            self.w(a + 0x80, ms)
            self.w(ms + 0x10, self.others['org'] if v['membership'] else 0)
        if v.get('mind') is not None:
            mind = self.alloc(0x40)
            self.w(a + 0x70, mind)
            gtid = 0x60006
            self.w(mind + 0x28, gtid)
            if v['mind']:
                self.register(gtid, self.others['mindobj'])
        t15c = self.alloc(0x200)
        self.tags[t15c] = 't15c'
        wall15c = self.alloc(0x40)
        self.tags[wall15c] = 'wall:t15c'
        self.w(t15c + 0x98, wall15c)
        self.t15c = t15c
        self.single = self.alloc(0x40)
        self.tags[self.single] = 'single'
        self.hooks.update({
            0x404f30: self.ev0('NotOwnable'), 0x404f60: self.ev0('ChangeLog'), 0x404ed0: self.ev0('DuplicateSingle'),
            0x4bedf0: self.kingdom_name, 0x401b00: self.empty_name,
            0x7a0a40: self.seam('SaiChange', 0x10, lambda: [self.tag(self.arg()), self.arg(1), self.tag(self.arg(2)),
                                                           self.tag(self.arg(3))]),
            0x4d5de0: self.snapshot_begin, 0x4cfd30: self.seam('SnapshotEnd', 4, lambda: None),
            0x589cc0: self.seam('Research94Detach', 0, lambda: self.tag(self.reg(UC_X86_REG_ECX))),
            0x565400: self.seam('ConstructionCancel', 0, lambda: self.tag(self.reg(UC_X86_REG_ECX))),
            0x565700: self.seam('UpgradeCancel', 0, lambda: self.tag(self.reg(UC_X86_REG_ECX))),
            0x4d19f0: self.seam('Object160', 8, lambda: [self.tag(self.reg(UC_X86_REG_ECX)), self.arg(), self.arg(1)]),
            0x4dff30: self.seam('Object164', 4, lambda: [self.tag(self.reg(UC_X86_REG_ECX)), self.tag(self.arg())]),
            0x4e2a60: self.find_single, 0x4e2cb0: self.seam('RemoveSingle', 8, lambda: [
                self.tag(self.reg(UC_X86_REG_ECX)), self.tag(self.arg()), self.arg(1)]),
            0x4e44e0: self.seam('KingdomRemoveActor', 4, lambda: [self.tag(self.reg(UC_X86_REG_ECX)),
                                                                  self.tag(self.arg())]),
            0x574590: self.seam('ComponentD8', 4, lambda: self.tag(self.arg())),
            0x5714d0: self.seam('MembershipRelease', 4, lambda: [self.tag(self.reg(UC_X86_REG_ECX)), self.arg()]),
            0x6f7430: self.seam('Free', 0, lambda: self.tag(self.arg())),
            0x4f6760: self.seam('RequestRemoval', 8, lambda: [self.tag(self.arg()), self.arg(1)]),
            0x4e4c30: self.seam('AddSingle', 4, lambda: [self.tag(self.reg(UC_X86_REG_ECX)), self.tag(self.arg())]),
            0x4e6180: self.seam('KingdomAddActor', 4, lambda: [self.tag(self.reg(UC_X86_REG_ECX)),
                                                               self.tag(self.arg())]),
            0x50c2b0: self.seam('Manager', 8, lambda: [self.reg(UC_X86_REG_ECX), self.tag(self.arg()),
                                                       self.arg(1) & 0xff]),
            0x584f10: self.seam('WallKingdom', 4, lambda: self.tag(self.arg())),
            0x579f10: self.seam('OrganizationKingdom', 4, lambda: self.tag(self.arg())),
            0x56ada0: self.seam('DenizenKingdom', 4, lambda: self.tag(self.arg())),
            0x5643a0: self.seam('ObjectD4Kingdom', 4, lambda: self.tag(self.arg())),
            0x589050: self.seam('Research94Kingdom', 4, lambda: self.tag(self.arg())),
            0x4d1350: self.seam('Notify', 0, lambda: self.tag(self.reg(UC_X86_REG_ECX))),
            0x4cfea0: self.seam('Update0EA0', 0, lambda: None),
            0x4ecb90: self.seam('Reveal', 8, lambda: [self.tag(self.arg()), self.arg(1)]),
            0x585ff0: self.seam('WallReveal', 4, lambda: [self.tag(self.reg(UC_X86_REG_ECX)), self.tag(self.arg())]),
        })

    def register(self, gtid, pointer):
        slot = gtid & 0xffff
        self.u.mem_write(self.registry + 4 + slot * 2, struct.pack('<H', gtid >> 16))
        self.w(self.registry + 0x20004 + slot * 4, pointer)

    def make_actor(self, name, v):
        a = self.alloc(0x200)
        self.tags[a] = name
        visible = v['visible']

        def visible_slot(m):
            k = self.tag(self.arg())
            self.events.append({'Visible': [k, self.arg(1)]})
            self.ret(8, int(k in visible))

        def outer_slot(m):
            self.events.append('Outer')
            self.ret(0, self.others['outer'] if v['outer'] else 0)

        def slot8c(m):
            self.events.append('Virtual8C')
            self.ret(0, int(v['reveal']))

        def slot15c(m):
            self.events.append('Virtual15C')
            self.ret(0, self.t15c if v['t15c'] else 0)
        self.w(a, self.vtable({0x70: visible_slot, 0x100: outer_slot, 0x8c: slot8c, 0x15c: slot15c}))
        return a

    def tag(self, p):
        return self.tags.get(p, 0 if p == 0 else -1)

    def ev0(self, name):
        def handler(m):
            self.events.append(name)
            self.ret(0)
        return handler

    def seam(self, name, pop, args):
        def handler(m):
            self.events.append({name: args()})
            self.ret(pop)
        return handler

    def recursive(self, name):
        def handler(m):
            self.events.append({'ChangeKingdom': [name, self.tag(self.arg())]})
            self.ret(4)
        return handler

    def name_slot(self, label):
        def handler(m):
            out = self.arg()
            self.w(out, self.fake_cstring())
            self.ret(4, out)
        return handler

    def fake_cstring(self):
        header = self.alloc(0x20)
        self.w(header + 0xc, 1000)
        return header + 0x10

    def kingdom_name(self, m):
        out = self.arg()
        self.w(out, self.fake_cstring())
        self.ret(4, out)

    def empty_name(self, m):
        self.w(self.reg(UC_X86_REG_ECX), self.fake_cstring())
        self.ret(4, self.reg(UC_X86_REG_ECX))

    def snapshot_begin(self, m):
        local = self.arg()
        self.events.append({'SnapshotBegin': [self.r(local), self.r(local + 4), self.r(local + 8)]})
        if self.c['snapshot_list']:
            node = self.alloc(0x10)
            self.tags[node] = 'snapshot'
            self.w(local, node)
        self.ret(4)

    def find_single(self, m):
        lst = self.tag(self.reg(UC_X86_REG_ECX))
        assert self.arg() == self.definition and self.arg(1) == 0
        found = self.c['singles'].get(lst, False)
        self.events.append({'FindSingle': [lst, found]})
        self.ret(8, self.single if found else 0)

    def run(self):
        new = self.c['new']
        self.call(0x4d8450, self.actor, (self.kingdoms.get(new, 0) if new is not None else 0,))
        return {'kingdom': self.tag(self.r(self.actor + 0xe8)), 'events': self.events}


def actor(**k):
    v = dict(ownable=True, kingdom=0, wall_link=False, flags100=0, visible=[], outer=False, reveal=False,
             t15c=False, membership=None, mind=None, obj164=None)
    v.update(k)
    return v


def base(name, **k):
    c = dict(name=name, kingdoms=[dict(index=0, own_visible=False), dict(index=1, own_visible=False),
                                  dict(index=2, own_visible=True), None],
             sai=None, actor=actor(), new=1, singles={}, snapshot_list=False)
    c.update(k)
    return c


def cases():
    rows = []
    rows.append(base('plain'))
    rows.append(base('same_kingdom', new=0))
    rows.append(base('not_ownable', actor=actor(ownable=False)))
    rows.append(base('not_ownable_same_null', actor=actor(ownable=False, kingdom=None)))
    rows.append(base('from_none', actor=actor(kingdom=None)))
    rows.append(base('to_none', new=None))
    rows.append(base('wall_link_no_log', actor=actor(wall_link=True)))
    rows.append(base('sai_on', sai=True))
    rows.append(base('sai_off', sai=False))
    rows.append(base('visibility', actor=actor(visible=['k0', 'k1', 'k2'], reveal=True, t15c=True)))
    rows.append(base('visibility_new_own_visible', new=2, actor=actor(visible=['k0', 'k1', 'k2'], reveal=True,
                                                                       t15c=True)))
    rows.append(base('visibility_no_reveal', actor=actor(visible=['k0'], reveal=False, t15c=True)))
    rows.append(base('components', snapshot_list=True,
                     actor=actor(flags100=0x60000000, comp94=True, comp9c=True, obj160=True, compd8=True,
                                 obj164=True, objd4=True)))
    rows.append(base('obj164_stale', actor=actor(obj164=False)))
    rows.append(base('single_moves', singles={'singles:k0': True}))
    rows.append(base('single_duplicate_member', singles={'singles:k0': True, 'singles:k1': True},
                     actor=actor(membership=True), snapshot_list=True))
    rows.append(base('single_duplicate_no_org', singles={'singles:k0': True, 'singles:k1': True},
                     actor=actor(membership=False)))
    rows.append(base('single_to_none', new=None, singles={'singles:k0': True}))
    rows.append(base('outer_recursion', actor=actor(outer=True)))
    rows.append(base('wall_kingdom', actor=actor(wall98=True)))
    rows.append(base('organization_actor', actor=actor(membership=True)))
    rows.append(base('organization_component', actor=actor(membership=False, orgcomp7c=True)))
    rows.append(base('mind_object', actor=actor(mind=True, c88=True)))
    rows.append(base('mind_stale_denizen', actor=actor(mind=False, c88=True)))
    rows.append(base('denizen_no_mind', actor=actor(c88=True)))
    return rows


def main():
    out = []
    visited = {}
    for c in cases():
        m = Run(c, PRECISIONS[0])
        try:
            result = m.run()
        except Exception as exc:
            raise RuntimeError(c['name']) from exc
        visited.update(m.visited)
        out.append({'input': {**c, 'precision': PRECISIONS[0]}, 'output': result})
    report('native-change-kingdom-fixtures.json', __doc__ + '\nInteger-only: one precision.', out, visited)


if __name__ == '__main__':
    main()
