"""Original combat reaction routines under Unicorn.

Executes unchanged:
- 501770 Modifier OnAttacked defender abilities with 4C8300 context init and
  4C9EB0 try-use (one shared context across the ability loop),
- 4D3FA0 rout handler: rout message, then for every engaged-by GTID the
  attacker's breaker morale and the real 5718E0/571820 rout experience,
  stored to binary32 and awarded with 4CE770(amount, 0, 1),
- 4D6190 attacked-by bookkeeping with the real 4D1B40 node constructor and
  4D3A70 attacker engagement list/message.

Seams: 4C8F20 CanUse and 4C9550 UseAbility (proven in phase7-ability; their
context argument is recorded and may be mutated), 579010 breaker (proven in
native-morale-fixtures.json), 4D01E0 actor value, 4CE770 award, 42D490 event
message, actor virtuals +24 (name CString), +108 (allegiance), +A8, +100 and
Mind +6C, allocator 6F7810. CString reference counts are fixture blocks that
never reach zero, so no release virtual runs.
"""
import copy
import struct
from harness import (Machine, PRECISIONS, f32, f64, install_settings, install_world, report,
                     REGISTRY_ROOT)
from unicorn.x86_const import UC_X86_REG_ECX

MANAGER = 0xb39d8c


class Run(Machine):
    def __init__(self, c, precision):
        super().__init__(precision)
        self.u.mem_map(0, 0x1000)
        self.c = copy.deepcopy(c)
        self.registry = self.alloc(0x60010)
        self.w(REGISTRY_ROOT, self.registry)
        install_settings(self, {0x160: f32(.25), 0x164: f32(6), 0x170: f32(2.5), 0x178: f32(1),
                                0x1a0: c.get('bombard_timeout', f32(3))})
        self.world = install_world(self, 0, c.get('now', f32(10)))
        self.w(MANAGER, 0xabcd0000)
        self.tags = {}
        self.actors = {}
        self.hooks.update({0x6f7810: self.allocate, 0x42d490: self.message})

    def allocate(self, m):
        self.ret(0, self.alloc(max(self.arg(), 16)))

    def message(self, m):
        assert self.arg() == 0xabcd0000
        sp_args = [self.arg(i) for i in range(1, 5)]
        self.events.append({'Message': [sp_args[0], self.tag(sp_args[1]), self.tag(sp_args[2])]})
        self.ret(0)

    def tag(self, p):
        return self.tags.get(p, 0 if p == 0 else -1)

    def register(self, gtid, pointer):
        slot = gtid & 0xffff
        self.u.mem_write(self.registry + 4 + slot * 2, struct.pack('<H', gtid >> 16))
        self.w(self.registry + 0x20004 + slot * 4, pointer)

    def cstring(self):
        header = self.alloc(0x40)
        self.w(header, self.vtable({4: lambda m: (_ for _ in ()).throw(RuntimeError('release'))}))
        self.w(header + 0xc, 1000)
        return header + 0x10

    def make_actor(self, tag, view):
        a = self.alloc(0x200)
        self.w(a + 4, self.alloc(0x600))
        self.w(a + 0x14, 0x10000 * tag + tag)
        self.tags[a] = tag
        self.actors[tag] = a
        return a

    def finish_actor(self, tag, view):
        a = self.actors[tag]
        name = self.cstring()

        def name_slot(m, a=a, name=name):
            out = self.arg()
            self.w(out, name)
            self.events.append({'Name': self.tag(a)})
            self.ret(4, out)

        def allegiance_slot(m, view=view):
            owner = view.get('allegiance', 0)
            self.events.append({'Allegiance': self.tag(self.reg(UC_X86_REG_ECX))})
            self.ret(0, self.actors[owner] if owner else 0)

        def a8_slot(m, view=view):
            self.events.append({'VirtualA8': self.tag(self.reg(UC_X86_REG_ECX))})
            self.ret(0, int(view.get('a8', False)))

        def outer_slot(m, view=view):
            outer = view.get('outer', 0)
            self.events.append({'Outer': self.tag(self.reg(UC_X86_REG_ECX))})
            self.ret(0, self.actors[outer] if outer else 0)
        self.w(a, self.vtable({0x24: name_slot, 0x108: allegiance_slot, 0xa8: a8_slot, 0x100: outer_slot}))
        self.w(a + 8, view.get('flags8', 0))
        membership = view.get('membership')
        if membership is not None:
            comp = self.alloc(0x40)
            self.w(a + 0x80, comp)
            self.w(comp + 0x10, self.actors[membership['organization']] if membership['organization'] else 0)
            self.wb(comp + 0x1c, int(membership['attached']))
        if view.get('organization'):
            self.w(a + 0x7c, self.alloc(0x100))
        if view.get('mind') is not None:
            mind = self.alloc(0x40)

            def mind_6c(m, view=view, a=a):
                self.events.append({'Mind6C': self.tag(a)})
                self.ret(0, int(view['mind']))
            self.w(mind, self.vtable({0x6c: mind_6c}))
            self.w(a + 0x70, mind)
        return a

    def actors_from(self, views):
        for tag in views:
            self.make_actor(int(tag), views[tag])
        for tag, view in views.items():
            self.finish_actor(int(tag), view)

    # operations ------------------------------------------------------------
    def defender_abilities(self):
        c = self.c
        modifier = self.alloc(0x100)
        abilities = [self.alloc(0x40) for _ in c['abilities']]
        for i, a in enumerate(abilities):
            self.tags[a] = 100 + i
        array = self.alloc(4 * max(1, len(abilities)))
        for i, a in enumerate(abilities):
            self.w(array + 4 * i, a)
        self.w(modifier + 0xb0, array)
        self.w(modifier + 0xb4, len(abilities))
        victim = self.alloc(0x200)
        attacker = self.alloc(0x200)
        self.tags[victim] = 1
        self.tags[attacker] = 2
        self.w(victim + 0x78, 0x7878)
        script = list(c['abilities'])

        def ctx_words(p):
            words = [self.r(p + 4 * i) for i in range(12)]
            for i in (2, 7):  # source and target actor handles -> fixture tags
                words[i] = self.tag(words[i])
            return words

        def can_use(m):
            ability = self.tag(self.reg(UC_X86_REG_ECX))
            ctx = self.arg()
            assert self.arg(1) == 1
            entry = script[ability - 100]
            self.events.append({'CanUse': [ability, ctx_words(ctx)]})
            for offset, value in entry.get('mutate', {}).items():
                self.w(ctx + int(offset), value)
            self.ret(8, entry['can_use'])

        def use(m):
            ability = self.tag(self.reg(UC_X86_REG_ECX))
            ctx = self.arg()
            self.events.append({'Use': [ability, ctx_words(ctx), self.arg(1)]})
            self.ret(8, 1)
        self.hooks[0x4c8f20] = can_use
        self.hooks[0x4c9550] = use
        self.call(0x501770, modifier, (victim, attacker))
        return {'events': self.events}

    def rout(self):
        c = self.c
        self.actors_from(c['actors'])
        routed = self.actors[1]
        nodes = 0
        for gtid in reversed(c['engaged_by']):
            node = self.alloc(8)
            self.w(node, gtid)
            self.w(node + 4, nodes)
            nodes = node
        self.w(routed + 0x118, nodes)
        for tag, view in c['actors'].items():
            if view.get('registered'):
                self.register(0x10000 * int(tag) + int(tag), self.actors[int(tag)])
        values = {k: list(v) for k, v in c['values'].items()}

        def value(m):
            tag = self.tag(self.reg(UC_X86_REG_ECX))
            v = values[str(tag)].pop(0)
            self.events.append({'ActorValue': tag})
            self.ret_ext(4, v)

        def breaker(m):
            self.events.append({'Breaker': [self.tag(self.owner_of(self.reg(UC_X86_REG_ECX))),
                                            self.tag(self.arg())]})
            self.ret(4)

        def award(m):
            self.events.append({'Award': [self.tag(self.reg(UC_X86_REG_ECX)), self.arg(),
                                          self.arg(1) & 0xff, self.arg(2) & 0xff]})
            self.ret(12)
        self.hooks.update({0x4d01e0: value, 0x579010: breaker, 0x4ce770: award})
        self.call(0x4d3fa0, routed, ())
        return {'events': self.events}

    def owner_of(self, component):
        for tag, a in self.actors.items():
            if self.r(a + 0x7c) == component:
                return a
        return 0

    def attacked_by(self):
        c = self.c
        self.actors_from(c['actors'])
        victim = self.actors[1]
        source = self.actors[c['source']]
        head = 0
        tail = 0
        for gtid, time, timeout in reversed(c['list']):
            node = self.alloc(0x10)
            self.w(node, gtid)
            self.w(node + 4, time)
            self.w(node + 8, timeout)
            self.w(node + 0xc, head)
            head = node
            tail = tail or node
        self.w(victim + 0x130, head)
        self.w(victim + 0x134, tail)
        weapon = 0
        if c['weapon'] is not None:
            weapon = self.alloc(0x40)
            if c['weapon']:
                definition = self.alloc(0x40)
                self.w(definition + 0x20, c['weapon'])
                self.w(weapon + 4, definition)
        attackers = {}
        for tag, a in self.actors.items():
            attackers[tag] = [self.r(a + 0x128), self.r(a + 0x12c)]
        self.call(0x4d6190, victim, (source, weapon))

        def walk(head, size, link):
            out = []
            node = self.r(head)
            while node:
                out.append([self.r(node + 4 * i) for i in range(size)])
                node = self.r(node + link)
            return out
        lists = {str(tag): walk(a + 0x128, 1, 4) for tag, a in self.actors.items()}
        attacked = {str(tag): walk(a + 0x130, 3, 0xc) for tag, a in self.actors.items()}
        tails = {str(tag): self.r(a + 0x134) != 0 for tag, a in self.actors.items()}
        return {'attacked_lists': attacked, 'attacked_tail_present': tails, 'engaged_lists': lists,
                'events': self.events}

    def run(self):
        return getattr(self, self.c['operation'])()


