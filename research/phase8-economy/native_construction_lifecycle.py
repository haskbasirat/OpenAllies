"""Original ConstructionComponent lifecycle in isolated x86: salvage, sell,
command dispatch and Tick.

Executed unchanged: 565500 salvage value (CRT floor 96D5E0, 4849F0 growth,
45F180 zero-resize), 565A20 sell, 5658C0 salvage refund, 565860 cancel
command, 5667F0 command dispatch, 566880 Tick, 501570 auto-upgrade timer,
4CD160/4CD000/4CCF10 cost object, 529C30, plus everything native_construction.py
executes (562FD0 family, 566110, 566360, 565700, 5656B0, 4E20A0/4E1690/4E39D0).
Seams (recorded): 565400 enable, 564DC0 disable, 4D6660 actor refresh,
4CF110 modifier token, 404ED0 diagnostic, 42D490 event 0x17, 5C4CB0 refund
text, actor virtual +0x24 name, 77DC50/4D9A40 command traces, allocator and
the native_construction.py seams.
"""
from pathlib import Path
import json, random, sys
sys.path.insert(0, str(Path(__file__).resolve().parent))
from harness import MODES, bits, pin, ret_pop  # noqa: E402
from native_construction import (ConstructionMachine, ACTOR, AVT, COMP, CACHE, MODEL, UDEF, K, TABLE,  # noqa: E402
                                 WORLD, ENABLED, BLOCKED, UPGRADE)
from unicorn.x86_const import UC_X86_REG_ECX  # noqa: E402

RANGES = {
    'salvage_565500': (0x565500, 0x5655B8), 'sell_565A20': (0x565A20, 0x565A61),
    'refund_5658C0': (0x5658C0, 0x565A1C), 'cancel_command_565860': (0x565860, 0x5658BB),
    'dispatch_5667F0': (0x5667F0, 0x566870), 'tick_566880': (0x566880, 0x566AC0),
    'auto_upgrade_due_501570': (0x501570, 0x5015AF), 'cost_object_4CD160': (0x4CD160, 0x4CD240),
    'cost_positive_4CD000': (0x4CD000, 0x4CD042), 'cost_object_release_4CCF10': (0x4CCF10, 0x4CCF40),
    'upgrade_afford_566110': (0x566110, 0x566353), 'upgrade_start_566360': (0x566360, 0x5663F1),
    'upgrade_cancel_565700': (0x565700, 0x565762), 'construction_cancel_5656B0': (0x5656B0, 0x565700),
    'can_afford_4E20A0': (0x4E20A0, 0x4E20F7), 'debit_4E1690': (0x4E1690, 0x4E16C5),
    'credit_4E39D0': (0x4E39D0, 0x4E3A4C), 'resize_45F180': (0x45F180, 0x45F1EE),
}
TOKEN, MGR, ALLOC, AVT2 = 0x2037800, 0x2038800, 0x2039000, 0x2039100
FRACTION, WINDOW, CMD, CMDDATA = 0x2039800, 0x2039810, 0x2039900, 0x2039A00
STUB_NAME, STUB_NIL, STUB_FREE = 0x3000120, 0x3000130, 0x3000140
UPGRADING, CONSTRUCTING, DISABLED = 1 << 29, 1 << 21, 1 << 30
TICK_QUOTE_RETURN = 0x5669C1


