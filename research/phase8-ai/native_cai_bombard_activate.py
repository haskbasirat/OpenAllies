"""Original KKC_CAIStateBombard::Activate 527B00 (regiment state cai_bombard
at Mind+D4) executing unchanged on an emulated regiment Mind
(native_cai_regiment.py) at Single24, Double53 and Extended64, with 529C30,
52A000 (current state), 4D1080 (organization +104 bit 2) and the Mind
activation 52A810 (52A470 / 52A420 / 52A290 stack operations).

Seams (recorded in order): 57A5E0 on the organization component (scripted
answer), 4D4FF0(target, 3, flag) on the organization (scripted relation;
the flag is a bool whose stack slot has only its low byte written),
527A60(target) on the bombard state (target setter: GTID and name CString),
the state virtuals +4/+18/+1C, Mind virtuals +1A0/+1A4 and the Updatable +8
virtual +4 scheduler; loggers 404F60 as in native_cai_regiment.py.
"""
from pathlib import Path
import json, random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import PRECISIONS, f32, header, instructions, sha
from native_cai_regiment import RegimentMachine, STATES
from native_uai_harness import body_instructions

ENTRY, END = 0x527b00, 0x527c18
TRACKED = ['bombard', 'move', 'sleep', 'engage', 'other']


class ActivateMachine(RegimentMachine):
    def __init__(self, c):
        super().__init__(c)
        t = c['target']
        self.target = 0
        if t is not None:
            self.target = self.add_object('target', t['gtid'], True)
            self.w(self.target + 0x20, t['pos'][0]); self.w(self.target + 0x24, t['pos'][1])
        self.w(self.org + 0x104, c['org_word104'])
        self.w(self.mind + 0xc, c['mind_word_c'])
        b = self.states['bombard']
        self.w(b + 0x10, c['bombard_target']); self.w(b + 0x18, 0x11111111); self.w(b + 0x1c, 0x22222222)
        self.seam(0x57a5e0, self.s_component)
        self.seam(0x4d4ff0, self.s_relation)
        self.seam(0x527a60, self.s_set_target)

    def ev(self, name, **kw):
        self.events.append({'event': name, **kw})

    def s_component(self, m):
        assert self.ecx() == self.comp
        r = self.c['component']; self.ev('component_57a5e0', result=r); self.ret(int(r))

    def s_relation(self, m):
        assert self.ecx() == self.org
        r = self.c['relation']
        self.ev('relation', target=self.name(self.arg(1)), a=self.arg(2), b=self.arg(3) & 0xff, result=r)
        self.ret(r & 0xffffffff, 12)

    def s_set_target(self, m):
        assert self.ecx() == self.states['bombard']
        self.ev('set_target', target=self.name(self.arg(1))); self.ret(0, 4)

    def run(self):
        r = self.call(ENTRY, ecx=self.states['bombard'], args=[self.c['mode'], self.target])
        b = self.states['bombard']
        return {'result': r & 0xff, 'events': self.events, 'point': [self.r(b + 0x18), self.r(b + 0x1c)],
                'bombard_target': self.r(b + 0x10), **self.snap()}


def cases():
    rnd = random.Random(0x527b00)
    rows = []
    precisions = list(PRECISIONS)
    for i in range(600):
        gtid = 0x00530530
        t = None if rnd.random() < 0.1 else {'gtid': gtid, 'pos': [f32(rnd.uniform(-90, 90)), f32(rnd.uniform(-90, 90))]}
        states = {k: rnd.choice([0, 0, 1, 2]) for k in TRACKED}
        rows.append({'name': 'activate%d' % i, 'precision': precisions[i % 3], 'target': t, 'states': states,
                     'tracked': TRACKED, 'can_activate': {k: rnd.random() < 0.7 for k in STATES},
                     'stack': rnd.choice([[], ['bombard'], ['bombard', 'move'], ['move'], ['sleep', 'bombard'], ['engage'], [None]]),
                     'words': [0, 0, 0, 0], 'mode': rnd.choice([0, 1, 2, 3, 3, 4]),
                     'org_word104': rnd.choice([0, 4, 0xfffffffb, 0xffffffff, 6]),
                     'mind_word_c': rnd.choice([0, 0, 1, 0x100]), 'component': rnd.random() < 0.85,
                     'relation': rnd.choice([0, 0, 5, 5, 1, 2, 3, -1]),
                     'bombard_target': rnd.choice([gtid, gtid, 0, 0x00540540])})
    return rows


def main():
    out, visited = [], {}
    for c in cases():
        try:
            m = ActivateMachine(c); result = m.run()
        except Exception:
            print(c['name']); raise
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': result})
    d = header(__file__, __doc__)
    cover = {'527B00': [len([a for a in visited if ENTRY <= a < END]), body_instructions(ENTRY, END)]}
    d.update(instructions=instructions(visited), coverage=cover, cases=out)
    p = HERE / 'native-cai-bombard-activate.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'instructions': len(visited), 'coverage': cover, 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
