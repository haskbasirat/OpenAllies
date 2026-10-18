"""Original GKingdom economy Tick 0x4E9C40, executed completely in isolated x86.

Executed unchanged: 4E9C40 (flow, overdraw clamp, upkeep-shortage ratio and
event decisions), 4E12D0 (recruit-single regeneration), 45D5B0 (ActorData+2CC
rate), 4E0EC0 (recruit-single "available" virtual), 4CDFD0 (faction XP income),
58E870 (score add), 4E37D0 (snapshot timer), 58ED00 (production statistics),
96CC4C (_ftol). Seams (recorded, bodies NOT executed): 6F7810/6F7430 allocator,
42D4B0 event dispatch, 4E9960/4E9A40 victory checks, 58F530 snapshot
constructor, 45F240 snapshot list push. Writes research/phase8-economy/
native-tick-fixtures.json and copies it into the aa-economy test fixtures.
"""
from pathlib import Path
import json, random, shutil, struct, sys
sys.path.insert(0, str(Path(__file__).resolve().parent))
from harness import Machine, MODES, bits, f32, pin, ret_pop, ROOT  # noqa: E402

RANGES = {
    'tick_4E9C40': (0x4E9C40, 0x4E9FBB), 'regen_4E12D0': (0x4E12D0, 0x4E13E4),
    'rate_45D5B0': (0x45D5B0, 0x45D5D2), 'single_available_4E0EC0': (0x4E0EC0, 0x4E0EC7),
    'faction_xp_4CDFD0': (0x4CDFD0, 0x4CE073), 'score_add_58E870': (0x58E870, 0x58E883),
    'snapshot_timer_4E37D0': (0x4E37D0, 0x4E3893), 'production_stats_58ED00': (0x58ED00, 0x58EDCF),
    'ftol_96CC4C': (0x96CC4C, 0x96CCC1), 'single_vtable_A29694': (0xA29694, 0xA296B0),
    'zero_9F32DC': (0x9F32DC, 0x9F32E4), 'hundred_9F2408': (0x9F2408, 0x9F240C),
    'seconds_to_minutes_A1B268': (0xA1B268, 0xA1B26C), 'ratio_threshold_A2A278': (0xA2A278, 0xA2A27C),
    'snapshot_period_A028CC': (0xA028CC, 0xA028D0), 'victory_time_B28194': (0xB28194, 0xB28198),
}
CODE_RANGES = [v for k, v in RANGES.items() if not k.split('_')[-1].startswith(('9F', 'A0', 'A1', 'A2', 'B2'))
               and 'vtable' not in k]
SEAMS = {0x6F7810: 'alloc', 0x6F7430: 'free', 0x42D4B0: 'event', 0x4E9960: 'team_victory',
         0x4E9A40: 'percentage_victory', 0x58F530: 'snapshot_new', 0x45F240: 'snapshot_push',
         0x4849F0: 'vector_grow_unexpected'}

K = 0x2000000           # GKingdom; economy updatable at K+0x180
K2 = 0x2008000          # other faction-XP owner kingdom
ARR = {'raw_provided': 0x2010000, 'effective_provided': 0x2010400, 'consumed': 0x2010800, 'stockpiled': 0x2010C00}
OFF = {'raw_provided': 0x1AC, 'effective_provided': 0x1A0, 'consumed': 0x1B8, 'stockpiled': 0x1C4}
REG, TABLE, DEFS, COSTS = 0x2020000, 0x2020100, 0x2021000, 0x2022000
CACHE, LIMITS = 0x2024000, 0x2024800
WORLD, GTID = 0x2026000, 0x2100000
SINGLES, ITEMS, ACTORDATA = 0x2030000, 0x2031000, 0x2034000
FACTION, FACTION_DEF = 0x2038000, 0x2038100
STATS, STATS2 = 0x2040000, 0x2041000
MANAGER = 0x7A7A0000
SCORES = 64


