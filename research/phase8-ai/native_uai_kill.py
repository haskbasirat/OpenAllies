"""Original unit kill decision 5340C0 and KKC_UAIStateKill::Think 535240.

5340C0 runs unchanged with 4C8300 (context init), 517F60 (enabled-ability
iterator), 52A1F0 (anchor arrival), 4F02B0 + inline 85D840 (coin flip on
the world RNG). Seams (recorded, scripted results; x87 results are loaded
from binary64 so unstored precision is exercised): 570650, 57A730, 4CF290,
570E30 (ROE rate), 5617B0, 4D4420, 4C8F20, 52ECC0, 578E50, 531FC0, 52ED30
(utility; records the candidate record), 4C83E0, 4CF110, 4C81C0, 4C8270,
Mind virtual +1AC.

535240 runs unchanged with 529E60 and 4DB0F0; seams: 4D4FF0, 5340C0 and
5307A0 (scripted record changes), actor virtual +108, target virtual +88,
performer 57DD90/57DEA0, Mind virtual +140, diagnostic 404F60.
"""
from pathlib import Path
import copy, json, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import Machine, PRECISIONS, f32, header, instructions, sha

f64b = lambda v: struct.unpack('<Q', struct.pack('<d', v))[0]


class KillMachine(Machine):
    def __init__(self, c):
        super().__init__(c['precision'])
        self.c = copy.deepcopy(c)
        self.seam(0x404f60, lambda m: (self.events.append({'event': 'diagnostic'}), self.ret(0)))
        self.seam(0x77da70, lambda m: self.ret(0, 12))
        world = self.world = self.alloc(0x200); self.w(0xb36534, world)
        self.w(world + 0xe0, c['time']); self.w(world + 0xd4, c['rng'])
        self.w(0xb3655c, self.alloc(0x40))
        svars = self.alloc(0x200); self.w(0xb39ab0, svars)
        for off, key in [(0x19c, 'retry_delay'), (0x1c4, 'attacker_bonus')]:
            s = self.alloc(4); self.w(s, c[key]); self.w(svars + off, s)
        reg = self.alloc(0x20004 + 0x10000 * 4 + 16); self.w(0xb364c0, reg)
        actor = self.actor = self.alloc(0x200)
        d = self.alloc(0x400); self.w(actor + 4, d); self.w(d + 0x2d0, c['ratio_threshold']); self.w(d + 0x2d4, c['value_threshold'])
        self.w(actor + 0x20, c['pos'][0]); self.w(actor + 0x24, c['pos'][1]); self.w(actor + 0x118, c['engaged'])
        self.w(actor + 0x14, 0x00070007)
        self.player = self.alloc(0x10)
        if c['body']:
            body = self.alloc(0x40); self.w(body + 0x10, c['health'][0]); self.w(body + 0x14, c['health'][1]); self.w(actor + 0x60, body)
            self.body = body
        if c['membership'] is not None:
            ms = self.alloc(0x40); self.w(actor + 0x80, ms); self.ms = ms
            if c['membership'].get('org', True):
                org = self.alloc(0x100); comp = self.comp = self.alloc(0x40); self.w(org + 0x7c, comp); self.w(ms + 0x10, org)
                x = self.alloc(0x40); self.w(comp + 0x18, x); self.w(x + 0x18, 0x5151)
        perf = self.perf = self.alloc(0x40)
        if c.get('performer', True):
            self.w(actor + 0x74, perf); self.w(perf + 4, actor)
            if c.get('performer_current', False):
                self.w(actor + 0xe4, perf)
            self.w(perf + 0x10, c.get('performer_stage', 4))
        self.abilities = []
        for a in c['abilities']:
            ab = self.alloc(0x40); df = self.alloc(0x80)
            self.w(ab + 4, df); self.wb(ab + 0x18, int(a['enabled'])); self.w(df + 0x54, a['flags54']); self.wb(df + 0x28, a['byte28'])
            self.abilities.append(ab)
        # Performer +14 is the inline ability vector {data, count}.
        self.w(perf + 0x14, self.array(self.abilities)); self.w(perf + 0x18, len(self.abilities))
        self.ab_index = {p: i for i, p in enumerate(self.abilities)}
        self.def_index = {self.r(p + 4): i for i, p in enumerate(self.abilities)}
        mind = self.mind = self.alloc(0x300)
        mvt = self.alloc(0x200); self.w(mind, mvt); self.w(mind + 4, actor); self.w(mind + 0x1ac, c['retry'])
        self.w(mvt + 0x1ac, self.stub(self.v_priority)); self.w(mvt + 0x140, self.stub(self.v_lost))
        tvt = self.alloc(0x200); self.w(tvt + 0x88, self.stub(self.v_in_range))
        avt = self.alloc(0x200); self.w(actor, avt); self.w(avt + 0x108, self.stub(self.v_position_ref))
        self.target = 0
        if c['target'] is not None:
            t = self.target = self.alloc(0x100); self.w(t, tvt); self.w(t + 0x20, c['target']['pos'][0]); self.w(t + 0x24, c['target']['pos'][1])
            self.w(t + 0x14, c['target']['gtid'])
            if c['target'].get('registered', True):
                g = c['target']['gtid']; self.u.mem_write(reg + 4 + 2 * (g & 0xffff), struct.pack('<H', g >> 16)); self.w(reg + 0x20004 + 4 * (g & 0xffff), t)
        self.counts = {}
        for addr, fn in [(0x570650, self.s_primary), (0x57a730, self.s_count), (0x4cf290, self.s_anchor), (0x570e30, self.s_rate),
                         (0x5617b0, self.s_value), (0x4d4420, self.s_attacks), (0x4c8f20, self.s_kind), (0x52ecc0, self.s_status),
                         (0x578e50, self.s_busy), (0x531fc0, self.s_can_attack), (0x52ed30, self.s_score), (0x4c83e0, self.s_point_score),
                         (0x4cf110, self.s_player), (0x4c81c0, self.s_range), (0x4c8270, self.s_value2),
                         (0x4d4ff0, self.s_relation), (0x5307a0, self.s_apply), (0x57dd90, self.s_approach), (0x57dea0, self.s_stop)]:
            self.seam(addr, fn)
        if c['entry'] == 'think':
            self.seam(0x5340c0, self.s_choose)

    def x87(self, value, pop):
        scratch = self.alloc(16); self.u.mem_write(scratch, struct.pack('<d', value))
        stub = self.alloc(16); self.u.mem_write(stub, b'\xdd\x05' + struct.pack('<I', scratch) + b'\xc2' + struct.pack('<H', pop))
        from unicorn.x86_const import UC_X86_REG_EIP
        self.u.reg_write(UC_X86_REG_EIP, stub)

    def ev(self, name, **k):
        self.events.append({'event': name, **k})

    def nth(self, key, default):
        i = self.counts.get(key, 0); self.counts[key] = i + 1
        seq = self.c['script'].get(key, default)
        return seq[i % len(seq)] if isinstance(seq, list) else seq

    def s_primary(self, m):
        r = self.c['primary']; self.ev('primary', result=r); self.ret(int(r))

    def s_count(self, m):
        r = self.c['member_count']; self.ev('member_count', result=r); self.ret(r & 0xffffffff)

    def s_anchor(self, m):
        out = self.arg(1); assert self.arg(2) == 0x5151
        self.w(out, self.c['anchor'][0]); self.w(out + 4, self.c['anchor'][1]); self.ev('anchor'); self.ret(out, 8)

    def s_rate(self, m):
        v = self.c['roe_rate']; self.ev('roe_rate', result=v); self.x87(v, 0)

    def s_value(self, m):
        v = self.c['roe_value']; self.ev('roe_value', rate=self.arg(1), result=v); self.x87(v, 4)

    def s_attacks(self, m):
        assert self.ecx() == self.target and self.arg(1) == self.actor
        r = self.c['attacks_us']; self.ev('target_attacks_us', result=r); self.ret(int(r), 4)

    def s_kind(self, m):
        i = self.ab_index[self.ecx()]; ctx = self.arg(1); assert self.arg(2) == 1
        context = {'kind': self.r(ctx), 'actor_ok': self.r(ctx + 8) == self.actor, 'target': int(self.r(ctx + 0x1c) == self.target and self.target != 0),
                   'point': [self.r(ctx + 0x20), self.r(ctx + 0x24)], 'word28': self.r(ctx + 0x28)}
        r = self.c['abilities'][i]['kind']; self.ev('ability_kind', ability=i, context=context, result=r); self.ret(r, 8)

    def s_status(self, m):
        k = self.arg(1); r = self.c['status_of_kind'][str(k)]; self.ev('decision_status', kind=k, result=r); self.ret(r, 4)

    def s_busy(self, m):
        r = self.c['org_busy']; self.ev('organization_busy', result=r); self.ret(int(r))

    def s_can_attack(self, m):
        i = self.def_index[self.arg(1)]; r = self.c['abilities'][i]['can_attack']
        self.ev('can_attack', ability=i, result=r); self.ret(int(r), 8)

    def rec(self, p):
        w = [self.r(p + 4 * k) for k in range(7)]
        w[0] = self.ab_index.get(w[0], w[0]); w[4] = int(w[4] == self.target and self.target != 0)
        return w

    def s_score(self, m):
        r = self.rec(self.arg(1)); i = r[0]
        v = self.c['abilities'][i]['score'][str(r[2])]
        self.ev('score', record=r, result=v); self.x87(v, 4)

    def s_point_score(self, m):
        i = self.ab_index[self.ecx()]; v = self.c['abilities'][i]['point_score']
        self.ev('point_score', ability=i, result=v); self.x87(v, 4)

    def s_player(self, m):
        self.ev('player'); self.ret(self.player)

    def s_range(self, m):
        i = self.ab_index[self.ecx()]; v = self.c['abilities'][i]['range']; self.ev('ability_range', ability=i, result=v); self.x87(v, 0)

    def s_value2(self, m):
        i = self.ab_index[self.ecx()]; assert self.arg(1) == self.player
        v = self.c['abilities'][i]['value']; self.ev('ability_value', ability=i, result=v); self.x87(v, 4)

    def v_priority(self, m):
        r = self.c['priority']; self.ev('priority', result=r); self.ret(r & 0xffffffff, 4)

    def s_relation(self, m):
        r = self.c['relation']; self.ev('relation', result=r); self.ret(r & 0xffffffff, 12)

    def s_choose(self, m):
        assert self.ecx() == self.mind and self.arg(2) == 0 and self.arg(4) == 0
        p = self.arg(3); before = self.rec(p)
        for k, v in enumerate(self.c['choose_record']):
            self.w(p + 4 * k, v)
        self.ev('choose', before=before); self.ret(self.c['choose_record'][2], 16)

    def s_apply(self, m):
        p = self.arg(1); before = self.rec(p); assert self.arg(2) == 0
        if self.c.get('apply_status') is not None:
            self.w(p + 8, self.c['apply_status'])
        self.ev('apply', before=before); self.ret(0, 8)

    def v_position_ref(self, m):
        self.ret(self.arg(1), 4)

    def v_in_range(self, m):
        r = self.c['in_range']; self.ev('in_range', result=r); self.ret(int(r), 4)

    def s_approach(self, m):
        assert self.arg(1) == 1 and self.arg(2) == self.target
        self.ev('approach'); self.ret(0, 12)

    def s_stop(self, m):
        self.ev('stop'); self.ret(0)

    def v_lost(self, m):
        self.ev('target_lost', gtid=self.arg(1)); self.ret(0, 4)

    def run(self):
        c = self.c
        from unicorn import UcError
        try:
            if c['entry'] == 'choose':
                out = 0
                if c['out']:
                    out = self.alloc(32)
                    for k, v in enumerate([0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77]):
                        self.w(out + 4 * k, v)
                pt = 0
                if c['point'] is not None:
                    pt = self.alloc(8); self.w(pt, c['point'][0]); self.w(pt + 4, c['point'][1])
                r = self.call(0x5340c0, ecx=self.mind, args=[self.target, pt, out, int(c['by_point'])])
                result = {'status': r, 'out': self.rec(out) if out else None}
            else:
                state = self.mind + 0xf8; self.w(state + 4, self.mind); self.w(state + 0xc, c['state_status'])
                meta = self.alloc(0x10); self.w(meta, self.alloc(4)); self.w(state + 8, meta)
                self.w(state + 0x10, c['target_gtid'])
                self.call(0x535240, ecx=state)
                result = {'state_status': self.r(state + 0xc)}
        except (AssertionError, UcError) as e:
            if not self.null_accesses and not isinstance(e, UcError):
                raise
            return {'fault': True, 'events': self.events}
        return {**result, 'retry': self.r(self.mind + 0x1ac), 'rng': self.r(self.world + 0xd4), 'events': self.events}


