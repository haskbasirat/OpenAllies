"""Native oracle for the Performer (component +74) turret turn and aim.

Original code runs: the static initializer 0x9C8660 that builds the arctangent
table (0x408F80 / 0x408DE0 with the fpatan virtual 0x409000), the table
arctangent 0x85D9F0 / 0x85D8D0 (with the CRT float-to-integer 0x96CC4C), the
turn step 0x57DEE0, the aim 0x57E730 (with 0x4C76A0, 0x529C30 and the turret
getters 0x45DC10 / 0x45DD10), the reset update 0x57EF00 (with 0x45DC30 /
0x45DCF0) and the stage-0 aim 0x57EE40.

Recorded seams: 0x50C290 (turret yaw changed, on world +130), 0x50AC00 /
0x50B1C0 / 0x50ACC0 (the Updatable at Performer +8), 0x4C76E0 (turret arc
test: arguments recorded, the case decides the result), and during the table
build only 0x6F7810 (allocator) and 0x96CAB0 (atexit registration).

Usage: native_turret_turn.py [AA.exe]. The executable is checked against the
accepted SHA-256 by the shared harness.
"""
from pathlib import Path
import math
import random
import struct
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import harness as h  # noqa: E402
import hashlib  # noqa: E402
if len(sys.argv) > 1:
    # An explicit AA.exe must be the accepted original the harness loads.
    assert hashlib.sha256(Path(sys.argv[1]).read_bytes()).hexdigest() == h.ORIGINAL_SHA, sys.argv[1]
from unicorn.x86_const import UC_X86_REG_ECX  # noqa: E402

TABLE_OBJECT = 0xB39A78
TABLE_ROOT = 0xB39B00
PI = h.f32(math.pi)
TWO_PI = h.f32(2 * math.pi)


def fl(value):
    return h.f32(value)


def fv(bits):
    return h.from_f32(bits)


