"""Original KKC_CAIStateGuard::Think 5294F0 executing unchanged on an emulated
regiment Mind (sleep +34, move +68, follow +C0, attack +D4, guard +17C) with
529E60, 529C30, 529C20, 52A000, 52A810 (mode 0), 517F30, 432E40, 4CF1B0,
4D09A0, the static one-entry list 763B90/4D1970/4D0900 and the SVar reader
4BE0F0.

Seams (recorded in order): Mind virtuals +140 +B8 +1A0 +1A4, Updatable +8
virtual +4 (schedule), state virtuals +4 +C +18 +1C, object virtuals +158
+15C, object Mind virtual +90, 569300 (component threat list: returns a
case-built list), 585160, 4D4FF0, 527CE0, 51D920, 56AEC0 (null component is a
fault), 51EE20, allocator 6F7810/6F7430 and the diagnostic loggers
404ED0/404F60 (not recorded). The once-initialised statics (guard bits
[B3CF88] = 3: the SVar GuardOldTargetBias cache B3CF7C and the static list
B3CF80) are pre-initialised as after their first use.
"""
from pathlib import Path
import json, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import Machine, PRECISIONS, f32, header, instructions, sha

STATES = {'sleep': 0x34, 'move': 0x68, 'follow': 0xc0, 'attack': 0xd4, 'guard': 0x17c}


