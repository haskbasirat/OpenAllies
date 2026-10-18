"""Original KKC_CAIStateEngage::Think 526A70 executing unchanged on the emulated
regiment Mind of native_cai_engage.py (engage state +1CC with its sorted
target list, chasing state +120), with 529E60, 529C30, the member iterator
4364C0 and the target add 524C10 (and its callees as seamed there).

Additional seams (recorded in order): Mind virtuals +4C +140 +F8, Updatable
+8 virtual +4 (schedule), organization virtual +164, member Mind virtuals +4C
+6C +F0 +17C, member +E4 object virtual +18, 578140, 51F920, 578470, 520D90,
the one-time diagnostic logger 404ED0 (not recorded). The SVar
KillTargetIsHighestPriority cache (B3CF6C, guard bit B3CF70) is
pre-initialised to a case-scripted byte, as after its first use.
"""
from pathlib import Path
import json, random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import header, instructions, sha
from native_cai_engage import EngageMachine
import native_cai_engage


class EngageThinkMachine(EngageMachine):
    def __init__(self, c):
        super().__init__(c)
        self.seam(0x404ed0, lambda m: self.ret(0))
        mvt = self.r(self.mind)
        for slot, fn in {0x4c: self.v_attacker, 0x140: self.v_140, 0xf8: self.v_f8}.items():
            self.w(mvt + slot, self.stub(fn))
        uvt = self.alloc(0x10); self.w(uvt + 4, self.stub(self.v_schedule)); self.w(self.mind + 8, uvt)
        self.w(self.engage + 0xc, c['engage_status'])
        meta = self.alloc(0x10); self.w(meta, self.alloc(4)); self.w(self.engage + 8, meta)
        defn = self.alloc(0x10); self.w(defn + 8, self.alloc(4)); self.w(self.org + 4, defn)
        self.w(self.engage + 0x20, c['last_attacker']); self.wb(self.engage + 0x1c, c['byte1c'])
        ovt = self.alloc(0x200); self.w(ovt + 0x164, self.stub(self.v_follows)); self.w(self.org, ovt)
        self.w(self.org + 0x118, c['word118'])
        svar = self.alloc(4); self.wb(svar, int(c['svar'])); self.w(0xb3cf6c, svar); self.w(0xb3cf70, 1)
        if self.comp == 0:
            raise AssertionError('engage think cases need a component')
        mmvt = self.alloc(0x200)
        for slot, fn in {0x17c: self.mm_17c, 0x6c: self.mm_6c, 0xf0: self.mm_f0, 0x4c: self.mm_4c}.items():
            self.w(mmvt + slot, self.stub(fn))
        evt = self.alloc(0x40); self.w(evt + 0x18, self.stub(self.e_status))
        slots = []
        self.members, self.member_minds, self.e4 = {}, {}, {}
        for i, spec in enumerate(c['members']):
            if spec is None:
                slots.append(0); continue
            p = self.alloc(0x200); key = 'm%d' % i
            self.members[p] = key
            if spec['mind']:
                mm = self.alloc(0x40); self.w(mm, mmvt); self.w(p + 0x70, mm); self.member_minds[mm] = key
            self.w(p + 0x100, spec['flags100'])
            if spec['e4'] is not None:
                e = self.alloc(0x40); self.w(e, evt); self.w(p + 0xe4, e); self.e4[e] = key
            slots.append(p)
        self.w(self.comp + 0x28, self.array(slots)); self.w(self.comp + 0x2c, len(slots))
        self.seam(0x578140, self.s_status)
        self.seam(0x51f920, self.s_wavering)
        self.seam(0x578470, self.s_range)
        self.seam(0x520d90, self.s_chase)

    def ev(self, name, **kw):
        self.events.append({'event': name, **kw})

    def v_attacker(self, m):
        assert self.ecx() == self.mind
        a = self.c['attacker']
        self.ev('mind_virtual_4c', result=a); self.ret(0 if a is None else self.actors[a])

    def v_140(self, m):
        assert self.ecx() == self.mind
        self.ev('mind_virtual_140', gtid=self.arg(1)); self.ret(0, 4)

    def v_f8(self, m):
        assert self.ecx() == self.mind
        r = self.c['f8']
        self.ev('mind_virtual_f8', actor=self.name(self.arg(1)) if self.arg(1) else 0, result=r); self.ret(int(r), 4)

    def v_schedule(self, m):
        assert self.ecx() == self.mind + 8
        self.ev('schedule', delay=self.arg(1), data=self.arg(2)); self.ret(0, 8)

    def v_follows(self, m):
        assert self.ecx() == self.org
        t = self.name(self.arg(1)); r = self.c['follows'].get(t, False)
        self.ev('organization_follows', target=t, result=r); self.ret(int(r), 4)

    def member_spec(self, key):
        return self.c['members'][int(key[1:])]

    def mm_17c(self, m):
        self.ev('member_mind_virtual_17c', member=self.member_minds[self.ecx()]); self.ret(0)

    def mm_6c(self, m):
        k = self.member_minds[self.ecx()]; r = self.member_spec(k)['busy']
        self.ev('member_mind_virtual_6c', member=k, result=r); self.ret(int(r))

    def mm_f0(self, m):
        k = self.member_minds[self.ecx()]
        self.ev('member_mind_virtual_f0', member=k, target=self.name(self.arg(1))); self.ret(0, 4)

    def mm_4c(self, m):
        k = self.member_minds[self.ecx()]; t = self.member_spec(k)['target']
        self.ev('member_mind_virtual_4c', member=k, result=t); self.ret(0 if t is None else self.actors[t])

    def e_status(self, m):
        k = self.e4[self.ecx()]; r = self.member_spec(k)['e4']
        self.ev('member_e4_status', member=k, result=r); self.ret(r)

    def s_status(self, m):
        assert self.ecx() == self.comp
        self.ev('organization_status', result=self.c['status']); self.ret(self.c['status'])

    def s_wavering(self, m):
        assert self.ecx() == self.mind
        self.ev('wavering_reaction'); self.ret(0)

    def s_range(self, m):
        assert self.ecx() == self.comp
        t = self.name(self.arg(1)); r = self.c['in_range']
        self.ev('within_engage_range', target=t, result=r); self.ret(int(r), 4)

    def s_chase(self, m):
        assert self.ecx() == self.chasing
        r = self.c['chase']
        self.ev('chase_continue', gtid=self.arg(1), result=r); self.ret(int(r), 4)

    def run(self):
        from unicorn import UcError
        try:
            self.call(0x526a70, ecx=self.engage)
        except (AssertionError, UcError) as e:
            if self.null_accesses or isinstance(e, UcError):
                return {'fault': 'null dereference', 'events': self.events}
            raise
        return {'events': self.events, 'list': self.read_list(), 'status': self.r(self.engage + 0xc),
                'last_attacker': self.r(self.engage + 0x20), 'byte1c': self.rb(self.engage + 0x1c)}


