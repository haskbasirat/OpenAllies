"""Original denizen/HQ/transport purchases and refunds in isolated x86.

Executed unchanged: 56D5A0/56D7C0/56DA00 denizen purchases, 574A10/574C10
HQ attach, 5735D0 attach cost with 4CD770 scale and 4CD230/4CD160/4CD440/
4CCF10 ResourceData helpers, 58BBA0 transport refund with 4CD250, kingdom
4E20A0/4E1690/4E39D0. Seams (recorded): owner virtual, 56CC10 denizen
creation, 4CA330/4CA760/4CA2D0 ability context, created virtual +0x10,
77DC50 telemetry, 40C230 messages and CString helpers, 56FA30 organization
cost (fixture vector), 574810 attach permission, 573790/574260 attachment
record, 5783B0 organization attach, 42D470 event, 4CF0F0 economy refresh.
"""
from pathlib import Path
import json, random, sys
sys.path.insert(0, str(Path(__file__).resolve().parent))
from harness import Machine, MODES, bits, pin, ret_pop  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX  # noqa: E402

RANGES = {'denizen_56D5A0': (0x56D5A0, 0x56D7BD), 'denizen_56D7C0': (0x56D7C0, 0x56D9F8),
          'denizen_56DA00': (0x56DA00, 0x56DC50), 'attach_gate_574A10': (0x574A10, 0x574BE2),
          'attach_574C10': (0x574C10, 0x574DA1), 'attach_cost_5735D0': (0x5735D0, 0x57364D),
          'scale_4CD770': (0x4CD770, 0x4CD894), 'transport_refund_58BBA0': (0x58BBA0, 0x58BCAA),
          'resource_clear_4CD250': (0x4CD250, 0x4CD268), 'can_afford_4E20A0': (0x4E20A0, 0x4E20F7),
          'debit_4E1690': (0x4E1690, 0x4E16C5), 'credit_4E39D0': (0x4E39D0, 0x4E3A4C)}
K, STOCK = 0x2000000, 0x2010000
REG, TABLE, DEFS, MANAGER = 0x2020000, 0x2020100, 0x2021000, 0x2022000
CACHE, LIMITS = 0x2024000, 0x2024800
ACT, AVT, AD = 0x2030000, 0x2030800, 0x2031000
DZ, HQ, TC, ORG, OVT, ORGAD, TDEF = 0x2032000, 0x2032100, 0x2032200, 0x2033000, 0x2033800, 0x2034000, 0x2035000
CREATED, CVT, SLOTCELL = 0x2036000, 0x2036800, 0x2037000
STUB_OWNER, STUB_NAME, STUB_USE, STUB_ORG_OWNER = 0x3000100, 0x3000110, 0x3000120, 0x3000130
RD_VTABLE = 0xA26F44


