"""Original SAI goal execution helper 7A5E10 (and, directly, the actor-goal
release test 7C9CE0) executing unchanged under Unicorn, with 7C9CE0,
7B96E0, 4D19A0, 4D0900, 4F1430, 4D03C0, 7B9050, 7A10F0, 529C30 and
7A0FC0 executed natively.

Seams (answered from the case RNG, recorded in call order): goal virtuals
+4 (type), +20 (location), +2C (held value, ST0, ret 4), +40; SAI actor
record virtual +C; object virtual +108 (owner); player-object virtuals
+138/+140; 7A5200 priority finalization (goal, owner economy +1C4, draw);
7C8AA0 actor-goal readiness; 4D0180 combat value (ST0); 7C8AF0 command
build on the actor goal (&command, actor, location) with 7B4FA0 queue on
the player and 7ADC60 destructor; allocator 6F7810/6F7430.

Memory: GTID registry; objects (+8 flags, +14 GTID, +154 record); records
(+8 object GTID, +10 holding goal, +30 route state, +3C actor goal);
goals (+4 -> +4 SAI player, +38 priority, +3C record); regions (+1C entry
array; entry +0 player, +8/+C mobile and +14/+18 settlement influence);
SAI player (+0 slot, +8 player object, +1C ego +304); [B36534]+148/+14C
kingdoms. The GTID list argument is {+8 head, +C tail} of {gtid, next}.

Outputs: events, the list after the call, each record's +30 (cleared or
not), the 7C9CE0 result for direct cases. Three precisions.
"""
from pathlib import Path
import json, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from sai_world import World, x87_of, x87_ext, f32, header, sha, PRECISIONS, WORLD
from unicorn import x86_const as X


