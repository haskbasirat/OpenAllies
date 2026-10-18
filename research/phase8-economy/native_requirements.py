"""Original research requirement evaluation in isolated x86.

Executed unchanged: 5842F0 research requirements, 4C5630 wrapper, 4C5250
evaluation, 45F240 list append, CString release paths. Seams (recorded):
4E1890 technology allowed list, 581280 property container lookup, property
definition virtual +0x18, 4E0FB0 kingdom property, 585500 settlement actor,
4E1480 kingdom actor, 4C5560 text builder (list contents recorded), 405D60
message, CString constructors/allocator, 401170 string clear, 6F7810/6F7430.
"""
from pathlib import Path
import json, random, sys
sys.path.insert(0, str(Path(__file__).resolve().parent))
from harness import Machine, MODES, pin, ret_pop  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX  # noqa: E402

RANGES = {'requirements_5842F0': (0x5842F0, 0x584491), 'wrapper_4C5630': (0x4C5630, 0x4C571F),
          'evaluate_4C5250': (0x4C5250, 0x4C54A3), 'list_append_45F240': (0x45F240, 0x45F29F)}
K, ACT, SETL, SETL_COMP = 0x2000000, 0x2001000, 0x2002000, 0x2002800
ACT_PROPS, SETL_PROPS = 0x2003000, 0x2003100
ITEM, DEFS, PDVT = 0x2004000, 0x2010000, 0x2005000
ALLOCATOR, AVT = 0x2006000, 0x2006100
STUB_TECH, STUB_NEWSTR = 0x3000100, 0x3000110


def D(d): return DEFS + d * 0x40


class RequirementMachine(Machine):
    def __init__(self, cw, case):
        super().__init__(cw)
        self.case = case
        w = case['world']
        self.put(PDVT + 0x18, STUB_TECH)
        for d in range(1, 40):
            self.put(D(d), PDVT); self.put(D(d) + 0x10, 0x2F00000 + d)
        self.put(ACT + 0x78, ACT_PROPS if w['actor_properties'] else 0)
        self.put(SETL + 0x78, SETL_PROPS if w['settlement_properties'] else 0)
        self.put(SETL + 0x98, SETL_COMP if w['settlement_component'] else 0)
        self.put(ALLOCATOR, AVT); self.put(AVT + 0xC, STUB_NEWSTR); self.put(AVT + 4, 0x3000120)
        self.trace = []
        self.seam(0x6F7810, lambda m, esp: (self.alloc(self.read(esp + 4)), 0))
        self.seam(0x6F7430, ret_pop(0))
        self.seam(0x4E1890, self._allowed)
        self.seam(0x581280, self._contains)
        self.seam(STUB_TECH, self._tech)
        self.seam(0x4E0FB0, self._kingdom_knows)
        self.seam(0x585500, self._settlement_has)
        self.seam(0x4E1480, self._kingdom_has)
        self.seam(0x856370, lambda m, esp: (ALLOCATOR, 0))
        self.seam(STUB_NEWSTR, lambda m, esp: (self._header(), 0))
        self.seam(0x3000120, ret_pop(1))
        self.seam(0x401170, lambda m, esp: (None, 0))
        self.seam(0x4C5560, self._describe)
        self.seam(0x405D60, self._message)
        self.seam(0x401B00, self._string_ctor)
        self.seam(0x4B9F40, self._format)

    def _header(self):
        h = self.alloc(0x40)
        self.put(h, ALLOCATOR); self.put(h + 8, 16); self.put(h + 0xC, 100)
        return h

    def _string_ctor(self, m, esp):
        this = self.uc.reg_read(UC_X86_REG_ECX)
        self.put(this, self._header() + 0x10)
        return this, 4

    def _format(self, m, esp):
        out = self.read(esp + 4)
        self.put(out, self._header() + 0x10)
        return out, 8

    def d(self, p):
        assert (p - DEFS) % 0x40 == 0
        return (p - DEFS) // 0x40

    def _allowed(self, m, esp):
        assert self.uc.reg_read(UC_X86_REG_ECX) == K
        d = self.d(self.read(esp + 4))
        self.trace.append({'kind': 'technology_allowed', 'definition': d})
        return int(d in self.case['world']['allowed']), 4

    def _contains(self, m, esp):
        this, d, z = self.uc.reg_read(UC_X86_REG_ECX), self.d(self.read(esp + 4)), self.read(esp + 8)
        assert z == 0
        owner = {ACT_PROPS: 'actor', SETL_PROPS: 'settlement'}[this]
        self.trace.append({'kind': 'properties_contain', 'owner': owner, 'definition': d})
        return int(d in self.case['world'][f'{owner}_known']), 8

    def _tech(self, m, esp):
        d = self.d(self.uc.reg_read(UC_X86_REG_ECX))
        self.trace.append({'kind': 'is_technology', 'definition': d})
        return int(d in self.case['world']['technologies']), 0

    def _kingdom_knows(self, m, esp):
        assert self.uc.reg_read(UC_X86_REG_ECX) == K and self.read(esp + 8) == 0
        d = self.d(self.read(esp + 4))
        self.trace.append({'kind': 'kingdom_knows', 'definition': d})
        return int(d in self.case['world']['kingdom_known']), 8

    def _settlement_has(self, m, esp):
        assert self.uc.reg_read(UC_X86_REG_ECX) == SETL_COMP and self.read(esp + 8) == 1 and self.read(esp + 12) == 0
        d = self.d(self.read(esp + 4))
        self.trace.append({'kind': 'settlement_has_actor', 'definition': d})
        return int(d in self.case['world']['settlement_actors']), 12

    def _kingdom_has(self, m, esp):
        assert self.uc.reg_read(UC_X86_REG_ECX) == K and self.read(esp + 8) == 1 and self.read(esp + 12) == 0
        d = self.d(self.read(esp + 4))
        self.trace.append({'kind': 'kingdom_has_actor', 'definition': d})
        return int(d in self.case['world']['kingdom_actors']), 12

    def _describe(self, m, esp):
        lst = self.read(esp + 4)
        values, node = [], self.read(lst)
        while node:
            values.append(self.d(self.read(node)))
            node = self.read(node + 4)
        self.trace.append({'kind': 'describe_missing', 'list': values})
        return None, 0

    def _message(self, m, esp):
        name = self.read(esp + 12)
        self.trace.append({'kind': 'missing_requirements_message', 'definition': name - 0x2F00000})
        return None, 0

    def write_list(self, head, values):
        nodes = [self.alloc(8) for _ in values]
        for i, (node, v) in enumerate(zip(nodes, values)):
            self.put(node, D(v)); self.put(node + 4, nodes[i + 1] if i + 1 < len(nodes) else 0)
        self.put(head, nodes[0] if nodes else 0); self.put(head + 4, nodes[-1] if nodes else 0)

    def run(self, op):
        self.trace = []
        item = self.case['item']
        r = item['requirements']
        self.put(ITEM, D(item['definition']))
        self.write_list(ITEM + 4, r['properties']); self.put8(ITEM + 4 + 8, int(r['any_property']))
        self.write_list(ITEM + 4 + 0xC, r['actors']); self.put8(ITEM + 4 + 0x14, int(r['any_actor']))
        message = self.alloc(4) if op['message'] else 0
        args = [K if op['kingdom'] else 0, SETL if op['settlement'] else 0, ACT if op['actor'] else 0, ITEM, message]
        eax, _ = self.call(0x5842F0, args, 0x7EADBEEF)
        return {'events': self.trace, 'result': eax}


