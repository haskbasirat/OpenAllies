"""Original world start 0x4F1E10 in isolated x86 (economy parts).

Executed unchanged: 4F1E10, 4A5BC0/4F0680 kingdom list iteration, 4E3690
statistics promotion (58EB60/58D560 constructors, 96BE33/55A5B0 map
destruction), 58EA00, 4EFBE0 GTID-table iteration, 4CEEC0/4CEED0, 4E37D0
forced snapshot timer. Seams (recorded): object virtuals +0xB4/+0x108,
58F010 statistics tracking, 58F530 snapshot constructor, 45F240 snapshot list
push, allocator.
"""
from pathlib import Path
import json, random, shutil, sys
sys.path.insert(0, str(Path(__file__).resolve().parent))
from harness import MODES, ROOT, bits, pin  # noqa: E402
from native_statistics import StatisticsMachine  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX  # noqa: E402

RANGES = {'world_start_4F1E10': (0x4F1E10, 0x4F1F6C), 'kingdom_iterator_4A5BC0': (0x4A5BC0, 0x4A5BD9),
          'kingdom_iterator_4F0680': (0x4F0680, 0x4F06AC), 'promote_4E3690': (0x4E3690, 0x4E37C6),
          'mission_reset_58EA00': (0x58EA00, 0x58EA23), 'gtid_next_4EFBE0': (0x4EFBE0, 0x4EFC40),
          'field_4CEEC0': (0x4CEEC0, 0x4CEEC7), 'field_4CEED0': (0x4CEED0, 0x4CEEDD),
          'snapshot_timer_4E37D0': (0x4E37D0, 0x4E3893)}
WORLD, KLIST, OVT = 0x2200000, 0x2200400, 0x2200800
KINGDOMS, OBJECTS, GTIDS = 0x2210000, 0x2220000, 0x2100000
STUB_ACTIVE, STUB_OWNER = 0x3000100, 0x3000110


def K(i): return KINGDOMS + 0x800 * i
def O(j): return OBJECTS + 0x200 * j


class WorldStartMachine(StatisticsMachine):
    def __init__(self, cw, case):
        super().__init__(cw)
        self.case = case
        self.trace = []
        self.put(0xB364C0, GTIDS)
        self.put(OVT + 0xB4, STUB_ACTIVE); self.put(OVT + 0x108, STUB_OWNER)
        self.put8(WORLD + 0x274, int(case['started']))
        kingdoms = case['kingdoms']
        data = self.alloc(4 * max(1, len(kingdoms)))
        self.put(WORLD + 0x154, data); self.put(WORLD + 0x158, len(kingdoms))  # inline vector
        self.stats = {}
        for i, k in enumerate(kingdoms):
            if k is None:
                self.put(data + 4 * i, 0)
                continue
            self.put(data + 4 * i, K(i))
            for off, key, size, ctor in ((0x32C, 'active', 0x108, 0x58EB60), (0x330, 'loaded', 0x108, 0x58EB60),
                                         (0x340, 'relations', 0xF0, 0x58D560), (0x344, 'loaded_relations', 0xF0, 0x58D560)):
                if k[key] is None:
                    self.put(K(i) + off, 0)
                    continue
                obj = self.alloc(size)
                self.call(ctor, [1] if size == 0x108 else [], obj)
                if size == 0x108:
                    self.put_words(obj + 0x28, k[key])
                self.put(K(i) + off, obj)
            self.put(K(i) + 0x334, k['timer']); self.put(K(i) + 0x338, 0x2230000 if k['has_snapshots'] else 0)
            self.put(K(i) + 0x190, k['word_190'])
        objects = case['objects']
        self.put(GTIDS + 0x60004, len([o for o in objects if o is not None]))
        self.put(GTIDS + 0x6000C, len(objects))
        for j, o in enumerate(objects, start=1):
            if o is None:
                continue
            self.put(GTIDS + 4 * j + 0x20004, O(j)); self.put16(GTIDS + 2 * j + 4, 1)
            self.put(O(j), OVT); self.put(O(j) + 0x14, (1 << 16) | j); self.put(O(j) + 0x13C, o['field_13c'])
        self.seam(STUB_ACTIVE, self._active)
        self.seam(STUB_OWNER, self._owner)
        self.seam(0x58F010, self._track)
        self.seam(0x58F530, self._snapshot)
        self.seam(0x45F240, self._push)

    def obj(self, p):
        return (p - OBJECTS) // 0x200

    def kingdom_of_stats(self, stats):
        for i in range(len(self.case['kingdoms'])):
            if self.case['kingdoms'][i] is not None and self.read(K(i) + 0x32C) == stats:
                return i
        raise AssertionError(hex(stats))

    def _active(self, m, esp):
        j = self.obj(self.uc.reg_read(UC_X86_REG_ECX))
        self.trace.append({'kind': 'active', 'object': j})
        return int(self.case['objects'][j - 1]['active']), 0

    def _owner(self, m, esp):
        j = self.obj(self.uc.reg_read(UC_X86_REG_ECX))
        owner = self.case['objects'][j - 1]['owner']
        self.trace.append({'kind': 'owner', 'object': j})
        return (K(owner) if owner is not None else 0), 0

    def _track(self, m, esp):
        k = self.kingdom_of_stats(self.uc.reg_read(UC_X86_REG_ECX))
        self.trace.append({'kind': 'track', 'kingdom': k, 'object': self.obj(self.read(esp + 4))})
        return None, 4

    def _snapshot(self, m, esp):
        stats, kingdom = self.read(esp + 4), self.read(esp + 8)
        k = (kingdom - KINGDOMS) // 0x800
        assert self.read(K(k) + 0x32C) == stats
        self.trace.append({'kind': 'snapshot', 'kingdom': k})
        return self.uc.reg_read(UC_X86_REG_ECX), 8

    def _push(self, m, esp):
        this = self.uc.reg_read(UC_X86_REG_ECX)
        self.put(this, 0x2230000)
        return None, 4

    def run(self):
        self.call(0x4F1E10, [], WORLD)
        kingdoms = []
        for i, k in enumerate(self.case['kingdoms']):
            if k is None:
                kingdoms.append(None)
                continue
            active = self.read(K(i) + 0x32C)
            kingdoms.append({'active_scores': self.words(active + 0x28, 56), 'loaded': self.read(K(i) + 0x330),
                             'relations': self.read(K(i) + 0x340) != 0, 'loaded_relations': self.read(K(i) + 0x344),
                             'timer': self.read(K(i) + 0x334), 'has_snapshots': self.read(K(i) + 0x338) != 0})
        fields = [self.read(O(j) + 0x13C) if o is not None else None
                  for j, o in enumerate(self.case['objects'], start=1)]
        return {'started': bool(self.read8(WORLD + 0x274)), 'kingdoms': kingdoms, 'fields_13c': fields,
                'events': self.trace}


