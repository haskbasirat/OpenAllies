"""Original City component tick 5647A0, combat value 4D0180 with the Performer
value 57DC40, and the City block hand-over 5643A0 under Unicorn.

5647A0, 4D0180, 57DC40, the kingdom iterators 4F0680/4F06B0 and the array
fill 45F180 execute unchanged. Seams: the City actor (City +4) and every
candidate and kingdom object are fixture objects whose virtuals +108, +134,
+138 and +140 answer from the fixture's ownership and relation table and are
recorded in call order; the allocator 6F7810 returns fresh fixture memory
filled with 0xCD (so the 45F180 zero fill is proven), 6F7430 is a silent free,
and the capture attempt 4D5230 records (City actor, winner, record words).
City +20 starts at 0x55 and its final byte is recorded. Candidate GTIDs go
through the [B364C0] table with live, stale-generation and null entries;
World +154/+158 is the kingdom table, null slots included, and kingdom +18C
indices are permuted against the slot order.

5643A0 runs over City +10 with live, stale and null entries; the block virtual
+160 records (block, kingdom).

Usage: native_city_capture.py (AA.exe and the accepted v14 copy are located
by the shared harness relative to this repository).
"""
import copy
import struct
from harness import Machine, PRECISIONS, f32, install_world, report, REGISTRY_ROOT
from unicorn.x86_const import UC_X86_REG_ECX

FRIEND, ENEMY = 'friend', 'enemy'
TINY = struct.unpack('<f', struct.pack('<I', 0x30800000))[0]  # 2^-30


