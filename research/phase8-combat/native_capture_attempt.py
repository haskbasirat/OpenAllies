"""Original capture attempt 4D5230 under Unicorn.

Executes the unchanged 4D5230: delegation to the outer actor (virtual +158 /
+100, recursive), the event-kind-1 and source gates, new-owner resolution
(+A8 ? source : source+108), the 54BEA0 notification, capture announcement
(4E9560 kingdom name or +24 actor name, then 42D490 event 0x10), wall (585A20)
and component88 (56ACB0) transfers, ownership change (+160), owner-change
announcement (42D490 event 0xF), strategic-AI lost/gained notifications
(79EA90/7B2050/7B21C0, only with the SAI global B36528), capture statistics
(58E870 indices 0x1A/0x19, 0x2B/0x2A, 0x1D/0x1C with 58F010 on the new owner)
and the 4CEBE0 capture reward. All of those callees are recorded seams; the
reward itself is proven in native-morale-fixtures.json. CString reference
counts (fixture value 1000) never reach zero, so no release virtual runs.
"""
import copy
from harness import Machine, PRECISIONS, f32, install_settings, install_world, report
from unicorn.x86_const import UC_X86_REG_ECX

MANAGER = 0xb39d8c
SAI_GLOBAL = 0xb36528
NOTIFY_GLOBAL = 0xb36520


class Run(Machine):
    def __init__(self, c, precision):
        super().__init__(precision)
        self.u.mem_map(0, 0x1000)
        self.c = copy.deepcopy(c)
        install_settings(self, {})
        install_world(self)
        self.w(MANAGER, 0xabcd0000)
        self.w(NOTIFY_GLOBAL, 0xabcd1000)
        self.w(SAI_GLOBAL, 0xabcd2000 if c['sai'] else 0)
        self.tags = {}
        self.actors = {}
        self.names = {}
        for tag in self.c['actors']:
            a = self.alloc(0x400)
            self.w(a + 4, self.alloc(0x600))
            self.tags[a] = int(tag)
            self.actors[int(tag)] = a
        for tag, view in self.c['actors'].items():
            self.setup(int(tag), view)
        self.hooks.update({
            0x54bea0: self.notify, 0x4e9560: self.kingdom_name, 0x42d490: self.message,
            0x585a20: self.seam('TransferWall', 4, lambda: [self.tag(self.arg())]),
            0x56acb0: self.seam('TransferComponent88', 4, lambda: [self.tag(self.arg())]),
            0x79ea90: self.strategic_ai, 0x7b2050: self.seam('SaiLost', 4, lambda: [self.sai_tag(), self.tag(self.arg())]),
            0x7b21c0: self.seam('SaiGained', 4, lambda: [self.sai_tag(), self.tag(self.arg())]),
            0x58e870: self.seam('AddStatistic', 8, lambda: [self.stat_tag(), self.arg(), self.arg(1)]),
            0x58f010: self.seam('CaptureStatistics', 4, lambda: [self.stat_tag(), self.tag(self.arg())]),
            0x58ee00: self.classification,
            0x4cebe0: self.seam('CaptureReward', 8, lambda: [self.tag(self.reg(UC_X86_REG_ECX)), self.tag(self.arg()),
                                                             self.r(self.arg(1) + 0x18)]),
        })

    def tag(self, p):
        if p in self.tags:
            return self.tags[p]
        if p in self.names:
            return 'name:%s' % self.names[p]
        return 0 if p == 0 else -1

    def sai_tag(self):
        return self.tags.get(self.reg(UC_X86_REG_ECX), -1)

    def stat_tag(self):
        return self.tags.get(self.reg(UC_X86_REG_ECX), -1)

    def cstring(self, label):
        header = self.alloc(0x40)
        self.w(header, self.vtable({4: lambda m: (_ for _ in ()).throw(RuntimeError('release'))}))
        self.w(header + 0xc, 1000)
        self.names[header + 0x10] = label
        return header + 0x10

    def seam(self, name, pop, args):
        def handler(m):
            self.events.append({name: args()})
            self.ret(pop)
        return handler

    def setup(self, tag, view):
        a = self.actors[tag]
        name = self.cstring(str(tag))

        def delegates(m, view=view, tag=tag):
            self.events.append({'Delegates': tag})
            self.ret(0, int(view.get('delegates', False)))

        def outer(m, view=view, tag=tag):
            self.events.append({'Outer': tag})
            target = view.get('outer', 0)
            self.ret(0, self.actors[target] if target else 0)

        def allegiance(m, view=view, tag=tag):
            values = view.setdefault('allegiance_sequence', [])
            owner = values.pop(0) if values else view.get('allegiance', 0)
            self.events.append({'Allegiance': [tag, owner]})
            self.ret(0, self.actors[owner] if owner else 0)

        def is_allegiance(m, view=view, tag=tag):
            self.events.append({'IsAllegiance': tag})
            self.ret(0, int(view.get('is_allegiance', False)))

        def name_slot(m, name=name, tag=tag):
            out = self.arg()
            self.w(out, name)
            self.events.append({'Name': tag})
            self.ret(4, out)

        def set_allegiance(m, tag=tag):
            self.events.append({'SetAllegiance': [tag, self.tag(self.arg())]})
            self.ret(4)
        self.w(a, self.vtable({0x158: delegates, 0x100: outer, 0x108: allegiance, 0xa8: is_allegiance,
                               0x24: name_slot, 0x160: set_allegiance}))
        if view.get('wall'):
            comp = self.alloc(0x40)
            self.w(a + 0x98, comp)
            self.tags[comp] = 'wall:%d' % tag
        if view.get('component88'):
            comp = self.alloc(0x40)
            self.w(a + 0x88, comp)
            self.tags[comp] = 'c88:%d' % tag
        self.w(a + 0xd4, 0x777 if view.get('d4') else 0)
        if view.get('statistics'):
            stats = self.alloc(0x40)
            self.w(a + 0x32c, stats)
            self.tags[stats] = 'stats:%d' % tag
        if view.get('sai'):
            sai = self.alloc(0x40)
            self.tags[sai] = 'sai:%d' % tag
            view['_sai'] = sai

    def notify(self, m):
        assert self.reg(UC_X86_REG_ECX) == 0xabcd1000
        self.events.append({'CaptureNotification': [self.tag(self.arg()), self.tag(self.arg(1))]})
        self.ret(8)

    def kingdom_name(self, m):
        kingdom = self.tag(self.reg(UC_X86_REG_ECX))
        out = self.arg()
        assert self.arg(1) == 0
        self.w(out, self.cstring('kingdom:%s' % kingdom))
        self.events.append({'KingdomName': kingdom})
        self.ret(8, out)

    def message(self, m):
        assert self.arg() == 0xabcd0000
        event = self.arg(1)
        count = 4 if event == 0x10 else 3
        args = [self.arg(2 + i) for i in range(count)]
        self.events.append({'Message': [event] + [self.tag(a) for a in args]})
        self.ret(0)

    def strategic_ai(self, m):
        assert self.reg(UC_X86_REG_ECX) == 0xabcd2000
        owner = self.tag(self.arg())
        view = self.c['actors'].get(str(owner), {}) if owner else {}
        sai = view.get('_sai', 0)
        self.events.append({'StrategicAi': owner})
        self.ret(4, sai)

    def classification(self, m):
        tag = self.tag(self.arg())
        self.events.append({'Classification': tag})
        self.ret(0, self.c['classification'])

    def run(self):
        record = self.alloc(0x20)
        self.w(record + 0x14, self.c['event_kind'])
        self.w(record + 0x18, self.c['record_kind'])
        source = self.actors[self.c['source']] if self.c['source'] else 0
        result = self.call(0x4d5230, self.actors[1], (source, record)) & 0xff
        return {'result': result, 'events': self.events}