class GuardMachine(Machine):
    def __init__(self, c):
        super().__init__(c['precision'])
        self.c = c
        self.seam(0x6f7430, lambda m: self.ret(0))
        self.seam(0x6f7810, lambda m: self.ret(self.alloc(max(self.arg(1), 4))))
        self.seam(0x404ed0, lambda m: self.ret(0))
        self.seam(0x404f60, lambda m: self.ret(0))
        reg = self.alloc(0x20004 + 0x10000 * 4 + 16); self.w(0xb364c0, reg)
        self.w(0xb3cf88, 3)
        bias = self.alloc(4); self.w(bias, c['bias']); self.w(0xb3cf7c, bias)
        mvt = self.alloc(0x200)
        for slot in range(0, 0x200, 4):
            self.w(mvt + slot, self.stub(self.unexpected('mind', slot)))
        for slot, fn in {0x140: self.v_140, 0xb8: self.v_named('mind_virtual_b8'),
                         0x1a0: self.v_named('reset_default'), 0x1a4: self.v_named('state_changed')}.items():
            self.w(mvt + slot, self.stub(fn))
        uvt = self.alloc(0x10); self.w(uvt + 4, self.stub(self.v_schedule))
        svt = self.alloc(0x40)
        for slot in range(0, 0x40, 4):
            self.w(svt + slot, self.stub(self.unexpected('state', slot)))
        self.w(svt + 4, self.stub(self.v_can_activate)); self.w(svt + 0xc, self.stub(self.v_think))
        self.w(svt + 0x18, self.stub(self.v_state('enter'))); self.w(svt + 0x1c, self.stub(self.v_state('leave')))
        mind = self.mind = self.alloc(0x400)
        self.w(mind, mvt); self.w(mind + 8, uvt)
        org = self.org = self.alloc(0x100)
        defn = self.alloc(0x10); self.w(defn + 8, self.alloc(4)); self.w(org + 4, defn)
        self.w(org + 0x20, c['org_position'][0]); self.w(org + 0x24, c['org_position'][1])
        self.w(mind + 4, org)
        self.states = {}
        for key, off in STATES.items():
            p = mind + off
            meta = self.alloc(0x10); self.w(meta, self.alloc(4))
            self.w(p, svt); self.w(p + 4, mind); self.w(p + 8, meta)
            self.w(p + 0xc, c['status'][key]); self.w(p + 0x10, c['word10'][key])
            self.states[key] = p
        g = self.states['guard']
        self.wb(g + 0x14, int(c['byte14'])); self.w(g + 0x18, c['guarded'])
        head = 0
        for key in reversed(c['stack']):
            node = self.alloc(8); self.w(node, self.states[key]); self.w(node + 4, head); head = node
        self.w(mind + 0x10, head)
        for i, word in enumerate(c['words']):
            self.w(mind + 0x24 + 4 * i, word)
        ovt = self.alloc(0x200)
        for slot in range(0, 0x200, 4):
            self.w(ovt + slot, self.stub(self.unexpected('object', slot)))
        self.w(ovt + 0x158, self.stub(self.o_158)); self.w(ovt + 0x15c, self.stub(self.o_15c))
        omvt = self.alloc(0x100); self.w(omvt + 0x90, self.stub(self.om_90))
        self.objects, self.omind, self.components, self.comp_lists = {}, {}, {}, {}
        for key, o in c['objects'].items():
            self.objects[key] = self.alloc(0x200)
        for key, o in c['objects'].items():
            p = self.objects[key]
            self.w(p, ovt); self.w(p + 8, o['flags']); self.w(p + 0x14, o['gtid'])
            self.w(p + 0x20, o['position'][0]); self.w(p + 0x24, o['position'][1])
            if o['mind'] is not None:
                mm = self.alloc(0x40); self.w(mm, omvt); self.w(p + 0x70, mm); self.omind[mm] = key
            if o['component'] is not None:
                comp = self.alloc(0x40); self.wb(comp + 0x20, int(o['component']['byte20']))
                self.w(p + 0x88, comp); self.components[comp] = key
                lst = self.alloc(0x10); self.write_gtids(lst, o['component']['threats']); self.comp_lists[comp] = lst
            self.w(p + 0x98, o['word98'])
            lst = p + 0x118; self.write_gtids(lst, o['threats'])
            if o['registered']:
                idx, gen = o['gtid'] & 0xffff, o['gtid'] >> 16
                self.u.mem_write(reg + 4 + 2 * idx, struct.pack('<H', gen)); self.w(reg + 0x20004 + 4 * idx, p)
        self.names = {p: k for k, p in self.objects.items()}
        self.state_names = {p: k for k, p in self.states.items()}
        for address, fn in {0x569300: self.s_threats, 0x585160: self.s_585160, 0x4d4ff0: self.s_relation,
                            0x527ce0: self.s_527ce0, 0x51d920: self.s_follow, 0x56aec0: self.s_settle,
                            0x51ee20: self.s_51ee20}.items():
            self.seam(address, fn)

    def write_gtids(self, h, gtids):
        head = 0
        for g in reversed(gtids):
            node = self.alloc(8); self.w(node, g); self.w(node + 4, head); head = node
        self.w(h, head)

    def unexpected(self, kind, slot):
        def fn(m):
            raise AssertionError('unexpected %s virtual +%x' % (kind, slot))
        return fn

    def name(self, p):
        return 0 if p == 0 else self.names[p]

    def ev(self, name, **kw):
        self.events.append({'event': name, **kw})

    def v_140(self, m):
        assert self.ecx() == self.mind
        self.ev('mind_virtual_140', gtid=self.arg(1)); self.ret(0, 4)

    def v_named(self, name):
        def fn(m):
            assert self.ecx() == self.mind
            self.ev(name); self.ret(0)
        return fn

    def v_schedule(self, m):
        assert self.ecx() == self.mind + 8
        self.ev('schedule', delay=self.arg(1), data=self.arg(2)); self.ret(0, 8)

    def v_can_activate(self, m):
        r = self.c['can_activate']
        self.ev('can_activate', state=self.state_names[self.ecx()], result=r); self.ret(int(r))

    def v_think(self, m):
        self.ev('embedded_think', state=self.state_names[self.ecx()]); self.ret(0)

    def v_state(self, name):
        def fn(m):
            self.ev(name, state=self.state_names[self.ecx()]); self.ret(0)
        return fn

    def o_158(self, m):
        k = self.names[self.ecx()]; r = self.c['objects'][k]['v158']
        self.ev('object_virtual_158', object=k, result=r); self.ret(int(r))

    def o_15c(self, m):
        k = self.names[self.ecx()]; t = self.c['objects'][k]['v15c']
        self.ev('object_virtual_15c', object=k, result=t); self.ret(0 if t is None else self.objects[t])

    def om_90(self, m):
        k = self.omind[self.ecx()]; t = self.c['objects'][k]['mind']
        self.ev('object_mind_virtual_90', object=k, result=t); self.ret(0 if t is None or t == '' else self.objects[t])

    def s_threats(self, m):
        k = self.components[self.ecx()]
        self.ev('component_threats', object=k); self.ret(self.comp_lists[self.ecx()])

    def s_585160(self, m):
        r = self.c['check_585160']
        self.ev('check_585160', value=self.ecx(), result=r); self.ret(int(r))

    def s_relation(self, m):
        assert self.ecx() == self.org and self.arg(2) == 3 and self.arg(3) == 0
        k = self.name(self.arg(1)); r = self.c['relation']
        self.ev('relation', object=k, result=r); self.ret(r & 0xffffffff, 12)

    def s_527ce0(self, m):
        assert self.ecx() == self.states['guard']
        self.ev('guard_527ce0', mode=self.arg(1), object=self.name(self.arg(2))); self.ret(0, 8)

    def s_follow(self, m):
        assert self.ecx() == self.states['follow']
        r = self.c['follow_ok']
        self.ev('follow_activate', mode=self.arg(1), object=self.name(self.arg(2)), result=r); self.ret(int(r), 8)

    def s_settle(self, m):
        if self.ecx() == 0:
            self.null_accesses.append((0x56aec0, 0, 4)); self.u.emu_stop(); return
        self.ev('home_settle', object=self.components[self.ecx()], word30=self.arg(1), word24=self.arg(2))
        self.ret(0, 8)

    def s_51ee20(self, m):
        assert self.ecx() == self.states['guard']
        r = self.c['check_51ee20']
        self.ev('guard_51ee20', result=r); self.ret(int(r))

    def run(self):
        from unicorn import UcError
        try:
            self.call(0x5294f0, ecx=self.states['guard'])
        except (AssertionError, UcError) as e:
            if self.null_accesses or isinstance(e, UcError):
                return {'fault': 'null dereference', 'events': self.events}
            raise
        g = self.states['guard']
        return {'events': self.events, 'status': {k: self.r(p + 0xc) for k, p in self.states.items()},
                'guard_word10': self.r(g + 0x10), 'guard_byte14': self.rb(g + 0x14)}