class PurchaseMachine(Machine):
    def __init__(self, cw, case):
        super().__init__(cw)
        self.case = case
        n = len(case['catalog']['resources'])
        self.n = n
        self.put(K + 0x1C4, STOCK); self.put(K + 0x1C8, n); self.put(K + 0x1CC, n)
        self.put_words(STOCK, case['stock'])
        self.put(0xB36530, REG); self.put(REG + 0x34, TABLE); self.put(REG + 0x38, n)
        self.put(0xB3A660, MANAGER); self.put(MANAGER + 0x28, n)
        for i, d in enumerate(case['catalog']['resources']):
            self.put(TABLE + 4 * i, DEFS + 0x40 * i)
            self.put(DEFS + 0x40 * i + 0x18, {'Stockpiled': 0, 'Upkeep': 1, 'LimitedConsumed': 2}[d['kind']])
        self.put(0xB39AB0, CACHE); self.put(CACHE + 0x1B8, LIMITS); self.put(LIMITS, case['limits']['stockpiled_maximum'])
        self.put(CACHE + 0xC0, LIMITS + 4); self.put(LIMITS + 4, case['attach_factor'])
        self.put(ACT, AVT); self.put(AVT + 0x108, STUB_OWNER); self.put(AVT + 0x24, STUB_NAME); self.put(ACT + 4, AD)
        self.inner(AD + 0x3F0, case['denizen_cost'][0]); self.inner(AD + 0x3FC, case['denizen_cost'][1])
        for comp in (DZ, HQ, TC):
            self.put(comp + 4, ACT)
        self.put(ORG, OVT); self.put(OVT + 0x24, STUB_NAME); self.put(OVT + 0x108, STUB_ORG_OWNER); self.put(ORG + 4, ORGAD)
        self.put(CREATED, CVT); self.put(CVT + 0x10, STUB_USE)
        self.put(TDEF + 0x10, 0x2F00000)
        self.trace = []
        self.seam(0x6F7810, lambda m, esp: (self.alloc(self.read(esp + 4)), 0))
        self.seam(0x6F7430, ret_pop(0))
        self.seam(STUB_OWNER, self._owner)
        self.seam(STUB_NAME, self._name)
        self.seam(STUB_ORG_OWNER, lambda m, esp: (K, 0))  # event argument only
        self.seam(0x56CC10, self._create)
        self.seam(0x4CA330, ret_pop(1))
        self.seam(0x4CA760, ret_pop(5))
        self.seam(0x4CA2D0, ret_pop(0))
        self.seam(STUB_USE, self._use)
        self.seam(0x77DC50, self._telemetry)
        self.seam(0x401B00, self._string_ctor)
        self.seam(0x4B9F40, self._format)
        self.seam(0x5C4D10, lambda m, esp: (self._fill(self.read(esp + 4)), 0))
        self.seam(0x4012C0, lambda m, esp: (self.read(esp + 4), 0))
        self.seam(0x401560, self._assign)
        self.seam(0x40C230, self._message)
        self.seam(0x56FA30, self._org_cost)
        self.seam(0x574810, self._allowed)
        self.seam(0x573790, lambda m, esp: (self.uc.reg_read(UC_X86_REG_ECX), 4))
        self.seam(0x574260, self._slot)
        self.seam(0x5783B0, self._attach_org)
        self.seam(0x42D470, self._event)
        self.seam(0x4CF0F0, self._refresh)
        self.watch = {0x4E1690: 'debit', 0x4E39D0: 'credit'}

    # memory helpers
    def resource(self, address, values):
        data = self.alloc(4 * max(1, len(values)))
        self.put_words(data, values)
        self.put(address, RD_VTABLE); self.put(address + 4, data); self.put(address + 8, len(values)); self.put(address + 12, len(values))

    def inner(self, address, values):
        data = self.alloc(4 * max(1, len(values)))
        self.put_words(data, values)
        self.put(address, data); self.put(address + 4, len(values)); self.put(address + 8, len(values))

    def rvalues(self, address):
        return self.words(self.read(address + 4), self.read(address + 8))

    def _header(self):
        h = self.alloc(0x40)
        self.put(h + 8, 16); self.put(h + 0xC, 100)
        return h

    def _fill(self, out):
        self.put(out, self._header() + 0x10)
        return out

    def _string_ctor(self, m, esp):
        return self._fill(self.uc.reg_read(UC_X86_REG_ECX)), 4

    def _format(self, m, esp):
        return self._fill(self.read(esp + 4)), 8

    def _assign(self, m, esp):
        this = self.uc.reg_read(UC_X86_REG_ECX)
        self.put(this, self.read(self.read(esp + 4)))
        return this, 4

    def _name(self, m, esp):
        return self._fill(self.read(esp + 4)), 4

    def _code(self, uc, address, size, _data):
        name = getattr(self, 'watch', {}).get(address)
        if name:
            from unicorn.x86_const import UC_X86_REG_ESP
            esp = uc.reg_read(UC_X86_REG_ESP)
            vec = self.read(esp + 4)
            self.trace.append({'kind': name, 'values': self.words(self.read(vec), self.n)})
        return Machine._code(self, uc, address, size, _data)

    def _owner(self, m, esp):
        self.trace.append({'kind': 'owner'})
        return (K if self.case['world']['kingdom'] else 0), 0

    def _create(self, m, esp):
        mode = self.read(esp + 4)
        assert self.read(esp + 8) == 1 and self.read(esp + 12) == 1
        self.trace.append({'kind': 'create_denizen', 'mode': mode})
        return (CREATED if self.op.get('created', True) else 0), 12

    def _use(self, m, esp):
        self.trace.append({'kind': 'use_denizen', 'mode': self.read(esp + 8)})
        return None, 8

    def _telemetry(self, m, esp):
        self.trace.append({'kind': 'telemetry'})
        return None, 8

    def _message(self, m, esp):
        caller = self.read(esp)
        if 0x56D5A0 <= caller < 0x56DC50:
            name = 'denizen_unaffordable'
        elif 0x574A10 <= caller < 0x574BE2:
            name = 'attach_unaffordable'
        elif 0x58BBA0 <= caller < 0x58BCAA:
            name = 'unloading_failed'
        else:
            raise AssertionError(hex(caller))
        self.trace.append({'kind': name, 'kingdom': self.read(esp + 16) != 0})
        return None, 0

    def _org_cost(self, m, esp):
        out, org = self.read(esp + 4), self.read(esp + 8)
        assert org == ORG
        self.trace.append({'kind': 'organization_cost'})
        values = self.case['organization_cost']
        data = self.read(out + 4)
        self.put_words(data, values)
        return out, 0

    def _allowed(self, m, esp):
        assert self.read(esp + 4) == ORGAD
        notify = bool(self.read(esp + 8) & 0xFF)
        self.trace.append({'kind': 'attach_allowed', 'notify': notify})
        return int(self.case['world']['attach_allowed']), 8

    def _slot(self, m, esp):
        self.trace.append({'kind': 'create_attachment', 'slot': self.read(esp + 4)})
        return SLOTCELL, 4

    def _attach_org(self, m, esp):
        assert self.uc.reg_read(UC_X86_REG_ECX) == ORG + 0x400 and self.read(esp + 4) == ACT
        self.trace.append({'kind': 'attach_to_organization'})
        return None, 4

    def _event(self, m, esp):
        assert self.read(esp + 8) == 0x3C
        self.trace.append({'kind': 'attach_event'})
        return None, 0

    def _refresh(self, m, esp):
        assert self.uc.reg_read(UC_X86_REG_ECX) == ACT
        self.trace.append({'kind': 'refresh_economy'})
        return None, 0

    def run(self, op):
        self.op = op
        self.trace = []
        w = self.case['world']
        w.update(op.get('world', {}))
        kind = op['kind']
        result = None
        if kind == 'denizen':
            addr, args = {'AirAttackTarget': (0x56D5A0, [0x5555]), 'AirAttackLocation': (0x56D7C0, [bits(3.0), bits(4.0)]),
                          'AirRecon': (0x56DA00, [bits(5.0), bits(6.0)])}[op['purchase']]
            eax, _ = self.call(addr, args, DZ); result = bool(eax & 0xFF)
        elif kind in ('attach_gate', 'attach'):
            org = ORG if op['organization'] else 0
            self.put(ORG + 0x7C, ORG + 0x400 if op.get('component', True) else 0)
            if kind == 'attach_gate':
                eax, _ = self.call(0x574A10, [org, int(op['check_cost']), int(op['notify'])], HQ)
            else:
                eax, _ = self.call(0x574C10, [org, int(op['pay']), op['slot']], HQ)
            result = bool(eax & 0xFF)
        elif kind == 'transport_refund':
            self.resource(TC + 8, op['cost']); self.resource(TC + 0x18, op['upkeep'])
            self.put(TC + 0x28, TDEF); self.put8(TC + 0x38, int(op['no_refund']))
            self.call(0x58BBA0, [], TC)
            result = {'cost': self.rvalues(TC + 8), 'upkeep': self.rvalues(TC + 0x18)}
        return {'stock': self.words(STOCK, self.n), 'events': self.trace, 'result': result}


