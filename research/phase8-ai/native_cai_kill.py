"""Original KKC_CAIStateKill::Think 520730 (regiment state cai_kill at Mind+C0,
vtable A32B5C slot +C) executing unchanged on an emulated regiment Mind
(native_cai_regiment.py) together with 529E60, 529C30, 52A000, the siege
state (Mind+F8) activation 51DA20 with its target check 51D130 and 529C00,
and the Mind activation 52A810 mode 1 (52A470 replace top, 52A290 push).

Seams (recorded in order, each with the tracked state statuses and the Mind
stack): Mind virtuals +140 +1A0 +1A4, Updatable +8 virtual +4 (schedule),
state virtuals +4 +C +18 +1C, organization virtual +16C, 4D11D0 on the target
object (its site object or null), 57A500 on the organization component
(results consumed from the case's `accepts` queue); allocator, free and
loggers as in native_cai_regiment.py.
"""
from pathlib import Path
import json, random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import header, instructions, sha
from native_cai_regiment import RegimentMachine, coverage

TRACKED = ['kill', 'siege', 'move', 'sleep', 'other']


class KillMachine(RegimentMachine):
    def __init__(self, c):
        super().__init__(c, mind_virtuals=[(0x140, 'mind_virtual_140', 1)],
                         org_virtuals=[(0x16c, self.o_16c)])
        self.accepts = list(c['accepts'])
        for key, o in c['objects'].items():
            p = self.add_object(key, o['gtid'], o['registered'])
            self.w(p + 0x90, o['word90'])
        self.w(self.states['kill'] + 0x10, c['target'])
        self.w(self.states['siege'] + 0x10, c['siege_words'][0])
        self.w(self.states['siege'] + 0x14, c['siege_words'][1])
        self.seam(0x57a500, self.s_57a500)
        self.seam(0x4d11d0, self.s_4d11d0)

    def o_16c(self, m):
        assert self.ecx() == self.org
        site = self.name(self.arg(1)); r = self.c['org_16c']
        self.ev('organization_virtual_16c', site=site, result=r); self.ret(int(r), 4)

    def s_57a500(self, m):
        assert self.ecx() == self.comp
        r = self.accepts.pop(0)
        self.ev('component_57a500', object=self.name(self.arg(1)), flags=[self.arg(2), self.arg(3)], result=r)
        self.ret(int(r), 12)

    def s_4d11d0(self, m):
        k = self.name(self.ecx()); site = self.c['objects'][k]['site']
        self.ev('object_4d11d0', object=k, result=site or 0)
        self.ret(0 if site is None else self.objects[site])

    def run(self):
        fault = self.run_think(0x520730, 'kill')
        if fault:
            return fault
        return {'events': self.events, **self.snap(),
                'siege_words': [self.r(self.states['siege'] + 0x10), self.r(self.states['siege'] + 0x14)],
                'kill_target': self.r(self.states['kill'] + 0x10)}


def cases():
    rnd = random.Random(0x520730)
    stacks = [['kill'], ['kill'], ['kill', 'sleep'], ['kill', 'other'], ['kill', 'sleep', 'other'],
              ['kill', None], ['kill', None, 'other'], ['other', 'kill'], ['sleep', 'kill'], [], ['siege', 'kill'],
              ['kill', 'siege'], ['kill', 'move']]
    rows = []
    for i in range(400):
        objects = {}
        for k in range(rnd.randrange(1, 4)):
            objects['o%d' % k] = {'gtid': (0x40 + k) << 16 | (0x400 + k), 'registered': rnd.random() < 0.85,
                                  'word90': rnd.choice([0, 0, 0, 1, 0x12345]), 'site': None}
        for k in range(2):
            objects['s%d' % k] = {'gtid': (0x50 + k) << 16 | (0x500 + k), 'registered': rnd.random() < 0.5,
                                  'word90': 0, 'site': None}
        for k in list(objects):
            if k.startswith('o') and rnd.random() < 0.75:
                objects[k]['site'] = rnd.choice(['s0', 's1'])
        targets = [o['gtid'] for k, o in objects.items() if k.startswith('o')]
        target = rnd.choice(targets + targets + [0, 0x7fff0009, (0x41 << 16) | 0x400])
        c = {'name': 'kill%d' % i, 'precision': 'Double53', 'objects': objects, 'target': target,
             'accepts': [rnd.random() < 0.7, rnd.random() < 0.7], 'org_16c': rnd.random() < 0.7,
             'siege_words': [rnd.getrandbits(32), rnd.getrandbits(32)],
             'states': {'kill': rnd.choice([0, 1, 2, 2]), 'siege': rnd.choice([0, 1, 2]),
                        'move': rnd.choice([0, 2]), 'sleep': rnd.choice([0, 1, 2]), 'other': rnd.choice([1, 2])},
             'stack': rnd.choice(stacks), 'words': [rnd.getrandbits(32) for _ in range(4)],
             'can_activate': {}, 'tracked': TRACKED}
        rows.append(c)
    return rows


def main():
    out, visited = [], {}
    for c in cases():
        m = KillMachine(c)
        result = m.run()
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': result})
    cov = coverage(visited, {'think_520730': (0x520730, 0x5207bb), 'siege_activate_51da20': (0x51da20, 0x51da88),
                             'siege_target_51d130': (0x51d130, 0x51d197), 'can_activate_529c00': (0x529c00, 0x529c1e)})
    d = header(__file__, __doc__)
    d.update(instructions=instructions(visited), coverage=cov, cases=out)
    p = HERE / 'native-cai-kill.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'faults': sum('fault' in r['output'] for r in out),
                      'instructions': len(visited),
                      'coverage': {k: [v['visited'], v['instructions'], v['unvisited']] for k, v in cov.items()},
                      'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
