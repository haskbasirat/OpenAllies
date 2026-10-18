"""Original research admission/payment/queue/completion in isolated x86.

Extends research/phase6-production/native_production.py (ProductionMachine), so
the structure queue, durations, scheduler seams, upkeep/refund arithmetic and
kingdom economy execute exactly as accepted in phase 6. Additionally executed
unchanged here: 583F30 research refund (credit + release), 584790 request, 5845F0 affordability, 5844A0 blocked reason,
588A40 queue lookup, 584030 completed lookup, 4C3D70 ResearchCost copy,
4CD160/4CCF10 vector copy/destroy, 589F00/589830 research item create/append,
4E2310/4E2360/4E2390 kingdom research list, 4F1430 list remove, 584120
completion (also when reached from 589F90), 58E870 score add.

Seams (bodies not executed, recorded as events): actor virtual +0x34 (state
0x23), +0x108 (owner kingdom; recorded only when called from research code),
4CED40 organization, 581280/4E0FB0 property containers, 5842F0 requirements,
40C230 messages, 42D490 research_complete event, 4CEF10 apply property,
4E3AD0 kingdom AddProperty, 42A590 researched trigger, string/CString helpers,
allocator. Output: research/phase8-economy/native-research-fixtures.json.
"""
from pathlib import Path
import copy, importlib.util, json, random, struct, sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
spec = importlib.util.spec_from_file_location('native_production', ROOT / 'research/phase6-production/native_production.py')
production = importlib.util.module_from_spec(spec)
spec.loader.exec_module(production)
economy = production.economy
from unicorn.x86_const import UC_X86_REG_ESP, UC_X86_REG_ECX, UC_X86_REG_EAX  # noqa: E402
sys.path.insert(0, str(HERE))
import harness  # noqa: E402  (null-page guard only)

bits = economy.bits
RESEARCH_CODE = [(0x583F30, 0x583F61), (0x584030, 0x584940)]
PINNED = {'research_refund_583F30': (0x583F30, 0x583F61), 'research_584030': (0x584030, 0x584052), 'research_584120': (0x584120, 0x584210),
          'research_5844A0': (0x5844A0, 0x584592), 'research_5845F0': (0x5845F0, 0x584790),
          'research_584790': (0x584790, 0x584933), 'queue_588A40': (0x588A40, 0x588A74),
          'queue_589830': (0x589830, 0x58991A), 'queue_589F00': (0x589F00, 0x589F84),
          'kingdom_4E2310': (0x4E2310, 0x4E2359), 'kingdom_4E2360': (0x4E2360, 0x4E2386),
          'kingdom_4E2390': (0x4E2390, 0x4E23C7), 'cost_4C3D70': (0x4C3D70, 0x4C3E0B),
          'list_remove_4F1430': (0x4F1430, 0x4F1493), 'score_58E870': (0x58E870, 0x58E883)}


