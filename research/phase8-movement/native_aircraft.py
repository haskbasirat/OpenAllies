"""Original AIRCRAFT component (AirCraft.cpp) movement step and its helpers.

Whole unchanged routines execute: MoveTick 0x5612F0 (vtable 0xA3AF78 +0x38,
reached from Mover Tick 0x577E50 when actor flag bit12 is set), path advance
0x55E2F0 with arc length 0x55DCA0, cubic evaluation 0x55DA10 and normalize
0x4453A0, holding-circle planner 0x560050 with leg target 0x55FE60, path reset
0x55E1B0, formation follow 0x561080 with slot offset 0x5685E0 and handle
getter 0x4CF1B0, member path reset 0x55EBE0, group leader 0x5685B0 / leader
test 0x567FF0, following-path predicate 0x516B10 and Mover requested-speed
clamp 0x576D20.

Seams (recorded, never executed): Mind virtuals +0x1B0 (movement blocked),
+0x144 (path exhausted) and (Mind+8) virtual +4 (reschedule), actor virtuals
+0x18 (current speed) and +0x104 (formation speed), attack descriptor
constructor 0x4C8300, ability check 0x4C8F20, combat fire 0x57E030 (actor+0x74),
diagnostics 0x404F60, formation teardown 0x55EDC0, formation follow planner
0x560A70, leg builder 0x55F230, Mover pose commit 0x576BB0, terrain height
0x4F0E50 and group virtual +0x14. 0x6F7430 (segment free) is an allocator
boundary: it is checked (every freed block is a live segment) but not
recorded. Seam effects on the aircraft are supplied per case ("mutation").
"""
import copy, json, math, random, struct, sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
from harness import Machine, NullDereference, write_document, f2b, b2f, d2b, PRECISION  # noqa: E402

AIRCRAFT_VTABLE, AIRCRAFT_VTABLE_8, PATH_VTABLE, SEGMENT_VTABLE = 0xa3af78, 0xa3af50, 0xa3af48, 0xa3af40
# Bezier basis copied by the constructor 0x55E8E0 from 0xB281D8 (via 0x55D9A0).
BEZIER = [0xbf800000, 0x40400000, 0xc0400000, 0x3f800000, 0x40400000, 0xc0c00000, 0x40400000, 0,
          0xc0400000, 0x40400000, 0, 0, 0x3f800000, 0, 0, 0]
TENTH = 0x3dcccccd
SENTINEL = 0xcdcdcdcd
WORD_FIELDS = {0x14, 0x2c, 0x30, 0x3c, 0x80, 0x84, 0x88, 0x8c}
BYTE_FIELDS = {0x10, 0x90}
SELF = 1


def fb(*values):
    return [f2b(v) for v in values]


