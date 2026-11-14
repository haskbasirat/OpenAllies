"""Original small Mind-state helpers under Unicorn, at Single24, Double53
and Extended64, called directly (native_uai_harness.py element Mind):

* 51E1D0(mode, target, flag) and 52D6D0(mode) run unchanged with 529C00,
  529C30 and the Mind activation 52A810 (52A470 / 52A420 / 52A290); the
  state virtuals +4/+18/+1C, Mind virtuals +1A0/+1A4 and the Updatable +8
  scheduler are recorded.
* 51D3F0 (engage status Mind+1D8 and byte Mind+1E8), 51EB80 (+20 byte and
  the +18 GTID list through the registry [B364C0]), 52E1D0 (actor within 3
  of the state's +14/+18, x87) and 51E030 (first word of the +14 array or
  0) run unchanged on scripted words.

Writes native-state-helpers.json next to this script.
"""
from pathlib import Path
import json, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_uai_harness import ELEMENT_STATES, DISCARD, MindMachine, PRECISIONS, f32, body_instructions
from native_machine import header, instructions, sha

RANGES = {'51E1D0': (0x51e1d0, 0x51e215), '52D6D0': (0x52d6d0, 0x52d6ff), '51D3F0': (0x51d3f0, 0x51d403),
          '51EB80': (0x51eb80, 0x51ebce), '52E1D0': (0x52e1d0, 0x52e20b), '51E030': (0x51e030, 0x51e05d)}


class HelperMachine(MindMachine):
    def __init__(self, c):
        super().__init__(c, ELEMENT_STATES, DISCARD)
        self.target = 0
        if c.get('target') is not None:
            self.target = self.obj('target', c['target'])
        if c.get('membership') is not None:
            ms = self.alloc(0x40); self.w(self.actor + 0x80, ms)
            if c['membership']:
                self.w(ms + 0x10, self.alloc(0x40))

    def run(self):
        c = self.c
        e = c['entry']
        if e == '51E1D0':
            s = self.states['engage']
            self.w(s + 0x10, 0x5a5a5a5a); self.wb(s + 0x15, 0x5a)
            r = self.call(0x51e1d0, ecx=s, args=[c['mode'], self.target, c['flag']])
            return dict(result=r & 0xff, events=self.events, target=self.r(s + 0x10), byte15=self.rb(s + 0x15), **self.snap())
        if e == '52D6D0':
            r = self.call(0x52d6d0, ecx=self.states['join'], args=[c['mode']])
            return dict(result=r & 0xff, events=self.events, **self.snap())
        if e == '51D3F0':
            m = self.alloc(0x400); self.w(m + 0x1d8, c['engage_status']); self.wb(m + 0x1e8, c['byte'])
            return {'result': self.call(0x51d3f0, ecx=m) & 0xff}
        if e == '51EB80':
            s = self.alloc(0x40); self.wb(s + 0x20, c['byte20'])
            head = 0
            for g in reversed(c['gtids']):
                n = self.alloc(8); self.w(n, g); self.w(n + 4, head); head = n
            self.w(s + 0x18, head)
            for g in c['registered']:
                self.register(g, self.alloc(0x20))
            return {'result': self.call(0x51eb80, ecx=s) & 0xff}
        if e == '52E1D0':
            s = self.states['move']
            self.w(s + 0x14, c['dest'][0]); self.w(s + 0x18, c['dest'][1])
            self.w(self.actor + 0x20, c['pos'][0]); self.w(self.actor + 0x24, c['pos'][1])
            return {'result': self.call(0x52e1d0, ecx=s) & 0xff}
        s = self.alloc(0x40); self.w(s + 0x10, c['count'])
        arr = self.alloc(0x10); self.w(arr, c['first']); self.w(s + 0x14, arr)
        out = self.alloc(4); self.w(out, 0x77777777)
        self.call(0x51e030, ecx=s, args=[out])
        return {'result': self.r(out)}


def cases():
    rnd = random.Random(0x51e1d0)
    rows = []
    pr = list(PRECISIONS)
    for i in range(240):
        st = {k: rnd.choice([0, 0, 1, 2]) for k in ELEMENT_STATES}
        stack = rnd.choice([[], ['engage'], ['move', 'engage'], ['sleep', 'move'], ['join'], ['deploy', 'kill']])
        base = {'precision': pr[i % 3], 'status': st, 'stack': stack, 'can_activate': rnd.random() < 0.7,
                'mode': rnd.choice([0, 1, 2, 3, 4])}
        rows.append({**base, 'name': 'target%d' % i, 'entry': '51E1D0', 'flag': rnd.choice([0, 1, 0x101, 0xff]),
                     'target': None if rnd.random() < 0.15 else 0x00610610})
        rows.append({**base, 'name': 'member%d' % i, 'entry': '52D6D0',
                     'membership': rnd.choice([None, False, True, True])})
    for i in range(40):
        rows.append({'name': 'engageflag%d' % i, 'precision': 'Single24', 'entry': '51D3F0',
                     'engage_status': rnd.choice([0, 1, 2, 2, 3]), 'byte': rnd.choice([0, 1, 2, 0xff])})
    for i in range(120):
        gt = [(0x70 + k) << 16 | (0x700 + k) for k in range(4)]
        lst = [rnd.choice(gt + [0, 0x00700799]) for _ in range(rnd.randrange(0, 5))]
        rows.append({'name': 'list%d' % i, 'precision': 'Single24', 'entry': '51EB80', 'byte20': rnd.choice([0, 0, 0, 1, 2]),
                     'gtids': lst, 'registered': rnd.sample(gt, rnd.randrange(0, 4))})
    for i in range(360):
        d = [f32(rnd.uniform(-500, 500)), f32(rnd.uniform(-500, 500))]
        dx, dy = (struct.unpack('<f', struct.pack('<I', v))[0] for v in d)
        r = rnd.choice([0.0, 1.0, 2.9999998, 3.0, 3.0000002, 2.12132, 2.1213204, 2.1213202, 4.0, rnd.uniform(0, 6)])
        ang = rnd.choice([(1, 0), (0, 1), (0.70710678, 0.70710678), (0.6, 0.8), (-0.8, 0.6)])
        pos = [f32(dx + r * ang[0]), f32(dy + r * ang[1])]
        rows.append({'name': 'near%d' % i, 'precision': pr[i % 3], 'entry': '52E1D0', 'dest': d, 'pos': pos})
    for i in range(20):
        rows.append({'name': 'first%d' % i, 'precision': 'Single24', 'entry': '51E030',
                     'count': rnd.choice([0, 1, 3]), 'first': rnd.getrandbits(32)})
    return rows


def main():
    out, visited = [], {}
    for c in cases():
        try:
            m = HelperMachine(c); result = m.run()
        except Exception:
            print(c['name']); raise
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': result})
    d = header(__file__, __doc__)
    cover = {k: [len([a for a in visited if lo <= a < hi]), body_instructions(lo, hi)] for k, (lo, hi) in RANGES.items()}
    d.update(instructions=instructions(visited), coverage=cover, cases=out)
    p = HERE / 'native-state-helpers.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'instructions': len(visited), 'coverage': cover, 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