def cases():
    rnd = random.Random(0x5294f0)
    precisions = list(PRECISIONS)
    rows = []
    for i in range(500):
        n = rnd.randrange(1, 6)
        keys = ['o%d' % k for k in range(n)]
        objects = {}
        for k, key in enumerate(keys):
            objects[key] = {'gtid': (0x30 + k) << 16 | (0x300 + k), 'flags': 1 if rnd.random() < 0.12 else 0,
                            'registered': rnd.random() < 0.9,
                            'position': [f32(rnd.uniform(-40, 40)), f32(rnd.uniform(-40, 40))],
                            'mind': None, 'component': None, 'word98': rnd.choice([0, 0x777]),
                            'threats': [], 'v158': rnd.random() < 0.4, 'v15c': None}
        gt = lambda: rnd.choice([objects[k]['gtid'] for k in keys] + [0x7fff0005])
        for key in keys:
            o = objects[key]
            if rnd.random() < 0.7:
                o['mind'] = rnd.choice(keys + [''])
            if rnd.random() < 0.35:
                o['component'] = {'byte20': rnd.random() < 0.3, 'threats': [gt() for _ in range(rnd.randrange(0, 4))]}
            if rnd.random() < 0.5:
                o['threats'] = [gt() for _ in range(rnd.randrange(1, 4))]
            if rnd.random() < 0.6:
                o['v15c'] = rnd.choice(keys)
        guarded = rnd.choice([0, gt(), gt(), gt()])
        follow_target = rnd.choice([0, gt(), gt()])
        attack_target = rnd.choice([0, gt(), gt()])
        c = {'name': 'guard%d' % i, 'precision': precisions[i % 3], 'objects': objects, 'guarded': guarded,
             'words': [rnd.getrandbits(32), rnd.choice([0, gt(), gt()]), rnd.choice([0, 0, 0, gt()]),
                       rnd.choice([0, rnd.getrandbits(32)])],
             'status': {'sleep': rnd.choice([0, 1, 2]), 'move': rnd.choice([0, 0, 2]),
                        'follow': rnd.choice([0, 2, 2, 1]), 'attack': rnd.choice([0, 2, 1]),
                        'guard': rnd.choice([1, 2, 2])},
             'word10': {'sleep': 0, 'move': 0, 'follow': follow_target, 'attack': attack_target,
                        'guard': rnd.choice([0, 1, 4])},
             'byte14': rnd.random() < 0.5, 'stack': rnd.choice([['guard'], ['guard'], ['sleep', 'guard'], ['sleep']]),
             'org_position': [f32(rnd.uniform(-40, 40)), f32(rnd.uniform(-40, 40))],
             'bias': f32(rnd.choice([0, 2.5, 10, 30])), 'relation': rnd.choice([0, 1, 2]),
             'follow_ok': rnd.random() < 0.7, 'check_585160': rnd.random() < 0.5,
             'check_51ee20': rnd.random() < 0.4, 'can_activate': True}
        if i % 10 == 0:
            # Empty threat lists so the guarded Mind's +90 fallback decides.
            for o in objects.values():
                o['component'] = None; o['threats'] = []
        rows.append(c)
    return rows


def main():
    out, visited = [], {}
    for c in cases():
        m = GuardMachine(c)
        result = m.run()
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': result})
    d = header(__file__, __doc__)
    d.update(instructions=instructions(visited), cases=out)
    p = HERE / 'native-guard-think.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    body = [a for a in visited if 0x5294f0 <= a < 0x529ba5]
    print(json.dumps({'cases': len(out), 'faults': sum('fault' in r['output'] for r in out),
                      'instructions': len(visited), 'think_instructions': len(body),
                      'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
