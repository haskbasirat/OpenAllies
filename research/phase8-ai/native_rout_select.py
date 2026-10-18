"""Original CAI rout destination selection 52B530, with 52A930 executing natively.

52B530 runs unchanged with the CAI Mind as `this` (Mind+4 = organization),
including its lazy SVar cache, the global direction table [B39AF8], the
original sin/cos lookups 85DAA0/85DA80 over table objects [B39B10]/[B39B08]
built from the recovered 4096-entry tables (research/phase4-trig/
lookup-tables.bin, period 2*pi, wrap flag set), the inline 85D840 draws, the
sparse Kingdom iterator and every 52A930 score call (whose own seams are those
of native_rout_score.py).

Additional seams: organization virtual +4 (x87 result discarded by 52B530),
52A690 reachable-delta query (returns case values in call order, recorded
with its arguments) and World 4F46E0 open-position search (returns case
results, optionally moving the candidate, recorded before/after).
"""
from pathlib import Path
import copy, json, math, random, struct, sys
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(HERE))
from native_machine import PRECISIONS, f32, header, instructions, sha
from native_rout_score import RoutMachine, cases as score_cases, OWNER

TABLES = (ROOT / 'research/phase4-trig/lookup-tables.bin').read_bytes()
assert len(TABLES) == 32768


class SelectMachine(RoutMachine):
    def __init__(self, c):
        super().__init__(c)
        for name, bits in c['select_svars'].items():
            slot = self.alloc(4); self.w(slot, bits); self.svars[name] = slot
        org = self.org
        vt = self.r(org)
        self.w(vt + 4, self.stub(self.v_org4))
        comp = self.alloc(0x100); self.w(org + 0x7c, comp); self.w(comp + 0xac, c['min_distance'])
        table = self.alloc(0x20)
        pairs = self.alloc(8 * max(1, len(c['directions'])))
        for i, (x, y) in enumerate(c['directions']):
            self.w(pairs + 8 * i, x); self.w(pairs + 8 * i + 4, y)
        self.w(table + 4, len(c['directions'])); self.w(table + 8, pairs)
        self.w(0xb39af8, table)
        for glob, offset in [(0xb39b08, 0), (0xb39b10, 16384)]:
            obj = self.alloc(0x20); data = self.alloc(16384)
            self.u.mem_write(data, TABLES[offset:offset + 16384])
            self.w(obj + 4, 4096); self.w(obj + 8, data); self.w(obj + 0xc, 0x40c90fdb); self.wb(obj + 0x10, 1)
            self.w(glob, obj)
        self.seam(0x52a690, self.s_reachable)
        self.seam(0x4f46e0, self.s_open)
        self.reach_calls = 0
        self.open_calls = 0
        self.out = self.alloc(8); self.w(self.out, 0xdeadbeef); self.w(self.out + 4, 0xfeedface)

    def s_svar(self, m):
        assert self.ecx() == 0xb39aa8 and self.arg(2) in (2, 3)
        p = self.arg(1); name = ''
        while True:
            ch = self.u.mem_read(p + 2 * len(name), 2)
            if ch == b'\0\0':
                break
            name += chr(ch[0] | ch[1] << 8)
        self.events.append({'event': 'svar', 'name': name})
        self.ret(self.svars[name], 8)

    def v_org4(self, m):
        assert self.ecx() == self.org
        self.events.append({'event': 'organization_virtual_4'})
        self.ret_st0_f32(f32(123.5), 0)

    def s_reachable(self, m):
        assert self.ecx() == self.state
        args = [self.arg(i) for i in range(1, 7)]
        value = self.c['reachable'][self.reach_calls % len(self.c['reachable'])]
        self.reach_calls += 1
        self.events.append({'event': 'reachable', 'origin': args[0:2], 'direction': args[2:4],
                            'base': args[4], 'delta': args[5], 'result': value})
        self.ret_st0_f32(value, 0x18)

    def s_open(self, m):
        assert self.ecx() == self.r(0xb36534) and self.arg(1) == self.org and self.arg(3) == 0x40800000
        p = self.arg(2)
        before = [self.r(p), self.r(p + 4)]
        rule = self.c['open'][self.open_calls % len(self.c['open'])]
        self.open_calls += 1
        if rule.get('move') is not None:
            self.w(p, rule['move'][0]); self.w(p + 4, rule['move'][1])
        self.events.append({'event': 'open', 'before': before, 'after': [self.r(p), self.r(p + 4)], 'result': rule['result']})
        self.ret(int(rule['result']), 0xc)

    def run(self):
        result = self.call(0x52b530, ecx=self.state, args=[self.rng, self.out], fpsw=0x20) & 0xff
        return {'selected': result, 'out': [self.r(self.out), self.r(self.out + 4)],
                'rng_state': self.r(self.rng), 'events': self.events}


def unit_directions(n, phase=0.0):
    return [[f32(math.cos(phase + 2 * math.pi * k / n)), f32(math.sin(phase + 2 * math.pi * k / n))] for k in range(n)]


