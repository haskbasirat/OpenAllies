"""Original AIRCRAFT flight entry points (AirCraft.cpp), executed unchanged.

- 0x560DB0 fly_to (thiscall aircraft: x, y, z, &direction or null; ret 0x10):
  teardown 0x55EDC0, circling stop, map clamp 0x552640, 0x5601E0, member
  path reset 0x55EBE0;
- 0x560ED0 replace_path (same arguments): 0x55EDC0, 0x55E1B0, 0x560DB0;
- 0x560F10 fly_formation (offset x, y, z, goal x, y, z, &direction; ret 0x1C):
  0x552640, 0x517DF0, 0x5601E0, 0x55EBE0;
- 0x560A70 follow_path (thiscall path: &pose, leader path, offset x, y, z,
  delta; ret 0x18): 0x55DCA0, 0x517DF0, 0x5601E0, 0x55E7E0, 0x4453A0,
  0x55E1E0;
- 0x517DF0 slot_point (cdecl) and 0x55E7E0 set_path_speed alone.

Everything below them (leg builder 0x55F230 and its helpers, the planner
0x5601E0, the formation teardown) runs unchanged too. Seams: the terrain
height 0x4F0E50 (queued responses), the allocator (segments tracked), the
group virtuals +0x14 / +0 (see native_aircraft_formation.py). The GMap at
[GWorld]+0x30 carries the case's cell size (+8) and extents (+0xC/+0x10).
"""
import copy, json, random, struct, sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
from harness import NullDereference, write_document, f2b  # noqa: E402
import native_aircraft as na  # noqa: E402
import native_aircraft_formation as nf  # noqa: E402
import native_aircraft_legs as nl  # noqa: E402


class FlightAircraft(nf.FormationAircraft, nl.LegAircraft):
    def __init__(self, case):
        super().__init__(case)
        del self.seams[0x560a70]
        gmap = self.alloc(0x40)
        self.w32(gmap + 8, *case['map'])
        self.w32(self.gworld + 0x30, gmap)

    def run(self):
        c = self.c
        a = c['args']
        op = c['operation']
        out = {}
        dirp = self.pose(a['direction']) if a.get('direction') is not None else 0
        if op in ('fly_to', 'replace_path'):
            entry = 0x560db0 if op == 'fly_to' else 0x560ed0
            out['return_al'] = self.call(entry, self.aircraft, [*a['point'], dirp], count=400_000_000) & 0xff
            assert self.final_esp == 0x2f08004 + 0x10
        elif op == 'fly_formation':
            self.call(0x560f10, self.aircraft, [*a['offset'], *a['goal'], dirp], count=400_000_000)
            assert self.final_esp == 0x2f08004 + 0x1c
        elif op == 'follow_path':
            pin = self.pose(a['pose'])
            leader = self.aircraft_ptr[a['leader']] + 0x28
            out['return_al'] = self.call(0x560a70, self.aircraft + 0x28,
                                         [pin, leader, *a['offset'], a['delta']], count=400_000_000) & 0xff
            assert self.final_esp == 0x2f08004 + 0x18
            assert self.r32(pin, 7) == a['pose']
        elif op == 'slot_point':
            res = self.alloc(16)
            self.call(0x517df0, 0, [res, *a['position'], *a['facing'], *a['offset']])
            assert self.final_esp == 0x2f08004
            out['point'] = self.r32(res, 3)
        elif op == 'set_path_speed':
            self.call(0x55e7e0, self.aircraft + 0x28, [a['speed']])
            assert self.final_esp == 0x2f08004 + 4
        else:
            raise ValueError(op)
        assert self.fpu_top() == 0, 'unbalanced x87 stack'
        out['aircraft'] = self.read_aircraft(self.aircraft)
        out['others'] = {str(o['key']): self.read_aircraft(self.aircraft_ptr[o['key']])
                         for o in c.get('others', []) if o['aircraft'] is not None}
        out['refs'] = {str(g['key']): self.r32(self.group_ptr[g['key']] + 4) for g in c['groups']
                       if not any(e['name'] == 'GroupDelete' and e['bits'][0] == g['key'] for e in self.events)}
        out['events'] = self.events
        return out


def world(rng, i, groups_on):
    """A formation world as in native_aircraft_formation.py (or without
    groups), the self aircraft's path and pose, and the map."""
    c = nf.formation_case(rng, i)
    if not groups_on:
        c['aircraft']['groups'] = [0, 0, 0]
        for o in c['others']:
            if o['aircraft'] is not None:
                o['aircraft']['groups'] = [0, 0, 0]
    ac = c['aircraft']
    ac['path'].update(nl.path_kw(rng))
    ac['path']['speed'] = f2b(rng.uniform(5, 30))
    ac['on_ground'] = rng.choice([0, 0, 0, 1])
    ac['circling'] = rng.choice([0, 0, 1])
    c['actor']['position'] = na.fb(rng.uniform(30, 200), rng.uniform(30, 160), rng.uniform(5, 40))
    c['actor']['facing'] = na.fb(*na.unit([rng.uniform(-1, 1), rng.uniform(-1, 1), rng.uniform(-0.2, 0.2)]))
    c['map'] = [f2b(rng.choice([4.0, 8.0])), f2b(rng.uniform(200, 300)), f2b(rng.uniform(150, 250))]
    c['responses'] = {'GroundHeight': nl.heights(rng, 4000)}
    c['precision'] = rng.choice(['Double53', 'Double53', 'Single24', 'Extended64'])
    holders = {g: 0 for g in nf.GROUPS}
    for k in range(1, 6):
        a = nf.aircraft_of(c, k)
        for g in (a['groups'] if a else []):
            if g:
                holders[g] += 1
    for g in c['groups']:
        g['refs'] = holders[g['key']] + 1
    return c


