"""Original 52E6A0(x, y) on the element move state (the point lies inside
one of the two turning circles) under Unicorn, at Single24, Double53 and
Extended64, on the phase8-ai machine (research/phase8-ai/native_machine.py).

Runs unchanged with 529C30 on a state whose Mind +4 actor holds +20/+24
(position), +2C/+30 (facing) and +6C (locomotion, +30 turning radius). No
seams. Points are placed around both circle boundaries.

Writes native-turn-circle.json next to this script.
"""
from pathlib import Path
import json, math, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / 'phase8-ai'))
from native_machine import Machine, PRECISIONS, f32, header, instructions, sha  # noqa: E402
from native_uai_harness import body_instructions  # noqa: E402

ENTRY, END = 0x52e6a0, 0x52e78d
fl = lambda b: struct.unpack('<f', struct.pack('<I', b))[0]


class TurnMachine(Machine):
    def __init__(self, c):
        super().__init__(c['precision'])
        self.c = c
        self.state = self.alloc(0x40); mind = self.alloc(0x40); actor = self.alloc(0x100); loco = self.alloc(0x40)
        self.w(self.state + 4, mind); self.w(mind + 4, actor); self.w(actor + 0x6c, loco)
        self.w(loco + 0x30, c['radius'])
        self.w(actor + 0x20, c['pos'][0]); self.w(actor + 0x24, c['pos'][1])
        self.w(actor + 0x2c, c['facing'][0]); self.w(actor + 0x30, c['facing'][1])

    def run(self):
        r = self.call(ENTRY, ecx=self.state, args=list(self.c['point']))
        return {'result': r & 0xff}


def cases():
    rnd = random.Random(0x52e6a0)
    rows = []
    pr = list(PRECISIONS)
    for i in range(1500):
        radius = rnd.choice([0.0, 1.0, 2.5, 7.3, 15.0, rnd.uniform(0.1, 30)])
        ang = rnd.uniform(0, 2 * math.pi)
        facing = [f32(math.cos(ang)), f32(math.sin(ang))]
        if rnd.random() < 0.1:
            facing = rnd.choice([[f32(1.0), 0], [0, f32(-1.0)], [f32(0.6), f32(0.8)]])
        pos = [f32(rnd.uniform(-3000, 3000)), f32(rnd.uniform(-3000, 3000))]
        r = f32(radius); rf = fl(r)
        fx, fy = fl(facing[0]), fl(facing[1])
        px, py = fl(pos[0]), fl(pos[1])
        centre = rnd.choice([(px + rf * fy, py - rf * fx), (px - rf * fy, py + rf * fx)])
        t = rnd.uniform(0, 2 * math.pi)
        d = rf * rnd.choice([1.0, 1.0, 1.0 + 1e-7, 1.0 - 1e-7, 0.5, 1.5, 0.0]) + rnd.choice([0.0, 0.0, 5e-6, -5e-6, 1e-5])
        point = [f32(centre[0] + d * math.cos(t)), f32(centre[1] + d * math.sin(t))]
        rows.append({'name': 'turn%d' % i, 'precision': pr[i % 3], 'radius': r, 'pos': pos, 'facing': facing, 'point': point})
    return rows


def main():
    out, visited = [], {}
    for c in cases():
        m = TurnMachine(c); result = m.run()
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': result})
    d = header(__file__, __doc__)
    cover = {'52E6A0': [len([a for a in visited if ENTRY <= a < END]), body_instructions(ENTRY, END)]}
    d.update(instructions=instructions(visited), coverage=cover, cases=out)
    p = HERE / 'native-turn-circle.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'instructions': len(visited), 'coverage': cover,
                      'inside': sum(r['output']['result'] for r in out), 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
