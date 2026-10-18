"""Original recover destination selection 52BB40, with 52A930 executing natively.

52BB40 runs unchanged with the CAI Mind as `this` (Mind+4 = organization):
lazy SVar RecoverSupplyZoneCandidates, the sparse list [B36534]+154 and its
Kingdom virtual +138, supply-thing filtering including the original 58AC70
supply predicate over emulated component data, the global direction table
[B39AF8], inline 85D840 draws, tile computation, and every 52A930 call in
recover mode with the caller's moved-from object.

Seams: organization virtual +4 (x87 result discarded), World 4F46E0 open
position (case results, optional move), 54F8B0 friendly zone (records the
subject argument: 0 here, the organization inside 52A930) and those of
native_rout_score.py for 52A930.
"""
from pathlib import Path
import copy, json, math, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import PRECISIONS, f32, header, instructions, sha
from native_rout_score import RoutMachine, cases as score_cases, OWNER


class RecoverMachine(RoutMachine):
    def __init__(self, c):
        super().__init__(c)
        slot = self.alloc(4); self.w(slot, c['recover_candidates']); self.svars['RecoverSupplyZoneCandidates'] = slot
        vt = self.r(self.org)
        self.w(vt + 4, self.stub(self.v_org4))
        table = self.alloc(0x20); pairs = self.alloc(8 * max(1, len(c['directions'])))
        for i, (x, y) in enumerate(c['directions']):
            self.w(pairs + 8 * i, x); self.w(pairs + 8 * i + 4, y)
        self.w(table + 4, len(c['directions'])); self.w(table + 8, pairs); self.w(0xb39af8, table)
        world = self.r(0xb36534)
        slots = []
        for i, k in enumerate(c['recover_kingdoms']):
            if k is None:
                slots.append(0); continue
            obj = self.alloc(0x300); kvt = self.alloc(0x200); self.w(obj, kvt)
            self.w(kvt + 0x138, self.stub(lambda m, i=i: self.v_recover_allied(i)))
            head = 0
            for t in reversed(k['things']):
                thing = 0
                if t is not None:
                    thing = self.alloc(0x100)
                    self.w(thing + 0x14, t['gtid']); self.w(thing + 0x20, t['position'][0]); self.w(thing + 0x24, t['position'][1])
                    comp = t.get('component')
                    if comp is not None:
                        cp = self.alloc(0x40); self.w(thing + 0x8c, cp); self.w(cp + 0x10, comp['radius'])
                        holder = self.alloc(0x200); self.w(cp + 4, holder)
                        entries = comp['entries']
                        arr = self.alloc(4 * max(1, len(entries)))
                        for j, e in enumerate(entries):
                            if e is None:
                                self.w(arr + 4 * j, 0)
                            else:
                                ent = self.alloc(0x20); self.w(ent + 8, e); self.w(arr + 4 * j, ent)
                        self.w(holder + 0x148, arr); self.w(holder + 0x14c, comp.get('count', len(entries)))
                node = self.alloc(8); self.w(node, thing); self.w(node + 4, head); head = node
            self.w(obj + 0x264, head)
            slots.append(obj)
        self.w(world + 0x154, self.array(slots)); self.w(world + 0x158, len(slots))
        self.seam(0x4f46e0, self.s_open)
        self.open_calls = 0
        self.out = self.alloc(8); self.w(self.out, 0xdeadbeef); self.w(self.out + 4, 0xfeedface)
        self.out_gtid = self.alloc(4); self.w(self.out_gtid, 0xcafef00d)

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

    def s_friendly(self, m):
        idx, owner, subject = self.tile_index(), self.arg(1), self.arg(2)
        assert owner == OWNER and subject in (0, self.org)
        r = self.c['friendly_zone'] if subject else self.c['recover_friendly'][self.tile_parity(idx)]
        self.events.append({'event': 'friendly_zone', 'tile': idx, 'organization': bool(subject), 'result': r})
        self.ret(int(r), 8)

    @staticmethod
    def tile_parity(idx):
        return str(idx & 1)

    def v_org4(self, m):
        assert self.ecx() == self.org
        self.events.append({'event': 'organization_virtual_4'})
        self.ret_st0_f32(f32(-7.0), 0)

    def v_recover_allied(self, i):
        assert self.arg(1) == OWNER
        r = self.c['recover_kingdoms'][i]['allied']
        self.events.append({'event': 'recover_allied', 'kingdom': i, 'result': r}); self.ret(int(r), 4)

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
        result = self.call(0x52bb40, ecx=self.state, args=[self.rng, self.out, self.out_gtid, self.moved], fpsw=0x20) & 0xff
        return {'selected': result, 'out': [self.r(self.out), self.r(self.out + 4)], 'gtid': self.r(self.out_gtid),
                'rng_state': self.r(self.rng), 'events': self.events}


def unit_directions(n, phase=0.0):
    return [[f32(math.cos(phase + 2 * math.pi * k / n)), f32(math.sin(phase + 2 * math.pi * k / n))] for k in range(n)]


def thing(gtid, x, y, radius=200, entries=(f32(1),), count=None):
    t = {'gtid': gtid, 'position': [f32(x), f32(y)], 'component': None if radius is None else {'radius': f32(radius), 'entries': list(entries)}}
    if count is not None and t['component'] is not None:
        t['component']['count'] = count
    return t