class Rig:
    """One machine per precision: the table, the registry and the world."""

    def __init__(self, precision):
        self.m = m = h.Machine(precision)
        m.u.mem_map(0, 0x1000)

        def allocate(mm):
            mm.ret(0, mm.alloc(mm.arg(0)))
        m.hooks[0x6F7810] = allocate
        m.hooks[0x96CAB0] = lambda mm: mm.ret(0, 0)
        m.call(0x9C8660, pop=0, limit=20_000_000)
        del m.hooks[0x6F7810]
        del m.hooks[0x96CAB0]
        assert m.r(TABLE_ROOT) == TABLE_OBJECT
        count = m.r(TABLE_OBJECT + 4)
        entries = m.r(TABLE_OBJECT + 8)
        self.table = {'count': count, 'maximum': m.r(TABLE_OBJECT + 0xC),
                      'entries': bytes(m.u.mem_read(entries, 4 * count)).hex()}
        self.registry = m.alloc(0x20004 + 0x10000 * 4)
        m.w(h.REGISTRY_ROOT, self.registry)
        self.world = m.alloc(0x200)
        m.w(h.WORLD_ROOT, self.world)
        self.notify = self.world + 0x130
        m.w(self.world + 0x130, 0x4E4F0000)
        self.events = []
        self.perf = 0
        self.defs = []
        m.hooks[0x50C290] = self._yaw_changed
        m.hooks[0x50AC00] = self._activate
        m.hooks[0x50B1C0] = self._deactivate
        m.hooks[0x50ACC0] = self._post
        m.hooks[0x4C76E0] = self._in_arc
        self.arc = []
        self.slot = 0x100

    # seams -------------------------------------------------------------------
    def _yaw_changed(self, mm):
        assert mm.reg(UC_X86_REG_ECX) == mm.r(self.world + 0x130)
        assert mm.arg(0) == self.actor
        self.events.append({'yaw_changed': mm.arg(1)})
        mm.ret(8)

    def _activate(self, mm):
        assert mm.reg(UC_X86_REG_ECX) == self.perf + 8
        self.events.append({'activate': mm.arg(0)})
        mm.ret(4)

    def _deactivate(self, mm):
        assert mm.reg(UC_X86_REG_ECX) == self.perf + 8
        self.events.append({'deactivate': [mm.arg(0), mm.arg(1)]})
        mm.ret(8)

    def _post(self, mm):
        assert mm.reg(UC_X86_REG_ECX) == self.perf + 8
        self.events.append({'post': [mm.arg(0), mm.arg(1)]})
        mm.ret(8)

    def _in_arc(self, mm):
        index = self.defs.index(mm.reg(UC_X86_REG_ECX))
        assert mm.arg(0) == self.actor + 0x20 and mm.arg(1) == self.actor + 0x2C
        point = mm.arg(2)
        self.events.append({'in_arc': index, 'point': [mm.r(point), mm.r(point + 4)]})
        result = self.arc[index]
        mm.ret(12, result)

    # objects -----------------------------------------------------------------
    def handle(self, kind, position=(0, 0)):
        """kind: none, live, flagged, stale (generation), empty (null slot)."""
        if kind == 'none':
            return 0, 0
        m = self.m
        self.slot += 1
        index = self.slot
        generation = 0x1234 + index
        actor = m.alloc(0x40)
        m.wb(actor + 8, 1 if kind == 'flagged' else 0)
        m.w(actor + 0x20, position[0])
        m.w(actor + 0x24, position[1])
        m.u.mem_write(self.registry + 4 + index * 2, struct.pack('<H', generation))
        m.w(self.registry + 0x20004 + index * 4, 0 if kind == 'empty' else actor)
        stored = generation + 1 if kind == 'stale' else generation
        return ((stored & 0xFFFF) << 16) | index, actor

    def performer(self, case):
        m = self.m
        self.events = []
        m.w(self.world + 0xE0, case['time'])
        m.w(self.world + 0xE8, case['tick'])
        perf = m.alloc(0x80)
        actor = m.alloc(0x40)
        data = m.alloc(0x400)
        self.perf, self.actor = perf, actor
        m.w(perf + 4, actor)
        m.w(actor + 4, data)
        m.w(actor + 0x20, case['position'][0])
        m.w(actor + 0x24, case['position'][1])
        m.w(actor + 0x2C, case['facing'][0])
        m.w(actor + 0x30, case['facing'][1])
        turrets = case['turrets']
        defs = m.alloc(4 * max(len(turrets), 1))
        records = m.alloc(4 * max(len(turrets), 1))
        self.defs, self.records = [], []
        for i, t in enumerate(turrets):
            d = m.alloc(0x40)
            m.w(d + 4, t['yaw_start'])
            m.w(d + 8, t['rate'])
            m.w(d + 0xC, fl(1.0))
            m.w(d + 0x24, t['offset'][0])
            m.w(d + 0x28, t['offset'][1])
            m.w(d + 0x2C, t['reset_time'])
            m.wb(d + 0x30, int(t['can_reset']))
            m.w(defs + 4 * i, d)
            self.defs.append(d)
            r = m.alloc(0x20)
            rec = t['record']
            for k, off in enumerate((0, 4, 8)):
                m.w(r + off, rec[k])
            m.wb(r + 0xC, rec[3])
            m.w(r + 0x10, rec[4])
            m.w(r + 0x14, rec[5])
            m.w(r + 0x18, rec[6])
            m.w(records + 4 * i, r)
            self.records.append(r)
        m.w(data + 0x1F8, defs)
        m.w(data + 0x1FC, len(turrets))
        m.w(perf + 0x4C, records)
        m.wb(perf + 0x5C, int(case['latch']))
        self.arc = case.get('arc', [0] * len(turrets))
        return perf

    def record_after(self):
        m = self.m
        return [[m.r(r), m.r(r + 4), m.r(r + 8), m.rb(r + 0xC), m.r(r + 0x10), m.r(r + 0x14),
                 m.r(r + 0x18)] for r in self.records]


def angle_inputs(rng):
    out = [(1.0, 0.0), (0.0, 1.0), (-1.0, 0.0), (0.0, -1.0), (0.0, 0.0), (-0.0, 0.0), (0.0, -0.0),
           (-0.0, -0.0), (1.0, 1.0), (-1.0, 1.0), (-1.0, -1.0), (1.0, -1.0)]
    eps = [0.01, fv(fl(0.01)) * (1 - 2 ** -23), fv(fl(0.01)) * (1 + 2 ** -23), 0.005, 0.02]
    for e in eps:
        for y in (1.0, 0.5, -1.0, 0.0, 2.0):
            out += [(e, y), (-e, y), (y, e), (y, -e)]
    for ratio in (100.0, 99.99999, 100.00001, 99.9, 0.5, 1.0, 25 / 4096, 50 / 4096):
        out += [(1.0, ratio), (-ratio, 1.0), (-1.0, -ratio), (ratio, -1.0)]
    for _ in range(220):
        t = rng.uniform(-7, 7)
        scale = rng.choice((1.0, 1.0, 1.0, rng.uniform(0.001, 600)))
        out.append((math.cos(t) * scale, math.sin(t) * scale))
    for _ in range(40):
        k = rng.randrange(16384)
        x = rng.choice((1.0, 2.0, 0.75))
        out.append((x, k * 25 / 4096 * x))
    return [(fl(x), fl(y)) for x, y in out]