def cases():
    rows = []
    # 1 captured, 2 source actor, 3 source kingdom, 4 old owner, 5 outer actor.
    def actors(**k):
        return {
            '1': dict(delegates=k.get('delegates', False), outer=5, allegiance=k.get('old', 4),
                      wall=k.get('wall', False), component88=k.get('c88', False), d4=k.get('d4', False)),
            '2': dict(is_allegiance=k.get('source_is_allegiance', False),
                      allegiance_sequence=k.get('source_allegiance', [3, 3, 3])),
            '3': dict(statistics=k.get('new_stats', True), sai=k.get('new_sai', True)),
            '4': dict(statistics=k.get('old_stats', True), sai=k.get('old_sai', True)),
            '5': dict(delegates=False, allegiance=k.get('outer_old', 4)),
        }
    for name, kw, extra in [
        ('basic', {}, {}), ('wrong_event_kind', {}, dict(event_kind=2)), ('no_source', {}, dict(source=0)),
        ('same_owner', dict(old=3), {}), ('source_is_kingdom', dict(source_is_allegiance=True, old=4), {}),
        ('source_kingdom_same', dict(source_is_allegiance=True, old=2), {}),
        ('source_no_allegiance', dict(source_allegiance=[0, 0, 0]), {}),
        ('wall_c88_d4', dict(wall=True, c88=True, d4=True), {}),
        ('classification_1', {}, dict(classification=1)), ('classification_2', {}, dict(classification=2)),
        ('no_old_owner', dict(old=0), {}), ('no_stats', dict(old_stats=False, new_stats=False, wall=True), {}),
        ('no_sai_global', {}, dict(sai=False)), ('no_sai_objects', dict(old_sai=False, new_sai=False), {}),
        ('delegates_to_outer', dict(delegates=True), {}),
        ('delegates_outer_same_owner', dict(delegates=True, outer_old=3), {}),
        ('record_kind_7', {}, dict(record_kind=7))]:
        c = dict(name=f'capture_attempt_{name}', actors=actors(**kw), source=2, event_kind=1, record_kind=0,
                 sai=True, classification=0)
        c.update(extra)
        rows.append(c)
    return rows


def main():
    out = []
    visited = {}
    for precision in PRECISIONS[:1]:
        for c in cases():
            m = Run(c, precision)
            try:
                result = m.run()
            except Exception as exc:
                raise RuntimeError(f'{c["name"]} {precision}') from exc
            visited.update(m.visited)
            out.append({'input': {**c, 'precision': precision}, 'output': result})
    report('native-capture-attempt-fixtures.json', __doc__ + '\nNo floating-point arithmetic: one precision.',
           out, visited)


if __name__ == '__main__':
    main()
