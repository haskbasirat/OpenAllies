"""Oracle: KKC_GActorCommand sync dump 0x4D9A40 on synchronizer A ([B3655C]).

Runs, in both pinned images at PC24/PC53/PC64, the ORIGINAL 0x4D9A40 and
natively inside it the ORIGINAL TGC_Synchronizer loggers 0x77DC50 (scope
marker), 0x77D930 (binary32 bits), 0x77DA70 (int), the window accessor
0x77D680, its growth 0x77CEE0 and the retire 0x77D7D0, over a laboratory
synchronizer (+0 enabled, +1 dump, +4 suspend, +8 hash, +C max entries, +10
window begin, +14 end, +18 ring capacity, +1C ring of 0x14-byte entries).

  4D9A40(label, command): 77DC50(label, 0); command != NULL:
  77D930("_command->m_value", [command+28], 0); 77DA70("_command->m_bits",
  [command+0C], 0). Every logger returns before any OS read unless
  enabled && suspend <= 0; then GetCurrentThreadId() == [B39828] and
  [B36528] == NULL or (+5C == 0 and +5D == 0) open it.

Seams (laboratory only, each recorded): the IAT GetCurrentThreadId
(0x9F01E4) returns the case thread, 0x856370 returns the laboratory CString
manager (vt+C: empty header), 0x405B80 CString assignment (stores the
source header and adds a reference). 0x77CD40 (ring reallocation) is never
reached (capacity 64 >> entries) and traps.

Labels are the two native callers' strings: "_command" (A2960C: 4ED360,
4DFE46, 57C76E) and "_cmd" (A346CC: every component command handler,
including KKC_RecruitComponent 583A70).

  python native_command_sync_4d9a40.py --write   (refuses to overwrite)
  python native_command_sync_4d9a40.py           (replay, byte-identical)
"""
from pathlib import Path
import itertools, random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import recruit_lab as lab  # noqa: E402
nm = lab.nm

OUT = HERE / 'native-command-sync-4d9a40.json'
FUNCTIONS = {'4D9A40': (0x4d9a40, 0x4d9a89), '77DC50': (0x77dc50, 0x77dcdb), '77D930': (0x77d930, 0x77d9c1),
             '77DA70': (0x77da70, 0x77db01), '77D680': (0x77d680, 0x77d6a7), '77CEE0': (0x77cee0, 0x77d002),
             '77D7D0': (0x77d7d0, 0x77d8fe)}
CAPACITY = 64
LABELS = {'_command': 0xA2960C, '_cmd': 0xA346CC}


class Machine(lab.Lab):
    def __init__(self, precision, image):
        super().__init__(precision, image)
        self.thread_reads = 0
        self.seam(0x856370, lambda m: m.ret(m.manager))
        self.seam(0x405B80, self._assign)
        self.seam(0x77CD40, self._realloc)
        self.w(0x9F01E4, self.stub(self._thread))

    def _thread(self, m):
        m.thread_reads += 1
        m.ret(m.case['thread'])

    def _assign(self, m):
        src = m.r(m.arg(1))
        m.w(src - 4, m.r(src - 4) + 1)
        m.w(m.ecx(), src)
        m.ret(m.ecx(), pop=4)

    def _realloc(self, m):
        raise AssertionError('ring reallocation 77CD40 is outside the laboratory domain')

    def run(self, case):
        self.case = case
        log = self.alloc(0x20)
        self.wb(log, case['enabled'])
        self.w(log + 4, case['suspend'])
        self.w(log + 8, case['hash'])
        self.w(log + 0xC, case['max'])
        start = (case['index'] + 1) & 0xffffffff
        self.w(log + 0x10, start)
        self.w(log + 0x14, start)
        self.w(log + 0x18, CAPACITY)
        self.w(log + 0x1C, self.alloc(0x14 * CAPACITY))
        self.w(0xB3655C, log)
        self.w(0xB39828, case['sim_thread'])
        if case['sai'] is None:
            self.w(0xB36528, 0)
        else:
            sai = self.alloc(0x60)
            self.wb(sai + 0x5C, case['sai'][0])
            self.wb(sai + 0x5D, case['sai'][1])
            self.w(0xB36528, sai)
        reads = []
        for call in case['calls']:
            before = self.thread_reads
            command = 0
            if call['command'] is not None:
                command = self.alloc(0x40)
                self.w(command + 0x0C, call['command']['flags'])
                self.w(command + 0x28, call['command']['value'])
            self.call(0x4D9A40, args=(LABELS[call['label']], command), pop=0)
            reads.append(self.thread_reads - before)
        begin, end = self.r(log + 0x10), self.r(log + 0x14)
        entries = []
        index = begin
        while index != end:
            slot = ((index - 0x80000000) & 0xffffffff) % CAPACITY
            e = self.r(log + 0x1C) + 0x14 * slot
            label = self.r(e)
            entries.append({'label': self.text(label) if label else None, 'kind': self.r(e + 4),
                            'value': self.r(e + 8), 'hash': self.r(e + 0x10) - (1 << 32) * (self.r(e + 0x10) >> 31)})
            index = (index + 1) & 0xffffffff
        signed = lambda v: v - (1 << 32) * (v >> 31)
        return {'hash': signed(self.r(log + 8)), 'begin': signed(begin), 'end': signed(end),
                'entries': entries, 'thread_reads': reads,
                'enabled': self.rb(log), 'suspend': signed(self.r(log + 4))}


