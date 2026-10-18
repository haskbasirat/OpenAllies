"""Original TGC_Synchronizer under Unicorn: constructor 0x77CCB0, loggers
0x77DA70 (int), 0x77D930 (float), 0x77D9D0 (bool), 0x77DB10 (string), reset
0x77D900 and ValidateChecksum 0x77D050, including the native windowed entry
array (0x77D680, 0x77CEE0, 0x77D7D0, 0x77D6B0, 0x77D010) with small maximum
sizes so retention and forgetting are exercised.

Seams: allocator / CString manager / logging (Phase3+4 machines);
GetCurrentThreadId (IAT 0x9F01E4) returns the value at [0xB39828] (simulation
thread); [0xB36528] = 0 (application flags clear).
"""
from pathlib import Path
import json, random, struct, sys
HERE = Path(__file__).resolve().parent; ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase4-orders'))
from recover_orders import OrdersMachine, PE0, PE1, sha
sys.path.insert(0, str(HERE))
import null_guard
from unicorn.x86_const import *

STUB = 0x7700000
SYNC = 0x2014000

class SyncMachine(OrdersMachine):
    def __init__(self):
        super().__init__()
        self.mode = 'sync'
        null_guard.install(self)
        self.u.mem_map(STUB, 0x1000)
        self.w(0x9f01e4, STUB); self.w(0xb39828, 0x1234); self.w(0xb36528, 0)
        self.thread = 0x1234
    def hook(self, u, pc, size, user):
        if self.mode == 'sync':
            self.visited[pc] = size
            if pc == STUB:
                self.ret(0, self.thread); return
            if pc == 0x404f60:
                self.ret(); return
            if pc == 0x77da70:
                return  # execute the ORIGINAL int logger (the Phase3 machine seams it)
        super().hook(u, pc, size, user)
    def state(self):
        s = lambda p: struct.unpack('<i', self.u.mem_read(p, 4))[0]
        begin, end, cap, data = s(SYNC + 0x10), s(SYNC + 0x14), self.r(SYNC + 0x18), self.r(SYNC + 0x1c)
        entries = []
        for i in range(begin, end):
            slot = ((i + 0x80000000) & 0xffffffff) % cap
            e = data + slot * 0x14
            entries.append({'kind': self.r(e + 4), 'value': self.r(e + 8), 'hash': s(e + 0x10)})
        return {'enabled': self.u.mem_read(SYNC, 1)[0], 'file_dump': self.u.mem_read(SYNC + 1, 1)[0],
                'suspend': s(SYNC + 4), 'hash': s(SYNC + 8), 'max': s(SYNC + 0xc), 'begin': begin, 'end': end, 'entries': entries}

def sequence(m, rng, max_entries):
    m.u.mem_write(SYNC, b'\0' * 0x40)
    m.call(0x77ccb0, [max_entries], this=SYNC, callee_pop=4)
    ops = [{'op': 'new', 'max': max_entries, 'state': m.state()}]
    def label(): return m.string(rng.choice(['o.GetBitSize()', 'm_decree_type.m_id', 'x']))
    for step in range(rng.randrange(20, 60)):
        k = rng.randrange(10)
        if k == 0 or step == 3:
            idx = rng.choice([-1, 0, 5, 100, rng.randrange(-5, 50)]); h = rng.choice([-1, 0, 1234, rng.randrange(-70000, 70000)])
            m.call(0x77d900, [idx & 0xffffffff, h & 0xffffffff], this=SYNC, callee_pop=8)
            ops.append({'op': 'reset', 'index': idx, 'hash': h})
        elif k <= 3:
            v = rng.choice([0, 1, 37, 65535, 65536, 0x7fffffff, 0x80000000, 0xffffffff, rng.getrandbits(32)])
            m.call(0x77da70, [label(), v, 0], this=SYNC, callee_pop=12)
            ops.append({'op': 'int', 'value': v})
        elif k == 4:
            v = rng.choice([0x3f800000, 0xbf800000, 0x7fc00000, rng.getrandbits(32)])
            m.call(0x77d930, [label(), v, 0], this=SYNC, callee_pop=12)
            ops.append({'op': 'float', 'bits': v})
        elif k == 5:
            v = rng.randrange(2)
            m.call(0x77d9d0, [label(), v, 0], this=SYNC, callee_pop=12)
            ops.append({'op': 'bool', 'value': v})
        elif k == 6:
            text = ''.join(rng.choice('AZaz09 é中') for _ in range(rng.randrange(0, 6)))
            m.call(0x77db10, [label(), m.string(text), 0], this=SYNC, callee_pop=12)
            ops.append({'op': 'text', 'units': list(text.encode('utf-16-le')[i] | text.encode('utf-16-le')[i + 1] << 8 for i in range(0, len(text) * 2, 2))})
        elif k == 7:
            st = m.state()
            idx = rng.choice([st['begin'] - 1, st['begin'], st['end'] - 1, st['end'], st['end'] + 1, rng.randrange(-3, 40)])
            h = rng.choice([st['hash'], 0, rng.randrange(-65535, 65536)] + [e['hash'] for e in st['entries'][-2:]])
            r = m.call(0x77d050, [idx & 0xffffffff, h & 0xffffffff], this=SYNC, callee_pop=8) & 0xff
            ops.append({'op': 'validate', 'index': idx, 'hash': h, 'result': bool(r)})
        elif k == 8:
            s = rng.choice([0, 1, -1]); m.w(SYNC + 4, s & 0xffffffff); ops.append({'op': 'suspend', 'value': s})
        else:
            t = rng.choice([0x1234, 0x1234, 0x99]); m.thread = t; ops.append({'op': 'thread_gate', 'open': t == 0x1234})
        ops[-1]['state'] = m.state()
    m.thread = 0x1234
    return ops

def main():
    m = SyncMachine(); rng = random.Random(77); rows = []
    for max_entries in [1, 2, 3, 5, 8, 64]:  # 0x10000 behaves alike but its 65536 ctor calls exceed the step budget
        for _ in range(6):
            rows.append(sequence(m, rng, max_entries))
    ranges = []
    for a, size in sorted(m.visited.items()):
        if not PE0.base <= a < PE0.base + PE0.image_size: continue
        if ranges and ranges[-1][1] >= a: ranges[-1][1] = max(ranges[-1][1], a + size)
        else: ranges.append([a, a + size])
    windows = []
    for a, b in ranges:
        raw = PE0.range(a, b - a); assert raw == PE1.range(a, b - a), hex(a)
        windows.append({'start': hex(a), 'end': hex(b), 'sha256': sha(raw)})
    report = {'schema': 1, 'scope': __doc__, 'original_sha256': sha(PE0.b), 'v14_sha256': sha(PE1.b),
              'producer_sha256': sha(Path(__file__).read_bytes()), 'sequences': rows, 'executed_windows': windows}
    (HERE / 'native-sync.json').write_text(json.dumps(report) + '\n')
    print(len(rows), 'sequences', sum(len(r) for r in rows), 'operations')

if __name__ == '__main__':
    main()
