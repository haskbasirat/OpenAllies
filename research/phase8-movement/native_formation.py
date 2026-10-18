"""Original Organization formation slot positions and slot reassignment.

Whole unchanged routines execute: ComputeSlotPosition 0x5797E0 (with
0x578250 facing rotation, 0x4F02B0 RNG accessor, 0x444A90 table direction,
0x45D400 extended unit draw), all-slot refresh 0x57BEF0, slot reassignment
0x57BA80 (with vector fill 0x4A8700) and leader change 0x579A00. The
random-direction table comes from native-direction-table.json (PC53 static
initializer output). Seams: RNG step 0x85D840 (supplied raw draws),
allocator 0x6F7810/0x6F7430, diagnostic 0x404F90, Mind virtual +0x114.
Sync-log objects at 0xB3655C/0xB36560 are disabled (byte 0), so 0x77DA70
executes its early return.
"""
import copy, json, random, struct, sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
from harness import Machine, NullDereference, write_document, f2b, PRECISION, HERE  # noqa: E402

TABLE = struct.unpack('<4096I', (HERE / 'direction-table.bin').read_bytes())


class FormationMachine(Machine):
    def __init__(self, case):
        super().__init__(case['precision'])
        self.c = case
        # Direction table object at its native static address.
        data = self.alloc(len(TABLE) * 4)
        self.w32(data, *TABLE)
        self.w32(0xb39a90, 0x9f32a0, 2048, data, 0x40c90fdb)
        self.w8(0xb39a90 + 0x10, 1)
        self.w32(0xb39af8, 0xb39a90)
        world = self.alloc(0x200)
        self.w32(0xb36534, world)
        sync = self.alloc(0x40)
        self.w32(0xb3655c, sync)
        self.w32(0xb36560, sync)
        o = case['organization']
        self.org = self.alloc(0x100)
        owner = self.alloc(0x200)
        self.w32(self.org + 4, owner)
        self.w32(owner + 0x14, o['owner_id'])
        self.w32(owner + 0x2c, *o['facing'])
        if o['owner_has_mind']:
            mind = self.alloc(0x40)
            mvt = self.alloc(0x200)
            self.w32(mind, mvt)
            stub = self.stub_next
            self.stub_next += 16
            self.seams[stub] = lambda m: (m.event('MindLeaderChanged'), m.ret(0))
            self.w32(mvt + 0x114, stub)
            self.w32(owner + 0x70, mind)
        layout = self.alloc(0x40)
        self.w32(self.org + 0x18, layout)
        n = o['slot_count']
        self.n = n
        self.w32(layout + 0x18, o['primary_slot'] & 0xffffffff)
        groups = self.alloc(16 * max(1, len(o['groups'])))
        for i, count in enumerate(o['groups']):
            self.w32(groups + 16 * i + 8, count & 0xffffffff)
        self.w32(layout + 0x1c, groups)
        self.w32(layout + 0x20, len(o['groups']))
        self.w32(layout + 0x2c, n)
        self.actors = {}
        members = self.alloc(4 * max(n, 1))
        for slot, member in enumerate(o['members']):
            if member is not None:
                self.w32(members + 4 * slot, self.actor(member))
        self.w32(self.org + 0x28, members)
        self.w32(self.org + 0x2c, n)
        leader = o['leader']
        self.w32(self.org + 0x40, self.actor(leader) if leader is not None else 0)
        if o['formation'] is not None:
            formation = self.alloc(0x20)
            definition = self.alloc(0x40)
            self.w8(definition + 0x18, o['formation']['oriented'])
            offsets = self.alloc(12 * max(1, len(o['formation']['offsets'])))
            for i, (x, y) in enumerate(o['formation']['offsets']):
                self.w32(offsets + 12 * i, 0xdeadbeef, x, y)
            self.w32(formation + 0xc, definition)
            self.w32(formation + 0x10, offsets)
            self.w32(self.org + 0x74, formation)
        self.positions = self.alloc(8 * max(n, 1))
        for slot, (x, y) in enumerate(o['slot_positions']):
            self.w32(self.positions + 8 * slot, x, y)
        self.w32(self.org + 0x78, self.positions)
        self.w32(self.org + 0xbc, *o['center'])
        self.draws = list(case['draws'])
        self.allocations = {}
        S = self.seams
        S[0x85d840] = self.random
        S[0x6f7810] = self.allocate
        S[0x6f7430] = self.free
        S[0x404f90] = self.diagnostic
        S[0x855d30] = lambda m: (_ for _ in ()).throw(AssertionError('e<32 assertion reached'))

    def actor(self, member):
        key = member['id']
        if key in self.actors:
            return self.actors[key]
        a = self.alloc(0x100)
        element = self.alloc(0x40)
        self.w32(a + 0x14, member['id'])
        self.w32(a + 0x20, *member['position'])
        self.w32(a + 0x80, element)
        self.w32(element + 0x18, member['slot'] & 0xffffffff)
        self.w8(element + 0x1c, member['attached'])
        self.actors[key] = a
        return a

    def random(self, m):
        value = self.draws.pop(0)
        m.event('Random', value)
        m.ret(0, value)

    def allocate(self, m):
        size = m.arg()
        p = m.alloc(size)
        self.allocations[p] = size
        m.ret(0, p)

    def free(self, m):
        p = m.arg()
        assert p in self.allocations
        del self.allocations[p]
        m.ret(0)

    def diagnostic(self, m):
        org_id, member_id = m.arg(5), m.arg(6)
        x_bits = struct.unpack('<Q', struct.pack('<2I', m.arg(7), m.arg(8)))[0]
        y_bits = struct.unpack('<Q', struct.pack('<2I', m.arg(9), m.arg(10)))[0]
        x = struct.unpack('<d', struct.pack('<Q', x_bits))[0]
        y = struct.unpack('<d', struct.pack('<Q', y_bits))[0]
        m.event('DetachedDiagnostic', org_id, member_id, f2b(x), f2b(y))
        m.ret(0)

    def run(self):
        c = self.c
        op = c['operation']
        if op == 'compute':
            self.call(0x5797e0, self.org, [c['slot'] & 0xffffffff])
            pops = 4
        elif op == 'compute_all':
            self.call(0x57bef0, self.org)
            pops = 0
        elif op == 'reassign':
            self.call(0x57ba80, self.org)
            pops = 0
        elif op == 'set_leader':
            new = c['new_leader']
            self.call(0x579a00, self.org, [self.actor(new) if new is not None else 0])
            pops = 4
        assert self.final_esp == 0x2f08000 + 4 + pops
        assert self.fpu_top() == 0, 'unbalanced x87 stack'
        assert not self.draws and not self.allocations
        leader = self.r32(self.org + 0x40)
        ids = {v: k for k, v in self.actors.items()}
        return {'slot_positions': [self.r32(self.positions + 8 * i, 2) for i in range(self.n)],
                'leader': ids.get(leader) if leader else None, 'events': self.events}