class ReinforceWorld(World):
    def __init__(self, c):
        super().__init__(c)
        objs = {int(k): v for k, v in c['objects'].items()}
        sizes = {'actor': 0x200, 'record': 0x40, 'goal': 0x80, 'region': 0x40, 'player': 0x40, 'kingdom': 0x10}
        for ident, o in objs.items():
            self.place(ident, sizes[o['kind']])
        actor_vt = self.vtable({0x108: self.v108, 0x138: self.v138, 0x140: self.v140})
        record_vt = self.vtable({0xc: self.record_dead})
        goal_vt = self.vtable({0x4: self.goal_type, 0x20: self.goal_location, 0x2c: self.held_value, 0x40: self.v40})
        self.routes = {}
        for ident, o in objs.items():
            p = self.a(ident)
            k = o['kind']
            if k == 'actor':
                self.w(p, actor_vt); self.w(p + 8, o['flags']); self.w(p + 0x14, o['gtid'])
                self.w(p + 0x154, self.a(o['record']))
            elif k == 'record':
                self.w(p, record_vt); self.w(p + 4, 1000); self.w(p + 8, o['object'])
                self.w(p + 0x10, self.a(o['owner_goal'])); self.w(p + 0x3c, self.a(o['goal']))
                if o['route'] is not None:
                    r = self.alloc(8); self.w(r, o['route']); self.w(p + 0x30, r)
            elif k == 'player':
                self.w(p, o['slot']); self.w(p + 8, self.a(o['object']))
                ego = self.alloc(0x400); self.w(ego + 0x304, o['ego304']); self.w(p + 0x1c, ego)
        for ident, o in objs.items():
            p = self.a(ident)
            if o['kind'] == 'goal':
                ctl = self.alloc(0x10); self.w(ctl + 4, self.a(o['player']))
                self.w(p, goal_vt); self.w(p + 4, ctl); self.w(p + 0x38, o['priority']); self.w(p + 0x3c, self.a(o['record']))
            elif o['kind'] == 'region':
                n = max(e['slot'] for e in o['entries']) + 1 if o['entries'] else 1
                ents = self.alloc(4 * n)
                for e in o['entries']:
                    ep = self.alloc(0x30); self.w(ep, self.a(e['player']))
                    for off, key in ((8, 'mobile'), (0x14, 'settlement'), (0x20, 'border')):
                        v = e[key]; d = self.alloc(4 * max(len(v), 1))
                        for i, x in enumerate(v):
                            self.w(d + 4 * i, x)
                        self.w(ep + off, d); self.w(ep + off + 4, len(v)); self.w(ep + off + 8, len(v))
                    self.w(ents + 4 * e['slot'], ep)
                self.w(p + 0x1c, ents); self.w(p + 0x20, n)
        self.registry(c['registry'])
        world = self.alloc(0x400); self.w(WORLD, world)
        ks = c['kingdoms']; ka = self.alloc(4 * max(len(ks), 1))
        for i, k in enumerate(ks):
            self.w(ka + 4 * i, self.a(k))
        self.w(world + 0x148, ka); self.w(world + 0x14c, len(ks))
        # The GTID list argument.
        self.list = self.alloc(0x10)
        head = 0; tail = 0
        for g in reversed(c['list']):
            node = self.alloc(8); self.w(node, g); self.w(node + 4, head); head = node
            if tail == 0:
                tail = node
        self.w(self.list + 8, head); self.w(self.list + 0xc, tail)
        self.holder = self.alloc(4); self.w(self.holder, self.list)
        for addr, fn in [(0x7a5200, self.s_finalize), (0x7c8aa0, self.s_ready), (0x4d0180, self.s_cv),
                         (0x7c8af0, self.s_build), (0x7b4fa0, self.s_queue), (0x7adc60, self.s_destroy)]:
            self.seam(addr, fn)

    def this(self):
        return self.id_of(self.ecx())

    def b(self, p):
        return self.rnd.random() < p

    def v108(self, m):
        r = self.pick(self.c['pools']['owner']); self.ev('owner', obj=self.this(), result=r); self.ret(self.a(r))

    def v138(self, m):
        r = self.b(0.5); self.ev('virtual_138', obj=self.this(), other=self.id_of(self.arg(1)), result=r); self.ret(int(r), 4)

    def v140(self, m):
        r = self.b(0.7); self.ev('virtual_140', obj=self.this(), other=self.id_of(self.arg(1)), result=r); self.ret(int(r), 4)

    def record_dead(self, m):
        r = self.b(0.1); self.ev('record_dead', record=self.this(), result=r); self.ret(int(r))

    def goal_type(self, m):
        r = self.pick([9, 0, 1, 9, 12]); self.ev('goal_type', goal=self.this(), result=r); self.ret(r)

    def goal_location(self, m):
        r = self.pick(self.c['pools']['location']); self.ev('goal_location', goal=self.this(), result=r); self.ret(self.a(r))

    def held_value(self, m):
        img = self.pick([x87_ext(self.rnd, -20, 300), x87_of(0.0), x87_ext(self.rnd, 0, 60)])
        self.ev('held_value', goal=self.this(), record=self.id_of(self.arg(1)), result=img); self.x87_return(img, 4)

    def v40(self, m):
        r = self.b(0.15); self.ev('goal_virtual_40', goal=self.this(), record=self.id_of(self.arg(1)), result=r)
        self.ret(int(r), 4)

    def s_finalize(self, m):
        res = self.arg(1)
        owner = 0 if res == 0x1c4 else self.id_of(res - 0x1c4)
        self.ev('finalize', goal=self.this(), owner=owner, draw=self.arg(2) & 0xff); self.ret(0, 8)

    def s_ready(self, m):
        r = self.b(0.75); self.ev('goal_ready', goal=self.this(), result=r); self.ret(int(r))

    def s_cv(self, m):
        img = self.pick([x87_ext(self.rnd, 0, 80), x87_of(0.0), x87_ext(self.rnd, 0, 400)])
        self.ev('combat_value', obj=self.this(), result=img); self.x87_return(img)

    def s_build(self, m):
        out = self.arg(1)
        self.ev('build_command', goal=self.this(), actor=self.id_of(self.arg(2)), location=self.id_of(self.arg(3)))
        self.w(out, 0x5a1c0de); self.ret(out, 0xc)

    def s_queue(self, m):
        assert self.r(self.arg(1)) == 0x5a1c0de
        self.ev('queue_command', player=self.this()); self.ret(0, 4)

    def s_destroy(self, m):
        self.ret(0)

    def run(self):
        out = {}
        c = self.c
        try:
            if c['entry'] == 'reinforce':
                self.call(0x7a5e10, ecx=self.a(c['goal']), args=[self.holder], count=2_000_000)
            elif c['entry'] == 'influence':
                region = self.a(c['region']); slot = c['objects'][str(c['player'])]['slot']
                entry = self.r(self.r(region + 0x1c) + 4 * slot)
                fn, args = {'7B96E0': (0x7b96e0, [c['flag']]), '7B9600': (0x7b9600, [c['flag']]),
                            '7B97C0': (0x7b97c0, []), '7B9840': (0x7b9840, [])}[c['function']]
                self.call(fn, ecx=entry, args=args)
                out['st0'] = self.st0()
            else:
                r = self.call(0x7c9ce0, ecx=self.a(c['goal']), args=[c['value'], c['flag']], count=2_000_000)
                out['result'] = bool(r & 0xff)
            out['fault'] = False
        except AssertionError as e:
            out['fault'] = True; out['fault_detail'] = str(e)[:200]; self.null_accesses.clear()
        except Exception as e:
            out['fault'] = True; out['fault_detail'] = '%s at %x' % (e, self.u.reg_read(X.UC_X86_REG_EIP))
        out['events'] = self.events
        lst, node = [], self.r(self.list + 8)
        while node:
            lst.append(self.r(node)); node = self.r(node + 4)
        out['list'] = lst
        out['routes'] = {k: self.r(self.a(int(k)) + 0x30) != 0 for k, o in c['objects'].items() if o['kind'] == 'record'}
        return out


