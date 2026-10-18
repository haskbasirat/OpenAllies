"""Original Body-damage response leaves under Unicorn.

Executes unchanged:
- 4D1120 capture eligibility (with real 4CEDA0 ancestor and 4579D0 lookup),
- 4F1A90 world death-queue push and 584E70 wall member-death dispatch,
- 4F65B0 world phase change with an EMPTY removal list, i.e. its death drain
  loop (pop head, free node, 561C20 CheckDeath on actor+60) and phase writes,
- 584CD0 outer/wall component damage-record copy,
- 579490 Organization OnElementDamaged (with real 578E90, 578470, 4CE640,
  432ED0, 5790F0 and 529C30), and
- 578660 TickShortfallDamage with the real 562D10 shortfall damage builder.

Seams: allocator 6F7810 / free 6F7430, 561C20 CheckDeath, Mind virtuals
+64/+7C, owner virtual +16C (can engage) and +108 (allegiance), 524C10 CAI
engage AddTarget, 404F60 diagnostic log, 77DC50 profiler scope, 4F02B0 world
RNG getter, 85D840 RNG draw and 5625C0 DealDamage (its request is recorded;
DealDamage itself is proven by research/phase6-combat).
"""
import copy
import struct
from harness import Machine, PRECISIONS, f32, install_settings, install_world, report, REGISTRY_ROOT
from unicorn.x86_const import UC_X86_REG_ECX


