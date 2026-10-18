"""Oracle: zone-set queries and the GMap tile wrappers (the AI's ZOC/ZOG/ZOS
queries) executed by the ORIGINAL code.

Tile wrappers: 54EF40, 54EF80, 54EFC0, 54EFF0, 54F020, 54F870 (+559530),
54F8B0 (+5595D0), 54F8F0. Direct set slots (leaf/union): +0x2C has-actor,
+0x30 leaf identity, +0x44 part identity, +0x40 detects(kingdom), +0x58 visit
(visitor +0x64), +0x5C notify(type) (tail call 4F3D90 seamed). The other slots
(+0x14, +0x1C, +0x20, +0x24, +0x28, +0x38, +0x3C) are reached through the
wrappers. Sets are built with 54FA10 (proven in native-zone-sets.json).

Recorded per query: arguments (actors/kingdoms as indices, set parts as
(tile, suffix depth) or leaf values), EAX, and the ordered actor virtual calls
(+0x108 owner, +0x138 allied, +0x140 enemy) and visitor/notify calls. Memory
reads (actor+4, +0xB0, +0xB8, +0x148) are inputs, not events.
Writes native-zone-queries.json.
"""
import json, random
from pathlib import Path
from zones_machine import Machine, NativeFault, header, instructions, visited_in
from zones_world import World, LEAF_VTABLE

HERE = Path(__file__).resolve().parent
ZONE_PTR_BASE = 0x30000000


def zone_value(a, i):
    return ZONE_PTR_BASE + a * 0x100 + i * 4


