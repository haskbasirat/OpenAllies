"""Original KKC_CAIStateSleep::Think 524D70 executing unchanged on an emulated
regiment Mind (sleep state at +34, flag-310 state at +54, follow state at
+C0) together with 529E60, 52A000, 51FFE0, 529C30 and the Mind activation
52A810 (mode 2: leave, stack clear, push, schedule, enter, state changed).

Seams (recorded in order): Mind virtuals +F4 +188 +190 +1A0 +1A4, the
Updatable +8 virtual +4 (schedule), state virtuals +4 (can activate) +C
(think) +18 (enter) +1C (leave), organization virtual +164, 578140, 51F9F0,
51D920, 4D7960, 56AEC0, 522890, 57BF30, 520120, 52BF30, 522F60, 523000,
allocator 6F7810/6F7430 and the diagnostic loggers 404ED0/404F90 (not
recorded). Objects are resolved through an emulated GTID registry [B364C0].
"""
from pathlib import Path
import json, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import Machine, f32, header, instructions, sha

STATE_OFFSETS = {'sleep': 0x34, 's54': 0x54, 'sC0': 0xc0}


class SleepMachine(Machine):
    def __init__(self, c):
        super().__init__('Double53')
        self.c = c
        self.seam(0x6f7430, lambda m: self.ret(0))
        self.seam(0x6f7810, lambda m: self.ret(self.alloc(max(self.arg(1), 4))))
        self.seam(0x404ed0, lambda m: self.ret(0))
        self.seam(0x404f90, lambda m: self.ret(0))
        reg = self.alloc(0x20004 + 0x10000 * 4 + 16); self.w(0xb364c0, reg); self.reg = reg
        self.w(0xb279c0, c['null_point'][0]); self.w(0xb279c4, c['null_point'][1])
        mvt = self.alloc(0x200)
        for slot in range(0, 0x200, 4):
            self.w(mvt + slot, self.stub(self.unexpected('mind', slot)))
        for slot, name in {0xf4: 'mind_virtual_f4', 0x188: 'mind_virtual_188', 0x190: 'mind_virtual_190',
                           0x1a0: 'reset_default', 0x1a4: 'state_changed'}.items():
            self.w(mvt + slot, self.stub(self.v_mind(name)))
        uvt = self.alloc(0x10); self.w(uvt + 4, self.stub(self.v_schedule))
        svt = self.alloc(0x40)
        for slot in range(0, 0x40, 4):
            self.w(svt + slot, self.stub(self.unexpected('state', slot)))
        self.w(svt + 4, self.stub(self.v_can_activate)); self.w(svt + 0xc, self.stub(self.v_think))
        self.w(svt + 0x18, self.stub(self.v_state('enter'))); self.w(svt + 0x1c, self.stub(self.v_state('leave')))
        mind = self.mind = self.alloc(0x400)
        self.w(mind, mvt); self.w(mind + 8, uvt)
        org = self.org = self.alloc(0x200)
        ovt = self.alloc(0x200)
        for slot in range(0, 0x200, 4):
            self.w(ovt + slot, self.stub(self.unexpected('organization', slot)))
        self.w(ovt + 0x164, self.stub(self.v_follows)); self.w(org, ovt)
        odata = self.alloc(0x400); self.w(org + 4, odata); self.w(odata + 8, self.alloc(4)); self.wb(odata + 0x310, int(c['flag310']))
        self.comp = 0
        if c['component'] is not None:
            self.comp = self.alloc(0x100); self.w(org + 0x7c, self.comp); self.w(self.comp + 0x74, c['component'])
        self.w(mind + 4, org)
        self.states = {}
        for key, off in STATE_OFFSETS.items():
            p = mind + off
            meta = self.alloc(0x10); self.w(p, svt); self.w(p + 4, mind); self.w(p + 8, meta)
            self.wb(meta + 8, 0)
            self.states[key] = p
        other = self.alloc(0x40); meta = self.alloc(0x10); self.w(other, svt); self.w(other + 4, mind); self.w(other + 8, meta)
        self.states['other'] = other
        for key, st in c['states'].items():
            self.w(self.states[key] + 0xc, st)
        sleep = self.states['sleep']
        self.wb(sleep + 0x10, int(c['byte10'])); self.w(sleep + 0x1c, c['word1c'])
        head = tail = 0
        for key in reversed(c['stack']):
            node = self.alloc(8); self.w(node, self.states[key]); self.w(node + 4, head); head = node
            if not tail:
                tail = node
        self.w(mind + 0x10, head); self.w(mind + 0x14, tail)
        for i, word in enumerate(c['words']):
            self.w(mind + 0x24 + 4 * i, word)
        self.w(mind + 0x2b4, c['point'][0]); self.w(mind + 0x2b8, c['point'][1])
        head = tail = 0
        for g in reversed(c['arrivals']):
            node = self.alloc(8); self.w(node, g); self.w(node + 4, head); head = node
            if not tail:
                tail = node
        self.w(mind + 0x2bc, head); self.w(mind + 0x2c0, tail)
        self.objects = {}
        for key, o in c['objects'].items():
            p = self.alloc(0x100)
            self.w(p + 8, o['flags']); self.w(p + 0x14, o['gtid'])
            defn = self.alloc(0x10); self.w(defn + 8, self.alloc(4)); self.w(p + 4, defn)
            if o['member'] is not None:
                ms = self.alloc(0x20); self.w(p + 0x80, ms)
                self.w(ms + 0x10, org if o['member'] == 'org' else 0x5150000)
            self.w(p + 0x88, o['home_component'])
            self.objects[key] = p
            if o['registered']:
                idx, gen = o['gtid'] & 0xffff, o['gtid'] >> 16
                self.u.mem_write(reg + 4 + 2 * idx, struct.pack('<H', gen)); self.w(reg + 0x20004 + 4 * idx, p)
        self.names = {p: k for k, p in self.objects.items()}
        self.state_names = {p: k for k, p in self.states.items()}
        for address, name in {0x578140: 'organization_status', 0x51f9f0: 'exhausted_reaction',
                              0x51d920: 'follow_activate', 0x4d7960: 'organization_4d7960',
                              0x56aec0: 'home_settle', 0x522890: 'member_arrived', 0x57bf30: 'organization_move',
                              0x520120: 'organization_busy', 0x52bf30: 'resume_pending',
                              0x522f60: 'sleep_522f60', 0x523000: 'sleep_523000'}.items():
            self.seam(address, self.s_service(name))

    def unexpected(self, kind, slot):
        def fn(m):
            raise AssertionError('unexpected %s virtual +%x' % (kind, slot))
        return fn

    def name(self, p):
        return self.names.get(p, 0 if p == 0 else hex(p))

    def ev(self, name, **kw):
        self.events.append({'event': name, **kw})

    def v_mind(self, name):
        def fn(m):
            assert self.ecx() == self.mind
            if name == 'mind_virtual_188':
                self.ev(name, object=self.name(self.arg(1))); self.ret(0, 4)
            elif name == 'mind_virtual_f4':
                assert self.arg(1) == self.org
                self.ev(name); self.ret(0, 4)
            else:
                self.ev(name); self.ret(0)
        return fn

    def v_schedule(self, m):
        assert self.ecx() == self.mind + 8
        self.ev('schedule', delay=self.arg(1), data=self.arg(2)); self.ret(0, 8)

    def v_can_activate(self, m):
        s = self.state_names[self.ecx()]; r = self.c['can_activate']
        self.ev('can_activate', state=s, result=r); self.ret(int(r))

    def v_think(self, m):
        s = self.state_names[self.ecx()]
        self.ev('embedded_think', state=s); self.ret(0)

    def v_state(self, name):
        def fn(m):
            self.ev(name, state=self.state_names[self.ecx()]); self.ret(0)
        return fn

    def v_follows(self, m):
        assert self.ecx() == self.org
        o = self.name(self.arg(1)); r = self.c['follows']
        self.ev('organization_follows', object=o, result=r); self.ret(int(r), 4)

    def s_service(self, name):
        def fn(m):
            c = self.c
            if name == 'organization_status':
                assert self.ecx() == self.comp
                if self.comp == 0:
                    self.null_accesses.append((0x578140, 0, 4)); self.u.emu_stop(); return
                self.ev(name, result=c['status']); self.ret(c['status'])
            elif name == 'exhausted_reaction':
                assert self.ecx() == self.mind
                self.ev(name, result=c['exhausted']); self.ret(int(c['exhausted']))
            elif name == 'follow_activate':
                assert self.ecx() == self.states['sC0'] and self.arg(1) == 2
                self.ev(name, target=self.name(self.arg(2))); self.ret(1, 8)
            elif name == 'organization_4d7960':
                assert self.ecx() == self.org
                self.ev(name); self.ret(0)
            elif name == 'home_settle':
                self.ev(name, component=self.ecx(), word30=self.arg(1), word24=self.arg(2)); self.ret(0, 8)
            elif name == 'member_arrived':
                assert self.ecx() == self.mind and self.arg(2) == 0
                self.ev(name, object=self.name(self.arg(1))); self.ret(0, 8)
            elif name == 'organization_move':
                assert self.ecx() == self.comp
                if self.comp == 0:
                    self.null_accesses.append((0x57bf30, 0, 4)); self.u.emu_stop(); return
                self.ev(name, point=[self.arg(1), self.arg(2)]); self.ret(0, 8)
            elif name == 'organization_busy':
                assert self.ecx() == self.states['sleep']
                self.ev(name, result=c['busy']); self.ret(int(c['busy']))
            elif name == 'resume_pending':
                assert self.ecx() == self.mind
                self.ev(name, result=c['resumed']); self.ret(int(c['resumed']))
            else:
                assert self.ecx() == self.states['sleep']
                self.ev(name); self.ret(0)
        return fn

    def snapshot(self):
        stack, node = [], self.r(self.mind + 0x10)
        while node:
            stack.append(self.state_names.get(self.r(node))); node = self.r(node + 4)
        arrivals, node = [], self.r(self.mind + 0x2bc)
        while node:
            arrivals.append(self.r(node)); node = self.r(node + 4)
        return {'states': {k: self.r(p + 0xc) for k, p in self.states.items()}, 'stack': stack,
                'arrivals': arrivals, 'point': [self.r(self.mind + 0x2b4), self.r(self.mind + 0x2b8)],
                'word1c': self.r(self.states['sleep'] + 0x1c)}

    def run(self):
        from unicorn import UcError
        try:
            self.call(0x524d70, ecx=self.states['sleep'])
        except (AssertionError, UcError) as e:
            if self.null_accesses or isinstance(e, UcError):
                return {'fault': 'null dereference', 'events': self.events}
            raise
        return {'events': self.events, **self.snapshot()}


