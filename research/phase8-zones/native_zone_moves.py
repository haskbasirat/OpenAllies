"""Oracle: KKC_Zone move/release with the ZoneSolver fast circle builder
(original code, three x87 precisions).

Entry points executed: 0x516640 (constructor), 0x515F40 (move to world
position: 0x552640 clamp, _ftol 0x96CC4C, sar), 0x515970 (KKC_Zone::Move),
0x514CE0 (release), 0x5406D0 (type-1 reveal-mask change), 0x515940 (state
bit). Downstream, all original: 0x540860/0x540590 fast branches, 0x540020,
0x5401A0, 0x53FE90 (+ _CIfmod 0x96DB7A), 0x53F730, 0x53FD30/0x53FDE0/
0x53FF00, 0x53F650/0x53F6C0, 0x54FA10/0x54FA80 and the tile callbacks
(proven separately in native-zone-tiles.json).

Fast-eligible zones only (types 1, 2, 6 with SVar ZoneBuilderFastDetectionZones
set); the flood-fill paths 0x5402E0 / 0x4F4050 are not ported yet and are
never reached here (a reach would stop at an unmapped vtable slot).
Seams: SAI hooks, 404F60 log, 50C4C0 presentation, actor virtuals. Per op:
ordered events, faults, zone states and the tiles that changed.
Writes native-zone-moves.json.
"""
import json, random, struct
from pathlib import Path
from zones_machine import Machine, NativeFault, header, instructions, visited_in, f32
from zones_world import ZoneWorld
from unicorn.x86_const import UC_X86_REG_ESP

HERE = Path(__file__).resolve().parent
PRECS = ['Single24', 'Double53', 'Extended64']


def rand_f32(rng, lo, hi):
    return f32(rng.uniform(lo, hi))