R = random.Random(0x574C10)


def make_case(i):
    n = R.randint(1, 4)
    kinds = [R.choice(['Stockpiled', 'Stockpiled', 'Upkeep']) for _ in range(n)]
    if 'Stockpiled' not in kinds:
        kinds[0] = 'Stockpiled'

    def vec(k=n):
        return [bits(R.choice([0.0, 10.0, 25.5, 80.0, R.uniform(0, 150)])) for _ in range(k)]
    ops = []
    for _ in range(R.randint(3, 8)):
        k = R.choice(['denizen', 'denizen', 'attach_gate', 'attach', 'attach', 'transport_refund'])
        op = {'kind': k}
        if k == 'denizen':
            op.update(purchase=R.choice(['AirAttackTarget', 'AirAttackLocation', 'AirRecon']), created=R.random() < 0.8)
        elif k == 'attach_gate':
            op.update(organization=R.random() < 0.9, component=R.random() < 0.9, check_cost=R.random() < 0.8, notify=R.random() < 0.5)
        elif k == 'attach':
            op.update(organization=R.random() < 0.9, component=R.random() < 0.9, pay=R.random() < 0.8, slot=R.randint(0, 5))
        else:
            op.update(cost=vec(), upkeep=vec(), no_refund=R.random() < 0.2)
        if R.random() < 0.3:
            op['world'] = {'kingdom': R.random() < 0.8, 'attach_allowed': R.random() < 0.8}
        if k == 'denizen' and not op.get('world', {}).get('kingdom', True):
            op['world']['kingdom'] = True  # native dereferences the owner kingdom
        ops.append(op)
    return {'name': f'random-{i}', 'catalog': {'resources': [{'id': f'r{j}', 'kind': kk, 'limited_provider': False,
                                                              'purchase_cost': [0] * n} for j, kk in enumerate(kinds)]},
            'limits': {'provided_maximum': 100, 'stockpiled_maximum': bits(R.choice([10000.0, 150.0]))},
            'stock': vec(), 'denizen_cost': [vec(), vec()], 'organization_cost': vec(R.choice([n, max(1, n - 1)])),
            'attach_factor': R.choice([bits(1.0), bits(0.5), bits(1.0 / 3.0), bits(1.75)]),
            'world': {'kingdom': True, 'attach_allowed': True}, 'ops': ops}


def main():
    cases = [make_case(i) for i in range(100)]
    fixtures, executions = [], 0
    for case in cases:
        for mode, cw in MODES:
            m = PurchaseMachine(cw, json.loads(json.dumps(case)))
            steps = []
            for op in case['ops']:
                if op['kind'] == 'denizen' and not m.case['world']['kingdom'] and not op.get('world', {}).get('kingdom'):
                    continue
                steps.append({'op': op, 'expected': m.run(op)})
                executions += 1
            fixtures.append({'name': case['name'], 'precision': mode, 'case': case, 'steps': steps})
    doc = {'schema': 1, 'pins': pin(RANGES), 'executions': executions, 'fixtures': fixtures}
    out = Path(__file__).resolve().parent / 'native-purchase-fixtures.json'
    out.write_text(json.dumps(doc, separators=(',', ':')) + '\n', encoding='utf-8')
    print(f'{len(fixtures)} fixtures, {executions} native executions')


if __name__ == '__main__':
    main()