def turret(rng, *, record=None):
    return {
        'yaw_start': fl(rng.choice((0.0, math.pi / 2, math.pi, rng.uniform(-1, 7)))),
        'rate': fl(rng.choice((0.0, 0.5, 2.0, math.pi, rng.uniform(0.1, 12)))),
        'offset': [fl(rng.choice((0.0, rng.uniform(-3, 3)))), fl(rng.choice((0.0, rng.uniform(-3, 3))))],
        'reset_time': fl(rng.choice((0.0, 1.0, 2.5, rng.uniform(0, 6)))),
        'can_reset': rng.random() < 0.6,
        'record': record or [fl(rng.uniform(0, 2 * math.pi)), fl(rng.uniform(0, 7)), fl(rng.uniform(0, 20)),
                             rng.choice((0, 1)), fl(rng.uniform(-500, 500)), fl(rng.uniform(-500, 500)),
                             rng.getrandbits(32)],
    }


def base_case(rng, turrets):
    t = rng.uniform(-7, 7)
    scale = rng.choice((1.0, 1.0, rng.uniform(0.2, 3)))
    return {
        'time': fl(rng.choice((0.0, 10.5, rng.uniform(0, 400)))),
        'tick': fl(rng.choice((0.1, 0.05, 1 / 30, 0.125, 1.0))),
        'position': [fl(rng.uniform(-500, 500)), fl(rng.uniform(-500, 500))],
        'facing': [fl(math.cos(t) * scale), fl(math.sin(t) * scale)],
        'latch': rng.random() < 0.4,
        'turrets': turrets,
    }


def step_cases(rng):
    pairs = [(0.0, math.pi), (math.pi, 0.0), (0.0, 2 * math.pi), (1.0, 1.0 + math.pi), (1.0, 1.5),
             (1.5, 1.0), (0.25, 6.0), (6.0, 0.25), (3.0, 3.0), (0.0, 0.0), (6.2, 0.1), (0.1, 6.2)]
    out = []
    for current, target in pairs:
        for rate, tick in ((0.5, 1.0), (4.0, 0.125), (0.0, 0.1), (100.0, 0.1), (2.0, 0.05)):
            out.append((fl(current), fl(target), fl(rate), fl(tick)))
    pi = fv(PI)
    for _ in range(4):
        c = fv(fl(rng.uniform(0, 3)))
        out.append((fl(c), fl(c + pi), fl(1.0), fl(0.1)))
        out.append((fl(c + pi), fl(c), fl(1.0), fl(0.1)))
    for _ in range(150):
        out.append((fl(rng.uniform(-0.5, 7)), fl(rng.uniform(-0.5, 7)), fl(rng.uniform(0, 10)),
                    fl(rng.choice((0.1, 0.05, 1 / 30, 0.125, 1.0)))))
    return out