def rf(rnd, lo, hi):
    return f32(rnd.uniform(lo, hi))


def make_case(rnd, entry, precision, seed):
    ids = iter(range(1, 10000))
    objects, registry, used = {}, {}, set()

    def gtid_for(obj, live=True):
        while True:
            idx = rnd.randrange(1, 0x400)
            if idx not in used:
                break
        used.add(idx)
        serial = rnd.randrange(1, 0x8000)
        registry[idx] = [serial, obj if live else 0]
        return (serial << 16) | idx

    kingdoms = []
    for i in range(rnd.randrange(1, 5)):
        k = next(ids); objects[k] = {'kind': 'kingdom'}; kingdoms.append(k)
    kingdoms_list = [rnd.choice(kingdoms + [0]) for _ in range(rnd.randrange(0, 6))]
    pobj = next(ids); objects[pobj] = {'kind': 'actor', 'flags': 0, 'gtid': 0, 'record': 0}
    player = next(ids)
    objects[player] = {'kind': 'player', 'slot': rnd.randrange(0, 3), 'object': pobj if rnd.random() < 0.97 else 0,
                       'ego304': rnd.choice([rf(rnd, 0.2, 3.0), f32(1.0), 0])}
    goals = []
    for i in range(rnd.randrange(2, 5)):
        g = next(ids); goals.append(g)
        objects[g] = {'kind': 'goal', 'player': player, 'record': 0,
                      'priority': rnd.choice([rf(rnd, 0.1, 300), rf(rnd, 0.1, 300), 0, f32(-2.0), rf(rnd, 0, 40)])}
    regions = []
    for i in range(rnd.randrange(1, 4)):
        r = next(ids); regions.append(r)
        def vec():
            return [rnd.choice([0, 0, rf(rnd, 0, 200), rf(rnd, -5, 50), f32(-1.0)]) for _ in range(rnd.randrange(0, 6))]
        objects[r] = {'kind': 'region', 'entries': [{'slot': objects[player]['slot'], 'player': player,
                                                      'mobile': vec(), 'settlement': vec(), 'border': vec()}]}
    actors, records = [], []
    for i in range(rnd.randrange(2, 9)):
        a = next(ids); actors.append(a)
    for a in actors:
        rec = next(ids) if rnd.random() < 0.85 else 0
        objects[a] = {'kind': 'actor', 'flags': rnd.choice([0, 0, 0, 1, 0x10]), 'gtid': 0, 'record': rec}
        objects[a]['gtid'] = gtid_for(a, rnd.random() < 0.9)
        if rec:
            records.append(rec)
            objects[rec] = {'kind': 'record', 'object': objects[a]['gtid'] if rnd.random() < 0.95 else gtid_for(a, False),
                            'owner_goal': rnd.choice(goals + [0]), 'goal': rnd.choice(goals + [0]) if rnd.random() < 0.85 else 0,
                            'route': rnd.choice([None, 2, 2, 1, 0])}
    for g in goals:
        if records and rnd.random() < 0.85:
            objects[g]['record'] = rnd.choice(records)
    lst = [objects[a]['gtid'] for a in actors if rnd.random() < 0.8]
    if rnd.random() < 0.2 and lst:
        lst.append(rnd.choice(lst))
    rnd.shuffle(lst)
    pools = {'owner': [0] + kingdoms + [pobj], 'location': regions + regions + ([0] if rnd.random() < 0.3 else [])}
    c = {'entry': entry, 'precision': precision, 'seed': seed, 'objects': {str(k): v for k, v in objects.items()},
         'registry': {str(k): v for k, v in registry.items()}, 'kingdoms': kingdoms_list, 'list': lst,
         'pools': pools, 'goal': rnd.choice(goals)}
    if entry == 'influence':
        c['function'] = rnd.choice(['7B96E0', '7B9600', '7B97C0', '7B9840'])
        c['region'] = rnd.choice(regions); c['player'] = player
        c['flag'] = rnd.choice([0, 1, 0x100])
    if entry == 'release':
        c['value'] = rnd.choice([rf(rnd, -10, 300), objects[c['goal']]['priority'], 0])
        c['flag'] = rnd.choice([0, 1, 1, 0x100])
    return c