class Run(Machine):
    def __init__(self, c, precision):
        super().__init__(precision)
        self.u.mem_map(0, 0x1000)  # fs:[0] exception-list reads
        self.c = copy.deepcopy(c)
        world = install_world(self)
        self.registry = self.alloc(0x20004 + 0x10000 * 4 + 0x10)
        self.w(REGISTRY_ROOT, self.registry)
        self.tags = {}
        self.objects = {}
        # kingdoms
        slots = self.c.get('slots', [])
        table = self.alloc(4 * max(1, len(slots)))
        for i, k in enumerate(slots):
            self.w(table + 4 * i, self.kingdom(k) if k else 0)
        self.w(world + 0x154, table)
        self.w(world + 0x158, len(slots))
        # candidates
        for tag, view in self.c.get('actors', {}).items():
            self.actor(int(tag), view)
        # City component and its actor
        self.city = self.alloc(0x40)
        city_actor = self.alloc(0x100)
        self.tags[city_actor] = 'city'
        owners = list(self.c.get('city_owner', [0]))

        def city_owner(m):
            owner = owners.pop(0) if len(owners) > 1 else owners[0]
            self.events.append({'CityOwner': owner})
            self.ret(0, self.objects[owner] if owner else 0)
        self.w(city_actor, self.vtable({0x108: city_owner}))
        self.w(self.city + 4, city_actor)
        self.wb(self.city + 0x20, 0x55)
        self.w(self.city + 0x18, self.gtid_list(self.c.get('list', [])))
        self.w(self.city + 0x10, self.gtid_list(self.c.get('blocks', [])))
        self.hooks.update({0x6f7810: self.allocate, 0x6f7430: self.free, 0x4d5230: self.capture})

    def tag(self, p):
        return self.tags.get(p, 0 if p == 0 else -1)

    def relation(self, a, b):
        return self.c.get('relations', {}).get('%d:%d' % (a, b), '')

    def kingdom(self, k):
        tag = k['tag']
        if tag in self.objects:
            return self.objects[tag]
        obj = self.alloc(0x200)
        self.objects[tag] = obj
        self.tags[obj] = tag

        def v138(m, tag=tag):
            other = self.tag(self.arg())
            self.events.append({'V138': [tag, other]})
            self.ret(4, int(self.relation(tag, other) == FRIEND))

        def v140(m, tag=tag):
            other = self.tag(self.arg())
            self.events.append({'V140': [tag, other]})
            self.ret(4, int(self.relation(tag, other) == ENEMY))
        self.w(obj, self.vtable({0x138: v138, 0x140: v140}))
        self.w(obj + 0x18c, k['index'])
        return obj

    def actor(self, tag, view):
        a = self.alloc(0x200)
        self.objects[tag] = a
        self.tags[a] = tag
        owner = view['owner']

        def answer(slot, arg):
            forced = view.get('answers', {}).get(slot)
            if forced is not None:
                return forced
            if slot == '134':
                return arg == owner
            if slot == '138':
                return arg == owner or self.relation(owner, arg) == FRIEND
            return self.relation(owner, arg) == ENEMY

        def make(slot):
            def handler(m):
                other = self.tag(self.arg())
                self.events.append({'V' + slot: [tag, other]})
                self.ret(4, int(answer(slot, other)))
            return handler

        def v108(m):
            self.events.append({'Owner': tag})
            self.ret(0, self.objects[owner] if owner else 0)
        self.w(a, self.vtable({0x108: v108, 0x134: make('134'), 0x138: make('138'), 0x140: make('140'),
                               0x160: self.change_kingdom}))
        for offset, field, key in ((0x7c, 0x68, 'organization'), (0x98, 0x30, 'component98'),
                                   (0x88, 0x10, 'denizen')):
            if view.get(key) is not None:
                comp = self.alloc(0x80)
                self.w(comp + field, view[key])
                self.w(a + offset, comp)
        perf = view.get('performer')
        if perf is not None:
            p = self.alloc(0x80)
            self.w(p + 4, a)
            self.w(p + 0x60, perf['value'])
            if perf.get('body') is not None:
                body = self.alloc(0x40)
                self.w(body + 0x10, perf['body'][0])
                self.w(body + 0x14, perf['body'][1])
                self.w(a + 0x60, body)
            self.w(a + 0x74, p)

    def gtid_list(self, entries):
        head = 0
        for i, e in reversed(list(enumerate(entries))):
            index = len(self.tags) * 3 + i + 1
            generation = 0x100 + index
            node = self.alloc(8)
            self.w(node, (generation << 16) | index)
            self.w(node + 4, head)
            head = node
            stored = generation + 1 if e['state'] == 'stale' else generation
            self.u.mem_write(self.registry + 4 + index * 2, struct.pack('<H', stored))
            target = 0 if e['state'] == 'null' else self.objects[e['actor']]
            self.w(self.registry + 0x20004 + index * 4, target)
        return head

    def change_kingdom(self, m):
        self.events.append({'ChangeKingdom': [self.tag(self.reg(UC_X86_REG_ECX)), self.tag(self.arg())]})
        self.ret(4)

    def allocate(self, m):
        size = self.arg()
        ptr = self.alloc(size + 16)
        self.u.mem_write(ptr, b'\xcd' * (size + 16))
        self.ret(0, ptr)

    def free(self, m):
        self.ret(0)

    def capture(self, m):
        record = self.arg(1)
        words = [self.r(record + 4 * i) for i in range(7)]
        self.events.append({'Capture': [self.tag(self.reg(UC_X86_REG_ECX)), self.tag(self.arg()), words]})
        self.ret(8, 1)

    def run(self):
        if self.c['function'] == 'tick':
            self.call(0x5647a0, self.city, ())
            return {'events': self.events, 'siege': self.rb(self.city + 0x20)}
        if self.c['function'] == 'blocks':
            k = self.c['kingdom']
            self.call(0x5643a0, self.city, (self.objects[k] if k else 0,))
            return {'events': self.events}
        if self.c['function'] == 'value':
            self.call(0x4d0180, self.objects[1], ())
            return {'st0': self.st0_ext()}
        raise ValueError(self.c['function'])


def a(owner, value=None, **k):
    """A candidate of `owner` whose whole value is the Denizen +10 unless
    other fields are given."""
    view = dict(owner=owner)
    if value is not None:
        view['denizen'] = f32(value)
    view.update(k)
    return view


def live(*actors):
    return [dict(actor=t, state='live') for t in actors]


def kingdoms(n, order=None):
    order = order or list(range(n))
    return [dict(tag=100 + i, index=order[i]) for i in range(n)]


def rel(pairs):
    out = {}
    for x, y, r in pairs:
        out['%d:%d' % (x, y)] = r
        out['%d:%d' % (y, x)] = r
    return out


# Kingdoms 100..103. 100/101 enemies, 102 friend of 100 and enemy of 101,
# 103 neutral to all but an enemy of 101.
REL = rel([(100, 101, ENEMY), (100, 102, FRIEND), (101, 102, ENEMY), (101, 103, ENEMY)])