def offsets_for(n, rng):
    return [[f2b(rng.uniform(-6, 6)), f2b(rng.uniform(-6, 6))] for _ in range(n)]


def organization(rng, n, groups, oriented=1, leader_slot=0, primary=0, empty=(), detached=(), formation=True):
    members = []
    for slot in range(n):
        if slot in empty:
            members.append(None)
            continue
        members.append({'id': 100 + slot, 'slot': slot, 'attached': 0 if slot in detached else 1,
                        'position': [f2b(rng.uniform(20, 60)), f2b(rng.uniform(20, 60))]})
    leader = members[leader_slot] if leader_slot is not None and leader_slot < n else None
    angle = rng.uniform(-3.1, 3.1)
    import math
    return {'owner_id': 77, 'owner_has_mind': True, 'facing': [f2b(math.cos(angle)), f2b(math.sin(angle))],
            'center': [f2b(rng.uniform(25, 55)), f2b(rng.uniform(25, 55))], 'slot_count': n,
            'primary_slot': primary, 'groups': groups, 'members': members, 'leader': leader,
            'formation': {'oriented': oriented, 'offsets': offsets_for(n, rng)} if formation else None,
            'slot_positions': [[f2b(rng.uniform(-9, 9)), f2b(rng.uniform(-9, 9))] for _ in range(n)]}


