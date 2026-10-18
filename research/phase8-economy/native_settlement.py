"""Original SettlementComponent structure building in isolated x86.

Executed unchanged: 587B60 CanBuildStructure, 5879B0 affordability, 587FD0
BuildStructure, 5657A0/5655C0 paid-cost store (+50B530 resize), 4E20A0,
4E1690, 529C30, 4C8C70/4C8C20 creation parameters, 432730 shape, list walks. Seams (recorded): actor virtuals
+0x34 state / +0x108 owner / +0x24 name, 4E1910 restrictions, 554DD0/554330 tile iteration (fixture tiles),
54EFC0 enemy tile test, 4F0B70 zone owner, 585500 settlement structure test,
404F30 logs, 40C230 messages and CString helpers, 45F4D0 cost quotes, 4CF110,
4F9C10 actor creation, 586B40 settlement registration, allocator.
"""
from pathlib import Path
import json, random, sys
sys.path.insert(0, str(Path(__file__).resolve().parent))
from harness import Machine, MODES, bits, pin, ret_pop  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX  # noqa: E402

RANGES = {'can_build_587B60': (0x587B60, 0x587FC7), 'affordable_5879B0': (0x5879B0, 0x587B5A),
          'build_587FD0': (0x587FD0, 0x5882AD), 'paid_store_5657A0': (0x5657A0, 0x5657AF),
          'paid_5655C0': (0x5655C0, 0x5656AE), 'can_afford_4E20A0': (0x4E20A0, 0x4E20F7),
          'debit_4E1690': (0x4E1690, 0x4E16C5)}
K, STOCK = 0x2000000, 0x2010000
REG, TABLE, RDEFS, MANAGER = 0x2020000, 0x2020100, 0x2021000, 0x2022000
CACHE, LIMITS = 0x2024000, 0x2024800
SA, SVT, SAD, SC, LIMTAB = 0x2030000, 0x2030800, 0x2031000, 0x2032000, 0x2032800
WORLD, OTHER = 0x2033000, 0x2034000
DEFS = 0x2040000
CREATED = 0x2050000
ALLOCATOR, AVT = 0x2060000, 0x2060100
FAKEMAP, FAKETILES = 0x2070000, 0x2080000
STUB_STATE, STUB_OWNER, STUB_NAME, STUB_NEWSTR = 0x3000100, 0x3000110, 0x3000120, 0x3000130
MESSAGES = {0x587E70: 'notify_enemies_nearby', 0x587DE7: 'notify_within_zone',
            0x587AC2: 'notify_settlement_unaffordable', 0x5881E8: 'notify_creation_failed'}
LOGS = {0xA3F2B8: 'MaxStructures', 0xA3F210: 'CannotBuild', 0xA3F1C0: 'AlreadyHas'}


def D(d): return DEFS + d * 0x100
def CA(k): return CREATED + k * 0x400