def tick_cases():
    K = kingdoms(4)
    rows = [
        ('no_owner_single', dict(city_owner=[0], actors={1: a(101, 5.0)}, list=live(1))),
        ('no_owner_empty', dict(city_owner=[0], actors={}, list=[])),
        ('owner_empty_list', dict(city_owner=[100], actors={}, list=[])),
        ('owner_defended_siege', dict(city_owner=[100], actors={1: a(100, 3.0), 2: a(101, 5.0)}, list=live(1, 2))),
        ('owner_defended_holds', dict(city_owner=[100], actors={1: a(100, 6.0), 2: a(101, 5.0)}, list=live(1, 2))),
        ('attack_no_defender', dict(city_owner=[100], actors={2: a(101, 5.0)}, list=live(2))),
        ('friend_does_not_defend', dict(city_owner=[100], actors={1: a(102, 4.0), 2: a(101, 4.0)}, list=live(1, 2))),
        ('attack_equals_ally', dict(city_owner=[100], actors={1: a(100, 2.0), 3: a(102, 2.0), 2: a(101, 4.0)},
                                    list=live(1, 3, 2))),
        ('attack_below_ally', dict(city_owner=[100], actors={1: a(100, 2.0), 3: a(102, 2.0), 2: a(101, 3.9990234375)},
                                   list=live(1, 3, 2))),
        ('no_enemy_no_siege_check', dict(city_owner=[100], actors={1: a(100, 1.0), 4: a(103, 7.0)}, list=live(1, 4))),
        ('friend_tie_first_wins', dict(city_owner=[0], actors={1: a(100, 5.0), 3: a(102, 5.0)}, list=live(1, 3))),
        ('friend_more_wins', dict(city_owner=[0], actors={1: a(100, 5.0), 3: a(102, 6.0)}, list=live(1, 3))),
        ('friend_under_threat', dict(city_owner=[0], actors={1: a(100, 5.0), 3: a(102, 1.0), 2: a(101, 1.0)},
                                     list=live(1, 3, 2))),
        ('neutral_takes_from_enemy', dict(city_owner=[101], actors={4: a(103, 2.0)}, list=live(4))),
        ('stale_and_null_entries', dict(city_owner=[100], actors={1: a(100, 1.0), 2: a(101, 5.0), 5: a(101, 9.0)},
                                        list=[dict(actor=5, state='stale'), dict(actor=1, state='null'),
                                              dict(actor=2, state='live')])),
        ('owner_changes_before_handover', dict(city_owner=[100, 101], actors={1: a(100, 3.0), 2: a(101, 5.0)},
                                               list=live(1, 2))),
        ('negative_attack', dict(city_owner=[100], actors={2: a(101, -2.0)}, list=live(2))),
        ('negative_defence_decides', dict(city_owner=[100], actors={1: a(100, -1.0), 2: a(101, 1.0)},
                                          list=live(1, 2))),
        ('forced_answers', dict(city_owner=[100], actors={
            1: a(101, 2.0, answers={'138': True, '134': False}),
            2: a(102, 3.0, answers={'138': False, '140': True})}, list=live(1, 2))),
    ]
    out = []
    for name, c in rows:
        c.setdefault('slots', K)
        c.setdefault('relations', REL)
        out.append((name, c))
    # Kingdom table with null slots and +18C indices permuted.
    permuted = [None, dict(tag=100, index=3), None, dict(tag=101, index=0), dict(tag=102, index=2),
                None, dict(tag=103, index=1), None]
    out.append(('null_slots_permuted', dict(city_owner=[101], slots=permuted, relations=REL,
                                            actors={1: a(100, 2.0), 3: a(102, 1.0), 4: a(103, 1.5)},
                                            list=live(1, 3, 4))))
    out.append(('null_slots_no_owner', dict(city_owner=[0], slots=permuted, relations=REL,
                                            actors={4: a(103, 1.5), 3: a(102, 1.0)}, list=live(4, 3))))
    # The decision with no kingdom slot at all and with only null slots.
    out.append(('no_kingdom_slots', dict(city_owner=[0], slots=[], relations=REL,
                                         actors={}, list=[])))
    out.append(('only_null_slots', dict(city_owner=[0], slots=[None, None], relations=REL,
                                        actors={}, list=[])))
    # Precision: the defence is 2^-30 with an extended or double sum and 0
    # with a single one (organization 1.0 + component98 2^-30 + Denizen -1.0).
    tiny = dict(organization=f32(1.0), component98=f32(TINY), denizen=f32(-1.0))
    out.append(('precision_defence', dict(city_owner=[100], slots=K, relations=REL,
                                          actors={1: dict(owner=100, **tiny), 2: a(101, 5.0)}, list=live(1, 2))))
    # Performer terms: ratio 1/3 times 3, the +98 local rounding, Body-less.
    third = dict(performer=dict(value=f32(3.0), body=[f32(1.0), f32(3.0)]))
    local = dict(organization=f32(1.0), component98=f32(TINY), performer=dict(value=f32(TINY), body=None),
                 denizen=f32(-1.0))
    out.append(('precision_performer', dict(city_owner=[0], slots=K, relations=REL,
                                            actors={1: dict(owner=100, **third), 3: dict(owner=102, **local),
                                                    2: dict(owner=101, **third)}, list=live(1, 3, 2))))
    seventh = dict(organization=f32(0.1), performer=dict(value=f32(7.0), body=[f32(2.0), f32(7.0)]),
                   denizen=f32(0.3))
    out.append(('precision_mixed_sum', dict(city_owner=[100], slots=K, relations=REL,
                                            actors={1: dict(owner=100, **seventh), 3: dict(owner=102, **seventh),
                                                    2: dict(owner=101, **seventh), 6: dict(owner=101, **seventh)},
                                            list=live(1, 3, 2, 6))))
    return out