def cases():
    rnd = random.Random(0x524d70)
    rows = []
    for i in range(400):
        objects = {}
        for k in range(rnd.randrange(0, 5)):
            objects['o%d' % k] = {'gtid': (0x20 + k) << 16 | (0x200 + k), 'flags': 1 if rnd.random() < 0.15 else 0,
                                  'registered': rnd.random() < 0.85,
                                  'member': rnd.choice(['org', 'org', 'other', None]) if rnd.random() < 0.95 else None,
                                  'home_component': rnd.choice([0, 0x6660000 + k]) if rnd.random() < 0.9 else 0}
        keys = list(objects)
        gt = lambda: rnd.choice([objects[k]['gtid'] for k in keys] + [0x7fff0001]) if keys else 0x7fff0001
        follow = gt() if rnd.random() < 0.3 else 0
        home = gt() if rnd.random() < 0.35 else 0
        words = [rnd.getrandbits(32), home, follow, rnd.choice([0, 0, rnd.getrandbits(32)])]
        null_point = rnd.choice([[0, 0], [f32(3.5), f32(-2)], [0x7fc00000, 0], [0, 0x7fc00001]])
        point = rnd.choice([null_point, null_point, [f32(rnd.uniform(-50, 50)), f32(rnd.uniform(-50, 50))],
                            [null_point[0], f32(1)], [0x80000000, null_point[1]], [0x7fc00000, 0]])
        stack = rnd.choice([['sleep'], ['sleep'], ['sleep', 'other'], ['other', 'sleep'], ['other'], []])
        c = {'name': 'sleep%d' % i, 'objects': objects, 'words': words, 'stack': stack,
             'states': {'sleep': rnd.choice([1, 2, 2]), 's54': rnd.choice([0, 1]), 'sC0': rnd.choice([0, 2]),
                        'other': rnd.choice([1, 2])},
             'arrivals': [gt() for _ in range(rnd.choice([0, 0, 1, 3]))],
             'null_point': null_point, 'point': point,
             'component': rnd.choice([rnd.getrandbits(32), 7, 7]) if rnd.random() < 0.97 else None,
             'word1c': rnd.choice([7, 7, rnd.getrandbits(32)]), 'byte10': rnd.random() < 0.5,
             'status': rnd.choice([0, 0, 1, 2]), 'exhausted': rnd.random() < 0.4, 'follows': rnd.random() < 0.4,
             'busy': rnd.random() < 0.4, 'resumed': rnd.random() < 0.3, 'flag310': rnd.random() < 0.4,
             'can_activate': rnd.random() < 0.8}
        rows.append(c)
    return rows


def main():
    out, visited = [], {}
    for c in cases():
        m = SleepMachine(c)
        result = m.run()
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': result})
    d = header(__file__, __doc__)
    d.update(instructions=instructions(visited), cases=out)
    p = HERE / 'native-sleep-think.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    body = [a for a in visited if 0x524d70 <= a < 0x524d70 + 899]
    print(json.dumps({'cases': len(out), 'faults': sum('fault' in r['output'] for r in out),
                      'instructions': len(visited), 'think_instructions': len(body),
                      'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
