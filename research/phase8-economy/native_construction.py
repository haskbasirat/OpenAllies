"""Original construction/upgrade affordability, payment and refunds in x86.

Executed unchanged: 562FD0 construct affordability, 566110 upgrade gate and
affordability, 566360 upgrade start, 565700 upgrade cancel, 5656B0
construction cancel, 565770 upgrade list, 4E20A0/4E1690/4E39D0 kingdom
stock checks/debit/credit, 45F180 vector zero-resize, 637820 vector release.
Seams (recorded; bodies not executed): actor virtuals +0x108 owner and +0x34
state, 4E1910 kingdom restrictions, 45F4D0 cost quote (fixture sequence),
4CF110 actor modifier token, 40C230 messages and CString helpers, 54BFC0
construction-begin trigger, 561D40 upgrade modifiers, 4D64F0 actor state,
4F6760 actor destruction, allocator.
"""
from pathlib import Path
import json, random, sys
sys.path.insert(0, str(Path(__file__).resolve().parent))
from harness import Machine, MODES, bits, pin, ret_pop  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX  # noqa: E402

RANGES = {
    'construct_afford_562FD0': (0x562FD0, 0x563153), 'upgrade_afford_566110': (0x566110, 0x566353),
    'upgrade_start_566360': (0x566360, 0x5663F1), 'upgrade_cancel_565700': (0x565700, 0x565762),
    'construction_cancel_5656B0': (0x5656B0, 0x565700), 'upgrade_list_565770': (0x565770, 0x565797),
    'can_afford_4E20A0': (0x4E20A0, 0x4E20F7), 'debit_4E1690': (0x4E1690, 0x4E16C5),
    'credit_4E39D0': (0x4E39D0, 0x4E3A4C), 'resize_45F180': (0x45F180, 0x45F1EE),
    'vector_release_637820': (0x637820, 0x637848),
}
K = 0x2000000
STOCK = 0x2010000
REG, TABLE, DEFS = 0x2020000, 0x2020100, 0x2021000
CACHE, LIMITS = 0x2024000, 0x2024800
COMP, ACTOR, AVT, ADATA, MODEL = 0x2030000, 0x2031000, 0x2032000, 0x2033000, 0x2034000
UDEF = 0x2040000  # upgrade/construct definition id d at UDEF + d*0x400
STUB_KINGDOM, STUB_STATE = 0x3000100, 0x3000110
MANAGER, TRIGGERS, WORLD = 0x7A7A0000, 0x2035000, 0x2036000