class Aircraft(Machine):
    def __init__(self, case):
        super().__init__(case['precision'])
        self.c = case
        self.queues = {k: list(v) for k, v in case.get('responses', {}).items()}
        self.used = {k: 0 for k in self.queues}
        self.segments = set()
        self.registry = self.alloc(0x20004 + 0x400)
        self.w32(0xb364c0, self.registry)
        self.gworld = self.alloc(0x40)
        self.w32(0xb36534, self.gworld)
        self.actor_ptr, self.actor_key = {}, {}
        self.group_ptr, self.group_key = {}, {}
        self.aircraft_ptr = {}
        S = self.seams
        S[0x4c8300] = self.attack_descriptor
        S[0x4c8f20] = self.ability_check
        S[0x57e030] = self.fire_ability
        S[0x404f60] = self.diagnostic
        S[0x55edc0] = self.leave_formation
        S[0x560a70] = self.follow_path
        S[0x55f230] = self.build_leg
        S[0x576bb0] = self.commit_pose
        S[0x4f0e50] = self.ground_height
        S[0x6f7430] = self.free
        self.build_world()

    # ---- world construction --------------------------------------------
    def stub(self, handler):
        a = self.stub_next
        self.stub_next += 16
        self.seams[a] = handler
        return a

    def build_world(self):
        c = self.c
        a = c.get('actor')
        if a is not None:
            actor = self.alloc(0x200)
            self.actor_ptr[SELF], self.actor_key[actor] = actor, SELF
            self.actor = actor
            vt = self.alloc(0x200)
            self.w32(actor, vt)
            self.w32(vt + 0x18, self.stub(lambda m: self.float_response(m, 'ActorSpeed')))
            self.w32(vt + 0x104, self.stub(lambda m: self.float_response(m, 'FormationSpeed')))
            definition = self.alloc(0x500)
            d = a['definition']
            self.w32(definition + 0x408, d['taxi_scale'])
            self.w32(definition + 0x40c, d['taxi_height'])
            self.w32(definition + 0x414, d['operational_height'])
            self.w32(actor + 4, definition)
            self.w32(actor + 8, a['flags'])
            self.w32(actor + 0x14, a['handle'])
            self.w32(actor + 0x20, *a['position'])
            self.w32(actor + 0x2c, *a['facing'])
            self.w32(actor + 0x3c, a['scale'])
            self.mover = self.alloc(0x100)
            mv = c['mover']
            self.w32(self.mover + 0x24, mv['requested_speed'])
            self.w32(self.mover + 0x28, mv['max_speed'])
            self.w32(self.mover + 0x50, mv['speed_factor'])
            self.w32(actor + 0x6c, self.mover)
            if a['has_mind']:
                mind = self.alloc(0x40)
                mvt, mvt8 = self.alloc(0x200), self.alloc(0x20)
                self.w32(mind, mvt)
                self.w32(mvt + 0x1b0, self.stub(self.mind_blocked))
                self.w32(mvt + 0x144, self.stub(lambda m: (m.event('MindPathExhausted'), m.ret(0))))
                self.w32(mind + 8, mvt8)
                self.w32(mvt8 + 4, self.stub(self.mind_reschedule))
                self.w32(mind + 0xc, 0x5ced1e if a['mind_scheduled'] else 0)
                self.mind = mind
                self.w32(actor + 0x70, mind)
            self.combat = self.alloc(0x20)
            self.w32(actor + 0x74, self.combat)
            self.ability = self.alloc(0x20)
            ability_def = self.alloc(0x80)
            self.w32(self.ability + 4, ability_def)
            self.w32(ability_def + 0x54, a.get('ability_word_54', 0))
        for o in c.get('others', []):
            p = self.alloc(0x100)
            self.actor_ptr[o['key']], self.actor_key[p] = p, o['key']
            self.w32(p + 8, o['flags'])
            self.w32(p + 0x14, o['handle'])
        for g in c.get('groups', []):
            p = self.alloc(0x40)
            self.group_ptr[g['key']], self.group_key[p] = p, g['key']
            vt = self.alloc(0x20)
            self.w32(p, vt)
            self.w32(vt + 0x14, self.stub(self.group_disbanded))
            self.w32(p + 4, 1)
            if g['layout'] is not None:
                lay = self.alloc(0x20)
                offsets = self.alloc(12 * max(1, len(g['layout']['offsets'])))
                for i, off in enumerate(g['layout']['offsets']):
                    self.w32(offsets + 12 * i, *off)
                self.w32(lay + 0x10, offsets)
                self.w32(lay + 0x14, g['layout']['slot_word'])
                self.w32(p + 8, lay)
            nxt = 0
            for h in reversed(g['members']):
                node = self.alloc(8)
                self.w32(node, h, nxt)
                nxt = node
            self.w32(p + 0xc, nxt)
            self.w32(p + 0x14, *g['anchor'])
        for index, generation, key in c.get('handles', []):
            self.u.mem_write(self.registry + 4 + index * 2, struct.pack('<H', generation))
            self.w32(self.registry + 0x20004 + index * 4, self.actor_ptr[key] if key else 0)
        if 'aircraft' in c:
            self.aircraft = self.write_aircraft(SELF, c['aircraft'])
            if a is not None:
                self.w32(self.actor + 0xbc, self.aircraft)
        for o in c.get('others', []):
            if o['aircraft'] is not None:
                ac = self.write_aircraft(o['key'], o['aircraft'])
                self.w32(self.actor_ptr[o['key']] + 0xbc, ac)

    def write_segment(self, ac, s, nxt=0):
        p = self.alloc(0x5c)
        self.w32(p, SEGMENT_VTABLE, ac + 0x40, *s['control'])
        self.w32(p + 0x38, s['length'], s['field_3c'], *s['end_direction'])
        self.w32(p + 0x4c, s['end_speed'], s['start_speed'], s['tolerance'], nxt)
        self.segments.add(p)
        return p

    def write_aircraft(self, key, a):
        ac = self.alloc(0xc0)
        self.aircraft_ptr[key] = ac
        self.w32(ac, AIRCRAFT_VTABLE, self.actor_ptr.get(key, 0), AIRCRAFT_VTABLE_8)
        self.w8(ac + 0x10, a['on_ground'])
        self.w32(ac + 0x14, a['cruise_speed'])
        p = a['path']
        self.w32(ac + 0x28, PATH_VTABLE, p['planning_speed'], p['turn_radius'])
        self.w32(ac + 0x3c, p['t'], *p['basis'])
        self.w32(ac + 0x80, p['altitude'], p['leg_speed'], p['total_length'], p['speed'])
        head = tail = 0
        for s in reversed(p['segments']):
            head = self.write_segment(ac, s, head)
            tail = tail or head
        self.w32(ac + 0x34, head, tail)
        self.w8(ac + 0x90, a['circling'])
        self.w32(ac + 0x94, a['attack_target'], *a['attack_point'])
        self.w32(ac + 0xa4, self.ability if a['ability'] else 0)
        self.w32(ac + 0xa8, *[self.group_ptr[g] if g else 0 for g in a['groups']])
        return ac

    def read_segment(self, ac, p):
        assert self.r32(p) == SEGMENT_VTABLE and self.r32(p + 4) == ac + 0x40, hex(p)
        return {'control': self.r32(p + 8, 12), 'length': self.r32(p + 0x38), 'field_3c': self.r32(p + 0x3c),
                'end_direction': self.r32(p + 0x40, 3), 'end_speed': self.r32(p + 0x4c),
                'start_speed': self.r32(p + 0x50), 'tolerance': self.r32(p + 0x54)}

    def read_aircraft(self, ac):
        segments, p, last = [], self.r32(ac + 0x34), 0
        while p:
            assert p in self.segments, hex(p)
            segments.append(self.read_segment(ac, p))
            last, p = p, self.r32(p + 0x58)
        assert self.r32(ac + 0x38) == last, 'tail does not match the last segment'
        ability = self.r32(ac + 0xa4)
        assert ability in (0, getattr(self, 'ability', None))
        return {
            'on_ground': self.r8(ac + 0x10), 'cruise_speed': self.r32(ac + 0x14), 'circling': self.r8(ac + 0x90),
            'attack_target': self.r32(ac + 0x94), 'attack_point': self.r32(ac + 0x98, 3),
            'ability': 1 if ability else 0,
            'groups': [self.group_key[g] if g else 0 for g in self.r32(ac + 0xa8, 3)],
            'path': {'planning_speed': self.r32(ac + 0x2c), 'turn_radius': self.r32(ac + 0x30),
                     't': self.r32(ac + 0x3c), 'basis': self.r32(ac + 0x40, 16), 'altitude': self.r32(ac + 0x80),
                     'leg_speed': self.r32(ac + 0x84), 'total_length': self.r32(ac + 0x88),
                     'speed': self.r32(ac + 0x8c), 'segments': segments},
        }

    def mutate(self, ac, mutation):
        """Seam effect on the aircraft, as specified by the case."""
        if mutation.get('clear'):
            p = self.r32(ac + 0x34)
            while p:
                self.segments.discard(p)
                p = self.r32(p + 0x58)
            self.w32(ac + 0x34, 0, 0, 0)
            self.w32(ac + 0x88, 0)
        for s in mutation.get('append', []):
            seg = self.write_segment(ac, s)
            tail = self.r32(ac + 0x38)
            if tail:
                self.w32(tail + 0x58, seg)
            else:
                self.w32(ac + 0x34, seg)
            self.w32(ac + 0x38, seg)
        for off, bits in mutation.get('words', []):
            assert off in WORD_FIELDS
            self.w32(ac + off, bits)
        for off, value in mutation.get('bytes', []):
            assert off in BYTE_FIELDS
            self.w8(ac + off, value)
        if 'groups' in mutation:
            self.w32(ac + 0xa8, *[self.group_ptr[g] if g else 0 for g in mutation['groups']])

    # ---- seams -----------------------------------------------------------
    def response(self, name):
        q = self.queues.get(name, [])
        assert self.used.get(name, 0) < len(q), 'missing response ' + name
        r = q[self.used[name]]
        self.used[name] += 1
        return r

    def float_response(self, m, name):
        assert m.ecx() == self.actor
        m.event(name)
        m.ret_float(int(self.response(name)['f64'], 16), 'f64')

    def mind_blocked(self, m):
        assert m.ecx() == self.mind
        m.event('MindBlocked')
        m.ret(0, int(self.response('MindBlocked')['value']))

    def mind_reschedule(self, m):
        assert m.ecx() == self.mind + 8
        m.event('MindReschedule', m.arg(0), m.arg(1))
        m.ret(8)

    def group_disbanded(self, m):
        key = self.group_key[m.ecx()]
        m.event('GroupDisbanded', key)
        g = next(g for g in self.c['groups'] if g['key'] == key)
        m.ret(0, int(g['disbanded']))

    def attack_descriptor(self, m):
        self.u.mem_write(m.ecx(), struct.pack('<I', SENTINEL) * 12)
        m.event('AttackDescriptor', m.arg(0))
        m.ret(4)

    def descriptor(self, p):
        source = self.actor_key[self.r32(p + 8)]
        target = self.r32(p + 0x1c)
        point = self.r32(p + 0x20, 3)
        if target != SENTINEL:
            assert point == [SENTINEL] * 3
            return [source, 1, self.actor_key[target], 0, 0]
        assert SENTINEL not in point
        return [source, 2] + point

    def ability_check(self, m):
        assert m.ecx() == self.ability
        m.event('AbilityCheck', 1, m.arg(1), *self.descriptor(m.arg(0)))
        m.ret(8, int(self.response('AbilityCheck')['value']))

    def fire_ability(self, m):
        assert m.ecx() == self.combat
        m.event('FireAbility', 1 if m.arg(0) == self.ability else 0, *self.descriptor(m.arg(1)))
        m.ret(8)

    def diagnostic(self, m):
        assert [m.arg(i) for i in (0, 1, 3, 4)] == [0xb38b70, 0xa3afc8, 0xa3b010, 0xa3b050]
        m.event('Diagnostic', m.arg(2), m.arg(5))
        m.ret(0)

    def leave_formation(self, m):
        assert m.ecx() == self.aircraft
        m.event('LeaveFormation')
        self.mutate(self.aircraft, self.response('LeaveFormation')['mutation'])
        m.ret(0)

    def follow_path(self, m):
        assert m.ecx() == self.aircraft + 0x28
        leader = self.actor_key[self.r32(m.arg(1) - 0x28 + 4)]
        m.event('FollowPath', *self.r32(m.arg(0), 7), leader, m.arg(2), m.arg(3), m.arg(4), m.arg(5))
        self.mutate(self.aircraft, self.response('FollowPath')['mutation'])
        m.ret(0x18, 0)

    def build_leg(self, m):
        assert m.ecx() == self.aircraft + 0x28
        m.event('BuildLeg', *self.r32(m.arg(0), 7), m.arg(2), m.arg(3), m.arg(4), *self.r32(m.arg(5), 3),
                m.arg(6), m.arg(7))
        r = self.response('BuildLeg')
        self.w32(m.arg(1), *r['out'])
        self.mutate(self.aircraft, r['mutation'])
        m.ret(0x20)

    def commit_pose(self, m):
        assert m.ecx() == self.mover
        m.event('CommitPose', *[m.arg(i) for i in range(7)])
        m.ret(0x1c, 1)

    def ground_height(self, m):
        assert m.ecx() == self.gworld
        x, y = self.r32(m.arg(0), 2)
        m.event('GroundHeight', x, y, m.arg(1))
        m.ret_float(int(self.response('GroundHeight')['f64'], 16), 'f64', 8)

    def free(self, m):
        p = m.arg(0)
        assert p in self.segments, 'free of a non-segment block'
        self.segments.discard(p)
        m.ret(0)

    # ---- operations ------------------------------------------------------
    def pose(self, words):
        p = self.alloc(0x20)
        self.w32(p, *words)
        return p

    def capture_st0(self, entry, ecx, args):
        out = self.alloc(16)
        stub = self.alloc(64)
        code = b''.join(b'\x68' + struct.pack('<I', a) for a in reversed(args))
        code += b'\xb9' + struct.pack('<I', ecx)
        site = stub + len(code)
        code += b'\xe8' + struct.pack('<i', entry - (site + 5)) + b'\xdb\x3d' + struct.pack('<I', out) + b'\xc3'
        self.u.mem_write(stub, code)
        self.call(stub, 0, (), count=60_000_000)
        assert self.final_esp == 0x2f08004
        return bytes(self.u.mem_read(out, 10)).hex()

    def run(self):
        c = self.c
        op, a = c['operation'], c.get('args', {})
        out = {}
        if op == 'evaluate':
            basis = self.alloc(64)
            self.w32(basis, *a['basis'])
            pts = [self.pose(a['control'][3 * i:3 * i + 3]) for i in range(4)]
            res = self.alloc(16)
            self.call(0x55da10, basis, pts + [res, a['t']])
            assert self.final_esp == 0x2f08004 + 0x18
            out['point'] = self.r32(res, 3)
        elif op == 'arc_length':
            basis = self.alloc(64)
            self.w32(basis, *a['basis'])
            seg = self.alloc(0x5c)
            s = a['segment']
            self.w32(seg, SEGMENT_VTABLE, basis, *s['control'])
            self.w32(seg + 0x38, s['length'], s['field_3c'], *s['end_direction'])
            self.w32(seg + 0x4c, s['end_speed'], s['start_speed'], s['tolerance'], 0)
            out['length_f80'] = self.capture_st0(0x55dca0, seg, [a['t0'], a['t1']])
        elif op == 'advance_path':
            pin, pout = self.pose(a['pose']), self.alloc(0x20)
            out['remaining_f80'] = self.capture_st0(0x55e2f0, self.aircraft + 0x28, [a['delta'], pin, pout])
            out['pose'] = self.r32(pout, 7)
            assert self.r32(pin, 7) == a['pose']
        elif op == 'move_tick':
            self.call(0x5612f0, self.aircraft, [a['delta']], count=60_000_000)
            assert self.final_esp == 0x2f08004 + 4
        elif op == 'plan_circle':
            self.call(0x560050, self.aircraft + 0x28, [self.pose(a['pose'])], count=10_000_000)
            assert self.final_esp == 0x2f08004 + 4
        elif op == 'reset_members':
            self.call(0x55ebe0, self.aircraft)
            assert self.final_esp == 0x2f08004
        elif op == 'follow_leader':
            self.call(0x561080, self.aircraft, [self.pose(a['pose']), a['delta']])
            assert self.final_esp == 0x2f08004 + 8
        elif op == 'formation_offset':
            res = self.alloc(16)
            self.w32(res, 0x7fc00001, 0x7fc00002, 0x7fc00003)
            self.call(0x5685e0, self.group_ptr[a['group']], [res, a['handle'], a['relative']])
            assert self.final_esp == 0x2f08004 + 12
            out['offset'] = self.r32(res, 3)
        elif op == 'leader':
            eax = self.call(0x5685b0, self.group_ptr[a['group']])
            assert self.final_esp == 0x2f08004
            out['leader'] = self.actor_key[eax] if eax else 0
        elif op == 'leader_is':
            eax = self.call(0x567ff0, self.group_ptr[a['group']], [a['handle']])
            assert self.final_esp == 0x2f08004 + 4
            out['return_al'] = eax & 255
        elif op == 'following_path':
            out['return_al'] = self.call(0x516b10, self.aircraft) & 255
        elif op == 'clear_path':
            self.call(0x55e1b0, self.aircraft + 0x28)
        elif op == 'set_mover_speed':
            self.call(0x576d20, self.mover, [a['speed'], a['factor']])
            assert self.final_esp == 0x2f08004 + 8
        else:
            raise ValueError(op)
        assert self.fpu_top() == 0, 'unbalanced x87 stack'
        if hasattr(self, 'aircraft'):
            out['aircraft'] = self.read_aircraft(self.aircraft)
            out['others'] = [{'key': o['key'], 'aircraft': self.read_aircraft(self.aircraft_ptr[o['key']])}
                             for o in c.get('others', []) if o['aircraft'] is not None]
        if hasattr(self, 'actor'):
            out['mover'] = self.r32(self.mover + 0x24, 2) + [self.r32(self.mover + 0x50)]
            out['actor_scale'] = self.r32(self.actor + 0x3c)
            out['actor_position'] = self.r32(self.actor + 0x20, 3)
            out['actor_facing'] = self.r32(self.actor + 0x2c, 3)
        out['events'] = self.events
        return out


