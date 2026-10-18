"""Original kingdom resource transfers in isolated x86.

Executed unchanged: 4E2080/4E1630 stock add+clamp, 4E16D0 stock subtract,
4CA680 steal-resources effect (+4C8330 target getter), 509840 team command
type 3 "give resources" including 473520 primary-resource lookup, 432650 display
rounding, 96D5E0 CRT floor, 96CC4C _ftol, 4E24B0/4CDD60 faction ability
affordability with 4E20A0, 4E0FD0/4CDFD0/58E870 faction XP with multiplier.
Seams (bodies not executed): allocator, actor virtuals +0xB4/+0x108, 404F30/
404F60 logs, 4E9560 glyph text, 856370/401330 CString allocator, 42D4B0 event
dispatch (recorded), 4BEB50 ability resource cost (fixture-supplied vector).
"""
from pathlib import Path
import json, random, shutil, sys
sys.path.insert(0, str(Path(__file__).resolve().parent))
from harness import Machine, MODES, bits, pin, ret_pop, ROOT  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX  # noqa: E402

RANGES = {
    'credit_row_4E2080': (0x4E2080, 0x4E209E), 'clamp_row_4E1630': (0x4E1630, 0x4E1687),
    'debit_row_4E16D0': (0x4E16D0, 0x4E16E9), 'steal_4CA680': (0x4CA680, 0x4CA759),
    'target_4C8330': (0x4C8330, 0x4C8351), 'team_command_509840': (0x509840, 0x50A61C),
    'primary_resource_473520': (0x473520, 0x47355C), 'display_round_432650': (0x432650, 0x432669),
    'floor_96D5E0': (0x96D5E0, 0x96D7D4), 'ftol_96CC4C': (0x96CC4C, 0x96CCC1),
    'ability_afford_4E24B0': (0x4E24B0, 0x4E2583), 'ability_xp_4CDD60': (0x4CDD60, 0x4CDD8D),
    'can_afford_4E20A0': (0x4E20A0, 0x4E20F7), 'faction_xp_wrapper_4E0FD0': (0x4E0FD0, 0x4E0FE9),
    'faction_xp_4CDFD0': (0x4CDFD0, 0x4CE073), 'score_58E870': (0x58E870, 0x58E883),
    'zero_9F32DC': (0x9F32DC, 0x9F32E0), 'half_9FBDB8': (0x9FBDB8, 0x9FBDBC),
    'tribute_509440': (0x509440, 0x509592), 'gtid_432E40': (0x432E40, 0x432E65),
}
# CRT floor's x87 path lives outside 96D5E0; it is CRT code, executed and
# recorded via the visited-window ledger rather than a named range.
K, K2 = 0x2000000, 0x2008000
ARR = {K: 0x2010000, K2: 0x2011000}
REG, TABLE, DEFS = 0x2020000, 0x2020100, 0x2021000
CACHE, LIMITS, MANAGER = 0x2024000, 0x2024800, 0x2025000
EFFECT, AMOUNTS, CTX, TARGET, TVT = 0x2030000, 0x2030100, 0x2031000, 0x2032000, 0x2033000
CMD, CMDDATA = 0x2034000, 0x2034100
ABILITY, ADEF, FACTION, FDEF, STATS = 0x2036000, 0x2036100, 0x2037000, 0x2037100, 0x2038000
ALLOCATOR, AVT = 0x2039000, 0x2039100
GTIDS, TACT, TAVT, SETTLE, SCOMP = 0x2100000, 0x2040000, 0x2041000, 0x2042000, 0x2042100
STUB_TNAME, STUB_TOWNER = 0x3000140, 0x3000150
STUB_ALIVE, STUB_KINGDOM, STUB_ALLOC, STUB_FREE = 0x3000100, 0x3000110, 0x3000120, 0x3000130
EVENT_MANAGER = 0x7A7A0000