R = random.Random(0x5842F0)


def make_case(i):
    pool = list(range(1, 30))
    props = R.sample(pool, R.randint(0, 4))
    actors = R.sample(pool, R.randint(0, 4))
    if R.random() < 0.2 and props:
        props.append(props[0])  # duplicate requirement: missing list dedup
    if R.random() < 0.2 and actors:
        actors.append(actors[0])
    def pick(src):
        return sorted(R.sample(src, R.randint(0, len(src)))) if src else []
    world = {'allowed': pool if R.random() < 0.8 else pick(pool), 'actor_properties': R.random() < 0.8,
             'settlement_properties': R.random() < 0.7, 'settlement_component': R.random() < 0.7,
             'actor_known': pick(props), 'settlement_known': pick(props), 'technologies': pick(props),
             'kingdom_known': pick(props), 'settlement_actors': pick(actors), 'kingdom_actors': pick(actors)}
    item = {'id': 2, 'definition': R.choice(pool), 'kind': R.choice([0, 1, 2]), 'cost': [0],
            'requirements': {'properties': props, 'any_property': R.random() < 0.3, 'actors': actors,
                             'any_actor': R.random() < 0.3}}
    ops = [{'kingdom': R.random() < 0.85, 'settlement': R.random() < 0.7, 'actor': R.random() < 0.85,
            'message': R.random() < 0.5} for _ in range(3)]
    return {'name': f'random-{i}', 'world': world, 'item': item, 'ops': ops}


def main():
    cases = [make_case(i) for i in range(150)]
    fixtures, executions = [], 0
    for case in cases:
        for mode, cw in MODES:
            m = RequirementMachine(cw, case)
            steps = []
            for op in case['ops']:
                steps.append({'op': op, 'expected': m.run(op)})
                executions += 1
            fixtures.append({'name': case['name'], 'precision': mode, 'case': case, 'steps': steps})
    doc = {'schema': 1, 'pins': pin(RANGES), 'executions': executions, 'fixtures': fixtures}
    out = Path(__file__).resolve().parent / 'native-requirements-fixtures.json'
    out.write_text(json.dumps(doc, separators=(',', ':')) + '\n', encoding='utf-8')
    print(f'{len(fixtures)} fixtures, {executions} native executions')


if __name__ == '__main__':
    main()
