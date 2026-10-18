"""Original SettlementComponent structure slots in isolated x86.

Executed unchanged: 585500 find structure, 5890C0 structure match, 4D1050,
529C30, 586B40 AddStructure with 93B220/766DE0 vector append. Seams
(recorded): 585F60 attach, 404F60 log, allocator.
"""
from pathlib import Path
import json, random, sys
sys.path.insert(0, str(Path(__file__).resolve().parent))
from harness import Machine, MODES, pin, ret_pop  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX  # noqa: E402

RANGES = {'find_structure_585500': (0x585500, 0x58557B), 'structure_match_5890C0': (0x5890C0, 0x5891F2),
          'transient_4D1050': (0x4D1050, 0x4D1079), 'add_structure_586B40': (0x586B40, 0x586BBE),
          'push_93B220': (0x93B220, 0x93B24D), 'append_766DE0': (0x766DE0, 0x766E37)}
SC, SA, SAD, LIMITS = 0x2000000, 0x2000100, 0x2000200, 0x2000A00
ACTORS, COMPS, DEFS = 0x2010000, 0x2020000, 0x2030000


def A(i): return ACTORS + 0x200 * i
def C(i): return COMPS + 0x40 * i
def D(d): return DEFS + 0x800 * d


class SlotMachine(Machine):
    def __init__(self, cw, case):
        super().__init__(cw)
        self.case = case
        self.seam(0x6F7810, lambda m, esp: (self.alloc(self.read(esp + 4)), 0))
        self.seam(0x6F7430, ret_pop(0))
        self.seam(0x585F60, self._attach)
        self.seam(0x404F60, self._log)
        self.put(SC + 4, SA); self.put(SA + 4, SAD); self.put(SAD + 0x464, LIMITS)
        for level, m in enumerate(case['limits']):
            self.put(LIMITS + 12 * level, m)
        self.put(SC + 0x10, case['level'])
        for d, lists in case['definitions'].items():
            for off, key in ((0x4C0, 'upgrades'), (0x4D0, 'list_4d0')):
                nodes = [self.alloc(8) for _ in lists[key]]
                for i, (node, x) in enumerate(zip(nodes, lists[key])):
                    self.put(node, D(x)); self.put(node + 4, nodes[i + 1] if i + 1 < len(nodes) else 0)
                self.put(D(int(d)) + off, nodes[0] if nodes else 0)
        for i, a in enumerate(case['actors']):
            self.put(A(i) + 4, D(a['definition']))
            self.put(A(i) + 0x94, C(i) if a['structure_component'] else 0)
            self.put(C(i) + 4, A(i))
            flags = (a['state_25'] << 25) | (a['state_26'] << 26) | (a['constructing'] << 21)
            self.put(A(i) + 0x100, flags); self.put(A(i) + 0x104, a['state_36'] << 4)
            self.put(D(a['definition']) + 8, 0x2F00000 + a['definition'])
        self.put(SC + 0x14, A(case['center']) if case['center'] is not None else 0)
        data = self.alloc(4 * 8)
        self.put_words(data, [A(i) for i in case['structures']])
        self.put(SC + 0x18, data); self.put(SC + 0x1C, len(case['structures'])); self.put(SC + 0x20, 8)
        self.trace = []

    def actor(self, p):
        return (p - ACTORS) // 0x200

    def _attach(self, m, esp):
        assert self.uc.reg_read(UC_X86_REG_ECX) == SC
        self.trace.append({'kind': 'attach_structure', 'actor': self.actor(self.read(esp + 4))})
        return None, 4

    def _log(self, m, esp):
        self.trace.append({'kind': 'log_add_failed'})
        return None, 0

    def structures(self):
        return [self.actor(p) for p in self.words(self.read(SC + 0x18), self.read(SC + 0x1C))]

    def run(self, op):
        self.trace = []
        if op['kind'] == 'find':
            eax, _ = self.call(0x585500, [D(op['definition']), int(op['require_complete']),
                                          int(op['include_upgrades'])], SC)
            result = self.actor(eax) if eax else None
        else:
            actor = A(op['actor']) if op['actor'] is not None else 0
            eax, _ = self.call(0x586B40, [actor], SC)
            result = bool(eax & 0xFF)
        return {'result': result, 'structures': self.structures(), 'events': self.trace}


R = random.Random(0x585500)


def make_case(i):
    defs = list(range(1, 7))
    actors = [{'definition': R.choice(defs), 'structure_component': R.random() < 0.85, 'state_25': R.random() < 0.1,
               'state_26': R.random() < 0.1, 'state_36': R.random() < 0.1, 'constructing': R.random() < 0.3}
              for _ in range(8)]
    listed = [i for i in range(8) if actors[i]['structure_component']]
    structures = R.sample(listed, R.randint(0, min(4, len(listed))))
    ops = []
    for _ in range(R.randint(3, 8)):
        if R.random() < 0.6:
            ops.append({'kind': 'find', 'definition': R.choice(defs), 'require_complete': R.random() < 0.5,
                        'include_upgrades': R.random() < 0.5})
        else:
            ops.append({'kind': 'add', 'actor': R.choice(list(range(8)) + [None])})
    return {'name': f'random-{i}', 'actors': actors, 'structures': structures,
            'center': R.choice([None] + list(range(8))), 'level': R.randrange(3),
            'limits': [R.randint(0, 6) for _ in range(3)],
            'definitions': {str(d): {'upgrades': R.sample(defs, R.randint(0, 2)),
                                     'list_4d0': R.sample(defs, R.randint(0, 2))} for d in defs},
            'ops': ops}


def main():
    cases = [make_case(i) for i in range(120)]
    fixtures, executions = [], 0
    for case in cases:
        for mode, cw in MODES:
            m = SlotMachine(cw, case)
            steps = [{'op': op, 'expected': m.run(op)} for op in case['ops']]
            executions += len(steps)
            fixtures.append({'name': case['name'], 'precision': mode, 'case': case, 'steps': steps})
    doc = {'schema': 1, 'pins': pin(RANGES), 'executions': executions, 'fixtures': fixtures}
    out = Path(__file__).resolve().parent / 'native-settlement-slot-fixtures.json'
    out.write_text(json.dumps(doc, separators=(',', ':')) + '\n', encoding='utf-8')
    print(f'{len(fixtures)} fixtures, {executions} native executions')


if __name__ == '__main__':
    main()