def cases():
    rng = random.Random(0x5107)
    rows = []
    def add(name, op, org, **kw):
        rows.append({'name': name, 'operation': op, 'organization': org, 'draws': kw.pop('draws', []), **kw})
    base = organization(rng, 6, [6], leader_slot=1, primary=0)
    for slot in range(-1, 6):
        add(f'compute_oriented_{slot}', 'compute', copy.deepcopy(base), slot=slot)
    loose = copy.deepcopy(base)
    loose['formation']['oriented'] = 0
    for slot in range(6):
        add(f'compute_loose_{slot}', 'compute', copy.deepcopy(loose), slot=slot)
    unformed = copy.deepcopy(base)
    unformed['formation'] = None
    unformed['members'][3]['attached'] = 0
    unformed['members'][4] = None
    for slot in range(6):
        draws = [0x9e3779b9, 0x7fffffff] if slot == 4 else []
        add(f'compute_unformed_{slot}', 'compute', copy.deepcopy(unformed), slot=slot, draws=draws)
    for i, draws in enumerate([[0, 0], [0xffffffff, 0xffffffff], [0x80000000, 0x80000000], [12345, 0x80000001],
                               [0x7ff, 0x40000000], [2047, 1], [2048, 0xfffffffe], [0x13579bdf, 0x2468ace0]]):
        for leader, primary in [(True, 0), (False, 2), (False, -1)]:
            org = copy.deepcopy(unformed)
            if not leader:
                org['leader'] = None
            org['primary_slot'] = primary
            add(f'compute_empty_{i}_{leader}_{primary}', 'compute', org, slot=4,
                draws=draws if (leader or primary >= 0) else [])
    for leader_slot in (0, 2, 5):
        org = organization(rng, 6, [3, 3], leader_slot=leader_slot, primary=1)
        add(f'compute_all_{leader_slot}', 'compute_all', org)
    for i in range(40):
        n = rng.randint(1, 12)
        cuts = sorted(rng.sample(range(1, n), rng.randint(0, min(3, n - 1)))) if n > 1 else []
        groups = [b - a for a, b in zip([0] + cuts, cuts + [n])]
        empty = set(rng.sample(range(n), rng.randint(0, n // 3)))
        leader_slot = rng.choice([s for s in range(n) if s not in empty] or [None])
        oriented = rng.choice([1, 1, 0])
        org = organization(rng, n, groups, oriented=oriented, leader_slot=leader_slot,
                           primary=rng.choice([-1, 0, rng.randrange(n)]), empty=empty)
        draws = []
        if org['leader'] is not None or org['primary_slot'] >= 0:
            for slot in range(n):
                if slot in empty and slot != org['primary_slot'] and not (oriented and org['formation']):
                    draws += [rng.getrandbits(32), rng.getrandbits(32)]
        if org['leader'] is None:
            continue
        add(f'compute_all_random_{i}', 'compute_all', org, draws=draws)
        add(f'reassign_random_{i}', 'reassign', copy.deepcopy(org))
    # Crafted reassignment: members far from their own slots so swaps occur.
    for i in range(24):
        n = rng.randint(2, 9)
        groups = [n] if rng.random() < 0.6 else [n // 2, n - n // 2]
        groups = [g for g in groups if g > 0]
        org = organization(rng, n, groups, leader_slot=rng.randrange(n))
        center = [rng.uniform(30, 50), rng.uniform(30, 50)]
        org['center'] = [f2b(center[0]), f2b(center[1])]
        perm = list(range(n))
        rng.shuffle(perm)
        for slot, member in enumerate(org['members']):
            if member is not None:
                target = org['slot_positions'][perm[slot]]
                member['position'] = [f2b(center[0] + struct.unpack('<f', struct.pack('<I', target[0]))[0]
                                          + rng.uniform(-0.3, 0.3)),
                                      f2b(center[1] + struct.unpack('<f', struct.pack('<I', target[1]))[0]
                                          + rng.uniform(-0.3, 0.3))]
        org['leader'] = org['members'][rng.randrange(n)]
        add(f'reassign_permuted_{i}', 'reassign', org)
    # Equal costs and empty members (zero columns) exercise ties/epsilon.
    tie = organization(rng, 4, [4], leader_slot=0)
    tie['slot_positions'] = [[f2b(1.0), 0], [f2b(-1.0), 0], [0, f2b(1.0)], [0, f2b(-1.0)]]
    tie['center'] = [f2b(40.0), f2b(40.0)]
    for slot, member in enumerate(tie['members']):
        member['position'] = [f2b(40.0), f2b(40.0)]
    add('reassign_all_equal', 'reassign', tie)
    zero = copy.deepcopy(tie)
    zero['members'][2] = None
    zero['members'][3] = None
    zero['members'][1]['position'] = [f2b(40.0), f2b(39.0)]
    add('reassign_empty_columns', 'reassign', zero)
    single = organization(rng, 3, [1, 1, 1], leader_slot=0)
    add('reassign_singleton_groups', 'reassign', single)
    nolead = organization(rng, 3, [3], leader_slot=None)
    add('reassign_no_leader', 'reassign', nolead)
    # Leader changes.
    lead = organization(rng, 5, [5], leader_slot=1, primary=0)
    lead['formation']['oriented'] = 0
    for new in (None, 2, 1, 4):
        org = copy.deepcopy(lead)
        add(f'set_leader_{new}', 'set_leader', org, new_leader=org['members'][new] if new is not None else None)
    org = copy.deepcopy(lead)
    org['owner_has_mind'] = False
    org['leader'] = None
    add('set_leader_from_none_no_mind', 'set_leader', org, new_leader=org['members'][3])
    return rows


def main():
    out = []
    for precision in PRECISION:
        for t in cases():
            c = copy.deepcopy(t)
            c['precision'] = precision
            m = FormationMachine(c)
            try:
                c['output'] = m.run()
            except NullDereference as error:
                # A real process faults here; Rust must return an explicit error.
                c['output'] = {'null_dereference': str(error), 'events': m.events}
            out.append(c)
    write_document('native-formation.json', __doc__, out)


if __name__ == '__main__':
    main()