# ---- case generation -----------------------------------------------------
def unit(v):
    n = math.sqrt(sum(x * x for x in v))
    return [x / n for x in v]


def bezier(t, p):
    u = 1 - t
    return [u ** 3 * p[0][i] + 3 * u * u * t * p[1][i] + 3 * u * t * t * p[2][i] + t ** 3 * p[3][i] for i in range(3)]


def segment(p0, d0, p3, d3, s0, s1, tolerance=TENTH, length_scale=1.0, bend=1.0 / 3.0, length=None):
    k = math.dist(p0, p3) * bend
    p1 = [p0[i] + d0[i] * k for i in range(3)]
    p2 = [p3[i] - d3[i] * k for i in range(3)]
    pts = [p0, p1, p2, p3]
    if length is None:
        samples = [bezier(i / 64, pts) for i in range(65)]
        length = sum(math.dist(samples[i], samples[i + 1]) for i in range(64)) * length_scale
    return {'control': fb(*p0, *p1, *p2, *p3), 'length': f2b(length), 'field_3c': f2b(length * 0.97),
            'end_direction': fb(*d3), 'end_speed': f2b(s1), 'start_speed': f2b(s0), 'tolerance': tolerance}


def chain(rng, start, direction, count, scale=1.0, **kw):
    out, p, d = [], list(start), unit(direction)
    for _ in range(count):
        heading = unit([d[0] + rng.uniform(-0.8, 0.8), d[1] + rng.uniform(-0.8, 0.8), rng.uniform(-0.15, 0.15)])
        q = [p[i] + heading[i] * rng.uniform(6, 40) * scale for i in range(3)]
        e = unit([heading[0] + rng.uniform(-0.5, 0.5), heading[1] + rng.uniform(-0.5, 0.5),
                  rng.uniform(-0.1, 0.1)])
        args = dict(kw)
        args.setdefault('length_scale', rng.choice([1.0, 1.0, 1.0, 0.8, 1.25, 0.5, 2.0]))
        out.append(segment(p, d, q, e, rng.uniform(4, 30), rng.uniform(4, 30), **args))
        p, d = q, e
    return out


