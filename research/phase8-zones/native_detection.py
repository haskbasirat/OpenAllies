"""Oracle: actor detection state (GObject.cpp) executed by the ORIGINAL code.

Actors carry real code in their vtables: +0x64 0x4EC010 (reveal), +0x68
0x4EC1B0 (hide), +0x6C 0x4EA0C0 (unexplore), +0x70 0x4D3B80 (visible to),
+0x74 0x4D3C50 (explored by); also called directly: 0x4EA0D0, 0x4EA1D0,
0x4EBC70 (memory refresh), 0x4EBFB0, 0x4EBFD0. Container forwarding and
organization recursion therefore run natively too.
Seams: actor virtuals +0 (size), +0x8C, +0xB4, +0x100, +0x148/+0x14C
(create/delete a memory record: the slot is set/cleared), 0x4EB490
(eligibility), 0x4EBB80 (clear memory), 0x5733A0 (carrier), 0x432E40 (first
sighting tail: returns a record whose data +0x430 is set, so the message
and SAI branches are skipped and only the call is observable), 0x4EA0F0 and
the shape iterator 0x554DD0/0x554330 (a given tile list). Recorded per op:
result, ordered events, every actor's +0x54 word and memory slots.
Writes native-detection.json.
"""
import json, random
from pathlib import Path
from zones_machine import Machine, NativeFault, header, instructions, visited_in, f32
from zones_world import World

HERE = Path(__file__).resolve().parent
PRECS = ['Single24', 'Double53', 'Extended64']