class LifecycleMachine(ConstructionMachine):
    def __init__(self, cw, case):
        super().__init__(cw, case)
        n = self.n
        self.put(0xB3A660, MGR); self.put(MGR + 0x24, TABLE); self.put(MGR + 0x28, n)
        self.put(CACHE + 0x13C, FRACTION); self.put(CACHE + 0x1CC, WINDOW)
        self.put(FRACTION, case['fraction'])
        self.put(AVT + 0x24, STUB_NAME)
        self.put(ALLOC, AVT2); self.put(AVT2 + 0xC, STUB_NIL); self.put(AVT2 + 4, STUB_FREE)
        self.refund = None
        self.seam(STUB_NAME, lambda m, esp: (self._fill(self.read(esp + 4)), 4))
        self.seam(STUB_NIL, lambda m, esp: (self._string() - 0x10, 0))
        self.seam(STUB_FREE, ret_pop(1))
        self.seam(0x856370, lambda m, esp: (ALLOC, 0))
        self.seam(0x565400, self._trace_seam('enable_structure'))
        self.seam(0x564DC0, self._trace_seam('disable_structure'))
        self.seam(0x4D6660, self._trace_seam('refresh_actor'))
        self.seam(0x4CF110, lambda m, esp: (self.token(), 0))
        self.seam(0x404ED0, self._log)
        self.seam(0x42D490, self._sold)
        self.seam(0x5C4CB0, self._refund_text)
        self.seam(0x77DC50, ret_pop(2))
        self.seam(0x4D9A40, ret_pop(0))

    def token(self):
        return TOKEN if self.case['world']['token'] else 0

    def _fill(self, out):
        self.put(out, self._string())
        return out

    def _trace_seam(self, name):
        def handler(m, esp):
            assert self.uc.reg_read(UC_X86_REG_ECX) in (COMP, ACTOR)
            self.trace.append({'kind': name})
            return None, 0
        return handler

    def _log(self, m, esp):
        self.trace.append({'kind': 'log_auto_upgrade_cost'})
        return None, 0

    def _refund_text(self, m, esp):
        vec = self.read(esp + 8)
        self.refund = self.words(self.read(vec), self.read(vec + 4))
        return self._fill(self.read(esp + 4)), 0

    def _sold(self, m, esp):
        assert self.read(esp + 8) == 0x17 and self.read(esp + 0x10) == ACTOR
        self.trace.append({'kind': 'notify_sold', 'kingdom': self.read(esp + 0xC) != 0, 'refund': self.refund})
        return None, 0

    def _state(self, m, esp):
        state = self.read(esp + 4)
        self.trace.append({'kind': 'state_active', 'state': state})
        w = self.case['world']
        return int(w['states'].get(str(state), w['state_active'])), 4

    def _quote(self, m, esp):
        d = self.ident(self.uc.reg_read(UC_X86_REG_ECX))
        out, modifier = self.read(esp + 4), self.read(esp + 8)
        q = self.quotes[str(d)]
        values = q.pop(0) if len(q) > 1 else q[0]
        data = self.alloc(4 * max(1, len(values)))
        self.put_words(data, values)
        self.put(out, data); self.put(out + 4, len(values)); self.put(out + 8, len(values))
        if self.read(esp) == TICK_QUOTE_RETURN:
            assert modifier == 0
            self.trace.append({'kind': 'base_quote', 'definition': d, 'values': values})
        else:
            assert modifier == self.token()
            self.trace.append({'kind': 'quote', 'definition': d, 'values': values})
        return out, 8

    def _modifiers(self, m, esp):
        assert self.uc.reg_read(UC_X86_REG_ECX) == MODEL and self.read(esp + 4) == self.token() and self.read(esp + 12) == 0
        self.trace.append({'kind': 'apply_upgrade_modifiers', 'definition': self.ident(self.read(esp + 8))})
        return None, 12

    def run(self, op):
        self.trace = []
        self.refund = None
        w = self.case['world']
        w.update(op.get('world', {}))
        self.put(ACTOR + 0x90, 0x2037000 if w['has_structure'] else 0)
        flags = w['actor_flags']
        self.put(ACTOR + 0x100, flags & 0xFFFFFFFF); self.put(ACTOR + 0x104, flags >> 32)
        self.put8(ACTOR + 8, int(w['destroyed']))
        self.put8(K + 0x239, int(w['dissolving']))
        self.put(TOKEN + 0x30, w['interval'])
        self.put(ACTOR + 0x1C, w['actor_time'])
        self.put(WORLD + 0xE0, w['world_time'])
        self.put(MODEL + 0x20, w['start_time'])
        self.put(WINDOW, w['window'])
        kind = op['kind']
        result = None
        if kind == 'salvage':
            vec = self.alloc(12)
            data = self.alloc(4 * max(1, len(op['out'])))
            self.put_words(data, op['out'])
            self.put(vec, data); self.put(vec + 4, len(op['out'])); self.put(vec + 8, len(op['out']))
            self.call(0x565500, [vec, int(op['accumulate'])], COMP)
            result = self.words(self.read(vec), self.read(vec + 4))
        elif kind == 'sell':
            self.call(0x565A20, [], COMP)
        elif kind == 'command':
            self.put(CMD + 8, CMDDATA); self.put(CMDDATA + 0xC, op['type'])
            self.put(CMD + 0x14, UDEF + op['definition'] * 0x400 if op['definition'] else 0)
            eax, _ = self.call(0x5667F0, [CMD], COMP)
            result = bool(eax & 0xFF)
        elif kind == 'tick':
            self.call(0x566880, [0, 0], COMP + 8)
        else:
            raise AssertionError(kind)
        return {'stock': self.words(0x2010000, self.n), 'payments': self.payments(), 'events': self.trace,
                'result': result}


