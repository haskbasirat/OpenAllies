"""Original Mind reactions 51F9F0 (exhausted), 51F920 (wavering) with the
native activation 52A810/52A420/52A290 and 52A000; 5206A0 (CAI kill
OnEnter); 52E830 (UAI engage reaction delay, inline 85D840 draw on the world
RNG through 4F02B0); 527240 (regiment Mind target priority).

Seams (recorded): SAI record notification 7B6800 (record from actor+154),
4D1080, state virtuals +4/+18/+1C, Mind virtuals +1A0/+1A4/+4C, Updatable +4,
520320 (move to actor), 4D03F0, target Mind +58, SVar binding 4BE130/4BE150,
SVar lookup 4B2E60, diagnostics 404F90/77DA70, allocator 6F7810/6F7430.
"""
from pathlib import Path
import copy, json, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import Machine, PRECISIONS, f32, header, instructions, sha

STATES = {'engage': 0x1cc, 'rout': 0x20c, 'other': 0x34, 'move': 0x68, 'kill': 0xc0}


class ReactionMachine(Machine):
    def __init__(self, c):
        super().__init__(c.get('precision', 'Extended64'))
        self.c = copy.deepcopy(c)
        self.seam(0x6f7810, lambda m: self.ret(self.alloc(max(self.arg(1), 4))))
        self.seam(0x6f7430, lambda m: self.ret(0))
        self.seam(0x404f90, lambda m: (self.events.append({'event': 'diagnostic'}), self.ret(0)))
        self.seam(0x77da70, lambda m: self.ret(0, 12))
        world = self.world = self.alloc(0x200); self.w(0xb36534, world)
        self.w(world + 0xd8, c.get('session_mode', 0)); self.w(world + 0xd4, c.get('rng', 0))
        self.w(0xb3655c, self.alloc(0x40))
        reg = self.alloc(0x20004 + 0x10000 * 4 + 16); self.w(0xb364c0, reg); self.reg = reg
        actor = self.actor = self.alloc(0x200)
        if c.get('record') is not None:
            r = c['record']; rec = self.alloc(0x60)
            self.w(rec + 4, 1); self.w(rec + 0x10, r['word10']); self.w(rec + 0x3c, r['word3c'])
            if r['state30'] is not None:
                s30 = self.alloc(8); self.w(s30, r['state30']); self.w(rec + 0x30, s30)
            self.w(rec + 0xc, self.alloc(0x20))
            rvt = self.alloc(0x10); self.w(rec, rvt); self.w(rvt, self.stub(lambda m: self.ret(0, 4)))
            self.w(actor + 0x154, rec)
        mind = self.mind = self.alloc(0x300)
        mvt = self.alloc(0x200); self.w(mind, mvt); self.w(mind + 4, actor)
        self.w(mvt + 0x1a0, self.stub(lambda m: self.v_mind('default')))
        self.w(mvt + 0x1a4, self.stub(lambda m: self.v_mind('changed')))
        self.w(mvt + 0x4c, self.stub(self.v_current))
        uvt = self.alloc(0x10); self.w(uvt + 4, self.stub(self.v_schedule)); self.w(mind + 8, uvt)
        svt = self.alloc(0x40)
        self.w(svt + 4, self.stub(lambda m: self.v_state('can_activate', self.c.get('can_activate', True))))
        self.w(svt + 0x18, self.stub(lambda m: self.v_state('enter')))
        self.w(svt + 0x1c, self.stub(lambda m: self.v_state('leave')))
        self.names = {}
        for name, off in STATES.items():
            p = mind + off; meta = self.alloc(0x10); self.w(meta, self.alloc(4)); self.wb(meta + 8, c.get('discard', {}).get(name, 0))
            self.w(p, svt); self.w(p + 4, mind); self.w(p + 8, meta); self.w(p + 0xc, c.get('status', {}).get(name, 0))
            self.names[p] = name
        head = tail = 0
        for name in reversed(c.get('stack', [])):
            node = self.alloc(8); self.w(node, mind + STATES[name]); self.w(node + 4, head); head = node
            if not tail:
                tail = node
        self.w(mind + 0x10, head); self.w(mind + 0x14, tail)
        self.seam(0x7b6800, self.s_notify)
        self.seam(0x4d1080, self.s_blocked)
        self.seam(0x520320, self.s_move)
        self.seam(0x4d03f0, self.s_4d03f0)
        self.seam(0x4be130, lambda m: self.ret(0, 8))
        self.seam(0x401b00, self.s_cstring)
        self.seam(0x4be150, self.s_svar_bool)
        self.svar = self.alloc(4); self.w(self.svar, c.get('reaction_range', 0))
        self.seam(0x4b2e60, lambda m: self.ret(self.svar, 8))
        self.targets = {}
        tmvt = self.alloc(0x100); self.w(tmvt + 0x58, self.stub(self.v_target58))
        for key, t in c.get('targets', {}).items():
            p = self.alloc(0x100); self.targets[key] = p
            self.w(p + 0x14, t['gtid'])
            if t.get('mind'):
                tm = self.alloc(0x20); self.w(tm, tmvt); self.w(p + 0x70, tm); self.names[tm] = key + '.mind'
            d = self.alloc(0x300); self.w(p + 4, d); self.wb(d + 0x290, t.get('b290', 0)); self.wb(d + 0x291, t.get('b291', 0))
            self.w(p + 0x60, t.get('word60', 0))
            if t.get('registered', True):
                idx, gen = t['gtid'] & 0xffff, t['gtid'] >> 16
                self.u.mem_write(reg + 4 + 2 * idx, struct.pack('<H', gen)); self.w(reg + 0x20004 + 4 * idx, p)
            self.names[p] = key

    def snap(self):
        stack, node = [], self.r(self.mind + 0x10)
        while node:
            stack.append(self.names.get(self.r(node))); node = self.r(node + 4)
        return {'status': {n: self.r(self.mind + o + 0xc) for n, o in STATES.items()}, 'stack': stack}

    def event(self, name, extra=None):
        self.events.append({'event': name, **(extra or {}), **self.snap()})

    def v_mind(self, name):
        self.event(name); self.ret(0)

    def v_state(self, name, result=None):
        self.event(name, {'state': self.names[self.ecx()]})
        self.ret(int(result) if result is not None else 0)

    def v_schedule(self, m):
        assert self.ecx() == self.mind + 8
        self.event('schedule', {'delay': self.arg(1), 'data': self.arg(2)}); self.ret(0, 8)

    def v_current(self, m):
        cur = self.c.get('current_target')
        self.events.append({'event': 'current_target', 'result': cur})
        self.ret(0 if cur is None else self.targets[cur])

    def v_target58(self, m):
        key = self.names[self.ecx()][:-5]; r = self.c['targets'][key].get('mind58', False)
        self.events.append({'event': 'target_mind_58', 'target': key, 'result': r}); self.ret(int(r))

    def s_cstring(self, m):
        base = self.alloc(0x40); self.w(base + 0xc, 2)
        self.w(self.ecx(), base + 0x10); self.ret(self.ecx(), 4)

    def s_notify(self, m):
        self.event('notify_sai'); self.ret(0, 4)

    def s_blocked(self, m):
        assert self.ecx() == self.actor
        r = self.c.get('blocked', False); self.event('actor_blocked', {'result': r}); self.ret(int(r))

    def s_move(self, m):
        assert self.ecx() == self.mind + 0x68
        args = [self.arg(i) for i in range(1, 6)]
        t = self.names.get(args[1]) if args[1] else None
        r = self.c.get('move_result', True)
        self.events.append({'event': 'activate_move_to_actor', 'target': t, 'zero': args[3:5], 'result': r})
        assert args[0] == 0 and args[2] == 0
        self.ret(int(r), 0x14)

    def s_4d03f0(self, m):
        key = self.names[self.ecx()]; r = self.c['targets'][key].get('a', False)
        self.events.append({'event': 'target_4d03f0', 'target': key, 'result': r}); self.ret(int(r))

    def s_svar_bool(self, m):
        r = self.c.get('kill_target_is_highest', False)
        self.events.append({'event': 'kill_target_is_highest', 'result': r}); self.ret(int(r))

    def run(self):
        e, c = self.c['entry'], self.c
        if e == 'exhausted':
            r = self.call(0x51f9f0, ecx=self.mind) & 0xff
        elif e == 'wavering':
            r = self.call(0x51f920, ecx=self.mind) & 0xff
        elif e == 'kill_enter':
            self.w(self.mind + STATES['kill'] + 0x10, c['kill_target_gtid'])
            self.w(0xb279c0, c['zero'][0]); self.w(0xb279c4, c['zero'][1])
            self.call(0x5206a0, ecx=self.mind + STATES['kill']); r = None
        elif e == 'reaction':
            self.call(0x52e830, ecx=self.mind + STATES['other']); r = None
        elif e == 'priority':
            r = self.call(0x527240, ecx=self.mind, args=[self.targets[c['target']]])
        return {'result': r, 'rng': self.r(self.world + 0xd4), 'events': self.events, **self.snap()}