class TickMachine(Machine):
    def __init__(self, cw, case):
        super().__init__(cw)
        self.case = case
        n = len(case['catalog']['resources'])
        self.count = n
        st = case['economy']
        for field, offset in OFF.items():
            self.put(K + offset, ARR[field]); self.put(K + offset + 4, n); self.put(K + offset + 8, n)
            self.put_words(ARR[field], st[field])
        self.put(0xB36530, REG); self.put(REG + 0x34, TABLE); self.put(REG + 0x38, n)
        for i, d in enumerate(case['catalog']['resources']):
            self.put(TABLE + 4 * i, DEFS + 0x40 * i)
            self.put(DEFS + 0x40 * i + 0x18, {'Stockpiled': 0, 'Upkeep': 1, 'LimitedConsumed': 2}[d['kind']])
            self.put(DEFS + 0x40 * i + 0x1C, COSTS + 0x40 * i)
            self.put_words(COSTS + 0x40 * i, d['purchase_cost'])
        self.put(0xB39AB0, CACHE)
        self.put(CACHE + 0x1B4, LIMITS); self.put(LIMITS, case['limits']['provided_maximum'])
        self.put(CACHE + 0x1B8, LIMITS + 4); self.put(LIMITS + 4, case['limits']['stockpiled_maximum'])
        self.put(CACHE + 0x180, LIMITS + 8); self.put(LIMITS + 8, case['settings']['commander_experience_rate'])
        self.put(CACHE + 0x17C, LIMITS + 12); self.put(LIMITS + 12, 0x7FC00000)  # unused by param_3=0 path
        self.put(0xB36534, WORLD)
        self.put(0xB39D8C, MANAGER)
        self.put(0xB364C0, GTID)
        for slot, (generation, live) in case['gtid_table'].items():
            slot = int(slot)
            self.put16(GTID + 4 + 2 * slot, generation)
            self.put(GTID + 0x20004 + 4 * slot, 0x5A5A0000 + slot if live else 0)
        t = case['tick']
        self.put(K + 0x190, int(t['suppress_overdrawn']))
        self.put(K + 0x1F0, t['shortage_ratio']); self.put(K + 0x1F4, t['announced_ratio'])
        self.put(K + 0x334, t['snapshot_timer'])
        singles = case['singles']
        self.put(K + 0x214, SINGLES); self.put(K + 0x218, len(singles['items'])); self.put(K + 0x220, singles['charging'])
        for i, item in enumerate(singles['items']):
            a, ad = ITEMS + 0x40 * i, ACTORDATA + 0x400 * i
            self.put(SINGLES + 4 * i, a)
            self.put(a, 0xA29694); self.put(a + 4, ad); self.put8(a + 8, int(item['available']))
            self.put(a + 0xC, item['actor']); self.put(a + 0x10, item['organization'])
            self.put(a + 0x1C, item['progress']); self.put(a + 0x20, 0xFFFFFFFF)
            self.put(ad + 0x2CC, item['regeneration_rate'])
        faction = case['faction']
        if faction is None:
            self.put(K + 0x230, 0)
        else:
            self.put(K + 0x230, FACTION)
            self.put(FACTION + 4, FACTION_DEF); self.put(FACTION_DEF + 0x3C, faction['maximum'])
            owner = {'Kingdom': K, 'Other': K2, 'None': 0}[faction['owner']]
            self.put(FACTION + 8, owner); self.put(FACTION + 0xC, faction['balance'])
            if owner == K2:
                stats2 = case['faction_owner_statistics']
                self.put(K2 + 0x32C, STATS2 if stats2 is not None else 0)
                if stats2 is not None:
                    self.put_words(STATS2, stats2['production']); self.put_words(STATS2 + 0x28, stats2['scores'])
        stats = case['statistics']
        self.put(K + 0x32C, STATS if stats is not None else 0)
        if stats is not None:
            self.put_words(STATS, stats['production']); self.put_words(STATS + 0x28, stats['scores'])
        self.trace = []
        self.seam(0x6F7810, self._alloc)
        self.seam(0x6F7430, ret_pop(0))
        self.seam(0x4849F0, self._unexpected)
        self.seam(0x42D4B0, self._event)
        self.seam(0x4E9960, self._record('team_victory_check', 0))
        self.seam(0x4E9A40, self._record('percentage_victory_check', 0))
        self.seam(0x58F530, self._snapshot)
        self.seam(0x45F240, self._push)

    def _alloc(self, m, esp):
        return self.alloc(self.read(esp + 4)), 0

    def _unexpected(self, m, esp):
        raise AssertionError('vector growth path is not part of this supported domain')

    def _event(self, m, esp):
        manager, kind, kingdom = self.read(esp + 4), self.read(esp + 8), self.read(esp + 12)
        assert manager == MANAGER and kingdom == K, (hex(manager), hex(kingdom))
        if kind == 3:
            percent = self.read(esp + 16)
            self.trace.append({'kind': 'upkeep_shortage', 'percent': percent - (1 << 32) if percent >> 31 else percent})
        elif kind == 4:
            self.trace.append({'kind': 'upkeep_repaired'})
        else:
            raise AssertionError(kind)
        return None, 0

    def _record(self, name, pop):
        def handler(m, esp):
            from unicorn.x86_const import UC_X86_REG_ECX
            assert self.uc.reg_read(UC_X86_REG_ECX) == K
            self.trace.append({'kind': name})
            return None, pop
        return handler

    def _snapshot(self, m, esp):
        from unicorn.x86_const import UC_X86_REG_ECX
        stats, kingdom = self.read(esp + 4), self.read(esp + 8)
        assert stats == STATS and kingdom == K
        self.trace.append({'kind': 'statistics_snapshot'})
        return self.uc.reg_read(UC_X86_REG_ECX), 8

    def _push(self, m, esp):
        from unicorn.x86_const import UC_X86_REG_ECX
        assert self.uc.reg_read(UC_X86_REG_ECX) == K + 0x338
        return None, 4

    def tick(self, seconds, world_time):
        self.put(WORLD + 0xE0, world_time)
        self.trace = []
        self.ranges = CODE_RANGES
        self.visited = set()
        self.call(0x4E9C40, [seconds, 0x13579BDF], K + 0x180)
        for address in self.visited:
            assert any(s <= address < e for s, e in CODE_RANGES) or address in SEAMS, hex(address)
        return self.snapshot()

    def snapshot(self):
        n = self.count
        c = self.case
        out = {'economy': {f: self.words(ARR[f], n) for f in ARR},
               'tick': {'suppress_overdrawn': bool(self.read(K + 0x190)), 'shortage_ratio': self.read(K + 0x1F0),
                        'announced_ratio': self.read(K + 0x1F4), 'snapshot_timer': self.read(K + 0x334)},
               'singles': {'charging': self.read(K + 0x220) - (1 << 32) if self.read(K + 0x220) >> 31 else self.read(K + 0x220),
                           'items': []},
               'events': self.trace}
        for i, item in enumerate(c['singles']['items']):
            a = ITEMS + 0x40 * i
            out['singles']['items'].append({'available': bool(self.read8(a + 8)), 'actor': self.read(a + 0xC),
                                            'organization': self.read(a + 0x10), 'progress': self.read(a + 0x1C),
                                            'regeneration_rate': item['regeneration_rate']})
        if c['faction'] is not None:
            out['faction'] = dict(c['faction'], balance=self.read(FACTION + 0xC))
        else:
            out['faction'] = None
        out['statistics'] = None if c['statistics'] is None else {
            'production': self.words(STATS, len(c['statistics']['production'])),
            'scores': self.words(STATS + 0x28, SCORES)}
        o = c.get('faction_owner_statistics')
        out['faction_owner_statistics'] = None if o is None else {
            'production': self.words(STATS2, len(o['production'])), 'scores': self.words(STATS2 + 0x28, SCORES)}
        return out


