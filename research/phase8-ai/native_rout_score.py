"""Original CAI rout/recover destination score 52A930 (Source/Sim/Tai).

52A930 scores one candidate location for a routing organization (CAI rout
state, caller 52B530) or for an SAI recover goal (caller 52BB40). It executes
unchanged, including the lazy SVar pointer cache, the sparse Kingdom iterator
4F0680/4F06B0, the angle routine 445400 with its CRT asin path, and the inline
85D840 draw on the caller-supplied RNG word.

Seams (all recorded in order): SVar lookup 4B2E60 (returns a slot holding the
case value for that SVar name), organization virtual +108 (owner), tile zone
query 54EFC0 (kinds 4 = ZOC, 5 = ZOG), friendly-zone query 54F8B0, Kingdom
virtual +138 (allied with owner). The GTID registry at [B364C0] is an emulated
table (generation word at +4+2*i, pointer at +20004+4*i).

The x87 status word enters with the sticky precision-exception flag set (as in
a running game), because 445400's CRT tail depends on it. Each 445400 call's
inputs and its unstored ST0 result are recorded so the Rust test can bind that
service and independently compare it with aa_trig::angle::signed_angle.
"""
from pathlib import Path
import copy, json, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import Machine, PRECISIONS, f32, from_f32, header, instructions, sha

KINDS = ['BaseValue', 'FactorEnemyDist', 'FactorEnemyDistCap', 'FactorEnemyAngle', 'FactorMovedDist',
         'FactorMovedDistCap', 'FactorSupplyDist', 'FactorSupplyDistCap', 'PenaltyEnemyZOC', 'PenaltyEnemyZOG',
         'BonusFriendlyZOS', 'BonusSupplyInRange', 'Fuzziness']
DEFAULTS = {'Rout': [0, 10, 2000, .25, 8, 2000, 1, 2000, 50, 75, 50, 50, 20],
            'Recover': [201, 0, 0, 1, 1, 200, 0, 0, 500, 500, 0, 0, 40]}
OWNER = 0x0bad0000
TILE_SIZE = 0x30


