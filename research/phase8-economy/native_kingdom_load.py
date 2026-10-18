"""Original GKingdom::Load v7+ property-record loop (0x4E8A09..0x4E8C2F) in
isolated x86, entered with the native frame words it reads (load version at
[esp+0x14], archive at [esp+0x40], second Load argument at [esp+0x44], the
shared counter slot [esp+0x24]) and ebp = kingdom.

Executed unchanged: the loop, 503080 source check, 4849F0 vector growth.
Seams: archive reader, 503B10 record Load (fixture fields), 404F60 / 5A1040
diagnostics, 405D60 formatting, allocator and string allocator.
"""
from pathlib import Path
import json, random, shutil, struct, sys
sys.path.insert(0, str(Path(__file__).resolve().parent))
from harness import Machine, MODES, ROOT, check_null_accesses, pin, ret_pop  # noqa: E402
from unicorn.x86_const import (UC_X86_REG_ECX, UC_X86_REG_EBP, UC_X86_REG_ESP, UC_X86_REG_EIP,  # noqa: E402
                               UC_X86_REG_FPCW)

RANGES = {'kingdom_load_properties_4E8A09': (0x4E8A09, 0x4E8C2F), 'source_check_503080': (0x503080, 0x50309B),
          'grow_4849F0': (0x4849F0, 0x484A60)}
K, WORLD, ARCHIVE, RVT, ALLOC, AVT2, DEFS = 0x2000000, 0x2001000, 0x2002000, 0x2002800, 0x2003000, 0x2003100, 0x2010000
STUB_READ, STUB_NIL, STUB_SAME, STUB_FREE = 0x3000100, 0x3000110, 0x3000120, 0x3000130
END = 0x4E8C2F


class LoopMachine(Machine):
    def __init__(self, cw, case):
        super().__init__(cw)
        self.case = case
        self.put(0xB36534, WORLD); self.put(WORLD + 0x2C, case['world_version'])
        self.put(ARCHIVE + 0x3C, RVT); self.put(RVT, STUB_READ)
        self.put(ALLOC, AVT2); self.put(AVT2 + 0xC, STUB_NIL); self.put(AVT2 + 0x10, STUB_SAME); self.put(AVT2 + 4, STUB_FREE)
        self.put(K + 0x318, 0); self.put(K + 0x31C, 0); self.put(K + 0x320, 0)
        self.stream = struct.pack('<I', len(case['records']))
        self.next = 0
        self.records = {}
        self.trace = []
        self.seam(0x6F7810, lambda m, esp: (self.alloc(self.read(esp + 4)), 0))
        self.seam(0x6F7430, self._free)
        self.seam(STUB_READ, self._read)
        self.seam(STUB_NIL, lambda m, esp: (self._header(), 0))
        self.seam(STUB_SAME, lambda m, esp: (ALLOC, 0))
        self.seam(STUB_FREE, ret_pop(1))
        self.seam(0x856370, lambda m, esp: (ALLOC, 0))
        self.seam(0x503B10, self._record_load)
        self.seam(0x404F60, self._log)
        self.seam(0x405D60, self._format)
        self.seam(0x5A1040, self._legacy_log)

    def _header(self):
        h = self.alloc(0x40)
        self.put(h, ALLOC); self.put(h + 0xC, 1)
        return h

    def _read(self, m, esp):
        dst, n = self.read(esp + 4), self.read(esp + 8)
        self.uc.mem_write(dst, self.stream[:n])
        self.stream = self.stream[n:]
        return None, 8

    def _record_load(self, m, esp):
        record = self.uc.reg_read(UC_X86_REG_ECX)
        assert self.read(esp + 4) == ARCHIVE
        r = self.case['records'][self.next]
        self.records[record] = self.next
        self.next += 1
        self.put(record + 8, r['source'] if r['source'] is not None else 0xFFFFFFFF)
        self.put(record + 0xC, DEFS + 0x40 * r['definition'])
        return None, 4

    def _free(self, m, esp):
        p = self.read(esp + 4)
        if p in self.records:
            self.trace.append({'kind': 'freed', 'record': self.records[p]})
        return None, 0

    def _log(self, m, esp):
        self.trace.append({'kind': 'ignored_log'})
        return None, 0

    def _format(self, m, esp):
        out = self.read(esp + 4)
        self.put(out, self._header() + 0x10)
        return None, 0

    def _legacy_log(self, m, esp):
        self.trace.append({'kind': 'legacy_log'})
        return None, 0

    def run(self):
        s = self.STACK
        for off in range(0, 0x60, 4):
            self.put(s + off, 0)
        self.put(s + 0x14, self.case['load_version'])
        self.put(s + 0x24, self.case['counter'])
        self.put(s + 0x40, ARCHIVE); self.put(s + 0x44, 0x77)
        uc = self.uc
        uc.reg_write(UC_X86_REG_ESP, s); uc.reg_write(UC_X86_REG_EBP, K); uc.reg_write(UC_X86_REG_FPCW, self.cw)
        uc.emu_start(0x4E8A09, END, count=1_000_000)
        check_null_accesses(self)
        assert uc.reg_read(UC_X86_REG_EIP) == END and uc.reg_read(UC_X86_REG_ESP) == s
        kept = [self.records[p] for p in self.words(self.read(K + 0x318), self.read(K + 0x31C))]
        return {'kept': kept, 'events': self.trace}


R = random.Random(0x4E8A09)


def make_case(i):
    return {'name': f'random-{i}', 'world_version': R.choice([0x14, 0x15, 0x16, 0x10]),
            'load_version': R.choice([7, 10, 14, 20]), 'counter': R.choice([0, 0, 3]),
            'records': [{'source': R.choice([None, 0, 1, 2, 3, 4, 7, 2]), 'definition': R.randint(1, 9)}
                        for _ in range(R.randint(0, 7))]}


def main():
    cases = [make_case(i) for i in range(120)]
    fixtures, executions = [], 0
    for case in cases:
        for mode, cw in MODES:
            m = LoopMachine(cw, case)
            fixtures.append({'name': case['name'], 'precision': mode, 'case': case, 'expected': m.run()})
            executions += 1
    doc = {'schema': 1, 'pins': pin(RANGES), 'executions': executions, 'fixtures': fixtures}
    out = Path(__file__).resolve().parent / 'native-kingdom-load-fixtures.json'
    out.write_text(json.dumps(doc, separators=(',', ':')) + '\n', encoding='utf-8')
    shutil.copyfile(out, ROOT / 'crates/aa-economy/tests/fixtures/native-kingdom-load-fixtures.json')
    print(f'{len(fixtures)} fixtures, {executions} native executions')


if __name__ == '__main__':
    main()
