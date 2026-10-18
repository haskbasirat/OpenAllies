"""Original GKingdom::AddProperty 0x4E3AD0 in isolated x86.

Executed unchanged: 4E3AD0, 5036D0 property state add, 5030F0 record
constructor, 5032D0 container search, 5031E0 origin resolve, 766DE0 vector
append. Seams (recorded): property definition virtual +0x18 (technology),
4E1890 allowed lists, 404F60 log + kingdom name virtual, 4CEF10 property
application to each kingdom actor, 42D4B0 event 5, allocator.
"""
from pathlib import Path
import json, random, sys
sys.path.insert(0, str(Path(__file__).resolve().parent))
from harness import Machine, MODES, bits, pin, ret_pop  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX  # noqa: E402

RANGES = {'add_property_4E3AD0': (0x4E3AD0, 0x4E3C22), 'state_add_5036D0': (0x5036D0, 0x503736),
          'record_5030F0': (0x5030F0, 0x50313B), 'search_5032D0': (0x5032D0, 0x50334A),
          'origin_5031E0': (0x5031E0, 0x503206), 'append_766DE0': (0x766DE0, 0x766E37),
          'remove_property_4E23D0': (0x4E23D0, 0x4E2449), 'state_remove_5034F0': (0x5034F0, 0x5035DC)}
K, KVT, WORLD, GT = 0x2000000, 0x2000800, 0x2001000, 0x2100000
DEFS, PDVT, ACTORS, ORIGINS = 0x2010000, 0x2011000, 0x2020000, 0x2030000
STUB_TECH, STUB_NAME = 0x3000100, 0x3000110


def D(d): return DEFS + d * 0x80
def A(i): return ACTORS + i * 0x100


