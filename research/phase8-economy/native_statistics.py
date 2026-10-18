"""Original kingdom statistics objects in isolated x86.

Executed unchanged: 58EB60 ordinary statistics constructor, 58D560 relation
statistics constructor (+58D4D0/58D520 map constructors, CRT 96D18D iterator),
58E870 score add, 58D950 relation add (slot index-52) and 58D8C0 by-definition
add (slot index-57) with 7BA3C0 insertion and 606750 rehash.
Seams: allocator only. Output: native-statistics-fixtures.json.
"""
from pathlib import Path
import json, random, shutil, sys
sys.path.insert(0, str(Path(__file__).resolve().parent))
from harness import Machine, MODES, bits, pin, ret_pop, ROOT  # noqa: E402

RANGES = {'ordinary_ctor_58EB60': (0x58EB60, 0x58EB9A), 'relation_ctor_58D560': (0x58D560, 0x58D61A),
          'map_ctor_58D4D0': (0x58D4D0, 0x58D512), 'map_ctor_58D520': (0x58D520, 0x58D55F),
          'score_add_58E870': (0x58E870, 0x58E883), 'relation_add_58D950': (0x58D950, 0x58D9C7),
          'definition_add_58D8C0': (0x58D8C0, 0x58D940),
          'map_insert_7BA3C0': (0x7BA3C0, 0x7BA42B), 'map_rehash_606750': (0x606750, 0x60683C)}
STATS, RELATIONS = 0x2000000, 0x2001000


class StatisticsMachine(Machine):
    def __init__(self, cw):
        super().__init__(cw)
        self.seam(0x6F7810, lambda m, esp: (self.alloc(self.read(esp + 4)), 0))
        self.seam(0x6F7430, ret_pop(0))
        self.uc.mem_write(STATS, b'\xCD' * 0x108)  # constructor leaves 47..55 unwritten
        self.call(0x58EB60, [1], STATS)
        self.call(0x58D560, [], RELATIONS)
        for i in range(47, 56):
            self.put(STATS + 0x28 + 4 * i, 0)  # Rust starts these at zero explicitly

    def snapshot(self):
        relations = []
        for i in range(10):
            m = RELATIONS + 0x10 * i
            buckets, count = self.read(m), self.read(m + 4)
            values = {}
            for b in range(count):
                node = self.read(buckets + 4 * b)
                while node:
                    key = self.read(node)
                    assert str(key) not in values
                    values[str(key)] = self.read(node + 4)
                    node = self.read(node + 8)
            assert len(values) == self.read(m + 8)
            index = 52 + i
            leader = self.read(RELATIONS + 4 * index - 0x30)
            relations.append({'values': values, 'maximum': self.read(RELATIONS + 4 * index - 8),
                              'leader': leader or None, 'buckets': count})
        return {'production': self.words(STATS, 10), 'scores': self.words(STATS + 0x28, 56), 'relations': relations}


R = random.Random(0x58D950)


def amount():
    return bits(R.choice([1.0, 1.0, 0.5, 3.0, -1.0, 0.0, 16777216.0, R.uniform(-5, 50), 1e-3, 0.1]))


def main():
    fixtures, executions = [], 0
    cases = []
    for i in range(40):
        ops = []
        keys = [R.randint(1, 0xFFFF) for _ in range(R.choice([3, 8, 40]))] + [0]
        for _ in range(R.randint(10, 80)):
            if R.random() < 0.4:
                ops.append({'kind': 'score', 'index': R.randrange(56), 'amount': amount()})
            elif R.random() < 0.5:
                ops.append({'kind': 'relation', 'index': R.randrange(52, 62), 'other': R.choice(keys), 'amount': amount()})
            else:
                ops.append({'kind': 'definition_relation', 'index': R.randrange(57, 62), 'other': R.choice(keys),
                            'amount': amount()})
        cases.append({'name': f'random-{i}', 'ops': ops})
    # Rounded-tie leader replacement: 16777216 + 1 rounds back to 16777216 in binary32.
    cases.append({'name': 'tie', 'ops': [{'kind': 'relation', 'index': 60, 'other': 7, 'amount': bits(16777216.0)},
                                         {'kind': 'relation', 'index': 60, 'other': 9, 'amount': bits(16777216.0)},
                                         {'kind': 'relation', 'index': 60, 'other': 9, 'amount': bits(1.0)}]})
    cases.append({'name': 'rehash', 'ops': [{'kind': 'relation', 'index': 57, 'other': 1000 + 13 * k, 'amount': bits(1.0 + k)}
                                            for k in range(40)] + [{'kind': 'relation', 'index': 57, 'other': 1000, 'amount': bits(100.0)}]})
    # 0x58D8C0 index 59 and 0x58D950 index 54 share slot 2 (map, leader, maximum).
    cases.append({'name': 'definition-slot-alias', 'ops': [
        {'kind': 'definition_relation', 'index': 59, 'other': 0x2040000, 'amount': bits(1.0)},
        {'kind': 'relation', 'index': 54, 'other': 0x2000000, 'amount': bits(5.0)},
        {'kind': 'definition_relation', 'index': 59, 'other': 0x2040000, 'amount': bits(1.0)},
        {'kind': 'definition_relation', 'index': 59, 'other': 0x2040400, 'amount': bits(6.0)},
        {'kind': 'definition_relation', 'index': 61, 'other': 0, 'amount': bits(6.0)},
        {'kind': 'relation', 'index': 60, 'other': 0x2000000, 'amount': bits(2.0)}]})
    for case in cases:
        for mode, cw in MODES:
            m = StatisticsMachine(cw)
            steps = []
            for op in case['ops']:
                if op['kind'] == 'score':
                    m.call(0x58E870, [op['index'], op['amount']], STATS)
                elif op['kind'] == 'relation':
                    m.call(0x58D950, [op['index'], op['other'], op['amount']], RELATIONS)
                else:
                    m.call(0x58D8C0, [op['index'], op['other'], op['amount']], RELATIONS)
                executions += 1
            steps.append(m.snapshot())
            fixtures.append({'name': case['name'], 'precision': mode, 'ops': case['ops'], 'expected': steps[-1]})
    doc = {'schema': 1, 'pins': pin(RANGES), 'executions': executions, 'fixtures': fixtures}
    out = Path(__file__).resolve().parent / 'native-statistics-fixtures.json'
    out.write_text(json.dumps(doc, separators=(',', ':')) + '\n', encoding='utf-8')
    shutil.copyfile(out, ROOT / 'crates/aa-economy/tests/fixtures/native-statistics-fixtures.json')
    print(f'{len(fixtures)} fixtures, {executions} native executions')


if __name__ == '__main__':
    main()
