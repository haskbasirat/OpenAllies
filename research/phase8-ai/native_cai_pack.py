"""Original KKC_CAIStatePack::Think 527090 (regiment state cai_pack at
Mind+2A4, vtable A32B34 slot +C) executing unchanged on an emulated regiment
Mind (native_cai_regiment.py) with 529E60, 529C30 and the stack action
constructor 4D9E20 (native, action registry [B36530]+298).

Emulated data read natively: organization component +44 (the unit) and the
unit's +100 flags.

Seams (recorded in order with the tracked statuses and the Mind stack): Mind
virtual +1B4, Updatable +8 virtual +4 (schedule), state virtual +C (sleep
Think), 4D1080 on the unit, 4D9B60(unit) on the stack action (records its
fields), 4CA2D0 (action release), and the player error message chain of UnitOrderMixin
(organization virtuals +24/+108, 401B00, 4B9F40, 401560, 40C230); allocator,
free and loggers as in native_cai_regiment.py.
"""
from pathlib import Path
import json, random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import header, instructions, sha
from native_cai_regiment import RegimentMachine, UnitOrderMixin, coverage

TRACKED = ['pack', 'sleep', 'move', 'other']


class PackMachine(UnitOrderMixin, RegimentMachine):
    def __init__(self, c):
        super().__init__(c, mind_virtuals=[(0x1b4, 'mind_virtual_1b4', 0)],
                         org_virtuals=self.org_unit_virtuals())
        self.install_unit_orders(c['definitions'], c['issue'], c['owner'])
        if c['unit'] is not None:
            u = self.add_object('unit', 0x00700700, True)
            self.w(u + 0x100, c['unit']['word100']); self.w(self.comp + 0x44, u)
        self.seam(0x4d1080, self.s_4d1080)

    def s_4d1080(self, m):
        k = self.name(self.ecx()); r = self.c['unit']['check_4d1080']
        self.ev('unit_4d1080', unit=k, result=r); self.ret(int(r))

    def run(self):
        fault = self.run_think(0x527090, 'pack')
        if fault:
            return fault
        return {'events': self.events, **self.snap()}


def cases():
    rnd = random.Random(0x527090)
    stacks = [['pack'], ['pack'], ['pack', 'sleep'], ['other', 'pack'], [], ['pack', None]]
    rows = []
    for i in range(160):
        unit = None if rnd.random() < 0.1 else {
            'word100': rnd.choice([0, 0x2000000, 0x4000000, 0x2000001, rnd.getrandbits(32)]),
            'check_4d1080': rnd.random() < 0.7}
        c = {'name': 'pack%d' % i, 'precision': 'Double53', 'unit': unit,
             'definitions': {'12': rnd.choice([0, 0x7ab0000])}, 'issue': [rnd.random() < 0.5],
             'owner': rnd.choice([0, 1, 3, 7]),
             'states': {'pack': rnd.choice([0, 1, 2, 2]), 'sleep': rnd.choice([0, 1, 2]),
                        'move': rnd.choice([0, 2]), 'other': rnd.choice([1, 2])},
             'stack': rnd.choice(stacks), 'words': [rnd.getrandbits(32) for _ in range(4)],
             'can_activate': {}, 'tracked': TRACKED}
        rows.append(c)
    return rows


def main():
    out, visited = [], {}
    for c in cases():
        m = PackMachine(c)
        result = m.run()
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': result})
    cov = coverage(visited, {'think_527090': (0x527090, 0x527232)})
    d = header(__file__, __doc__)
    d.update(instructions=instructions(visited), coverage=cov, cases=out)
    p = HERE / 'native-cai-pack.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'faults': sum('fault' in r['output'] for r in out),
                      'instructions': len(visited),
                      'coverage': {k: [v['visited'], v['instructions'], v['unvisited']] for k, v in cov.items()},
                      'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