class ConstructionMachine(Machine):
    def __init__(self, cw, case):
        super().__init__(cw)
        self.case = case
        n = len(case['catalog']['resources'])
        self.n = n
        self.put(K + 0x1C4, STOCK); self.put(K + 0x1C8, n); self.put(K + 0x1CC, n)
        self.put_words(STOCK, case['stock'])
        self.put(0xB36530, REG); self.put(REG + 0x34, TABLE); self.put(REG + 0x38, n)
        for i, d in enumerate(case['catalog']['resources']):
            self.put(TABLE + 4 * i, DEFS + 0x40 * i)
            self.put(DEFS + 0x40 * i + 0x18, {'Stockpiled': 0, 'Upkeep': 1, 'LimitedConsumed': 2}[d['kind']])
        self.put(0xB39AB0, CACHE); self.put(CACHE + 0x1B8, LIMITS); self.put(LIMITS, case['limits']['stockpiled_maximum'])
        self.put(0xB36520, TRIGGERS); self.put(0xB36534, WORLD)
        self.put(WORLD + 0x30, WORLD + 0x800)  # map object (read by 6939B0 for the grid argument)
        self.put(COMP + 4, ACTOR)
        self.put(ACTOR, AVT); self.put(AVT + 0x108, STUB_KINGDOM); self.put(AVT + 0x34, STUB_STATE)
        self.put(ACTOR + 4, ADATA); self.put(ACTOR + 0x60, MODEL)
        w = case['world']
        nodes = [self.alloc(8) for _ in w['upgrade_list']]
        for i, (node, d) in enumerate(zip(nodes, w['upgrade_list'])):
            self.put(node, UDEF + d * 0x400); self.put(node + 4, nodes[i + 1] if i + 1 < len(nodes) else 0)
        self.put(ADATA + 0x4C0, nodes[0] if nodes else 0)
        for d, flags in w['definition_flags'].items():
            self.put(UDEF + int(d) * 0x400 + 0x174, flags)
            self.put(UDEF + int(d) * 0x400 + 0x10, 0x2050000 + int(d))
        p = case['payments']
        for off, values in ((0x10, p['paid']), (0x1C, p['upgrade_paid'])):
            data = self.alloc(4 * max(1, len(values)))
            self.put_words(data, values)
            self.put(COMP + off, data); self.put(COMP + off + 4, len(values)); self.put(COMP + off + 8, len(values))
        self.put(COMP + 0x28, UDEF + p['upgrade'] * 0x400 if p['upgrade'] else 0)
        self.quotes = {d: list(q) for d, q in case['quotes'].items()}
        self.trace = []
        self.seam(0x6F7810, lambda m, esp: (self.alloc(self.read(esp + 4)), 0))
        self.seam(0x6F7430, ret_pop(0))
        self.seam(STUB_KINGDOM, self._kingdom)
        self.seam(STUB_STATE, self._state)
        self.seam(0x4E1910, self._allows)
        self.seam(0x45F4D0, self._quote)
        self.seam(0x4CF110, lambda m, esp: (0x4D4D0000, 0))
        self.seam(0x401B00, self._string_ctor)
        self.seam(0x4B9F40, self._format)
        self.seam(0x5C4D10, self._cost_text)
        self.seam(0x4012C0, lambda m, esp: (self.read(esp + 4), 0))
        self.seam(0x401560, self._assign)
        self.seam(0x40C230, self._message)
        self.seam(0x54BFC0, self._begun)
        self.seam(0x561D40, self._modifiers)
        self.seam(0x4D64F0, self._actor_state)
        self.seam(0x4F6760, self._destroy)

    def ident(self, pointer):
        return (pointer - UDEF) // 0x400 if pointer else 0

    def _string(self):
        header = self.alloc(0x40)
        self.put(header + 8, 16); self.put(header + 0xC, 100)
        return header + 0x10

    def _string_ctor(self, m, esp):
        this = self.uc.reg_read(UC_X86_REG_ECX)
        self.put(this, self._string())
        return this, 4

    def _format(self, m, esp):
        out = self.read(esp + 4)
        self.put(out, self._string())
        return out, 8

    def _cost_text(self, m, esp):
        out = self.read(esp + 4)
        self.put(out, self._string())
        return out, 0

    def _assign(self, m, esp):
        this = self.uc.reg_read(UC_X86_REG_ECX)
        self.put(this, self.read(self.read(esp + 4)))
        return this, 4

    def _kingdom(self, m, esp):
        self.trace.append({'kind': 'kingdom'})
        return (K if self.case['world']['kingdom'] else 0), 0

    def _state(self, m, esp):
        self.trace.append({'kind': 'state_active', 'state': self.read(esp + 4)})
        return int(self.case['world']['state_active']), 4

    def _allows(self, m, esp):
        assert self.uc.reg_read(UC_X86_REG_ECX) == K and self.read(esp + 4) == ACTOR
        self.trace.append({'kind': 'kingdom_allows', 'definition': self.ident(self.read(esp + 8))})
        return int(self.case['world']['kingdom_allows']), 8

    def _quote(self, m, esp):
        d = self.ident(self.uc.reg_read(UC_X86_REG_ECX))
        out, modifier = self.read(esp + 4), self.read(esp + 8)
        assert modifier == 0x4D4D0000
        q = self.quotes[str(d)]
        values = q.pop(0) if len(q) > 1 else q[0]
        data = self.alloc(4 * max(1, len(values)))
        self.put_words(data, values)
        self.put(out, data); self.put(out + 4, len(values)); self.put(out + 8, len(values))
        self.trace.append({'kind': 'quote', 'definition': d, 'values': values})
        return out, 8

    def _message(self, m, esp):
        caller = self.read(esp)
        assert self.read(esp + 8) == 0x11 and self.read(esp + 12) == ACTOR
        name = {0x5630E5: 'notify_construct_unaffordable', 0x5662CE: 'notify_upgrade_unaffordable'}[caller]
        self.trace.append({'kind': name, 'kingdom': self.read(esp + 16) != 0})
        return None, 0

    def _begun(self, m, esp):
        assert self.uc.reg_read(UC_X86_REG_ECX) == TRIGGERS
        self.trace.append({'kind': 'construction_begun', 'kingdom': self.read(esp + 4) != 0,
                           'definition': self.ident(self.read(esp + 8))})
        return None, 8

    def _modifiers(self, m, esp):
        assert self.uc.reg_read(UC_X86_REG_ECX) == MODEL and self.read(esp + 4) == 0x4D4D0000 and self.read(esp + 12) == 0
        self.trace.append({'kind': 'apply_upgrade_modifiers', 'definition': self.ident(self.read(esp + 8))})
        return None, 12

    def _actor_state(self, m, esp):
        assert self.uc.reg_read(UC_X86_REG_ECX) == ACTOR
        self.trace.append({'kind': 'actor_state', 'state': self.read(esp + 4), 'enabled': bool(self.read(esp + 8) & 0xFF)})
        return None, 8

    def _destroy(self, m, esp):
        assert self.uc.reg_read(UC_X86_REG_ECX) == WORLD and self.read(esp + 4) == ACTOR and self.read(esp + 8) == 0
        self.trace.append({'kind': 'destroy_actor'})
        return None, 8

    def payments(self):
        def vec(off):
            return self.words(self.read(COMP + off), self.read(COMP + off + 4))
        return {'paid': vec(0x10), 'upgrade_paid': vec(0x1C), 'upgrade': self.ident(self.read(COMP + 0x28))}

    def run(self, op):
        self.trace = []
        w = self.case['world']
        w.update(op.get('world', {}))
        self.put(ACTOR + 0x90, 0x2037000 if w['has_structure'] else 0)
        flags = w['actor_flags']
        self.put(ACTOR + 0x100, flags & 0xFFFFFFFF); self.put(ACTOR + 0x104, flags >> 32)
        self.put8(ACTOR + 8, int(w['destroyed']))
        kind = op['kind']
        d = UDEF + op.get('definition', 0) * 0x400 if op.get('definition') else 0
        result = None
        if kind == 'construct_afford':
            eax, _ = self.call(0x562FD0, [d], COMP)
            result = bool(eax & 0xFF)
        elif kind == 'upgrade_afford':
            eax, _ = self.call(0x566110, [d, int(op['check_cost']), int(op['notify'])], COMP)
            result = bool(eax & 0xFF)
        elif kind == 'upgrade_start':
            self.call(0x566360, [d], COMP)
        elif kind == 'upgrade_cancel':
            self.call(0x565700, [], COMP)
        elif kind == 'construction_cancel':
            self.call(0x5656B0, [], COMP)
        else:
            raise AssertionError(kind)
        return {'stock': self.words(STOCK, self.n), 'payments': self.payments(), 'events': self.trace, 'result': result}


