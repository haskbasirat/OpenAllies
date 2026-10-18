"""Original KKC_CAIStateRampage::Think 5267E0 (regiment state cai_rampage at
Mind+54, vtable A32A14 slot +C) executing unchanged on an emulated regiment
Mind (native_cai_regiment.py) at x87 precision Single24, Double53 and
Extended64, together with 529E60, 529C30, 529C20, the Tai helpers 51D6E0
(go to sleep), 51E910 (attack a target), 526740 (wander), the Mind activation
52A810 mode 0 and the world RNG draw 4F02B0/45D400/85D840.

Emulated data read natively: the organization definition +314 rampage
config (+0 binary32 wander chance, +18 binary32 rampage threshold, +1C hunt
byte), organization component +5C/+60 (binary32), +28/+2C member array and
count, member +80 (+1C byte) and +70 Mind, kill target Mind+D0, the world
singleton [B36534] (+D4 RNG word) and the null point [B279C0]/[B279C4].

Seams (recorded in order with the tracked statuses and the Mind stack): Mind
virtual +B8, member Mind virtual +FC, state virtuals +C +18 +1C,
578790 (component, ST0 return given as binary64), 577F20 (component, binary32
ST0 return), 51E450(&organization+20, radius) on the Mind (nearest target,
the event records the position words), 51D920(mode, target) on the kill
state, 525100(&out) on the rampage state (writes the wander point), 520440
(6 words) on the move state; 77DA70 (RNG trace, not recorded); allocator,
free and loggers as in native_cai_regiment.py. Some cases model a sleep
Think callback that leaves the kill or move state active (think_effects), so
the helpers' status rereads are exercised. A null member Mind (+70) is a
native null dereference (fault case).
"""
from pathlib import Path
import json, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import PRECISIONS, f32, header, instructions, sha
from native_cai_regiment import RegimentMachine, coverage

TRACKED = ['rampage', 'sleep', 'move', 'kill', 'other']
f64 = lambda v: struct.unpack('<Q', struct.pack('<d', v))[0]


class RampageMachine(RegimentMachine):
    def __init__(self, c):
        super().__init__(c, mind_virtuals=[(0xb8, 'mind_virtual_b8', 0)])
        s = self.states['rampage']
        self.wb(s + 0x10, c['byte10'])
        cfg = self.alloc(0x40); self.w(self.org_defn + 0x314, cfg)
        self.w(cfg, c['wander_chance']); self.w(cfg + 0x18, c['threshold']); self.wb(cfg + 0x1c, c['hunts'])
        self.w(self.comp + 0x5c, c['comp5c']); self.w(self.comp + 0x60, c['comp60'])
        self.w(self.org + 0x20, c['org_position'][0]); self.w(self.org + 0x24, c['org_position'][1])
        mmvt = self.alloc(0x200)
        for slot in range(0, 0x200, 4):
            self.w(mmvt + slot, self.stub(self.unexpected('member mind', slot)))
        self.w(mmvt + 0xfc, self.stub(self.mm_fc))
        self.member_minds = {}
        words = []
        for i, mem in enumerate(c['members']):
            if mem is None:
                words.append(0); continue
            p = self.alloc(0x100)
            if mem['c80'] is not None:
                c80 = self.alloc(0x40); self.wb(c80 + 0x1c, mem['c80']); self.w(p + 0x80, c80)
            if mem['mind']:
                mm = self.alloc(0x40); self.w(mm, mmvt); self.w(p + 0x70, mm); self.member_minds[mm] = i
            words.append(p)
        self.w(self.comp + 0x28, self.array(words)); self.w(self.comp + 0x2c, c['member_count'])
        for key, o in c['objects'].items():
            self.add_object(key, o['gtid'], o['registered'])
        self.w(self.states['kill'] + 0x10, c['kill_target'])
        world = self.alloc(0x100); self.w(0xb36534, world); self.w(world + 0xd4, c['rng'])
        self.world = world
        self.w(0xb3655c, self.alloc(0x40)); self.w(0xb36560, self.alloc(0x40))
        self.w(0xb279c0, c['null_point'][0]); self.w(0xb279c4, c['null_point'][1])
        self.seam(0x77da70, lambda m: self.ret(0, 12))
        self.seam(0x578790, self.s_578790)
        self.seam(0x577f20, self.s_577f20)
        self.seam(0x51e450, self.s_nearest)
        self.seam(0x51d920, self.s_kill_activate)
        self.seam(0x525100, self.s_wander_point)
        self.seam(0x520440, self.s_move_order)

    def mm_fc(self, m):
        self.ev('member_mind_virtual_fc', member=self.member_minds[self.ecx()]); self.ret(0)

    def s_578790(self, m):
        assert self.ecx() == self.comp
        r = self.c['fraction']
        self.ev('component_578790', result=r); self.ret_st0_f64(r)

    def s_577f20(self, m):
        assert self.ecx() == self.comp
        r = self.c['radius']
        self.ev('component_577f20', result=r); self.ret_st0_f32(r)

    def s_nearest(self, m):
        assert self.ecx() == self.mind and self.arg(1) == self.org + 0x20
        r = self.c['nearest']
        self.ev('nearest_target', position=[self.r(self.org + 0x20), self.r(self.org + 0x24)], radius=self.arg(2),
                result=r or 0)
        self.ret(0 if r is None else self.objects[r], 8)

    def s_kill_activate(self, m):
        assert self.ecx() == self.states['kill']
        r = self.c['kill_activate']
        self.ev('kill_activate', mode=self.arg(1), target=self.name(self.arg(2)), result=r); self.ret(int(r), 8)

    def s_wander_point(self, m):
        assert self.ecx() == self.states['rampage']
        out = self.arg(1); pt = self.c['wander_point']
        self.w(out, pt[0]); self.w(out + 4, pt[1])
        self.ev('wander_point', result=pt); self.ret(out, 4)

    def s_move_order(self, m):
        assert self.ecx() == self.states['move']
        self.ev('move_order', args=[self.arg(i) for i in range(1, 7)]); self.ret(0, 0x18)

    def run(self):
        fault = self.run_think(0x5267e0, 'rampage')
        if fault:
            return fault
        return {'events': self.events, **self.snap(), 'byte10': self.rb(self.states['rampage'] + 0x10),
                'rng': self.r(self.world + 0xd4)}


