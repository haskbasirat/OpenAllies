"""Original regiment movement (KKC_CAIStateMove, vtable 0xA33804) helpers and
Element return-to-slot, part 1.

Whole unchanged routines execute: line intersection 0x445280, slot path
0x52CBD0 (after the native copy 0x52D0F0), translated path 0x52D230, extent
clamp 0x5525D0, destination 0x51E9B0 with group offset 0x51D840, activation
0x520320/0x520440 (with 0x529C00), Enter 0x523080 (with speed broadcast
0x5205E0, member iterator 0x4364C0 and path reset 0x52CD90/0x5189F0), Leave
0x51D7B0, move tries 0x526940, return to slot 0x52E9D0/0x52E900 (with
0x52DF60 and 0x529C20).

Seams (recorded): allocation/free, state virtual +4 (eligibility), state
activation 0x52A810, group helpers 0x4D1900/0x4DDEE0/0x4DE6C0, member Mind
virtuals +0x64/+0x88, Mind virtuals +0x124/+0x180, MoveState virtuals
+0x0C/+0x1C, Mind timer (Mind+8)+4, diagnostics 0x404F60. The
CAIMoveMinSecondsBeforeFail static is pre-initialized; its value is an input.
"""
import copy, json, math, random, struct, sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
from harness import Machine, NullDereference, write_document, f2b, b2f, PRECISION  # noqa: E402

PATH_VTABLE = 0xa34738


