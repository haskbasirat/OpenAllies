"""Original supply component and actor in-supply logic in isolated x86.

Executed unchanged: 58B120 thunk + 58AE10 SupplyComponent Tick, 58ACE0, 58ADB0,
4D0C90, 4D0A80, 4D7CE0, 56F2B0 EconomyComponent::Tick, 44DEE0, 4CE830, 4CF2C0,
432EF0, 529C30, 4F0460, and the native loop/list skeletons around the spatial
queries. Seams (recorded; bodies not executed): actor virtual +0x108 owner,
4EE700 large actor, 54F8B0 tile ownership, 4CF210 shape, 554DD0/554CC0/554330
spatial iteration (driven from the fixture), 54F8F0 candidate collection,
4D64F0 state change (applies the state bit like the native setter), 4D1350,
4CF0F0, 404F60 log + 50B1C0 unschedule, 514CE0/515F40 zone raster, allocator.
"""
from pathlib import Path
import copy, json, random, shutil, sys
sys.path.insert(0, str(Path(__file__).resolve().parent))
from harness import Machine, MODES, bits, pin, ret_pop, ROOT  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX  # noqa: E402

RANGES = {
    'supply_tick_thunk_58B120': (0x58B120, 0x58B130), 'supply_tick_58AE10': (0x58AE10, 0x58B0CC),
    'provides_58ACE0': (0x58ACE0, 0x58ADA3), 'relay_chain_58ADB0': (0x58ADB0, 0x58AE07),
    'stockpile_in_supply_4D0C90': (0x4D0C90, 0x4D0CC1), 'in_supply_4D0A80': (0x4D0A80, 0x4D0C8C),
    'supply_state_4D7CE0': (0x4D7CE0, 0x4D7D25), 'economy_tick_56F2B0': (0x56F2B0, 0x56F306),
    'gtid_d0_44DEE0': (0x44DEE0, 0x44DF17), 'relay_type_4CE830': (0x4CE830, 0x4CE84F),
    'relay_live_4CF2C0': (0x4CF2C0, 0x4CF2F4), 'siege_432EF0': (0x432EF0, 0x432F0A),
    'definition_529C30': (0x529C30, 0x529C37), 'suspended_4F0460': (0x4F0460, 0x4F0475),
    'zero_9F32DC': (0x9F32DC, 0x9F32E0),
}
AVT, GT = 0x2000000, 0x2100000
WORLD, WOBJ, REALMAP, FAKEMAP = 0x2200000, 0x2200100, 0x2200200, 0x2200300
REALTILES, FAKETILES = 0x2300000, 0x2400000
CACHE, SETTINGS = 0x2201000, 0x2201800
KING = 0x2210000
STUB_OWNER = 0x3000100
TILE = 48


def A(i): return 0x2500000 + i * 0x1000
def DEF(i): return 0x2500800 + i * 0x1000
def SC(i): return 0x2500600 + i * 0x1000
def EC(i): return 0x2500700 + i * 0x1000
def COMP(i): return 0x2500900 + i * 0x1000   # props +0, org comp +0x100, element +0x200, city +0x300
def ZONE(i): return 0x2500D00 + i * 0x1000


