"""Original AIRCRAFT formation join 0x55EB30, teardown 0x55EDC0 and follower
test 0x55E660 (AirCraft.cpp), executed unchanged.

0x55EB30(group) (thiscall aircraft, ret 4) joins a formation group (+0xA8, then
+0xAC follows it) or, for a null group, is the teardown 0x55EDC0. 0x55EDC0
(thiscall aircraft) runs with its own recursion over the follower group
+0xB0, the leader getter 0x5685B0, the inline handle resolution
([0xB364C0] registry) and the inline path clear. Seams: the group virtuals
+0x14 (the per-case usable answer) and +0 (the deleting destructor, recorded;
the group block and its member nodes are then poisoned so that any later
native access is reported as UseAfterFree), the segment free 0x6F7430 and
the definition speed (definition +0x228, read by 0x45D6F0 unchanged).

Cases whose follower recursion could revisit an aircraft (the native
recursion never ends) are not generated.
"""
import copy, json, random, sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
from harness import NullDereference, write_document, f2b, PRECISION  # noqa: E402
from unicorn import UC_HOOK_MEM_READ, UC_HOOK_MEM_WRITE  # noqa: E402
import native_aircraft as na  # noqa: E402


class UseAfterFree(Exception):
    pass


class FormationAircraft(na.Aircraft):
    def __init__(self, case):
        super().__init__(case)
        del self.seams[0x55edc0]
        speeds = case['definition_speeds']
        self.w32(self.r32(self.actor + 4) + 0x228, speeds[str(na.SELF)])
        for o in case.get('others', []):
            definition = self.alloc(0x300)
            self.w32(definition + 0x228, speeds[str(o['key'])])
            self.w32(self.actor_ptr[o['key']] + 4, definition)
        for g in case['groups']:
            p = self.group_ptr[g['key']]
            self.w32(self.r32(p), self.stub(self.group_delete))
            self.w32(p + 4, g['refs'])

    def group_delete(self, m):
        p = m.ecx()
        key = self.group_key[p]
        assert m.arg(0) == 1
        m.event('GroupDelete', key)
        blocks = [(p, 0x40)]
        node = self.r32(p + 0xc)
        while node:
            blocks.append((node, 8))
            node = self.r32(node + 4)
        for start, size in blocks:
            self.u.hook_add(UC_HOOK_MEM_READ | UC_HOOK_MEM_WRITE, self._freed, begin=start, end=start + size - 1)
        m.ret(4)

    def _freed(self, u, access, address, size, value, _):
        raise UseAfterFree(hex(address))

    def run(self):
        c = self.c
        a = c['args']
        if c['operation'] == 'leave':
            self.call(0x55edc0, self.aircraft, [], count=20_000_000)
            assert self.final_esp == 0x2f08004
        elif c['operation'] == 'join':
            group = self.group_ptr[a['group']] if a['group'] else 0
            self.call(0x55eb30, self.aircraft, [group], count=20_000_000)
            assert self.final_esp == 0x2f08004 + 4
        elif c['operation'] == 'follower':
            value = self.call(0x55e660, self.aircraft) & 0xff
            assert self.final_esp == 0x2f08004
        else:
            raise ValueError(c['operation'])
        assert self.fpu_top() == 0, 'unbalanced x87 stack'
        out = {'aircraft': self.read_aircraft(self.aircraft),
               'others': {str(o['key']): self.read_aircraft(self.aircraft_ptr[o['key']])
                          for o in c.get('others', []) if o['aircraft'] is not None},
               'refs': {str(g['key']): self.r32(self.group_ptr[g['key']] + 4) for g in c['groups']
                        if not any(e['name'] == 'GroupDelete' and e['bits'][0] == g['key'] for e in self.events)},
               'events': self.events}
        if c['operation'] == 'follower':
            out['value'] = value
        return out


# Actor keys 1 (SELF) .. 5; handle index i, generation g; 'stale' and 'null'
# handles never resolve.
HANDLES = [[1, 3, 1], [2, 5, 2], [3, 1, 3], [4, 2, 4], [5, 7, 5], [6, 9, 2], [7, 4, 0]]
H = {1: 0x00030001, 2: 0x00050002, 3: 0x00010003, 4: 0x00020004, 5: 0x00070005,
     'stale': 0x00080006, 'null': 0x00040007}
GROUPS = [1, 2, 3, 4]


def resolves(case, handle):
    for index, generation, key in case['handles']:
        if index == handle & 0xffff:
            return key if key and generation == handle >> 16 else None
    return None


