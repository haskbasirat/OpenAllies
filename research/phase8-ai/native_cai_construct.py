"""Original KKC_CAIStateConstruct::Think 51DB80 (regiment state cai_construct
at Mind+134, vtable A32A38 slot +C) executing unchanged on an emulated
regiment Mind (native_cai_regiment.py) with 529E60 and 529C30.

Seams (recorded in order with the tracked statuses and the Mind stack): Mind
virtuals +170(value) +174, state virtual +C (the move state Mind+68 Think,
reached by a tail jump), 4F2420(entry, x, y) on the map singleton [B36534],
57ACC0(word14, &state+18, &state+20, word28) on the organization component
(the event records the four words behind the two pointers at call time);
allocator, free and loggers as in native_cai_regiment.py. The organization's
+148 array and +14C count are emulated data read natively.
"""
from pathlib import Path
import json, random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import f32, header, instructions, sha
from native_cai_regiment import RegimentMachine, coverage

TRACKED = ['construct', 'move', 'sleep', 'other']
MAP = 0x0bad0000


class ConstructMachine(RegimentMachine):
    def __init__(self, c):
        super().__init__(c, mind_virtuals=[(0x170, 'mind_virtual_170', 1), (0x174, 'mind_virtual_174', 0)])
        s = self.states['construct']
        self.wb(s + 0x10, c['byte10']); self.w(s + 0x14, c['word14'])
        for i, v in enumerate(c['points']):
            self.w(s + 0x18 + 4 * i, v)
        self.w(s + 0x28, c['word28'])
        self.w(self.org + 0x14c, c['count'])
        self.w(self.org + 0x148, self.array(c['entries']))
        self.w(0xb36534, MAP)
        self.seam(0x4f2420, self.s_map)
        self.seam(0x57acc0, self.s_construct)

    def s_map(self, m):
        assert self.ecx() == MAP
        r = self.c['map_result']
        self.ev('map_4f2420', entry=self.arg(1), point=[self.arg(2), self.arg(3)], result=r)
        self.ret(int(r), 12)

    def s_construct(self, m):
        assert self.ecx() == self.comp
        s = self.states['construct']
        assert self.arg(2) == s + 0x18 and self.arg(3) == s + 0x20
        r = self.c['construct_result']
        self.ev('component_57acc0', word14=self.arg(1), points=[self.r(s + 0x18 + 4 * i) for i in range(4)],
                word28=self.arg(4), result=r)
        self.ret(r, 16)

    def run(self):
        fault = self.run_think(0x51db80, 'construct')
        if fault:
            return fault
        return {'events': self.events, **self.snap(), 'byte10': self.rb(self.states['construct'] + 0x10)}


def cases():
    rnd = random.Random(0x51db80)
    stacks = [['construct'], ['construct', 'sleep'], ['other', 'construct'], [], ['construct', None]]
    rows = []
    for i in range(200):
        c = {'name': 'construct%d' % i, 'precision': 'Double53',
             'byte10': rnd.choice([0, 1, 0xff]), 'word14': rnd.getrandbits(32), 'word28': rnd.getrandbits(32),
             'points': [f32(rnd.uniform(-500, 500)) for _ in range(4)],
             'count': rnd.choice([0, 1, 4, 5, 6, 9, 0xffffffff, 0x80000000, 0x7fffffff]),
             'entries': [rnd.getrandbits(32) if rnd.random() < 0.8 else 0 for _ in range(6)],
             'map_result': rnd.random() < 0.6, 'construct_result': rnd.choice([0, rnd.getrandbits(32)]),
             'states': {'construct': rnd.choice([0, 1, 2, 2]), 'move': rnd.choice([0, 2]),
                        'sleep': rnd.choice([0, 1]), 'other': rnd.choice([1, 2])},
             'stack': rnd.choice(stacks), 'words': [rnd.getrandbits(32) for _ in range(4)],
             'can_activate': {}, 'tracked': TRACKED}
        rows.append(c)
    return rows


def main():
    out, visited = [], {}
    for c in cases():
        m = ConstructMachine(c)
        result = m.run()
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': result})
    cov = coverage(visited, {'think_51db80': (0x51db80, 0x51dc18)})
    d = header(__file__, __doc__)
    d.update(instructions=instructions(visited), coverage=cov, cases=out)
    p = HERE / 'native-cai-construct.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'faults': sum('fault' in r['output'] for r in out),
                      'instructions': len(visited),
                      'coverage': {k: [v['visited'], v['instructions'], v['unvisited']] for k, v in cov.items()},
                      'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