def main():
    rng = random.Random(0x57DEE0)
    fixtures = {'table': None, 'atan': [], 'step': [], 'aim': [], 'reset': [], 'stage0': []}
    visited = {}
    inputs = angle_inputs(rng)
    steps = step_cases(rng)
    aim_cases = []
    for _ in range(180):
        case = base_case(rng, [turret(rng) for _ in range(rng.randint(1, 3))])
        index = rng.randrange(len(case['turrets']))
        t = case['turrets'][index]
        if rng.random() < 0.15:
            # The point on the mount: a zero direction (length <= 1e-4).
            point = 'mount'
        else:
            point = [fl(fv(case['position'][0]) + rng.uniform(-60, 60)),
                     fl(fv(case['position'][1]) + rng.uniform(-60, 60))]
        aim_cases.append((case, index, point, rng.choice(('none', 'live', 'live', 'flagged', 'stale', 'empty'))))
        del t
    reset_cases = []
    for _ in range(90):
        turrets = [turret(rng) for _ in range(rng.randint(0, 4))]
        case = base_case(rng, turrets)
        for t in turrets:
            choice = rng.random()
            if choice < 0.3:
                # elapsed exactly equal to reset_time
                t['record'][2] = fl(fv(case['time']) - fv(t['reset_time']))
            elif choice < 0.5:
                t['record'][2] = case['time']
        case['arc'] = [rng.choice((0, 1, 1)) for _ in turrets]
        reset_cases.append(case)
    stage_cases = []
    for _ in range(70):
        turrets = [turret(rng) for _ in range(rng.randint(1, 3))]
        case = base_case(rng, turrets)
        case['index'] = rng.randrange(len(turrets))
        case['stage'] = rng.choice((0, 1, 2))
        case['handle_kind'] = rng.choice(('none', 'live', 'live', 'flagged', 'stale', 'empty'))
        case['point'] = [fl(rng.uniform(-500, 500)), fl(rng.uniform(-500, 500))]
        case['target_position'] = [fl(fv(case['position'][0]) + rng.uniform(-80, 80)),
                                   fl(fv(case['position'][1]) + rng.uniform(-80, 80))]
        stage_cases.append(case)

    for precision in h.PRECISIONS:
        rig = Rig(precision)
        m = rig.m
        if fixtures['table'] is None:
            fixtures['table'] = rig.table
        assert rig.table == fixtures['table'], 'table differs between precisions'
        for x, y in inputs:
            m.call(0x85D9F0, args=(x, y), pop=0)
            ext = m.st0_ext()
            fixtures['atan'].append({'precision': precision, 'x': x, 'y': y,
                                     'result': [ext['mantissa'], ext['sign_exponent']]})
        for current, target, rate, tick in steps:
            t = {'yaw_start': 0, 'rate': rate, 'offset': [0, 0], 'reset_time': 0, 'can_reset': False,
                 'record': [current, fl(9.0), 0, 0, 0, 0, 0]}
            case = {'time': 0, 'tick': tick, 'position': [0, 0], 'facing': [fl(1.0), 0], 'latch': False,
                    'turrets': [t]}
            perf = rig.performer(case)
            result = m.call(0x57DEE0, ecx=perf, args=(0, target), pop=8) & 0xFF
            fixtures['step'].append({'precision': precision, 'case': case, 'target': target,
                                     'result': result, 'after': rig.record_after(), 'events': rig.events})
        for case, index, point, kind in aim_cases:
            perf = rig.performer(case)
            handle, _ = rig.handle(kind)
            if point == 'mount':
                t = case['turrets'][index]
                # 0x4C76A0 at this precision, read back from a native run of the aim below.
                spot = m.alloc(8)
                m.call(0x4C76A0, ecx=rig.defs[index], args=(spot, rig.actor + 0x2C), pop=8)
                pt = [fl(fv(case['position'][0]) + fv(m.r(spot))), fl(fv(case['position'][1]) + fv(m.r(spot + 4)))]
                del t
            else:
                pt = point
            pp = m.alloc(8)
            m.w(pp, pt[0])
            m.w(pp + 4, pt[1])
            result = m.call(0x57E730, ecx=perf, args=(index, pp, handle), pop=12) & 0xFF
            fixtures['aim'].append({'precision': precision, 'case': case, 'index': index, 'point': pt,
                                    'handle_kind': kind, 'handle': handle, 'result': result,
                                    'after': rig.record_after(), 'latch_after': m.rb(perf + 0x5C),
                                    'events': rig.events})
        for case in reset_cases:
            perf = rig.performer(case)
            m.call(0x57EF00, ecx=perf, args=(0,), pop=4)
            fixtures['reset'].append({'precision': precision, 'case': case, 'after': rig.record_after(),
                                      'latch_after': m.rb(perf + 0x5C), 'events': rig.events})
        for case in stage_cases:
            perf = rig.performer(case)
            handle, _ = rig.handle(case['handle_kind'], case['target_position'])
            m.w(perf + 0x10, case['stage'])
            m.w(perf + 0x28, handle)
            m.w(perf + 0x2C, case['point'][0])
            m.w(perf + 0x30, case['point'][1])
            m.w(perf + 0x58, case['index'])
            m.call(0x57EE40, ecx=perf, args=(0,), pop=4)
            fixtures['stage0'].append({'precision': precision, 'case': case, 'handle': handle,
                                       'after': rig.record_after(), 'latch_after': m.rb(perf + 0x5C),
                                       'events': rig.events})
        visited.update(m.visited)
    table = fixtures.pop('table')
    cases = fixtures
    h.report('native-turret-turn-fixtures.json',
             '0x9C8660 arctangent table build, 0x85D9F0/0x85D8D0 table arctangent, 0x57DEE0 turn step, '
             '0x57E730 aim, 0x57EF00 reset update and 0x57EE40 stage-0 aim (50C290/50AC00/50B1C0/50ACC0/'
             '4C76E0 recorded) at PC24, PC53 and PC64.',
             cases, visited, extra={'table': table})


if __name__ == '__main__':
    main()
