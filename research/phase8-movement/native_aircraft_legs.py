"""Original AIRCRAFT path leg planning (AirCraft.cpp), executed unchanged.

0x55F230 is the path object's (aircraft+0x28) leg builder:
`(from: *pose7, out: *pose7, target x, y, z, direction: *[3] or null, flag, extra)`,
ret 0x20. It is the boundary `build_leg` of native_aircraft.py's MoveTick
oracle; here it runs with its own leaves (normalize 0x4453A0, altitude band
0x55D920, curve 0x55DA10, segment constructor 0x55DC00 / init 0x55E6E0, arc
lengths 0x55DCA0 / 0x55DEF0, length bisection 0x55E0F0, append 0x55E6A0,
__ftol2 0x96CC4C and its own recursion). 0x5601E0 (`append_legs`: pose,
target x, y, z, direction or null, flag, single; ret 0x1C) runs with
0x55F230, the leg target 0x55FE60, the circle test 0x55E1E0 and the path
clear 0x55E1B0. The helpers are also recorded alone. Seams: the terrain
height 0x4F0E50 (a queued response per call) and the engine allocator
0x6F7810 / 0x6F7430 (segments are 0x5C blocks on the emulated heap, every one
tracked).
"""
import copy, json, random, struct, sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
from harness import write_document, f2b, PRECISION  # noqa: E402
import native_aircraft as na  # noqa: E402


class LegAircraft(na.Aircraft):
    def __init__(self, case):
        super().__init__(case)
        del self.seams[0x55f230]
        self.seams[0x6f7810] = self.new
        self.allocations = []

    def new(self, m):
        size = m.arg(0)
        p = self.alloc(size)
        self.allocations.append(size)
        if size == 0x5c:
            self.segments.add(p)
        m.event('New', size)
        m.ret(0, p)

    def segment_object(self, s):
        seg = self.alloc(0x5c)
        self.w32(seg, na.SEGMENT_VTABLE, self.aircraft + 0x40, *s['control'])
        self.w32(seg + 0x38, s['length'], s['field_3c'], *s['end_direction'])
        self.w32(seg + 0x4c, s['end_speed'], s['start_speed'], s['tolerance'], 0)
        return seg

    def run(self):
        c = self.c
        a = c['args']
        op = c['operation']
        out = {}
        if op == 'build_leg':
            pin = self.pose(a['from'])
            pout = self.alloc(0x20)
            self.w32(pout, *([0x7fc0dead] * 7))
            dirp = self.pose(a['direction']) if a['direction'] is not None else 0
            self.call(0x55f230, self.aircraft + 0x28,
                      [pin, pout, *a['target'], dirp, a['flag'], a['extra']], count=200_000_000)
            assert self.final_esp == 0x2f08004 + 0x20
            out['out'] = self.r32(pout, 7)
            assert self.r32(pin, 7) == a['from']
            if dirp:
                assert self.r32(dirp, 3) == a['direction']
        elif op == 'append_legs':
            pin = self.pose(a['pose'])
            dirp = self.pose(a['direction']) if a['direction'] is not None else 0
            eax = self.call(0x5601e0, self.aircraft + 0x28,
                            [pin, *a['target'], dirp, a['flag'], a['single']], count=400_000_000)
            assert self.final_esp == 0x2f08004 + 0x1c
            out['return_al'] = eax & 0xff
            assert self.r32(pin, 7) == a['pose']
        elif op == 'flat_length':
            seg = self.segment_object(a['segment'])
            out['length_f80'] = self.capture_st0(0x55def0, seg, [a['t0'], a['t1']])
        elif op == 'length_parameter':
            seg = self.segment_object(a['segment'])
            out['parameter_f80'] = self.capture_st0(0x55e0f0, seg, [a['t0'], a['piece']])
        elif op == 'altitude_band':
            target = self.pose(a['target'])
            pin = self.pose(a['from'])
            self.call(0x55d920, self.aircraft + 0x28, [target, pin])
            assert self.final_esp == 0x2f08004 + 8
            out['target'] = self.r32(target, 3)
        elif op == 'circle_side':
            pin = self.pose(a['pose'])
            side = self.pose(a['side'])
            out['return_eax'] = self.call(0x55e1e0, self.aircraft + 0x28, [pin, side, *a['target']])
            assert self.final_esp == 0x2f08004 + 0x14
        else:
            raise ValueError(op)
        assert self.fpu_top() == 0, 'unbalanced x87 stack'
        out['aircraft'] = self.read_aircraft(self.aircraft)
        out['events'] = self.events
        return out


