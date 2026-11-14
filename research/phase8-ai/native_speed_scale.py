"""Original 52D620(divisor, factor) on the element move state (speed
scaling) under Unicorn on the phase8-ai machine (native_machine.py), at
Single24, Double53 and Extended64, called directly with 529C30 unchanged.
Seam (recorded): 576D20 on the actor +6C locomotion (both arguments).
Divisors are nonzero finite binary32 values.

Writes native-speed-scale.json next to this script.
"""
from pathlib import Path
import json, random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import Machine, PRECISIONS, f32, header, instructions, sha
from native_uai_harness import body_instructions

ENTRY, END = 0x52d620, 0x52d65b


class ScaleMachine(Machine):
    def __init__(self, c):
        super().__init__(c['precision'])
        self.c = c
        self.state = self.alloc(0x40); mind = self.alloc(0x40); actor = self.alloc(0x100)
        self.loco = self.alloc(0x80)
        self.w(self.state + 4, mind); self.w(mind + 4, actor); self.w(actor + 0x6c, self.loco)
        self.w(self.state + 0x24, c['speed']); self.w(self.loco + 0x50, c['word50'])
        self.seam(0x576d20, self.s_speed)

    def s_speed(self, m):
        assert self.ecx() == self.loco
        self.events.append({'event': 'locomotion_set_speed', 'speed': self.arg(1), 'factor': self.arg(2)}); self.ret(0, 8)

    def run(self):
        r = self.call(ENTRY, ecx=self.state, args=[self.c['divisor'], self.c['factor']])
        return {'result': r & 0xff, 'speed': self.r(self.state + 0x24), 'events': self.events}


def cases():
    rnd = random.Random(0x52d620)
    rows = []
    pr = list(PRECISIONS)
    vals = lambda: f32(rnd.choice([1.0, 3.0, 0.1, 1.0 / 3.0, 7.0, 2.5, rnd.uniform(0.01, 20), rnd.uniform(-5, 5)]))
    for i in range(900):
        d = vals()
        if d in (0, 0x80000000):
            d = f32(1.0)
        rows.append({'name': 'scale%d' % i, 'precision': pr[i % 3], 'speed': vals(), 'divisor': d, 'factor': vals(),
                     'word50': vals()})
    return rows


def main():
    out, visited = [], {}
    for c in cases():
        m = ScaleMachine(c); result = m.run()
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': result})
    d = header(__file__, __doc__)
    cover = {'52D620': [len([a for a in visited if ENTRY <= a < END]), body_instructions(ENTRY, END)]}
    d.update(instructions=instructions(visited), coverage=cover, cases=out)
    p = HERE / 'native-speed-scale.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'instructions': len(visited), 'coverage': cover, 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