R = random.Random(0x566880)
FRACTIONS = [0.5, 0.75, 1.0 / 3.0, 0.1, 0.6, 0.25]


def money():
    return bits(R.choice([0.0, 1.0, 3.0, 25.0, 60.0, 0.9999999, 2.9999998, 16777215.0, R.uniform(0, 120), R.uniform(0, 5)]))


def make_case(i):
    n = R.randint(1, 4)
    kinds = [R.choice(['Stockpiled', 'Stockpiled', 'Upkeep', 'LimitedConsumed']) for _ in range(n)]
    defs = [1, 2, 3, 4]

    def cost():
        return [money() for _ in range(n)]

    def times():
        start = R.uniform(0, 500)
        return {'world_time': bits(start + R.choice([0.0, 0.5, 2.0, 30.0, R.uniform(0, 60)])),
                'actor_time': bits(start + R.choice([0.0, -5.0, -60.0, R.uniform(-90, 5)])),
                'start_time': bits(start - R.choice([0.0, 1.0, 9.999, 10.0, R.uniform(0, 30)])),
                'interval': bits(R.choice([-1.0, 0.0, 5.0, 30.0, R.uniform(0, 60)])),
                'window': bits(R.choice([10.0, 5.0, R.uniform(0, 20)]))}
    flags = [ENABLED, ENABLED | UPGRADING, ENABLED | CONSTRUCTING, DISABLED, ENABLED | DISABLED, 0, ENABLED | BLOCKED]
    world = {'kingdom': R.random() < 0.9, 'state_active': R.random() < 0.7, 'kingdom_allows': R.random() < 0.85,
             'has_structure': R.random() < 0.7, 'destroyed': R.random() < 0.1, 'dissolving': R.random() < 0.25,
             'states': {'22': R.random() < 0.4, '20': R.random() < 0.4}, 'token': R.random() < 0.8,
             'actor_flags': R.choice(flags), 'upgrade_list': R.sample(defs, R.randint(0, 3)),
             'definition_flags': {str(d): R.choice([UPGRADE, UPGRADE, 0]) for d in defs}}
    world.update(times())
    ops = []
    for _ in range(R.randint(3, 8)):
        k = R.choice(['salvage', 'sell', 'command', 'command', 'tick', 'tick', 'tick'])
        op = {'kind': k}
        if k == 'salvage':
            op['accumulate'] = R.random() < 0.5
            op['out'] = [money() for _ in range(R.choice([0, n, n + 1, max(0, n - 1)]))]
        if k == 'command':
            op['type'] = R.choice([0x13, 0x13, 0x14, 0x15, 0x16, 0x12, 0x17])
            op['definition'] = R.choice(defs)
        if R.random() < 0.35:
            op['world'] = {'actor_flags': R.choice(flags), 'kingdom': R.random() < 0.9, 'dissolving': R.random() < 0.3,
                           'destroyed': R.random() < 0.1, **times()}
        ops.append(op)
    return {'name': f'random-{i}', 'catalog': {'resources': [{'id': f'r{j}', 'kind': kk, 'limited_provider': False,
                                                              'purchase_cost': [0] * n} for j, kk in enumerate(kinds)]},
            'limits': {'provided_maximum': 100, 'stockpiled_maximum': bits(R.choice([10000.0, 200.0]))},
            'stock': [bits(R.choice([0.0, 30.0, 150.0, R.uniform(0, 300)])) for _ in range(n)],
            'quotes': {str(d): [cost() for _ in range(R.randint(1, 3))] for d in defs},
            'payments': {'paid': cost(), 'upgrade_paid': cost(), 'upgrade': R.choice([0, 2])},
            'fraction': bits(R.choice(FRACTIONS)), 'world': world, 'ops': ops}


