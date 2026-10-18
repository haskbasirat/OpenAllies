"""Oracle: zone tile callbacks and fog/supply tile routines (original code).

Real KKC_Zone objects (0x516640, vtable 0xA3229C) of all nine types are added
to and removed from tiles through 0x54FA10 / 0x54FA80, which run the zone
virtuals +0x14 (0x515AD0: 0x54F7F0 actor effects 0x514DA0/0x514E00/0x514E20/
0x514E40, 0x54F4F0 reveal, 0x54ED20 occupy, type-1 0x54F670) and +0x10
(0x515BA0: 0x54F5E0 hide, 0x54F6A0 unoccupy, 0x54F690). Also direct calls of
0x54F930 (fog restored in area, per tile), 0x54F670, 0x54F690, 0x54F3C0,
0x54F3E0. Recorded: ordered events (actor virtual calls +0x108/+0x138/+0x140/
+0x64/+0x68/+0x6C/+0xB4, 0x4EE700, 0x4EA200, component effects, SAI hooks),
results, native faults, and every tile's (set, fog word, supply word) after
each operation. No FPU use. Writes native-zone-tiles.json.
"""
import json, random
from pathlib import Path
from zones_machine import Machine, NativeFault, header, instructions, visited_in
from zones_world import ZoneWorld

HERE = Path(__file__).resolve().parent


def run_case(seed):
    rng = random.Random(seed)
    m = Machine('Double53')
    nk = rng.randint(1, 4)
    ks = [{'index': rng.choice([i, i, i, rng.randrange(20)]), 'nonmajor': rng.choice([0, 0, 0, 1]),
           'shared': rng.choice([None, rng.randrange(1 << 32)]),
           'supply': rng.choice([None, rng.randrange(1 << 16)])} for i in range(nk)]
    if rng.random() < 0.2:
        ks.insert(rng.randrange(len(ks) + 1), None)
    w = ZoneWorld(m, width=4, height=3, kingdoms=ks, fog_enabled=rng.choice([0, 1, 1, 1]),
                  sai=rng.random() < 0.5, effects_gate=rng.random() < 0.8)
    real = [i for i, k in enumerate(w.kingdoms) if k]
    na = rng.randint(2, 6)
    actors, ainfo = [], []
    for a in range(na):
        owner = rng.choice([None] + real * 3)
        comps = {off: rng.choice([0, 0x50000000 + a * 0x100 + off]) for off in (0x7c, 0x88, 0x90, 0xb0, 0xd4)}
        info = {'owner': owner, 'allied': sorted(k for k in real if rng.random() < 0.4),
                'enemy': sorted(k for k in real if rng.random() < 0.5),
                'flags': rng.choice([0, 0x4000, 0x4000, 0x4001, 1, 0x20000400]),
                'reveal_mask': rng.choice([None, 0, rng.randrange(1 << 18), (1 << len(w.kingdoms)) - 1]),
                'b4': rng.choice([0, 1, 1]), 'components': comps,
                'tile': [rng.randrange(4), rng.randrange(3)], 'multi': rng.choice([0, 0, 1])}
        ainfo.append(info)
        actors.append(None)
    for a, info in enumerate(ainfo):
        redirect = None
        if rng.random() < 0.3:
            redirect = [rng.randrange(na), rng.choice([0, 1])]
            info['redirect'] = redirect
        actors[a] = w.full_actor(owner=info['owner'], allied=info['allied'], enemy=info['enemy'],
                                 flags=info['flags'], reveal_mask=info['reveal_mask'], b4=info['b4'],
                                 components=info['components'], tile=tuple(info['tile']), multi=info['multi'])
    for a, info in enumerate(ainfo):
        if 'redirect' in info:
            r, byte = info['redirect']
            c = m.r(actors[a] + 0x80)
            m.w(c + 0x10, actors[r])
            m.wb(c + 0x1c, byte)
    # tile occupants (lists hold placement order head-first)
    occupants = [[] for _ in range(12)]
    for a in range(na):
        for _ in range(rng.choice([0, 1, 1, 2])):
            x, y = rng.randrange(4), rng.randrange(3)
            w.place(actors[a], x, y)
            occupants[y * 4 + x].insert(0, a)
    for t in range(12):
        p = w.tiles + 0x30 * t
        m.w(p + 0x14, rng.choice([0, rng.randrange(1 << 32)]))
        m.w16(p + 0x18, rng.choice([0, rng.randrange(1 << 16)]))
    initial = [w.tile_state(t) for t in range(12)]
    zones = []
    for _ in range(rng.randint(2, 8)):
        a, ty = rng.randrange(na), rng.randrange(9)
        if m.r(m.r(actors[a] + 0x148) + 4 * ty):
            continue
        w.zone(actors[a], ty)
        zones.append([a, ty])
    zstates = [w.zone_state(z) for z in w.zones]
    ops = []
    for _ in range(rng.randint(10, 40)):
        t = rng.randrange(12)
        tile = w.tiles + 0x30 * t
        r = rng.random()
        del w.events[:]
        op = {'tile': t}
        try:
            if r < 0.45 and zones:
                zi = rng.randrange(len(zones))
                op.update(op='add', zone=zi)
                m.call(0x54fa10, ecx=tile, args=(w.zones[zi], m.r16(w.zones[zi] + 0x14)), pop=8)
            elif r < 0.7 and zones:
                present = {e[0] for e in w.set_list(m.r(tile + 4))}
                cands = [i for i, (a, ty) in enumerate(zones) if a in present]
                if not cands:
                    continue
                zi = rng.choice(cands)
                op.update(op='remove', zone=zi)
                m.call(0x54fa80, ecx=tile, args=(w.zones[zi],), pop=4)
            elif r < 0.8:
                op.update(op='restore')
                m.call(0x54f930, ecx=tile)
            else:
                ki = rng.randrange(len(w.kingdoms))
                kind = rng.choice(['reveal', 'hide', 'visible', 'shared_visible'])
                op.update(op=kind, kingdom=ki)
                fn = {'reveal': 0x54f670, 'hide': 0x54f690, 'visible': 0x54f3c0, 'shared_visible': 0x54f3e0}[kind]
                v = m.call(fn, ecx=tile, args=(w.kingdoms[ki],), pop=4)
                if kind in ('visible', 'shared_visible'):
                    op['result'] = v & 0xff
            op['fault'] = None
        except NativeFault as e:
            op['fault'] = str(e)
        op['events'] = list(w.events)
        op['tiles'] = [w.tile_state(i) for i in range(12)]
        ops.append(op)
    m.verify_v14()
    kout = [{'addr': p, 'index': m.r(p + 0x18c), 'nonmajor': m.r(p + 0x190), 'shared': m.r(p + 0x228),
             'supply': m.r16(p + 0x22c)} if p else None for p in w.kingdoms]
    return {'seed': seed, 'fog': m.r(m.r(m.r(0xB39AB0) + 0x108)), 'sai': m.rb(m.r(0xB36528) + 0x20),
            'effects_gate': m.r(m.r(0xB36550) + 0xe4) != 3, 'kingdoms': kout,
            'actors': [dict(info, addr=actors[i]) for i, info in enumerate(ainfo)],
            'occupants': occupants, 'zones': zones, 'zone_keys': list(w.zones), 'zone_states': zstates,
            'initial_tiles': initial, 'ops': ops}, m.visited