class TransferMachine(Machine):
    def __init__(self, cw, case):
        super().__init__(cw)
        self.case = case
        n = len(case['catalog']['resources'])
        self.n = n
        for k in (K, K2):
            for off in (0x1A0, 0x1AC, 0x1B8, 0x1C4):
                self.put(k + off, ARR[k] + (off - 0x1A0) * 0x10); self.put(k + off + 4, n)
            self.put_words(ARR[k] + 0x240, case['stock'][str(k == K2 and 1 or 0)])
            self.put(k + 0x19C, 0)
        self.put(0xB36530, REG); self.put(REG + 0x34, TABLE); self.put(REG + 0x38, n); self.put(REG + 0x844, 0)
        for i, d in enumerate(case['catalog']['resources']):
            self.put(TABLE + 4 * i, DEFS + 0x40 * i)
            self.put(DEFS + 0x40 * i + 0xC, i)
            self.put(DEFS + 0x40 * i + 0x18, {'Stockpiled': 0, 'Upkeep': 1, 'LimitedConsumed': 2}[d['kind']])
            # +0x28 display object: its +0x18 glyph unit is read for the message text
            # (509A8C); a null object would be a null-pointer read.
            glyph = self.alloc(0x40)
            self.put(DEFS + 0x40 * i + 0x28, glyph); self.put16(glyph + 0x18, 0x24 + i)
        self.put(0xB3A660, MANAGER); self.put(MANAGER + 0x28, n)
        self.put(0xB39AB0, CACHE)
        self.put(CACHE + 0x1B8, LIMITS); self.put(LIMITS, case['limits']['stockpiled_maximum'])
        self.put(CACHE + 0x17C, LIMITS + 4); self.put(LIMITS + 4, case['faction_multiplier'])
        self.put(0xB39D8C, EVENT_MANAGER)
        self.put(AVT + 0xC, STUB_ALLOC); self.put(AVT + 4, STUB_FREE); self.put(ALLOCATOR, AVT)
        self.put(TVT + 0xB4, STUB_ALIVE); self.put(TVT + 0x108, STUB_KINGDOM); self.put(TARGET, TVT)
        self.trace = []
        self.seam(0x6F7810, lambda m, esp: (self.alloc(self.read(esp + 4)), 0))
        self.seam(0x6F7430, ret_pop(0))
        self.seam(0x404F30, self._log)
        self.seam(0x404F60, self._log)
        self.seam(STUB_ALIVE, lambda m, esp: (int(self.op.get('target_alive', True)), 0))
        self.seam(STUB_KINGDOM, lambda m, esp: ({'K': K, 'K2': K2, None: 0}[self.op.get('victim')], 0))
        self.seam(0x4E9560, self._glyph)
        self.seam(0x856370, lambda m, esp: (ALLOCATOR, 0))
        self.seam(STUB_ALLOC, self._new_string)
        self.seam(STUB_FREE, ret_pop(1))
        self.seam(0x401330, self._grow)
        self.seam(0x42D4B0, self._event)
        self.seam(0x4BEB50, self._ability_cost)
        self.put(0xB364C0, GTIDS)
        self.put(TAVT + 0x24, STUB_TNAME); self.put(TAVT + 0x160, STUB_TOWNER)
        for i in range(8):
            self.put(TACT + 0x100 * i, TAVT)
        self.seam(STUB_TNAME, lambda m, esp: (self._glyph_out(self.read(esp + 4)), 4))
        self.seam(STUB_TOWNER, self._change_owner)
        self.seam(0x4CED40, self._settlement)
        self.seam(0x4D9E20, self._filter_ctor)
        self.seam(0x4D9B60, self._keeps)
        self.seam(0x4F6760, self._destroy)
        self.seam(0x404F30, self._log)
        self.seam(0x42D490, self._event_490)

    def _log(self, m, esp):
        self.trace.append({'kind': 'log'})
        return None, 0

    def _string(self, refcount):
        header = self.alloc(0x40)
        self.put(header, ALLOCATOR); self.put(header + 4, 0); self.put(header + 8, 16); self.put(header + 0xC, refcount)
        return header

    def _glyph_out(self, out):
        self.put(out, self._string(100) + 0x10)
        return out

    def actor_index(self, p):
        assert (p - TACT) % 0x100 == 0 and 0 <= (p - TACT) // 0x100 < 8
        return (p - TACT) // 0x100

    def _change_owner(self, m, esp):
        i = self.actor_index(self.uc.reg_read(UC_X86_REG_ECX))
        self.trace.append({'kind': 'change_owner', 'actor': i, 'receiver': {K: 'K', K2: 'K2'}[self.read(esp + 4)]})
        return None, 4

    def _settlement(self, m, esp):
        i = self.actor_index(self.uc.reg_read(UC_X86_REG_ECX))
        self.trace.append({'kind': 'settlement_of', 'actor': i})
        members = self.op['members']
        if members is None:
            return 0, 0
        self.put(SETTLE + 0x98, SCOMP)
        self.put(SCOMP + 0x1C, len(members))
        data = self.alloc(4 * max(1, len(members)))
        self.put_words(data, [TACT + 0x100 * mm for mm in members])
        self.put(SCOMP + 0x18, data)
        return SETTLE, 0

    def _filter_ctor(self, m, esp):
        assert self.read(esp + 4) == 0x14
        this = self.uc.reg_read(UC_X86_REG_ECX)
        self.put(this + 0x34, 0)
        return this, 4

    def _keeps(self, m, esp):
        i = self.actor_index(self.read(esp + 4))
        self.trace.append({'kind': 'keeps_member', 'member': i})
        return int(i in self.op['kept']), 4

    def _destroy(self, m, esp):
        assert self.read(esp + 8) == 0
        i = self.actor_index(self.read(esp + 4))
        self.trace.append({'kind': 'destroy_actor', 'member': i})
        return None, 8

    def _event_490(self, m, esp):
        kind, kingdom = self.read(esp + 8), self.read(esp + 12)
        self.trace.append({'kind': {0x1F: 'tribute_given', 0x20: 'tribute_received'}[kind],
                           'kingdom': {K: 'K', K2: 'K2'}[kingdom]})
        return None, 0

    def _glyph(self, m, esp):
        out = self.read(esp + 4)
        self.put(out, self._string(100) + 0x10)
        return out, 8

    def _new_string(self, m, esp):
        return self._string(1), 0

    def _grow(self, m, esp):
        raise AssertionError('CString growth not expected')

    def _event(self, m, esp):
        manager, kind, kingdom = self.read(esp + 4), self.read(esp + 8), self.read(esp + 12)
        assert manager == EVENT_MANAGER
        value = self.read(esp + 16)
        self.trace.append({'kind': {0x1D: 'resources_given', 0x1E: 'resources_received'}[kind],
                           'kingdom': {K: 'K', K2: 'K2'}[kingdom], 'display': value - (1 << 32) if value >> 31 else value})
        return None, 0

    def _ability_cost(self, m, esp):
        vec = self.read(esp + 4)
        values = self.op['ability']['resources']
        data = self.alloc(4 * max(1, len(values)))
        self.put_words(data, values)
        self.put(vec, data); self.put(vec + 4, len(values)); self.put(vec + 8, len(values))
        return vec, 4

    def stock(self, k):
        return self.words(ARR[k] + 0x240, self.n)

    def run(self, op):
        self.op = op
        self.trace = []
        kind = op['kind']
        result = None
        kingdom = {'K': K, 'K2': K2}
        if kind == 'credit_row':
            self.call(0x4E2080, [op['index'], op['amount']], kingdom[op['kingdom']])
        elif kind == 'debit_row':
            self.call(0x4E16D0, [op['index'], op['amount']], kingdom[op['kingdom']])
        elif kind == 'steal':
            self.put_words(AMOUNTS, op['amounts']); self.put(EFFECT + 0x14, AMOUNTS)
            self.put(CTX + 0x1C, TARGET if op.get('target', True) else 0)
            self.put(CTX + 0xC, kingdom[op['thief']])
            eax, _ = self.call(0x4CA680, [0x11111111, CTX, 0x22222222], EFFECT)
            result = bool(eax & 0xFF)
        elif kind == 'give':
            self.put(CMD + 4, CMDDATA); self.put(CMDDATA + 0xC, 3)
            self.put(CMD + 8, kingdom[op['giver']]); self.put(CMD + 0xC, kingdom[op['receiver']])
            self.put(CMD + 0x1C, op['amount'])
            self.put(kingdom[op['receiver']] + 0x19C, 2 if op.get('defeated') else 0)
            self.call(0x509840, [], CMD)
            self.put(kingdom[op['receiver']] + 0x19C, 0)
            result = self.read(CMD + 0x1C)
        elif kind == 'tribute':
            for slot in range(1, 9):
                self.put16(GTIDS + 4 + 2 * slot, 7); self.put(GTIDS + 0x20004 + 4 * slot, TACT + 0x100 * (slot - 1))
            self.put(CMD + 4, CMDDATA); self.put(CMDDATA + 0xC, 4)
            self.put(CMD + 8, kingdom[op['giver']]); self.put(CMD + 0xC, kingdom[op['receiver']])
            self.put(CMD + 0x10, (7 << 16) | (op['actor'] + 1))
            self.put(kingdom[op['receiver']] + 0x19C, 2 if op.get('defeated') else 0)
            self.call(0x509840, [], CMD)
            self.put(kingdom[op['receiver']] + 0x19C, 0)
        elif kind == 'ability_afford':
            a = op['ability']
            self.put(ABILITY + 8, ADEF if a is not None else 0)
            if a is not None:
                self.put(ADEF + 0x74, a['experience'])
            f = op['faction']
            self.put(K + 0x230, FACTION if f is not None else 0)
            if f is not None:
                self.put(FACTION + 0xC, f['balance'])
            eax, _ = self.call(0x4E24B0, [ABILITY], K)
            result = bool(eax & 0xFF)
        elif kind == 'faction_xp':
            f = op['faction']
            self.put(K + 0x230, FACTION if f is not None else 0)
            if f is not None:
                self.put(FACTION + 4, FDEF); self.put(FDEF + 0x3C, f['maximum'])
                self.put(FACTION + 8, K if op['statistics'] is not None else K2)
                self.put(K + 0x32C, STATS); self.put(K2 + 0x32C, 0)
                self.put(FACTION + 0xC, f['balance'])
                if op['statistics'] is not None:
                    self.put(STATS + 0x28 + 44 * 4, op['statistics'])
            self.call(0x4E0FD0, [op['amount']], K)
            if f is not None:
                result = {'balance': self.read(FACTION + 0xC),
                          'score44': self.read(STATS + 0x28 + 44 * 4) if op['statistics'] is not None else None}
        else:
            raise AssertionError(kind)
        return {'stock': [self.stock(K), self.stock(K2)], 'events': self.trace, 'result': result}


