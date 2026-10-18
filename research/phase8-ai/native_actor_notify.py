"""Original SAI actor (re)classification 7A0C20 executing unchanged under
Unicorn. Seams (recorded in order): 79E300 and 79E080 (predicates), 4CEFF0
and the actor virtual +20 on the actor, 79F6E0 / 7A0390 / 7A0A40 on the SAI.
Memory: SAI +20 (ready), actor +8 (flags), +C8 and +140 (category).

Cases: every combination of the ready byte, +C8, the predicates, the
category (2..5 and others), the removed flag; the SAI ready byte may be
cleared by the 4CEFF0 seam to reach the re-check of +20.
"""
from pathlib import Path
import itertools, json, random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import Machine, header, instructions, sha


class NotifyMachine(Machine):
    def __init__(self, c):
        super().__init__('Double53')
        self.c = c
        self.sai = self.alloc(0x40); self.wb(self.sai + 0x20, int(c['ready']))
        vt = self.alloc(0x40); self.w(vt + 0x20, self.stub(self.v20))
        self.actor = self.alloc(0x200); self.w(self.actor, vt)
        self.w(self.actor + 8, c['flags']); self.w(self.actor + 0xc8, int(c['c8'])); self.w(self.actor + 0x140, c['category'])
        for addr, fn in [(0x79e300, self.s_79e300), (0x79e080, self.s_79e080), (0x4ceff0, self.s_4ceff0),
                         (0x79f6e0, self.s_unregister), (0x7a0390, self.s_register), (0x7a0a40, self.s_unit)]:
            self.seam(addr, fn)

    def ev(self, op, **kw):
        self.events.append({'op': op, **kw})

    def s_79e300(self, m):
        assert self.arg(1) == self.actor
        r = self.c['p79e300']; self.ev('79e300', result=r); self.ret(int(r))

    def s_79e080(self, m):
        assert self.arg(1) == self.actor
        r = self.c['p79e080']; self.ev('79e080', result=r); self.ret(int(r))

    def s_4ceff0(self, m):
        assert self.ecx() == self.actor
        r = self.c['p4ceff0']; self.ev('4ceff0', result=r)
        if self.c['clear_ready']:
            self.wb(self.sai + 0x20, 0)
        self.ret(int(r))

    def v20(self, m):
        assert self.ecx() == self.actor and self.arg(1) == 0
        r = self.c['v20']; self.ev('virtual_20', result=r); self.ret(int(r), 4)

    def s_unregister(self, m):
        assert self.ecx() == self.sai and self.arg(1) == self.actor and self.arg(2) == 0
        self.ev('unregister'); self.ret(0, 8)

    def s_register(self, m):
        assert self.ecx() == self.sai and self.arg(1) == self.actor and self.arg(3) == 0
        self.ev('register', category=self.arg(2)); self.ret(0, 12)

    def s_unit(self, m):
        assert self.ecx() == self.sai and self.arg(1) == self.actor and self.arg(3) == 0 and self.arg(4) == 0
        self.ev('register_unit', flag=self.arg(2) & 0xff); self.ret(0, 16)

    def run(self):
        self.call(0x7a0c20, ecx=self.sai, args=[self.actor], pop=4)
        return {'events': self.events}


def cases():
    rows = []
    for ready, c8, p300, p4ceff0, v20, p080, category, removed in itertools.product(
            [False, True], [False, True], [False, True], [False, True], [False, True], [False, True],
            [2, 3, 4, 5, 0], [False, True]):
        rows.append({'ready': ready, 'c8': c8, 'p79e300': p300, 'p4ceff0': p4ceff0, 'v20': v20, 'p79e080': p080,
                     'category': category, 'flags': 0x10 | int(removed), 'clear_ready': False})
    rnd = random.Random(0x7a0c20)
    for _ in range(40):
        rows.append({'ready': True, 'c8': False, 'p79e300': False, 'p4ceff0': True, 'v20': rnd.random() < 0.5,
                     'p79e080': False, 'category': rnd.choice([2, 3, 5]), 'flags': rnd.choice([0, 1]),
                     'clear_ready': True})
    return rows


def main():
    out, visited = [], {}
    for c in cases():
        m = NotifyMachine(c)
        r = m.run()
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': r})
    d = header(__file__, __doc__)
    d.update(instructions=instructions(visited), cases=out)
    p = HERE / 'native-actor-notify.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    body = [a for a in visited if 0x7a0c20 <= a < 0x7a0d37]
    print(json.dumps({'cases': len(out), 'body_instructions': len(body), 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