class Run(Machine):
    def __init__(self, c, precision):
        super().__init__(precision)
        self.u.mem_map(0, 0x1000)
        self.c = copy.deepcopy(c)
        self.registry = self.alloc(0x60010)
        self.w(REGISTRY_ROOT, self.registry)
        install_settings(self, {})
        self.world = install_world(self, c.get('game_mode', 0), c.get('now', f32(10)))
        self.hooks.update({0x6f7810: self.allocate, 0x6f7430: self.free})
        self.tags = {}

    def register(self, gtid, pointer):
        slot = gtid & 0xffff
        self.u.mem_write(self.registry + 4 + slot * 2, struct.pack('<H', gtid >> 16))
        self.w(self.registry + 0x20004 + slot * 4, pointer)

    def actor(self, tag, gtid=None):
        a = self.alloc(0x200)
        d = self.alloc(0x600)
        self.w(a + 4, d)
        self.w(a + 0x14, gtid if gtid is not None else 0x10000 + tag)
        self.tags[a] = tag
        return a

    def allocate(self, m):
        self.ret(0, self.alloc(max(self.arg(), 16)))

    def free(self, m):
        self.ret(0)

    def tag(self, p):
        return self.tags.get(p, 0 if p == 0 else -1)

    def list_actors(self, head):
        out = []
        node = self.r(head)
        while node:
            out.append(self.tag(self.r(node)))
            node = self.r(node + 4)
        return out

    def push_node(self, world, actor):
        node = self.alloc(8)
        self.w(node, actor)
        self.w(node + 4, self.r(world + 0x100))
        self.w(world + 0x100, node)
        if self.r(world + 0x104) == 0:
            self.w(world + 0x104, node)

    # operations ------------------------------------------------------------
    def capture_eligible(self):
        chain = self.c['chain']
        actors = [self.actor(i + 1) for i in range(len(chain))]
        for i, (a, view) in enumerate(zip(actors, chain)):
            if view['component98']:
                comp = self.alloc(0x40)
                self.w(a + 0x98, comp)
                owner = view['component98_owner']
                self.w(comp + 0x14, actors[owner - 1] if owner else 0)
            self.w(a + 0x100, view['flags100'])
            if view['properties'] is not None:
                props = self.alloc(0x100)
                self.w(a + 0x78, props)
                self.w(props + 0x1c, view['properties'])
            if view['component94'] is not None:
                comp = self.alloc(0x40)
                self.w(a + 0x94, comp)
                gtid = 0x50000 + i
                self.w(comp + 0x34, gtid)
                if view['component94'] == 'live':
                    self.register(gtid, self.alloc(0x40))
                elif view['component94'] == 'stale':
                    self.register(gtid ^ 0x10000, self.alloc(0x40))
            self.w(a + 0xd4, 0x777 if view['d4'] else 0)
            self.wb(self.r(a + 4) + 0x290, view['captureable'])
        result = self.call(0x4d1120, actors[0], ()) & 0xff
        return {'result': result}

    def queue(self):
        for tag in self.c['queue']:
            self.push_node(self.world, self.actor(tag))
        self.w(self.world + 0x104, self.r(self.world + 0x104))

    def queue_death(self):
        self.queue()
        self.call(0x4f1a90, self.world, (self.actor(self.c['actor']),))
        return {'queue': self.list_actors(self.world + 0x100),
                'tail': self.tag(self.r(self.r(self.world + 0x104))) if self.r(self.world + 0x104) else 0}

    def wall_death(self):
        self.queue()
        comp = self.alloc(0x40)
        owner = self.actor(90)
        self.w(comp + 4, owner)
        self.wb(self.r(owner + 4) + 0x4a8, int(self.c['all_segments']))
        members = self.c['segments']
        array = self.alloc(4 * max(1, len(members)))
        self.w(comp + 0x24, array)
        self.w(comp + 0x28, len(members))
        for i, tag in enumerate(members):
            self.w(array + 4 * i, self.actor(tag) if tag else 0)
        self.call(0x584e70, comp, (self.actor(self.c['actor']),))
        return {'queue': self.list_actors(self.world + 0x100),
                'tail': self.tag(self.r(self.r(self.world + 0x104))) if self.r(self.world + 0x104) else 0}

    def drain(self):
        self.w(self.world + 0xd8, self.c['phase'])
        bodies = {}
        for tag in self.c['queue']:
            a = self.actor(tag)
            body = self.alloc(0x40)
            self.w(a + 0x60, body)
            self.w(body + 4, a)
            bodies[body] = tag
            self.push_node(self.world, a)
        pushes = list(self.c['pushes'])

        def check_death(m):
            body = self.reg(UC_X86_REG_ECX)
            tag = bodies[body]
            self.events.append({'CheckDeath': tag, 'phase': self.r(self.world + 0xd8)})
            if pushes and pushes[0][0] == tag:
                _, new = pushes.pop(0)
                a = self.actor(new)
                b = self.alloc(0x40)
                self.w(a + 0x60, b)
                self.w(b + 4, a)
                bodies[b] = new
                self.push_node(self.world, a)
            self.ret(0)
        self.hooks[0x561c20] = check_death
        self.call(0x4f65b0, self.world, (self.c['new_phase'],))
        assert not pushes
        return {'phase': self.r(self.world + 0xd8), 'queue': self.list_actors(self.world + 0x100),
                'tail_null': self.r(self.world + 0x104) == 0}

    def outer_record(self):
        comp = self.alloc(0x80)
        for i in range(7):
            self.w(comp + 0x4c + 4 * i, 0xdead0000 + i)
        record = self.alloc(0x20)
        for i, word in enumerate(self.c['record']):
            self.w(record + 4 * i, word)
        self.call(0x584cd0, comp, (0x31, record))
        return {'record': [self.r(comp + 0x4c + 4 * i) for i in range(7)]}

    def element_damaged(self):
        c = self.c
        org = self.alloc(0x100)
        owner = self.actor(1)
        element = self.actor(2)
        attacker = self.actor(3, 0x30003)
        self.w(org + 4, owner)
        self.wb(org + 0x8d, int(c['byte8d']))
        for i in range(7):
            self.w(org + 0x90 + 4 * i, 0xdead0000 + i)
        definition = self.r(owner + 4)
        self.w(definition + 0x304, c['radius_squared'])
        self.w(owner + 0x20, c['owner_xy'][0])
        self.w(owner + 0x24, c['owner_xy'][1])
        self.w(attacker + 0x20, c['attacker_xy'][0])
        self.w(attacker + 0x24, c['attacker_xy'][1])
        self.w(owner + 0x100, c['owner_flags100'])
        if c['element_membership'] is not None:
            membership = self.alloc(0x40)
            self.w(element + 0x80, membership)
            self.wb(membership + 0x1c, int(c['element_membership']))
        if c['attacker_registered']:
            self.register(0x30003, attacker)
        mind = self.alloc(0x40)

        def mind_slot(name):
            def handler(m):
                value = c['mind'][name]
                self.events.append({name: value})
                self.ret(0, int(value))
            return handler
        self.w(mind, self.vtable({0x64: mind_slot('busy64'), 0x7c: mind_slot('busy7c')}))
        self.w(owner + 0x70, mind if c['mind'] is not None else 0)

        def can_engage(m):
            self.events.append({'CanEngage': self.tag(self.arg())})
            self.ret(4, int(c['can_engage']))
        self.w(owner, self.vtable({0x16c: can_engage}))

        def add_target(m):
            assert self.reg(UC_X86_REG_ECX) == mind
            self.events.append({'AddEngageTarget': self.tag(self.arg())})
            self.ret(4, 1)
        self.hooks[0x524c10] = add_target

        def diagnostic(m):
            self.events.append('OutsideOperatingRadiusDiagnostic')
            self.ret(0)
        self.hooks[0x404f60] = diagnostic
        record = self.alloc(0x20)
        for i, word in enumerate(c['record']):
            self.w(record + 4 * i, word)
        result = self.call(0x579490, org, (element, record)) & 0xff
        return {'result': result, 'last_damage': [self.r(org + 0x90 + 4 * i) for i in range(7)],
                'events': self.events}

    def shortfall(self):
        c = self.c
        org = self.alloc(0x100)
        owner = self.actor(1)
        self.w(org + 4, owner)
        self.w(self.r(owner + 4) + 0x2f8, c['rate'])
        members = c['members']
        array = self.alloc(4 * max(1, len(members)))
        self.w(org + 0x28, array)
        self.w(org + 0x2c, len(members))
        draws = list(c['draws'])
        allegiance = self.alloc(0x40)
        self.w(allegiance + 0x14, 0x7000a)
        bodies = {}
        actors = []
        for i, m in enumerate(members):
            if m is None:
                self.w(array + 4 * i, 0)
                actors.append(None)
                continue
            a = self.actor(10 + i)
            actors.append(a)
            self.w(array + 4 * i, a)

            def allegiance_slot(mm, present=m['allegiance']):
                self.events.append('Allegiance')
                self.ret(0, allegiance if present else 0)
            self.w(a, self.vtable({0x108: allegiance_slot}))
            if m['body'] is not None:
                body = self.alloc(0x40)
                self.w(a + 0x60, body)
                self.w(body + 4, a if m['body_owner'] else 0)
                self.w(body + 0x14, m['body'])
                bodies[body] = 10 + i
        removals = list(c.get('remove_after', []))

        def deal(m):
            body = self.reg(UC_X86_REG_ECX)
            record = self.arg()
            words = [self.r(record + 4 * k) for k in range(7)]
            self.events.append({'DealDamage': {'body': bodies[body], 'record': words,
                                               'apply_type_modifier': self.arg(1) & 0xff,
                                               'weapon': self.arg(2), 'propagate': self.arg(3) & 0xff}})
            if removals and removals[0] == bodies[body]:
                removals.pop(0)
                self.w(org + 0x2c, self.r(org + 0x2c) - 1)
            self.ret(16)
        self.hooks[0x5625c0] = deal
        rng = self.alloc(0x40)

        def random_source(m):
            self.events.append('RandomSource')
            self.ret(0, rng)

        def draw(m):
            assert self.arg() == rng
            value = draws.pop(0)
            self.events.append({'Random': value})
            self.ret(0, value)
        self.hooks[0x4f02b0] = random_source
        self.hooks[0x85d840] = draw
        self.hooks[0x77dc50] = lambda m: m.ret(8)
        self.call(0x578660, org, (c['delta'],))
        assert not draws and not removals
        return {'events': self.events}

    def run(self):
        return getattr(self, self.c['operation'])()


