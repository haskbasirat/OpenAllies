"""Original kingdom statistics load/promotion in isolated x86.

Executed unchanged: 58EE80 ordinary statistics load, 58DBE0 relation
statistics load (+7BA3C0-free head insertion, 606750 rehash), 8589B0 version
reader, 4E3690 statistics promotion (58EB60/58D560 constructors, 96BE33/55A5B0
map destruction), 58EA00 mission score reset. Seams: archive reader virtuals
(read/is-storing), 58DB20 definition key reader (synthetic key ids), 401330
CString buffer, 41DFB0 version error, allocator.
"""
from pathlib import Path
import json, random, shutil, struct, sys
sys.path.insert(0, str(Path(__file__).resolve().parent))
from harness import MODES, ROOT, bits, pin, ret_pop  # noqa: E402
from native_statistics import StatisticsMachine, STATS, RELATIONS  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX  # noqa: E402

RANGES = {'statistics_load_58EE80': (0x58EE80, 0x58EF5B), 'relation_load_58DBE0': (0x58DBE0, 0x58DFB0),
          'version_8589B0': (0x8589B0, 0x8589FE), 'promote_4E3690': (0x4E3690, 0x4E37C6),
          'mission_reset_58EA00': (0x58EA00, 0x58EA23), 'ordinary_ctor_58EB60': (0x58EB60, 0x58EB9A),
          'relation_ctor_58D560': (0x58D560, 0x58D61A), 'map_rehash_606750': (0x606750, 0x60683C)}
ARCHIVE, RVT, ALLOC, AVT2 = 0x2005000, 0x2005800, 0x2005900, 0x2005A00
GTIDS, KOBJ, DEFS, KING = 0x2100000, 0x2006000, 0x2007000, 0x2008000
STUB_READ, STUB_STORING, STUB_NIL, STUB_FREE = 0x3000100, 0x3000110, 0x3000120, 0x3000130


def D(i): return DEFS + 0x40 * i
def KO(i): return KOBJ + 0x40 * i