def cases():
    rnd = random.Random(0x526a70)
    rows = []
    for i, c in enumerate(native_cai_engage.cases()):
        c = dict(c)
        c['name'] = 'engage_think%d' % i
        c['entry'] = 'engage_think'
        c['component'] = True
        keys = list(c['actors'])
        c['current'] = rnd.choice(['engage', 'engage', 'chasing', 'other', None])
        c['engage_status'] = rnd.choice([1, 2, 2])
        c['status'] = rnd.choice([0, 1, 1, 2])
        c['word118'] = rnd.choice([0, 0, 5])
        c['attacker'] = rnd.choice(keys + [None, None])
        c['last_attacker'] = rnd.choice([0, 0x7fff0042] + [c['actors'][k]['gtid'] for k in keys])
        c['byte1c'] = rnd.choice([0, 1])
        c['svar'] = rnd.random() < 0.6
        c['in_range'] = rnd.random() < 0.6
        c['chase'] = rnd.random() < 0.4
        c['f8'] = rnd.random() < 0.4
        c['follows'] = {k: rnd.random() < 0.7 for k in keys}
        members = []
        for k in range(rnd.choice([0, 1, 2, 3, 5])):
            if rnd.random() < 0.15:
                members.append(None); continue
            members.append({'mind': rnd.random() < 0.96, 'flags100': rnd.choice([0, 0, 0x200, 0x10]),
                            'busy': rnd.random() < 0.3, 'e4': rnd.choice([None, 2, 2, 1]),
                            'target': rnd.choice(keys + [None])})
        c['members'] = members
        rows.append(c)
    return rows


def main():
    out, visited = [], {}
    for c in cases():
        m = EngageThinkMachine(c)
        result = m.run()
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': result})
    d = header(__file__, __doc__)
    d.update(instructions=instructions(visited), cases=out)
    p = HERE / 'native-engage-think.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    body = [a for a in visited if 0x526a70 <= a < 0x526e58]
    print(json.dumps({'cases': len(out), 'faults': sum('fault' in r['output'] for r in out),
                      'instructions': len(visited), 'think_instructions': len(body),
                      'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
