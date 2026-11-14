"""Original reference-counted actor state 38: 4D7D30 (hold) and 4D7D70
(release) under Unicorn on the phase8-ai machine
(research/phase8-ai/native_machine.py), called directly on an actor whose
+174 count is a case input, with a sequence of holds and releases.

Seam (recorded in order): 4D64F0(state, enabled) on the actor; a case may
make it rewrite the +174 count to prove the count is reread after it.

Writes native-state-refcount.json next to this script.
"""
from pathlib import Path
import json, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / 'phase8-ai'))
from native_machine import Machine, header, instructions, sha  # noqa: E402
from native_uai_harness import body_instructions  # noqa: E402

HOLD, HOLD_END = 0x4d7d30, 0x4d7d66
RELEASE, RELEASE_END = 0x4d7d70, 0x4d7d98
i32 = lambda v: struct.unpack('<i', struct.pack('<I', v & 0xffffffff))[0]


class RefMachine(Machine):
    def __init__(self, c):
        super().__init__('Single24')
        self.c = c
        self.actor = self.alloc(0x200)
        self.w(self.actor + 0x174, c['count'] & 0xffffffff)
        self.rewrites = list(c['rewrites'])
        self.seam(0x4d64f0, self.s_state)

    def s_state(self, m):
        assert self.ecx() == self.actor
        self.events.append({'event': 'set_state', 'state': self.arg(1), 'enabled': self.arg(2)})
        if self.rewrites:
            v = self.rewrites.pop(0)
            if v is not None:
                self.w(self.actor + 0x174, v & 0xffffffff)
        self.ret(0, 8)

    def run(self):
        counts = []
        for op in self.c['ops']:
            self.call(HOLD if op == 'hold' else RELEASE, ecx=self.actor, args=[0x5eed])
            counts.append(i32(self.r(self.actor + 0x174)))
        return {'events': self.events, 'counts': counts}


def cases():
    rnd = random.Random(0x4d7d30)
    rows = []
    for i in range(300):
        rows.append({'name': 'refcount%d' % i, 'count': rnd.choice([0, 0, 1, 2, -1, -5, 0x7fffffff, -0x80000000]),
                     'ops': [rnd.choice(['hold', 'release']) for _ in range(rnd.randrange(1, 8))],
                     'rewrites': [rnd.choice([None, None, None, 0, 3, -2]) for _ in range(4)]})
    return rows


def main():
    out, visited = [], {}
    for c in cases():
        m = RefMachine(c); result = m.run()
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': result})
    d = header(__file__, __doc__)
    cover = {'4D7D30': [len([a for a in visited if HOLD <= a < HOLD_END]), body_instructions(HOLD, HOLD_END)],
             '4D7D70': [len([a for a in visited if RELEASE <= a < RELEASE_END]), body_instructions(RELEASE, RELEASE_END)]}
    d.update(instructions=instructions(visited), coverage=cover, cases=out)
    p = HERE / 'native-state-refcount.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'instructions': len(visited), 'coverage': cover, 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
