"""Original KKC_CAIStateUnpack::Think 526E60 (regiment state cai_unpack at
Mind+280, vtable A32B10 slot +C) executing unchanged on an emulated regiment
Mind (native_cai_regiment.py) with 529E60, 529C30, the Mind activation 52A810
mode 0, the Tai unpack action constructor 51E800 and the stack action
constructor 4D9E20 (native, action registry [B36530]+298).

Emulated data read natively: organization component +44 (the unit), the
unit's +70 Mind, state+10..+1C (two binary32 points) and state+20 (signed
attempt count).

Seams (recorded in order with the tracked statuses and the Mind stack): Mind
virtual +1B0(flag), Updatable +8 virtual +4 (schedule), state virtuals +C +18,
unit Mind virtual +7C, 4D1050 and 4D1080 on the unit, 4D9B60(unit) on the
stack action (records its fields), 4CA2D0 (action release) and the player error message chain of
UnitOrderMixin; allocator, free and loggers as in native_cai_regiment.py. A
null unit Mind is a native null dereference (fault case). Signalling NaN
point words are avoided (the emulator does not quiet them on fld).
"""
from pathlib import Path
import json, random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import f32, header, instructions, sha
from native_cai_regiment import RegimentMachine, UnitOrderMixin, coverage

TRACKED = ['unpack', 'sleep', 'move', 'other']


class UnpackMachine(UnitOrderMixin, RegimentMachine):
    def __init__(self, c):
        super().__init__(c, mind_virtuals=[(0x1b0, 'mind_virtual_1b0', 1)],
                         org_virtuals=self.org_unit_virtuals())
        self.install_unit_orders(c['definitions'], c['issue'], c['owner'])
        s = self.states['unpack']
        for i, v in enumerate(c['points']):
            self.w(s + 0x10 + 4 * i, v)
        self.w(s + 0x20, c['attempts'])
        unit = c['unit']
        if unit is not None:
            u = self.add_object('unit', 0x00700700, True)
            self.w(self.comp + 0x44, u)
            if unit['mind']:
                umvt = self.alloc(0x100)
                for slot in range(0, 0x100, 4):
                    self.w(umvt + slot, self.stub(self.unexpected('unit mind', slot)))
                self.w(umvt + 0x7c, self.stub(self.um_7c))
                um = self.alloc(0x40); self.w(um, umvt); self.w(u + 0x70, um)
        self.seam(0x4d1050, self.s_pred('unit_4d1050', 'check_4d1050'))
        self.seam(0x4d1080, self.s_pred('unit_4d1080', 'check_4d1080'))

    def um_7c(self, m):
        r = self.c['unit']['mind_7c']
        self.ev('unit_mind_virtual_7c', result=r); self.ret(int(r))

    def s_pred(self, event, key):
        def fn(m):
            k = self.name(self.ecx()); r = self.c['unit'][key]
            self.ev(event, unit=k, result=r); self.ret(int(r))
        return fn

    def run(self):
        fault = self.run_think(0x526e60, 'unpack')
        if fault:
            return fault
        return {'events': self.events, **self.snap(), 'attempts': self.r(self.states['unpack'] + 0x20)}


def cases():
    rnd = random.Random(0x526e60)
    stacks = [['unpack'], ['unpack'], ['unpack', 'sleep'], ['other', 'unpack'], [], ['unpack', None]]
    rows = []
    for i in range(240):
        unit = None if rnd.random() < 0.08 else {
            'mind': rnd.random() < 0.97, 'mind_7c': rnd.random() < 0.25, 'check_4d1050': rnd.random() < 0.25,
            'check_4d1080': rnd.random() < 0.3}
        c = {'name': 'unpack%d' % i, 'precision': 'Double53', 'unit': unit,
             'points': [f32(rnd.uniform(-300, 300)) for _ in range(4)],
             'attempts': rnd.choice([0, 1, 1, 2, 3, 0xffffffff, 0x80000000, 0x7fffffff]),
             'definitions': {'11': rnd.choice([0, 0x7ab0000])}, 'issue': [rnd.random() < 0.6],
             'owner': rnd.choice([0, 1, 3, 7]),
             'states': {'unpack': rnd.choice([0, 1, 2, 2]), 'sleep': rnd.choice([0, 1, 2]),
                        'move': rnd.choice([0, 0, 2]), 'other': rnd.choice([1, 2])},
             'stack': rnd.choice(stacks), 'words': [rnd.getrandbits(32) for _ in range(4)],
             'can_activate': {}, 'tracked': TRACKED}
        rows.append(c)
    return rows


def main():
    out, visited = [], {}
    for c in cases():
        m = UnpackMachine(c)
        result = m.run()
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': result})
    cov = coverage(visited, {'think_526e60': (0x526e60, 0x527083), 'action_51e800': (0x51e800, 0x51e84a)})
    d = header(__file__, __doc__)
    d.update(instructions=instructions(visited), coverage=cov, cases=out)
    p = HERE / 'native-cai-unpack.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'faults': sum('fault' in r['output'] for r in out),
                      'instructions': len(visited),
                      'coverage': {k: [v['visited'], v['instructions'], v['unvisited']] for k, v in cov.items()},
                      'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