def cases():
    base = next(c for c in score_cases() if c['name'] == 'defaults_rout_Single24')
    base = copy.deepcopy(base)
    base.update(name='defaults', precision='Single24',
                select_svars={'RoutRandomCandidates': 20, 'RoutMaxDistanceDelta': f32(24),
                              'RoutSupplyZoneCandidates': 2, 'RoutSupplyZoneMaxAngleDelta': f32(50)},
                min_distance=f32(60), directions=unit_directions(36),
                reachable=[f32(24), f32(10.5), f32(0), f32(24)],
                open=[{'result': True}, {'result': True, 'move': [f32(1180.25), f32(940.5)]}, {'result': False}])
    rows = []

    def add(name, change):
        c = copy.deepcopy(base); c['name'] = name; change(c); rows.append(c)

    add('defaults', lambda c: None)
    add('no_random_candidates', lambda c: c['select_svars'].update(RoutRandomCandidates=0))
    add('negative_random_candidates', lambda c: c['select_svars'].update(RoutRandomCandidates=0xffffffff))
    add('one_random_candidate', lambda c: c['select_svars'].update(RoutRandomCandidates=1))
    add('no_supply_candidates', lambda c: c['select_svars'].update(RoutSupplyZoneCandidates=0))
    add('five_supply_candidates', lambda c: c['select_svars'].update(RoutSupplyZoneCandidates=5))
    add('never_open', lambda c: c.update(open=[{'result': False}]))
    add('always_open', lambda c: c.update(open=[{'result': True}]))
    add('zero_distance_skip', lambda c: c.update(min_distance=0, reachable=[0]))
    add('zero_distance_then_moved', lambda c: c.update(min_distance=0, reachable=[0, f32(3)]))
    add('supply_at_org', lambda c: c.update(kingdoms=[{'allied': True, 'supplies': [{'position': c['org'], 'radius': f32(100)}]}]))
    add('supply_tiny_offset', lambda c: c.update(kingdoms=[{'allied': True, 'supplies': [{'position': [c['org'][0], f32(1000.00005)], 'radius': f32(100)}]}]))
    add('supply_without_component', lambda c: c.update(kingdoms=[{'allied': True, 'supplies': [{'position': [f32(1300), f32(900)], 'radius': None}]}]))
    add('no_kingdoms', lambda c: c.update(kingdoms=[]))
    add('not_allied', lambda c: c.update(kingdoms=[{'allied': False, 'supplies': [{'position': [f32(1300), f32(900)], 'radius': f32(10)}]}]))
    add('angle_delta_zero', lambda c: c['select_svars'].update(RoutSupplyZoneMaxAngleDelta=0))
    add('angle_delta_large', lambda c: c['select_svars'].update(RoutSupplyZoneMaxAngleDelta=f32(400)))
    add('angle_delta_negative', lambda c: c['select_svars'].update(RoutSupplyZoneMaxAngleDelta=f32(-7.25)))
    add('one_direction', lambda c: c.update(directions=[[f32(0), f32(1)]]))
    add('odd_directions', lambda c: c.update(directions=unit_directions(7, 0.3) + [[0, 0], [f32(1e-6), f32(-1e-6)]]))
    add('engaged_enemies', lambda c: c['engaged'].extend([{'gtid': 0x00080030, 'position': [f32(1200), f32(800)]}]))
    for seed in [0, 0x80000000, 0xffffffff, 0x13579bdf]:
        add(f'rng_{seed:08x}', lambda c, s=seed: c.update(rng_state=s))
    rnd = random.Random(0x52b530)
    for i in range(30):
        def change(c, r=random.Random(rnd.random())):
            pt = lambda: [f32(r.uniform(0, 3000)), f32(r.uniform(0, 3000))]
            c['svars']['Rout'] = [f32(r.uniform(-300, 300) if k in (0, 8, 9, 10, 11) else r.uniform(0, 3000) if k in (2, 5, 7) else r.uniform(-5, 20)) for k in range(13)]
            c['select_svars'] = {'RoutRandomCandidates': r.randrange(0, 12), 'RoutMaxDistanceDelta': f32(r.uniform(0, 80)),
                                 'RoutSupplyZoneCandidates': r.randrange(0, 4), 'RoutSupplyZoneMaxAngleDelta': f32(r.uniform(-60, 60))}
            c.update(org=pt(), min_distance=f32(r.uniform(0, 120)), rng_state=r.getrandbits(32),
                     directions=unit_directions(r.randrange(1, 40), r.uniform(0, 6)),
                     reachable=[f32(r.uniform(0, 80)) for _ in range(r.randrange(1, 5))],
                     open=[{'result': r.random() < 0.8, 'move': pt() if r.random() < 0.3 else None} for _ in range(r.randrange(1, 5))],
                     zone_results={'4': r.choice([0, 1]), '5': r.choice([0, 3])}, friendly_zone=bool(r.randrange(2)))
            c['engaged'] = [{'gtid': (r.randrange(1, 0x7fff) << 16) | (0x200 + j), 'position': pt()} for j in range(r.randrange(3))]
            c['kingdoms'] = []
            for j in range(r.randrange(4)):
                if r.random() < 0.2:
                    c['kingdoms'].append(None); continue
                c['kingdoms'].append({'allied': r.random() < 0.7, 'supplies': [
                    {'position': pt(), 'radius': f32(r.uniform(0, 800)) if r.random() < 0.85 else None} for _ in range(r.randrange(3))]})
        add(f'random{i}', change)
    for p in PRECISIONS:
        for c in rows:
            yield {**copy.deepcopy(c), 'name': c['name'] + '_' + p, 'precision': p}


def main():
    out, visited = [], {}
    for c in cases():
        try:
            m = SelectMachine(c); result = m.run()
        except Exception:
            print(c['name']); raise
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': result})
    d = header(__file__, __doc__)
    d.update(instructions=instructions(visited), cases=out)
    p = HERE / 'native-rout-select.json'
    p.write_text(json.dumps(d, indent=1) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'selected': sum(r['output']['selected'] for r in out),
                      'instructions': len(visited), 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