def heights(rng, n=24):
    return [{'f64': na.f64h(rng.uniform(-5, 30))} for _ in range(n)]


def path_kw(rng):
    return {'altitude': f2b(rng.choice([20.0, 0.0, 35.0, rng.uniform(5, 50)])),
            'turn_radius': f2b(rng.choice([8.0, 8.0, 20.0, rng.uniform(2, 60)])),
            'planning_speed': f2b(rng.uniform(5, 40)), 'leg_speed': f2b(rng.uniform(5, 40))}


def case(rng, name, op, ac, args, n_heights=24):
    return na.base(name, op, precision=rng.choice(['Double53', 'Double53', 'Single24', 'Extended64']),
                   actor=na.actor(), mover={'requested_speed': f2b(10.0), 'max_speed': f2b(30.0),
                                            'speed_factor': f2b(1.0)},
                   aircraft=ac, responses={'GroundHeight': heights(rng, n_heights)}, args=args)


def unit_dir(rng, z=0.2):
    return na.fb(*na.unit([rng.uniform(-1, 1), rng.uniform(-1, 1), rng.uniform(-z, z)]))


def leg_cases(rng, count):
    rows = []
    for i in range(count):
        start = [rng.uniform(30, 200), rng.uniform(30, 160), rng.uniform(10, 35)]
        d = na.unit([rng.uniform(-1, 1), rng.uniform(-1, 1), 0.0])
        segs = na.chain(rng, start, d, rng.choice([0, 0, 1, 2, 3]))
        ac = na.aircraft(segs, path_kw=path_kw(rng))
        frm = na.rand_pose(rng)
        target = [rng.uniform(10, 240), rng.uniform(10, 180), rng.uniform(0, 40)]
        if i % 7 == 0:
            # Short and degenerate legs.
            f = [struct.unpack('<f', struct.pack('<I', w))[0] for w in frm[:3]]
            target = [f[0] + rng.uniform(-3, 3), f[1] + rng.uniform(-3, 3), f[2]]
        direction = None if i % 3 == 0 else unit_dir(rng)
        if i % 11 == 5:
            frm[6] = rng.choice([0, f2b(5e-5), f2b(1e-4)])  # at most the 1e-4 speed floor
        rows.append(case(rng, f'leg_{i}', 'build_leg', ac,
                         {'from': frm, 'target': na.fb(*target), 'direction': direction,
                          'flag': rng.choice([0, 0, 1]), 'extra': rng.choice([0, 0, 0, 1])}, 200))
    return rows


def append_cases(rng, count):
    rows = []
    for i in range(count):
        ac = na.aircraft(na.chain(rng, [90.0, 80.0, 20.0], [1.0, 0.0, 0.0], rng.choice([0, 0, 1])),
                         path_kw=path_kw(rng))
        pose = na.rand_pose(rng)
        target = [rng.uniform(10, 240), rng.uniform(10, 180), rng.uniform(0, 40)]
        if i % 6 == 0:
            p = [struct.unpack('<f', struct.pack('<I', w))[0] for w in pose[:3]]
            target = [p[0] + rng.uniform(-15, 15), p[1] + rng.uniform(-15, 15), p[2]]
        direction = None if i % 3 == 0 else unit_dir(rng, 0.0)
        degenerate = i % 10 == 4
        if degenerate:
            # No horizontal heading: the zero side vector of 0x5601E0. A turn
            # from it aims at the start point itself, a zero-length leg that
            # the original computes through infinities and NaNs (outside the
            # port's finite domain, so the replay accepts an error there).
            pose[3:6] = na.fb(0.0, 0.0, rng.choice([1.0, -1.0]))
        rows.append(case(rng, f'append_{i}', 'append_legs', ac,
                         {'pose': pose, 'target': na.fb(*target), 'direction': direction,
                          'flag': rng.choice([0, 0, 0, 1]), 'single': rng.choice([0, 0, 0, 1]),
                          'degenerate': degenerate}, 4000))
    return rows