class RoutMachine(Machine):
    def __init__(self, c):
        super().__init__(c['precision'])
        self.c = c
        self.svars = {}
        for prefix in ['Rout', 'Recover']:
            for k, v in zip(KINDS, c['svars'][prefix]):
                slot = self.alloc(4); self.w(slot, v); self.svars[prefix + k] = slot
        self.seam(0x4b2e60, self.s_svar)
        # Registry of GTIDs.
        reg = self.alloc(0x20004 + 0x10000 * 4 + 16)
        self.w(0xb364c0, reg); self.reg = reg
        # Organization.
        org = self.org = self.alloc(0x200)
        vt = self.alloc(0x200); self.w(org, vt); self.w(vt + 0x108, self.stub(self.v_owner))
        self.w(org + 0x20, c['org'][0]); self.w(org + 0x24, c['org'][1])
        head = 0
        for e in reversed(c['engaged']):
            node = self.alloc(8); self.w(node, e['gtid']); self.w(node + 4, head); head = node
            if e.get('position') is not None:
                idx, gen = e['gtid'] & 0xffff, e['gtid'] >> 16
                actor = self.alloc(0x40); self.w(actor + 0x20, e['position'][0]); self.w(actor + 0x24, e['position'][1])
                self.u.mem_write(reg + 4 + 2 * idx, struct.pack('<H', gen))
                self.w(reg + 0x20004 + 4 * idx, actor)
            elif e.get('stale') == 'generation':
                idx, gen = e['gtid'] & 0xffff, e['gtid'] >> 16
                actor = self.alloc(0x40)
                self.u.mem_write(reg + 4 + 2 * idx, struct.pack('<H', (gen + 1) & 0xffff))
                self.w(reg + 0x20004 + 4 * idx, actor)
            elif e.get('stale') == 'null':
                idx, gen = e['gtid'] & 0xffff, e['gtid'] >> 16
                self.u.mem_write(reg + 4 + 2 * idx, struct.pack('<H', gen))
        self.w(org + 0x118, head)
        state = self.state = self.alloc(0x40); self.w(state + 4, org)
        # Optional moved-from object (+54/+58).
        self.moved = 0
        if c['moved_from'] is not None:
            self.moved = self.alloc(0x80); self.w(self.moved + 0x54, c['moved_from'][0]); self.w(self.moved + 0x58, c['moved_from'][1])
        # World, map and Kingdom sparse array.
        world = self.alloc(0x200); self.w(0xb36534, world)
        mp = self.alloc(0x40); self.w(world + 0x30, mp)
        self.w(mp + 4, c['map']['shift']); self.w(mp + 0x1c, c['map']['width'])
        self.tiles = HEAP_TILES = 0x21000000  # never dereferenced: both tile queries are seams
        self.w(mp + 0x28, self.tiles)
        slots = []
        for i, k in enumerate(c['kingdoms']):
            if k is None:
                slots.append(0); continue
            obj = self.alloc(0x300); kvt = self.alloc(0x200); self.w(obj, kvt)
            self.w(kvt + 0x138, self.stub(lambda m, i=i: self.v_allied(i)))
            shead = 0
            for s in reversed(k['supplies']):
                thing = self.alloc(0x100); self.w(thing + 0x20, s['position'][0]); self.w(thing + 0x24, s['position'][1])
                if s['radius'] is not None:
                    comp = self.alloc(0x40); self.w(comp + 0x10, s['radius']); self.w(thing + 0x8c, comp)
                node = self.alloc(8); self.w(node, thing); self.w(node + 4, shead); shead = node
            self.w(obj + 0x264, shead)
            slots.append(obj)
        self.w(world + 0x148, self.array(slots)); self.w(world + 0x14c, len(slots))
        self.rng = self.alloc(8); self.w(self.rng, c['rng_state'])
        self.seam(0x54efc0, self.s_zone)
        self.seam(0x54f8b0, self.s_friendly)
        self.observe(0x445400, self.s_angle_entry)
        self.observe(0x52b227, self.s_angle_return)
        self.angles = []

    def s_svar(self, m):
        assert self.ecx() == 0xb39aa8 and self.arg(2) == 2
        p = self.arg(1); name = ''
        while True:
            ch = self.u.mem_read(p + 2 * len(name), 2)
            if ch == b'\0\0':
                break
            name += chr(ch[0] | ch[1] << 8)
        self.events.append({'event': 'svar', 'name': name})
        self.ret(self.svars[name], 8)

    def v_owner(self, m):
        assert self.ecx() == self.org
        self.events.append({'event': 'owner'}); self.ret(OWNER)

    def tile_index(self):
        delta = (self.ecx() - self.tiles) & 0xffffffff
        if delta >= 0x80000000:
            delta -= 0x100000000
        assert delta % TILE_SIZE == 0
        return delta // TILE_SIZE

    def s_zone(self, m):
        idx, owner, kind = self.tile_index(), self.arg(1), self.arg(2)
        assert owner == OWNER and kind in (4, 5)
        result = self.c['zone_results'][str(kind)]
        self.events.append({'event': 'zone', 'tile': idx, 'kind': kind, 'result': result})
        self.ret(result, 8)

    def s_friendly(self, m):
        idx, owner, org = self.tile_index(), self.arg(1), self.arg(2)
        assert owner == OWNER and org == self.org
        r = self.c['friendly_zone']
        self.events.append({'event': 'friendly_zone', 'tile': idx, 'result': r}); self.ret(int(r), 8)

    def v_allied(self, i):
        assert self.arg(1) == OWNER
        r = self.c['kingdoms'][i]['allied']
        self.events.append({'event': 'allied', 'kingdom': i, 'result': r}); self.ret(int(r), 4)

    def s_angle_entry(self, m):
        a, b = self.arg(1), self.arg(2)
        from unicorn.x86_const import UC_X86_REG_FPSW
        self.angles.append({'from': [self.r(a), self.r(a + 4)], 'to': [self.r(b), self.r(b + 4)],
                            'pe': bool(self.u.reg_read(UC_X86_REG_FPSW) & 0x20)})

    def s_angle_return(self, m):
        a = self.angles[-1]
        a.update(self.st0_live())
        self.events.append({'event': 'angle', **a})

    def run(self):
        c = self.c
        self.call(0x52a930, ecx=self.state,
                  args=[self.rng, c['candidate'][0], c['candidate'][1], self.moved, int(c['recover'])], fpsw=0x20)
        out = self.st0()
        return {'result': out, 'rng_state': self.r(self.rng), 'events': self.events}