class LoadMachine(StatisticsMachine):
    def __init__(self, cw):
        super().__init__(cw)
        self.put(ARCHIVE + 0x3C, RVT); self.put(RVT, STUB_READ); self.put(RVT + 0xC, STUB_STORING)
        self.put(ALLOC, AVT2); self.put(AVT2 + 0xC, STUB_NIL); self.put(AVT2 + 4, STUB_FREE)
        self.put(0xB364C0, GTIDS)
        for i in range(1, 6):
            self.put16(GTIDS + 2 * i + 4, 1); self.put(GTIDS + 4 * i + 0x20004, KO(i))
        self.stream = b''
        self.at = 0
        self.seam(STUB_READ, self._read)
        self.seam(STUB_STORING, lambda m, esp: (0, 0))
        self.seam(STUB_NIL, lambda m, esp: (self._header(0), 0))
        self.seam(STUB_FREE, ret_pop(1))
        self.seam(0x856370, lambda m, esp: (ALLOC, 0))
        self.seam(0x401330, self._buffer)
        self.seam(0x41DFB0, self._error)
        self.seam(0x58DB20, self._key)

    def _header(self, capacity):
        h = self.alloc(0x10 + 2 * (capacity + 1))
        self.put(h, ALLOC); self.put(h + 4, 0); self.put(h + 8, capacity); self.put(h + 0xC, 1)
        return h

    def _read(self, m, esp):
        assert self.uc.reg_read(UC_X86_REG_ECX) == ARCHIVE + 0x3C
        dst, n = self.read(esp + 4), self.read(esp + 8)
        chunk = self.stream[self.at:self.at + n]
        assert len(chunk) == n, 'stream underrun'
        self.uc.mem_write(dst, chunk)
        self.at += n
        return None, 8

    def _buffer(self, m, esp):
        this = self.uc.reg_read(UC_X86_REG_ECX)
        n = self.read(esp + 4)
        self.put(this, self._header(n) + 0x10)
        return None, 4

    def _error(self, m, esp):
        raise AssertionError('native version error path')

    def _key(self, m, esp):
        out = self.read(esp + 8)
        (k,) = struct.unpack('<I', self.stream[self.at:self.at + 4])
        self.at += 4
        self.put(out, D(k) if k else 0)
        return None, 0

    def load_statistics(self, version, words, prior):
        self.put_words(STATS, prior[:10]); self.put_words(STATS + 0x28, prior[10:])
        self.stream = struct.pack('<I', version) + struct.pack('<' + 'I' * len(words), *words)
        self.at = 0
        self.call(0x58EE80, [ARCHIVE], STATS)
        assert self.at == len(self.stream)
        return {'production': self.words(STATS, 10), 'scores': self.words(STATS + 0x28, 56)}

    def load_relations(self, version, groups):
        out = struct.pack('<I', version)
        for family in (groups[:1] if version == 0 else groups):  # version 0: one family
            for group in family:
                out += struct.pack('<I', len(group))
                for key, value in group:
                    if version == 0:
                        name = 'k%d' % key
                        out += struct.pack('<H', len(name)) + name.encode('utf-16le') + struct.pack('<I', value)
                    else:
                        out += struct.pack('<II', key, value)
        self.stream, self.at = out, 0
        self.call(0x58DBE0, [ARCHIVE], RELATIONS)
        assert self.at == len(self.stream)
        snap = self.lookup_snapshot()
        result = []
        for slot, r in enumerate(snap):
            values = {}
            for key, value in r['values'].items():
                p = int(key)
                values[str((p - DEFS) // 0x40 if slot < 5 else (p - KOBJ) // 0x40)] = value
            leader = r['leader']
            if leader is not None:
                leader = (leader - DEFS) // 0x40 if slot < 5 else (leader - KOBJ) // 0x40
            result.append({'values': values, 'maximum': r['maximum'], 'leader': leader})
        return result

    def lookup_snapshot(self):
        """Per key, the value a lookup finds (first node of its chain)."""
        relations = []
        for i in range(10):
            m = RELATIONS + 0x10 * i
            buckets, count = self.read(m), self.read(m + 4)
            values, nodes = {}, 0
            for bucket in range(count):
                node = self.read(buckets + 4 * bucket)
                while node:
                    values.setdefault(str(self.read(node)), self.read(node + 4))
                    nodes += 1
                    node = self.read(node + 8)
            assert nodes == self.read(m + 8)
            index = 52 + i
            leader = self.read(RELATIONS + 4 * index - 0x30)
            relations.append({'values': values, 'maximum': self.read(RELATIONS + 4 * index - 8),
                              'leader': leader or None})
        return relations

    def promote(self, case):
        k = KING
        pointers = {}
        for off, key, size, ctor in ((0x32C, 'active', 0x108, 0x58EB60), (0x330, 'loaded', 0x108, 0x58EB60),
                                     (0x340, 'relations', 0xF0, 0x58D560), (0x344, 'loaded_relations', 0xF0, 0x58D560)):
            if case[key] is None:
                self.put(k + off, 0)
                continue
            obj = self.alloc(size)
            self.call(ctor, [1] if size == 0x108 else [], obj)
            if size == 0x108:
                self.put_words(obj + 0x28, case[key])
            pointers[obj] = key
            self.put(k + off, obj)
        self.call(0x4E3690, [], k)
        self.call(0x58EA00, [], self.read(k + 0x32C))
        active = self.read(k + 0x32C)
        return {'active': pointers.get(active, 'created'), 'active_scores': self.words(active + 0x28, 56),
                'loaded': self.read(k + 0x330), 'relations': pointers.get(self.read(k + 0x340), 'created'),
                'loaded_relations': self.read(k + 0x344)}


R = random.Random(0x58EE80)


def word():
    return bits(R.choice([0.0, 1.0, 3.5, -2.0, 1e6, R.uniform(0, 500)])) if R.random() < 0.95 else R.choice([0x7FC00000, 0xFF800000])


def main():
    fixtures, executions = [], 0
    for i in range(60):
        version = i % 2
        words = [word() for _ in range(60 if version == 0 else 66)]
        prior = [word() for _ in range(66)]
        groups_version = R.choice([0, 1, 2, 2])
        count = 4 + (groups_version > 1)
        keys = list(range(1, 9))

        def group(kingdom):
            out = []
            for _ in range(R.randint(0, 4)):
                if kingdom:
                    key = R.choice([(1 << 16) | R.randint(1, 5), (2 << 16) | R.randint(1, 5)])
                else:
                    key = R.choice(keys + [0])
                out.append([key, R.choice([bits(R.uniform(0, 20)), bits(5.0), 0x7FC00000, bits(-1.0)])])
            return out
        groups = [[group(False) for _ in range(count)], [group(True) for _ in range(count)]]
        promote = {k: (None if R.random() < 0.3 else [word() for _ in range(56)]) for k in ('active', 'loaded')}
        promote.update({k: (None if R.random() < 0.3 else True) for k in ('relations', 'loaded_relations')})
        case = {'name': f'random-{i}', 'version': version, 'words': words, 'prior': prior,
                'relation_version': groups_version, 'groups': groups, 'promote': promote}
        for mode, cw in MODES:
            m = LoadMachine(cw)
            expected = {'statistics': m.load_statistics(version, words, prior),
                        'relations': m.load_relations(groups_version, groups)}
            m2 = LoadMachine(cw)
            expected['promote'] = m2.promote(promote)
            executions += 4
            fixtures.append({'name': case['name'], 'precision': mode, 'case': case, 'expected': expected})
    doc = {'schema': 1, 'pins': pin(RANGES), 'executions': executions, 'fixtures': fixtures}
    out = Path(__file__).resolve().parent / 'native-statistics-load-fixtures.json'
    out.write_text(json.dumps(doc, separators=(',', ':')) + '\n', encoding='utf-8')
    shutil.copyfile(out, ROOT / 'crates/aa-economy/tests/fixtures/native-statistics-load-fixtures.json')
    print(f'{len(fixtures)} fixtures, {executions} native executions')


if __name__ == '__main__':
    main()
