"""Original construction/upgrade completion in isolated x86.

Executed unchanged: 566AC0 completion dispatch, 565B00 OnConstructionFinished,
566400 OnUpgradeFinished, 4D18C0 settlement-founding test, 529C30, the creation
parameter construction, the GTID-table group lookup, and on upgrade failure
565700 upgrade cancel (4E39D0 credit, 45F180) as in native_construction.py.
Seams (recorded with arguments, bodies not executed): 4F9C10 creation, 586470,
4D21A0 CreateComponents, 404ED0/404F60/404F90 logs, 4F6760 destruction,
4E4470 kingdom completion (proven by native_registry.py), 4CF0F0 economy
refresh, 0xB36528 AI manager 79EA90/7B2350, actor virtuals +0x100/+0x158/
+0x15C/+0x24/+0x108, 584B40, 4D1350, 564ED0/564E10 statistics (proven by
native_completion_statistics.py), 42D490 events, 54C050 trigger, 588970/
58CAB0/588AE0/58CB80/5698C0/5814A0 components, 4CE6C0/4D11A0/controller
virtual +0x40, 42A590, 4D04B0, 4D3DD0, 4D0000, 4D7EA0, 4D64F0, 4D5EA0,
4D1930, 4D7F40, 40C230 messages, string helpers, allocator.
"""
from pathlib import Path
import json, random, sys
sys.path.insert(0, str(Path(__file__).resolve().parent))
from harness import MODES, bits, pin, ret_pop  # noqa: E402
from native_construction import (ConstructionMachine, ACTOR, AVT, ADATA, COMP, UDEF, K, WORLD,  # noqa: E402
                                 TRIGGERS, ENABLED, UPGRADE)
from unicorn.x86_const import UC_X86_REG_ECX  # noqa: E402

RANGES = {
    'dispatch_566AC0': (0x566AC0, 0x566BB4), 'construction_finished_565B00': (0x565B00, 0x565F12),
    'upgrade_finished_566400': (0x566400, 0x5667E3), 'founds_settlement_4D18C0': (0x4D18C0, 0x4D18FD),
    'upgrade_cancel_565700': (0x565700, 0x565762), 'credit_4E39D0': (0x4E39D0, 0x4E3A4C),
    'resize_45F180': (0x45F180, 0x45F1EE),
}
CREATED, SCOMP, AIMGR, AI = 0x2038000, 0x2038400, 0x2038800, 0x2038900
SETTLEMENT, SCOMP2, BUILDER, CONTROLLER = 0x2038A00, 0x2038B00, 0x2038C00, 0x2038D00
NOTIFY, NVT, GROUP, BREF = 0x2038E00, 0x2038F00, 0x2039000, 0x2039100
C78, C88, C90, C94 = 0x2039200, 0x2039300, 0x2039400, 0x2039500
ALLOC, AVT2 = 0x2039600, 0x2039700
GTIDS = 0x2100000
STUB_V100, STUB_IN_SETTLEMENT, STUB_SETTLEMENT, STUB_NAME = 0x3000120, 0x3000130, 0x3000140, 0x3000150
STUB_NOTIFY, STUB_NIL, STUB_FREE = 0x3000160, 0x3000170, 0x3000180
GROUP_GTID = (3 << 16) | 7
ACTOR_DEFINITION = 0xADA  # marker for the actor's own ActorData
LOGS = {0xA3B7A8: 'SettlementCreationFailed', 0xA3B8C8: 'CreateComponentsFailed', 0xA3BA44: 'Upgraded'}
COMPONENTS = {0x588970: ('C94', False), 0x58CAB0: ('C90', False), 0x588AE0: ('C94', True), 0x58CB80: ('C90', True),
              0x5698C0: ('C88', True), 0x5814A0: ('C78', True)}