class Company(Machine):
    def __init__(self, case):
        super().__init__(case['precision'])
        self.c = case
        self.responses = list(case.get('responses', []))
        self.live = {}
        S = self.seams
        S[0x6f7810] = self.allocate
        S[0x6f7430] = self.free
        S[0x85c680] = self.node_alloc
        S[0x85c2b0] = self.node_free
        S[0x404f60] = lambda m: m.ret(0)
        S[0x52a810] = self.activation
        S[0x4ddee0] = self.group_active
        S[0x4de6c0] = self.group_offset
        world = self.alloc(0x200)
        gmap = self.alloc(0x40)
        self.w32(0xb36534, world)
        self.w32(world + 0x30, gmap)
        self.w32(world + 0xe0, case.get('world_time', f2b(50.0)))
        self.w32(gmap + 0xc, *case.get('extent', [f2b(256.0), f2b(192.0)]))
        self.w32(0xb364c0, self.alloc(0x40100))

    # ---- generic helpers ----
    def allocate(self, m):
        n = m.arg()
        p = m.alloc(n)
        self.live[p] = n
        m.ret(0, p)

    def node_alloc(self, m):
        p = m.alloc(0x28)
        self.live[p] = 0x28
        m.ret(0, p)

    def node_free(self, m):
        p = m.arg()
        assert p in self.live, hex(p)
        del self.live[p]
        m.ret(4)

    def free(self, m):
        p = m.arg()
        assert p in self.live, hex(p)
        del self.live[p]
        m.ret(0)

    def response(self, name):
        r = self.responses.pop(0)
        assert r['name'] == name, (r, name)
        return r

    def stub(self, handler):
        a = self.stub_next
        self.stub_next += 16
        self.seams[a] = handler
        return a

    def register(self, handle, obj):
        reg = self.r32(0xb364c0)
        slot = handle & 0xffff
        self.u.mem_write(reg + 4 + slot * 2, struct.pack('<H', handle >> 16))
        self.w32(reg + 0x20004 + slot * 4, obj)

    def path(self, collection, p):
        self.w32(collection, PATH_VTABLE)
        nodes = []
        for words in p['nodes']:
            n = self.alloc(0x28)
            self.live[n] = 0x28
            self.w32(n, *words)
            nodes.append(n)
        for i, n in enumerate(nodes):
            self.w32(n + 0x20, nodes[i - 1] if i else 0)
            self.w32(n + 0x24, nodes[i + 1] if i + 1 < len(nodes) else 0)
        self.w32(collection + 4, nodes[0] if nodes else 0, nodes[-1] if nodes else 0,
                 p['kind1_count'] & 0xffffffff, p['kind2_count'] & 0xffffffff)
        self.w8(collection + 0x14, int(p.get('dirty', False)))
        self.w32(collection + 0x18, p.get('word18', 0))

    def read_path(self, collection):
        nodes, n, last = [], self.r32(collection + 4), 0
        while n:
            nodes.append(self.r32(n, 8))
            assert self.r32(n + 0x20) == last
            last = n
            n = self.r32(n + 0x24)
        assert last == self.r32(collection + 8)
        k1, k2 = self.r32(collection + 0xc, 2)
        return {'nodes': nodes, 'kind1_count': k1 - (1 << 32) if k1 >> 31 else k1,
                'kind2_count': k2 - (1 << 32) if k2 >> 31 else k2, 'dirty': bool(self.r8(collection + 0x14)),
                'word18': self.r32(collection + 0x18)}

    # ---- seams ----
    def activation(self, m):
        m.event('Activate', m.arg())
        m.ret(4, int(self.response('Activate')['value']))

    def group_active(self, m):
        m.event('GroupActive')
        m.ret(4, int(self.response('GroupActive')['value']))

    def group_offset(self, m):
        m.event('GroupOffset')
        out = m.arg(0)
        self.w32(out, *self.response('GroupOffset')['offset'])
        m.ret(0xc, out)

    # ---- regiment state machine ----
    def regiment(self):
        c = self.c
        self.mind = self.alloc(0x400)
        self.actor = self.alloc(0x200)
        mvt = self.alloc(0x200)
        self.w32(self.mind, mvt)
        self.w32(self.mind + 4, self.actor)
        timer_vt = self.alloc(0x10)
        self.w32(self.mind + 8, timer_vt)
        self.w32(timer_vt + 4, self.stub(lambda m: (m.event('Schedule', m.arg(0), m.arg(1)), m.ret(8))))
        self.w32(mvt + 0x124, self.stub(lambda m: (m.event('MoveFailed'), m.ret(0))))
        self.state = self.mind + 0x68
        svt = self.alloc(0x40)
        self.w32(self.state, svt)
        self.w32(svt + 4, self.stub(self.can_activate))
        self.w32(self.state + 4, self.mind)
        s = c['state']
        self.w32(self.state + 0xc, s['status'], s['target_id'], *s['path_destination_bits'],
                 *s['destination_bits'], *s['auxiliary_bits'], s['speed_bits'], s['tries'] & 0xffffffff)
        self.path(self.state + 0x34, s['path'])
        self.w32(self.state + 0x50, s['formation_word'], s['start_time_bits'])
        a = c['actor']
        self.w32(self.actor + 0x20, *a['position'])
        self.w32(self.actor + 0x2c, *a['facing'])
        self.w32(self.actor + 0x164, a['group_handle'])
        if a['group_valid']:
            self.register(a['group_handle'], self.alloc(0x40))
        org = self.alloc(0x200)
        self.w32(self.actor + 0x7c, org)
        self.w32(org + 0xd8, a['speed_cap'])
        members = self.alloc(4 * max(1, len(a['members'])))
        for i, mem in enumerate(a['members']):
            if mem is None:
                continue
            actor = self.alloc(0x200)
            mind = self.alloc(0x40)
            vt = self.alloc(0x200)
            self.w32(mind, vt)
            self.w32(vt + 0x64, self.stub(lambda m, i=i: (m.event('MemberAcceptsSpeed', i),
                                                          m.ret(0, int(self.response('MemberAcceptsSpeed')['value'])))))
            self.w32(vt + 0x88, self.stub(lambda m, i=i: (m.event('MemberSetSpeed', i, m.arg()), m.ret(4))))
            self.w32(actor + 0x70, mind)
            self.w32(members + 4 * i, actor)
        self.w32(org + 0x28, members)
        self.w32(org + 0x2c, len(a['members']))
        for t in c.get('targets', []):
            obj = self.alloc(0x100)
            self.w32(obj + 0x14, t['id'])
            self.w32(obj + 0x20, *t['position'])
            self.register(t['id'], obj)

    def can_activate(self, m):
        m.event('CanActivate')
        m.ret(0, int(self.response('CanActivate')['value']))

    def state_out(self):
        s = self.state
        t = self.r32(s + 0x30)
        return {'status': self.r32(s + 0xc), 'target_id': self.r32(s + 0x10),
                'path_destination_bits': self.r32(s + 0x14, 2), 'destination_bits': self.r32(s + 0x1c, 2),
                'auxiliary_bits': self.r32(s + 0x24, 2), 'speed_bits': self.r32(s + 0x2c),
                'tries': t - (1 << 32) if t >> 31 else t, 'path': self.read_path(s + 0x34),
                'formation_word': self.r32(s + 0x50), 'start_time_bits': self.r32(s + 0x54),
                'mind_auxiliary': self.r32(self.mind + 0x2b4, 2)}

    def run(self):
        c = self.c
        op = c['operation']
        out = {}
        if op == 'intersection':
            pts = self.alloc(0x40)
            for i, p in enumerate(c['points']):
                self.w32(pts + 8 * i, *p)
            res = self.alloc(8)
            self.w32(res, 0xdeadbeef, 0xdeadbeef)
            stub = self.alloc(64)
            code = b''.join(b'\x68' + struct.pack('<I', a) for a in [res, pts + 24, pts + 16, pts + 8, pts])
            code += b'\xe8' + struct.pack('<i', 0x445280 - (stub + len(code) + 5)) + b'\x83\xc4\x14\xc3'
            self.u.mem_write(stub, code)
            al = self.call(stub) & 255
            out = {'result': al, 'point': self.r32(res, 2)}
        elif op in ('slot_path', 'translate_path'):
            src = self.alloc(0x20)
            self.path(src, c['source'])
            dst = self.alloc(0x20)
            self.path(dst, c['target'])
            vec = self.alloc(16)
            self.w32(vec, *c['slot_offset'], *c['facing'])
            if op == 'slot_path':
                self.call(0x52d0f0, dst, [src])
                self.call(0x52cbd0, dst, [vec, vec + 8, src])
            else:
                self.call(0x52d230, dst, [src, *c['slot_offset']])
            out = {'path': self.read_path(dst)}
        elif op == 'clamp':
            pt = self.alloc(8)
            self.w32(pt, *c['point'])
            self.call(0x5525d0, self.r32(self.r32(0xb36534) + 0x30), [pt])
            out = {'point': self.r32(pt, 2)}
        elif op == 'return_to_slot':
            out = self.return_to_slot()
        else:
            self.regiment()
            if op == 'set_destination':
                self.call(0x51e9b0, self.state, c['args'])
            elif op == 'activate_actor':
                target = 0
                if c['target'] is not None:
                    target = self.alloc(0x100)
                    self.w32(target + 0x14, c['target']['id'])
                    self.w32(target + 0x20, *c['target']['position'])
                out['al'] = self.call(0x520320, self.state, [c['mode'], target, *c['args']]) & 255
            elif op == 'activate_location':
                self.call(0x520440, self.state, [c['mode'], *c['args']])
            elif op == 'enter':
                self.call(0x523080, self.state)
            elif op == 'leave':
                self.call(0x51d7b0, self.state)
            elif op == 'tries':
                self.w32(0xb3cf68, self.r32(0xb3cf68) | 1)
                v = self.alloc(8)
                self.w32(v, c['min_seconds'])
                self.w32(0xb3cf64, v)
                out['al'] = self.call(0x526940, self.state) & 255
            out['state'] = self.state_out()
        assert self.fpu_top() == 0
        assert not self.responses, self.responses
        out['events'] = self.events
        return out

    # ---- Element return to slot ----
    def return_to_slot(self):
        c = self.c
        st = self.alloc(0x40)        # caller state (ecx of 0x52E9D0)
        uai = self.alloc(0x200)
        actor = self.alloc(0x200)
        uvt = self.alloc(0x200)
        self.w32(uai, uvt)
        self.w32(uvt + 0x180, self.stub(lambda m: (m.event('MindVirtual180'), m.ret(0))))
        self.w32(st + 4, uai)
        self.w32(uai + 4, actor)
        ms = uai + 0x7c
        mvt = self.alloc(0x40)
        self.w32(ms, mvt)
        self.w32(mvt + 4, self.stub(lambda m: (m.event('CanActivate'), m.ret(0, 1))))
        self.w32(mvt + 0xc, self.stub(lambda m: (m.event('MoveStateThink'), m.ret(0))))
        self.w32(mvt + 0x1c, self.stub(lambda m: (m.event('MoveStateLeave'), m.ret(0))))
        self.w32(ms + 4, uai)
        s = c['move']
        self.w32(ms + 0xc, s['status'])
        self.w32(ms + 0x14, *s['destination_bits'])
        self.w32(ms + 0x30, s['mode'])
        self.w32(ms + 0x24, s['requested_speed_bits'])
        if c['organization'] is not None:
            o = c['organization']
            element = self.alloc(0x40)
            org_actor = self.alloc(0x100)
            comp = self.alloc(0x100)
            slots = self.alloc(8 * 8)
            self.w32(slots + 8 * o['slot'], *o['slot_offset'])
            self.w32(comp + 0x78, slots)
            self.w32(comp + 0xbc, *o['center'])
            self.w32(org_actor + 0x7c, comp)
            self.w32(element + 0x10, org_actor)
            self.w32(element + 0x18, o['slot'])
            self.w32(actor + 0x80, element)
        self.call(0x52e9d0, st)
        return {'move': {'status': self.r32(ms + 0xc), 'target_id': self.r32(ms + 0x10),
                         'destination_bits': self.r32(ms + 0x14, 2), 'auxiliary_bits': self.r32(ms + 0x1c, 2),
                         'requested_speed_bits': self.r32(ms + 0x24), 'phase': self.r32(ms + 0x2c),
                         'mode': self.r32(ms + 0x30)}}


