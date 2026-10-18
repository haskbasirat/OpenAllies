"""Original NetNode send selector 0x763EE0 under Unicorn: over the NetNode's
connection list (+0x78, nodes {conn, next}) it skips a NetConn whose
bandwidth use +0x94 exceeds its allowance +0x98, asks GetWantedPacketType
0x74BDE0 on the AdminConn +0xA0 (0 without one) and picks the greatest
(wanted type, -NetConn+4) pair, strictly greater, first in list order on a
tie; 0 when the best type is 0. 0x74BDE0 is a recording seam returning the
case's value (it is proven by native-send-gate.json).
"""
from pathlib import Path
import json, random, struct, sys
HERE = Path(__file__).resolve().parent; ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase4-orders'))
from recover_orders import OrdersMachine, PE0, PE1, sha
sys.path.insert(0, str(HERE))
import null_guard
from unicorn.x86_const import *

NODE = 0x2050000; CONNS = 0x2051000; ADMINS = 0x2058000; LIST = 0x205f000

class SelectorMachine(OrdersMachine):
    def __init__(self):
        super().__init__()
        self.mode = 'selector'; self.wanted = {}; self.asked = []
        null_guard.install(self)
    def hook(self, u, pc, size, user):
        if self.mode == 'selector':
            if pc == 0x74bde0:
                a = u.reg_read(UC_X86_REG_ECX); self.asked.append(a)
                self.ret(0, self.wanted[a]); return
            self.visited[pc] = size
        super().hook(u, pc, size, user)

def main():
    m = SelectorMachine(); rng = random.Random(763); rows = []
    for k in range(400):
        n = rng.choice([0, 1, 2, 3, 5, 8])
        conns = []
        m.u.mem_write(LIST, b'\0' * 0x200)
        for i in range(n):
            c = CONNS + i * 0x100; a = ADMINS + i * 0x100
            used = rng.choice([0, 10, 100, 500]); allow = rng.choice([0, 10, 100, 1000])
            has_admin = rng.random() < 0.85
            wanted = rng.choice([0, 0, 2, 3, 4, 5, 5, 6, 7, 8, 9, 10, 11])
            last = rng.choice([0, 1, 2, 5, 9, -3, 0x7fffffff, -0x80000000])
            m.w(c + 0x94, used); m.w(c + 0x98, allow); m.w(c + 0xa0, a if has_admin else 0); m.w(c + 4, last & 0xffffffff)
            m.wanted[a] = wanted
            node = LIST + i * 8; m.w(node, c); m.w(node + 4, LIST + (i + 1) * 8 if i + 1 < n else 0)
            conns.append({'used': used, 'allowance': allow, 'admin': has_admin, 'wanted': wanted,
                          'last_served': struct.unpack('<i', struct.pack('<I', last & 0xffffffff))[0]})
        m.w(NODE + 0x78, LIST if n else 0)
        m.asked = []
        r = m.call(0x763ee0, [], this=NODE, callee_pop=0)
        sel = None if r == 0 else (r - CONNS) // 0x100
        rows.append({'conns': conns, 'selected': sel, 'asked': [(a - ADMINS) // 0x100 for a in m.asked]})
    ranges = []
    for a, size in sorted(m.visited.items()):
        if not PE0.base <= a < PE0.base + PE0.image_size: continue
        if ranges and ranges[-1][1] >= a: ranges[-1][1] = max(ranges[-1][1], a + size)
        else: ranges.append([a, a + size])
    windows = [{'start': hex(a), 'end': hex(b), 'sha256': sha(PE0.range(a, b - a)),
                'v14_identical': PE0.range(a, b - a) == PE1.range(a, b - a)} for a, b in ranges]
    report = {'schema': 1, 'scope': __doc__, 'original_sha256': sha(PE0.b), 'v14_sha256': sha(PE1.b),
              'producer_sha256': sha(Path(__file__).read_bytes()), 'cases': rows, 'executed_windows': windows}
    (HERE / 'native-send-selector.json').write_text(json.dumps(report) + '\n')
    print(len(rows), 'cases;', sum(r['selected'] is not None for r in rows), 'selected;',
          'v14 differs at', [w['start'] for w in windows if not w['v14_identical']])

if __name__ == '__main__':
    main()