R = random.Random(0x8E0C)
NICE = [0.0, 0.25, 0.5, 1.0, 1.5, 2.0, 3.0, 4.0, 5.0, 7.5, 10.0, 12.0, 20.0, 33.0, 100.0]


def fv(lo, hi, nice=0.3):
    if R.random() < nice:
        return bits(R.choice([x for x in NICE if lo <= x <= hi] or [lo]))
    return bits(R.uniform(lo, hi))


def make_case(name, n=None, *, force_negative=None, suppress=None, faction=None, stats=True,
              singles=None, ratio=None, announced=None, timer=None):
    n = n or R.randint(1, 6)
    kinds = [R.choice(['Stockpiled', 'Stockpiled', 'Upkeep', 'LimitedConsumed']) for _ in range(n)]
    if 'Stockpiled' not in kinds:
        kinds[0] = 'Stockpiled'
    catalog = {'resources': [{'id': f'r{i}', 'kind': k, 'limited_provider': k == 'Upkeep' and R.random() < 0.5,
                              'purchase_cost': [fv(0, 5) if kinds[j] == 'Stockpiled' else bits(0.0) for j in range(n)]}
                             for i, k in enumerate(kinds)]}
    eff = [fv(0, 60) for _ in range(n)]
    con = [fv(0, 80) for _ in range(n)]
    stock = [fv(0, 400) if R.random() < 0.7 else fv(0, 3) for _ in range(n)]
    if force_negative:
        i = kinds.index('Stockpiled')
        eff[i], con[i], stock[i] = bits(R.uniform(0, 10)), bits(R.uniform(30, 90)), bits(R.uniform(0, 0.4))
    raw = list(eff)
    economy = {'raw_provided': raw, 'effective_provided': eff, 'consumed': con, 'stockpiled': stock}
    gtid_table = {}
    items = []
    for i in range(singles if singles is not None else R.randint(0, 4)):
        def gtid(live):
            slot = R.randint(1, 900)
            generation = R.randint(1, 0x7FFF)
            if live == 'live':
                gtid_table[str(slot)] = (generation, True)
                return (generation << 16) | slot
            if live == 'stale':
                gtid_table[str(slot)] = (generation ^ 1, True)
                return (generation << 16) | slot
            gtid_table[str(slot)] = (generation, False)
            return (generation << 16) | slot
        a_state = R.choice(['live', 'stale', 'empty', 'empty'])
        o_state = R.choice(['live', 'stale', 'empty', 'empty'])
        progress = R.choice([fv(0, 0.99), bits(1.0), bits(R.uniform(0.9, 0.9999)), bits(0.0), bits(1.25)])
        items.append({'available': R.random() < 0.3, 'actor': gtid(a_state), 'organization': gtid(o_state),
                      'progress': progress, 'regeneration_rate': R.choice([fv(0, 0.2), bits(0.0), bits(1.0 / 30), bits(0.05)])})
    charging = R.choice([-1, -1] + list(range(len(items))) + [7])
    fac = None
    owner_stats = None
    if faction if faction is not None else R.random() < 0.7:
        owner = R.choice(['Kingdom', 'Kingdom', 'Other', 'None'])
        fac = {'balance': R.choice([fv(0, 500), bits(0.0), bits(-3.0)]),
               'maximum': R.choice([fv(0, 600), bits(-1.0), bits(100.0), bits(0.0)]), 'owner': owner}
        if owner == 'Other' and R.random() < 0.8:
            owner_stats = {'production': [fv(0, 1000) for _ in range(n)], 'scores': [fv(0, 50) for _ in range(SCORES)]}
    st = {'production': [fv(0, 1000) for _ in range(n)], 'scores': [fv(0, 50) for _ in range(SCORES)]} if stats else None
    tick = {'suppress_overdrawn': bool(suppress) if suppress is not None else R.random() < 0.2,
            'shortage_ratio': ratio if ratio is not None else R.choice([bits(0.0), fv(0, 1), bits(0.3)]),
            'announced_ratio': announced if announced is not None else R.choice([bits(0.0), fv(0, 1)]),
            'snapshot_timer': timer if timer is not None else R.choice([fv(0, 59.9), bits(59.5), bits(0.0), bits(61.0)])}
    return {'name': name, 'catalog': catalog, 'limits': {'provided_maximum': 100, 'stockpiled_maximum': bits(10000.0)},
            'settings': {'commander_experience_rate': R.choice([fv(0, 20), bits(0.0), bits(-2.0), bits(6.0)])},
            'economy': economy, 'gtid_table': gtid_table, 'tick': tick,
            'singles': {'charging': charging, 'items': items}, 'faction': fac,
            'faction_owner_statistics': owner_stats, 'statistics': st}