def fb(*v):
    return [f2b(x) for x in v]


def rand_path(rng, n, start=(40.0, 40.0)):
    nodes = []
    x, y = start
    for i in range(n):
        a = rng.uniform(-math.pi, math.pi)
        L = rng.uniform(0.5, 12)
        nx, ny = x + math.cos(a) * L, y + math.sin(a) * L
        nodes.append([f2b(nx), f2b(ny), f2b(math.cos(a)), f2b(math.sin(a)), rng.choice([0, 1, 2]), 0, f2b(nx), f2b(ny)])
        x, y = nx, ny
    return {'nodes': nodes, 'kind1_count': sum(1 for n in nodes if n[4] == 1),
            'kind2_count': sum(1 for n in nodes if n[4] == 2), 'dirty': False, 'word18': 7}


def base_state(rng):
    return {'status': 0, 'target_id': 0, 'path_destination_bits': fb(1, 2), 'destination_bits': fb(3, 4),
            'auxiliary_bits': fb(0.5, -0.25), 'speed_bits': f2b(2.5), 'tries': 0, 'path': rand_path(rng, 3),
            'formation_word': 0x1234, 'start_time_bits': f2b(10.0)}


def base_actor(rng, members=3):
    return {'position': fb(30.5, 41.25), 'facing': fb(0.6, 0.8), 'group_handle': 0x30005, 'group_valid': False,
            'speed_cap': f2b(3.0), 'members': [{} for _ in range(members)]}


