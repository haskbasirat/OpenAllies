"""Original kingdom/team restriction checks in isolated x86.

Executed unchanged: 4E1910, 4E1810, 4E1890, 4F0400, 504560, 504500, 504530,
4154D0 string map lookup, 5043F0 bucket index, 405030 key hash. Seams: 401B00
CString copy (aliases the key buffer), 96CAD6 _wcsicmp (C-locale path,
reimplemented: A-Z folded), allocator. Maps are built with node placement from
the native hash (405030 executed for every key).
"""
from pathlib import Path
import json, random, shutil, sys
sys.path.insert(0, str(Path(__file__).resolve().parent))
from harness import Machine, MODES, ROOT, pin  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX  # noqa: E402

RANGES = {'restriction_4E1910': (0x4E1910, 0x4E1960), 'restriction_4E1810': (0x4E1810, 0x4E1883),
          'technology_4E1890': (0x4E1890, 0x4E1903), 'manager_4F0400': (0x4F0400, 0x4F040B),
          'profile_504560': (0x504560, 0x504584), 'actors_504500': (0x504500, 0x50452A),
          'technologies_504530': (0x504530, 0x50455A), 'lookup_4154D0': (0x4154D0, 0x41550E),
          'bucket_5043F0': (0x5043F0, 0x504449), 'hash_405030': (0x405030, 0x40506F)}
K, WORLD, MGR, TEAM, DEF = 0x2000000, 0x2001000, 0x2002000, 0x2003000, 0x2004000
BUCKETS = 7


class RestrictionMachine(Machine):
    def __init__(self, cw):
        super().__init__(cw)
        self.seam(0x401B00, self._copy)
        self.seam(0x96CAD6, self._wcsicmp)
        self.seam(0x6F7810, lambda m, esp: (self.alloc(self.read(esp + 4)), 0))
        self.seam(0x6F7430, lambda m, esp: (None, 0))
        self.put(0xB36534, WORLD)
        self.put(WORLD + 0x12C, MGR)

    def cstring(self, text):
        units = text.encode('utf-16le') + b'\0\0'
        h = self.alloc(0x10 + len(units))
        self.put(h + 4, len(text)); self.put(h + 8, len(text)); self.put(h + 0xC, 1000)
        self.uc.mem_write(h + 0x10, units)
        return h + 0x10

    def _copy(self, m, esp):
        this = self.uc.reg_read(UC_X86_REG_ECX)
        self.put(this, self.read(esp + 4))
        return this, 4

    def text(self, p):
        out = []
        while True:
            u = int.from_bytes(self.uc.mem_read(p + 2 * len(out), 2), 'little')
            if u == 0:
                return out
            out.append(u)

    def _wcsicmp(self, m, esp):
        a, b = self.text(self.read(esp + 4)) + [0], self.text(self.read(esp + 8)) + [0]
        fold = lambda u: u + 0x20 if 0x41 <= u <= 0x5A else u  # noqa: E731
        for x, y in zip(a, b):
            if fold(x) != fold(y):
                return (fold(x) - fold(y)) & 0xFFFFFFFF, 0
            if x == 0:
                break
        return 0, 0

    def hash_bucket(self, key):
        field = self.alloc(4)
        self.put(field, key)
        h, _ = self.call(0x405030, [field], 0)
        return h % BUCKETS

    def string_map(self, at, entries):
        """entries: list of (key text, value pointer); head insertion per bucket."""
        buckets = self.alloc(4 * BUCKETS)
        self.put(at, buckets); self.put(at + 4, BUCKETS); self.put(at + 8, len(entries))
        for text, value in entries:
            key = self.cstring(text)
            i = self.hash_bucket(key)
            node = self.alloc(12)
            self.put(node, key); self.put(node + 4, value); self.put(node + 8, self.read(buckets + 4 * i))
            self.put(buckets + 4 * i, node)

    def setup(self, case):
        profiles = []
        for name, p in case['profiles'].items():
            obj = self.alloc(0x60)
            self.string_map(obj + 0x34, [(k, 0x5000 if v else 0) for k, v in p['actors']])
            self.string_map(obj + 0x44, [(k, 0x5000 if v else 0) for k, v in p['technologies']])
            profiles.append((name, obj))
        self.string_map(MGR + 4, profiles)
        self.put(K + 0x1EC, TEAM if case['team'] is not None else 0)
        self.put(TEAM + 0x64, self.cstring(case['team'] or ''))
        self.put(K + 0x348, self.cstring(case['kingdom']))

    def run(self, op):
        self.put(DEF + 8, self.cstring(op['id']))
        if op['kind'] == 'actor':
            eax, _ = self.call(0x4E1910, [0x2009000, DEF], K)
            via_1810, _ = self.call(0x4E1810, [DEF], K)
            assert (eax & 0xFF) == (via_1810 & 0xFF)
        else:
            eax, _ = self.call(0x4E1890, [DEF], K)
        return bool(eax & 0xFF)


R = random.Random(0x4E1910)
IDS = ['Tank', 'tank', 'TANK', 'Infantry', 'Artillery', 'Radar', 'Fighter', 'fighter_ace', 'Bomber', '']


def entries():
    return [(R.choice(IDS[:-1]), R.random() < 0.85) for _ in range(R.randint(0, 4))]


def make_case(i):
    names = ['Allies', 'axis', 'Neutral', 'Germany']
    profiles = {n: {'actors': entries(), 'technologies': entries()} for n in R.sample(names, R.randint(0, 3))}
    return {'name': f'random-{i}', 'profiles': profiles,
            'team': R.choice([None, 'Allies', 'ALLIES', 'axis', 'Unknown']),
            'kingdom': R.choice(['Germany', 'germany', 'Neutral', '', 'Missing']),
            'ops': [{'kind': R.choice(['actor', 'technology']), 'id': R.choice(IDS)} for _ in range(R.randint(3, 8))]}


def main():
    cases = [make_case(i) for i in range(120)]
    fixtures, executions = [], 0
    for case in cases:
        for mode, cw in MODES:
            m = RestrictionMachine(cw)
            m.setup(case)
            results = [m.run(op) for op in case['ops']]
            executions += len(results)
            fixtures.append({'name': case['name'], 'precision': mode, 'case': case, 'expected': results})
    doc = {'schema': 1, 'pins': pin(RANGES), 'executions': executions, 'fixtures': fixtures}
    out = Path(__file__).resolve().parent / 'native-restriction-fixtures.json'
    out.write_text(json.dumps(doc, separators=(',', ':')) + '\n', encoding='utf-8')
    shutil.copyfile(out, ROOT / 'crates/aa-economy/tests/fixtures/native-restriction-fixtures.json')
    print(f'{len(fixtures)} fixtures, {executions} native executions')


if __name__ == '__main__':
    main()