def run_case(seed, prec):
    rng = random.Random(seed)
    m = Machine(prec)
    nk = rng.randint(1, 4)
    ks = [{'index': rng.choice([i, i, i, rng.randrange(20)]), 'nonmajor': rng.choice([0, 0, 0, 1]),
           'shared': rng.choice([None, rng.randrange(1 << 32), 0xffffffff])} for i in range(nk)]
    if rng.random() < 0.2:
        ks.insert(rng.randrange(len(ks) + 1), None)
    shift = 3
    tile = rng.choice([8.0, 8.0, 6.0])
    w = World(m, width=6, height=4, shift=shift, tile_size=tile, kingdoms=ks, fog_enabled=rng.choice([0, 1, 1, 1]))
    ev = w.events
    for t in range(24):
        m.w(w.tiles + 0x30 * t + 0x14, rng.choice([0, rng.randrange(1 << 32), 0xffff0000]))
    na = rng.randint(2, 6)
    actors, info = [], []
    for a in range(na):
        p = m.alloc(0x200)
        actors.append(p)
        w.actor_index[p] = a
    w.actors = actors
    for a, p in enumerate(actors):
        i = {'size': f32(rng.choice([1.0, 2.0, 3.5, 4.0, 5.0, 0.5])), 'keeps': rng.choice([0, 1, 1]),
             'eligible': rng.choice([0, 1]), 'b4': rng.choice([0, 1]),
             'container': rng.choice([None] * 3 + list(range(na))), 'flags': rng.choice([0, 0, 1, 0x80]),
             'announce': rng.choice([0, 1]), 'carrier': rng.choice([0] * 4 + [0x70000000 + a]),
             'members': rng.choice([None, None, [rng.randrange(na) for _ in range(rng.randint(0, 3))]]),
             'target': rng.choice([None, None] + list(range(a + 1, na))), 'target_byte': rng.choice([0, 1]),
             'word': rng.choice([0, rng.randrange(1 << 32)]), 'slots': [rng.choice([0, 0, 0x7700]) for _ in range(rng.randint(0, 20))],  # +0x58 count
             'shape': [rng.randrange(24) for _ in range(rng.randint(0, 4))], 'carrier_visible': rng.choice([0, 1]),
             'owner': rng.choice([None] + [j for j, k in enumerate(w.kingdoms) if k] * 2),
             'reveal_mask': rng.choice([None, 0, rng.randrange(1 << 6)])}
        if i['container'] == a:
            i['container'] = None
        info.append(i)

    def slot_stub(name, a, f):
        return f

    for a, p in enumerate(actors):
        i = info[a]
        vt = m.vtable({
            0x0: lambda m, a=a: (ev.append(['size', a]), m.ret_st0_f32(info[a]['size'])),
            0x8c: lambda m, a=a: (ev.append(['keeps', a]), m.ret(info[a]['keeps'])),
            0xb4: lambda m, a=a: (ev.append(['b4', a]), m.ret(info[a]['b4'])),
            0x100: lambda m, a=a: (ev.append(['container', a]),
                                   m.ret(0 if info[a]['container'] is None else actors[info[a]['container']])),
            0x148: lambda m, a=a: create(m, a),
            0x14c: lambda m, a=a: delete(m, a),
            0x108: lambda m, a=a: (ev.append(['owner', a]),
                                   m.ret(0 if info[a]['owner'] is None else w.kingdoms[info[a]['owner']])),
        })
        for off, fn in ((0x64, 0x4ec010), (0x68, 0x4ec1b0), (0x6c, 0x4ea0c0), (0x70, 0x4d3b80), (0x74, 0x4d3c50)):
            m.w(vt + off, fn)
        m.w(p, vt)
        data = m.alloc(0x500)
        m.wb(data + 0x16d, i['announce'])
        m.w(p + 4, data)
        m.w(p + 8, i['flags'])
        m.w(p + 0x54, i['word'])
        slots = m.array(i['slots'] + [0] * (32 - len(i['slots'])))   # backing store covers index 0..31
        m.w(p + 0x58, len(i['slots']))
        m.w(p + 0x5c, slots)
        m.w(p + 0xc8, i['carrier'])
        if i['reveal_mask'] is None:
            m.w(p + 0xb8, 0)
        else:
            rc = m.alloc(0x20)
            m.w(rc + 0x10, i['reveal_mask'])
            m.w(p + 0xb8, rc)
        if i['members'] is not None:
            org = m.alloc(0x40)
            m.w(org + 0x28, m.array([actors[x] for x in i['members']] + [0] * rng.randint(0, 2)))
            n = len(i['members'])
            m.w(org + 0x2c, n)
            m.w(p + 0x7c, org)
        if i['target'] is not None:
            c = m.alloc(0x20)
            m.w(c + 0x10, actors[i['target']])
            m.wb(c + 0x1c, i['target_byte'])
            m.w(p + 0x80, c)

    def kidx(k):
        return w.kingdom_index(k)

    def create(m, a):
        k = m.arg(1)
        ev.append(['create', a, kidx(k)])
        m.w(m.r(actors[a] + 0x5c) + 4 * m.r(k + 0x18c), 0x7700)
        m.ret(0, 4)

    def delete(m, a):
        k = m.arg(1)
        ev.append(['delete', a, kidx(k)])
        m.w(m.r(actors[a] + 0x5c) + 4 * m.r(k + 0x18c), 0)
        m.ret(0, 4)
    m.seam(0x4eb490, lambda m: (ev.append(['eligible', w.actor_idx(m.ecx())]), m.ret(info[w.actor_idx(m.ecx())]['eligible'])))
    m.seam(0x4ebb80, lambda m: (ev.append(['clear_memory', w.actor_idx(m.ecx())]), m.ret(0)))
    m.seam(0x5733a0, lambda m: (ev.append(['carrier_visible', m.ecx(), kidx(m.arg(1))]),
                                m.ret(info[m.ecx() - 0x70000000]['carrier_visible'], 4)))
    record = m.alloc(0x10)
    rdata = m.alloc(0x500)
    m.w(record + 4, rdata)
    m.w(rdata + 0x430, 1)

    def gtid(m):
        # ECX = actor+0x50; the reveal's kingdom is the first argument of 4EC010
        a = w.actor_idx(m.ecx() - 0x50)
        ev.append(['first_sighting', a])
        m.ret(record)
    m.seam(0x432e40, gtid)
    m.seam(0x4ea0f0, lambda m: m.ret(m.arg(1), 4))
    m.seam(0x4cf210, lambda m: m.ret(m.arg(1), 4))
    stubvt = m.vtable({0x14: lambda m: m.ret(0, 8), 0x10: lambda m: m.ret(0, 4)})
    for t in range(24):
        for _ in range(rng.choice([0, 0, 1, 2, 3])):
            src = rng.randrange(na)
            fl = rng.choice([0x40, 0x40, 0x2, 0x42, 0x10, 0x1])
            z = m.alloc(0x18)
            m.w(z, stubvt)
            m.w(z + 4, actors[src])
            m.w16(z + 0x14, fl)
            m.call(0x54fa10, ecx=w.tiles + 0x30 * t, args=(z, fl), pop=8)
    tile_sets = [w.set_list(m.r(w.tiles + 0x30 * t + 4)) for t in range(24)]
    iters = {}

    def it_setup(m):
        it = m.ecx()
        a = cur_actor[0]
        tiles = list(info[a]['shape'])
        iters[it] = tiles
        m.w(it, w.map)
        m.w(it + 0x10, 100)
        nxt(m, it)
        m.ret(0, 0x10)

    def nxt(m, it):
        tiles = iters[it]
        if tiles:
            t = tiles.pop(0)
            m.w(it + 0x24, t % w.width)
            m.w(it + 0x28, t // w.width)
        else:
            m.w(it + 0x28, 101)

    m.seam(0x554dd0, it_setup)
    m.seam(0x554330, lambda m: (nxt(m, m.ecx()), m.ret(0, 4)))
    cur_actor = [0]
    initial = [{'word': m.r(p + 0x54), 'slots': [m.r(m.r(p + 0x5c) + 4 * j) for j in range(32)]}
               for a, p in enumerate(actors)]
    ops = []
    for _ in range(rng.randint(10, 40)):
        a = rng.randrange(na)
        cur_actor[0] = a
        p = actors[a]
        ki = rng.randrange(len(w.kingdoms))
        k = w.kingdoms[ki]
        kind = rng.choice(['reveal', 'reveal', 'reveal', 'hide', 'hide', 'unexplore', 'visible_to', 'explored_by',
                           'shared_explored', 'shared_visible', 'refresh', 'all_explored', 'all_visible',
                           'tile_changed', 'tile_changed', 'refresh_tiles'])
        op = {'op': kind, 'actor': a, 'kingdom': ki}
        del ev[:]
        try:
            if kind == 'reveal':
                ex, vis = rng.choice([0, 1]), rng.choice([0, 1])
                op.update(explored=ex, visible=vis)
                r = m.call(0x4ec010, ecx=p, args=(k, ex, vis), pop=12)
            elif kind == 'hide':
                r = m.call(0x4ec1b0, ecx=p, args=(k,), pop=4)
            elif kind == 'unexplore':
                r = m.call(0x4ea0c0, ecx=p, args=(k,), pop=4)
            elif kind in ('visible_to', 'explored_by'):
                deep = rng.choice([0, 1])
                op['deep'] = deep
                r = m.call(0x4d3b80 if kind == 'visible_to' else 0x4d3c50, ecx=p, args=(k, deep), pop=8)
            elif kind in ('shared_explored', 'shared_visible'):
                r = m.call(0x4ea0d0 if kind == 'shared_explored' else 0x4ea1d0, ecx=p, args=(k, 0), pop=8)
            elif kind == 'refresh':
                r = m.call(0x4ebc70, ecx=p)
            elif kind == 'tile_changed':
                r = m.call(0x4ec270, ecx=p, args=(0, 0), pop=8)
            elif kind == 'refresh_tiles':
                r = m.call(0x4ea150, ecx=p)
            elif kind == 'all_explored':
                r = m.call(0x4ebfb0, ecx=p)
            else:
                r = m.call(0x4ebfd0, ecx=p)
            op['result'] = r & 0xff
            op['fault'] = None
        except NativeFault as e:
            op['fault'] = str(e)
        op['events'] = list(ev)
        op['state'] = [{'word': m.r(q + 0x54), 'slots': [m.r(m.r(q + 0x5c) + 4 * j) for j in range(32)]}
                       for b, q in enumerate(actors)]
        ops.append(op)
    m.verify_v14()
    kout = [{'addr': p, 'index': m.r(p + 0x18c), 'nonmajor': m.r(p + 0x190), 'shared': m.r(p + 0x228),
             'supply': m.r16(p + 0x22c)} if p else None for p in w.kingdoms]
    fogs = [m.r(w.tiles + 0x30 * t + 0x14) for t in range(24)]
    return {'seed': seed, 'precision': prec, 'tile': f32(tile), 'fog': m.r(m.r(m.r(0xB39AB0) + 0x108)),
            'kingdoms': kout, 'actors': [dict(i, addr=actors[a]) for a, i in enumerate(info)],
            'initial': initial, 'tile_fog': fogs, 'tile_sets': tile_sets, 'ops': ops}, m.visited


def main():
    cases, visited = [], {}
    for seed in range(250):
        for prec in PRECS:
            c, v = run_case(seed, prec)
            cases.append(c)
            visited.update(v)
    out = header(__file__, 'Unicorn 2.1.4, original AA.exe actor detection (GObject) routines, PC24/53/64')
    ranges = [(0x4ec010, 0x191), (0x4ec1b0, 0xb3), (0x4ea0c0, 8), (0x4ea0d0, 0x10), (0x4ea1d0, 0x25),
              (0x4d3b80, 0xc7), (0x4d3c50, 0xc4), (0x4ebc70, 0x15c), (0x4ebfb0, 0x18), (0x4ebfd0, 0x18),
              (0x54eb40, 0x2d), (0x54eb70, 0x2d), (0x4364c0, 0x48), (0x4ec270, 0x1f), (0x4ea150, 0x77),
              (0x5158a0, 0xa0), (0x515000, 0x3c)]
    nops = sum(len(c['ops']) for c in cases)
    out.update({'cases': cases, 'case_count': len(cases), 'op_count': nops,
                'faults': sum(1 for c in cases for o in c['ops'] if o['fault']),
                'coverage': {hex(a): len(visited_in(visited, a, a + s)) for a, s in ranges},
                'instructions': instructions(visited)})
    (HERE / 'native-detection.json').write_text(json.dumps(out, separators=(',', ':')))
    print('cases', len(cases), 'ops', nops, 'faults', out['faults'])
    print(out['coverage'])


if __name__ == '__main__':
    main()