def path(segments=(), **kw):
    p = {'planning_speed': f2b(12.0), 'turn_radius': f2b(8.0), 't': 0, 'basis': list(BEZIER),
         'altitude': f2b(20.0), 'leg_speed': f2b(12.0), 'total_length': f2b(55.5), 'speed': f2b(10.0),
         'segments': list(segments)}
    p.update(kw)
    return p


def aircraft(segments=(), path_kw=None, **kw):
    a = {'on_ground': 0, 'cruise_speed': f2b(12.0), 'circling': 0, 'attack_target': 0,
         'attack_point': fb(0.0, 0.0, 0.0), 'ability': 0, 'groups': [0, 0, 0],
         'path': path(segments, **(path_kw or {}))}
    a.update(kw)
    return a


def actor(**kw):
    a = {'handle': 0x00030001, 'flags': 0, 'position': fb(100.0, 80.0, 25.0), 'facing': fb(0.6, 0.8, 0.0),
         'scale': f2b(1.0), 'has_mind': True, 'mind_scheduled': True, 'ability_word_54': 0,
         'definition': {'taxi_scale': f2b(0.6), 'taxi_height': f2b(2.0), 'operational_height': f2b(20.0)}}
    a.update(kw)
    return a


HANDLES = [[1, 3, 1], [2, 5, 2], [3, 1, 3], [4, 2, 4], [5, 9, 2], [6, 4, 0]]
H = {1: 0x00030001, 2: 0x00050002, 3: 0x00010003, 4: 0x00020004, 'stale': 0x00080005, 'null': 0x00040006}


def others(rng, flags4=1, extra_paths=True):
    rows = []
    for key, flags in [(2, 0), (3, 0), (4, flags4)]:
        segs = chain(rng, [50.0 + 10 * key, 60.0, 22.0], [1.0, 0.2, 0.0], 1) if extra_paths else []
        rows.append({'key': key, 'handle': H[key], 'flags': flags,
                     'aircraft': aircraft(segs, on_ground=1 if key == 3 and rng.random() < 0.3 else 0,
                                          path_kw={'t': f2b(0.25)})})
    return rows


def groups(rng, leader=2, layout=True, disbanded=(False, False, False), members_c=None):
    def lay(n):
        return {'slot_word': 3, 'offsets': [fb(rng.uniform(-20, 20), rng.uniform(-20, 20), rng.uniform(-3, 3))
                                            for _ in range(n)]} if layout else None
    a_members = [H[1], H[2], H[3], H[4], H['stale'], H['null']]
    b_members = [H[leader] if leader in H else leader, H[1], H[3]]
    c_members = members_c if members_c is not None else [H[3], H[4], H[1]]
    return [
        {'key': 1, 'disbanded': disbanded[0], 'members': a_members, 'layout': lay(6),
         'anchor': fb(rng.uniform(-5, 5), rng.uniform(-5, 5), rng.uniform(-1, 1))},
        {'key': 2, 'disbanded': disbanded[1], 'members': b_members, 'layout': lay(3),
         'anchor': fb(rng.uniform(-5, 5), rng.uniform(-5, 5), rng.uniform(-1, 1))},
        {'key': 3, 'disbanded': disbanded[2], 'members': c_members, 'layout': lay(3),
         'anchor': fb(0.5, -0.5, 0.0)},
    ]


def pose_words(position, direction, speed):
    return fb(*position, *direction, speed)


def rand_pose(rng):
    return pose_words([rng.uniform(10, 240), rng.uniform(10, 180), rng.uniform(5, 40)],
                      unit([rng.uniform(-1, 1), rng.uniform(-1, 1), rng.uniform(-0.2, 0.2)]), rng.uniform(1, 30))


