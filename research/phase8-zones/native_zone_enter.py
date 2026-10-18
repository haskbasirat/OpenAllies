"""Oracle: actor entering a tile (0x54EF10 -> zone-set slot +0x54, leaf
0x515620 / union 0x514FC0) executed by the ORIGINAL code.

Real KKC_Zone objects (types 2, 3, 4, 5, 8 and others) sit in their actors'
zone vectors and in tile sets (built through 0x54FA10). Executed natively:
0x54EF10, 0x514FC0, 0x515620, 0x514DA0, 0x514E40, 0x46E080, 0x432730,
0x54F4F0 and the fog-word helpers. Seams: actor virtuals +0 (size), +0x64
(reveal), +0x74 (explored, deep), +0x108 (owner), +0x140 (enemy), +0xB4;
ActorData virtual +0x24 (marker); the circle iteration 0x554DD0/0x554330
(a per-actor tile list); component effects; SAI hooks. The SVar
MarkerRevealPadding static is pre-initialised (binary32 4.0, shipped value).
Recorded per op: events, faults, every tile, zone actors' +8 flags.
Writes native-zone-enter.json.
"""
import json, random
from pathlib import Path
from zones_machine import Machine, NativeFault, header, instructions, visited_in, f32
from zones_world import ZoneWorld

HERE = Path(__file__).resolve().parent
PRECS = ['Single24', 'Double53', 'Extended64']


