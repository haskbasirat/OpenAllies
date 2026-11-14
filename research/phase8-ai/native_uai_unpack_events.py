"""Original element unpack state Enter 52EB90 and the unpack/reattach Leave
52EB50 under Unicorn on the emulated element Mind (native_uai_harness.py),
at Single24, Double53 and Extended64, called directly on the embedded
unpack (Mind+188) or reattach (Mind+178) state. 52A240 (within 0.1 of the
unpack point) and 529C20 run unchanged. Seams (recorded in order): 52DF60
on the move state (all seven arguments) and the state virtual +1C (leave).

Writes native-uai-unpack-events.json next to this script.
"""
from pathlib import Path
import json, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_uai_harness import ELEMENT_STATES, DISCARD, MindMachine, PRECISIONS, f32, body_instructions
from native_machine import header, instructions, sha

ENTER, ENTER_END = 0x52eb90, 0x52ebef
LEAVE, LEAVE_END = 0x52eb50, 0x52eb67
NEAR, NEAR_END = 0x52a240, 0x52a287
fl = lambda b: struct.unpack('<f', struct.pack('<I', b))[0]


class UnpackMachine(MindMachine):
    def __init__(self, c):
        super().__init__(c, ELEMENT_STATES, DISCARD)
        self.w(self.actor + 0x20, c['pos'][0]); self.w(self.actor + 0x24, c['pos'][1])
        u = self.states['unpack']
        self.w(u + 0x10, c['point'][0]); self.w(u + 0x14, c['point'][1])
        self.seam(0x52df60, self.s_move)

    def s_move(self, m):
        assert self.ecx() == self.states['move']
        self.ev('move_order', args=[self.arg(i) for i in range(1, 8)]); self.ret(0, 0x1c)

    def run(self):
        if self.c['entry'] == 'enter':
            self.call(ENTER, ecx=self.states['unpack'])
        else:
            self.call(LEAVE, ecx=self.states[self.c['this']])
        return dict(events=self.events, **self.snap())


def cases():
    rnd = random.Random(0x52eb90)
    rows = []
    pr = list(PRECISIONS)
    for i in range(600):
        st = {k: rnd.choice([0, 0, 1, 2]) for k in ELEMENT_STATES}
        pos = [f32(rnd.uniform(-1500, 1500)), f32(rnd.uniform(-1500, 1500))]
        d = rnd.choice([0.0, 0.05, 0.0999999, 0.1, 0.1000001, 0.0707106, 0.07071068, 0.2, 5.0])
        ang = rnd.choice([(1, 0), (0, 1), (0.70710678, 0.70710678), (-0.6, 0.8)])
        point = [f32(fl(pos[0]) + d * ang[0]), f32(fl(pos[1]) + d * ang[1])]
        rows.append({'name': 'unpack%d' % i, 'precision': pr[i % 3], 'entry': 'enter' if i % 3 else 'leave',
                     'this': rnd.choice(['unpack', 'reattach']), 'status': st, 'stack': [],
                     'pos': pos, 'point': point, 'null_point': rnd.choice([[0, 0], [f32(1.5), f32(-2.0)]])})
    return rows


def main():
    out, visited = [], {}
    for c in cases():
        m = UnpackMachine(c); result = m.run()
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': result})
    d = header(__file__, __doc__)
    cover = {'52EB90': [len([a for a in visited if ENTER <= a < ENTER_END]), body_instructions(ENTER, ENTER_END)],
             '52EB50': [len([a for a in visited if LEAVE <= a < LEAVE_END]), body_instructions(LEAVE, LEAVE_END)],
             '52A240': [len([a for a in visited if NEAR <= a < NEAR_END]), body_instructions(NEAR, NEAR_END)]}
    d.update(instructions=instructions(visited), coverage=cover, cases=out)
    p = HERE / 'native-uai-unpack-events.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'instructions': len(visited), 'coverage': cover, 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
