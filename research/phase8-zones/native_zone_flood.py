"""Oracle: the ZoneSolver / flood-solver paths (original code, three precisions).

Real solver objects are built by the original constructors: the ZoneSolver at
GWorld+0x248 (0x5402A0: vtable 0xA36954, open list 0x53CDF0 with its native
0x1F9C limit) and the flood solver at GWorld+0x244 (0x53F840: vtable 0xA36048,
FIFO 0xA3602C). Zones of every type move/release through 0x515F40 /
0x515970 / 0x514CE0 (slow types always flood; fast types flood when the SVar
is clear), reveal-mask changes run 0x5406D0, and 0x5147C0 runs the effects
walk (0x540690). Executed natively: 0x540860 slow branch, 0x5402E0, 0x53F9F0,
0x53F970, 0x53C5C0, 0x53C630, 0x53F9B0, 0x53C410, 0x552440, 0x53C430,
0x53C590, 0x554ED0, 0x4F2000, 0x4F2760, 0x4F14A0, 0x53D2A0/0x53D090,
0x53D150, 0x53CE20, 0x53D2F0, 0x539AD0, 0x4F4050, 0x53F890, 0x53F8D0,
0x539B10, 0x53F5B0, 0x53F880, 0x53F610, 0x53F600, 0x539DB0, 0x539D20,
0x53F5D0, 0x539DF0, 0x539AC0, 0x45BC70 and everything downstream.

Tile inputs: +0x20 flags (bit 0 impassable, bit 8 terrain class), +0x1C GTIDs
resolved through a synthetic [B364C0] registry (blockers), random initial
+0x24 scratch (binary32 costs) and GMap+0x48 stamp. Seams: 855D30 assertion
(returns 0), SAI hooks, 404F60, 50C4C0, actor virtuals, component effects.
Recorded per op: events, faults, zone states, changed tiles, every tile's
+0x24 (bits, or ['link', tile] for a pointer into the tile array), the stamp
and the busy byte. Writes native-zone-flood.json.
"""
import json, random
from pathlib import Path
from zones_machine import Machine, NativeFault, header, instructions, visited_in, f32
from zones_world import ZoneWorld
from unicorn.x86_const import UC_X86_REG_ESP

HERE = Path(__file__).resolve().parent
PRECS = ['Single24', 'Double53', 'Extended64']
REGISTRY = 0xB364C0
BUSY = 0xB3D43D


