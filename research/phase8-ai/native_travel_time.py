"""Original SAI actor travel cost 7A1B30 and travel time 7A1C10 executing
unchanged under Unicorn. Seams: the actor virtuals +8 (start-region holder),
+C (unavailable) and +10 (speed, an 80-bit ST0), virtual +20 of the actor's
+10 object (cached location), and the route block 53F320 / 53E5B0 / 5C2CD0
(recorded with its solver, start, goal and flag; 53E5B0 answers a binary32
cost through ST0 as the original list sum is stored per step). Actor +14 (the
cached binary32 cost) and the location's +0 region are plain memory.

Cases: random actors/locations at the three precisions through 7A1C10 (and
directly through 7A1B30): null location, unavailable answers (the +C virtual
is asked twice on the normal path), cached costs below/at/above zero with a
matching or other cached location, empty routes (-1.0), and speeds with
extended-precision significands (the fdivr rounds at the precision control).
"""
from pathlib import Path
import json, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import Machine, PRECISIONS, f32, header, instructions, sha
from unicorn import x86_const as X


def x87_of(v):
    """80-bit image {significand, sign_exponent} of a finite nonzero Python float."""
    import math
    m, e = math.frexp(abs(v))
    sig = int(m * (1 << 64))
    return {'significand': sig & ((1 << 64) - 1), 'sign_exponent': ((v < 0) << 15) | (e - 1 + 16383)}


class TravelMachine(Machine):
    def __init__(self, precision, c):
        super().__init__(precision)
        self.c = c
        self.unavailable_answers = list(c['unavailable'])
        vt = self.alloc(0x40)
        self.w(vt + 0x8, self.stub(self.s_start)); self.w(vt + 0xc, self.stub(self.s_unavailable))
        self.w(vt + 0x10, self.stub(self.s_speed))
        self.actor = self.alloc(0x40); self.w(self.actor, vt)
        vt2 = self.alloc(0x40); self.w(vt2 + 0x20, self.stub(self.s_cached_location))
        holder = self.alloc(0x10); self.w(holder, vt2)
        self.w(self.actor + 0x10, holder); self.w(self.actor + 0x14, c['cached_cost'])
        self.start_holder = self.alloc(0x10); self.w(self.start_holder, c['start_region'])
        self.locations = {}
        for name in ('location', 'cached_location'):
            h = c[name]
            if h and h not in self.locations:
                p = self.alloc(0x10); self.w(p, c['regions'][str(h)]); self.locations[h] = p
        mgr = self.alloc(0x400); self.w(0xb36534, mgr)
        self.solver = self.alloc(0x40); self.w(mgr + 0x234, self.solver)
        self.seam(0x53f320, self.s_search); self.seam(0x53e5b0, self.s_cost); self.seam(0x5c2cd0, self.s_release)
        self.route = None

    def handle(self, p):
        for h, q in self.locations.items():
            if q == p:
                return h
        return 0 if p == 0 else -1

    def s_unavailable(self, m):
        assert self.ecx() == self.actor
        r = self.unavailable_answers.pop(0)
        self.events.append({'op': 'unavailable', 'result': r}); self.ret(int(r))

    def s_cached_location(self, m):
        r = self.c['cached_location']
        self.events.append({'op': 'cached_location', 'result': r}); self.ret(self.locations.get(r, 0))

    def s_start(self, m):
        assert self.ecx() == self.actor
        self.events.append({'op': 'start_region', 'result': self.c['start_region']}); self.ret(self.start_holder)

    def s_speed(self, m):
        assert self.ecx() == self.actor
        v = self.c['speed']
        self.events.append({'op': 'speed', 'result': v})
        scratch = self.alloc(16)
        self.u.mem_write(scratch, struct.pack('<QH', v['significand'], v['sign_exponent']))
        stub = self.alloc(16)
        self.u.mem_write(stub, b'\xdb\x2d' + struct.pack('<I', scratch) + b'\xc3')
        self.u.reg_write(X.UC_X86_REG_EIP, stub)

    def s_search(self, m):
        assert self.ecx() == self.solver
        lst, start, goal, flag = (self.arg(i) for i in (1, 2, 3, 4))
        assert flag == 1
        self.route = lst
        self.events.append({'op': 'route', 'start': start, 'goal': goal, 'result': self.c['route_cost']})
        self.ret(1, 0x10)

    def s_cost(self, m):
        assert self.ecx() == self.route
        self.ret_st0_f32(self.c['route_cost'])

    def s_release(self, m):
        assert self.ecx() == self.route
        self.ret(0)

    def run(self):
        entry = self.c['entry']
        loc = self.locations.get(self.c['location'], 0)
        self.call(entry, ecx=self.actor, args=[loc], pop=4)
        out = {'events': self.events, 'st0': self.st0()}
        assert not self.unavailable_answers, 'unused answers'
        return out


def case(rnd, i):
    loc = rnd.choice([0, 0x5000, 0x5000, 0x5000, 0x5040])
    cached_loc = rnd.choice([0, 0x5000, 0x5040, 0x5080])
    entry = 0x7a1b30 if i % 5 == 4 else 0x7a1c10
    if entry == 0x7a1b30 and loc == 0:
        loc = 0x5000
    unav = [rnd.random() < 0.15, rnd.random() < 0.1]
    speed = rnd.choice([x87_of(rnd.uniform(0.5, 30)), x87_of(3.0), x87_of(7.0),
                        {'significand': (1 << 63) | rnd.getrandbits(40), 'sign_exponent': 16384}])
    c = {'entry': entry, 'location': loc, 'cached_location': cached_loc,
         'regions': {'20480': rnd.randrange(60), '20544': rnd.randrange(60), '20608': rnd.randrange(60)},
         'cached_cost': rnd.choice([0xbf800000, 0x80000000, 0, f32(rnd.uniform(0, 500)), f32(rnd.uniform(0, 500))]),
         'start_region': rnd.randrange(60),
         'route_cost': rnd.choice([0xbf800000, 0, f32(rnd.uniform(0, 900)), f32(rnd.uniform(0, 900))]),
         'speed': speed}
    # Exactly the virtual +C answers the original will ask for.
    if entry == 0x7a1c10:
        if loc == 0:
            c['unavailable'] = []
        elif unav[0]:
            c['unavailable'] = [True]
        else:
            c['unavailable'] = [False, unav[1]]
    else:
        c['unavailable'] = [unav[1]]
    return c


def main():
    rnd = random.Random(0x7a1c10)
    out, visited = [], {}
    precisions = list(PRECISIONS)
    for i in range(600):
        c = case(rnd, i)
        c['precision'] = precisions[i % 3]
        m = TravelMachine(c['precision'], c)
        r = m.run()
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': r})
    d = header(__file__, __doc__)
    d.update(instructions=instructions(visited), cases=out)
    p = HERE / 'native-travel-time.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    body = [a for a in visited if 0x7a1b30 <= a < 0x7a1c60]
    print(json.dumps({'cases': len(out), 'instructions': len(visited), 'body': len(body), 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