def leg_response(rng, append=0, around=None):
    p = around or [rng.uniform(20, 230), rng.uniform(20, 170), rng.uniform(10, 40)]
    d = unit([rng.uniform(-1, 1), rng.uniform(-1, 1), 0.0])
    q = [p[0] + rng.uniform(-30, 30), p[1] + rng.uniform(-30, 30), p[2] + rng.uniform(-3, 3)]
    segs = [segment(p, d, q, unit([rng.uniform(-1, 1), rng.uniform(-1, 1), 0.0]), rng.uniform(5, 20),
                    rng.uniform(5, 20)) for _ in range(append)]
    words = [[0x88, f2b(rng.uniform(10, 200))]] if append else []
    return {'out': pose_words(q, d, rng.uniform(5, 20)),
            'mutation': {'append': segs, 'words': words}}


def f64h(x):
    return hex(d2b(x))


def base(name, op, **kw):
    c = {'name': name, 'operation': op, 'precision': 'Double53'}
    c.update(kw)
    return c


def math_cases(rng):
    rows = []
    for i in range(40):
        basis = list(BEZIER) if i % 3 else fb(*[rng.uniform(-4, 4) for _ in range(16)])
        ctrl = fb(*[rng.uniform(-200, 200) for _ in range(12)])
        t = rng.choice([0.0, 1.0, 0.5, rng.random(), rng.random(), rng.uniform(-0.5, 1.5), 1.0 / 3.0])
        rows.append(base(f'evaluate_{i}', 'evaluate', args={'basis': basis, 'control': ctrl, 't': f2b(t)}))
    rows.append(base('evaluate_tiny', 'evaluate', args={'basis': list(BEZIER),
                                                        'control': fb(*[1e-30 * (k + 1) for k in range(12)]),
                                                        't': f2b(0.375)}))
    rows.append(base('evaluate_mixed_scale', 'evaluate', args={'basis': list(BEZIER),
                                                               'control': fb(1e6, -3e-4, 7.0, 1.0, 2e5, -1e-6,
                                                                             3.0, 4.0, 5.0, -1e7, 1e-7, 2.5),
                                                               't': f2b(0.1234567)}))
    for i in range(44):
        p0 = [rng.uniform(0, 250), rng.uniform(0, 190), rng.uniform(5, 40)]
        d0 = unit([rng.uniform(-1, 1), rng.uniform(-1, 1), rng.uniform(-0.2, 0.2)])
        p3 = [p0[k] + rng.uniform(-60, 60) for k in range(3)]
        d3 = unit([rng.uniform(-1, 1), rng.uniform(-1, 1), rng.uniform(-0.2, 0.2)])
        tol = rng.choice([TENTH, TENTH, TENTH, f2b(0.5), f2b(2.0), f2b(0.02)])
        s = segment(p0, d0, p3, d3, 10.0, 12.0, tolerance=tol, bend=rng.choice([1 / 3, 0.8, 1.5, 0.1]))
        basis = list(BEZIER) if i % 4 else fb(*[rng.uniform(-3, 3) for _ in range(16)])
        t0 = rng.choice([0.0, rng.random(), rng.uniform(0, 0.5)])
        t1 = rng.choice([1.0, rng.random(), rng.uniform(0.5, 1.0), t0])
        rows.append(base(f'arc_{i}', 'arc_length', args={'basis': basis, 'segment': s, 't0': f2b(t0),
                                                         't1': f2b(t1)}))
    straight = segment([0.0, 0.0, 10.0], [1.0, 0.0, 0.0], [30.0, 0.0, 10.0], [1.0, 0.0, 0.0], 5.0, 5.0)
    rows.append(base('arc_straight', 'arc_length', args={'basis': list(BEZIER), 'segment': straight,
                                                         't0': 0, 't1': f2b(1.0)}))
    rows.append(base('arc_reversed', 'arc_length', args={'basis': list(BEZIER), 'segment': straight,
                                                         't0': f2b(0.9), 't1': f2b(0.2)}))
    degenerate = segment([3.0, 4.0, 5.0], [1.0, 0.0, 0.0], [3.0, 4.0, 5.0], [1.0, 0.0, 0.0], 5.0, 5.0)
    rows.append(base('arc_degenerate', 'arc_length', args={'basis': list(BEZIER), 'segment': degenerate,
                                                           't0': 0, 't1': f2b(1.0)}))
    return rows


def advance_cases(rng):
    rows = []
    for i in range(110):
        start = [rng.uniform(30, 200), rng.uniform(30, 160), rng.uniform(10, 35)]
        d = unit([rng.uniform(-1, 1), rng.uniform(-1, 1), 0.0])
        n = rng.choice([1, 1, 2, 2, 3, 4])
        segs = chain(rng, start, d, n, tolerance=rng.choice([TENTH, TENTH, TENTH, f2b(0.5), f2b(0.03)]))
        if i % 9 == 0:
            for s in segs:
                s['start_speed'] = f2b(rng.uniform(0.0, 0.12))
        if i % 11 == 0:
            segs[0]['length'] = f2b(rng.uniform(0.0, 0.009))
        if i % 13 == 0:
            segs[0]['length'] = 0
        basis = list(BEZIER) if i % 17 else fb(*[b2f(x) * rng.uniform(0.9, 1.1) for x in BEZIER])
        t = rng.choice([0.0, 0.0, rng.uniform(0, 0.95), 0.999, 0.5])
        delta = rng.choice([rng.uniform(0.005, 0.2), rng.uniform(0.2, 1.5), rng.uniform(1.5, 6.0)])
        dir_in = unit([rng.uniform(-1, 1), rng.uniform(-1, 1), rng.uniform(-0.1, 0.1)])
        rows.append(base(f'advance_{i}', 'advance_path', aircraft=aircraft(segs, path_kw={'t': f2b(t), 'basis': basis}),
                         args={'delta': f2b(delta), 'pose': pose_words(start, dir_in, rng.uniform(3, 20))}))
    straight = segment([0.0, 0.0, 10.0], [1.0, 0.0, 0.0], [30.0, 0.0, 10.0], [1.0, 0.0, 0.0], 5.0, 5.0)
    edge = [
        ('advance_empty', [], 0.0, 0.5, [1.0, 0.0, 0.0]),
        ('advance_zero_delta', [straight], 0.0, 0.0, [1.0, 0.0, 0.0]),
        ('advance_negative_delta', [straight], 0.0, -0.25, [1.0, 0.0, 0.0]),
        ('advance_opposed_direction', [straight], 0.2, 0.25, [-1.0, 0.0, 0.0]),
        ('advance_t_one', [straight], 1.0, 0.25, [1.0, 0.0, 0.0]),
        ('advance_exact_end', [straight], 0.5, 3.0, [1.0, 0.0, 0.0]),
        ('advance_t2_beyond', [straight], 0.8, 0.9, [1.0, 0.0, 0.0]),
        ('advance_degenerate', [segment([3.0, 4.0, 5.0], [1.0, 0.0, 0.0], [3.0, 4.0, 5.0], [0.0, 1.0, 0.0],
                                        5.0, 5.0, length=1.0)], 0.3, 0.05, [0.0, 0.0, 1.0]),
        ('advance_zero_length', [dict(straight, length=0)], 0.3, 0.05, [1.0, 0.0, 0.0]),
        ('advance_negative_length', [dict(straight, length=f2b(-30.0))], 0.6, 0.05, [1.0, 0.0, 0.0]),
        ('advance_slow_clamp', [dict(straight, start_speed=0, end_speed=0)], 0.3, 2.0, [1.0, 0.0, 0.0]),
        ('advance_huge_delta', [straight, straight], 0.0, 1000.0, [1.0, 0.0, 0.0]),
        ('advance_denormal_length', [dict(straight, length=f2b(1e-40))], 0.3, 0.5, [1.0, 0.0, 0.0]),
        ('advance_denormal_length_two', [dict(straight, length=f2b(-1e-40)), straight], 0.3, 0.5, [0.0, 1.0, 0.0]),
    ]
    for name, segs, t, delta, d in edge:
        rows.append(base(name, 'advance_path', aircraft=aircraft(segs, path_kw={'t': f2b(t)}),
                         args={'delta': f2b(delta), 'pose': pose_words([1.0, 0.0, 10.0], d, 7.5)}))
    # A negative denormal length gives dt = -inf: the original then evaluates
    # the curve at t = -inf (NaN pose). Rust must refuse it (NonFinite).
    rows[-1]['unsupported'] = 'nonfinite'
    assert rows[-1]['name'] == 'advance_denormal_length_two'
    return rows