R = random.Random(0x4CA680)


def fv(lo, hi):
    return bits(R.choice([lo, hi, 0.0, 0.5, 1.0, 2.5, 10.0, 99.5, R.uniform(lo, hi), R.uniform(lo, hi)]))


def make_case(i):
    n = R.randint(1, 5)
    kinds = [R.choice(['Stockpiled', 'Stockpiled', 'Upkeep', 'LimitedConsumed']) for _ in range(n)]
    if 'Stockpiled' not in kinds:
        kinds[R.randrange(n)] = 'Stockpiled'
    maximum = R.choice([10000.0, 100.0, 250.0])
    stock = [[fv(0, maximum) for _ in range(n)] for _ in range(2)]
    ops = []
    for _ in range(R.randint(3, 8)):
        op = R.choice(['credit_row', 'debit_row', 'steal', 'steal', 'give', 'give', 'ability_afford', 'faction_xp', 'tribute'])
        if op in ('credit_row', 'debit_row'):
            ops.append({'kind': op, 'kingdom': R.choice(['K', 'K2']), 'index': R.randrange(n),
                        'amount': bits(R.choice([R.uniform(-400, 400), 0.0, -0.0, 1e-30, maximum, -maximum * 2]))})
        elif op == 'steal':
            ops.append({'kind': op, 'amounts': [bits(R.choice([0.0, -3.0, R.uniform(0, 300), 25.0, 1e6])) for _ in range(n)],
                        'thief': R.choice(['K', 'K2', 'K2']), 'victim': R.choice(['K', 'K', 'K2', None]),
                        'target': R.random() < 0.9, 'target_alive': R.random() < 0.9})
        elif op == 'give':
            ops.append({'kind': op, 'giver': R.choice(['K', 'K']), 'receiver': R.choice(['K2', 'K2', 'K']),
                        'amount': bits(R.choice([R.uniform(0, 500), 50.0, 0.49999997, 1.5, 2.5, -10.0, 0.0, 1e7])),
                        'defeated': R.random() < 0.15})
        elif op == 'tribute':
            members = None if R.random() < 0.3 else R.sample(range(8), R.randint(0, 5))
            ops.append({'kind': op, 'giver': 'K', 'receiver': R.choice(['K2', 'K2', 'K']), 'actor': R.randrange(8),
                        'members': members, 'kept': R.sample(range(8), R.randint(0, 8)), 'defeated': R.random() < 0.15})
        elif op == 'ability_afford':
            ops.append({'kind': op, 'ability': None if R.random() < 0.15 else
                        {'experience': R.choice([0, 5, 50, 500, -5]), 'resources': [fv(0, 300) for _ in range(n)]},
                        'faction': None if R.random() < 0.25 else {'balance': fv(-10, 600)}})
        else:
            ops.append({'kind': op, 'amount': bits(R.choice([R.uniform(-20, 80), 0.0, 3.0])),
                        'faction': None if R.random() < 0.2 else {'balance': fv(-5, 500), 'maximum': R.choice([bits(100.0), bits(-1.0), fv(0, 600)])},
                        'statistics': R.choice([None, bits(0.0), fv(0, 1000)])})
    return {'name': f'random-{i}', 'catalog': {'resources': [{'id': f'r{j}', 'kind': k, 'limited_provider': False,
                                                              'purchase_cost': [0] * n} for j, k in enumerate(kinds)]},
            'limits': {'provided_maximum': 100, 'stockpiled_maximum': bits(maximum)},
            'faction_multiplier': R.choice([bits(1.0), bits(0.5), bits(1.5), bits(0.1)]),
            'stock': {'0': stock[0], '1': stock[1]}, 'ops': ops}


