"""Original under-siege refresh 4D6660 and recently-damaged test 4D0440.

4D6660 is the Body-damage response aa-body names Action::ResetRecovery: it
recomputes actor state 20 (under_siege). Executes unchanged 4D6660 and
4D0440 (last damage time from Body+28, else Organization+90, else wall
component+4C; strictly less than UnderSiegeRecentlyDamagedSeconds, settings
cache +40). Seams: actor virtuals +100 (outer) and +158 (delegates), 585160
(wall component predicate), 569540 (component88 predicate), 586370 (wall
component update), 4D64F0 state transition and 4D1350 notification.
"""
import copy
from harness import Machine, PRECISIONS, f32, install_settings, install_world, report
from unicorn.x86_const import UC_X86_REG_ECX


class Run(Machine):
    def __init__(self, c, precision):
        super().__init__(precision)
        self.c = copy.deepcopy(c)
        c = self.c
        install_settings(self, {0x40: c['threshold']})
        install_world(self, 0, c['now'])
        self.tags = {}
        self.actors = {}
        for tag in ('1', '2'):
            v = c['actors'].get(tag)
            if v is None:
                continue
            a = self.alloc(0x200)
            self.tags[a] = int(tag)
            self.actors[int(tag)] = a
        for tag, a in self.actors.items():
            self.setup(a, c['actors'][str(tag)])
        self.hooks.update({0x585160: self.seam('WallPredicate', 0, 'wall_predicate'),
                           0x569540: self.seam('Component88Predicate', 4, 'c88_predicate'),
                           0x586370: self.seam('WallUpdate', 0, None),
                           0x4d64f0: self.set_state, 0x4d1350: self.notify})

    def setup(self, a, v):
        self.w(a + 8, v.get('flags8', 0))
        self.w(a + 0x100, v.get('flags100', 0))
        damage = v.get('damage')  # ['body'|'org'|'wall', time]
        if damage:
            rec = self.alloc(0x40)
            self.w(rec + 0x10, damage[1])
            if damage[0] == 'body':
                self.w(a + 0x60, rec - 0x28)
            elif damage[0] == 'org':
                self.w(a + 0x7c, rec - 0x90)
            else:
                self.w(a + 0x98, rec - 0x4c)
        if v.get('wall') and not self.r(a + 0x98):
            comp = self.alloc(0x80)
            self.w(a + 0x98, comp)
        if self.r(a + 0x98):
            self.tags[self.r(a + 0x98)] = 'wall:%d' % self.tags[a]
        self.w(a + 0x90, 0x9090 if v.get('link90') else 0)
        self.w(a + 0x128, 0x1280 if v.get('engaged_list') else 0)
        if v.get('d4') is not None:
            obj = self.alloc(0x40)
            self.w(a + 0xd4, obj)
            self.wb(obj + 0x20, v['d4'])
        if v.get('c88') is not None:
            g = v['c88']
            comp = self.alloc(0x40)
            self.tags[comp] = 'c88:%d' % self.tags[a]
            self.w(a + 0x88, comp)
            self.w(comp + 0x24, 0x2424 if g['field24'] else 0)
            objs = self.alloc(0x40)
            self.w(a + 0x148, objs)
            self.w(a + 0x14c, g['count'])
            self.w(objs + 0x14, 0x1414 if g['object14'] else 0)
        seq = list(v.get('outer', []))

        def outer(m, seq=seq, tag=self.tags[a]):
            t = seq.pop(0) if len(seq) > 1 else (seq[0] if seq else 0)
            self.events.append({'Outer': tag})
            self.ret(0, self.actors.get(t, 0))

        def delegates(m, v=v, tag=self.tags[a]):
            self.events.append({'Delegates': tag})
            self.ret(0, int(v.get('delegates', False)))
        self.w(a, self.vtable({0x100: outer, 0x158: delegates}))

    def seam(self, name, pop, key):
        def handler(m):
            tag = self.tags.get(self.reg(UC_X86_REG_ECX), -1)
            arg = self.tags.get(self.arg(), -1) if pop else None
            value = self.c[key] if key else None
            self.events.append({name: [tag, arg, value]})
            self.ret(pop, int(bool(value)))
        return handler

    def set_state(self, m):
        self.events.append({'SetState': [self.arg(), bool(self.arg(1) & 0xff)]})
        self.ret(8)

    def notify(self, m):
        self.events.append({'Notify': self.tags[self.reg(UC_X86_REG_ECX)]})
        self.ret(0)

    def run(self):
        if self.c['operation'] == 'recently_damaged':
            result = self.call(0x4d0440, self.actors[1], ()) & 0xff
        else:
            result = None
            self.call(0x4d6660, self.actors[1], ())
        return {'result': result, 'events': self.events}


