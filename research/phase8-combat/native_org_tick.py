"""Original OrganizationComponent TickMaintenance 57C8E0 under Unicorn.

Executes unchanged 57C8E0 together with 4D7DA0 (targeted state), 5788B0 and
57DC40 (current/maximum combat value), 579110 (regeneration gate), 4D7CE0
(out-of-supply state sync), 57C800 (follow leader, with real 578E90 and the
4515F0 XY equality), 58B150, 5147C0, 529C30 and 4CF110.

Seams (recorded with arguments): the proven sub-steps 57A0A0 morale tick,
579600 engagement experience, 5785D0 attack efficiency and 578660 shortfall
(see native-morale/-damage-response fixtures), 57B5A0 regeneration, 57AEB0
regroup check, 515F40 flare, 57C680 move-to-leader, 540690 marker, 45D520
entrench time (binary32 return), 4D0A80 in-supply predicate, 4D49B0 state
descriptor, 4D64F0 state transition, 4D1350 actor notification, 404F60
diagnostic, Mind virtuals +64/+7C/+150 and owner virtual +108 allegiance.
"""
import copy
import struct
from harness import (Machine, PRECISIONS, f32, install_settings, install_world, report,
                     REGISTRY_ROOT)
from unicorn.x86_const import UC_X86_REG_ECX