def tick_world(rng, formation=False, leader=2, layout=True, disbanded=(False, False, False), members_c=None):
    w = {'others': others(rng), 'handles': HANDLES}
    if formation:
        w['groups'] = groups(rng, leader=leader, layout=layout, disbanded=disbanded, members_c=members_c)
    return w


def tick_case(rng, name, *, empty=False, formation=False, ability=False, **opts):
    pos = [rng.uniform(30, 220), rng.uniform(30, 160), rng.uniform(8, 35)]
    facing = [rng.uniform(-1, 1), rng.uniform(-1, 1), rng.uniform(-0.3, 0.3)]
    segs = [] if empty else chain(rng, pos, [facing[0], facing[1], 0.0], rng.choice([1, 2, 3]))
    on_ground = opts.get('on_ground', int(rng.random() < 0.25))
    w = tick_world(rng, formation=formation, leader=opts.get('leader', 2), layout=opts.get('layout', True),
                   disbanded=opts.get('disbanded', (False, False, False)), members_c=opts.get('members_c'))
    groups_ref = opts.get('groups_ref', [1, 2, 3] if formation else [0, 0, 0])
    ac = aircraft(segs, on_ground=on_ground, circling=opts.get('circling', int(rng.random() < 0.3)),
                  cruise_speed=f2b(rng.uniform(5, 25)), groups=groups_ref,
                  path_kw={'t': f2b(rng.choice([0.0, rng.uniform(0, 0.9)])), 'speed': f2b(rng.uniform(4, 20)),
                           'turn_radius': f2b(rng.uniform(4, 30)), 'altitude': f2b(rng.uniform(10, 40))})
    if ability:
        ac['ability'] = 1
        ac['attack_target'] = opts.get('target', rng.choice([H[2], H[3], H['stale'], H['null'], 0]))
        ac['attack_point'] = fb(rng.uniform(0, 250), rng.uniform(0, 190), rng.uniform(0, 10))
    taxi = rng.uniform(0, 6)
    operational = opts.get('operational', taxi + rng.uniform(5, 40))
    a = actor(position=fb(*pos), facing=fb(*facing), has_mind=opts.get('has_mind', True),
              mind_scheduled=opts.get('mind_scheduled', rng.random() < 0.5),
              ability_word_54=rng.choice([0, 8, 0xfffffff7, 0xffffffff, 0x10]),
              definition={'taxi_scale': f2b(rng.uniform(0.2, 1.0)), 'taxi_height': f2b(opts.get('taxi', taxi)),
                          'operational_height': f2b(operational)})
    ground = opts.get('ground', pos[2] - rng.choice([rng.uniform(-5, 2), rng.uniform(2, 50), rng.uniform(50, 90)]))
    responses = {
        'MindBlocked': [{'value': opts.get('blocked', False)}],
        'AbilityCheck': [{'value': opts.get('check', rng.choice([0, 0, 1, 0x100]))}],
        'ActorSpeed': [{'f64': f64h(rng.uniform(2, 30) + 2 ** -40)}],
        'FormationSpeed': [{'f64': f64h(opts.get('formation_speed', rng.uniform(4, 40) + 2 ** -37))}],
        'LeaveFormation': [{'mutation': opts.get('leave', {})}],
        'FollowPath': [{'mutation': opts.get('follow', {'append': chain(rng, pos, facing[:2] + [0.0],
                                                                        rng.choice([0, 1, 2]))})}],
        'BuildLeg': [leg_response(rng, append=opts.get('leg_append', rng.choice([0, 0, 1])),
                                  around=pos if i == 0 else None) for i in range(4)],
        'GroundHeight': [{'f64': f64h(ground)}],
    }
    mover = {'requested_speed': f2b(rng.uniform(1, 20)), 'max_speed': f2b(opts.get('max_speed', rng.uniform(5, 30))),
             'speed_factor': f2b(rng.uniform(0.5, 1.5))}
    return base(name, 'move_tick', actor=a, aircraft=ac, mover=mover, responses=responses,
                args={'delta': f2b(opts.get('delta', rng.uniform(0.01, 0.5)))}, **w)


