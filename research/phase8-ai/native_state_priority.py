"""Original Mind-state priority admission under Unicorn on the emulated
element Mind (native_uai_harness.py), called directly: 529F80(priority) on
the Mind and 529FC0 on an embedded state, with scripted metadata +4
priorities per state type and scripted stacks (including a null head
node). No seams.

Writes native-state-priority.json next to this script.
"""
from pathlib import Path
import json, random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_uai_harness import ELEMENT_STATES, DISCARD, MindMachine, body_instructions
from native_machine import header, instructions, sha

RANGES = {'529F80': (0x529f80, 0x529fb5), '529FC0': (0x529fc0, 0x529ff2)}


class PriorityMachine(MindMachine):
    def __init__(self, c):
        super().__init__(c, ELEMENT_STATES, DISCARD)
        for key, p in self.states.items():
            self.w(self.r(p + 8) + 4, c['priorities'][key] & 0xffffffff)

    def run(self):
        c = self.c
        if c['entry'] == '529F80':
            r = self.call(0x529f80, ecx=self.mind, args=[c['priority'] & 0xffffffff])
        else:
            r = self.call(0x529fc0, ecx=self.states[c['state']])
        return {'result': r & 0xff}


def cases():
    rnd = random.Random(0x529fc0)
    rows = []
    keys = list(ELEMENT_STATES)
    for i in range(600):
        rows.append({'name': 'priority%d' % i, 'precision': 'Single24', 'entry': '529F80' if i % 2 else '529FC0',
                     'priorities': {k: rnd.choice([0, 1, 2, 2, 3, 5, -1, -0x80000000]) for k in keys},
                     'stack': rnd.choice([[], [None], ['move'], ['sleep', 'move'], ['kill', 'engage'], ['deploy'], ['unpack']]),
                     'state': rnd.choice(keys), 'priority': rnd.choice([0, 1, 2, 3, -1, 5, 0x7fffffff])})
    return rows


def main():
    out, visited = [], {}
    for c in cases():
        m = PriorityMachine(c); result = m.run()
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': result})
    d = header(__file__, __doc__)
    cover = {k: [len([a for a in visited if lo <= a < hi]), body_instructions(lo, hi)] for k, (lo, hi) in RANGES.items()}
    d.update(instructions=instructions(visited), coverage=cover, cases=out)
    p = HERE / 'native-state-priority.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'instructions': len(visited), 'coverage': cover, 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