class Run(Machine):
    def __init__(self, c, precision):
        super().__init__(precision)
        self.u.mem_map(0, 0x1000)
        self.c = copy.deepcopy(c)
        c = self.c
        self.registry = self.alloc(0x60010)
        self.w(REGISTRY_ROOT, self.registry)
        install_settings(self, {0xbc: c['regroup_time']})
        install_world(self, c['mode'], c['now'])
        self.tags = {}
        self.org = self.alloc(0x100)
        self.owner = self.actor(1)
        self.w(self.org + 4, self.owner)
        d = self.r(self.owner + 4)
        self.wb(d + 0x2f4, int(c['def2f4']))
        o = c['owner']
        self.w(self.owner + 8, o['flags8'])
        self.w(self.owner + 0x100, o['flags100'])
        self.w(self.owner + 0x118, 0x1118 if o['engaged'] else 0)
        self.w(self.owner + 0x7c, self.org if o['organization'] else 0)
        self.w(self.owner + 0x20, o['xy'][0])
        self.w(self.owner + 0x24, o['xy'][1])
        if o['properties']:
            props = self.alloc(0x100)
            self.w(self.owner + 0x78, props)
            self.tags[props + 0x1c] = 'modifier'
        allegiance = self.alloc(0x200)
        self.tags[allegiance] = 'allegiance'
        self.w(allegiance + 0x1f0, c['shortage'])
        mind = self.alloc(0x40)
        self.tags[mind] = 'mind'
        m = c['mind']
        self.w(mind, self.vtable({0x64: self.flag('Mind64', m and m['busy64']),
                                  0x7c: self.flag('Mind7C', m and m['busy7c']),
                                  0x150: self.flag('Mind150', 0)}))
        self.w(self.owner + 0x70, mind if m is not None else 0)
        self.w(self.owner, self.vtable({0x108: self.allegiance(allegiance if c['allegiance'] else 0)}))
        # leader (+40) and primary (+44)
        if c['leader'] is not None:
            leader = self.actor(2)
            self.w(self.org + 0x40, leader)
            self.w(leader + 8, c['leader']['flags8'])
            self.w(leader + 0x20, c['leader']['xy'][0])
            self.w(leader + 0x24, c['leader']['xy'][1])
        if c['primary'] is not None:
            primary = self.actor(3)
            self.w(self.org + 0x44, primary)
            self.w(primary + 8, c['primary'])
        for k, off in [('b0', 0xb0), ('b8e', 0x8e)]:
            self.wb(self.org + off, int(c[k]))
        self.w(self.org + 0xac, 0x5151)
        self.w(self.org + 0x48, c['regroup_started'])
        self.w(self.org + 0xb4, c['entrench_base'])
        # owner +148/+14C objects (marker/flare target)
        objects = self.alloc(0x40)
        self.w(self.owner + 0x148, objects)
        self.w(self.owner + 0x14c, c['objects_count'])
        if c['object10'] is not None:
            obj = self.alloc(0x20)
            self.tags[obj] = 'object10'
            self.w(objects + 0x10, obj)
            self.w(obj + 8, c['object10'])
        # members
        members = c['members']
        array = self.alloc(4 * max(1, len(members)))
        self.w(self.org + 0x28, array)
        self.w(self.org + 0x2c, len(members))
        definition = self.alloc(0x100)
        self.w(self.org + 0x18, definition)
        slot_index = self.alloc(4 * max(1, len(members)))
        self.w(definition + 0x28, slot_index)
        slot_defs = self.alloc(4 * max(1, len(members)))
        self.w(self.org + 0x1c, slot_defs)
        for i, mem in enumerate(members):
            self.w(slot_index + 4 * i, len(members) - 1 - i)
            if mem['kind'] == 'live':
                a = self.actor(10 + i)
                self.w(array + 4 * i, a)
                if mem['combat'] is not None:
                    comp = self.alloc(0x80)
                    self.w(a + 0x74, comp)
                    self.w(comp + 4, a)
                    self.w(comp + 0x60, mem['combat'])
                    if mem['body'] is not None:
                        body = self.alloc(0x40)
                        self.w(a + 0x60, body)
                        self.w(body + 0x10, mem['body'][0])
                        self.w(body + 0x14, mem['body'][1])
            else:
                if mem['slot'] is not None:
                    sd = self.alloc(0x200)
                    self.w(sd + 0x174, mem['slot'][0])
                    self.w(sd + 0x1e4, mem['slot'][1])
                    self.w(slot_defs + 4 * (len(members) - 1 - i), sd)
        # d4 target
        t = c['d4']
        if t is not None:
            gtid = 0x40004
            self.w(self.org + 0xd4, gtid)
            if t != 'stale':
                obj = self.alloc(0x100)
                slot = gtid & 0xffff
                self.u.mem_write(self.registry + 4 + slot * 2, struct.pack('<H', gtid >> 16))
                self.w(self.registry + 0x20004 + slot * 4, obj)
                self.w(obj + 8, 1 if t == 'destructed' else 0)
                comp = self.alloc(0x80)
                self.w(obj + 0xc0, comp)
                self.w(comp + 0x3c, 7)
                self.w(comp + 0x40, 7 if t == 'complete' else 9)
        self.descriptor = self.alloc(0x40)
        g = c['group']
        if g is not None:
            for i, k in enumerate(['byte1c', 'byte1d', 'byte1e']):
                self.wb(self.descriptor + 0x1c + i, int(g[k]))
        self.hooks.update({
            0x57a0a0: self.step('MoraleTick'), 0x579600: self.step('EngagementExperience'),
            0x5785d0: self.step('AttackEfficiency'), 0x578660: self.step('Shortfall'),
            0x57b5a0: self.step('Regenerate'), 0x57aeb0: self.regroup_check,
            0x515f40: self.flare, 0x57c680: self.move_to_leader, 0x540690: self.marker,
            0x45d520: self.entrench_time, 0x4d0a80: self.in_supply, 0x4d49b0: self.state_group,
            0x4d64f0: self.set_state, 0x4d1350: self.notify, 0x404f60: self.diagnostic,
        })

    def actor(self, tag):
        a = self.alloc(0x200)
        self.w(a + 4, self.alloc(0x600))
        self.tags[a] = tag
        return a

    def tag(self, p):
        return self.tags.get(p, 0 if p == 0 else -1)

    def ev(self, name, value=None):
        self.events.append({name: value})

    def flag(self, name, value):
        def handler(m):
            self.ev(name, bool(value))
            self.ret(0, int(bool(value)))
        return handler

    def allegiance(self, pointer):
        def handler(m):
            self.ev('Allegiance', self.tag(pointer))
            self.ret(0, pointer)
        return handler

    def step(self, name):
        def handler(m):
            assert self.reg(UC_X86_REG_ECX) == self.org
            self.ev(name, self.arg())
            self.ret(4)
        return handler

    def regroup_check(self, m):
        self.ev('RegroupCheck', self.c['regroup_ok'])
        self.ret(0, int(self.c['regroup_ok']))

    def flare(self, m):
        self.ev('Flare', [self.tag(self.reg(UC_X86_REG_ECX)), self.arg(), self.arg(1), self.arg(2)])
        self.ret(12)

    def move_to_leader(self, m):
        assert self.reg(UC_X86_REG_ECX) == self.org
        self.ev('MoveToLeader', [self.arg(), self.arg(1)])
        self.ret(8)

    def marker(self, m):
        self.ev('Marker', self.tag(self.arg()))
        self.ret(4)

    def entrench_time(self, m):
        self.ev('EntrenchTime', self.tag(self.arg()))
        self.ret_f32(4, self.c['entrench_time'])

    def in_supply(self, m):
        self.ev('InSupply', self.c['in_supply'])
        self.ret(0, int(self.c['in_supply']))

    def state_group(self, m):
        self.ev('StateGroup', self.arg())
        self.ret(4, self.descriptor if self.c['group'] is not None else 0)

    def set_state(self, m):
        assert self.reg(UC_X86_REG_ECX) == self.owner
        self.ev('SetState', [self.arg(), bool(self.arg(1) & 0xff)])
        self.ret(8)

    def notify(self, m):
        self.ev('Notify', self.tag(self.reg(UC_X86_REG_ECX)))
        self.ret(0)

    def diagnostic(self, m):
        self.ev('UnableToRegroup')
        self.ret(0)

    def run(self):
        self.call(0x57c8e0, self.org, (self.c['delta'],))
        return {'events': self.events, 'current_value': self.r(self.org + 0x68),
                'maximum_value': self.r(self.org + 0x6c), 'b0': self.rb(self.org + 0xb0),
                'b8e': self.rb(self.org + 0x8e), 'd4': self.r(self.org + 0xd4)}