def steps_for(case):
    count = R.randint(1, 4)
    return [{'seconds': R.choice([bits(0.1), bits(0.25), bits(1.0), fv(0, 5), bits(0.0), bits(12.0)]),
             'world_time': R.choice([bits(0.5), bits(1.0), fv(1, 900), bits(0.999)])} for _ in range(count)]


def directed():
    out = []
    # Truncation of ratio*100 near integers and the 0.05 announce threshold.
    for target in [0.29, 0.57, 0.07, 0.999, 0.5, 0.123456]:
        c = make_case(f'ratio-{target}', 2, force_negative=True, suppress=False, singles=0, faction=False)
        c['catalog']['resources'] = [
            {'id': 'stock', 'kind': 'Stockpiled', 'limited_provider': False, 'purchase_cost': [bits(0.0), bits(0.0)]},
            {'id': 'other', 'kind': 'Stockpiled', 'limited_provider': False, 'purchase_cost': [bits(0.0), bits(0.0)]}]
        cons = 100.0
        c['economy'] = {'raw_provided': [bits(cons * (1 - target)), bits(0.0)],
                        'effective_provided': [bits(cons * (1 - target)), bits(0.0)],
                        'consumed': [bits(cons), bits(0.0)], 'stockpiled': [bits(0.01), bits(5.0)]}
        out.append((c, [{'seconds': bits(1.0), 'world_time': bits(2.0)}, {'seconds': bits(1.0), 'world_time': bits(2.0)}]))
    # Upkeep resource contributes min(provided, consumed)*cost to both sums.
    c = make_case('upkeep-ratio', 3, force_negative=True, suppress=False, singles=0)
    c['catalog']['resources'] = [
        {'id': 'oil', 'kind': 'Upkeep', 'limited_provider': True, 'purchase_cost': [bits(0.0), bits(2.5), bits(0.75)]},
        {'id': 'money', 'kind': 'Stockpiled', 'limited_provider': False, 'purchase_cost': [bits(0.0), bits(0.0), bits(0.0)]},
        {'id': 'steel', 'kind': 'Stockpiled', 'limited_provider': False, 'purchase_cost': [bits(0.0), bits(0.0), bits(0.0)]}]
    c['economy'] = {'raw_provided': [bits(40.0), bits(10.0), bits(3.0)], 'effective_provided': [bits(40.0), bits(10.0), bits(3.0)],
                    'consumed': [bits(55.0), bits(70.0), bits(1.0)], 'stockpiled': [bits(0.0), bits(0.2), bits(9.0)]}
    out.append((c, [{'seconds': bits(1.0), 'world_time': bits(5.0)}] * 3))
    # Repair event: previous ratio nonzero and no new overdraw.
    for prev in [bits(0.4), bits(0.0)]:
        c = make_case(f'repair-{prev:08x}', 2, suppress=False, singles=0, ratio=prev, announced=prev)
        c['economy']['stockpiled'] = [bits(5000.0), bits(5000.0)]
        out.append((c, [{'seconds': bits(1.0), 'world_time': bits(3.0)}]))
    # Suppressed overdraw: clamp without ratio events.
    c = make_case('suppressed', 3, force_negative=True, suppress=True, ratio=bits(0.6), announced=bits(0.6))
    out.append((c, [{'seconds': bits(2.0), 'world_time': bits(3.0)}]))
    # Snapshot period boundary and multiple snapshots.
    for timer in [bits(59.0), bits(59.9), bits(0.0)]:
        c = make_case(f'snapshot-{timer:08x}', 2, timer=timer)
        out.append((c, [{'seconds': bits(1.0), 'world_time': bits(2.0)}, {'seconds': bits(0.1), 'world_time': bits(2.0)},
                        {'seconds': bits(60.0), 'world_time': bits(2.0)}]))
    # Recruit singles sequencing: index-0 quirk and one-at-a-time charging.
    c = make_case('singles-sequence', 2, singles=0)
    c['gtid_table'] = {'5': (3, False), '6': (4, True), '7': (9, False)}
    c['singles'] = {'charging': -1, 'items': [
        {'available': False, 'actor': (3 << 16) | 5, 'organization': (9 << 16) | 7, 'progress': bits(1.0), 'regeneration_rate': bits(0.1)},
        {'available': False, 'actor': (3 << 16) | 5, 'organization': 0, 'progress': bits(0.5), 'regeneration_rate': bits(0.2)},
        {'available': False, 'actor': (4 << 16) | 6, 'organization': 0, 'progress': bits(0.0), 'regeneration_rate': bits(0.3)},
        {'available': True, 'actor': 0, 'organization': 0, 'progress': bits(0.25), 'regeneration_rate': bits(0.25)},
        {'available': False, 'actor': 0, 'organization': 0, 'progress': bits(1.0), 'regeneration_rate': bits(0.5)}]}
    c['gtid_table']['0'] = (0, False)
    out.append((c, [{'seconds': bits(1.0), 'world_time': bits(2.0)}] * 6))
    # Faction XP clamps and owner statistics.
    for owner, maximum, balance, rate in [('Kingdom', bits(10.0), bits(9.9), bits(6.0)), ('Other', bits(-1.0), bits(5.0), bits(6.0)),
                                          ('Kingdom', bits(100.0), bits(0.05), bits(-6.0)), ('None', bits(50.0), bits(1.0), bits(3.0))]:
        c = make_case(f'faction-{owner}-{maximum:08x}', 2, faction=True)
        c['faction'] = {'balance': balance, 'maximum': maximum, 'owner': owner}
        c['settings']['commander_experience_rate'] = rate
        c['faction_owner_statistics'] = ({'production': [bits(1.0)] * 2, 'scores': [bits(2.0)] * SCORES} if owner == 'Other' else None)
        out.append((c, [{'seconds': bits(1.0), 'world_time': bits(0.5)}, {'seconds': bits(3.0), 'world_time': bits(1.0)}]))
    return out