class PropertyMachine(Machine):
    def __init__(self, cw, case):
        super().__init__(cw)
        self.case = case
        w = case['world']
        self.put(K, KVT); self.put(KVT + 0x24, STUB_NAME)
        self.put(0xB36534, WORLD); self.put(WORLD + 0xE0, w['time'])
        self.put(0xB364C0, GT)
        for slot in range(1, 6):
            self.put16(GT + 4 + 2 * slot, 3); self.put(GT + 0x20004 + 4 * slot, ORIGINS + slot * 0x40 if slot != 5 else 0)
            self.put(ORIGINS + slot * 0x40 + 0x14, (3 << 16) | slot)
        self.put(PDVT + 0x18, STUB_TECH)
        for d in range(1, 16):
            self.put(D(d), PDVT); self.put(D(d) + 0x10, 0x2F00000 + d); self.put8(D(d) + 0x5C, int(d in w['restricted']))
        nodes = [self.alloc(8) for _ in w['actors']]
        for i, (node, a) in enumerate(zip(nodes, w['actors'])):
            self.put(node, A(a)); self.put(node + 4, nodes[i + 1] if i + 1 < len(nodes) else 0)
        self.put(K + 0x284, nodes[0] if nodes else 0)
        records = case['records']
        data = self.alloc(4 * max(4, len(records)))
        for i, r in enumerate(records):
            rec = self.alloc(0x18)
            self.put(rec, 0xA29678); self.put(rec + 4, r['time']); self.put(rec + 8, r['source'])
            self.put(rec + 0xC, D(r['definition'])); self.put8(rec + 0x10, int(r['duplicate'])); self.put(rec + 0x14, r['origin'])
            self.put(data + 4 * i, rec)
        self.put(K + 0x318, data); self.put(K + 0x31C, len(records)); self.put(K + 0x320, max(4, len(records)))
        self.trace = []
        self.seam(0x6F7810, lambda m, esp: (self.alloc(self.read(esp + 4)), 0))
        self.seam(0x6F7430, ret_pop(0))
        self.seam(STUB_TECH, self._tech)
        self.seam(STUB_NAME, self._name)
        self.seam(0x4E1890, self._allowed)
        self.seam(0x404F60, self._log)
        self.seam(0x4CEF10, self._apply)
        self.seam(0x42D4B0, self._event)
        self.seam(0x4CEF70, self._remove_effect)
        self.seam(0x404F90, self._log_missing)

    def d(self, p):
        return (p - DEFS) // 0x80

    def _tech(self, m, esp):
        d = self.d(self.uc.reg_read(UC_X86_REG_ECX))
        self.trace.append({'kind': 'is_technology', 'definition': d})
        return int(d in self.case['world']['technologies']), 0

    def _name(self, m, esp):
        out = self.read(esp + 4)
        h = self.alloc(0x40); self.put(h + 8, 16); self.put(h + 0xC, 100); self.put(out, h + 0x10)
        return out, 4

    def _allowed(self, m, esp):
        d = self.d(self.read(esp + 4))
        self.trace.append({'kind': 'technology_allowed', 'definition': d})
        return int(d in self.case['world']['allowed']), 4

    def _log(self, m, esp):
        self.trace.append({'kind': 'log_not_allowed'})
        return None, 0

    def _apply(self, m, esp):
        actor = (self.uc.reg_read(UC_X86_REG_ECX) - ACTORS) // 0x100
        origin = self.read(esp + 12)
        self.trace.append({'kind': 'apply_property', 'actor': actor, 'definition': self.d(self.read(esp + 4)),
                           'source': self.read(esp + 8), 'origin': (origin - ORIGINS) // 0x40 if origin else None})
        return None, 12

    def _event(self, m, esp):
        kind = self.read(esp + 8)
        assert kind in (5, 6) and self.read(esp + 12) == K
        self.trace.append({'kind': {5: 'technology_gained', 6: 'technology_lost'}[kind],
                           'definition': self.read(esp + 16) - 0x2F00000})
        return None, 0

    def _remove_effect(self, m, esp):
        actor = (self.uc.reg_read(UC_X86_REG_ECX) - ACTORS) // 0x100
        origin = self.read(esp + 8)
        self.trace.append({'kind': 'remove_property_effect', 'actor': actor, 'definition': self.d(self.read(esp + 4)),
                           'origin': (origin - ORIGINS) // 0x40 if origin else None})
        return None, 8

    def _log_missing(self, m, esp):
        self.trace.append({'kind': 'log_missing_property'})
        return None, 0

    def records(self):
        data, n = self.read(K + 0x318), self.read(K + 0x31C)
        out = []
        for i in range(n):
            rec = self.read(data + 4 * i)
            out.append({'time': self.read(rec + 4), 'source': self.read(rec + 8), 'definition': self.d(self.read(rec + 0xC)),
                        'duplicate': bool(self.read8(rec + 0x10)), 'origin': self.read(rec + 0x14)})
        return out

    def run(self, op):
        self.trace = []
        origin = ORIGINS + op['origin'] * 0x40 if op['origin'] else 0
        if op.get('remove'):
            self.call(0x4E23D0, [D(op['definition']), origin], K)
        else:
            self.call(0x4E3AD0, [D(op['definition']), op['source'], origin, int(op['notify'])], K)
        return {'records': self.records(), 'events': self.trace}


R = random.Random(0x4E3AD0)


def make_case(i):
    pool = list(range(1, 12))
    w = {'time': bits(R.uniform(0, 900)), 'technologies': R.sample(pool, R.randint(0, 11)),
         'restricted': R.sample(pool, R.randint(0, 11)), 'allowed': R.sample(pool, R.randint(0, 11)),
         'actors': R.sample(range(6), R.randint(0, 6))}
    records = [{'time': bits(R.uniform(0, 100)), 'source': R.choice([0, 2, 8]), 'definition': R.choice(pool),
                'duplicate': R.random() < 0.2, 'origin': 0} for _ in range(R.randint(0, 5))]
    ops = [{'definition': R.choice(pool), 'source': R.choice([2, 0, 7]), 'origin': R.choice([0, 0, 1, 2, 5]),
            'notify': R.random() < 0.5, 'remove': R.random() < 0.35} for _ in range(R.randint(2, 8))]
    return {'name': f'random-{i}', 'world': w, 'records': records, 'ops': ops}


def main():
    cases = [make_case(i) for i in range(120)]
    fixtures, executions = [], 0
    for case in cases:
        for mode, cw in MODES:
            m = PropertyMachine(cw, case)
            initial = m.records()
            steps = [{'op': op, 'expected': m.run(op)} for op in case['ops']]
            executions += len(steps)
            fixtures.append({'name': case['name'], 'precision': mode, 'case': case, 'initial': initial, 'steps': steps})
    doc = {'schema': 1, 'pins': pin(RANGES), 'executions': executions, 'fixtures': fixtures}
    out = Path(__file__).resolve().parent / 'native-property-fixtures.json'
    out.write_text(json.dumps(doc, separators=(',', ':')) + '\n', encoding='utf-8')
    print(f'{len(fixtures)} fixtures, {executions} native executions')


if __name__ == '__main__':
    main()