def record(source=0x30003, kind=1, amount=12.5):
    return [source, 0x20007, 9, f32(amount), f32(9), kind, 4]


def cases():
    rows = []
    view = lambda **k: {**dict(component98=False, component98_owner=0, flags100=0, properties=None,
                               component94=None, d4=False, captureable=1), **k}
    for name, chain in [
        ('plain', [view()]), ('not_captureable', [view(captureable=0)]),
        ('flag21', [view(flags100=0x200000)]), ('flag_other', [view(flags100=0x100000)]),
        ('properties_400', [view(properties=0x400)]), ('properties_other', [view(properties=0x3ff)]),
        ('properties_zero', [view(properties=0)]), ('component94_live', [view(component94='live')]),
        ('component94_stale', [view(component94='stale')]), ('component94_empty', [view(component94='empty')]),
        ('d4', [view(d4=True)]), ('segment_owner', [view(component98=True, component98_owner=2), view()]),
        ('segment_owner_blocked', [view(component98=True, component98_owner=2), view(captureable=0)]),
        ('segment_no_owner', [view(component98=True, component98_owner=0)]),
        ('segment_chain', [view(component98=True, component98_owner=2),
                           view(component98=True, component98_owner=3, captureable=0), view(flags100=0)]),
        ('segment_self_flags_ignored', [view(component98=True, component98_owner=2, flags100=0x200000,
                                             d4=True), view()]),
        ('captureable_byte_2', [view(captureable=2)])]:
        rows.append(dict(name=f'capture_eligible_{name}', operation='capture_eligible', chain=chain))
    for queue in [[], [5], [5, 6, 7]]:
        rows.append(dict(name=f'queue_death_{queue}', operation='queue_death', queue=queue, actor=9))
        for all_segments, segments in [(False, [11, 12]), (True, [11, 0, 12]), (True, []), (True, [0])]:
            rows.append(dict(name=f'wall_death_{queue}_{all_segments}_{segments}', operation='wall_death',
                             queue=queue, all_segments=all_segments, segments=segments, actor=9))
    for phase, new, queue, pushes in [(1, 5, [5, 6, 7], []), (4, 5, [5], []), (2, 5, [5, 6], []),
                                      (1, 1, [5, 6], []), (3, 6, [], []), (1, 6, [], []),
                                      (1, 5, [5, 6], [(6, 8), (8, 9)]), (4, 2, [5], [(5, 7)])]:
        rows.append(dict(name=f'drain_{phase}_{new}_{queue}_{pushes}', operation='drain', phase=phase,
                         new_phase=new, queue=queue, pushes=pushes))
    rows.append(dict(name='outer_record', operation='outer_record', record=record()))
    base = dict(operation='element_damaged', byte8d=True, radius_squared=f32(100), owner_xy=[f32(0), f32(0)],
                attacker_xy=[f32(6), f32(8)], owner_flags100=0, element_membership=True,
                attacker_registered=True, mind=dict(busy64=False, busy7c=False), can_engage=True,
                record=record())
    variants = [('engage', {}), ('kind0', dict(record=record(kind=0))), ('kind3', dict(record=record(kind=3))),
                ('no_mind', dict(mind=None)), ('stale', dict(attacker_registered=False)),
                ('cannot_engage', dict(can_engage=False)), ('outside', dict(attacker_xy=[f32(6), f32(8.0001)])),
                ('outside_detached', dict(attacker_xy=[f32(60), f32(8)], element_membership=False)),
                ('outside_no_membership', dict(attacker_xy=[f32(60), f32(8)], element_membership=None)),
                ('inside_detached_blocked', dict(element_membership=False)),
                ('inside_detached_flag10', dict(element_membership=False, owner_flags100=0x10)),
                ('inside_detached_flag8000', dict(element_membership=None, owner_flags100=0x8000)),
                ('latch_off_mind_free', dict(byte8d=False)),
                ('latch_off_mind_busy64', dict(byte8d=False, mind=dict(busy64=True, busy7c=False))),
                ('latch_off_mind_busy7c', dict(byte8d=False, mind=dict(busy64=False, busy7c=True))),
                ('fractional', dict(radius_squared=f32(0.3), owner_xy=[f32(0.1), f32(0.2)],
                                    attacker_xy=[f32(0.5), f32(0.4)])),
                ('edge_rounding', dict(radius_squared=f32(1), owner_xy=[f32(0), f32(0)],
                                       attacker_xy=[f32(0.70710677), f32(0.70710677)])),
                ('source_zero', dict(record=record(source=0)))]
    for name, edit in variants:
        rows.append(dict(name=f'element_damaged_{name}', **{**base, **edit}))
    member = lambda body=100.0, owner=True, allegiance=True: dict(body=f32(body), body_owner=owner,
                                                                   allegiance=allegiance)
    for name, members, draws, delta, rate, extra in [
        ('one', [member()], [0x12345678], .1, .05, {}),
        ('several', [member(), None, member(250), dict(body=None, body_owner=True, allegiance=True), member(7.5)],
         [0, 0xffffffff, 0x80000000], .1, .05, {}),
        ('negative_draw', [member(), member()], [0xfffffffe, 0x7fffffff], 0.0333333, .2, {}),
        ('no_allegiance', [member(80, True, False), member(80, True, True)], [1, 2], .5, .01, {}),
        ('remove_during', [member(), member(), member()], [5, 6], .1, .05, {'remove_after': [10]}),
        ('zero_rate', [member()], [77], .1, 0, {}),
        ('empty', [], [], .1, .05, {})]:
        rows.append(dict(name=f'shortfall_{name}', operation='shortfall', members=members, draws=draws,
                         delta=f32(delta), rate=f32(rate), now=f32(42.5), **extra))
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
    report('native-damage-response-fixtures.json', __doc__, out, visited)


if __name__ == '__main__':
    main()