def directed():
    cases = []
    base = make_case(-1)
    base['catalog']['resources'] = [{'id': 'oil', 'kind': 'Upkeep', 'limited_provider': True, 'purchase_cost': [0, 0, 0]},
                                    {'id': 'money', 'kind': 'Stockpiled', 'limited_provider': False, 'purchase_cost': [0, 0, 0]},
                                    {'id': 'steel', 'kind': 'Stockpiled', 'limited_provider': False, 'purchase_cost': [0, 0, 0]}]
    base['limits']['stockpiled_maximum'] = bits(1000.0)
    base['stock'] = {'0': [bits(5.0), bits(120.0), bits(7.5)], '1': [bits(1.0), bits(990.0), bits(0.0)]}
    base['name'] = 'directed-give-steal'
    base['ops'] = [
        {'kind': 'give', 'giver': 'K', 'receiver': 'K2', 'amount': bits(50.0)},
        {'kind': 'give', 'giver': 'K', 'receiver': 'K2', 'amount': bits(500.0)},
        {'kind': 'give', 'giver': 'K2', 'receiver': 'K', 'amount': bits(0.49999997)},
        {'kind': 'give', 'giver': 'K', 'receiver': 'K', 'amount': bits(3.25)},
        {'kind': 'give', 'giver': 'K', 'receiver': 'K2', 'amount': bits(10.0), 'defeated': True},
        {'kind': 'steal', 'amounts': [bits(3.0), bits(400.0), bits(2.0)], 'thief': 'K2', 'victim': 'K'},
        {'kind': 'steal', 'amounts': [bits(1.0), bits(1.0), bits(1.0)], 'thief': 'K', 'victim': 'K'},
        {'kind': 'steal', 'amounts': [bits(1.0), bits(1.0), bits(1.0)], 'thief': 'K', 'victim': None},
        {'kind': 'steal', 'amounts': [bits(1.0), bits(1.0), bits(1.0)], 'thief': 'K', 'victim': 'K2', 'target': False},
        {'kind': 'steal', 'amounts': [bits(1.0), bits(1.0), bits(1.0)], 'thief': 'K', 'victim': 'K2', 'target_alive': False},
    ]
    cases.append(base)
    return cases


def main():
    cases = directed() + [make_case(i) for i in range(90)]
    fixtures = []
    executions = 0
    visited = set()
    for case in cases:
        for mode, cw in MODES:
            m = TransferMachine(cw, case)
            initial = [m.stock(K), m.stock(K2)]
            steps = []
            for op in case['ops']:
                steps.append({'op': op, 'expected': m.run(op)})
                executions += 1
            fixtures.append({'name': case['name'], 'precision': mode, 'case': case, 'initial': initial, 'steps': steps})
    doc = {'schema': 1, 'pins': pin(RANGES), 'executions': executions, 'fixtures': fixtures}
    out = Path(__file__).resolve().parent / 'native-transfer-fixtures.json'
    out.write_text(json.dumps(doc, separators=(',', ':')) + '\n', encoding='utf-8')
    shutil.copyfile(out, ROOT / 'crates/aa-economy/tests/fixtures/native-transfer-fixtures.json')
    print(f'{len(fixtures)} fixtures, {executions} native executions')


if __name__ == '__main__':
    main()