def cases():
    rng = random.Random(0x520320)
    rows = []
    def add(**c):
        rows.append(c)
    # Geometry.
    for i in range(80):
        pts = [fb(rng.uniform(-50, 50), rng.uniform(-50, 50)) for _ in range(4)]
        if i % 10 == 0:
            pts[3] = [f2b(b2f(pts[2][0]) + (b2f(pts[1][0]) - b2f(pts[0][0]))),
                      f2b(b2f(pts[2][1]) + (b2f(pts[1][1]) - b2f(pts[0][1])))]
        add(name=f'intersection_{i}', operation='intersection', points=pts)
    add(name='intersection_degenerate', operation='intersection', points=[fb(1, 1)] * 4)
    for i in range(60):
        src = rand_path(rng, rng.randint(1, 6))
        a = rng.uniform(-math.pi, math.pi)
        add(name=f'slot_path_{i}', operation='slot_path', source=src,
            target={'nodes': [], 'kind1_count': 0, 'kind2_count': 0},
            slot_offset=fb(rng.uniform(-8, 8), rng.uniform(-8, 8)), facing=fb(math.cos(a), math.sin(a)))
    for i in range(20):
        src = rand_path(rng, rng.randint(0, 5))
        add(name=f'translate_{i}', operation='translate_path', source=src,
            target=rand_path(rng, rng.randint(0, 2)), slot_offset=fb(rng.uniform(-8, 8), rng.uniform(-8, 8)),
            facing=fb(1, 0))
    for p in [(-1.0, 5.0), (-0.0, 3.0), (255.9999, 191.9999), (256.0, 192.0), (300.0, -2.0), (0.0, 0.0),
              (255.99990844726562, 1.0), (10.0, 191.99991)]:
        add(name=f'clamp_{p}', operation='clamp', point=fb(*p))
    # Destination / activation.
    for i, (valid, active, offset) in enumerate([(False, None, None), (True, False, None), (True, True, (1.5, -2.25)),
                                                 (True, True, (300.0, 0.0)), (True, True, (-0.1, -0.1))]):
        a = base_actor(rng)
        a['group_valid'] = valid
        resp = []
        if valid:
            resp.append({'name': 'GroupActive', 'value': active})
            if active:
                resp.append({'name': 'GroupOffset', 'offset': fb(*offset)})
        for dest in [(20.0, 30.0), (-5.0, 200.0), (255.99999, 0.5)]:
            add(name=f'set_destination_{i}_{dest}', operation='set_destination', state=base_state(rng), actor=a,
                args=fb(*dest), responses=copy.deepcopy(resp))
    for mode in (0, 1, 2, 3):
        for can in (True, False):
            for act in (True, False):
                resp = ([] if mode < 2 else [{'name': 'CanActivate', 'value': can}])
                ok = mode < 2 or can
                if ok:
                    resp.append({'name': 'Activate', 'value': act})
                add(name=f'activate_actor_m{mode}_c{can}_a{act}', operation='activate_actor', state=base_state(rng),
                    actor=base_actor(rng), mode=mode, target={'id': 0x20009, 'position': fb(12.5, 99.0)},
                    args=[f2b(1.75), *fb(0.25, 0.5)], responses=resp)
                add(name=f'activate_location_m{mode}_c{can}_a{act}', operation='activate_location',
                    state=base_state(rng), actor=base_actor(rng), mode=mode,
                    args=[*fb(80.0, -3.0), f2b(4.0), *fb(-1.0, 1.0)], responses=copy.deepcopy(resp))
    add(name='activate_actor_null', operation='activate_actor', state=base_state(rng), actor=base_actor(rng),
        mode=2, target=None, args=[f2b(1.0), 0, 0], responses=[])
    # Enter / Leave / tries.
    for i, (speed, cap, accepts, target) in enumerate([(2.5, 3.0, [True, False, True], None),
                                                       (5.0, 3.0, [True, True, True], 0x20009),
                                                       (0.00001, 3.0, [False, False, False], 0x40001),
                                                       (0.0, 2.0, [True, True], None), (3.0, 3.0, [True], 0x20009)]):
        st = base_state(rng)
        st['speed_bits'] = f2b(speed)
        st['target_id'] = target or 0
        a = base_actor(rng, len(accepts))
        a['members'][0] = {}
        a['speed_cap'] = f2b(cap)
        if i == 4:
            a['members'] = [None, {}, None]
            accepts = [True]
        resp = [{'name': 'MemberAcceptsSpeed', 'value': v} for v in accepts]
        add(name=f'enter_{i}', operation='enter', state=st, actor=a,
            targets=[{'id': 0x20009, 'position': fb(70.0, 12.0)}], responses=resp, world_time=f2b(42.5))
    add(name='leave', operation='leave', state=base_state(rng), actor=base_actor(rng), responses=[])
    for tries in (0, 1, 2, 3, 7):
        for start, now in [(10.0, 50.0), (10.0, 15.0), (10.0, 20.0), (10.0, 19.999)]:
            st = base_state(rng)
            st['tries'] = tries
            st['start_time_bits'] = f2b(start)
            add(name=f'tries_{tries}_{start}_{now}', operation='tries', state=st, actor=base_actor(rng),
                min_seconds=f2b(10.0), world_time=f2b(now), responses=[])
    # Return to slot.
    for status in (0, 2):
        for dest_equal in (True, False):
            for mode in (0, 3):
                org = {'slot': 2, 'slot_offset': fb(-1.5, 2.25), 'center': fb(40.125, 60.5)}
                goal = [f2b(40.125 - 1.5), f2b(60.5 + 2.25)]
                move = {'status': status, 'destination_bits': goal if dest_equal else fb(1, 1), 'mode': mode,
                        'requested_speed_bits': f2b(2.0)}
                resp = []
                acts = 0
                if status != 2:
                    acts += 1
                if not (dest_equal or status != 2) or mode != 0 or (status == 2 and not dest_equal):
                    acts += 1
                add(name=f'return_s{status}_e{dest_equal}_m{mode}', operation='return_to_slot', move=move,
                    organization=org, responses=[{'name': 'Activate', 'value': True}] * 4)
    add(name='return_no_organization', operation='return_to_slot',
        move={'status': 0, 'destination_bits': fb(1, 1), 'mode': 0, 'requested_speed_bits': 0},
        organization=None, responses=[])
    return rows


def main():
    out = []
    for t in cases():
        for precision in PRECISION:
            c = copy.deepcopy(t)
            c['precision'] = precision
            m = Company(c)
            try:
                c['output'] = m.run()
            except NullDereference as error:
                c['output'] = {'null_dereference': str(error), 'events': m.events}
            except AssertionError:
                if m.responses:
                    # Trim unconsumed optional responses (Activate pads) and rerun.
                    used = len(c['responses']) - len(m.responses)
                    c['responses'] = c['responses'][:used]
                    m = Company(c)
                    c['output'] = m.run()
                else:
                    raise
            out.append(c)
    write_document('native-company.json', __doc__, out)


if __name__ == '__main__':
    main()