def main():
    n = int(sys.argv[1]) if len(sys.argv) > 1 else 900
    out, visited, stats = [], {}, {}
    precisions = list(PRECISIONS)
    rnd = random.Random(0x7a5e10)
    for i in range(n):
        entry = ['release', 'reinforce', 'reinforce', 'influence'][i % 4]
        c = make_case(rnd, entry, precisions[i % 3], rnd.getrandbits(32))
        m = ReinforceWorld(c)
        r = m.run()
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': r})
        s = stats.setdefault(entry, {'cases': 0, 'faults': 0}); s['cases'] += 1; s['faults'] += int(r['fault'])
    d = header(__file__, __doc__)
    d.update(cases=out, stats=stats)
    p = HERE / 'native-reinforce.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    from fn_extent import extent
    cover = {}
    for name, start in [('7A5E10', 0x7a5e10), ('7C9CE0', 0x7c9ce0), ('7B96E0', 0x7b96e0), ('7B9600', 0x7b9600),
                        ('7B97C0', 0x7b97c0), ('7B9840', 0x7b9840), ('4D19A0', 0x4d19a0),
                        ('4F1430', 0x4f1430), ('7A0FC0', 0x7a0fc0)]:
        body = [x.address for x in extent(start)]
        cover[name] = '%d/%d' % (len([a for a in body if a in visited]), len(body))
    print(json.dumps({'cases': len(out), 'stats': stats, 'coverage': cover, 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