R = random.Random(0x566360)
UPGRADE = 1 << 15
ENABLED, BLOCKED = 1 << 27, 1 << 21


def make_case(i):
    n = R.randint(1, 4)
    kinds = [R.choice(['Stockpiled', 'Stockpiled', 'Upkeep', 'LimitedConsumed']) for _ in range(n)]
    if 'Stockpiled' not in kinds:
        kinds[0] = 'Stockpiled'
    defs = [1, 2, 3, 4]

    def cost():
        return [bits(R.choice([0.0, 5.0, 25.0, 60.0, R.uniform(0, 120)])) for _ in range(n)]
    world = {'kingdom': R.random() < 0.9, 'state_active': R.random() < 0.7, 'kingdom_allows': R.random() < 0.85,
             'has_structure': R.random() < 0.6, 'destroyed': R.random() < 0.15,
             'actor_flags': R.choice([ENABLED, ENABLED, ENABLED | BLOCKED, 0, ENABLED | (1 << 40)]),
             'upgrade_list': R.sample(defs, R.randint(0, 3)),
             'definition_flags': {str(d): R.choice([UPGRADE, UPGRADE, 0, UPGRADE | 1]) for d in defs}}
    ops = []
    for _ in range(R.randint(3, 7)):
        k = R.choice(['construct_afford', 'upgrade_afford', 'upgrade_start', 'upgrade_start', 'upgrade_cancel', 'construction_cancel'])
        op = {'kind': k}
        if k in ('construct_afford', 'upgrade_afford', 'upgrade_start'):
            op['definition'] = R.choice(defs + [0]) if k != 'construct_afford' else R.choice(defs)
        if k == 'upgrade_afford':
            op['check_cost'] = R.random() < 0.8
            op['notify'] = R.random() < 0.5
        if R.random() < 0.3:
            op['world'] = {'kingdom': R.random() < 0.85, 'destroyed': R.random() < 0.2, 'has_structure': R.random() < 0.6,
                           'state_active': R.random() < 0.7}
        ops.append(op)
    return {'name': f'random-{i}', 'catalog': {'resources': [{'id': f'r{j}', 'kind': k, 'limited_provider': False,
                                                              'purchase_cost': [0] * n} for j, k in enumerate(kinds)]},
            'limits': {'provided_maximum': 100, 'stockpiled_maximum': bits(R.choice([10000.0, 200.0]))},
            'stock': [bits(R.choice([0.0, 30.0, 150.0, R.uniform(0, 300)])) for _ in range(n)],
            'quotes': {str(d): [cost() for _ in range(R.randint(1, 3))] for d in defs},
            'payments': {'paid': cost(), 'upgrade_paid': cost(), 'upgrade': R.choice([0, 2])},
            'world': world, 'ops': ops}