def cases():
    rows = []
    for src in ['body', 'org', 'wall', None]:
        for t in [0.0, 7.0, 7.000001, 2.0, 12.0, 6.999999]:
            for now, thr in [(10.0, 3.0), (10.000001, 3.0)]:
                rows.append(dict(name=f'recent_{src}_{t}_{now}', operation='recently_damaged', now=f32(now),
                                 threshold=f32(thr), wall_predicate=False, c88_predicate=False,
                                 actors={'1': dict(damage=[src, f32(t)] if src else None)}))
    base = lambda **k: dict(operation='refresh', now=f32(10), threshold=f32(3), wall_predicate=False,
                            c88_predicate=False, **k)
    recent = ['body', f32(9)]
    old = ['body', f32(1)]
    c88 = lambda count=6, o14=True, f24=True: dict(count=count, object14=o14, field24=f24)
    variants = [
        ('recent_sets', {'1': dict(damage=recent)}, {}),
        ('old_clears', {'1': dict(damage=old, flags100=0x100000)}, {}),
        ('already_set', {'1': dict(damage=recent, flags100=0x100000)}, {}),
        ('engaged_list', {'1': dict(damage=old, engaged_list=True)}, {}),
        ('d4_byte', {'1': dict(damage=old, d4=1)}, {}),
        ('d4_zero', {'1': dict(damage=old, d4=0)}, {}),
        ('destructed', {'1': dict(damage=recent, flags8=1, flags100=0x100000)}, {}),
        ('destructed_unset', {'1': dict(damage=recent, flags8=1)}, {}),
        ('wall_delegates', {'1': dict(damage=recent, wall=True, delegates=True, outer=[2]),
                            '2': dict(damage=old)}, {}),
        ('wall_not_delegating', {'1': dict(damage=recent, wall=True, delegates=False)}, {}),
        ('outer_no_link', {'1': dict(damage=recent, outer=[2], delegates=False), '2': dict()}, {}),
        ('outer_with_link', {'1': dict(damage=recent, outer=[2], link90=True), '2': dict()}, {}),
        ('delegate_outer_recent', {'1': dict(damage=old, outer=[2], delegates=True, wall=True),
                                   '2': dict(damage=recent, wall=True)}, {}),
        ('delegate_outer_engaged', {'1': dict(damage=old, outer=[2], delegates=True, wall=True),
                                    '2': dict(damage=old, engaged_list=True, wall=True)}, {}),
        ('delegate_outer_quiet', {'1': dict(damage=old, outer=[2], delegates=True, wall=True, flags100=0x100000),
                                  '2': dict(damage=old, wall=True)}, {}),
        ('c88_no_outer', {'1': dict(damage=old, c88=c88(), outer=[0])}, {}),
        ('c88_outer_wall_true', {'1': dict(damage=old, c88=c88(), outer=[2], link90=True), '2': dict(wall=True)},
         dict(wall_predicate=True)),
        ('c88_outer_c88_true', {'1': dict(damage=old, c88=c88(), outer=[2], link90=True), '2': dict(wall=True)},
         dict(c88_predicate=True)),
        ('c88_outer_both_false', {'1': dict(damage=old, c88=c88(), outer=[2], link90=True), '2': dict(wall=True)}, {}),
        ('c88_few_objects', {'1': dict(damage=old, c88=c88(count=5))}, {}),
        ('c88_no_object14', {'1': dict(damage=old, c88=c88(o14=False))}, {}),
        ('c88_no_field24', {'1': dict(damage=old, c88=c88(f24=False))}, {}),
        ('no_damage_record', {'1': dict()}, {}),
    ]
    for name, actors, extra in variants:
        rows.append({**base(name=f'refresh_{name}', actors=actors), **extra})
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
    report('native-siege-fixtures.json', __doc__, out, visited)


if __name__ == '__main__':
    main()