def ability(rnd):
    return {'enabled': rnd.random() < 0.85, 'flags54': rnd.choice([0, 1 << 3, (1 << 2) | (1 << 17), 1 << 6, (1 << 6) | (1 << 3), 0x2000c, 0x44]),
            'byte28': rnd.choice([0, 1]), 'kind': rnd.choice([1, 2, 5]), 'can_attack': rnd.random() < 0.5,
            'score': {s: rnd.choice([10.0, 10.0, 5.0, 12.5, 1.0 / 3.0, 10.0 + 2 ** -30, rnd.uniform(-5, 20)]) for s in ['0', '1', '2', '4']},
            'point_score': rnd.choice([3.0, 7.0, 1.0 / 7.0, rnd.uniform(0, 10)]),
            'range': rnd.choice([100.0, 100.0, 50.0, 100.0 + 2 ** -40]), 'value': rnd.choice([2.0, 2.0, 3.0, 2.0 + 2 ** -35])}


def cases():
    rnd = random.Random(0x5340c0)
    rows = []
    for i in range(220):
        abil = [ability(rnd) for _ in range(rnd.randrange(0, 6))]
        has_target = rnd.random() < 0.85
        c = {'name': f'choose{i}', 'entry': 'choose', 'precision': 'Single24',
             'time': f32(rnd.choice([100, 50])), 'retry': f32(rnd.choice([0, 100, 120, 50])), 'rng': rnd.getrandbits(32),
             'retry_delay': f32(rnd.choice([3, 7.5])), 'attacker_bonus': f32(rnd.choice([0.5, 0, 1.25])),
             'ratio_threshold': f32(rnd.choice([0.3, 0.6])), 'value_threshold': f32(rnd.choice([0.2, 5])),
             'pos': [f32(rnd.uniform(0, 100)), f32(rnd.uniform(0, 100))], 'engaged': rnd.choice([0, 1]),
             'body': rnd.random() < 0.9, 'health': [f32(rnd.choice([10, 50, 100])), f32(100)],
             'membership': rnd.choice([None, {'org': True}, {'org': True}, {'org': False}]),
             'primary': rnd.random() < 0.7, 'member_count': rnd.choice([1, 2, 5]),
             'anchor': [f32(rnd.uniform(0, 100)), f32(rnd.uniform(0, 100))],
             'roe_rate': rnd.choice([0.1, 1.0 / 3.0, 10.0]), 'roe_value': rnd.choice([0.1, 10.0, 0.2]),
             'attacks_us': rnd.random() < 0.4, 'org_busy': rnd.random() < 0.3, 'priority': rnd.randrange(0, 7),
             'status_of_kind': {'1': rnd.choice([0, 0, 1, 2]), '2': rnd.choice([0, 2, 3]), '5': rnd.choice([1, 3, 4])},
             'abilities': abil, 'script': {},
             'target': {'pos': [f32(rnd.uniform(0, 100)), f32(rnd.uniform(0, 100))], 'gtid': 0x00080008} if has_target else None,
             'point': [f32(rnd.uniform(0, 100)), f32(rnd.uniform(0, 100))] if (not has_target or rnd.random() < 0.4) else None,
             'by_point': rnd.random() < 0.3, 'out': rnd.random() < 0.9}
        if c['membership'] is None or not c['membership'].get('org', True):
            c['primary'] = c['primary'] and c['membership'] is not None
        rows.append(c)
    for i in range(60):
        base = copy.deepcopy(rows[i]); base.update(name=f'think{i}', entry='think', target={'pos': [f32(1), f32(2)], 'gtid': 0x00080008, 'registered': rnd.random() < 0.85},
                                                    target_gtid=rnd.choice([0x00080008, 0x00090009]), relation=rnd.choice([0, 0, 2]),
                                                    performer=rnd.random() < 0.95, performer_current=rnd.random() < 0.5,
                                                    performer_stage=rnd.choice([4, 4, 1]), state_status=rnd.choice([0, 2]),
                                                    choose_record=[0, 1, rnd.choice([0, 1, 2, 3]), f32(1), 0, f32(5), 0],
                                                    apply_status=rnd.choice([None, 0, 1]), in_range=rnd.random() < 0.5)
        rows.append(base)
    out = []
    for p in PRECISIONS:
        for c in rows:
            out.append({**copy.deepcopy(c), 'name': c['name'] + '_' + p, 'precision': p})
    return out


def main():
    out, visited = [], {}
    for c in cases():
        try:
            m = KillMachine(c); result = m.run()
        except Exception:
            print(c['name']); raise
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': result})
    d = header(__file__, __doc__)
    d.update(instructions=instructions(visited), cases=out)
    p = HERE / 'native-uai-kill.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'faults': sum('fault' in r['output'] for r in out),
                      'instructions': len(visited), 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
