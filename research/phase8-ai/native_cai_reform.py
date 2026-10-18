"""Original KKC_CAIStateReform::Think 51DE70 (regiment state cai_reform at
Mind+1B8, vtable A32A80 slot +C) executing unchanged on an emulated regiment
Mind (native_cai_regiment.py) with 529E60, 52A000 and 529C30, at x87
precision Single24, Double53 and Extended64 (the timeout test subtracts two
binary32 values at the live precision).

Emulated data read natively: the map singleton [B36534] +E0 (binary32 game
time), the SVar table [B39AB0] +B4 -> binary32 reform timeout, state+10
(binary32 reform start), Mind+2B4/+2B8 (sleep point), the null point
[B279C0]/[B279C4] and the organization component +40 word.

Seams (recorded in order with the tracked statuses and the Mind stack): Mind
virtuals +178 +11C (tail jumps), Updatable +8 virtual +4 (schedule),
57BF30(x, y) on the organization component, 4CEDE0 on the component +40
object; allocator, free and loggers as in native_cai_regiment.py.
Signalling NaN inputs are avoided (the emulator does not quiet them on fld).
"""
from pathlib import Path
import json, random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import PRECISIONS, f32, header, instructions, sha
from native_cai_regiment import RegimentMachine, coverage

TRACKED = ['reform', 'sleep', 'move', 'other']
MAP = 0x0bad0000


class ReformMachine(RegimentMachine):
    def __init__(self, c):
        super().__init__(c, mind_virtuals=[(0x178, 'mind_virtual_178', 0), (0x11c, 'mind_virtual_11c', 0)])
        s = self.states['reform']
        self.w(s + 0x10, c['start'])
        mp = self.alloc(0x100); self.w(0xb36534, mp); self.w(mp + 0xe0, c['time'])
        table = self.alloc(0x100); self.w(0xb39ab0, table)
        thr = self.alloc(4); self.w(thr, c['timeout']); self.w(table + 0xb4, thr)
        self.w(0xb279c0, c['null_point'][0]); self.w(0xb279c4, c['null_point'][1])
        self.w(self.mind + 0x2b4, c['point'][0]); self.w(self.mind + 0x2b8, c['point'][1])
        self.w(self.comp + 0x40, c['word40'])
        self.seam(0x57bf30, self.s_move)
        self.seam(0x4cede0, self.s_4cede0)

    def s_move(self, m):
        assert self.ecx() == self.comp
        self.ev('organization_move', point=[self.arg(1), self.arg(2)]); self.ret(0, 8)

    def s_4cede0(self, m):
        r = self.c['check_4cede0']
        self.ev('check_4cede0', object=self.ecx(), result=r); self.ret(int(r))

    def run(self):
        fault = self.run_think(0x51de70, 'reform')
        if fault:
            return fault
        return {'events': self.events, **self.snap(), 'point': [self.r(self.mind + 0x2b4), self.r(self.mind + 0x2b8)]}


def cases():
    rnd = random.Random(0x51de70)
    precisions = list(PRECISIONS)
    stacks = [['reform'], ['reform'], ['reform', 'sleep'], ['other', 'reform'], [], ['reform', None], [None]]
    rows = []
    # Exact-difference boundaries: (time, start, timeout) where the rounded
    # difference decides differently at PC24 and PC53/PC64.
    crafted = [(2.0 ** 24, 0.25, 2.0 ** 24), (2.0 ** 24, 0.75, 2.0 ** 24 - 1), (1e30, 1.0, 1e30),
               (100.0, 40.0, 60.0), (100.0, 40.0, 60.000004), (5.0, 5.0, 0.0), (5.0, 5.0, -0.0),
               (0.0, 0.0, 0.0), (-3.0, 2.0, -5.0), (3.4e38, -3.4e38, 3.4e38), (2.0 ** 25 + 2, 1.0, 2.0 ** 25),
               (1.5e-45, 0.0, 1.4e-45)]
    for i in range(360):
        if i < len(crafted) * 3:
            t, s, th = crafted[i // 3]
            time, start, timeout = f32(t), f32(s), f32(th)
        else:
            time = f32(rnd.uniform(0, 5000))
            start = rnd.choice([f32(rnd.uniform(0, 5000)), time, f32(rnd.uniform(-10, 10))])
            timeout = f32(rnd.choice([0.0, 5.0, 30.0, 120.0, rnd.uniform(0, 3000)]))
        null_point = rnd.choice([[0, 0], [f32(3.5), f32(-2)], [0x7fc00000, 0], [0, 0x7fc00001]])
        point = rnd.choice([null_point, null_point, [f32(rnd.uniform(-50, 50)), f32(rnd.uniform(-50, 50))],
                            [null_point[0], f32(1)], [0x80000000, null_point[1]], [0x7fc00000, 0]])
        c = {'name': 'reform%d' % i, 'precision': precisions[i % 3], 'time': time, 'start': start,
             'timeout': timeout, 'null_point': null_point, 'point': point,
             'word40': rnd.getrandbits(32) | 1, 'check_4cede0': rnd.random() < 0.5,
             'states': {'reform': rnd.choice([0, 1, 2, 2]), 'sleep': rnd.choice([0, 1]), 'move': rnd.choice([0, 2]),
                        'other': rnd.choice([1, 2])},
             'stack': ['reform'] if i < len(crafted) * 3 else rnd.choice(stacks),
             'words': [rnd.getrandbits(32) for _ in range(4)], 'can_activate': {}, 'tracked': TRACKED}
        rows.append(c)
    return rows


def main():
    out, visited = [], {}
    for c in cases():
        m = ReformMachine(c)
        result = m.run()
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': result})
    cov = coverage(visited, {'think_51de70': (0x51de70, 0x51df80)})
    d = header(__file__, __doc__)
    d.update(instructions=instructions(visited), coverage=cov, cases=out)
    p = HERE / 'native-cai-reform.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'faults': sum('fault' in r['output'] for r in out),
                      'instructions': len(visited),
                      'coverage': {k: [v['visited'], v['instructions'], v['unvisited']] for k, v in cov.items()},
                      'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