R = random.Random(0x4F1E10)


def make_case(i):
    def scores():
        return [bits(R.choice([0.0, 1.0, 7.5, R.uniform(0, 100)])) for _ in range(56)]
    kingdoms = []
    for _ in range(R.randint(1, 4)):
        if R.random() < 0.2:
            kingdoms.append(None)
            continue
        kingdoms.append({'active': scores() if R.random() < 0.6 else None,
                         'loaded': scores() if R.random() < 0.6 else None,
                         'relations': True if R.random() < 0.6 else None,
                         'loaded_relations': True if R.random() < 0.6 else None,
                         'timer': bits(R.choice([0.0, 12.5, 59.0])), 'has_snapshots': R.random() < 0.4,
                         'word_190': R.choice([0, 0, 1])})
    real = [i for i, k in enumerate(kingdoms) if k is not None]
    objects = []
    for _ in range(R.randint(0, 6)):
        if R.random() < 0.15:
            objects.append(None)
            continue
        objects.append({'active': R.random() < 0.7, 'field_13c': R.choice([0x38, 0, 7]),
                        'owner': R.choice(real + [None]) if real else None})
    return {'name': f'random-{i}', 'started': R.random() < 0.1, 'kingdoms': kingdoms, 'objects': objects}


def main():
    cases = [make_case(i) for i in range(120)]
    fixtures, executions = [], 0
    for case in cases:
        for mode, cw in MODES:
            m = WorldStartMachine(cw, case)
            fixtures.append({'name': case['name'], 'precision': mode, 'case': case, 'expected': m.run()})
            executions += 1
    doc = {'schema': 1, 'pins': pin(RANGES), 'executions': executions, 'fixtures': fixtures}
    out = Path(__file__).resolve().parent / 'native-world-start-fixtures.json'
    out.write_text(json.dumps(doc, separators=(',', ':')) + '\n', encoding='utf-8')
    shutil.copyfile(out, ROOT / 'crates/aa-economy/tests/fixtures/native-world-start-fixtures.json')
    print(f'{len(fixtures)} fixtures, {executions} native executions')


if __name__ == '__main__':
    main()