class SupplyMachine(Machine):
    def __init__(self, cw, case):
        super().__init__(cw)
        self.uc.mem_map(0x2400000, 0x400000)
        self.case = case
        w = case['world']
        self.n = len(case['actors'])
        self.put(0xB364C0, GT)
        for slot, (generation, live) in w['gtids'].items():
            self.put16(GT + 4 + 2 * int(slot), generation)
            self.put(GT + 0x20004 + 4 * int(slot), A(live) if live is not None else 0)
        self.put(0xB36534, WORLD); self.put(WORLD + 0x28, WOBJ); self.put(WOBJ + 0xA8, int(w['suspended']))
        self.put(WORLD + 0x30, REALMAP); self.put(REALMAP + 0x1C, w['map_width']); self.put(REALMAP + 0x28, REALTILES)
        self.put(FAKEMAP + 0x1C, 1); self.put(FAKEMAP + 0x28, FAKETILES)
        self.put(0xB39AB0, CACHE); self.put(CACHE + 0xFC, SETTINGS); self.put8(SETTINGS, int(w['provide_under_siege']))
        self.put(CACHE + 0x100, SETTINGS + 4); self.put(SETTINGS + 4, w['recharge_rate'])
        for j, k in enumerate(w['kingdoms']):
            self.put(KING + 0x400 * j + 0x190, k['word_190']); self.put(KING + 0x400 * j + 0x1F0, k['shortage_ratio'])
        self.put(AVT + 0x108, STUB_OWNER)
        for i, a in enumerate(case['actors']):
            self.write_actor(i, a)
        self.trace = []
        self.current = None
        self.shape_actor = None
        self.seam(0x6F7810, lambda m, esp: (self.alloc(self.read(esp + 4)), 0))
        self.seam(0x6F7430, ret_pop(0))
        self.seam(STUB_OWNER, self._owner)
        self.seam(0x4EE700, self._large)
        self.seam(0x54F8B0, self._tile)
        self.seam(0x4CF210, self._shape)
        self.seam(0x554DD0, self._iterate_tiles)
        self.seam(0x554CC0, self._iterate_once)
        self.seam(0x554330, self._advance)
        self.seam(0x54F8F0, self._candidates)
        self.seam(0x4D64F0, self._state)
        self.seam(0x4D1350, self._record('refresh_actor', 0))
        self.seam(0x4CF0F0, self._record('refresh_economy', 0))
        self.seam(0x404F60, lambda m, esp: (None, 0))
        self.seam(0x50B1C0, self._stop)
        self.seam(0x514CE0, self._clear_zone)
        self.seam(0x515F40, self._set_zone)

    def actor_of(self, pointer):
        i = (pointer - 0x2500000) // 0x1000
        assert 0 <= i < self.n and A(i) == pointer, hex(pointer)
        return i

    def write_actor(self, i, a):
        p, d = A(i), DEF(i)
        self.put(p, AVT); self.put(p + 4, d); self.put(p + 8, a['flags8']); self.put(p + 0x14, a['gtid'])
        self.put_words(p + 0x20, a['position']); self.put_words(p + 0x44, a['tile'])
        self.put(p + 0x100, a['states'] & 0xFFFFFFFF); self.put(p + 0x104, a['states'] >> 32)
        df = a['definition']
        self.put8(d + 0x424, int(df['relay'])); self.put(d + 0x428, df['stockpile_max'])
        self.put8(d + 0x348, int(df['economy_ticks'])); self.put8(d + 0x31D, int(df['byte_31d'])); self.put8(d + 0x2F4, int(df['byte_2f4']))
        c = COMP(i)
        self.put(p + 0x78, c if a['properties_flag20'] is not None else 0)
        if a['properties_flag20'] is not None:
            self.put(c + 0x1C, 0x20 if a['properties_flag20'] else 0x4)
        self.put(p + 0x7C, c + 0x100 if a['organization_component'] is not None else 0)
        if a['organization_component'] is not None:
            self.put(c + 0x100 + 0xD4, int(a['organization_component']))
        self.put(p + 0x80, c + 0x200 if a['element'] is not None else 0)
        if a['element'] is not None:
            self.put(c + 0x200 + 0x10, A(a['element'][0]) if a['element'][0] is not None else 0)
        self.put(p + 0xD4, c + 0x300 if a['city_flag20'] is not None else 0)
        if a['city_flag20'] is not None:
            self.put8(c + 0x300 + 0x20, int(a['city_flag20']))
        self.put(p + 0x118, int(a['word_118'])); self.put(p + 0x128, int(a['word_128'])); self.put(p + 0xD8, int(a['word_d8']))
        self.put(p + 0xD0, a['gtid_d0'])
        self.put(p + 0xE8, KING + 0x400 * a['kingdom'] if a['kingdom'] is not None else 0)
        s = a['supply']
        self.put(p + 0x8C, SC(i) if s is not None else 0)
        if s is not None:
            self.put(SC(i) + 4, p); self.put(SC(i) + 0x10, s['radius']); self.put8(SC(i) + 0x14, int(s['dirty']))
            self.put(SC(i) + 0x18, s['stockpile']); self.put(SC(i) + 0x1C, s['relay'])
        self.put(EC(i) + 4, p)
        z = a['zone_word8']
        if z == 'empty':
            self.put(p + 0x148, 0); self.put(p + 0x14C, 0)
        else:
            self.put(p + 0x148, ZONE(i)); self.put(p + 0x14C, 1)
            self.put(ZONE(i), ZONE(i) + 0x10 if z is not None else 0)
            if z is not None:
                self.put(ZONE(i) + 0x10 + 8, z)

    def read_actor(self, i):
        p = A(i)
        out = {'states': self.read(p + 0x100) | (self.read(p + 0x104) << 32)}
        if self.read(p + 0x8C):
            out['supply'] = {'radius': self.read(SC(i) + 0x10), 'dirty': bool(self.read8(SC(i) + 0x14)),
                             'stockpile': self.read(SC(i) + 0x18), 'relay': self.read(SC(i) + 0x1C)}
        else:
            out['supply'] = None
        return out

    def ecx(self):
        return self.uc.reg_read(UC_X86_REG_ECX)

    def _owner(self, m, esp):
        i = self.actor_of(self.ecx())
        self.trace.append({'kind': 'owner', 'actor': i})
        return self.read(A(i) + 0xE8), 0

    def _large(self, m, esp):
        i = self.actor_of(self.ecx())
        self.trace.append({'kind': 'large_actor', 'actor': i})
        return int(self.case['actors'][i]['large']), 0

    def _tile(self, m, esp):
        ptr, kingdom, actor = self.ecx(), self.read(esp + 4), self.read(esp + 8)
        i = self.actor_of(actor)
        if REALTILES <= ptr < FAKETILES:
            assert (ptr - REALTILES) % TILE == 0
            tile = (ptr - REALTILES) // TILE
        else:
            assert (ptr - FAKETILES) % TILE == 0
            tile = self.case['actors'][i]['covered'][(ptr - FAKETILES) // TILE]
        k = None if kingdom == 0 else (kingdom - KING) // 0x400
        self.trace.append({'kind': 'tile_supplied', 'tile': tile, 'kingdom': k, 'actor': i})
        return int(tile in self.case['world']['supplied_tiles']), 8

    def _shape(self, m, esp):
        self.shape_actor = self.actor_of(self.ecx())
        return self.read(esp + 4), 4

    def _iterate_tiles(self, m, esp):
        it = self.ecx()
        i = self.shape_actor
        n = len(self.case['actors'][i]['covered'])
        self.trace.append({'kind': 'covered_tiles', 'actor': i})
        self.put(it, FAKEMAP); self.put(it + 0x10, n - 1); self.put(it + 0x24, 0); self.put(it + 0x28, 0)
        return it, 16

    def _iterate_once(self, m, esp):
        it = self.ecx()
        self.put(it, FAKEMAP); self.put(it + 0x10, 0); self.put(it + 0x24, 0); self.put(it + 0x28, 0)
        return it, 12

    def _advance(self, m, esp):
        it = self.ecx()
        self.put(it + 0x28, self.read(it + 0x28) + 1)
        return None, 4

    def _candidates(self, m, esp):
        lst, kingdom, zero, actor = (self.read(esp + 4 + 4 * k) for k in range(4))
        assert zero == 0
        i = self.actor_of(actor)
        k = None if kingdom == 0 else (kingdom - KING) // 0x400
        self.trace.append({'kind': 'relay_candidates', 'actor': i, 'kingdom': k})
        for c in self.case['actors'][i]['candidates']:
            node = self.alloc(8)
            self.put(node, A(c)); self.put(node + 4, 0)
            if self.read(lst) == 0:
                self.put(lst, node)
            else:
                self.put(self.read(lst + 4) + 4, node)
            self.put(lst + 4, node)
        return None, 16

    def _state(self, m, esp):
        i = self.actor_of(self.ecx())
        state, enabled = self.read(esp + 4), bool(self.read(esp + 8) & 0xFF)
        self.trace.append({'kind': 'set_state', 'actor': i, 'state': state, 'enabled': enabled})
        word = A(i) + 0x100 + 4 * (state // 32)
        mask = 1 << (state % 32)
        self.put(word, (self.read(word) | mask) if enabled else (self.read(word) & ~mask))
        return None, 8

    def _record(self, name, pop):
        def h(m, esp):
            self.trace.append({'kind': name, 'actor': self.actor_of(self.ecx())})
            return None, pop
        return h

    def _stop(self, m, esp):
        assert self.read(esp + 4) == 1 and self.read(esp + 8) == 0
        i = self.actor_of(self.read(self.ecx() - 4))
        self.trace.append({'kind': 'stop_economy_ticks', 'actor': i})
        return None, 8

    def _clear_zone(self, m, esp):
        self.trace.append({'kind': 'clear_zone', 'actor': self.current})
        return None, 0

    def _set_zone(self, m, esp):
        self.trace.append({'kind': 'set_zone', 'actor': self.current, 'position': [self.read(esp + 4), self.read(esp + 8)],
                           'radius': self.read(esp + 12)})
        return None, 12

    def relay_walk_terminates(self, start):
        seen, i = set(), start
        while True:
            p = A(i)
            if not self.read(p + 0x8C) or not self.read8(DEF(i) + 0x424):
                return True
            if i in seen:
                return False
            seen.add(i)
            g = self.read(SC(i) + 0x1C)
            slot, gen = g & 0xFFFF, g >> 16
            if self.uc.mem_read(GT + 4 + 2 * slot, 2) != gen.to_bytes(2, 'little'):
                return True
            ptr = self.read(GT + 0x20004 + 4 * slot)
            if not ptr:
                return True
            i = self.actor_of(ptr)

    def run(self, op):
        self.trace = []
        i = op['actor']
        self.current = i
        result = None
        if op['kind'] == 'supply_tick':
            self.call(0x58B120, [op['seconds'], 0x12345678], SC(i) + 8)
        elif op['kind'] == 'economy_tick':
            self.call(0x56F2B0, [op['seconds'], 0x12345678], EC(i) + 8)
        elif op['kind'] == 'in_supply':
            eax, _ = self.call(0x4D0A80, [], A(i)); result = bool(eax & 0xFF)
        elif op['kind'] == 'provides':
            eax, _ = self.call(0x58ACE0, [], SC(i)); result = bool(eax & 0xFF)
        return {'actors': [self.read_actor(k) for k in range(self.n)], 'events': self.trace, 'result': result}


R = random.Random(0x58AE10)


def make_case(name):
    n = R.randint(2, 6)
    kingdoms = [{'word_190': R.choice([0, 0, 0, 1]), 'shortage_ratio': R.choice([bits(0.0), bits(0.0), bits(0.35), bits(-0.1)])}
                for _ in range(R.randint(1, 2))]
    gtids = {}
    actors = []
    for i in range(n):
        slot = 10 + i
        generation = R.randint(1, 500)
        gtids[str(slot)] = (generation, i)
        actors.append({'gtid': (generation << 16) | slot})
    dead = [(R.randint(1, 500) << 16) | (100 + k) for k in range(3)]
    for k, g in enumerate(dead):
        gtids[str(g & 0xFFFF)] = (g >> 16, None)
    for i, a in enumerate(actors):
        relay_type = R.random() < 0.5
        has_supply = R.random() < 0.8
        lower = [actors[j]['gtid'] for j in range(i)]
        a.update({
            'flags8': R.choice([0, 0, 0, 1]),
            'states': R.choice([0, 0, 1 << 19, 1 << 20, 1 << 21, 1 << 39, (1 << 19) | (1 << 20), 1 << 27]),
            'definition': {'relay': relay_type, 'stockpile_max': R.choice([bits(100.0), bits(30.0), bits(0.0)]),
                           'economy_ticks': R.random() < 0.85, 'byte_31d': R.random() < 0.5, 'byte_2f4': R.random() < 0.5},
            'supply': None if not has_supply else {
                'radius': R.choice([bits(12.0), bits(0.0), bits(-1.0), bits(30.5)]), 'dirty': R.random() < 0.4,
                'stockpile': R.choice([bits(0.0), bits(10.0), bits(99.5), bits(0.25), bits(R.uniform(0, 120))]),
                'relay': R.choice([0] + lower + dead)},
            'element': None if R.random() < 0.6 else ([R.randrange(n)] if R.random() < 0.8 else [None]),
            'organization_component': None if R.random() < 0.6 else R.random() < 0.3,
            'properties_flag20': None if R.random() < 0.3 else R.random() < 0.5,
            'city_flag20': None if R.random() < 0.7 else R.random() < 0.5,
            'word_118': R.random() < 0.15, 'word_128': R.random() < 0.15, 'word_d8': R.random() < 0.3,
            'gtid_d0': R.choice([0, actors[R.randrange(n)]['gtid'], dead[0]]),
            'kingdom': None if R.random() < 0.1 else R.randrange(len(kingdoms)),
            'position': [bits(R.uniform(0, 500)), bits(R.uniform(0, 500))],
            'tile': [R.randrange(8), R.randrange(8)],
            'zone_word8': R.choice(['empty', None, bits(-1.0), bits(0.0), bits(12.0)]),
            'large': R.random() < 0.3,
            'covered': [R.randrange(64) for _ in range(R.randint(0, 4))],
            'candidates': [R.randrange(n) for _ in range(R.randint(0, 3))],
        })
    world = {'gtids': gtids, 'suspended': R.random() < 0.1, 'map_width': 8, 'provide_under_siege': R.random() < 0.5,
             'recharge_rate': R.choice([bits(2.0), bits(0.5), bits(1.0 / 3.0)]), 'kingdoms': kingdoms,
             'supplied_tiles': sorted(R.sample(range(64), R.randint(5, 40)))}
    ops = []
    for _ in range(R.randint(4, 12)):
        ops.append({'kind': R.choice(['supply_tick', 'supply_tick', 'economy_tick', 'in_supply', 'provides']),
                    'actor': R.randrange(n), 'seconds': R.choice([bits(0.1), bits(1.0), bits(0.25), bits(3.0)])})
    return {'name': name, 'world': world, 'actors': actors, 'ops': ops}


def main():
    cases = [make_case(f'random-{i}') for i in range(90)]
    fixtures, executions, skipped = [], 0, 0
    for case in cases:
        for mode, cw in MODES:
            m = SupplyMachine(cw, case)
            initial = [m.read_actor(k) for k in range(m.n)]
            steps = []
            for op in case['ops']:
                a = case['actors'][op['actor']]
                if op['kind'] in ('provides', 'supply_tick') and a['supply'] is None:
                    continue  # 58ACE0/58AE10 are SupplyComponent methods
                if op['kind'] == 'supply_tick' and not all(m.relay_walk_terminates(c) for c in a['candidates']):
                    skipped += 1
                    continue
                if op['kind'] in ('in_supply', 'economy_tick') and a['element'] == [None]:
                    pass
                steps.append({'op': op, 'expected': m.run(op)})
                executions += 1
            fixtures.append({'name': case['name'], 'precision': mode, 'case': case, 'initial': initial, 'steps': steps})
    doc = {'schema': 1, 'pins': pin(RANGES), 'executions': executions, 'skipped_nonterminating': skipped, 'fixtures': fixtures}
    out = Path(__file__).resolve().parent / 'native-supply-fixtures.json'
    out.write_text(json.dumps(doc, separators=(',', ':')) + '\n', encoding='utf-8')
    shutil.copyfile(out, ROOT / 'crates/aa-economy/tests/fixtures/native-supply-fixtures.json')
    print(f'{len(fixtures)} fixtures, {executions} native executions, {skipped} skipped')


if __name__ == '__main__':
    main()
