"""Original GKingdom start-mission setup 0x4E6DB0 in isolated x86.

Executed unchanged: 4E6DB0, 4E39D0 credit, 4730D0 nation faction test,
4CDF10 Commander experience set. Seams (recorded): 4E3AD0 AddProperty (ported
in aa-production research.rs, proven by native_properties.py), 5032D0 owned
property search, 48D040 profile technology lookup, 4E4BC0 diagnostic, kingdom
virtual +0x24 name, string helpers.
"""
from pathlib import Path
import json, random, shutil, sys
sys.path.insert(0, str(Path(__file__).resolve().parent))
from harness import Machine, MODES, ROOT, bits, pin, ret_pop  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX  # noqa: E402

RANGES = {'start_mission_4E6DB0': (0x4E6DB0, 0x4E704B), 'credit_4E39D0': (0x4E39D0, 0x4E3A4C),
          'faction_4730D0': (0x4730D0, 0x473107), 'commander_xp_4CDF10': (0x4CDF10, 0x4CDF59)}
K, KVT, STOCK, CACHE = 0x2000000, 0x2000800, 0x2001000, 0x2002000
CMD, CDEF, PROFILE, NATION = 0x2003000, 0x2003400, 0x2003800, 0x2003C00
VALUES, DEFS = 0x2004000, 0x2010000
STUB_NAME = 0x3000100


def D(d): return DEFS + 0x100 * d


class StartMachine(Machine):
    def __init__(self, cw, case):
        super().__init__(cw)
        self.case = case
        n = len(case['stock'])
        self.n = n
        self.put(K, KVT); self.put(KVT + 0x24, STUB_NAME)
        self.put(K + 0x1C4, STOCK); self.put(K + 0x1C8, n); self.put(K + 0x1CC, n)
        self.put_words(STOCK, case['stock'])
        self.put(0xB39AB0, CACHE)
        for off, value in ((0x110, case['starting_stockpile']), (0x1B8, case['maximum']), (0x134, case['commander_start'])):
            p = self.alloc(4); self.put(p, value); self.put(CACHE + off, p)
        self.put8(K + 0x23A, int(case['reset']))
        self.put(K + 0x19C, 1); self.put8(K + 0x23B, 0)
        self.vector(K + 0x258, case['starting_resources'])
        c = case['commander']
        if c is not None:
            self.put(K + 0x230, CMD); self.put(CMD + 4, CDEF)
            self.put(CMD + 0xC, c['balance']); self.put(CDEF + 0x3C, c['maximum'])
            nodes = [self.alloc(8) for _ in c['technologies']]
            for i, (node, d) in enumerate(zip(nodes, c['technologies'])):
                self.put(node, D(d)); self.put(node + 4, nodes[i + 1] if i + 1 < len(nodes) else 0)
            self.put(CDEF + 0x30, nodes[0] if nodes else 0)
            self.commander_definition = CDEF
        p = case['profile']
        if p is not None:
            self.put(K + 0x244, PROFILE)
            self.vector(PROFILE + 0x20, p['resources'])
        self.vector(K + 0x24C, [D(d) if d else 0 for d in case['start_technologies']])
        nat = case['nation']
        if nat is not None:
            self.put(K + 0x234, NATION)
            data = self.alloc(4 * max(1, len(nat)))
            self.put_words(data, [CDEF if d == 'commander' else D(d) for d in nat])
            self.put(NATION + 0x48, data); self.put(NATION + 0x4C, len(nat))
        self.trace = []
        self.seam(0x4E3AD0, self._add)
        self.seam(0x5032D0, self._has)
        self.seam(0x48D040, self._profile_technology)
        self.seam(0x4E4BC0, self._invalid)
        self.seam(STUB_NAME, lambda m, esp: (self._fill(self.read(esp + 4)), 4))
        self.seam(0x401B00, lambda m, esp: (self._fill(self.uc.reg_read(UC_X86_REG_ECX)), 4))
        self.seam(0x4B9F40, lambda m, esp: (self._fill(self.read(esp + 4)), 8))

    def vector(self, at, values):
        data = self.alloc(4 * max(1, len(values)))
        self.put_words(data, values)
        self.put(at, data); self.put(at + 4, len(values)); self.put(at + 8, len(values))

    def _fill(self, out):
        h = self.alloc(0x40)
        self.put(h + 0xC, 100)
        self.put(out, h + 0x10)
        return out

    def ident(self, p):
        return (p - DEFS) // 0x100

    def _add(self, m, esp):
        assert self.uc.reg_read(UC_X86_REG_ECX) == K
        assert self.read(esp + 8) == 2 and self.read(esp + 12) == K and self.read(esp + 16) == 1
        d = self.ident(self.read(esp + 4))
        self.trace.append({'kind': 'add_property', 'definition': d})
        self.case['owned'].append(d)
        return None, 16

    def _has(self, m, esp):
        assert self.read(esp + 4) == K + 0x318 and self.read(esp + 12) == 0 and self.read(esp + 16) == 0
        d = self.ident(self.read(esp + 8))
        self.trace.append({'kind': 'has_property', 'definition': d})
        return int(d in self.case['owned']), 0

    def _profile_technology(self, m, esp):
        assert self.uc.reg_read(UC_X86_REG_ECX) == PROFILE
        t = self.case['profile']['technology']
        return (D(t) if t else 0), 0

    def _invalid(self, m, esp):
        self.trace.append({'kind': 'invalid_faction'})
        return None, 0

    def run(self):
        self.call(0x4E6DB0, [], K)
        c = self.read(CMD + 0xC) if self.case['commander'] is not None else None
        return {'stock': self.words(STOCK, self.n), 'commander_balance': c, 'status': self.read(K + 0x19C),
                'started': bool(self.read8(K + 0x23B)), 'events': self.trace}