class FinishMachine(ConstructionMachine):
    def __init__(self, cw, case):
        super().__init__(cw, case)
        w = case['world']
        self.put(AVT + 0x100, STUB_V100); self.put(AVT + 0x158, STUB_IN_SETTLEMENT)
        self.put(AVT + 0x15C, STUB_SETTLEMENT); self.put(AVT + 0x24, STUB_NAME)
        self.put(ALLOC, AVT2); self.put(AVT2 + 0xC, STUB_NIL); self.put(AVT2 + 4, STUB_FREE)
        self.put(ADATA + 0x430, UDEF + w['settlement_definition'] * 0x400 if w['settlement_definition'] else 0)
        self.put(ACTOR + 0x14, 0x50033)
        self.put_words(ACTOR + 0x20, w['placement'])
        self.put(SETTLEMENT + 0x98, SCOMP2)
        self.put(CONTROLLER + 0x70, NOTIFY); self.put(NOTIFY, NVT); self.put(NVT + 0x40, STUB_NOTIFY)
        self.put(0xB364C0, GTIDS)
        self.put(ACTOR + 0x158, GROUP_GTID)
        self.put(CREATED + 0x14, 0x50044)
        for address, handler in [
                (STUB_V100, self._trace('virtual_100', 'virtual_100')),
                (STUB_IN_SETTLEMENT, self._trace('in_settlement', 'in_settlement')),
                (STUB_SETTLEMENT, lambda m, esp: (SETTLEMENT, 0)),
                (STUB_NAME, lambda m, esp: (self._fill(self.read(esp + 4)), 4)),
                (STUB_NIL, lambda m, esp: (self._string() - 0x10, 0)),
                (STUB_FREE, ret_pop(1)),
                (0x856370, lambda m, esp: (ALLOC, 0)),
                (0x4F9C10, self._create),
                (0x586470, self._join),
                (0x4D21A0, self._components_created),
                (0x404ED0, self._log), (0x404F60, self._log), (0x404F90, self._log),
                (0x4E4470, self._kingdom_completed),
                (0x4CF0F0, self._simple('refresh_economy', ACTOR, 0)),
                (0x79EA90, self._ai_for),
                (0x7B2350, self._ai_constructed),
                (0x584B40, self._simple('refresh_settlement', SCOMP2, 0)),
                (0x4D1350, self._refresh_organization),
                (0x564ED0, self._simple('completion_statistics', COMP, 0)),
                (0x564E10, self._upgrade_statistics),
                (0x42D490, self._event),
                (0x54C050, self._trigger),
                (0x4CE6C0, self._controller),
                (0x4D11A0, self._busy),
                (STUB_NOTIFY, self._notify),
                (0x42A590, self._built),
                (0x4D04B0, self._simple('prepare_upgrade', ACTOR, 0)),
                (0x4D3DD0, self._leave),
                (0x4D0000, self._rejoin),
                (0x4D7EA0, self._simple('restore_after_failed_upgrade', ACTOR, 0)),
                (0x4D5EA0, self._transfer),
                (0x4D1930, self._default_name),
                (0x4D7F40, self._copy_name)]:
            self.seam(address, handler)
        for address in COMPONENTS:
            self.seam(address, self._component)

    def _fill(self, out):
        self.put(out, self._string())
        return out

    def _trace(self, kind, flag):
        def handler(m, esp):
            self.trace.append({'kind': kind})
            return int(self.op[flag]), 0
        return handler

    def _simple(self, kind, this, pop):
        def handler(m, esp):
            assert self.uc.reg_read(UC_X86_REG_ECX) == this, (kind, hex(self.uc.reg_read(UC_X86_REG_ECX)))
            self.trace.append({'kind': kind})
            return None, pop
        return handler

    def _create(self, m, esp):
        p = self.read(esp + 4)
        params = {'definition': self.ident(self.read(p + 4)), 'owner': self.read(p + 0x28) != 0,
                  'placement': self.words(p + 8, 5), 'mode': self.read(p + 0x38), 'source': self.read(p + 0x3C)}
        assert self.read(p) == 0xA25B28
        self.trace.append({'kind': 'replace_create', 'params': params})
        if not self.op['created']:
            return 0, 4
        self.put(CREATED + 4, UDEF + params['definition'] * 0x400 if params['definition'] else 0)
        self.put(CREATED + 0x98, SCOMP if self.op['created_settlement'] else 0)
        return CREATED, 4

    def _join(self, m, esp):
        assert self.uc.reg_read(UC_X86_REG_ECX) == SCOMP and self.read(esp + 4) == ACTOR
        self.trace.append({'kind': 'join_new_settlement'})
        return int(self.op['joins']), 4

    def _components_created(self, m, esp):
        assert self.uc.reg_read(UC_X86_REG_ECX) == ACTOR and self.read(esp + 4) == 1
        self.trace.append({'kind': 'create_components'})
        return int(self.op['components_created']), 4

    def _log(self, m, esp):
        self.trace.append({'kind': 'log_finish', 'log': LOGS[self.read(esp + 0x14)]})
        return None, 0

    def _destroy(self, m, esp):
        assert self.uc.reg_read(UC_X86_REG_ECX) == WORLD and self.read(esp + 4) == ACTOR
        self.trace.append({'kind': 'destroy_replaced' if self.read(esp + 8) else 'destroy_actor'})
        return None, 8

    def _kingdom_completed(self, m, esp):
        assert self.uc.reg_read(UC_X86_REG_ECX) == K and self.read(esp + 4) == ACTOR
        self.trace.append({'kind': 'kingdom_actor_completed'})
        return None, 4

    def _ai_for(self, m, esp):
        assert self.uc.reg_read(UC_X86_REG_ECX) == AIMGR
        self.trace.append({'kind': 'ai_for', 'owner': self.read(esp + 4) != 0})
        return (AI if self.op['ai'] else 0), 4

    def _ai_constructed(self, m, esp):
        assert self.uc.reg_read(UC_X86_REG_ECX) == AI and self.read(esp + 4) == ACTOR
        self.trace.append({'kind': 'ai_constructed'})
        return None, 4

    def _refresh_organization(self, m, esp):
        this = self.uc.reg_read(UC_X86_REG_ECX)
        assert this in (ACTOR, CREATED)
        self.trace.append({'kind': 'refresh_organization', 'created': this == CREATED})
        return None, 0

    def _upgrade_statistics(self, m, esp):
        assert self.uc.reg_read(UC_X86_REG_ECX) == COMP and self.read(esp + 4) == CREATED
        self.trace.append({'kind': 'upgrade_statistics'})
        return None, 4

    def _event(self, m, esp):
        assert self.read(esp + 0x10) == ACTOR
        self.trace.append({'kind': 'completion_event', 'event': self.read(esp + 8), 'owner': self.read(esp + 0xC) != 0})
        return None, 0

    def _trigger(self, m, esp):
        assert self.uc.reg_read(UC_X86_REG_ECX) == TRIGGERS
        definition = self.read(esp + 8)
        self.trace.append({'kind': 'construction_trigger', 'owner': self.read(esp + 4) != 0,
                           'definition': ACTOR_DEFINITION if definition == ADATA else self.ident(definition)})
        return None, 8

    def _component(self, m, esp):
        this = self.uc.reg_read(UC_X86_REG_ECX)
        name = {C78: 'C78', C88: 'C88', C90: 'C90', C94: 'C94'}[this]
        caller_entry = self.current_seam
        expected, with_created = COMPONENTS[caller_entry]
        assert expected == name
        created = None
        if with_created:
            assert self.read(esp + 4) == CREATED
            created = True
        self.trace.append({'kind': 'complete_component', 'component': name, 'created': created})
        return None, 4 if with_created else 0

    def _code(self, uc, address, size, data):
        if address in COMPONENTS:
            self.current_seam = address
        return ConstructionMachine._code(self, uc, address, size, data)

    def _controller(self, m, esp):
        assert self.uc.reg_read(UC_X86_REG_ECX) == BUILDER
        self.trace.append({'kind': 'builder_controller'})
        return CONTROLLER, 0

    def _busy(self, m, esp):
        assert self.uc.reg_read(UC_X86_REG_ECX) == CONTROLLER
        self.trace.append({'kind': 'controller_busy'})
        return int(self.op['controller_busy']), 0

    def _notify(self, m, esp):
        assert self.uc.reg_read(UC_X86_REG_ECX) == NOTIFY and self.read(esp + 4) == ACTOR
        self.trace.append({'kind': 'notify_builder'})
        return None, 4

    def _built(self, m, esp):
        assert self.uc.reg_read(UC_X86_REG_ECX) == ADATA + 0xE4
        assert self.read(esp + 4) == ACTOR and self.read(esp + 8) == 0
        self.trace.append({'kind': 'built_trigger'})
        return None, 8

    def _leave(self, m, esp):
        assert self.uc.reg_read(UC_X86_REG_ECX) == ACTOR and self.read(esp + 4) == 0
        self.trace.append({'kind': 'leave_group'})
        return None, 4

    def _rejoin(self, m, esp):
        assert self.uc.reg_read(UC_X86_REG_ECX) == ACTOR and self.read(esp + 4) == GROUP
        self.trace.append({'kind': 'rejoin_group'})
        return None, 4

    def _transfer(self, m, esp):
        assert self.uc.reg_read(UC_X86_REG_ECX) == CREATED and self.read(esp + 4) == ACTOR
        self.trace.append({'kind': 'transfer_state'})
        return None, 4

    def _default_name(self, m, esp):
        assert self.uc.reg_read(UC_X86_REG_ECX) == ACTOR
        self.trace.append({'kind': 'has_default_name'})
        return int(self.op['default_name']), 0

    def _copy_name(self, m, esp):
        assert self.uc.reg_read(UC_X86_REG_ECX) == CREATED
        self.trace.append({'kind': 'copy_name'})
        return None, 4

    def _actor_state(self, m, esp):
        this = self.uc.reg_read(UC_X86_REG_ECX)
        assert this in (ACTOR, CREATED)
        kind = 'actor_state' if this == ACTOR else 'created_state'
        self.trace.append({'kind': kind, 'state': self.read(esp + 4), 'enabled': bool(self.read(esp + 8) & 0xFF)})
        return None, 8

    def _message(self, m, esp):
        if self.read(esp) == 0x566637:
            assert self.read(esp + 8) == 0x11 and self.read(esp + 12) == ACTOR
            self.trace.append({'kind': 'notify_creation_failed', 'kingdom': self.read(esp + 16) != 0})
            return None, 0
        return ConstructionMachine._message(self, m, esp)

    def run(self, op):
        self.op = op
        self.trace = []
        w = self.case['world']
        w.update(op.get('world', {}))
        flags = w['actor_flags']
        self.put(ACTOR + 0x100, flags & 0xFFFFFFFF); self.put(ACTOR + 0x104, flags >> 32)
        self.put8(ACTOR + 8, int(w['destroyed']))
        for off, obj, key in ((0x78, C78, 'c78'), (0x88, C88, 'c88'), (0x90, C90, 'c90'), (0x94, C94, 'c94')):
            self.put(ACTOR + off, obj if op['components'][key] else 0)
        self.put(0xB36528, AIMGR if op['ai_manager'] else 0)
        self.put16(GTIDS + 7 * 2 + 4, 3 if op['group'] else 2)
        self.put(GTIDS + 7 * 4 + 0x20004, GROUP)
        self.put(BREF, BUILDER if op['builder'] else 0)
        self.call(0x566AC0, [BREF if op['builder_ref'] else 0], COMP)
        return {'stock': self.words(0x2010000, self.n), 'payments': self.payments(), 'events': self.trace}


