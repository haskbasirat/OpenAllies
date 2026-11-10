"""Original AIRCRAFT group order 0x55F160(group, leading) (thiscall aircraft,
ret 8; AirCraft.cpp), executed unchanged with the formation join 0x55EB30,
the teardown 0x55EDC0 (with its follower recursion), the leader getter
0x5685B0 and the reference assignment 0x7A1030.

The world is the formation oracle's (native_aircraft_formation.py): the
group virtuals +0x14 (per-case usable answer) and +0 (deleting destructor,
recorded, the block and its member nodes then poisoned so that a later
native access is UseAfterFree), the segment free 0x6F7430 and the definition
speed. Cases whose teardown recursion could revisit an aircraft are not
generated (the native recursion never ends).

Usage: native_aircraft_follow.py [count] --write (AA.exe is located by the
shared harness relative to this repository).
"""
import copy, json, random, sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
from harness import NullDereference, write_document  # noqa: E402
import native_aircraft_formation as naf  # noqa: E402


class FollowAircraft(naf.FormationAircraft):
    def run(self):
        c = self.c
        if c['operation'] != 'follow':
            return super().run()
        a = c['args']
        group = self.group_ptr[a['group']] if a['group'] else 0
        leading = self.group_ptr[a['leading']] if a['leading'] else 0
        self.call(0x55f160, self.aircraft, [group, leading], count=20_000_000)
        assert self.final_esp == 0x2f08004 + 8
        assert self.fpu_top() == 0, 'unbalanced x87 stack'
        return {'aircraft': self.read_aircraft(self.aircraft),
                'others': {str(o['key']): self.read_aircraft(self.aircraft_ptr[o['key']])
                           for o in c.get('others', []) if o['aircraft'] is not None},
                'refs': {str(g['key']): self.r32(self.group_ptr[g['key']] + 4) for g in c['groups']
                         if not any(e['name'] == 'GroupDelete' and e['bits'][0] == g['key'] for e in self.events)},
                'events': self.events}


def run_case(c):
    m = FollowAircraft(c)
    try:
        return m.run()
    except (NullDereference, naf.UseAfterFree) as error:
        return {'error': f'{type(error).__name__}: {error}', 'events': m.events}


def main():
    rng = random.Random(0x55F160)
    count = int(sys.argv[1]) if len(sys.argv) > 1 and sys.argv[1].isdigit() else 400
    cases, skipped = [], 0
    while len(cases) < count:
        c = naf.formation_case(rng, len(cases) + skipped)
        c['name'] = 'follow_%d' % len(cases)
        c['operation'] = 'follow'
        c['args'] = {'group': rng.choice([0, 1, 2, 3, 4, 4]), 'leading': rng.choice([0, 0, 1, 2, 3, 4])}
        if not naf.recursion_is_finite(c):
            skipped += 1
            continue
        c['output'] = run_case(copy.deepcopy(c))
        cases.append(c)
    kinds = {}
    for c in cases:
        k = c['operation'] + ('/' + c['output']['error'].split(':')[0] if 'error' in c['output'] else '')
        kinds[k] = kinds.get(k, 0) + 1
    deletes = sum(1 for c in cases for e in c['output']['events'] if e['name'] == 'GroupDelete')
    print(json.dumps({'cases': len(cases), 'skipped_cyclic': skipped, 'kinds': kinds, 'deletes': deletes}))
    if '--write' in sys.argv:
        write_document('native-aircraft-follow.json', 'AIRCRAFT group order 0x55F160', cases, {'kinds': kinds})


if __name__ == '__main__':
    main()