R = random.Random(0x4E6DB0)


def money():
    return bits(R.choice([0.0, 10.0, 250.0, 9999.0, 12000.0, -5.0, R.uniform(0, 3000)]))


def make_case(i):
    n = R.randint(1, 4)
    defs = list(range(1, 9))
    commander = None
    if R.random() < 0.7:
        commander = {'balance': bits(R.uniform(0, 50)),
                     'maximum': bits(R.choice([-1.0, 0.0, 25.0, 100.0, R.uniform(0, 200)])),
                     'technologies': R.sample(defs, R.randint(0, 3))}
    nation = None
    if R.random() < 0.6:
        nation = R.sample(defs, R.randint(0, 3)) + (['commander'] if R.random() < 0.5 else [])
    return {'name': f'random-{i}', 'stock': [money() for _ in range(n)],
            'starting_stockpile': bits(R.choice([0.0, -3.0, 100.0, 500.0, 20000.0, R.uniform(0, 1000)])),
            'maximum': bits(R.choice([10000.0, 300.0, 99999.0])),
            'commander_start': bits(R.choice([0.0, -10.0, 30.0, 150.0, R.uniform(0, 300)])),
            'reset': R.random() < 0.5, 'starting_resources': [money() for _ in range(n)],
            'commander': commander,
            'profile': ({'resources': [money() for _ in range(n)], 'technology': R.choice([0] + defs)}
                        if R.random() < 0.5 else None),
            'start_technologies': [R.choice([0] + defs) for _ in range(R.randint(0, 4))],
            'nation': nation, 'owned': R.sample(defs, R.randint(0, 3))}


def directed():
    nan_case = make_case(-1)
    nan_case['name'] = 'nan-and-infinite-values'
    nan_case['reset'] = True
    nan_case['starting_stockpile'] = 0x7FC00000
    nan_case['commander'] = {'balance': 0, 'maximum': 0x7FC00000, 'technologies': [1, 1]}
    nan_case['commander_start'] = bits(50.0)
    inf_case = json.loads(json.dumps(nan_case))
    inf_case['name'] = 'infinite-start-stockpile'
    inf_case['starting_stockpile'] = 0x7F800000
    inf_case['commander_start'] = 0xFFC00000
    return [nan_case, inf_case]


def main():
    cases = directed() + [make_case(i) for i in range(120)]
    fixtures, executions = [], 0
    for case in cases:
        for mode, cw in MODES:
            m = StartMachine(cw, json.loads(json.dumps(case)))
            fixtures.append({'name': case['name'], 'precision': mode, 'case': case, 'expected': m.run()})
            executions += 1
    doc = {'schema': 1, 'pins': pin(RANGES), 'executions': executions, 'fixtures': fixtures}
    out = Path(__file__).resolve().parent / 'native-start-mission-fixtures.json'
    out.write_text(json.dumps(doc, separators=(',', ':')) + '\n', encoding='utf-8')
    shutil.copyfile(out, ROOT / 'crates/aa-economy/tests/fixtures/native-start-mission-fixtures.json')
    print(f'{len(fixtures)} fixtures, {executions} native executions')


if __name__ == '__main__':
    main()