def run_case(seed, prec):
    rng = random.Random(seed)
    m = Machine(prec)
    W, H = rng.randint(6, 16), rng.randint(5, 12)
    shift = rng.choice([2, 3, 3, 4])
    tile = rng.choice([float(1 << shift)] * 3 + [rng.choice([3.0, 5.0, 6.5, 7.25, 12.0, 1.5])])
    nk = rng.randint(1, 4)
    ks = [{'index': rng.choice([i, i, i, rng.randrange(20)]), 'nonmajor': rng.choice([0, 0, 0, 0, 1]),
           'shared': rng.choice([None, rng.randrange(1 << 32)])} for i in range(nk)]
    w = ZoneWorld(m, width=W, height=H, shift=shift, tile_size=tile, kingdoms=ks,
                  fog_enabled=rng.choice([0, 1, 1, 1]), sai=rng.random() < 0.4,
                  solvers=(rng.random() < 0.95, rng.random() < 0.95))
    real = [i for i, k in enumerate(w.kingdoms) if k]
    na = rng.randint(1, 5)
    actors, ainfo = [], []
    for a in range(na):
        info = {'owner': rng.choice([None] + real * 3), 'allied': [], 'enemy': [],
                'flags': rng.choice([0, 0, 0, 1, 0x4000]),
                'reveal_mask': rng.choice([None, 0, rng.randrange(1 << nk), (1 << nk) - 1])}
        ainfo.append(info)
        actors.append(w.full_actor(owner=info['owner'], flags=info['flags'], reveal_mask=info['reveal_mask']))
    occupants = [[] for _ in range(W * H)]
    for a in range(na):
        for _ in range(rng.choice([0, 1, 2, 3])):
            x, y = rng.randrange(W), rng.randrange(H)
            w.place(actors[a], x, y)
            occupants[y * W + x].insert(0, a)
    zones = []
    for _ in range(rng.randint(1, 4)):
        a, ty = rng.randrange(na), rng.choice([1, 2, 6, 6])
        if m.r(m.r(actors[a] + 0x148) + 4 * ty):
            continue
        w.zone(actors[a], ty)
        zones.append([a, ty])
    if not zones:
        w.zone(actors[0], 6)
        zones.append([0, 6])
    zstates = [w.zone_state(z) for z in w.zones]
    initial_actors = [dict(info, addr=actors[i]) for i, info in enumerate(ainfo)]
    prev = [w.tile_state(t) for t in range(W * H)]
    # 0x516560 walks into the final leaf when the actor is absent (memory
    # corruption): stop the case there instead of recording garbage.
    ub = []

    def check_remove(m):
        s, actor = m.ecx(), m.r(m.u.reg_read(UC_X86_REG_ESP) + 4)
        if actor not in [w.actors[e[0]] for e in w.set_list(s)]:
            ub.append(True)
            m.u.emu_stop()
    m.observe(0x516560, check_remove)
    ops = []
    for _ in range(rng.randint(6, 25)):
        zi = rng.randrange(len(zones))
        z = w.zones[zi]
        r = rng.random()
        radius = rng.choice([rand_f32(rng, 0, 4 * tile), rand_f32(rng, 0, 12 * tile), 0, f32(tile),
                             f32(2.5 * tile)] + ([0xbf800000] if rng.random() < 0.05 else []))
        del w.events[:]
        op = {'zone': zi}
        try:
            if r < 0.45:
                pos = [rand_f32(rng, -tile, (W + 1) * tile), rand_f32(rng, -tile, (H + 1) * tile)]
                op.update(op='move_to', pos=pos, radius=radius)
                m.call(0x515f40, ecx=z, args=(pos[0], pos[1], radius), pop=12)
            elif r < 0.75:
                cx, cy = m.r(z + 0xc), m.r(z + 0x10)
                if rng.random() < 0.7:
                    x = (cx + rng.choice([-1, 0, 0, 1])) & 0xffffffff
                    y = (cy + rng.choice([-1, 0, 1])) & 0xffffffff
                else:
                    x, y = rng.randrange(-2, W + 2) & 0xffffffff, rng.randrange(-2, H + 2) & 0xffffffff
                op.update(op='move', x=x, y=y, radius=radius)
                m.call(0x515970, ecx=z, args=(x, y, radius), pop=12)
            elif r < 0.85:
                op.update(op='release')
                m.call(0x514ce0, ecx=z)
            elif r < 0.95:
                setm = rng.randrange(1 << 5)
                changed = rng.choice([0, rng.randrange(1 << 5), (1 << len(w.kingdoms)) - 1])
                op.update(op='mask', set=setm, mask_changed=changed)
                m.call(0x5406d0, ecx=m.r(w.g + 0x248), args=(z, setm, changed), pop=12)
            else:
                a = rng.randrange(na)
                if ainfo[a]['reveal_mask'] is None:
                    continue
                newm = rng.randrange(1 << nk)
                ainfo[a]['reveal_mask'] = newm
                m.w(m.r(actors[a] + 0xb8) + 0x10, newm)
                op.update(op='set_reveal', actor=a, mask=newm)
            op['fault'] = None
        except NativeFault as e:
            op['fault'] = 'native UB: 0x516560 absent actor' if ub else str(e)
        op['events'] = list(w.events)
        op['zones'] = [w.zone_state(zz) for zz in w.zones]
        cur = [w.tile_state(t) for t in range(W * H)]
        op['changed'] = [[t, s] for t, s in enumerate(cur) if s != prev[t]]
        prev = cur
        ops.append(op)
        if ub:
            break
    m.verify_v14()
    g = w.map
    kout = [{'addr': p, 'index': m.r(p + 0x18c), 'nonmajor': m.r(p + 0x190), 'shared': m.r(p + 0x228),
             'supply': m.r16(p + 0x22c)} if p else None for p in w.kingdoms]
    return {'seed': seed, 'precision': prec,
            'map': {'width': W, 'height': H, 'shift': shift, 'tile': m.r(g + 8), 'extent': [m.r(g + 0xc), m.r(g + 0x10)]},
            'fog': m.r(m.r(m.r(0xB39AB0) + 0x108)), 'sai': m.rb(m.r(0xB36528) + 0x20), 'fast': 1,
            'solvers': [1 if m.r(w.g + 0x244) else 0, 1 if m.r(w.g + 0x248) else 0],
            'kingdoms': kout, 'actors': initial_actors,
            'occupants': occupants, 'zones': zones, 'zone_keys': list(w.zones), 'zone_states': zstates,
            'ops': ops}, m.visited


def main():
    cases, visited = [], {}
    for seed in range(150):
        for prec in PRECS:
            c, v = run_case(seed, prec)
            cases.append(c)
            visited.update(v)
    out = header(__file__, 'Unicorn 2.1.4, original AA.exe KKC_Zone move/release, fast ZoneSolver, PC24/53/64')
    ranges = [(0x515f40, 0x58), (0x515970, 0x15c), (0x514ce0, 0xb7), (0x540860, 0x1c7), (0x540590, 0xfa),
              (0x5406d0, 0x18f), (0x540020, 0x171), (0x5401a0, 0xff), (0x53fe90, 0x67), (0x53f730, 0xec),
              (0x53fd30, 0xa7), (0x53fde0, 0xaf), (0x53ff00, 0x87), (0x53f650, 0x6b), (0x53f6c0, 0x6b),
              (0x552640, 0x76), (0x96db7a, 0x28)]
    nops = sum(len(c['ops']) for c in cases)
    out.update({'cases': cases, 'case_count': len(cases), 'op_count': nops,
                'faults': sum(1 for c in cases for o in c['ops'] if o['fault']),
                'coverage': {hex(a): len(visited_in(visited, a, a + s)) for a, s in ranges},
                'instructions': instructions(visited)})
    (HERE / 'native-zone-moves.json').write_text(json.dumps(out, separators=(',', ':')))
    print('cases', len(cases), 'ops', nops, 'faults', out['faults'])
    print(out['coverage'])


if __name__ == '__main__':
    main()