class SettlementMachine(Machine):
    def __init__(self, cw, case):
        super().__init__(cw)
        self.case = case
        w = case['world']
        n = len(case['catalog']['resources'])
        self.n = n
        self.put(K + 0x1C4, STOCK); self.put(K + 0x1C8, n); self.put(K + 0x1CC, n)
        self.put_words(STOCK, case['stock'])
        self.put(0xB36530, REG); self.put(REG + 0x34, TABLE); self.put(REG + 0x38, n)
        self.put(0xB3A660, MANAGER); self.put(MANAGER + 0x28, n)
        for i, d in enumerate(case['catalog']['resources']):
            self.put(TABLE + 4 * i, RDEFS + 0x40 * i)
            self.put(RDEFS + 0x40 * i + 0x18, {'Stockpiled': 0, 'Upkeep': 1, 'LimitedConsumed': 2}[d['kind']])
        self.put(0xB39AB0, CACHE); self.put(CACHE + 0x1B8, LIMITS); self.put(LIMITS, case['limits']['stockpiled_maximum'])
        self.put(0xB36534, WORLD)
        self.put(SA, SVT); self.put(SVT + 0x34, STUB_STATE); self.put(SVT + 0x108, STUB_OWNER); self.put(SVT + 0x24, STUB_NAME)
        self.put(SA + 4, SAD); self.put(SA + 0x14, 0x50001)
        nodes = [self.alloc(8) for _ in w['buildable']]
        for i, (node, d) in enumerate(zip(nodes, w['buildable'])):
            self.put(node, D(d)); self.put(node + 4, nodes[i + 1] if i + 1 < len(nodes) else 0)
        self.put(SAD + 0x45C, nodes[0] if nodes else 0)
        self.put(SAD + 0x464, LIMTAB)
        for level, m in enumerate(w['limits']):
            self.put(LIMTAB + 12 * level, m)
        self.put(SC + 4, SA); self.put(SC + 0x10, w['level']); self.put(SC + 0x1C, w['count'])
        for d in range(1, 9):
            self.put(D(d) + 0x174, w['definition_flags'][str(d)]); self.put(D(d) + 0x10, 0x2F00000 + d)
        self.put(ALLOCATOR, AVT); self.put(AVT + 0xC, STUB_NEWSTR); self.put(AVT + 4, 0x3000140)
        self.put(FAKEMAP + 0x1C, 1); self.put(FAKEMAP + 0x28, FAKETILES)
        self.quotes = {d: list(q) for d, q in case['quotes'].items()}
        self.created = 0
        self.trace = []
        self.seam(0x6F7810, lambda m, esp: (self.alloc(self.read(esp + 4)), 0))
        self.seam(0x6F7430, ret_pop(0))
        self.seam(STUB_STATE, self._state)
        self.seam(STUB_OWNER, self._owner)
        self.seam(STUB_NAME, lambda m, esp: (self._fill(self.read(esp + 4)), 4))
        self.seam(STUB_NEWSTR, lambda m, esp: (self._header(1), 0))
        self.seam(0x3000140, ret_pop(1))
        self.seam(0x4E1910, self._allows)
        self.seam(0x554DD0, self._iterate)
        self.seam(0x554330, self._advance)
        self.seam(0x54EFC0, self._enemy)
        self.seam(0x4F0B70, self._zone)
        self.seam(0x585500, self._has_structure)
        self.seam(0x404F30, self._log)
        self.seam(0x40C230, self._message)
        self.seam(0x401B00, lambda m, esp: (self._fill(self.uc.reg_read(UC_X86_REG_ECX)), 4))
        self.seam(0x4B9F40, lambda m, esp: (self._fill(self.read(esp + 4)), 8))
        self.seam(0x5C4D10, lambda m, esp: (self._fill(self.read(esp + 4)), 0))
        self.seam(0x4012C0, lambda m, esp: (self.read(esp + 4), 0))
        self.seam(0x401560, self._assign)
        self.seam(0x856370, lambda m, esp: (ALLOCATOR, 0))
        self.seam(0x45F4D0, self._quote)
        self.seam(0x4CF110, lambda m, esp: (0x4D4D0000, 0))
        self.seam(0x4F9C10, self._create)
        self.seam(0x586B40, self._add_structure)

    def _code(self, uc, address, size, _data):
        if address == 0x4E1690:
            from unicorn.x86_const import UC_X86_REG_ESP
            vec = self.read(uc.reg_read(UC_X86_REG_ESP) + 4)
            self.trace.append({'kind': 'debit', 'values': self.words(self.read(vec), self.n)})
        return Machine._code(self, uc, address, size, _data)

    def _header(self, refcount=100):
        h = self.alloc(0x40)
        self.put(h, ALLOCATOR); self.put(h + 8, 16); self.put(h + 0xC, refcount)
        return h

    def _fill(self, out):
        self.put(out, self._header() + 0x10)
        return out

    def _assign(self, m, esp):
        this = self.uc.reg_read(UC_X86_REG_ECX)
        self.put(this, self.read(self.read(esp + 4)))
        return this, 4

    def d(self, p):
        return (p - DEFS) // 0x100

    def _state(self, m, esp):
        self.trace.append({'kind': 'state_active', 'state': self.read(esp + 4)})
        return int(self.case['world']['state_active']), 4

    def _owner(self, m, esp):
        self.trace.append({'kind': 'kingdom'})
        return (K if self.case['world']['kingdom'] else 0), 0

    def _allows(self, m, esp):
        assert self.uc.reg_read(UC_X86_REG_ECX) == K and self.read(esp + 4) == SA
        self.trace.append({'kind': 'kingdom_allows', 'definition': self.d(self.read(esp + 8))})
        return int(self.case['world']['kingdom_allows']), 8

    def _iterate(self, m, esp):
        it = self.uc.reg_read(UC_X86_REG_ECX)
        n = len(self.op['tiles'])
        self.trace.append({'kind': 'enemies_nearby'})
        self.put(it, FAKEMAP); self.put(it + 0x10, n - 1); self.put(it + 0x24, 0); self.put(it + 0x28, 0)
        return it, 16

    def _advance(self, m, esp):
        it = self.uc.reg_read(UC_X86_REG_ECX)
        self.put(it + 0x28, self.read(it + 0x28) + 1)
        return None, 4

    def _enemy(self, m, esp):
        tile = (self.uc.reg_read(UC_X86_REG_ECX) - FAKETILES) // 48
        assert self.read(esp + 8) == 5
        return int(self.op['tiles'][tile]), 8

    def _zone(self, m, esp):
        out = self.read(esp + 8)
        self.trace.append({'kind': 'zone_is_own'})
        self.put(out, SA if self.op['own_zone'] else OTHER)
        return out, 8

    def _has_structure(self, m, esp):
        assert self.uc.reg_read(UC_X86_REG_ECX) == SC and self.read(esp + 8) == 0 and self.read(esp + 12) == 1
        d = self.d(self.read(esp + 4))
        self.trace.append({'kind': 'settlement_has_structure', 'definition': d})
        return int(d in self.case['world']['has']), 12

    def _log(self, m, esp):
        fmt = self.read(esp + 20)
        self.trace.append({'kind': 'settlement_log', 'log': LOGS[fmt]})
        return None, 0

    def _message(self, m, esp):
        self.trace.append({'kind': MESSAGES[self.read(esp)], 'kingdom': self.read(esp + 16) != 0})
        return None, 0

    def _quote(self, m, esp):
        d = self.d(self.uc.reg_read(UC_X86_REG_ECX))
        out = self.read(esp + 4)
        q = self.quotes[str(d)]
        values = q.pop(0) if len(q) > 1 else q[0]
        data = self.alloc(4 * max(1, len(values)))
        self.put_words(data, values)
        self.put(out, data); self.put(out + 4, len(values)); self.put(out + 8, len(values))
        self.trace.append({'kind': 'quote', 'definition': d, 'values': values})
        return out, 8

    def _create(self, m, esp):
        params = self.read(esp + 4)
        self.trace.append({'kind': 'create_structure', 'definition': self.d(self.read(params + 4)),
                           'x': self.read(params + 8), 'y': self.read(params + 0xC), 'owner': self.read(params + 0x28) != 0})
        if not self.op['created']:
            return 0, 4
        k = self.created
        self.created += 1
        p = CA(k)
        if self.op['component']:
            comp = p + 0x200
            self.put(p + 0x9C, comp)
            data = self.alloc(4 * self.n)
            self.put(comp + 0x10, data); self.put(comp + 0x14, self.n); self.put(comp + 0x18, self.n)
        else:
            self.put(p + 0x9C, 0)
        return p, 4

    def _add_structure(self, m, esp):
        self.trace.append({'kind': 'settlement_add_structure', 'created': (self.read(esp + 4) - CREATED) // 0x400})
        return None, 4

    def run(self, op):
        self.op = op
        self.trace = []
        self.case['world'].update(op.get('world', {}))
        kind = op['kind']
        result = None
        d = D(op['definition']) if op['definition'] else 0
        if kind == 'can_build':
            eax, _ = self.call(0x587B60, [d, op['x'], op['y'], int(op['check_cost']), int(op['check_location']),
                                          int(op['notify'])], SC)
            result = bool(eax & 0xFF)
        elif kind == 'affordable':
            eax, _ = self.call(0x5879B0, [d, int(op['notify'])], SC)
            result = bool(eax & 0xFF)
        elif kind == 'build':
            before = self.created
            self.call(0x587FD0, [d, op['x'], op['y']], SC)
            if self.created > before:
                p = CA(before)
                comp = self.read(p + 0x9C)
                result = {'created': before,
                          'paid': self.words(self.read(comp + 0x10), self.read(comp + 0x14)) if comp else None}
        return {'stock': self.words(STOCK, self.n), 'events': self.trace, 'result': result}


R = random.Random(0x587FD0)
STRUCT = 1 << 13


def make_case(i):
    n = R.randint(1, 4)
    kinds = [R.choice(['Stockpiled', 'Stockpiled', 'Upkeep']) for _ in range(n)]
    if 'Stockpiled' not in kinds:
        kinds[0] = 'Stockpiled'

    def cost():
        return [bits(R.choice([0.0, 10.0, 25.0, 60.0, R.uniform(0, 120)])) for _ in range(n)]
    defs = list(range(1, 9))
    world = {'state_active': R.random() < 0.9, 'kingdom': True, 'kingdom_allows': R.random() < 0.9,
             'buildable': R.sample(defs, R.randint(2, 7)), 'has': R.sample(defs, R.randint(0, 3)),
             'level': R.randrange(3), 'limits': [R.randint(0, 6) for _ in range(3)], 'count': R.randint(0, 5),
             'definition_flags': {str(d): R.choice([STRUCT, STRUCT, STRUCT, 0, STRUCT | 4]) for d in defs}}
    ops = []
    for _ in range(R.randint(3, 8)):
        k = R.choice(['can_build', 'can_build', 'affordable', 'build', 'build', 'build'])
        op = {'kind': k, 'definition': R.choice(defs + [0]) if k != 'build' else R.choice(defs),
              'x': bits(R.uniform(0, 500)), 'y': bits(R.uniform(0, 500)),
              'tiles': [R.random() < 0.15 for _ in range(R.randint(0, 4))], 'own_zone': R.random() < 0.85,
              'created': R.random() < 0.85, 'component': R.random() < 0.8,
              'check_cost': R.random() < 0.7, 'check_location': R.random() < 0.7, 'notify': R.random() < 0.6}
        if R.random() < 0.2:
            op['world'] = {'kingdom': R.random() < 0.5}
        ops.append(op)
    return {'name': f'random-{i}', 'catalog': {'resources': [{'id': f'r{j}', 'kind': kk, 'limited_provider': False,
                                                              'purchase_cost': [0] * n} for j, kk in enumerate(kinds)]},
            'limits': {'provided_maximum': 100, 'stockpiled_maximum': bits(10000.0)},
            'stock': [bits(R.choice([30.0, 150.0, 500.0, R.uniform(0, 300)])) for _ in range(n)],
            'quotes': {str(d): [cost() for _ in range(R.randint(1, 3))] for d in defs},
            'world': world, 'ops': ops}


def reaches_owner_deref(world, op):
    """0x4E1910 dereferences the owner kingdom when the checks reach it."""
    if op['kind'] == 'affordable' or not world['state_active']:
        return False
    d = op['definition']
    if not d or not (world['definition_flags'][str(d)] & STRUCT):
        return False
    return world['count'] < world['limits'][world['level']]


def main():
    cases = [make_case(i) for i in range(100)]
    fixtures, executions = [], 0
    for case in cases:
        for mode, cw in MODES:
            m = SettlementMachine(cw, json.loads(json.dumps(case)))
            steps = []
            for op in case['ops']:
                w = dict(m.case['world'], **op.get('world', {}))
                if not w['kingdom'] and reaches_owner_deref(w, op):
                    continue
                steps.append({'op': op, 'expected': m.run(op)})
                executions += 1
            fixtures.append({'name': case['name'], 'precision': mode, 'case': case, 'steps': steps})
    doc = {'schema': 1, 'pins': pin(RANGES), 'executions': executions, 'fixtures': fixtures}
    out = Path(__file__).resolve().parent / 'native-settlement-fixtures.json'
    out.write_text(json.dumps(doc, separators=(',', ':')) + '\n', encoding='utf-8')
    print(f'{len(fixtures)} fixtures, {executions} native executions')


if __name__ == '__main__':
    main()