def main():
    cases, visited = [], {}
    for seed in range(400):
        c, v = run_case(seed)
        cases.append(c)
        visited.update(v)
    out = header(__file__, 'Unicorn 2.1.4, original AA.exe zone tile callbacks and fog/supply routines')
    ranges = [(0x516640, 0x71), (0x515ad0, 0xc8), (0x515ba0, 0xa8), (0x54f7f0, 0x7a), (0x514da0, 0x5b),
              (0x514e00, 0x1b), (0x514e20, 0x1b), (0x514e40, 0x4f), (0x54f4f0, 0xe3), (0x54f5e0, 0x8d),
              (0x54f6a0, 0x4b), (0x54ed20, 0x3d), (0x54f930, 0xd8), (0x54f670, 0x13), (0x54f3c0, 0x1d),
              (0x54f3e0, 0x1d), (0x54ec70, 0x31), (0x54ecb0, 0x2c), (0x54ea80, 0x2b), (0x54eab0, 0x2d),
              (0x54eae0, 0x2c), (0x54eb10, 0x2f), (0x54eb70, 0x2d), (0x559530, 0x37), (0x559570, 0x23),
              (0x5595a0, 0x25)]
    nops = sum(len(c['ops']) for c in cases)
    out.update({'cases': cases, 'case_count': len(cases), 'op_count': nops,
                'faults': sum(1 for c in cases for o in c['ops'] if o['fault']),
                'coverage': {hex(a): len(visited_in(visited, a, a + s)) for a, s in ranges},
                'instructions': instructions(visited)})
    (HERE / 'native-zone-tiles.json').write_text(json.dumps(out, separators=(',', ':')))
    print('cases', len(cases), 'ops', nops, 'faults', out['faults'])
    print(out['coverage'])


if __name__ == '__main__':
    main()