def tick_cases(rng):
    rows = []
    for i in range(60):
        rows.append(tick_case(rng, f'tick_generated_{i}', empty=rng.random() < 0.3,
                              formation=rng.random() < 0.4, ability=rng.random() < 0.3))
    rows.append(tick_case(rng, 'tick_mind_blocked', blocked=True, ability=True))
    rows.append(tick_case(rng, 'tick_no_mind', has_mind=False))
    rows.append(tick_case(rng, 'tick_empty_unscheduled', empty=True, mind_scheduled=False, leg_append=0))
    rows.append(tick_case(rng, 'tick_empty_scheduled_legs', empty=True, mind_scheduled=True, leg_append=1))
    rows.append(tick_case(rng, 'tick_exhausts_path', circling=0, delta=30.0, on_ground=0))
    rows.append(tick_case(rng, 'tick_exhausts_path_ground', circling=0, delta=30.0, on_ground=1))
    for target in (H[2], H[3], H['stale'], H['null'], 0, H[1]):
        for check in (0, 1):
            rows.append(tick_case(rng, f'tick_ability_{target:x}_{check}', ability=True, target=target, check=check))
    rows.append(tick_case(rng, 'tick_formation_follow', formation=True, on_ground=0, circling=0, leader=2))
    rows.append(tick_case(rng, 'tick_formation_follow_circling', formation=True, on_ground=0, circling=1, leader=2))
    rows.append(tick_case(rng, 'tick_formation_follow_empty', formation=True, on_ground=0, empty=True, leader=2))
    rows.append(tick_case(rng, 'tick_formation_self_leader', formation=True, on_ground=0, leader=1))
    rows.append(tick_case(rng, 'tick_formation_stale_leader', formation=True, on_ground=0, leader=H['stale'],
                          circling=1))
    rows.append(tick_case(rng, 'tick_formation_disbanded', formation=True, on_ground=0, disbanded=(False, True, False),
                          leave={'clear': True, 'words': [[0x14, f2b(17.5)]], 'groups': [0, 0, 0]}))
    rows.append(tick_case(rng, 'tick_formation_disbanded_keep', formation=True, on_ground=0,
                          disbanded=(False, True, False), leave={}))
    rows.append(tick_case(rng, 'tick_formation_on_ground', formation=True, on_ground=1))
    rows.append(tick_case(rng, 'tick_formation_null_b', formation=True, on_ground=0, groups_ref=[1, 0, 3]))
    rows.append(tick_case(rng, 'tick_formation_same_groups', formation=True, on_ground=0, circling=1,
                          groups_ref=[2, 2, 3]))
    rows.append(tick_case(rng, 'tick_formation_no_layout', formation=True, on_ground=0, circling=1, layout=False))
    rows.append(tick_case(rng, 'tick_formation_speed_clamped', formation=True, on_ground=0, circling=1,
                          max_speed=3.0, formation_speed=40.0))
    rows.append(tick_case(rng, 'tick_formation_follow_nothing', formation=True, on_ground=0, circling=1,
                          follow={}, empty=False))
    rows.append(tick_case(rng, 'tick_band_zero_positive', on_ground=1, taxi=5.0, operational=5.0, ground=0.0))
    rows.append(tick_case(rng, 'tick_band_zero_negative', on_ground=1, taxi=5.0, operational=5.0, ground=500.0))
    rows.append(tick_case(rng, 'tick_band_zero_air', on_ground=0, taxi=5.0, operational=5.0, ground=0.0))
    rows.append(tick_case(rng, 'tick_band_denormal_up', on_ground=1, taxi=0.0, operational=1e-40, ground=-100.0))
    rows.append(tick_case(rng, 'tick_band_denormal_down', on_ground=1, taxi=0.0, operational=1e-40, ground=1000.0))
    rows.append(tick_case(rng, 'tick_band_negative_denormal', on_ground=1, taxi=0.0, operational=-1e-40, ground=-100.0))
    for g in (-100.0, 0.0, 5.0, 1000.0):
        rows.append(tick_case(rng, f'tick_ground_{g}', on_ground=1, ground=g))
    return rows


def special_band_nan(rng):
    """(z - h) - taxi == 0 and operational == taxi: 0/0 on the ground."""
    c = tick_case(rng, 'tick_band_zero_indefinite', on_ground=1, taxi=5.0, operational=5.0, empty=False)
    c['responses']['GroundHeight'] = [{'f64': 'PLACEHOLDER'}]
    return c


def formation_cases(rng):
    rows = []
    for i in range(10):
        w = tick_world(rng, formation=True, leader=rng.choice([2, 2, 1, 3]),
                       members_c=rng.choice([None, [H[1], H[3]], [H[2]], []]))
        ac = aircraft(chain(rng, [100.0, 90.0, 20.0], [1.0, 0.0, 0.0], 1), groups=rng.choice([[1, 2, 3], [1, 2, 0],
                                                                                               [0, 2, 3], [1, 0, 3]]),
                      on_ground=int(rng.random() < 0.2))
        rows.append(base(f'reset_members_{i}', 'reset_members', actor=actor(flags=rng.choice([0, 1])),
                         aircraft=ac, mover={'requested_speed': 0, 'max_speed': 0, 'speed_factor': 0}, **w))
    # Deterministic: this aircraft leads +0xAC, so both member loops run.
    for i, (refs, members_c, on_ground, flags) in enumerate([
            ([1, 2, 3], None, 0, 0), ([1, 2, 3], [H['stale'], H['null'], H[2], H[1]], 0, 0),
            ([1, 2, 0], None, 0, 0), ([1, 2, 3], None, 1, 0), ([1, 2, 3], None, 0, 1),
            ([2, 2, 2], [H[1], H[2]], 0, 0)]):
        w = tick_world(rng, formation=True, leader=1, members_c=members_c)
        ac = aircraft(chain(rng, [100.0, 90.0, 20.0], [1.0, 0.0, 0.0], 2), groups=refs, on_ground=on_ground)
        rows.append(base(f'reset_members_leader_{i}', 'reset_members', actor=actor(flags=flags), aircraft=ac,
                         mover={'requested_speed': 0, 'max_speed': 0, 'speed_factor': 0}, **w))
    w = tick_world(rng, formation=True, leader=1, members_c=[])
    w['groups'][0]['members'] = []
    rows.append(base('reset_members_empty_lists', 'reset_members', actor=actor(),
                     aircraft=aircraft(chain(rng, [100.0, 90.0, 20.0], [1.0, 0.0, 0.0], 1), groups=[1, 2, 3]),
                     mover={'requested_speed': 0, 'max_speed': 0, 'speed_factor': 0}, **w))
    # The leader resolves through registry slot 7 (generation 6) but its own
    # actor+0x14 handle carries generation 7: 0x561080 rejects it.
    w = tick_world(rng, formation=True)
    w['others'].append({'key': 5, 'handle': 0x00070007, 'flags': 0,
                        'aircraft': aircraft(chain(rng, [70.0, 60.0, 20.0], [1.0, 0.0, 0.0], 1))})
    w['handles'] = HANDLES + [[7, 6, 5]]
    w['groups'][1]['members'][0] = 0x00060007
    rows.append(base('follow_leader_handle_mismatch', 'follow_leader', actor=actor(), aircraft=aircraft([], groups=[1, 2, 3]),
                     mover={'requested_speed': 0, 'max_speed': 0, 'speed_factor': 0},
                     args={'pose': rand_pose(rng), 'delta': f2b(0.1)}, **w))
    w = tick_world(rng, formation=True)
    w['groups'][0]['layout']['slot_word'] = 0
    rows.append(base('formation_offset_zero_slot_word', 'formation_offset', actor=actor(), aircraft=aircraft(),
                     mover={'requested_speed': 0, 'max_speed': 0, 'speed_factor': 0},
                     args={'group': 1, 'handle': H[1], 'relative': 1}, **w))
    w = tick_world(rng, formation=True)
    w['groups'][2]['members'] = []
    rows.append(base('formation_offset_empty_group', 'formation_offset', actor=actor(), aircraft=aircraft(),
                     mover={'requested_speed': 0, 'max_speed': 0, 'speed_factor': 0},
                     args={'group': 3, 'handle': 0, 'relative': 1}, **w))
    for i in range(14):
        leader = rng.choice([2, 2, 3, 1, H['stale'], 4])
        w = tick_world(rng, formation=True, leader=leader, layout=rng.random() < 0.85,
                       disbanded=(False, rng.random() < 0.15, False))
        if i % 5 == 4:
            w['others'][0]['aircraft']['path']['segments'] = []
        refs = rng.choice([[1, 2, 3], [2, 2, 0], [1, 2, 0], [3, 2, 1]])
        ac = aircraft([], groups=refs)
        rows.append(base(f'follow_leader_{i}', 'follow_leader', actor=actor(),
                         aircraft=ac, mover={'requested_speed': 0, 'max_speed': 0, 'speed_factor': 0},
                         responses={'FollowPath': [{'mutation': {'append': chain(rng, [90.0, 90.0, 20.0],
                                                                                 [0.0, 1.0, 0.0], 1)}}]},
                         args={'pose': rand_pose(rng), 'delta': f2b(rng.uniform(0.01, 0.3))}, **w))
    for i in range(12):
        w = tick_world(rng, formation=True, layout=i % 6 != 5)
        if i == 4:
            w['groups'][0]['layout']['slot_word'] = 0
        handle = rng.choice([H[1], H[2], H[3], H['null'], 0x12345678])
        rows.append(base(f'formation_offset_{i}', 'formation_offset', actor=actor(), aircraft=aircraft(),
                         mover={'requested_speed': 0, 'max_speed': 0, 'speed_factor': 0},
                         args={'group': rng.choice([1, 2, 3]), 'handle': handle, 'relative': rng.choice([0, 1, 0x100])},
                         **w))
    for i, (leader, members) in enumerate([(2, None), (1, None), (H['stale'], None), (H['null'], None), (3, None)]):
        w = tick_world(rng, formation=True, leader=leader)
        for g in (1, 2, 3):
            rows.append(base(f'leader_{i}_{g}', 'leader', actor=actor(), aircraft=aircraft(),
                             mover={'requested_speed': 0, 'max_speed': 0, 'speed_factor': 0},
                             args={'group': g}, **w))
            rows.append(base(f'leader_is_{i}_{g}', 'leader_is', actor=actor(), aircraft=aircraft(),
                             mover={'requested_speed': 0, 'max_speed': 0, 'speed_factor': 0},
                             args={'group': g, 'handle': rng.choice([H[2], H[1], H['stale'], H[3]])}, **w))
    w = tick_world(rng, formation=True)
    w['groups'][1]['members'] = []
    rows.append(base('leader_empty_group', 'leader', actor=actor(), aircraft=aircraft(),
                     mover={'requested_speed': 0, 'max_speed': 0, 'speed_factor': 0}, args={'group': 2}, **w))
    rows.append(base('leader_is_empty_group', 'leader_is', actor=actor(), aircraft=aircraft(),
                     mover={'requested_speed': 0, 'max_speed': 0, 'speed_factor': 0},
                     args={'group': 2, 'handle': 0}, **w))
    return rows