def cases():
    base = copy.deepcopy(next(c for c in score_cases() if c['name'] == 'defaults_recover_Single24'))
    base.update(name='defaults', precision='Single24', recover_candidates=5, directions=unit_directions(24),
                moved_from=None,
                recover_kingdoms=[{'allied': True, 'things': [thing(0x00110001, 1200, 1000), thing(0x00110002, 800, 1300, 150)]},
                                  None, {'allied': False, 'things': [thing(0x00110003, 1000, 1010)]}],
                recover_friendly={'0': True, '1': True},
                open=[{'result': True}, {'result': True, 'move': [f32(1210.5), f32(990.25)]}, {'result': False}])
    rows = []

    def add(name, change):
        c = copy.deepcopy(base); c['name'] = name; change(c); rows.append(c)

    add('defaults', lambda c: None)
    add('moved_from', lambda c: c.update(moved_from=[f32(900), f32(1200)]))
    add('no_candidates', lambda c: c.update(recover_candidates=0))
    add('negative_candidates', lambda c: c.update(recover_candidates=0xfffffffe))
    add('never_open', lambda c: c.update(open=[{'result': False}]))
    add('not_friendly', lambda c: c.update(recover_friendly={'0': False, '1': False}))
    add('friendly_odd_tiles', lambda c: c.update(recover_friendly={'0': False, '1': True}))
    add('null_thing', lambda c: c['recover_kingdoms'][0]['things'].insert(0, None))
    add('no_component', lambda c: c['recover_kingdoms'][0]['things'].insert(0, thing(0x00110009, 1100, 1100, None)))
    add('inactive_component', lambda c: c['recover_kingdoms'][0]['things'].__setitem__(0, thing(0x0011000a, 1100, 1100, 50, [f32(-1)])))
    add('empty_component_list', lambda c: c['recover_kingdoms'][0]['things'].__setitem__(0, thing(0x0011000b, 1100, 1100, 50, [])))
    add('null_first_entry', lambda c: c['recover_kingdoms'][0]['things'].__setitem__(0, thing(0x0011000c, 1100, 1100, 50, [None, f32(1)])))
    add('negative_zero_entry_is_active', lambda c: c['recover_kingdoms'][0]['things'].__setitem__(0, thing(0x0011000d, 1100, 1100, 50, [0x80000000])))
    add('zero_radius', lambda c: c['recover_kingdoms'][0]['things'].__setitem__(0, thing(0x0011000e, 1000, 1000, 0)))
    add('thing_on_org', lambda c: c['recover_kingdoms'][0]['things'].__setitem__(0, thing(0x0011000f, 1000, 1000, 1e-30)))
    add('all_negative_scores', lambda c: c['svars'].update(Recover=[f32(-5000)] + c['svars']['Recover'][1:]))
    add('no_kingdoms', lambda c: c.update(recover_kingdoms=[]))
    for seed in [0, 0x80000000, 0xffffffff]:
        add(f'rng_{seed:08x}', lambda c, s=seed: c.update(rng_state=s))
    rnd = random.Random(0x52bb40)
    for i in range(30):
        def change(c, r=random.Random(rnd.random())):
            pt = lambda: [f32(r.uniform(0, 3000)), f32(r.uniform(0, 3000))]
            c['svars']['Recover'] = [f32(r.uniform(-300, 600) if k in (0, 8, 9, 10, 11) else r.uniform(0, 3000) if k in (2, 5, 7) else r.uniform(-5, 20)) for k in range(13)]
            c.update(org=pt(), rng_state=r.getrandbits(32), recover_candidates=r.randrange(0, 6),
                     moved_from=pt() if r.random() < 0.5 else None,
                     directions=unit_directions(r.randrange(1, 30), r.uniform(0, 6)),
                     recover_friendly={'0': r.random() < 0.8, '1': r.random() < 0.8},
                     open=[{'result': r.random() < 0.85, 'move': pt() if r.random() < 0.3 else None} for _ in range(r.randrange(1, 4))],
                     zone_results={'4': r.choice([0, 1]), '5': r.choice([0, 3])}, friendly_zone=bool(r.randrange(2)))
            c['engaged'] = [{'gtid': (r.randrange(1, 0x7fff) << 16) | (0x300 + j), 'position': pt()} for j in range(r.randrange(3))]
            c['recover_kingdoms'] = []
            for j in range(r.randrange(1, 4)):
                if r.random() < 0.15:
                    c['recover_kingdoms'].append(None); continue
                c['recover_kingdoms'].append({'allied': r.random() < 0.75, 'things': [
                    None if r.random() < 0.1 else thing((r.randrange(1, 0x7fff) << 16) | (0x400 + j * 8 + q), *pt_f(r),
                                                        None if r.random() < 0.1 else r.uniform(0, 400),
                                                        [f32(-1) if r.random() < 0.15 else f32(r.uniform(0, 5))])
                    for q in range(r.randrange(0, 3))]})
        add(f'random{i}', change)
    for p in PRECISIONS:
        for c in rows:
            yield {**copy.deepcopy(c), 'name': c['name'] + '_' + p, 'precision': p}


def pt_f(r):
    return [r.uniform(0, 3000), r.uniform(0, 3000)]


def main():
    out, visited = [], {}
    for c in cases():
        try:
            m = RecoverMachine(c); result = m.run()
        except Exception:
            print(c['name']); raise
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': result})
    d = header(__file__, __doc__)
    d.update(instructions=instructions(visited), cases=out)
    p = HERE / 'native-recover-select.json'
    p.write_text(json.dumps(d, indent=1) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'selected': sum(r['output']['selected'] for r in out),
                      'instructions': len(visited), 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