def directed():
    base = make_case(-1)
    base['name'] = 'directed-upgrade-cycle'
    n = len(base['catalog']['resources'])
    base['catalog']['resources'][0]['kind'] = 'Stockpiled'
    base['stock'] = [bits(100.0)] + [bits(10.0)] * (n - 1)
    base['quotes'] = {'1': [[bits(40.0)] + [0] * (n - 1), [bits(90.0)] + [0] * (n - 1)], '2': [[bits(500.0)] + [0] * (n - 1)],
                      '3': [[bits(10.0)] + [0] * (n - 1)], '4': [[bits(1.0)] + [0] * (n - 1)]}
    base['payments'] = {'paid': [bits(12.5)] + [0] * (n - 1), 'upgrade_paid': [0] * n, 'upgrade': 0}
    base['world'] = {'kingdom': True, 'state_active': True, 'kingdom_allows': True, 'has_structure': True, 'destroyed': False,
                     'actor_flags': ENABLED, 'upgrade_list': [1, 2, 3], 'definition_flags': {'1': UPGRADE, '2': UPGRADE, '3': UPGRADE, '4': UPGRADE}}
    base['ops'] = [{'kind': 'upgrade_start', 'definition': 1},  # afford 40, debit second quote 90
                   {'kind': 'upgrade_cancel'},
                   {'kind': 'upgrade_start', 'definition': 2},  # unaffordable with message
                   {'kind': 'upgrade_start', 'definition': 4},  # not in upgrade list
                   {'kind': 'construct_afford', 'definition': 2},
                   {'kind': 'construct_afford', 'definition': 2, 'world': {'kingdom': False}},
                   {'kind': 'upgrade_afford', 'definition': 3, 'check_cost': True, 'notify': True, 'world': {'kingdom': True}},
                   {'kind': 'construction_cancel'},
                   {'kind': 'construction_cancel', 'world': {'destroyed': True}}]
    return [base]


def main():
    cases = directed() + [make_case(i) for i in range(80)]
    fixtures, executions = [], 0
    for case in cases:
        for mode, cw in MODES:
            m = ConstructionMachine(cw, json.loads(json.dumps(case)))
            steps = []
            for op in case['ops']:
                w = dict(m.case['world'], **op.get('world', {}))
                reaches_restriction = (op['kind'] in ('upgrade_afford', 'upgrade_start') and op.get('definition')
                                       and (w['has_structure'] or w['state_active'])
                                       and (w['actor_flags'] & BLOCKED) == 0 and (w['actor_flags'] & ENABLED)
                                       and w['definition_flags'][str(op['definition'])] & UPGRADE)
                if reaches_restriction and not w['kingdom']:
                    continue  # native null-kingdom dereference in 4E1910: outside domain
                steps.append({'op': op, 'expected': m.run(op)})
                executions += 1
            fixtures.append({'name': case['name'], 'precision': mode, 'case': case, 'steps': steps})
    doc = {'schema': 1, 'pins': pin(RANGES), 'executions': executions, 'fixtures': fixtures}
    out = Path(__file__).resolve().parent / 'native-construction-fixtures.json'
    out.write_text(json.dumps(doc, separators=(',', ':')) + '\n', encoding='utf-8')
    print(f'{len(fixtures)} fixtures, {executions} native executions')


if __name__ == '__main__':
    main()
