"""Original element Mind unreachable-target memory under Unicorn on the
phase8-ai machine (native_machine.py): 532840(object, other) with 530E00
and 530ED0 (record push at the head) runs unchanged for a sequence of
calls on one Mind (list Mind+1B0 / +1B4), followed by 530E00 queries.
The allocator 6F7810 is a bump-heap seam. World time [B36534]+E0 and the
null point [B279C0] are case inputs; objects carry +14 GTIDs and +20/+24
positions.

Writes native-unreachable-memory.json next to this script.
"""
from pathlib import Path
import json, random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import Machine, f32, header, instructions, sha
from native_uai_harness import body_instructions

RANGES = {'530E00': (0x530e00, 0x530e48), '532840': (0x532840, 0x5328d2), '530ED0': (0x530ed0, 0x530f03)}


class MemoryMachine(Machine):
    def __init__(self, c):
        super().__init__('Single24')
        self.c = c
        self.seam(0x6f7810, lambda m: self.ret(self.alloc(max(self.arg(1), 4))))
        world = self.alloc(0x200); self.w(0xb36534, world); self.world = world
        self.w(0xb279c0, c['null_point'][0]); self.w(0xb279c4, c['null_point'][1])
        self.mind = self.alloc(0x400)
        self.objects = {}
        for key, o in c['objects'].items():
            p = self.objects[key] = self.alloc(0x40)
            self.w(p + 0x14, o['gtid']); self.w(p + 0x20, o['pos'][0]); self.w(p + 0x24, o['pos'][1])

    def obj(self, key):
        return 0 if key is None else self.objects[key]

    def list(self):
        out, n = [], self.r(self.mind + 0x1b0)
        while n:
            out.append([self.r(n), self.r(n + 4), self.r(n + 8), self.r(n + 0xc), self.r(n + 0x10)]); n = self.r(n + 0x14)
        tail = self.r(self.mind + 0x1b4)
        return out, tail

    def run(self):
        for step in self.c['remember']:
            self.w(self.world + 0xe0, step['time'])
            self.call(0x532840, ecx=self.mind, args=[self.obj(step['object']), self.obj(step['other'])])
        queries = [self.call(0x530e00, ecx=self.mind, args=[self.obj(k)]) & 0xff for k in self.c['queries']]
        records, tail = self.list()
        first = self.r(self.mind + 0x1b0)
        last = first
        while last and self.r(last + 0x14):
            last = self.r(last + 0x14)
        return {'records': records, 'queries': queries, 'tail_is_first_pushed': tail == last}


def cases():
    rnd = random.Random(0x532840)
    rows = []
    for i in range(200):
        objects = {'o%d' % k: {'gtid': rnd.choice([(0x60 + k) << 16 | (0x600 + k), 0]) if k == 3 else (0x60 + k) << 16 | (0x600 + k),
                               'pos': [f32(rnd.uniform(-90, 90)), f32(rnd.uniform(-90, 90))]} for k in range(4)}
        keys = list(objects) + [None]
        rows.append({'name': 'memory%d' % i, 'objects': objects, 'null_point': rnd.choice([[0, 0], [f32(2.5), f32(-4.0)]]),
                     'remember': [{'object': rnd.choice(keys), 'other': rnd.choice(keys), 'time': f32(rnd.uniform(0, 500))}
                                  for _ in range(rnd.randrange(0, 7))],
                     'queries': [rnd.choice(keys) for _ in range(4)]})
    return rows


def main():
    out, visited = [], {}
    for c in cases():
        m = MemoryMachine(c); result = m.run()
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': result})
    d = header(__file__, __doc__)
    cover = {k: [len([a for a in visited if lo <= a < hi]), body_instructions(lo, hi)] for k, (lo, hi) in RANGES.items()}
    d.update(instructions=instructions(visited), coverage=cover, cases=out)
    p = HERE / 'native-unreachable-memory.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'instructions': len(visited), 'coverage': cover, 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