def flags(case, key):
    if key == na.SELF:
        return case['actor']['flags']
    return next(o['flags'] for o in case['others'] if o['key'] == key)


def aircraft_of(case, key):
    if key == na.SELF:
        return case['aircraft']
    return next(o['aircraft'] for o in case['others'] if o['key'] == key)


def recursion_is_finite(case):
    """0x55EDC0 recurses into the live members of a usable +0xB0 group. Group
    words only ever get cleared during a teardown, so an acyclic initial
    graph guarantees a finite native recursion."""
    groups = {g['key']: g for g in case['groups']}

    def followers(key):
        ac = aircraft_of(case, key)
        if ac is None:
            return []
        c = ac['groups'][2]
        if not c or groups[c]['disbanded']:
            return []
        out = []
        for h in groups[c]['members']:
            k = resolves(case, h)
            if k is not None and flags(case, k) & 1 == 0:
                out.append(k)
        return out

    def visit(key, stack):
        if key in stack:
            return False
        return all(visit(k, stack + [key]) for k in followers(key))
    return visit(na.SELF, [])


def formation_case(rng, i):
    members = [H[1], H[2], H[3], H[4], H[5], H['stale'], H['null']]
    groups = []
    for key in GROUPS:
        chosen = [h for h in members if rng.random() < 0.45]
        rng.shuffle(chosen)
        groups.append({'key': key, 'disbanded': rng.random() < 0.25, 'members': chosen, 'layout': None,
                       'anchor': na.fb(0.0, 0.0, 0.0)})

    def refs():
        return [rng.choice([0, 0, 1, 2, 3, 4]) for _ in range(3)]

    ac = na.aircraft(na.chain(rng, [80.0, 70.0, 20.0], [1.0, 0.0, 0.0], rng.choice([0, 1, 2])),
                     groups=refs(), on_ground=rng.choice([0, 0, 1]),
                     cruise_speed=f2b(rng.uniform(5, 25)), circling=rng.choice([0, 0, 1]))
    others = []
    for key in (2, 3, 4, 5):
        oac = None
        if rng.random() < 0.9:
            oac = na.aircraft(na.chain(rng, [40.0 + 15 * key, 60.0, 22.0], [1.0, 0.2, 0.0], rng.choice([0, 1])),
                              groups=refs(), on_ground=rng.choice([0, 0, 1]), cruise_speed=f2b(rng.uniform(5, 25)))
        others.append({'key': key, 'handle': H[key], 'flags': 1 if rng.random() < 0.12 else 0, 'aircraft': oac})
    case = na.base(f'formation_{i}', 'leave', precision=rng.choice(list(PRECISION)),
                   actor=na.actor(flags=1 if rng.random() < 0.05 else 0), aircraft=ac,
                   mover={'requested_speed': 0, 'max_speed': 0, 'speed_factor': 0},
                   others=others, groups=groups, handles=HANDLES,
                   definition_speeds={str(k): f2b(rng.choice([12.0, 30.0, rng.uniform(5, 60)])) for k in range(1, 6)})
    # One reference per holder, plus the owner's (the base's mission list)
    # unless the case tests the deleting destructor.
    holders = {g: 0 for g in GROUPS}
    for k in range(1, 6):
        a = aircraft_of(case, k)
        for g in (a['groups'] if a else []):
            if g:
                holders[g] += 1
    for g in groups:
        g['refs'] = holders[g['key']] + rng.choice([1, 1, 1, 2, 0])
    if i % 3 == 0:
        case['operation'] = 'join'
        case['args'] = {'group': rng.choice([0, 1, 2, 3, 4, 4])}
    elif i % 5 == 1:
        case['operation'] = 'follower'
        case['args'] = {}
    else:
        case['args'] = {}
    return case


def run_case(c):
    m = FormationAircraft(c)
    try:
        return m.run()
    except (NullDereference, UseAfterFree) as error:
        return {'error': f'{type(error).__name__}: {error}', 'events': m.events}


def main():
    rng = random.Random(0x55EDC0)
    count = int(sys.argv[1]) if len(sys.argv) > 1 else 300
    cases, skipped = [], 0
    while len(cases) < count:
        c = formation_case(rng, len(cases) + skipped)
        if not recursion_is_finite(c):
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
        write_document('native-aircraft-formation.json', 'AIRCRAFT formation join 0x55EB30 / teardown 0x55EDC0',
                       cases, {'kinds': kinds})


if __name__ == '__main__':
    main()