class ResearchMachine(production.ProductionMachine):
    RESEARCH = 0x2061400  # actor+0xB4 (phase6 harness already links +4 to the actor)
    ORG = 0x2066800
    ORG_PROPS = 0x2066900
    PROPDEF = 0x207C000
    STATS = 0x207D000

    def __init__(self, case, cw):
        super().__init__(case, cw)
        harness.guard_null_page(self)
        r = case['research']
        self.put(self.VTABLE + 0x34, 0x3000120)
        self.put(self.RESEARCH + 4, self.ACTOR)
        self.put(self.ORG + 0x78, self.ORG_PROPS if r['organization_properties'] else 0)
        for item in r['items']:
            p = self.definition(item['id'])
            d = item['definition']
            self.put(p, self.PROPDEF + d * 0x40 if d else 0)
            if d:
                self.put(self.PROPDEF + d * 0x40 + 0x10, 0x207F000 + d)
            self.resource(p + 0x18, item['cost'])
            self.put(p + 0x2C, item['kind'])
        self.write_list(self.RESEARCH + 0x10, [self.definition(i) for i in r['completed']])
        self.write_list(self.KINGDOM + 0x324, [self.definition(x['item']) for x in r['kingdom_in_progress']])
        self.put(self.KINGDOM + 0x32C, self.STATS if r['statistics'] else 0)
        self.put(self.STATS + 0x28 + 4 * 2, r['techs_researched'])

    def write_list(self, head, values):
        nodes = [self.allocate(8) for _ in values]
        for i, (node, value) in enumerate(zip(nodes, values)):
            self.put(node, value)
            self.put(node + 4, nodes[i + 1] if i + 1 < len(nodes) else 0)
        self.put(head, nodes[0] if nodes else 0)
        self.put(head + 4, nodes[-1] if nodes else 0)

    def read_list(self, head):
        out, node = [], self.read(head)
        while node:
            out.append(self.read(node))
            node = self.read(node + 4)
        return out

    def ident(self, pointer):
        return (pointer - self.DEFINITIONS) // 0x400 if pointer else 0

    def research_state(self):
        items = {self.definition(i['id']): i for i in self.case['research']['items']}
        return {'completed': [self.ident(p) for p in self.read_list(self.RESEARCH + 0x10)],
                'kingdom_in_progress': [{'item': self.ident(p), 'definition': items[p]['definition']}
                                        for p in self.read_list(self.KINGDOM + 0x324)],
                'techs_researched': self.read(self.STATS + 0x28 + 4 * 2)}

    def observe(self, uc, address, size, user):
        stack = uc.reg_read(UC_X86_REG_ESP)
        this = uc.reg_read(UC_X86_REG_ECX)
        arg = lambda i: self.read(stack + 4 + 4 * i)
        r = self.case['research']
        w = self.case['world']
        image = economy.ORIGINAL.base <= address < economy.ORIGINAL.base + economy.ORIGINAL.size
        if image:
            raw = economy.ORIGINAL.at(address, size)
            assert raw == economy.ACCEPTED.at(address, size), hex(address)
            self.executed[address] = raw
        caller = self.read(stack)
        from_research = any(lo <= caller < hi for lo, hi in RESEARCH_CODE)
        if address == 0x3000100 and from_research:
            self.event('kingdom')
            return self.ret(self.KINGDOM if w['kingdom'] else 0)
        if address == 0x3000120:
            assert arg(0) == 0x23
            self.event('research_state_active', state=arg(0))
            return self.ret(int(r['state_active']), 4)
        if address == 0x4CED40:
            self.event('organization')
            return self.ret(self.ORG if r['organization'] else 0)
        if address == 0x581280:
            assert this == self.ORG_PROPS
            self.event('organization_has_property', definition=(arg(0) - self.PROPDEF) // 0x40 if arg(0) else 0)
            return self.ret(int(r['organization_has_property']), 8)
        if address == 0x4E0FB0:
            assert this == self.KINGDOM and arg(1) == 0
            self.event('kingdom_has_property', definition=(arg(0) - self.PROPDEF) // 0x40 if arg(0) else 0)
            return self.ret(int(r['kingdom_has_property']), 8)
        if address == 0x5842F0:
            self.event('research_requirements', kingdom=arg(0) != 0, organization=arg(1) != 0,
                       item=self.ident(arg(3)), notify=bool(arg(4) & 255))
            assert arg(2) == self.ACTOR
            return self.ret(r['requirements'])
        if address == 0x4012C0:
            return self.ret(arg(0))
        if address == 0x40C230 and caller in (0x584842, 0x5846F7):
            assert arg(1) == 0x11 and arg(2) == self.ACTOR
            name = 'notify_research_blocked' if caller == 0x584842 else 'notify_research_unaffordable'
            self.event(name, kingdom=arg(3) != 0)
            return self.ret()
        if address == 0x42D490:
            assert arg(1) == 0xB and arg(3) == self.ACTOR
            self.event('research_complete_event', kingdom=arg(2) != 0, property=arg(4) - 0x207F000)
            assert 0 < arg(4) - 0x207F000 < 0x40
            return self.ret()
        if address == 0x4CEF10:
            target = 'actor' if this == self.ACTOR else ('organization' if this == self.ORG else hex(this))
            self.event('apply_property', target=target, definition=(arg(0) - self.PROPDEF) // 0x40 if arg(0) else 0,
                       source=arg(1), actor=arg(2) == self.ACTOR)
            return self.ret(pop=12)
        if address == 0x4E3AD0:
            assert this == self.KINGDOM
            self.event('kingdom_add_property', definition=(arg(0) - self.PROPDEF) // 0x40 if arg(0) else 0,
                       source=arg(1), third=arg(2), notify=arg(3))
            return self.ret(pop=16)
        if address == 0x42A590:
            definition = (this - 0x54 - self.PROPDEF) // 0x40 if this != 0x54 else 0
            assert arg(0) == self.ACTOR and arg(1) == 0
            self.event('when_researched', definition=definition)
            return self.ret(pop=8)
        if address in (0x584120, 0x4E2360):
            return  # executed natively in this phase (phase6 treated them as seams)
        if image:
            # Delegate remaining seams to the accepted phase6 handler without
            # re-recording identity (already checked above).
            return production.ProductionMachine.observe(self, uc, address, size, user)
        return production.ProductionMachine.observe(self, uc, address, size, user)

    def research_action(self, action):
        kind = action['kind']
        if kind in ('start', 'process', 'cancel', 'request'):
            out = self.action(action)
        else:
            self.events = []
            if 'world' in action:
                self.update(action['world'])
            item = self.definition(action['item'])
            address, args = {'research': (0x584790, [item]), 'blocked': (0x5844A0, [item, int(action.get('notify', False))]),
                             'afford': (0x5845F0, [item, int(action.get('notify', False))]),
                             'complete': (0x584120, [item, 0])}[kind]
            self.uc.context_restore(self.context)
            self.put(self.STACK, self.STOP)
            for i, v in enumerate(args):
                self.put(self.STACK + 4 + i * 4, v)
            self.uc.reg_write(UC_X86_REG_ESP, self.STACK)
            self.uc.reg_write(UC_X86_REG_ECX, self.RESEARCH)
            from unicorn.x86_const import UC_X86_REG_EBX, UC_X86_REG_ESI, UC_X86_REG_EDI, UC_X86_REG_EBP, UC_X86_REG_EIP
            preserved = {reg: 0x77330000 + i for i, reg in enumerate([UC_X86_REG_EBX, UC_X86_REG_ESI, UC_X86_REG_EDI, UC_X86_REG_EBP])}
            for reg, value in preserved.items():
                self.uc.reg_write(reg, value)
            self.uc.emu_start(address, self.STOP, count=800000)
            harness.check_null_accesses(self)
            assert self.uc.reg_read(UC_X86_REG_EIP) == self.STOP
            assert self.uc.reg_read(UC_X86_REG_ESP) == self.STACK + 4 + 4 * len(args)
            for reg, value in preserved.items():
                assert self.uc.reg_read(reg) == value
            eax = self.uc.reg_read(UC_X86_REG_EAX)
            out = {'queue': self.queue(), 'economy': self.state(), 'events': self.events,
                   'next_item_id': self.read(self.WORLD + 0x198)}
            out['result'] = eax if kind == 'blocked' else bool(eax & 255)
        out['research'] = self.research_state()
        return out


def research_case(name, actions, *, items=None, research=None, **world):
    c = production.case(name, [], actions, **world)
    items = items or [{'id': 2, 'definition': 7, 'kind': 2, 'cost': [bits(10), 0, 0]}]
    for item in items:
        c['world']['definitions'].setdefault(str(item['id']), {'organization': False, 'recruit_time_bits': 0,
                                                               'research_duration_bits': bits(5), 'recruit_rate_bits': 0})
    r = {'items': items, 'state_active': True, 'organization': True, 'organization_properties': True,
         'organization_has_property': False, 'kingdom_has_property': False, 'requirements': 0,
         'completed': [], 'kingdom_in_progress': [], 'statistics': True, 'techs_researched': bits(3)}
    r.update(research or {})
    c['research'] = r
    return c


def scenarios():
    out = []
    research = lambda item=2: {'kind': 'research', 'item': item}
    later = lambda t: {'kind': 'process', 'world': {'now_bits': bits(t)}}
    levels = [{'id': 2, 'definition': 7, 'kind': 0, 'cost': [bits(10), 0, 0]},
              {'id': 3, 'definition': 8, 'kind': 1, 'cost': [bits(12.5), 0, 0]},
              {'id': 4, 'definition': 9, 'kind': 2, 'cost': [bits(7.25), 0, 0]},
              {'id': 5, 'definition': 11, 'kind': 2, 'cost': [bits(1), 0, 0]},
              {'id': 6, 'definition': 7, 'kind': 2, 'cost': [bits(3), 0, 0]}]
    out.append(research_case('request_complete_each_level', [research(2), later(15), research(3), later(21),
                                                             research(4), later(27), research(5), later(33)], items=levels))
    out.append(research_case('fifo_and_reservations', [research(4), research(6), research(2), {'kind': 'cancel', 'id': 102},
                                                       later(15), later(21)], items=levels))
    out.append(research_case('queued_blocks_duplicate', [research(4), research(4), {'kind': 'blocked', 'item': 4}], items=levels))
    out.append(research_case('kingdom_in_progress_blocks_same_definition', [{'kind': 'blocked', 'item': 6},
                                                                            research(6)], items=levels,
                             research={'kingdom_in_progress': [{'item': 4, 'definition': 9}, {'item': 2, 'definition': 7}]}))
    out.append(research_case('kind2_list_compares_definition_not_kind',
                             [{'kind': 'blocked', 'item': 2}], items=levels,
                             research={'kingdom_in_progress': [{'item': 6, 'definition': 7}]}))
    out.append(research_case('state_inactive', [research(4), {'kind': 'blocked', 'item': 4}], items=levels,
                             research={'state_active': False}))
    for req in [1, 2, 3, -1]:
        out.append(research_case(f'requirements_{req}', [research(3), {'kind': 'blocked', 'item': 3, 'notify': True}],
                                 items=levels, research={'requirements': req}))
    out.append(research_case('already_completed_actor_level', [{'kind': 'blocked', 'item': 2}, research(2)], items=levels,
                             research={'completed': [3, 2]}))
    out.append(research_case('organization_has_property', [research(3)], items=levels,
                             research={'organization_has_property': True}))
    out.append(research_case('organization_without_properties', [research(3), later(16)], items=levels,
                             research={'organization_has_property': True, 'organization_properties': False}))
    out.append(research_case('no_organization', [research(3), later(16)], items=levels,
                             research={'organization': False, 'organization_has_property': True}))
    out.append(research_case('kingdom_has_property', [research(4), research(2)], items=levels,
                             research={'kingdom_has_property': True}))
    out.append(research_case('no_kingdom', [research(4), {'kind': 'blocked', 'item': 4}, {'kind': 'afford', 'item': 4, 'notify': True}],
                             items=levels, kingdom=False))
    out.append(research_case('unaffordable', [research(3), {'kind': 'afford', 'item': 3, 'notify': False},
                                              {'kind': 'afford', 'item': 3, 'notify': True}],
                             items=[{'id': 3, 'definition': 8, 'kind': 1, 'cost': [bits(50.5), 0, 0]}]))
    out.append(research_case('exact_stock_affordable', [research(3), later(16)],
                             items=[{'id': 3, 'definition': 8, 'kind': 1, 'cost': [bits(50), 0, 0]}]))
    out.append(research_case('cost_ignores_nonstockpiled_rows', [research(3)],
                             items=[{'id': 3, 'definition': 8, 'kind': 1, 'cost': [bits(1), bits(999), bits(999)]}]))
    out.append(research_case('no_statistics', [research(4), later(16)], items=levels, research={'statistics': False}))
    out.append(research_case('complete_direct', [{'kind': 'complete', 'item': 4}, {'kind': 'complete', 'item': 2},
                                                 {'kind': 'complete', 'item': 3}, {'kind': 'complete', 'item': 5}],
                             items=levels, research={'kingdom_in_progress': [{'item': 6, 'definition': 7}, {'item': 4, 'definition': 9},
                                                                            {'item': 4, 'definition': 9}]}))
    out.append(research_case('complete_without_kingdom_actor_level', [{'kind': 'complete', 'item': 2},
                                                                      {'kind': 'complete', 'item': 3}], items=levels, kingdom=False))
    out.append(research_case('instant_research', [research(4), research(2)], items=levels, instant_production=True))
    out.append(research_case('cancel_refunds_research', [research(4), {'kind': 'cancel', 'id': 101}, research(4)], items=levels))
    out.append(research_case('paused_then_complete', [research(4), {'kind': 'process', 'world': {'now_bits': bits(12), 'in_supply': False}},
                                                      {'kind': 'process', 'world': {'now_bits': bits(13), 'in_supply': True}}, later(30)],
                             items=levels))
    rng = random.Random(0x584790)
    for i in range(24):
        items = []
        for j, ident in enumerate(range(2, 2 + rng.randint(2, 5))):
            items.append({'id': ident, 'definition': rng.choice([7, 8, 9, 10]), 'kind': rng.choice([0, 1, 2, 2, 3]),
                          'cost': [bits(rng.choice([0.0, 2.5, 7.0, 13.375, rng.uniform(0, 30)])), bits(rng.uniform(0, 5)), 0]})
        ids = [x['id'] for x in items]
        actions = []
        now = 10.0
        for _ in range(rng.randint(3, 7)):
            op = rng.choice(['research', 'research', 'process', 'cancel', 'blocked', 'complete', 'afford'])
            if op == 'research':
                actions.append(research(rng.choice(ids)))
            elif op == 'process':
                now += rng.uniform(0.5, 9)
                actions.append(later(now))
            elif op == 'cancel':
                actions.append({'kind': 'cancel', 'id': rng.choice([101, 102, 103])})
            elif op == 'complete':
                actions.append({'kind': 'complete', 'item': rng.choice(ids)})
            else:
                actions.append({'kind': op, 'item': rng.choice(ids), 'notify': rng.random() < 0.5})
        r = {'state_active': rng.random() < 0.9, 'organization': rng.random() < 0.8,
             'organization_properties': rng.random() < 0.8, 'organization_has_property': rng.random() < 0.3,
             'kingdom_has_property': rng.random() < 0.3, 'requirements': rng.choice([0, 0, 0, 1, 2]),
             'completed': rng.sample(ids, rng.randint(0, 1)), 'statistics': rng.random() < 0.8,
             'techs_researched': bits(rng.choice([0.0, 1.0, 16777216.0, 5.5]))}
        if rng.random() < 0.4:
            pick = rng.choice(items)
            r['kingdom_in_progress'] = [{'item': pick['id'], 'definition': pick['definition']}]
        c = research_case(f'random_{i}', actions, items=items, research=r)
        c['economy']['stockpiled'] = [bits(rng.choice([5.0, 50.0, 200.0])), 0, 0]
        out.append(c)
    return out


def main():
    rows = []
    executed = {}
    cases = scenarios()
    for pc, cw in economy.MODES:
        for scenario in cases:
            machine = ResearchMachine(scenario, cw)
            steps = []
            for action in scenario['actions']:
                if action['kind'] == 'complete':
                    item = next(i for i in scenario['research']['items'] if i['id'] == action['item'])
                    if item['kind'] == 2 and item['definition'] and not scenario['world']['kingdom']:
                        continue  # native null-kingdom dereference: outside domain
                steps.append({'input': action, 'output': machine.research_action(action)})
            harness.check_null_accesses(machine)
            rows.append({'precision': pc, 'input': scenario, 'steps': steps})
            executed.update(machine.executed)
    pins = {}
    for name, (start, end) in PINNED.items():
        a = economy.ORIGINAL.at(start, end - start)
        assert a == economy.ACCEPTED.at(start, end - start), name
        pins[name] = {'start': hex(start), 'end': hex(end), 'sha256': economy.sha(a)}
    doc = {'schema': 1, 'original_sha256': economy.ORIGINAL_SHA, 'v14_sha256': economy.ACCEPTED_SHA,
           'producer_sha256': economy.sha(Path(__file__).read_bytes()), 'method': __doc__, 'pins': pins,
           'cases': rows, 'visited_windows': len(executed)}
    out = HERE / 'native-research-fixtures.json'
    out.write_text(json.dumps(doc, indent=1) + '\n', encoding='utf-8')
    steps = sum(len(r['steps']) for r in rows)
    print(json.dumps({'cases': len(rows), 'steps': steps, 'visited_instruction_windows': len(executed)}))


if __name__ == '__main__':
    main()