def main():
    cases = directed() + [(make_case(f'random-{i}'), None) for i in range(120)]
    fixtures = []
    executions = 0
    for case, steps in cases:
        steps = steps or steps_for(case)
        for mode, cw in MODES:
            machine = TickMachine(cw, case)
            initial = machine.snapshot()
            results = []
            for step in steps:
                results.append({'input': step, 'expected': machine.tick(step['seconds'], step['world_time'])})
                executions += 1
            fixtures.append({'name': case['name'], 'precision': mode, 'case': case, 'initial': initial, 'steps': results})
    doc = {'schema': 1, 'function': '0x4E9C40 complete GKingdom economy Tick', 'pins': pin(RANGES),
           'seams': {f'0x{a:06X}': n for a, n in SEAMS.items()}, 'executions': executions, 'fixtures': fixtures}
    out = Path(__file__).resolve().parent / 'native-tick-fixtures.json'
    out.write_text(json.dumps(doc, separators=(',', ':')) + '\n', encoding='utf-8')
    target = ROOT / 'crates/aa-economy/tests/fixtures/native-tick-fixtures.json'
    shutil.copyfile(out, target)
    events = sum(len(s['expected']['events']) for f in fixtures for s in f['steps'])
    print(f'{len(fixtures)} fixtures, {executions} native tick executions, {events} recorded seam events')


if __name__ == '__main__':
    main()