def directed():
    out = []
    base = make_case(-1)
    kinds = ['Stockpiled', 'Stockpiled', 'Upkeep', 'Stockpiled']
    base['catalog']['resources'] = [{'id': f'r{j}', 'kind': k, 'limited_provider': False, 'purchase_cost': [0] * 4}
                                    for j, k in enumerate(kinds)]
    base['stock'] = [bits(500.0), bits(80.0), bits(0.0), bits(9000.0)]
    base['limits']['stockpiled_maximum'] = bits(10000.0)
    base['name'] = 'directed-salvage-rounding'
    base['fraction'] = bits(0.5)
    # 0.5 * paid + 0.5 with ties and near-ties: 1 -> 1, 3 -> 2, 0.9999999 -> 0,
    # 2.9999998 -> floor(1.9999999) = 1. Precision-dependent salvage values come
    # from the random cases (fractions 1/3, 0.1, 0.6).
    paid = [bits(1.0), bits(3.0), bits(0.9999999), bits(2.9999998)]
    base['payments'] = {'paid': paid, 'upgrade_paid': [bits(10.0), 0, bits(4.0), 0], 'upgrade': 1}
    base['quotes'] = {'1': [[bits(40.0), 0, bits(5.0), 0], [bits(45.0), bits(1.0), 0, 0]], '2': [[bits(900.0), 0, 0, 0]],
                      '3': [[0, 0, bits(2.0), 0]], '4': [[bits(1.0), 0, 0, 0]]}
    base['world'].update({'kingdom': True, 'state_active': True, 'kingdom_allows': True, 'has_structure': True,
                          'destroyed': False, 'dissolving': False, 'token': True, 'states': {'22': False, '20': False},
                          'actor_flags': ENABLED, 'upgrade_list': [2, 1, 3],
                          'definition_flags': {'1': UPGRADE, '2': UPGRADE, '3': UPGRADE, '4': UPGRADE},
                          'world_time': bits(100.0), 'actor_time': bits(50.0), 'interval': bits(30.0),
                          'start_time': bits(95.0), 'window': bits(10.0)})
    base['ops'] = [{'kind': 'salvage', 'accumulate': False, 'out': []},
                   {'kind': 'salvage', 'accumulate': True, 'out': [bits(0.25)] * 4},
                   {'kind': 'salvage', 'accumulate': True, 'out': [bits(0.25)] * 6},
                   {'kind': 'salvage', 'accumulate': True, 'out': [bits(7.0)]},
                   {'kind': 'tick'},  # auto-upgrade: 2 unaffordable, 1 started (base quote logged)
                   {'kind': 'tick', 'world': {'actor_flags': ENABLED | UPGRADING}},  # recent start -> state 0x20 on
                   {'kind': 'command', 'type': 0x13, 'definition': 0},  # cancel upgrade
                   {'kind': 'command', 'type': 0x15, 'definition': 3},  # start upgrade 3
                   {'kind': 'command', 'type': 0x14, 'definition': 0},  # sell while upgrading
                   {'kind': 'command', 'type': 0x13, 'definition': 0, 'world': {'actor_flags': ENABLED | CONSTRUCTING}},
                   {'kind': 'command', 'type': 0x13, 'definition': 0, 'world': {'actor_flags': DISABLED}},
                   {'kind': 'command', 'type': 0x16, 'definition': 0},
                   {'kind': 'tick', 'world': {'dissolving': True, 'states': {'22': False, '20': True}, 'actor_flags': ENABLED}},
                   {'kind': 'tick', 'world': {'states': {'22': True, '20': True}}},
                   {'kind': 'tick', 'world': {'dissolving': False, 'interval': bits(50.0)}},  # not due (elapsed == 50)
                   {'kind': 'tick', 'world': {'interval': bits(-1.0)}},
                   {'kind': 'sell', 'world': {'kingdom': False}},
                   {'kind': 'sell', 'world': {'kingdom': True, 'destroyed': True}}]
    out.append(base)
    return out


def outside_domain(w, op):
    """4E1910 dereferences the owner when 566110 reaches the restriction check."""
    if w['kingdom']:
        return False
    if op['kind'] == 'tick' and w['token']:
        return True
    return op['kind'] == 'command' and op['type'] == 0x15


def main():
    cases = directed() + [make_case(i) for i in range(90)]
    fixtures, executions = [], 0
    for case in cases:
        for mode, cw in MODES:
            m = LifecycleMachine(cw, json.loads(json.dumps(case)))
            initial = m.payments()
            steps = []
            for op in case['ops']:
                w = dict(m.case['world'], **op.get('world', {}))
                if outside_domain(w, op):
                    continue
                steps.append({'op': op, 'expected': m.run(op)})
                executions += 1
            fixtures.append({'name': case['name'], 'precision': mode, 'case': case, 'initial': initial, 'steps': steps})
    doc = {'schema': 1, 'pins': pin(RANGES), 'executions': executions, 'fixtures': fixtures}
    out = Path(__file__).resolve().parent / 'native-construction-lifecycle-fixtures.json'
    out.write_text(json.dumps(doc, separators=(',', ':')) + '\n', encoding='utf-8')
    print(f'{len(fixtures)} fixtures, {executions} native executions')


if __name__ == '__main__':
    main()