def cases():
    rnd = random.Random(0x5267e0)
    precisions = list(PRECISIONS)
    stacks = [['rampage'], ['rampage'], ['rampage', 'sleep'], ['sleep', 'rampage'], ['other', 'rampage'], [],
              ['rampage', None], ['kill', 'rampage'], ['move']]
    rows = []
    for i in range(600):
        objects = {}
        for k in range(2):
            objects['t%d' % k] = {'gtid': (0x60 + k) << 16 | (0x600 + k), 'registered': rnd.random() < 0.8}
        members = []
        for k in range(rnd.randrange(0, 4)):
            if rnd.random() < 0.15:
                members.append(None); continue
            members.append({'c80': rnd.choice([None, 0, 1]), 'mind': rnd.random() < 0.97})
        frac = rnd.choice([0.0, 1.0, 0.5, 1 / 3, 2 / 3, 0.25, 0.9999, 0.99995, 0.9999000072479248,
                           rnd.random(), 1.0 - 2.0 ** -40])
        c = {'name': 'rampage%d' % i, 'precision': precisions[i % 3],
             'byte10': rnd.choice([0, 0, 1, 0x80]),
             'wander_chance': f32(rnd.choice([0.0, 0.25, 0.5, 0.9, 1.0, rnd.random()])),
             'threshold': f32(rnd.choice([0.0, 0.25, 0.5, 2 / 3, 0.75, rnd.random(), -1.0])),
             'hunts': rnd.choice([0, 1, 1]),
             'comp5c': f32(rnd.choice([0.0, 1.0, 5.0, rnd.uniform(0, 10)])),
             'comp60': f32(rnd.choice([0.0, 1.0, 5.0, rnd.uniform(0, 10)])),
             'org_position': [f32(rnd.uniform(-100, 100)), f32(rnd.uniform(-100, 100))],
             'members': members, 'member_count': len(members) if rnd.random() < 0.9 else rnd.choice([0, 0xffffffff]),
             'objects': objects, 'kill_target': rnd.choice([objects['t0']['gtid'], objects['t1']['gtid'], 0, 0x7fff0009]),
             'fraction': f64(frac), 'radius': f32(rnd.choice([0.0, 10.0, 55.5, rnd.uniform(0, 100)])),
             'nearest': rnd.choice([None, None, 't0', 't1']), 'kill_activate': rnd.random() < 0.7,
             'wander_point': [f32(rnd.uniform(-100, 100)), f32(rnd.uniform(-100, 100))],
             'rng': rnd.getrandbits(32), 'null_point': rnd.choice([[0, 0], [f32(3.5), f32(-2)]]),
             'states': {'rampage': rnd.choice([0, 1, 2, 2]), 'sleep': rnd.choice([0, 1, 2]),
                        'move': rnd.choice([0, 0, 0, 1, 2]), 'kill': rnd.choice([0, 0, 0, 1, 2]),
                        'other': rnd.choice([1, 2])},
             'stack': rnd.choice(stacks), 'words': [rnd.getrandbits(32) for _ in range(4)],
             'can_activate': {}, 'tracked': TRACKED}
        if rnd.random() < 0.3:
            c['comp5c'] = c['comp60']
        c['think_effects'] = rnd.choice([{}, {}, {'sleep': {'kill': 2}}, {'sleep': {'move': 2}}])
        rows.append(c)
    return rows


def main():
    out, visited = [], {}
    for c in cases():
        m = RampageMachine(c)
        result = m.run()
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': result})
    cov = coverage(visited, {'think_5267e0': (0x5267e0, 0x52693f), 'sleep_51d6e0': (0x51d6e0, 0x51d726),
                             'attack_51e910': (0x51e910, 0x51e9a9), 'wander_526740': (0x526740, 0x5267d1)})
    d = header(__file__, __doc__)
    d.update(instructions=instructions(visited), coverage=cov, cases=out)
    p = HERE / 'native-cai-rampage.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'faults': sum('fault' in r['output'] for r in out),
                      'instructions': len(visited),
                      'coverage': {k: [v['visited'], v['instructions'], v['unvisited']] for k, v in cov.items()},
                      'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