def cases():
    rnd = random.Random(0x51f9f0)
    rows = []
    for e in ['exhausted', 'wavering']:
        for i in range(60):
            rec = None if rnd.random() < 0.2 else {'word10': rnd.choice([1, 2]), 'word3c': rnd.choice([1, 2]),
                                                   'state30': rnd.choice([None, 0, 2, 3])}
            stack = rnd.choice([[], ['other'], ['engage'], ['rout'], ['other', 'rout'], ['engage', 'other'], ['rout', 'other']])
            rows.append({'name': f'{e}{i}', 'entry': e, 'session_mode': rnd.choice([0, 6, 2]), 'record': rec,
                         'stack': stack, 'blocked': rnd.random() < 0.2, 'can_activate': rnd.random() < 0.8,
                         'status': {n: rnd.choice([0, 1, 2]) for n in STATES},
                         'discard': {n: rnd.choice([0, 1]) for n in STATES}})
    for i in range(20):
        rows.append({'name': f'kill_enter{i}', 'entry': 'kill_enter', 'stack': ['kill'],
                     'targets': {'t0': {'gtid': 0x00030004, 'registered': rnd.random() < 0.8}},
                     'kill_target_gtid': rnd.choice([0x00030004, 0x00050006, 0]),
                     'zero': [rnd.choice([0, f32(1.5)]), rnd.choice([0, f32(-2)])], 'move_result': rnd.random() < 0.6,
                     'status': {'kill': rnd.choice([0, 1, 3])}})
    for p in PRECISIONS:
        for i, (seed, rng) in enumerate([(0x12345678, 0.3), (0, 0.3), (0xffffffff, 0.3), (1, 0.0), (7, 1e-6), (99, 2.5)] +
                                        [(rnd.getrandbits(32), rnd.uniform(0, 3)) for _ in range(10)]):
            rows.append({'name': f'reaction{i}_{p}', 'entry': 'reaction', 'precision': p, 'rng': seed,
                         'reaction_range': f32(rng), 'stack': ['other']})
    for i in range(60):
        targets = {'t0': {'gtid': 0x00030004, 'mind': rnd.random() < 0.5, 'mind58': rnd.random() < 0.5,
                          'a': rnd.random() < 0.5, 'word60': rnd.choice([0, 9]), 'b290': rnd.choice([0, 0, 1]),
                          'b291': rnd.choice([0, 0, 1])},
                   't1': {'gtid': 0x00040005}}
        rows.append({'name': f'priority{i}', 'entry': 'priority', 'targets': targets, 'target': 't0',
                     'current_target': rnd.choice(['t0', 't1', None]), 'kill_target_is_highest': rnd.random() < 0.5})
    return rows


def main():
    out, visited = [], {}
    for c in cases():
        try:
            m = ReactionMachine(c); result = m.run()
        except Exception:
            print(c['name']); raise
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': result})
    d = header(__file__, __doc__)
    d.update(instructions=instructions(visited), cases=out)
    p = HERE / 'native-cai-reactions.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'instructions': len(visited), 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