def cases():
    rows = []
    for name, abilities in [
        ('none', []), ('one_ok', [dict(can_use=0)]), ('one_rejected', [dict(can_use=5)]),
        ('three', [dict(can_use=0), dict(can_use=1), dict(can_use=0)]),
        ('shared_context', [dict(can_use=0, mutate={'4': 3, '12': 0x99}), dict(can_use=0),
                            dict(can_use=2, mutate={'28': 0})])]:
        rows.append(dict(name=f'defender_{name}', operation='defender_abilities', abilities=abilities,
                         now=f32(17.25)))
    # Rout: actor 1 routed; 2,3,4 attackers.
    base_actors = {'1': dict(allegiance=5), '2': dict(registered=True, organization=True),
                   '3': dict(registered=True), '4': dict(registered=False, organization=True),
                   '5': dict()}
    for name, engaged, values in [
        ('none', [], {}), ('one_org', [0x20002], {'1': [f64(2)], '2': [f64(4)]}),
        ('one_plain', [0x30003], {'1': [f64(3)], '3': [f64(1)]}),
        ('stale_and_two', [0x40004, 0x20002, 0x30003],
         {'1': [f64(2), f64(9)], '2': [f64(4)], '3': [f64(1.0 / 3.0)]}),
        ('zero_value', [0x20002], {'1': [f64(0)], '2': [f64(4)]}),
        ('repeat', [0x20002, 0x20002], {'1': [f64(2), f64(2)], '2': [f64(5), f64(7)]})]:
        rows.append(dict(name=f'rout_{name}', operation='rout', actors=base_actors, engaged_by=engaged,
                         values=values))
    rows.append(dict(name='rout_no_allegiance', operation='rout',
                     actors={**base_actors, '1': dict()}, engaged_by=[0x30003],
                     values={'1': [f64(1)], '3': [f64(1)]}))
    # Attacked-by: victim 1, attacker/source 2, organization actor 3, owner 4.
    def actors(**k):
        v = {'1': dict(mind=k.get('victim_mind'), a8=k.get('victim_a8', False),
                       membership=k.get('victim_membership'), outer=k.get('victim_outer', 0)),
             '2': dict(membership=k.get('source_membership'), outer=k.get('source_outer', 0),
                       flags8=k.get('flags8', 0)),
             '3': dict(a8=k.get('org_a8', False)), '4': dict(flags8=k.get('flags8', 0)),
             '5': dict()}
        return v
    for name, kw, lst, weapon, timeout in [
        ('new_plain', {}, [], None, 3), ('new_weapon_mind', dict(victim_mind=True), [], f32(1.5), 3),
        ('mind_declines_weapon', dict(victim_mind=False), [], f32(1.5), 3),
        ('weapon_without_definition', dict(victim_mind=True), [], 0, 3),
        ('refresh_existing', {}, [[0x20002, f32(1), f32(3)], [0x50005, f32(2), f32(3)]], None, 3),
        ('refresh_second', {}, [[0x50005, f32(2), f32(3)], [0x20002, f32(1), f32(3)]], None, 3),
        ('prepend_to_list', {}, [[0x50005, f32(2), f32(3)]], None, 2.5),
        ('victim_a8', dict(victim_a8=True), [], None, 3),
        ('victim_member_redirect', dict(victim_membership={'organization': 3, 'attached': True}), [], None, 3),
        ('victim_member_redirect_a8', dict(victim_membership={'organization': 3, 'attached': True},
                                           org_a8=True), [], None, 3),
        ('source_attached_org', dict(source_membership={'organization': 3, 'attached': True}), [], None, 3),
        ('source_detached_outer', dict(source_membership={'organization': 3, 'attached': False},
                                       source_outer=4), [], None, 3),
        ('source_outer', dict(source_outer=4), [], None, 3),
        ('attacker_detached_no_message', dict(source_membership={'organization': 3, 'attached': False}),
         [], None, 3),
        ('attacker_flag29', dict(flags8=0x20000000), [], None, 3)]:
        rows.append(dict(name=f'attacked_by_{name}', operation='attacked_by', actors=actors(**kw), source=2,
                         list=lst, weapon=weapon, bombard_timeout=f32(timeout), now=f32(12.5)))
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
    report('native-combat-reaction-fixtures.json', __doc__, out, visited)


if __name__ == '__main__':
    main()