R = random.Random(0x565B00)
CONSTRUCTING, UPGRADING = 1 << 21, 1 << 29


def make_case(i):
    n = R.randint(1, 3)
    kinds = [R.choice(['Stockpiled', 'Stockpiled', 'Upkeep']) for _ in range(n)]

    def cost():
        return [bits(R.choice([0.0, 5.0, 25.0, R.uniform(0, 80)])) for _ in range(n)]
    world = {'kingdom': R.random() < 0.85, 'state_active': True, 'kingdom_allows': True, 'has_structure': True,
             'destroyed': False, 'actor_flags': ENABLED, 'upgrade_list': [1, 2],
             'definition_flags': {str(d): UPGRADE for d in (1, 2, 3, 4)},
             'settlement_definition': R.choice([0, 0, 3]), 'placement': [bits(R.uniform(0, 500)) for _ in range(5)]}
    ops = []
    for _ in range(R.randint(3, 7)):
        flags = R.choice([CONSTRUCTING, CONSTRUCTING, UPGRADING, UPGRADING, CONSTRUCTING | UPGRADING, 0,
                          CONSTRUCTING | (1 << 20), UPGRADING | (1 << 20)]) | ENABLED
        op = {'world': {'actor_flags': flags, 'kingdom': R.random() < 0.85},
              'components': {k: R.random() < 0.5 for k in ('c78', 'c88', 'c90', 'c94')},
              'virtual_100': R.random() < 0.3, 'created': R.random() < 0.85, 'created_settlement': R.random() < 0.85,
              'joins': R.random() < 0.85, 'components_created': R.random() < 0.9, 'ai_manager': R.random() < 0.5,
              'ai': R.random() < 0.6, 'in_settlement': R.random() < 0.5, 'builder_ref': R.random() < 0.8,
              'builder': R.random() < 0.8, 'controller_busy': R.random() < 0.4, 'group': R.random() < 0.5,
              'default_name': R.random() < 0.5}
        ops.append(op)
    return {'name': f'random-{i}', 'catalog': {'resources': [{'id': f'r{j}', 'kind': kk, 'limited_provider': False,
                                                              'purchase_cost': [0] * n} for j, kk in enumerate(kinds)]},
            'limits': {'provided_maximum': 100, 'stockpiled_maximum': bits(10000.0)},
            'stock': [bits(R.choice([30.0, 150.0, R.uniform(0, 300)])) for _ in range(n)],
            'quotes': {str(d): [cost()] for d in (1, 2, 3, 4)},
            'payments': {'paid': cost(), 'upgrade_paid': cost(), 'upgrade': R.choice([1, 2])},
            'world': world, 'ops': ops}


def main():
    cases = [make_case(i) for i in range(90)]
    fixtures, executions = [], 0
    for case in cases:
        for mode, cw in MODES:
            m = FinishMachine(cw, json.loads(json.dumps(case)))
            initial = m.payments()
            steps = []
            for op in case['ops']:
                flags = op['world']['actor_flags']
                if not flags & CONSTRUCTING and flags & UPGRADING and not m.read(COMP + 0x28):
                    continue  # 0x4F9C10 dereferences the null upgrade definition: outside domain
                steps.append({'op': op, 'expected': m.run(op)})
                executions += 1
            fixtures.append({'name': case['name'], 'precision': mode, 'case': case, 'initial': initial, 'steps': steps})
    doc = {'schema': 1, 'pins': pin(RANGES), 'executions': executions, 'fixtures': fixtures}
    out = Path(__file__).resolve().parent / 'native-construction-finish-fixtures.json'
    out.write_text(json.dumps(doc, separators=(',', ':')) + '\n', encoding='utf-8')
    print(f'{len(fixtures)} fixtures, {executions} native executions')


if __name__ == '__main__':
    main()