def run_case(seed, prec):
    rng = random.Random(seed)
    m = Machine(prec)
    nk = rng.randint(1, 4)
    ks = [{'index': i, 'nonmajor': rng.choice([0, 0, 0, 1])} for i in range(nk)]
    W, H = 5, 4
    w = ZoneWorld(m, width=W, height=H, kingdoms=ks, fog_enabled=rng.choice([0, 1, 1, 1]), sai=rng.random() < 0.4,
                  effects_gate=rng.random() < 0.85)
    ev = w.events
    flag48 = rng.choice([0, 1, 1])
    m.w(w.g + 0x48, flag48)
    pad = m.alloc(4)
    m.w(pad, f32(4.0))
    m.w(0xB3CE58, m.r(0xB3CE58) | 1)
    m.w(0xB3CE54, pad)
    real = [i for i, k in enumerate(w.kingdoms) if k]
    na = rng.randint(2, 6)
    actors, info = [], []
    for a in range(na):
        comps = {off: rng.choice([0, 0x50000000 + a * 0x100 + off]) for off in (0x7c, 0x88, 0x90, 0xb0, 0xd4)}
        i = {'owner': rng.choice([None] + real * 3), 'allied': [], 'enemy': sorted(k for k in real if rng.random() < 0.5),
             'flags': rng.choice([0x4000, 0x4000, 0, 0x40004000, 0x40000000, 1]), 'b4': 1, 'components': comps,
             'tile': [rng.randrange(W), rng.randrange(H)], 'multi': 0, 'size': f32(rng.choice([1.0, 2.5, 6.0])),
             'marker': rng.choice([0, 1, 1]), 'explored_deep': rng.choice([0, 0, 1]),
             'has_comp': rng.choice([0, 1, 1]), 'discovery': [rng.randrange(W * H) for _ in range(rng.randint(0, 5))],
             'zone_slots': rng.choice([9, 9, 9, 4])}
        info.append(i)
        p = w.full_actor(owner=i['owner'], enemy=i['enemy'], flags=i['flags'], components=comps, tile=tuple(i['tile']))
        actors.append(p)
        if i['zone_slots'] != 9:
            m.w(p + 0x14c, i['zone_slots'])
        vt = m.r(p)
        m.w(vt + 0x0, m.stub(lambda m, a=a: (ev.append(['size', a]), m.ret_st0_f32(info[a]['size']))))
        m.w(vt + 0x74, m.stub(lambda m, a=a: (ev.append(['explored_deep', a, w.kingdom_index(m.arg(1)), m.arg(2) & 0xff]),
                                               m.ret(info[a]['explored_deep'], 8))))
        data = m.alloc(0x40)
        m.w(data, m.vtable({0x24: lambda m, a=a: (ev.append(['marker', a]), m.ret(info[a]['marker']))}))
        m.w(p + 4, data)
        m.w(p + 0xb8, m.alloc(0x20) if i['has_comp'] else 0)
        if i['has_comp']:
            m.w(m.r(p + 0xb8) + 0x10, 0)
    for a in range(na):
        r = rng.random()
        if r < 0.3:
            info[a]['redirect'] = [rng.randrange(na), rng.choice([0, 1])]
            c = m.alloc(0x20)
            m.w(c + 0x10, actors[info[a]['redirect'][0]])
            m.wb(c + 0x1c, info[a]['redirect'][1])
            m.w(actors[a] + 0x80, c)
    zones = []
    for a in range(na):
        for ty in rng.sample([2, 2, 3, 4, 5, 8, 6], rng.randint(0, 4)):
            if ty >= info[a]['zone_slots'] or m.r(m.r(actors[a] + 0x148) + 4 * ty):
                continue
            w.zone(actors[a], ty)
            zones.append([a, ty])
    # tile sets: real zones added through 54FA10 (callbacks run, events dropped)
    for t in range(W * H):
        for _ in range(rng.choice([0, 1, 2, 3])):
            if not zones:
                break
            zi = rng.randrange(len(zones))
            z = w.zones[zi]
            try:
                m.call(0x54fa10, ecx=w.tiles + 0x30 * t, args=(z, m.r16(z + 0x14)), pop=8)
            except NativeFault:
                pass
    # occupants for reveal callbacks of 54F4F0
    occupants = [[] for _ in range(W * H)]
    for a in range(na):
        if rng.random() < 0.5:
            x, y = rng.randrange(W), rng.randrange(H)
            w.place(actors[a], x, y)
            occupants[y * W + x].insert(0, a)
    iters = {}
    cur = [0]

    def it_setup(m):
        it = m.ecx()
        iters[it] = list(info[cur[0]]['discovery'])
        m.w(it, w.map)
        m.w(it + 0x10, 100)
        nxt(m, it)
        m.ret(0, 0x10)

    def nxt(m, it):
        tiles = iters[it]
        if tiles:
            t = tiles.pop(0)
            m.w(it + 0x24, t % W)
            m.w(it + 0x28, t // W)
        else:
            m.w(it + 0x28, 101)
    m.seam(0x554dd0, it_setup)
    m.seam(0x554330, lambda m: (nxt(m, m.ecx()), m.ret(0, 4)))
    # discovery lists keyed by the zone actor whose circle is iterated: the
    # iteration happens inside 515620 for the leaf actor; track it via 46E080's caller
    orig_size = {}

    def track(m):
        # observer at 0x5157A0 (zone actor's size call): ESI = zone actor
        from unicorn.x86_const import UC_X86_REG_ESI
        cur[0] = w.actor_idx(m.u.reg_read(UC_X86_REG_ESI))
    m.observe(0x5157a0, track)
    initial_tiles = [w.tile_state(t) for t in range(W * H)]
    prev = list(initial_tiles)
    initial = [dict(i, addr=actors[a]) for a, i in enumerate(info)]
    ops = []
    for _ in range(rng.randint(5, 25)):
        t = rng.randrange(W * H)
        a = rng.randrange(na)
        del ev[:]
        op = {'tile': t, 'actor': a}
        try:
            m.call(0x54ef10, ecx=w.tiles + 0x30 * t, args=(actors[a],), pop=4)
            op['fault'] = None
        except NativeFault as e:
            op['fault'] = str(e)
        op['events'] = list(ev)
        cur_t = [w.tile_state(x) for x in range(W * H)]
        op['changed'] = [[x, s] for x, s in enumerate(cur_t) if s != prev[x]]
        prev = cur_t
        op['flags'] = [m.r(p + 8) for p in actors]
        ops.append(op)
    m.verify_v14()
    kout = [{'addr': p, 'index': m.r(p + 0x18c), 'nonmajor': m.r(p + 0x190), 'shared': m.r(p + 0x228),
             'supply': m.r16(p + 0x22c)} if p else None for p in w.kingdoms]
    return {'seed': seed, 'precision': prec, 'fog': m.r(m.r(m.r(0xB39AB0) + 0x108)),
            'sai': m.rb(m.r(0xB36528) + 0x20), 'effects_gate': m.r(m.r(0xB36550) + 0xe4) != 3, 'flag48': flag48,
            'kingdoms': kout, 'actors': initial, 'occupants': occupants, 'zones': zones,
            'zone_states': [w.zone_state(z) for z in w.zones], 'initial_tiles': initial_tiles, 'ops': ops}, m.visited


def main():
    cases, visited = [], {}
    for seed in range(250):
        for prec in PRECS:
            c, v = run_case(seed, prec)
            cases.append(c)
            visited.update(v)
    out = header(__file__, 'Unicorn 2.1.4, original AA.exe actor-entered zone effects (0x54EF10/0x515620), PC24/53/64')
    ranges = [(0x54ef10, 0x2d), (0x514fc0, 0x3c), (0x515620, 0x278), (0x514da0, 0x5b), (0x514e40, 0x4f),
              (0x46e080, 0x20), (0x432730, 0x2e)]
    nops = sum(len(c['ops']) for c in cases)
    out.update({'cases': cases, 'case_count': len(cases), 'op_count': nops,
                'faults': sum(1 for c in cases for o in c['ops'] if o['fault']),
                'coverage': {hex(a): len(visited_in(visited, a, a + s)) for a, s in ranges},
                'instructions': instructions(visited)})
    (HERE / 'native-zone-enter.json').write_text(json.dumps(out, separators=(',', ':')))
    print('cases', len(cases), 'ops', nops, 'faults', out['faults'])
    print(out['coverage'])


if __name__ == '__main__':
    main()
