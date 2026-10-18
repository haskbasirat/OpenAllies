"""Oracle: the detection component (vtable 0xA3C568) executed by the ORIGINAL
code, with real zones (KKC_GActor::CreateZone 0x4D69A0 + 0x4849F0 vector
growth + 0x516640) moved through the fast ZoneSolver (proven separately).

Entry points: 0x56F140 (updatable tick, ECX = component+8), 0x56EC90
(component +0x14), 0x56EB40 (zone enable), 0x56EA30 / 0x56EC10 (ZOD / RZOD
enabled), 0x56EFC0 (reveal mask; 0x4CF110 native), 0x56EA10 (radius from
ActorData+0x1CC, context 0).
Seams: 0x56EDC0 (engagement sources, recorded and answered from the case),
0x50ACC0 (tick scheduling), actor virtuals +0x100/+0x158/+0x108, kingdom
virtual +0x140, plus the zone-world seams (SAI, 50C4C0, 404F60).
Recorded per op: result, events, component +0x10/+0x14, each actor's zone
vector (size and zone states) and the changed tiles. Writes
native-detection-component.json.
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
    W, H = 12, 9
    nk = rng.randint(1, 4)
    ks = [{'index': i, 'nonmajor': rng.choice([0, 0, 0, 1])} for i in range(nk)]
    w = ZoneWorld(m, width=W, height=H, shift=3, tile_size=8.0, kingdoms=ks, fog_enabled=rng.choice([0, 1, 1]),
                  sai=rng.random() < 0.3)
    ev = w.events
    real = [i for i, k in enumerate(w.kingdoms) if k]
    # kingdom vtables (+0x140 enemy) and +0x78 reveal byte holders (0x4CF110 native)
    kinfo = []
    for i, k in enumerate(w.kingdoms):
        enemy = sorted(j for j in real if rng.random() < 0.5)
        byte = rng.choice([None, 0, 0x80, 0x7f, 0xff])
        kinfo.append({'enemy': enemy, 'byte': byte})

        def kenemy(m, i=i):
            o = w.kingdom_index(m.arg(1))
            ev.append(['kingdom_enemy', i, o])
            m.ret(1 if o in kinfo[i]['enemy'] else 0, 4)
        m.w(k, m.vtable({0x140: kenemy}))
        if byte is None:
            m.w(k + 0x78, 0)
        else:
            holder = m.alloc(0x40)
            m.wb(holder + 0x1c, byte)
            m.w(k + 0x78, holder)
    na = rng.randint(1, 4)
    actors, info, comps = [], [], []
    for a in range(na):
        i = {'owner': rng.choice([None] + real * 3), 'flags': rng.choice([0, 0, 0, 1]),
             'reveal_mask': 0, 'pos': [f32(rng.uniform(0, W * 8)), f32(rng.uniform(0, H * 8))],
             'w100': rng.choice([0, 0, 0x200000]), 'w138': rng.choice([0, 0, 1]), 'w94': rng.choice([0, 0x99]),
             'w90': rng.choice([0, 0, 0x90]), 'v100': rng.choice([0, 0, 1]), 'v158': rng.choice([0, 1]),
             'c80': rng.choice([None, [0, 0], [1, 0], [1, 1]]), 'zod': f32(rng.choice([0.0, 12.0, 20.0, 31.5, -1.0])),
             'remote': f32(rng.choice([0.0, 10.0, 24.0])), 'd430': rng.choice([0, 0, 1]),
             'sources': rng.choice([0, 0, rng.randrange(1 << nk)]), 'members': None, 'slots': rng.choice([0, 9])}
        info.append(i)
        p = w.actor(owner=i['owner'], flags=i['flags'], reveal_mask=0, zone_slots=i['slots'])
        actors.append(p)
    for a, p in enumerate(actors):
        i = info[a]
        vt = m.r(p)
        m.w(vt + 0x100, m.stub(lambda m, a=a: (ev.append(['v100', a]), m.ret(info[a]['v100']))))
        m.w(vt + 0x158, m.stub(lambda m, a=a: (ev.append(['v158', a]), m.ret(info[a]['v158']))))
        m.w(p + 0x20, i['pos'][0])
        m.w(p + 0x24, i['pos'][1])
        m.w(p + 0x100, i['w100'])
        m.w(p + 0x138, i['w138'])
        m.w(p + 0x94, i['w94'])
        m.w(p + 0x90, i['w90'])
        if i['c80'] is None:
            m.w(p + 0x80, 0)
        else:
            c = m.alloc(0x20)
            m.w(c + 0x10, actors[(a + 1) % na] if i['c80'][0] else 0)
            m.wb(c + 0x1c, i['c80'][1])
            m.w(p + 0x80, c)
        data = m.alloc(0x500)
        m.w(data + 0x1cc, i['zod'])
        m.w(data + 0x1d0, i['remote'])
        m.w(data + 0x430, i['d430'])
        m.w(p + 4, data)
        if rng.random() < 0.3:
            mem = [rng.choice([None, *range(na)]) for _ in range(rng.randint(0, 3))]
            org = m.alloc(0x40)
            m.w(org + 0x28, m.array([0 if x is None else actors[x] for x in mem] or [0]))
            m.w(org + 0x2c, len(mem))
            m.w(p + 0x7c, org)
            i['members'] = mem
        comp = m.alloc(0x18)
        m.w(comp, 0xA3C568)
        m.w(comp + 4, p)
        m.w(comp + 8, 0xA3C540)
        m.w(comp + 0x10, 0)
        m.w(comp + 0x14, i['zod'])
        m.w(p + 0xb8, comp)
        comps.append(comp)
    for a, p in enumerate(actors):
        if info[a]['c80'] is not None:
            info[a]['c80_target'] = (a + 1) % na if info[a]['c80'][0] else None

    def sources(m):
        a = w.actor_idx(m.arg(2))
        ev.append(['sources', a])
        m.w(m.arg(1), info[a]['sources'])
        m.ret(m.arg(1), 8)
    m.seam(0x56edc0, sources)
    for addr, name in ((0x4ec270, 'tile_changed'), (0x4d4130, '4d4130'), (0x579ba0, '579ba0'), (0x4cfea0, '4cfea0')):
        pops = 8 if addr == 0x4ec270 else 0
        m.seam(addr, lambda m, name=name, pops=pops: (ev.append([name, w.actor_idx(m.ecx()) if name != '579ba0' else None]),
                                                     m.ret(0, pops)))
    m.seam(0x50acc0, lambda m: (ev.append(['schedule', w.actor_idx(m.r(m.ecx() - 8 + 4)), m.arg(1)]), m.ret(0, 8)))
    initial = [dict(i, addr=actors[a]) for a, i in enumerate(info)]
    zone_addrs = {}

    def zone_vec(a):
        p = actors[a]
        n = m.r(p + 0x14c)
        data = m.r(p + 0x148)
        out = []
        for j in range(n):
            z = m.r(data + 4 * j)
            out.append(None if z == 0 else w.zone_state(z))
        return out
    prev = [w.tile_state(t) for t in range(W * H)]
    ops = []
    for _ in range(rng.randint(8, 30)):
        a = rng.randrange(na)
        comp = comps[a]
        kind = rng.choice(['tick', 'tick', 'tick', 'activate', 'enable', 'update_mask', 'zod_enabled', 'rzod_enabled',
                           'set_radius', 'mutate', 'actor_moved'])
        op = {'op': kind, 'actor': a}
        del ev[:]
        try:
            if kind == 'tick':
                m.call(0x56f140, ecx=comp + 8, args=(0, 0), pop=8)
            elif kind == 'activate':
                m.call(0x56ec90, ecx=comp)
            elif kind == 'enable':
                idx = rng.choice([1, 6, 6, 2])
                on = rng.choice([0, 1, 1])
                r = rng.choice([info[a]['zod'], info[a]['remote'], f32(16.0)])
                op.update(index=idx, enable=on, radius=r)
                m.call(0x56eb40, ecx=comp, args=(idx, on, r), pop=12)
            elif kind == 'update_mask':
                m.call(0x56efc0, ecx=comp)
            elif kind in ('zod_enabled', 'rzod_enabled'):
                op['result'] = m.call(0x56ea30 if kind == 'zod_enabled' else 0x56ec10, ecx=comp) & 0xff
            elif kind == 'actor_moved':
                m.call(0x4d6350, ecx=actors[a], args=(0, 0), pop=8)
            elif kind == 'set_radius':
                m.call(0x56ea10, ecx=comp, args=(0,), pop=4)
            else:
                what = rng.choice(['pos', 'sources', 'flags', 'zod'])
                op['what'] = what
                if what == 'pos':
                    pos = [f32(rng.uniform(0, W * 8)), f32(rng.uniform(0, H * 8))]
                    info[a]['pos'] = pos
                    m.w(actors[a] + 0x20, pos[0])
                    m.w(actors[a] + 0x24, pos[1])
                    op['value'] = pos
                elif what == 'sources':
                    info[a]['sources'] = rng.randrange(1 << nk)
                    op['value'] = info[a]['sources']
                elif what == 'flags':
                    info[a]['flags'] ^= 1
                    m.w(actors[a] + 8, info[a]['flags'])
                    op['value'] = info[a]['flags']
                else:
                    v = f32(rng.choice([0.0, 12.0, 20.0, 31.5]))
                    m.w(m.r(actors[a] + 4) + 0x1cc, v)
                    info[a]['zod'] = v
                    op['value'] = v
            op['fault'] = None
        except NativeFault as e:
            op['fault'] = str(e)
        op['events'] = list(ev)
        op['components'] = [[m.r(c + 0x10), m.r(c + 0x14)] for c in comps]
        op['zones'] = [zone_vec(b) for b in range(na)]
        cur = [w.tile_state(t) for t in range(W * H)]
        op['changed'] = [[t, s] for t, s in enumerate(cur) if s != prev[t]]
        prev = cur
        ops.append(op)
        if op['fault']:
            break
    m.verify_v14()
    kout = [{'addr': p, 'index': m.r(p + 0x18c), 'nonmajor': m.r(p + 0x190), 'shared': m.r(p + 0x228),
             'supply': m.r16(p + 0x22c)} if p else None for p in w.kingdoms]
    return {'seed': seed, 'precision': prec,
            'map': {'width': W, 'height': H, 'shift': 3, 'tile': m.r(w.map + 8),
                    'extent': [m.r(w.map + 0xc), m.r(w.map + 0x10)]},
            'fog': m.r(m.r(m.r(0xB39AB0) + 0x108)), 'sai': m.rb(m.r(0xB36528) + 0x20), 'fast': 1,
            'solvers': [1, 1], 'kingdoms': kout, 'kingdom_info': kinfo, 'actors': initial,
            'occupants': [[] for _ in range(W * H)], 'ops': ops}, m.visited


def main():
    cases, visited = [], {}
    for seed in range(200):
        for prec in PRECS:
            c, v = run_case(seed, prec)
            cases.append(c)
            visited.update(v)
    out = header(__file__, 'Unicorn 2.1.4, original AA.exe detection component, PC24/53/64')
    ranges = [(0x56f140, 0xbb), (0x56ec90, 0x125), (0x56eb40, 0xcd), (0x56ea30, 0x6b), (0x56ec10, 0x7a),
              (0x56efc0, 0x17a), (0x56ea10, 0x19), (0x4d69a0, 0x132), (0x4849f0, 0x66), (0x4cf110, 0xe)]
    nops = sum(len(c['ops']) for c in cases)
    out.update({'cases': cases, 'case_count': len(cases), 'op_count': nops,
                'faults': sum(1 for c in cases for o in c['ops'] if o['fault']),
                'coverage': {hex(a): len(visited_in(visited, a, a + s)) for a, s in ranges},
                'instructions': instructions(visited)})
    (HERE / 'native-detection-component.json').write_text(json.dumps(out, separators=(',', ':')))
    print('cases', len(cases), 'ops', nops, 'faults', out['faults'])
    print(out['coverage'])


if __name__ == '__main__':
    main()
