"""Oracle: interned zone sets (KKC_Zone sets, .\\Source\\Sim\\Zones.cpp) through the
tile entry points 0x54FA10 (add a zone's flags to a tile) and 0x54FA80 (remove).

Executes the ORIGINAL leaf/union classes (vtables 0xA323E8 / 0xA32388):
516360 leaf intern, 5162D0, 515FA0, 514EF0 (leaf rehash), 516410/516480 add,
516460/516560 remove, 516290 rebuild, 5161C0/515FF0 union intern, 515DE0 +
5154E0 + 514960 structural equality, 514E90/514800 union insert/rehash,
5160D0/516160/515C90/515D20 destruction, refcounts in 54FA10/54FA80.

Each case is a random sequence of operations on a small map shared by all
tiles (the intern tables are global, so one tile's sets can reorder another's
two-element sets). After every operation the touched tile's list (native
iteration order) and the zone callbacks (+0x14 after add, +0x10 after remove)
are recorded; at the end of a case every tile and every interned set with its
reference count. Writes native-zone-sets.json.
"""
import json, random, sys
from pathlib import Path
from zones_machine import Machine, NativeFault, header, instructions, visited_in
from zones_world import World, LEAF_VTABLE

HERE = Path(__file__).resolve().parent


def make_zone(w, actor, flags, idx):
    m = w.m
    z = m.alloc(0x18)

    def added(m, idx=idx):
        w.events.append(['zone_added', idx, w.tile_index(m.arg(1)), m.arg(2) & 0xffff])
        m.ret(0, 8)

    def removed(m, idx=idx):
        w.events.append(['zone_removed', idx, w.tile_index(m.arg(1))])
        m.ret(0, 4)
    vt = m.vtable({0x10: removed, 0x14: added})
    m.w(z, vt)
    m.w(z + 4, actor)
    m.w(z + 8, 0xbf800000)
    m.w16(z + 0x14, flags)
    return z


def run_case(seed):
    rng = random.Random(seed)
    m = Machine('Double53')
    w = World(m, width=4, height=2, kingdoms=[{'index': 0}], table_size=rng.choice([1, 3, 7]))
    n_actors = rng.randint(2, 6)
    actors = [w.actor(record=False) for _ in range(n_actors)]
    zones = []
    for i in range(rng.randint(3, 9)):
        a = rng.randrange(n_actors)
        flags = rng.choice([1 << rng.randrange(9), 1 << rng.randrange(9), rng.randrange(1, 0x200),
                            rng.randrange(0, 0x10000)])
        zones.append({'actor': a, 'flags': flags})
    zone_ptrs = [make_zone(w, actors[z['actor']], z['flags'], i) for i, z in enumerate(zones)]
    ops = []
    ntiles = w.width * w.height
    for step in range(rng.randint(10, 60)):
        t = rng.randrange(min(ntiles, rng.choice([2, 3, ntiles])))
        tile = w.tiles + 0x30 * t
        current = w.set_list(m.r(tile + 4))
        present = {a for a, f in current}
        z = rng.randrange(len(zones))
        kind = rng.random()
        del w.events[:]
        if kind < 0.55 or not current:
            # add: usually the zone's own flags (as the solver does), sometimes other words
            r = rng.random()
            flags = zones[z]['flags'] if r < 0.7 else (0 if r < 0.75 else rng.randrange(0x10000))
            op = {'op': 'add', 'tile': t, 'zone': z, 'flags': flags}
            if not current and rng.random() < 0.08:
                op = {'op': 'remove', 'tile': t, 'zone': z}
        else:
            # remove: only for actors present or a one-leaf set (absent actor in a
            # longer list is undefined behaviour in 516560: it walks into a leaf)
            cands = [i for i, zz in enumerate(zones) if zz['actor'] in present]
            if len(current) == 1:
                cands = list(range(len(zones)))
            z = rng.choice(cands)
            op = {'op': 'remove', 'tile': t, 'zone': z}
        try:
            if op['op'] == 'add':
                m.call(0x54fa10, ecx=tile, args=(zone_ptrs[z], op['flags']), pop=8)
            else:
                m.call(0x54fa80, ecx=tile, args=(zone_ptrs[z],), pop=4)
            op['fault'] = None
        except NativeFault as e:
            op['fault'] = str(e)
        op['events'] = list(w.events)
        op['list'] = w.set_list(m.r(tile + 4))
        ops.append(op)
    final = [w.set_list(m.r(w.tiles + 0x30 * t + 4)) for t in range(ntiles)]
    live = w.live_sets()
    m.verify_v14()
    return {'seed': seed, 'actors': n_actors, 'zones': zones, 'ops': ops, 'final_tiles': final,
            'live_sets': live}, m.visited


def main():
    cases, visited = [], {}
    for seed in range(400):
        c, v = run_case(seed)
        cases.append(c)
        visited.update(v)
    faults = sum(1 for c in cases for o in c['ops'] if o['fault'])
    out = header(__file__, 'Unicorn 2.1.4, original AA.exe zone-set classes; allocator seamed (0xCD fill)')
    out.update({'cases': cases, 'case_count': len(cases), 'op_count': sum(len(c['ops']) for c in cases),
                'faults': faults,
                'coverage': {hex(a): len(visited_in(visited, a, a + s)) for a, s in (
                    (0x516360, 0xac), (0x516410, 0x45), (0x516460, 0x1f), (0x516480, 0xd4),
                    (0x516560, 0xd5), (0x516290, 0x3b), (0x5161c0, 0xa4), (0x515ff0, 0xb8),
                    (0x5154e0, 0x92), (0x514960, 0x85), (0x514e90, 0x5e), (0x514800, 0x106),
                    (0x514ef0, 0xcf), (0x515d20, 0xb1), (0x515c90, 0x8e), (0x5160d0, 0x8f),
                    (0x515de0, 0x37), (0x515fa0, 0x4f), (0x54fa10, 0x6f), (0x54fa80, 0x56))},
                'instructions': instructions(visited)})
    (HERE / 'native-zone-sets.json').write_text(json.dumps(out, separators=(',', ':')))
    print('cases', len(cases), 'ops', out['op_count'], 'faults', faults)
    print(out['coverage'])


if __name__ == '__main__':
    main()