def point(rng, c):
    p = [struct.unpack('<f', struct.pack('<I', w))[0] for w in c['actor']['position']]
    if rng.random() < 0.15:
        return na.fb(rng.uniform(-50, 350), rng.uniform(-50, 300), rng.uniform(0, 40))  # often off the map
    return na.fb(p[0] + rng.uniform(-120, 120), p[1] + rng.uniform(-120, 120), rng.uniform(0, 40))


def flight_cases(rng, count):
    rows = []
    i = 0
    while len(rows) < count:
        i += 1
        op = ['fly_to', 'fly_to', 'replace_path', 'fly_formation', 'follow_path', 'follow_path', 'slot_point',
              'set_path_speed'][i % 8]
        c = world(rng, i, groups_on=rng.random() < 0.3)
        if not nf.recursion_is_finite(c):
            continue
        c['name'] = f'{op}_{i}'
        c['operation'] = op
        direction = None if rng.random() < 0.3 else nl.unit_dir(rng, 0.0)
        if op in ('fly_to', 'replace_path'):
            c['args'] = {'point': point(rng, c), 'direction': direction}
        elif op == 'fly_formation':
            c['args'] = {'offset': na.fb(rng.uniform(-20, 20), rng.uniform(-20, 20), rng.uniform(-3, 3)),
                         'goal': point(rng, c), 'direction': direction}
        elif op == 'follow_path':
            leader = next(o for o in c['others'] if o['key'] == 2)
            if leader['aircraft'] is None:
                leader['aircraft'] = na.aircraft()
            lp = leader['aircraft']['path']
            start = [rng.uniform(40, 200), rng.uniform(40, 150), rng.uniform(10, 35)]
            lp['segments'] = na.chain(rng, start, [rng.uniform(-1, 1), rng.uniform(-1, 1), 0.0], rng.choice([1, 1, 2, 3]))
            lp['t'] = f2b(rng.choice([0.0, rng.uniform(0, 0.95)]))
            c['args'] = {'pose': na.rand_pose(rng), 'leader': 2,
                         'offset': na.fb(rng.uniform(-25, 25), rng.uniform(-25, 25), rng.uniform(-3, 3)),
                         'delta': f2b(rng.uniform(0, 0.2))}
            if rng.random() < 0.5:
                # Near the slot, so the timing branch is reached.
                s = lp['segments'][0]
                end = [struct.unpack('<f', struct.pack('<I', w))[0] for w in s['control'][9:12]]
                c['args']['pose'][:3] = na.fb(end[0] + rng.uniform(-60, 0), end[1] + rng.uniform(-30, 30), end[2])
        elif op == 'slot_point':
            c['args'] = {'position': na.fb(rng.uniform(0, 200), rng.uniform(0, 200), rng.uniform(0, 40)),
                         'facing': na.fb(*na.unit([rng.uniform(-1, 1), rng.uniform(-1, 1)])),
                         'offset': na.fb(rng.uniform(-30, 30), rng.uniform(-30, 30),
                                         rng.choice([0.0, -0.0, rng.uniform(-5, 5)]))}
        else:
            c['args'] = {'speed': f2b(rng.uniform(1, 40))}
        rows.append(c)
    return rows


def run_case(c):
    m = FlightAircraft(c)
    try:
        out = m.run()
    except (NullDereference, nf.UseAfterFree) as error:
        out = {'error': f'{type(error).__name__}: {error}', 'events': m.events}
    c['responses'] = {k: v[:m.used[k]] for k, v in c['responses'].items() if m.used.get(k)}
    return out


def main():
    rng = random.Random(0x560DB0)
    count = int(sys.argv[1]) if len(sys.argv) > 1 else 400
    cases = []
    for c in flight_cases(rng, count):
        c['output'] = run_case(c)
        cases.append(c)
    kinds = {}
    for c in cases:
        k = c['operation'] + ('/' + c['output']['error'].split(':')[0] if 'error' in c['output'] else
                              ('/' + str(c['output']['return_al']) if 'return_al' in c['output'] else ''))
        kinds[k] = kinds.get(k, 0) + 1
    print(json.dumps({'cases': len(cases), 'kinds': kinds}))
    if '--write' in sys.argv:
        write_document('native-aircraft-flight.json', 'AIRCRAFT flight entry points 0x560DB0 / 0x560ED0 / '
                       '0x560F10 / 0x560A70', cases, {'kinds': kinds})


if __name__ == '__main__':
    main()