def run_case(seed, prec):
    rng = random.Random(seed)
    m = Machine(prec)
    W, H = rng.randint(8, 14), rng.randint(6, 11)
    shift = rng.choice([3, 3, 3, 2, 4])
    tile = float(1 << shift)   # GMap tile size is 1 << shift (consistent maps only)
    nk = rng.randint(1, 3)
    ks = [{'index': i, 'nonmajor': rng.choice([0, 0, 0, 1])} for i in range(nk)]
    fast = 1 if rng.random() < 0.7 else 0
    w = ZoneWorld(m, width=W, height=H, shift=shift, tile_size=tile, kingdoms=ks, fast=fast,
                  fog_enabled=rng.choice([0, 1, 1]), sai=rng.random() < 0.4)
    g = w.g
    # original solver objects
    zs = m.alloc(0x58, fill=0xcd)
    m.call(0x5402a0, ecx=zs)
    fs = m.alloc(0x40, fill=0xcd)
    m.call(0x53f840, ecx=fs)
    m.w(g + 0x248, zs)
    m.w(g + 0x244, fs)
    m.w(w.map + 0x48, rng.randrange(1, 1000))
    m.wb(BUSY, 0)
    m.seam(0x855d30, lambda m: (w.events.append(['assert_blocked_start']), m.ret(0)))
    # GTID registry for blockers (45BC70)
    reg = m.alloc(0x20004 + 4 * 0x10000)
    m.w(REGISTRY, reg)
    ntiles = W * H
    flags, blockers = [], []
    water = (rng.randrange(W), rng.randrange(H), rng.randrange(0, 4))
    slot = 1
    for t in range(ntiles):
        x, y = t % W, t // W
        border = x == 0 or y == 0 or x == W - 1 or y == H - 1
        f = 1 if (border or rng.random() < 0.08) else 0
        if abs(x - water[0]) + abs(y - water[1]) <= water[2]:
            f |= 0x100
        if rng.random() < 0.05:
            f |= rng.choice([0x2, 0x400, 0x10])
        p = w.tiles + 0x30 * t
        m.w(p + 0x20, f)
        flags.append(f)
        b = 0
        if rng.random() < 0.08:
            gen = rng.randrange(1, 0xffff)
            m.w16(reg + 4 + slot * 2, gen)
            b = 0x60000000 + slot * 0x10
            m.w(reg + 0x20004 + slot * 4, b)
            m.w(p + 0x1c, (gen << 16) | slot)
            slot += 1
        elif rng.random() < 0.05:
            m.w(p + 0x1c, (0x1234 << 16) | 0x7777)   # stale GTID
        else:
            m.w(p + 0x1c, 0)
        blockers.append(b)
        m.w(p + 0x24, rng.choice([0, 0, f32(rng.uniform(0, 80)), 0x7f7fffff]))
    real = [i for i, k in enumerate(w.kingdoms) if k]
    na = rng.randint(1, 4)
    actors, ainfo = [], []
    for a in range(na):
        comps = {off: rng.choice([0, 0x50000000 + a * 0x100 + off]) for off in (0x7c, 0x88, 0x90, 0xb0, 0xd4)}
        info = {'owner': rng.choice([None] + real * 3), 'allied': [], 'enemy': sorted(k for k in real if rng.random() < 0.5),
                'flags': rng.choice([0, 0x4000, 0x4000, 1]), 'reveal_mask': rng.choice([0, (1 << nk) - 1, 1]),
                'b4': 1, 'components': comps, 'tile': [rng.randrange(W), rng.randrange(H)], 'multi': rng.choice([0, 1])}
        ainfo.append(info)
        actors.append(w.full_actor(owner=info['owner'], enemy=info['enemy'], flags=info['flags'],
                                   reveal_mask=info['reveal_mask'], components=comps, tile=tuple(info['tile']),
                                   multi=info['multi']))
    occupants = [[] for _ in range(ntiles)]
    for a in range(na):
        for _ in range(rng.choice([1, 2, 3])):
            x, y = rng.randrange(W), rng.randrange(H)
            w.place(actors[a], x, y)
            occupants[y * W + x].insert(0, a)
    zones = []
    for _ in range(rng.randint(1, 4)):
        a, ty = rng.randrange(na), rng.choice([0, 3, 4, 4, 5, 5, 7, 8, 1, 2, 6])
        if m.r(m.r(actors[a] + 0x148) + 4 * ty):
            continue
        w.zone(actors[a], ty)
        zones.append([a, ty])
    if not zones:
        w.zone(actors[0], 4)
        zones.append([0, 4])
    zstates = [w.zone_state(z) for z in w.zones]
    initial_actors = [dict(info, addr=actors[i]) for i, info in enumerate(ainfo)]
    ub = []

    def check_remove(m):
        s, actor = m.ecx(), m.r(m.u.reg_read(UC_X86_REG_ESP) + 4)
        if actor not in [w.actors[e[0]] for e in w.set_list(s)]:
            ub.append(True)
            m.u.emu_stop()
    m.observe(0x516560, check_remove)

    def scratch():
        out = []
        for t in range(ntiles):
            v = m.r(w.tiles + 0x30 * t + 0x24)
            d = v - w.tiles
            out.append(['link', d // 0x30] if 0 <= d < 0x30 * ntiles and d % 0x30 == 0 else v)
        return out
    initial_scratch = scratch()
    initial_stamp = m.r(w.map + 0x48)
    prev = [w.tile_state(t) for t in range(ntiles)]
    ops = []
    for _ in range(rng.randint(5, 18)):
        zi = rng.randrange(len(zones))
        z = w.zones[zi]
        r = rng.random()
        radius = rng.choice([f32(rng.uniform(0, 5 * tile)), f32(rng.uniform(0, 2 * tile)), 0, f32(tile),
                             f32(3 * tile)])
        del w.events[:]
        op = {'zone': zi}
        try:
            if r < 0.4:
                pos = [f32(rng.uniform(0, W * tile)), f32(rng.uniform(0, H * tile))]
                op.update(op='move_to', pos=pos, radius=radius)
                m.call(0x515f40, ecx=z, args=(pos[0], pos[1], radius), pop=12)
            elif r < 0.7:
                cx, cy = m.r(z + 0xc), m.r(z + 0x10)
                if rng.random() < 0.75:
                    x = min(max(cx + rng.choice([-1, 0, 0, 1]), 0), W - 1)
                    y = min(max(cy + rng.choice([-1, 0, 1]), 0), H - 1)
                else:
                    x, y = rng.randrange(1, W - 1), rng.randrange(1, H - 1)
                op.update(op='move', x=x, y=y, radius=radius)
                m.call(0x515970, ecx=z, args=(x, y, radius), pop=12)
            elif r < 0.82:
                op.update(op='release')
                m.call(0x514ce0, ecx=z)
            elif r < 0.92:
                setm = rng.randrange(1 << nk)
                changed = rng.randrange(1, 1 << nk)
                op.update(op='mask', set=setm, mask_changed=changed)
                m.call(0x5406d0, ecx=fs, args=(z, setm, changed), pop=12)
            else:
                op.update(op='effects')
                m.call(0x5147c0, ecx=z)
            op['fault'] = None
        except NativeFault as e:
            op['fault'] = 'native UB: 0x516560 absent actor' if ub else str(e)
        op['events'] = list(w.events)
        op['zones'] = [w.zone_state(zz) for zz in w.zones]
        cur = [w.tile_state(t) for t in range(ntiles)]
        op['changed'] = [[t, s] for t, s in enumerate(cur) if s != prev[t]]
        prev = cur
        op['scratch'] = scratch()
        op['stamp'] = m.r(w.map + 0x48)
        op['busy'] = m.rb(BUSY)
        ops.append(op)
        if ub or op['fault']:
            break
    m.verify_v14()
    kout = [{'addr': p, 'index': m.r(p + 0x18c), 'nonmajor': m.r(p + 0x190), 'shared': m.r(p + 0x228),
             'supply': m.r16(p + 0x22c)} if p else None for p in w.kingdoms]
    return {'seed': seed, 'precision': prec,
            'map': {'width': W, 'height': H, 'shift': shift, 'tile': m.r(w.map + 8),
                    'extent': [m.r(w.map + 0xc), m.r(w.map + 0x10)]},
            'fog': m.r(m.r(m.r(0xB39AB0) + 0x108)), 'sai': m.rb(m.r(0xB36528) + 0x20), 'fast': fast,
            'solvers': [1, 1], 'tile_flags': flags, 'tile_blockers': blockers,
            'initial_scratch': initial_scratch, 'initial_stamp': initial_stamp,
            'kingdoms': kout, 'actors': initial_actors, 'occupants': occupants, 'zones': zones,
            'zone_keys': list(w.zones), 'zone_states': zstates, 'ops': ops}, m.visited


def main():
    cases, visited = [], {}
    for seed in range(120):
        for prec in PRECS:
            c, v = run_case(seed, prec)
            cases.append(c)
            visited.update(v)
    out = header(__file__, 'Unicorn 2.1.4, original AA.exe ZoneSolver/flood-solver zone paths, PC24/53/64')
    ranges = [(0x540860, 0x1c7), (0x5402e0, 0x2aa), (0x53f9f0, 0x33b), (0x53f970, 0x34), (0x53c5c0, 0x5f),
              (0x53c630, 0x45), (0x554ed0, 0x14c), (0x4f2000, 0x167), (0x4f2760, 0x47), (0x53d090, 0xb7),
              (0x53d150, 0x148), (0x4f4050, 0x2ff), (0x53f890, 0x38), (0x53f8d0, 0x9f), (0x53f5b0, 0x15),
              (0x53f880, 0x10), (0x53f610, 0x27), (0x53f600, 0x10), (0x539db0, 0x3d), (0x539d20, 0x3d),
              (0x53f5d0, 0x24), (0x539df0, 0x3a), (0x552440, 0x62), (0x45bc70, 0x26), (0x540590, 0xfa),
              (0x5406d0, 0x18f), (0x540690, 0x3f), (0x5147c0, 0x1b)]
    nops = sum(len(c['ops']) for c in cases)
    out.update({'cases': cases, 'case_count': len(cases), 'op_count': nops,
                'faults': sum(1 for c in cases for o in c['ops'] if o['fault']),
                'coverage': {hex(a): len(visited_in(visited, a, a + s)) for a, s in ranges},
                'instructions': instructions(visited)})
    (HERE / 'native-zone-flood.json').write_text(json.dumps(out, separators=(',', ':')))
    print('cases', len(cases), 'ops', nops, 'faults', out['faults'])
    print(out['coverage'])


if __name__ == '__main__':
    main()
