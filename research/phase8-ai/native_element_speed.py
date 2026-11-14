"""Original 52E610(speed) on the element move state under Unicorn, called
directly on the phase8-ai machine (native_machine.py).

Runs unchanged with 529C30 and 4CE6C0 (group holder: the actor, or the
organization of a member whose +7C is null) and the registry [B364C0].
Seams (recorded in order): 52D660 (stop, on the move state) and 576D20 on
the actor +6C locomotion (both arguments). The SVar [B39AB0]+190 is a case
input.

Writes native-element-speed.json next to this script.
"""
from pathlib import Path
import json, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import Machine, f32, header, instructions, sha
from native_uai_harness import body_instructions

ENTRY, END = 0x52e610, 0x52e69e
HOLDER, HOLDER_END = 0x4ce6c0, 0x4ce6dd


class SpeedMachine(Machine):
    def __init__(self, c):
        super().__init__('Single24')
        self.c = c
        reg = self.reg = self.alloc(0x20004 + 0x10000 * 4 + 16); self.w(0xb364c0, reg)
        sv = self.alloc(0x200); s = self.alloc(4); self.w(s, c['svar']); self.w(sv + 0x190, s); self.w(0xb39ab0, sv)
        self.state = self.alloc(0x40); mind = self.alloc(0x40); a = self.actor = self.alloc(0x200)
        self.w(self.state + 4, mind); self.w(mind + 4, a)
        self.loco = self.alloc(0x40); self.w(a + 0x6c, self.loco)
        if c['has_7c']:
            self.w(a + 0x7c, self.alloc(0x10))
        self.org = 0
        if c['membership'] is not None:
            ms = self.alloc(0x40); self.w(a + 0x80, ms)
            if c['membership']:
                self.org = self.alloc(0x200); self.w(ms + 0x10, self.org)
                self.w(self.org + 0x164, c['org_group'])
        self.w(a + 0x164, c['actor_group'])
        for g in c['registered']:
            idx, gen = g & 0xffff, g >> 16
            self.u.mem_write(reg + 4 + 2 * idx, struct.pack('<H', gen)); self.w(reg + 0x20004 + 4 * idx, self.alloc(0x10))
        self.seam(0x52d660, self.s_stop)
        self.seam(0x576d20, self.s_speed)

    def s_stop(self, m):
        assert self.ecx() == self.state
        self.events.append({'event': 'stop'}); self.ret(0)

    def s_speed(self, m):
        assert self.ecx() == self.loco
        self.events.append({'event': 'locomotion_set_speed', 'speed': self.arg(1), 'factor': self.arg(2)}); self.ret(0, 8)

    def run(self):
        self.w(self.state + 0x24, 0x5a5a5a5a)
        self.call(ENTRY, ecx=self.state, args=[self.c['speed']])
        return {'events': self.events, 'requested': self.r(self.state + 0x24)}


def cases():
    rnd = random.Random(0x52e610)
    rows = []
    groups = [0x00810810, 0x00820820]
    for i in range(600):
        rows.append({'name': 'speed%d' % i, 'speed': rnd.choice([0, f32(0.1), 0x3dcccccc, 0x3dccccce, f32(0.05), f32(1.0), f32(2.5),
                                                                f32(3.0), f32(3.0000002), 0x80000000, f32(-1.0), 0x7fc00000]),
                     'svar': rnd.choice([f32(3.0), f32(0.5), f32(0.0), 0x7fc00000]),
                     'has_7c': rnd.random() < 0.4, 'membership': rnd.choice([None, False, True, True]),
                     'org_group': rnd.choice(groups + [0]), 'actor_group': rnd.choice(groups + [0, 0x00830830]),
                     'registered': rnd.sample(groups, rnd.randrange(0, 3))})
    return rows


def main():
    out, visited = [], {}
    for c in cases():
        m = SpeedMachine(c); result = m.run()
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': result})
    d = header(__file__, __doc__)
    cover = {'52E610': [len([a for a in visited if ENTRY <= a < END]), body_instructions(ENTRY, END)],
             '4CE6C0': [len([a for a in visited if HOLDER <= a < HOLDER_END]), body_instructions(HOLDER, HOLDER_END)]}
    d.update(instructions=instructions(visited), coverage=cover, cases=out)
    p = HERE / 'native-element-speed.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'instructions': len(visited), 'coverage': cover, 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