def random_segment(rng):
    p0 = [rng.uniform(0, 200), rng.uniform(0, 200), rng.uniform(0, 40)]
    p3 = [p0[0] + rng.uniform(-80, 80), p0[1] + rng.uniform(-80, 80), p0[2] + rng.uniform(-10, 10)]
    s = na.segment(p0, na.unit([rng.uniform(-1, 1), rng.uniform(-1, 1), 0.1]), p3,
                   na.unit([rng.uniform(-1, 1), rng.uniform(-1, 1), 0.0]), rng.uniform(4, 30), rng.uniform(4, 30),
                   tolerance=rng.choice([na.TENTH, na.TENTH, f2b(0.01), f2b(1.0)]),
                   bend=rng.choice([1.0 / 3.0, 0.5, 1.0, 0.1]))
    return s


def helper_cases(rng):
    rows = []
    for i in range(60):
        t0 = rng.choice([0.0, 0.0, rng.uniform(0, 0.8)])
        t1 = rng.choice([1.0, 1.0, rng.uniform(t0 + 0.05, 1.0)])
        rows.append(case(rng, f'flat_{i}', 'flat_length', na.aircraft(path_kw=path_kw(rng)),
                         {'segment': random_segment(rng), 't0': f2b(t0), 't1': f2b(t1)}))
    for i in range(60):
        rows.append(case(rng, f'param_{i}', 'length_parameter', na.aircraft(path_kw=path_kw(rng)),
                         {'segment': random_segment(rng), 't0': f2b(rng.choice([0.0, rng.uniform(0, 0.9)])),
                          'piece': f2b(rng.uniform(1, 60))}))
    for i in range(60):
        alt = rng.choice([20.0, 0.0, -0.0, rng.uniform(1, 50)])
        ground = rng.uniform(-5, 30)
        rows.append(case(rng, f'band_{i}', 'altitude_band',
                         na.aircraft(path_kw={'altitude': f2b(alt)}),
                         {'target': na.fb(rng.uniform(0, 200), rng.uniform(0, 200), ground + alt),
                          'from': na.pose_words([50.0, 50.0, rng.choice([ground + alt, ground + rng.uniform(-30, 80)])],
                                                [1.0, 0.0, 0.0], 10.0)}))
    for i in range(60):
        pose = na.rand_pose(rng)
        p = [struct.unpack('<f', struct.pack('<I', w))[0] for w in pose[:3]]
        target = [p[0] + rng.uniform(-40, 40), p[1] + rng.uniform(-40, 40), p[2] + rng.uniform(-3, 3)]
        rows.append(case(rng, f'circle_{i}', 'circle_side', na.aircraft(path_kw=path_kw(rng)),
                         {'pose': pose, 'side': unit_dir(rng, 0.0), 'target': na.fb(*target)}))
    return rows


def run_case(c):
    m = LegAircraft(c)
    try:
        out = m.run()
        c['responses'] = {k: v[:m.used[k]] for k, v in c['responses'].items() if m.used.get(k)}
        return {'case': c, 'result': out}
    except Exception as e:  # noqa: BLE001
        return {'case': c, 'error': f'{type(e).__name__}: {e}', 'events': m.events}


def main():
    rng = random.Random(0x55F230)
    count = int(sys.argv[1]) if len(sys.argv) > 1 else 200
    templates = leg_cases(rng, count) + append_cases(rng, count // 2) + helper_cases(rng)
    cases = [run_case(copy.deepcopy(c)) for c in templates]
    errors = [r for r in cases if 'error' in r]
    kinds = {}
    for r in cases:
        kinds[r['case']['operation']] = kinds.get(r['case']['operation'], 0) + 1
    print(json.dumps({'cases': len(cases), 'kinds': kinds, 'errors': len(errors),
                      'first_errors': [(r['case']['name'], r['error']) for r in errors[:8]],
                      'allocations': sorted({s for r in cases if 'result' in r
                                             for s in [e['bits'][0] for e in r['result']['events']
                                                       if e['name'] == 'New']}),
                      'max_heights': max(len(r['case']['responses'].get('GroundHeight', [])) for r in cases
                                         if 'result' in r)}))
    if '--write' in sys.argv:
        write_document('native-aircraft-legs.json', 'AIRCRAFT leg planning 0x55F230 / 0x5601E0', cases,
                       {'operation_counts': kinds})


if __name__ == '__main__':
    main()