def run_case(seed):
    rng = random.Random(seed)
    m = Machine('Double53')
    nk = rng.randint(1, 5)
    kingdoms = []
    for i in range(nk):
        kingdoms.append({'index': rng.choice([i, i, i, rng.randrange(40)]),
                         'nonmajor': rng.choice([0, 0, 0, 1]),
                         'shared': rng.randrange(1 << 32) if rng.random() < 0.3 else None,
                         'supply': rng.randrange(1 << 16) if rng.random() < 0.4 else None})
    holes = rng.random() < 0.2
    kspec = list(kingdoms) + ([None] if holes else [])
    w = World(m, width=4, height=3, kingdoms=kspec, table_size=3)
    na = rng.randint(2, 6)
    actors, ainfo = [], []
    for a in range(na):
        owner = rng.choice([None] + list(range(nk)) * 2)
        allied = {k for k in range(nk) if rng.random() < 0.4}
        enemy = {k for k in range(nk) if rng.random() < 0.4}
        reveal = rng.choice([None, 0, rng.randrange(1 << 20), (1 << nk) - 1])
        b0 = rng.choice([0, 0x1234])
        slots = rng.randint(0, 9)
        p = w.actor(owner=owner, allied=allied, enemy=enemy, reveal_mask=reveal, b0=b0, zone_slots=slots)
        word4 = rng.choice([0x40000000, 0x40000010, 0x40000020])
        m.w(p + 4, word4)
        zone_vals = []
        data = m.r(p + 0x148)
        for i in range(slots):
            v = zone_value(a, i) if rng.random() < 0.8 else 0
            m.w(data + 4 * i, v)
            zone_vals.append(v)
        actors.append(p)
        ainfo.append({'addr': p, 'owner': owner, 'allied': sorted(allied), 'enemy': sorted(enemy),
                      'reveal_mask': reveal, 'b0': b0, 'word4': word4, 'zones': zone_vals})
    # tiles: random zone sets via 54FA10 with stub zones (no callbacks recorded)
    stubvt = m.vtable({0x14: lambda m: m.ret(0, 8), 0x10: lambda m: m.ret(0, 4)})
    ntiles = w.width * w.height
    for t in range(ntiles):
        for _ in range(rng.choice([0, 1, 1, 2, 3, 4, 6])):
            a = rng.randrange(na)
            flags = rng.choice([1 << rng.randrange(9)] * 4 + [rng.randrange(1, 0x10000)])
            z = m.alloc(0x18)
            m.w(z, stubvt)
            m.w(z + 4, actors[a])
            m.w16(z + 0x14, flags)
            m.call(0x54fa10, ecx=w.tile(t % w.width, t // w.width), args=(z, flags), pop=8)
        m.w16(w.tiles + 0x30 * t + 0x18, rng.randrange(1 << 16) if rng.random() < 0.5 else 0)
    tiles = [w.set_list(m.r(w.tiles + 0x30 * t + 4)) for t in range(ntiles)]
    supply = [m.r16(w.tiles + 0x30 * t + 0x18) for t in range(ntiles)]

    def kpick():
        r = rng.random()
        if r < 0.1:
            return None, 0
        i = rng.randrange(len(w.kingdoms))
        return i, w.kingdoms[i]

    def apick():
        if rng.random() < 0.3:
            return None, 0
        i = rng.randrange(na)
        return i, actors[i]

    # visitor (+0x64 recorded) for +0x58
    def visit(m):
        w.events.append(['visit', w.kingdom_index(m.arg(1)), m.arg(2) & 0xff, m.arg(3) & 0xff])
        m.ret(0, 12)
    visitor = m.alloc(0x10)
    m.w(visitor, m.vtable({0x64: visit}))
    m.seam(0x4f3d90, lambda m: (w.events.append(['notify', m.arg(1)]), m.ret(0, 4)))

    queries = []
    for _ in range(rng.randint(20, 60)):
        t = rng.randrange(ntiles)
        tile = w.tiles + 0x30 * t
        s = m.r(tile + 4)
        kind = rng.choice(['54ef40', '54ef80', '54efc0', '54efc0', '54eff0', '54f020', '54f870', '54f8b0',
                           '54f8b0', '54f8f0', 'has_actor', 'leaf_id', 'part_id', 'detects', 'visit', 'notify'])
        if kind in ('has_actor', 'leaf_id', 'part_id', 'detects', 'visit', 'notify') and not s:
            kind = '54efc0'
        q = {'q': kind, 'tile': t}
        del w.events[:]
        try:
            if kind == '54ef40':
                if rng.random() < 0.3:
                    q['zone'] = None
                    r = m.call(0x54ef40, ecx=tile, args=(0,), pop=4)
                else:
                    ai, ap = rng.randrange(na), None
                    flags = rng.choice([1 << rng.randrange(9), rng.randrange(1 << 16)])
                    z = m.alloc(0x18)
                    m.w(z + 4, actors[ai])
                    m.w16(z + 0x14, flags)
                    q['zone'] = [ai, flags]
                    r = m.call(0x54ef40, ecx=tile, args=(z,), pop=4)
            elif kind == '54ef80':
                ty = rng.randrange(12)
                ai, ap = apick()
                x = rng.choice([0, 0x40000000, 0x40000010, 0x40000020])
                q.update(type=ty, exclude=ai, word4=x)
                r = m.call(0x54ef80, ecx=tile, args=(ty, ap, x), pop=12)
            elif kind in ('54efc0', '54eff0'):
                ki, kp = kpick()
                ty = rng.randrange(12)
                q.update(kingdom=ki, type=ty)
                r = m.call(int(kind, 16), ecx=tile, args=(kp, ty), pop=8)
            elif kind == '54f020':
                ki, kp = kpick()
                q.update(kingdom=ki)
                r = m.call(0x54f020, ecx=tile, args=(kp,), pop=4)
            elif kind in ('54f870', '54f8b0'):
                ki, kp = kpick()
                ai, ap = apick()
                q.update(kingdom=ki, exclude=ai)
                r = m.call(int(kind, 16), ecx=tile, args=(kp, ap), pop=8)
            elif kind == '54f8f0':
                ki, kp = kpick()
                ai, ap = apick()
                ty = rng.randrange(10)
                pre = [rng.randrange(na) for _ in range(rng.choice([0, 0, 1, 2]))]
                lst = m.alloc(8)
                head, tail = 0, 0
                for a in reversed(pre):
                    node = m.alloc(8)
                    m.w(node, actors[a])
                    m.w(node + 4, head)
                    head = node
                    if tail == 0:
                        tail = node
                m.w(lst, head)
                m.w(lst + 4, tail)
                q.update(kingdom=ki, exclude=ai, type=ty, list_before=pre)
                r = m.call(0x54f8f0, ecx=tile, args=(lst, kp, ty, ap), pop=16)
                items, node = [], m.r(lst)
                while node:
                    items.append(w.actor_idx(m.r(node)))
                    node = m.r(node + 4)
                q['list_after'] = items
                q['tail_set'] = m.r(lst + 4) != 0
            elif kind == 'has_actor':
                ai = rng.randrange(na)
                q['actor'] = ai
                r = m.call(m.r(m.r(s) + 0x2c), ecx=s, args=(actors[ai],), pop=4)
            elif kind in ('leaf_id', 'part_id'):
                # pick an existing object: a leaf of some tile or a suffix of some tile's set
                t2 = rng.choice([i for i in range(ntiles) if m.r(w.tiles + 0x30 * i + 4)])
                obj = m.r(w.tiles + 0x30 * t2 + 4)
                depth = 0
                while m.r(obj) != LEAF_VTABLE and rng.random() < 0.6:
                    obj = m.r(obj + 8)
                    depth += 1
                take_head = m.r(obj) != LEAF_VTABLE and rng.random() < 0.5
                if take_head:
                    obj = m.r(obj + 0xc)
                q['part'] = [t2, depth, take_head]
                slot = 0x30 if kind == 'leaf_id' else 0x44
                r = m.call(m.r(m.r(s) + slot), ecx=s, args=(obj,), pop=4)
            elif kind == 'detects':
                ki = rng.randrange(len(w.kingdoms))
                kp = w.kingdoms[ki]
                if kp == 0:
                    ki = 0
                    kp = w.kingdoms[0]
                q['kingdom'] = ki
                r = m.call(m.r(m.r(s) + 0x40), ecx=s, args=(kp,), pop=4)
            elif kind == 'visit':
                r = m.call(m.r(m.r(s) + 0x58), ecx=s, args=(visitor,), pop=4)
            elif kind == 'notify':
                ty = rng.randrange(-2, 18)
                q['type'] = ty
                r = m.call(m.r(m.r(s) + 0x5c), ecx=s, args=(ty & 0xffffffff,), pop=4)
            q['eax'] = r
            q['fault'] = None
        except NativeFault as e:
            q['eax'] = None
            q['fault'] = str(e)
        q['events'] = list(w.events)
        queries.append(q)
    m.verify_v14()
    kout = [{'addr': p, 'index': m.r(p + 0x18c), 'nonmajor': m.r(p + 0x190), 'shared': m.r(p + 0x228),
             'supply': m.r16(p + 0x22c)} if p else None for p in w.kingdoms]
    return {'seed': seed, 'kingdoms': kout, 'actors': ainfo, 'tiles': tiles, 'supply': supply,
            'queries': queries}, m.visited


def main():
    cases, visited = [], {}
    for seed in range(300):
        c, v = run_case(seed)
        cases.append(c)
        visited.update(v)
    out = header(__file__, 'Unicorn 2.1.4, original AA.exe zone-set queries and tile wrappers')
    nq = sum(len(c['queries']) for c in cases)
    faults = sum(1 for c in cases for q in c['queries'] if q['fault'])
    ranges = [(0x514bf0, 0x35), (0x514a70, 0x23), (0x5149f0, 0x70), (0x514aa0, 0x68), (0x514b10, 0x68),
              (0x514b80, 0x2f), (0x514bb0, 0x3a), (0x514c30, 0x4e), (0x515ec0, 0x71), (0x5155c0, 0x56),
              (0x5158a0, 0xa0), (0x515040, 0x56), (0x5150a0, 0x4b), (0x5150f0, 0x5b), (0x515150, 0x4e),
              (0x5151a0, 0x4e), (0x5151f0, 0x49), (0x515240, 0x49), (0x5152f0, 0x49), (0x515340, 0x64),
              (0x5153b0, 0x63), (0x515420, 0x64), (0x515490, 0x49), (0x515000, 0x3c), (0x515580, 0x3c),
              (0x54ef40, 0x31), (0x54ef80, 0x32), (0x54efc0, 0x2d), (0x54eff0, 0x2d), (0x54f020, 0x11),
              (0x54f870, 0x3a), (0x54f8b0, 0x3a), (0x54f8f0, 0x36), (0x559530, 0x37), (0x5595d0, 0x2b)]
    out.update({'cases': cases, 'case_count': len(cases), 'query_count': nq, 'faults': faults,
                'coverage': {hex(a): len(visited_in(visited, a, a + s)) for a, s in ranges},
                'instructions': instructions(visited)})
    (HERE / 'native-zone-queries.json').write_text(json.dumps(out, separators=(',', ':')))
    print('cases', len(cases), 'queries', nq, 'faults', faults)
    print(out['coverage'])


if __name__ == '__main__':
    main()