def cases():
    bits = lambda v: [f32(x) for x in v]
    base = {
        'name': 'defaults', 'precision': 'Single24', 'recover': False,
        'svars': {k: bits(v) for k, v in DEFAULTS.items()},
        'org': bits([1000, 1000]), 'candidate': bits([1100, 900]), 'moved_from': None,
        'engaged': [{'gtid': 0x00030005, 'position': bits([950, 1080])},
                    {'gtid': 0x00010009, 'position': bits([1020, 1150])}],
        'map': {'shift': 3, 'width': 512},
        'zone_results': {'4': 0, '5': 0}, 'friendly_zone': False,
        'kingdoms': [{'allied': True, 'supplies': [{'position': bits([1400, 700]), 'radius': f32(200)},
                                                   {'position': bits([1150, 880]), 'radius': f32(80)}]},
                     None,
                     {'allied': False, 'supplies': [{'position': bits([1101, 901]), 'radius': f32(500)}]}],
        'rng_state': 0x2468ace1,
    }
    rows = []

    def add(name, change):
        c = copy.deepcopy(base); c['name'] = name; change(c); rows.append(c)

    add('defaults_rout', lambda c: None)
    add('defaults_recover', lambda c: c.update(recover=True))
    for zoc in [0, 1, 0xffffffff]:
        for zog in [0, 7]:
            for zos in [False, True]:
                add(f'zones_{zoc:x}_{zog}_{zos}', lambda c, a=zoc, b=zog, z=zos: c.update(zone_results={'4': a, '5': b}, friendly_zone=z))
    add('no_engaged', lambda c: c.update(engaged=[]))
    add('zero_gtid_node', lambda c: c['engaged'].append({'gtid': 0}))
    add('stale_generation', lambda c: c['engaged'].append({'gtid': 0x00070011, 'stale': 'generation'}))
    add('stale_null_pointer', lambda c: c['engaged'].append({'gtid': 0x00070012, 'stale': 'null'}))
    add('only_stale', lambda c: c.update(engaged=[{'gtid': 0x00020003, 'stale': 'null'}, {'gtid': 0x00020004, 'stale': 'generation'}]))
    add('enemy_on_org', lambda c: c['engaged'].append({'gtid': 0x00040020, 'position': c['org']}))
    add('enemy_on_candidate', lambda c: c['engaged'].append({'gtid': 0x00040021, 'position': c['candidate']}))
    add('candidate_on_org', lambda c: c.update(candidate=c['org']))
    add('candidate_tiny_offset', lambda c: c.update(candidate=bits([1000.00005, 1000])))
    add('enemy_tiny_offset', lambda c: c['engaged'].append({'gtid': 0x00040022, 'position': bits([1000, 1000.00009])}))
    add('enemy_behind', lambda c: c.update(engaged=[{'gtid': 0x00040023, 'position': bits([900, 1100])}]))
    add('enemy_far_capped', lambda c: c.update(engaged=[{'gtid': 0x00040024, 'position': bits([-5000, 9000])}]))
    for mv in [[1000, 1000], [1100, 900], [700, 1500], [5000, -3000]]:
        add(f'moved_{mv[0]}_{mv[1]}', lambda c, mv=mv: c.update(moved_from=bits(mv)))
    add('no_kingdoms', lambda c: c.update(kingdoms=[]))
    add('all_null_kingdoms', lambda c: c.update(kingdoms=[None, None]))
    add('allied_empty_supplies', lambda c: c.update(kingdoms=[{'allied': True, 'supplies': []}]))
    add('allied_no_component', lambda c: c.update(kingdoms=[{'allied': True, 'supplies': [{'position': bits([1100, 900]), 'radius': None}]}]))
    add('supply_exact_radius', lambda c: c.update(kingdoms=[{'allied': True, 'supplies': [{'position': bits([1103, 904]), 'radius': f32(5)}]}]))
    add('supply_on_candidate', lambda c: c.update(kingdoms=[{'allied': True, 'supplies': [{'position': c['candidate'], 'radius': f32(0)}]}]))
    add('many_supplies', lambda c: c.update(kingdoms=[{'allied': True, 'supplies': [
        {'position': bits([1100 + 37 * i, 900 - 11 * i]), 'radius': f32(25 * i) if i % 3 else None} for i in range(9)]},
        None, {'allied': True, 'supplies': [{'position': bits([1090, 905]), 'radius': f32(30)}]}]))
    for shift, width in [(0, 4096), (5, 128), (31, 7)]:
        add(f'map_{shift}_{width}', lambda c, s=shift, w=width: c.update(map={'shift': s, 'width': w}))
    add('negative_candidate', lambda c: c.update(candidate=bits([-17.75, -0.5])))
    for seed in [0, 0x7fffffff, 0x80000000, 0xffffffff, 0xdeadbeef]:
        add(f'rng_{seed:08x}', lambda c, s=seed: c.update(rng_state=s))
    rnd = random.Random(0x52a930)
    for i in range(60):
        def change(c, r=random.Random(rnd.random())):
            pt = lambda: bits([r.uniform(0, 3000), r.uniform(0, 3000)])
            for prefix in ['Rout', 'Recover']:
                c['svars'][prefix] = bits([r.uniform(-300, 300) if k.endswith(('Value', 'ZOC', 'ZOG', 'ZOS', 'Range'))
                                           else r.uniform(0, 3000) if k.endswith('Cap')
                                           else r.uniform(-5, 20) for k in KINDS])
            c.update(recover=bool(r.randrange(2)), org=pt(), candidate=pt(), rng_state=r.getrandbits(32),
                     moved_from=pt() if r.randrange(2) else None,
                     zone_results={'4': r.choice([0, 1]), '5': r.choice([0, 3])}, friendly_zone=bool(r.randrange(2)),
                     map={'shift': r.randrange(6), 'width': r.randrange(1, 1024)})
            c['engaged'] = []
            for j in range(r.randrange(5)):
                g = (r.randrange(1, 0x7fff) << 16) | (0x100 + 7 * j)
                kind = r.random()
                c['engaged'].append({'gtid': g, 'position': pt()} if kind < 0.75 else {'gtid': g, 'stale': r.choice(['null', 'generation'])})
            c['kingdoms'] = []
            for j in range(r.randrange(4)):
                if r.random() < 0.2:
                    c['kingdoms'].append(None); continue
                c['kingdoms'].append({'allied': r.random() < 0.7, 'supplies': [
                    {'position': pt(), 'radius': f32(r.uniform(0, 800)) if r.random() < 0.85 else None} for _ in range(r.randrange(4))]})
        add(f'random{i}', change)
    for p in PRECISIONS:
        for c in rows:
            yield {**copy.deepcopy(c), 'name': c['name'] + '_' + p, 'precision': p}


def main():
    out, visited = [], {}
    for c in cases():
        try:
            m = RoutMachine(c); result = m.run()
        except Exception:
            print(c['name']); raise
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': result})
    d = header(__file__, __doc__)
    d.update(instructions=instructions(visited), cases=out)
    p = HERE / 'native-rout-score.json'
    p.write_text(json.dumps(d, indent=1) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'instructions': len(visited), 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