def base(name, **k):
    c = dict(name=name, mode=0, now=f32(50), regroup_time=f32(5), delta=f32(0.1), def2f4=False,
             owner=dict(flags8=0, flags100=0, engaged=False, organization=True, xy=[f32(1), f32(2)],
                        properties=False),
             shortage=f32(0), allegiance=True, mind=dict(busy64=False, busy7c=False),
             leader=dict(flags8=0, xy=[f32(1), f32(2)]), primary=0, b0=False, b8e=False,
             regroup_started=f32(40), entrench_base=f32(3), objects_count=0, object10=None,
             members=[], d4=None, group=dict(byte1c=0, byte1d=0, byte1e=0), regroup_ok=True, entrench_time=f32(4), in_supply=True)
    c.update(k)
    return c


def cases():
    rows = []
    live = lambda combat=10.0, body=(50.0, 100.0): dict(kind='live', combat=f32(combat),
                                                        body=None if body is None else [f32(body[0]), f32(body[1])])
    dead = lambda flags=0x20, value=7.5: dict(kind='dead', slot=None if flags is None else [flags, f32(value)])
    rows.append(base('no_leader', leader=None))
    rows.append(base('no_leader_targeted', leader=None, owner=dict(flags8=0, flags100=0, engaged=True,
                     organization=True, xy=[f32(1), f32(2)], properties=False)))
    rows.append(base('plain'))
    rows.append(base('targeted_engaged_bit', owner=dict(flags8=0, flags100=0x10, engaged=True, organization=True,
                     xy=[f32(1), f32(2)], properties=False)))
    rows.append(base('no_organization_component', owner=dict(flags8=0, flags100=0, engaged=True,
                     organization=False, xy=[f32(1), f32(2)], properties=False)))
    for name, members in [
        ('values', [live(), live(3.3, (1.0, 3.0)), dead(), dead(0, 2.0), dead(None),
                    dict(kind='live', combat=None, body=None), live(2.5, None)]),
        ('fractions', [live(0.1, (1.0, 3.0)), live(0.2, (2.0, 7.0)), live(1e7, (1.0, 1.0)), dead(0x21, 0.3)])]:
        rows.append(base(f'combat_value_{name}', members=members))
    rows.append(base('flare', b0=True, objects_count=5, object10=f32(1)))
    rows.append(base('flare_few_objects', b0=True, objects_count=4, object10=f32(1)))
    for m, lead in [(dict(busy64=True, busy7c=False), dict(flags8=0, xy=[f32(5), f32(6)])),
                    (dict(busy64=False, busy7c=True), dict(flags8=0, xy=[f32(5), f32(6)])),
                    (dict(busy64=True, busy7c=False), dict(flags8=0, xy=[f32(1), f32(2)])),
                    (dict(busy64=True, busy7c=False), dict(flags8=0x40000000, xy=[f32(1), f32(2)])),
                    (None, dict(flags8=0, xy=[f32(5), f32(6)]))]:
        rows.append(base(f'follow_{m}_{lead["flags8"]}_{lead["xy"]}', mind=m, leader=lead))
    rows.append(base('follow_both_bit30', mind=dict(busy64=True, busy7c=False),
                     owner=dict(flags8=0x40000000, flags100=0, engaged=False, organization=True,
                                xy=[f32(1), f32(2)], properties=False),
                     leader=dict(flags8=0x40000000, xy=[f32(1), f32(2)])))
    rows.append(base('follow_mode6', mode=6, mind=dict(busy64=True, busy7c=False),
                     leader=dict(flags8=0, xy=[f32(5), f32(6)])))
    for supply, flag in [(True, 0), (False, 0), (True, 0x80000), (False, 0x80000)]:
        for shortage in [0.0, 0.25, -1.0]:
            rows.append(base(f'supply_{supply}_{flag}_{shortage}', def2f4=True, in_supply=supply,
                             shortage=f32(shortage), members=[live(), dict(kind='dead', slot=None)],
                             owner=dict(flags8=0, flags100=flag, engaged=False, organization=True,
                                        xy=[f32(1), f32(2)], properties=False)))
    for g, engaged, flag in [(dict(byte1c=1, byte1d=0, byte1e=0), False, 0),
                             (dict(byte1c=1, byte1d=0, byte1e=0), True, 0),
                             (dict(byte1c=0, byte1d=0, byte1e=0), False, 0),
                             (dict(byte1c=1, byte1d=0, byte1e=0), False, 0x80000), (None, False, 0)]:
        rows.append(base(f'regenerate_{g}_{engaged}_{flag}', def2f4=True, group=g,
                         owner=dict(flags8=0, flags100=flag, engaged=engaged, organization=True,
                                    xy=[f32(1), f32(2)], properties=False)) if g is not None else
                    base(f'regenerate_none_{engaged}_{flag}', def2f4=True, group=None,
                         owner=dict(flags8=0, flags100=flag | 0x400000, engaged=engaged, organization=True,
                                    xy=[f32(1), f32(2)], properties=False)))
    for primary, mind, started, ok in [(0, dict(busy64=False, busy7c=False), f32(40), True),
                                      (0, dict(busy64=False, busy7c=False), f32(40), False),
                                      (0, dict(busy64=False, busy7c=False), f32(46), False),
                                      (0, dict(busy64=False, busy7c=False), f32(45), False),
                                      (None, None, f32(10), False),
                                      (None, dict(busy64=False, busy7c=False), f32(46), False),
                                      (None, dict(busy64=False, busy7c=False), f32(45), True)]:
        rows.append(base(f'regroup_{primary}_{mind}_{started}_{ok}', primary=primary, mind=mind,
                         regroup_started=started, regroup_ok=ok,
                         owner=dict(flags8=0, flags100=0x400 | 0x400000, engaged=False, organization=True,
                                    xy=[f32(1), f32(2)], properties=False)))
    rows.append(base('marker', b8e=True, objects_count=5, object10=f32(2)))
    rows.append(base('marker_minus_one', b8e=True, objects_count=5, object10=f32(-1)))
    rows.append(base('marker_few', b8e=True, objects_count=4, object10=f32(2)))
    for t, now, prim in [(4.0, 50.0, None), (47.0, 50.0, None), (48.0, 50.0, None), (46.9999, 50.0, None), (47.0, 50.0, 0),
                         (47.0, 50.0, 0x20000000), (40.0, 50.0, 0x20000000)]:
        for props in [False, True]:
            rows.append(base(f'entrench_{t}_{now}_{prim}_{props}', group=dict(byte1c=0, byte1d=0, byte1e=1),
                             entrench_time=f32(t), now=f32(now), primary=prim,
                             owner=dict(flags8=0, flags100=0, engaged=False, organization=True,
                                        xy=[f32(1), f32(2)], properties=props)))
    rows.append(base('entrench_gate_off', group=dict(byte1c=0, byte1d=0, byte1e=0)))
    for t in ['stale', 'destructed', 'complete', 'incomplete']:
        rows.append(base(f'd4_{t}', d4=t, group=dict(byte1c=0, byte1d=0, byte1e=0)))
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
    report('native-org-tick-fixtures.json', __doc__, out, visited)


if __name__ == '__main__':
    main()