def cases():
    rng = random.Random(0x4D9A40)
    out = []
    values = [0, 1, 0x7fc12345, 0xffffffff, 0x3f800000, 0x80000000]
    flags = [0, 0x5c, 0x7ff, 0x42, 0xffffffff]
    # Gate grid over one call per label.
    for enabled, suspend, thread_match, sai, label, cmd in itertools.product(
            [0, 1], [-1, 0, 1], [True, False], [None, (0, 0), (1, 0), (0, 1)], ['_command', '_cmd'], [False, True]):
        out.append({'enabled': enabled, 'suspend': suspend, 'thread': 7, 'sim_thread': 7 if thread_match else 8,
                    'sai': sai, 'hash': -1, 'index': 0, 'max': 0x10000,
                    'calls': [{'label': label, 'command': {'flags': 0x42, 'value': 0x7fc12345} if cmd else None}]})
    # Hash/value arithmetic, window positions and retire (small max).
    for _ in range(160):
        n = rng.randrange(1, 5)
        calls = []
        for _ in range(n):
            cmd = None if rng.random() < 0.25 else {'flags': rng.choice(flags + [rng.getrandbits(32)]),
                                                     'value': rng.choice(values + [rng.getrandbits(32)])}
            calls.append({'label': rng.choice(['_command', '_cmd']), 'command': cmd})
        out.append({'enabled': 1, 'suspend': rng.choice([0, 0, -5]), 'thread': 3, 'sim_thread': 3, 'sai': rng.choice([None, (0, 0)]),
                    'hash': rng.choice([-1, 0, 17, 65535, -65535, rng.randrange(-70000, 70000), rng.randrange(-2**31, 2**31)]),
                    'index': rng.choice([0, -1, 41, 0x7ffffffe, -0x80000000 + 5, rng.randrange(-2**31, 2**31 - 2)]),
                    'max': rng.choice([0x10000, 1, 2, 3, 5]), 'calls': calls})
    return out


def main():
    write = '--write' in sys.argv
    rows, visited = [], {}
    for label, image in lab.IMAGES.items():
        for pc in nm.PRECISIONS:
            m = Machine(pc, image)
            for case in cases():
                m.heap_mark = m.heap
                out = m.run(case)
                m.heap = m.heap_mark
                rows.append({'image': label, 'precision': pc, 'input': case, 'output': out})
            m.verify_v14()
            visited.update(m.visited)
    # The same case produces the same output in every image/precision.
    by = {}
    for r in rows:
        key = repr(r['input'])
        by.setdefault(key, r['output'])
        assert by[key] == r['output'], key
    doc = nm.header(__file__, 'unicorn original 4D9A40 + 77DC50/77D930/77DA70/77D680/77CEE0/77D7D0')
    doc['scope'] = __doc__
    doc['coverage'] = lab.coverage(FUNCTIONS, visited)
    doc['rows'] = rows
    doc['instructions'] = nm.instructions(visited)
    lab.write_or_replay(OUT, doc, write)
    print({k: f"{v['executed']}/{v['instructions']}" for k, v in doc['coverage'].items()}, len(rows), 'rows')


if __name__ == '__main__':
    main()