def value_cases():
    fields = [
        ('empty', {}),
        ('organization', dict(organization=f32(2.5))),
        ('component98', dict(component98=f32(0.75))),
        ('org_plus_98_unrounded', dict(organization=f32(1.0), component98=f32(TINY))),
        ('org_98_denizen', dict(organization=f32(1.0), component98=f32(TINY), denizen=f32(-1.0))),
        ('performer_body', dict(performer=dict(value=f32(3.0), body=[f32(1.0), f32(3.0)]))),
        ('performer_no_body', dict(performer=dict(value=f32(0.1), body=None))),
        ('performer_after_local', dict(organization=f32(1.0), component98=f32(TINY),
                                       performer=dict(value=f32(0.1), body=[f32(2.0), f32(7.0)]))),
        ('all_four', dict(organization=f32(0.1), component98=f32(0.2),
                          performer=dict(value=f32(7.0), body=[f32(2.0), f32(7.0)]), denizen=f32(0.3))),
    ]
    return [('value_' + name, dict(actors={1: dict(owner=100, **f)}, slots=kingdoms(1), list=[]))
            for name, f in fields]


def block_cases():
    acts = {1: a(100), 2: a(100), 3: a(101)}
    entries = [dict(actor=1, state='live'), dict(actor=2, state='stale'), dict(actor=3, state='null'),
               dict(actor=3, state='live'), dict(actor=1, state='live')]
    return [
        ('blocks_to_kingdom', dict(actors=acts, blocks=entries, kingdom=101, slots=kingdoms(2))),
        ('blocks_to_none', dict(actors=acts, blocks=entries, kingdom=0, slots=kingdoms(2))),
        ('blocks_empty', dict(actors=acts, blocks=[], kingdom=100, slots=kingdoms(2))),
    ]


def main():
    out = []
    visited = {}
    jobs = [(n, dict(c, function='tick'), PRECISIONS) for n, c in tick_cases()]
    jobs += [(n, dict(c, function='value'), PRECISIONS) for n, c in value_cases()]
    jobs += [(n, dict(c, function='blocks'), PRECISIONS[:1]) for n, c in block_cases()]
    for name, c, precisions in jobs:
        for precision in precisions:
            m = Run(c, precision)
            try:
                result = m.run()
            except Exception as exc:
                raise RuntimeError(f'{name} {precision}') from exc
            visited.update(m.visited)
            stored = copy.deepcopy(c)
            stored['actors'] = {str(k): v for k, v in stored.get('actors', {}).items()}
            out.append({'input': {**stored, 'name': name, 'precision': precision}, 'output': result})
    report('native-city-capture-fixtures.json',
           __doc__ + '\nTick and value cases at every precision control; the block hand-over has no '
           'arithmetic (one precision).', out, visited)


if __name__ == '__main__':
    main()