def small_cases(rng):
    rows = []
    for i in range(8):
        segs = chain(rng, [50.0, 50.0, 20.0], [0.0, 1.0, 0.0], i % 3)
        rows.append(base(f'clear_path_{i}', 'clear_path', actor=actor(), aircraft=aircraft(
            segs, path_kw={'t': f2b(0.3), 'total_length': f2b(99.0)}),
            mover={'requested_speed': 0, 'max_speed': 0, 'speed_factor': 0}))
        rows.append(base(f'following_path_{i}', 'following_path', actor=actor(),
                         aircraft=aircraft(segs, circling=i % 2),
                         mover={'requested_speed': 0, 'max_speed': 0, 'speed_factor': 0}))
    for i, (speed, limit) in enumerate([(5.0, 10.0), (10.0, 10.0), (12.0, 10.0), (-1.0, 0.0), (0.0, -0.0),
                                        (1e-40, 0.0), (3.4e38, 1.0), (7.25, 7.2500005)]):
        rows.append(base(f'set_mover_speed_{i}', 'set_mover_speed', actor=actor(), aircraft=aircraft(),
                         mover={'requested_speed': f2b(99.0), 'max_speed': f2b(limit), 'speed_factor': f2b(2.0)},
                         args={'speed': f2b(speed), 'factor': f2b(0.75 + i)}))
    for i in range(10):
        pose = rand_pose(rng)
        if i == 0:
            pose[3:6] = fb(0.0, 0.0, 1.0)
        rows.append(base(f'plan_circle_{i}', 'plan_circle', actor=actor(), aircraft=aircraft(
            path_kw={'turn_radius': f2b(rng.uniform(3, 40)), 'altitude': f2b(rng.uniform(5, 50))}),
            mover={'requested_speed': 0, 'max_speed': 0, 'speed_factor': 0},
            responses={'BuildLeg': [leg_response(rng, append=rng.choice([0, 1])) for _ in range(4)]},
            args={'pose': pose}))
    return rows


def run_case(c):
    m = Aircraft(c)
    try:
        out = m.run()
    except NullDereference as error:
        out = {'null_dereference': str(error), 'events': m.events}
    # Keep only the responses the original consumed (generation supplies extras).
    if 'responses' in c:
        c['responses'] = {k: v[:m.used[k]] for k, v in c['responses'].items() if m.used.get(k)}
    return out


def main():
    rng = random.Random(0xa1c5a7)
    templates = math_cases(rng) + advance_cases(rng) + tick_cases(rng) + formation_cases(rng) + small_cases(rng)
    nan_case = special_band_nan(rng)
    cases = []
    for precision in PRECISION:
        for t in templates:
            c = copy.deepcopy(t)
            c['precision'] = precision
            c['output'] = run_case(c)
            cases.append(c)
        # Make (z - ground) - taxi_height exactly zero: ground = z_out - taxi.
        c = copy.deepcopy(nan_case)
        c['precision'] = precision
        probe = copy.deepcopy(c)
        probe['responses']['GroundHeight'] = [{'f64': f64h(0.0)}]
        events = run_case(probe)['events']
        commit = next(e for e in events if e['name'] == 'CommitPose')
        c['responses']['GroundHeight'] = [{'f64': f64h(b2f(commit['bits'][2]) - 5.0)}]
        c['output'] = run_case(c)
        assert c['output']['actor_scale'] == 0xffc00000, hex(c['output']['actor_scale'])
        cases.append(c)
    counts = {}
    for c in cases:
        counts[c['operation']] = counts.get(c['operation'], 0) + 1
    print(json.dumps(counts))
    write_document('native-aircraft.json', __doc__, cases, {'operation_counts': counts})


if __name__ == '__main__':
    main()
