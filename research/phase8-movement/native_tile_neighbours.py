"""Original 4F14A0(direction) (neighbour tile record address of a tile
cursor) under Unicorn on the phase8-ai machine
(research/phase8-ai/native_machine.py), called directly for every
direction 0..8 on cursors with scripted x, y, grid width and record base.
No seams. The [B322E0] (dx, dy) table is read from the mapped image and
written to the fixture.

Writes native-tile-neighbours.json next to this script.
"""
from pathlib import Path
import json, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / 'phase8-ai'))
from native_machine import Machine, header, instructions, sha  # noqa: E402
from native_uai_harness import body_instructions  # noqa: E402

ENTRY, END = 0x4f14a0, 0x4f14d6


class TileMachine(Machine):
    def run_case(self, c):
        cur = self.alloc(0x40); grid = self.alloc(0x40)
        self.w(cur, c['x'] & 0xffffffff); self.w(cur + 4, c['y'] & 0xffffffff); self.w(cur + 0x18, grid)
        self.w(grid + 0x1c, c['width'] & 0xffffffff); self.w(grid + 0x28, c['base'])
        return [self.call(ENTRY, ecx=cur, args=[d]) for d in range(9)]


def cases():
    rnd = random.Random(0x4f14a0)
    rows = []
    for i in range(400):
        rows.append({'name': 'tile%d' % i, 'x': rnd.choice([0, 1, 255, 511, -1, rnd.randrange(0, 1024)]),
                     'y': rnd.choice([0, 1, 255, 511, -1, rnd.randrange(0, 1024)]),
                     'width': rnd.choice([64, 256, 512, 1024, rnd.randrange(1, 4096)]),
                     'base': rnd.choice([0x20000000, 0x10000000, rnd.getrandbits(32)])})
    return rows


def main():
    m = TileMachine('Single24')
    table = [list(struct.unpack('<ii', bytes(m.u.mem_read(0xb322e0 + 8 * d, 8)))) for d in range(9)]
    out = []
    for c in cases():
        out.append({'input': c, 'output': m.run_case(c)})
    m.verify_v14()
    d = header(__file__, __doc__)
    cover = {'4F14A0': [len([a for a in m.visited if ENTRY <= a < END]), body_instructions(ENTRY, END)]}
    d.update(instructions=instructions(m.visited), coverage=cover, table=table, cases=out)
    p = HERE / 'native-tile-neighbours.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'coverage': cover, 'table': table, 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
