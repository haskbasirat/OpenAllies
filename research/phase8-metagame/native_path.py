"""Original metagame territory pathfinder on the InitCreate worlds.

For every shipped campaign (state from 939B90 via mg_campaign) a seeded
sequence of phases is executed. Each phase sets a kingdom relation matrix
(team based in phase 0, random enemy matrices with false diagonal after),
changes owners with the original 955620, walks armies along map links and
teleports them with the original 940100 (path append / truncate), sets the
battles-fought counter (component +0x10) and then queries:

  search  the generic search exactly as 93FCC0/93FAD0 drive it: a 0x50-byte
          path object constructed by 961550(start, goal, kingdom actor,
          budget) (vtable AC7E94, node store 961230 / heap 53E7B0 created by
          961430), run by 961350, the node store path (+0x4C -> +0x1C) read,
          destroyed by 93E930. Kingdom actor 0 is included, budgets include
          -1 (unlimited), 0, negative and large values.
  army    93EAE0 (moves used along the component path +0x24) and the army
          pathfinder 93FDC0(target, out vector) (alias target -> partner via
          939B60, 93FCC0 budget rules, fallback 93FAD0), with the output
          vector {ptr, count, capacity} allocated empty.
  heuristic  960E70 on a node store (vtable ACDD68, +0x0C goal) with ST0
          captured as 80 bits.

After eight phases on the shipped locations, the definition locations
(+0x14/+0x18) are replaced twice for coverage (fixture op 'locations'; the
Rust replay patches MetaGameData the same way): 'grid' (multiples of 16,
many exact heuristic ties) and 'random' (random sign, magnitude and
significand), three phases each.

Every query runs under x87 PC53 (the metagame thread, lead policy), PC24
and PC64 (control words 0x027F, 0x007F, 0x037F) on the same state; state
changes run at PC53. Seams: those of mg_campaign (CRT heap, CRT _wcsicmp
96CAD6 with C-locale folding, engine lookups, army actor creation) plus the
kingdom actor virtual +0x140 (relation matrix) and army actor +0x108
(owner) of mg_world. Nothing else is seamed: the search, heap, node store
and the territory hash lookup 939B60 (4154D0, 5043F0) run natively.

Harness speed-up (no semantic change): the per-instruction trace hook of
mg_machine.Machine is replaced, before any Machine is built, by one that
byte-checks every distinct (address, size) once against the v14 image and
records it in VISITED, instead of re-checking it on every execution.
Allocations made by a query (path object, node store, heap, output vector)
are released by resetting the bump allocator after the query: nothing the
query allocates is referenced afterwards.
"""
import json, hashlib, random, struct
from pathlib import Path
import mg_machine
from mg_machine import P, V, VISITED, HERE, ext_to_json

CW = {'pc53': 0x027f, 'pc24': 0x007f, 'pc64': 0x037f}   # PC53 = GAME_PRECISION
_CHECKED = set()


def _hook(self, u, pc, size, _):
    seam = self.seams.get(pc)
    if seam is not None:
        self.seam_log.append(pc)
        seam(self)
        return
    if (pc, size) in _CHECKED:
        return
    if P.base <= pc < P.base + P.image_size:
        raw = P.range(pc, size)
        assert raw == V.range(pc, size), f'original/v14 differ at {pc:#x}'
        VISITED[pc] = raw.hex()
    _CHECKED.add((pc, size))


mg_machine.Machine._hook = _hook
from mg_campaign import DATA, run, snapshot  # noqa: E402  (after the hook patch)


class World:
    def __init__(self, ci):
        self.snap, self.ctx = run(ci, keep=True)
        c = self.ctx
        self.m, self.w, self.st = c['m'], c['w'], c['st']
        self.T = self.w.read_vector(self.st + 0x28)
        self.K = self.w.read_vector(self.st + 0x1c)
        self.actors = c['actors']
        self.created = c['created']
        self.tindex = {p: i for i, p in enumerate(self.T)}
        mp = DATA['maps'][c['c']['map']]
        self.mp = mp
        self.tdef = [t['definition'] for t in self.snap['territories']]
        def_to_t = {d: i for i, d in enumerate(self.tdef)}
        self.adj = [[def_to_t[l['target']] for l in mp['territories'][d]['links']] for d in self.tdef]
        self.linked = [i for i, a in enumerate(self.adj) if a]
        self.kind = [mp['territories'][d]['kind'] for d in self.tdef]

    def comp(self, a):
        return self.m.r32(self.created[a]['actor'] + 0xcc)

    def army_state(self, a):
        comp = self.comp(a)
        return {'territory': self.tindex[self.m.r32(comp + 0x20)],
                'path': [self.tindex[t] for t in self.w.read_vector(comp + 0x24)],
                'fought': self.m.r32(comp + 0x10)}

    def move(self, a, t):
        self.m.cw = CW['pc53']
        self.m.call(0x940100, args=(self.T[t],), ecx=self.comp(a))

    def set_owner(self, t, k):
        self.m.cw = CW['pc53']
        self.m.call(0x955620, args=(self.K[k],), ecx=self.T[t])

    def search(self, s, g, actor, budget, cw):
        m = self.m
        mark = m.next
        m.cw = cw
        obj = m.alloc(0x50)
        m.call(0x961550, args=(self.T[s], self.T[g], 0 if actor is None else self.actors[actor],
                               budget & 0xffffffff), ecx=obj)
        code, _ = m.call(0x961350, ecx=obj, count=50_000_000)
        store = m.r32(obj + 0x4c)
        path = [self.tindex[p] for p in self.w.read_vector(store + 0x1c)]
        m.call(0x93e930, ecx=obj)
        m.next = mark
        return {'code': code, 'path': path}

    def moves_used(self, a):
        self.m.cw = CW['pc53']
        eax, _ = self.m.call(0x93eae0, ecx=self.comp(a))
        return eax

    def heuristic(self, node, goal, cw):
        """960E70 on a node store (vtable ACDD68, +0x0C goal) -> ST0 (80-bit)."""
        m = self.m
        mark = m.next
        m.cw = cw
        store = m.alloc(0x28)
        m.w32(store, 0xacdd68)
        m.w32(store + 0xc, goal)
        _, st0 = m.call(0x960e70, args=(node,), ecx=store, store_st0=True)
        m.next = mark
        return ext_to_json(st0)

    def set_locations(self, xy):
        bits = []
        for d, (x, y) in zip(self.ctx['defs'], xy):
            self.m.w32(d + 0x14, x)
            self.m.w32(d + 0x18, y)
            bits.append([x, y, self.m.r32(d + 0x1c)])
        return bits

    def army_path(self, a, target, cw):
        m = self.m
        mark = m.next
        m.cw = cw
        out = m.alloc(12)
        eax, _ = m.call(0x93fdc0, args=(self.T[target], out), ecx=self.comp(a), count=50_000_000)
        path = [self.tindex[p] for p in self.w.read_vector(out)]
        m.next = mark
        return {'ok': eax & 0xff, 'path': path}


def f32bits(v):
    return struct.unpack('<I', struct.pack('<f', v))[0]


def location_variant(W, rng, variant):
    """Replacement definition locations (x, y binary32 bits) for coverage:
    'grid' = multiples of 16 in [0, 80] (many exact heuristic ties: heap
    tie-breaking and the unrounded best-node compare), 'random' = random signs,
    magnitudes 10^-2..10^4 and full significands (inexact x87 differences)."""
    xy = []
    for _ in W.ctx['defs']:
        if variant == 'grid':
            xy.append((f32bits(16.0 * rng.randrange(6)), f32bits(16.0 * rng.randrange(6))))
        else:
            v = lambda: rng.choice((-1.0, 1.0)) * 10.0 ** rng.uniform(-2, 4)
            xy.append((f32bits(v()), f32bits(v())))
    return xy


def phase(W, rng, steps, totals, team, first):
    n, nt, na = len(W.actors), len(W.T), len(W.created)
    if first:
        rel = team
    else:
        p = rng.choice((0.3, 0.6, 0.9))
        rel = [[a != b and rng.random() < p for b in range(n)] for a in range(n)]
    W.w.relations['enemy_140'] = rel
    steps.append({'op': 'relations', 'enemy_140': rel})
    # owner changes
    for _ in range(rng.randrange(0, 8)):
        t = rng.choice(W.linked) if rng.random() < 0.85 else rng.randrange(nt)
        k = rng.randrange(n)
        W.set_owner(t, k)
        steps.append({'op': 'owner', 'territory': t, 'kingdom': k})
    # army walks / teleports
    for _ in range(rng.randrange(6, 16)):
        a = rng.randrange(na)
        cur = W.army_state(a)['territory']
        r = rng.random()
        if r < 0.15:
            t = rng.randrange(nt)                          # teleport anywhere (aliases too)
            W.move(a, t)
            steps.append({'op': 'move', 'army': a, 'territory': t})
        elif r < 0.25 and W.army_state(a)['path']:
            t = rng.choice(W.army_state(a)['path'])        # go back (truncate)
            W.move(a, t)
            steps.append({'op': 'move', 'army': a, 'territory': t})
        else:
            for _ in range(rng.randrange(1, 5)):           # walk along links
                if not W.adj[cur]:
                    break
                cur = rng.choice(W.adj[cur])
                W.move(a, cur)
                steps.append({'op': 'move', 'army': a, 'territory': cur})
    # battles fought this turn
    for _ in range(rng.randrange(0, 4)):
        a = rng.randrange(na)
        v = rng.choice((0, 0, 1, 2))
        W.m.w32(W.comp(a) + 0x10, v)
        steps.append({'op': 'fought', 'army': a, 'value': v})
    steps.append({'op': 'check', 'owners': [tt['owner'] for tt in snapshot(W.ctx)['territories']]})
    # heuristic 960E70 (ST0) on random pairs
    for _ in range(25):
        node, goal = rng.randrange(nt), rng.randrange(nt)
        if rng.random() < 0.1:
            goal = node
        steps.append({'op': 'heuristic', 'node': node, 'goal': goal,
                      'st0': {p: W.heuristic(node, goal, cw) for p, cw in CW.items()}})
        totals['heuristic'] += 1
    # direct searches
    for _ in range(70):
        s = rng.choice(W.linked) if rng.random() < 0.85 else rng.randrange(nt)
        r = rng.random()
        if r < 0.05:
            g = s
        elif r < 0.15 and W.adj[s]:
            g = rng.choice(W.adj[s])
        elif r < 0.85:
            g = rng.choice(W.linked)
        else:
            g = rng.randrange(nt)
        actor = None if rng.random() < 0.2 else rng.randrange(n)
        budget = rng.choice((-1, -1, 0, 1, 2, 3, 4, 5, 6, 8, 12, 40, -2, rng.randrange(0, 10)))
        res = {p: W.search(s, g, actor, budget, cw) for p, cw in CW.items()}
        steps.append({'op': 'search', 'start': s, 'goal': g, 'actor': actor, 'budget': budget,
                      'results': res})
        totals['search'] += 1
    # army queries
    for _ in range(35):
        a = rng.randrange(na)
        st = W.army_state(a)
        r = rng.random()
        cur = st['territory']
        if r < 0.08:
            target = cur
        elif r < 0.3 and W.adj[cur]:
            target = rng.choice(W.adj[cur])
        elif r < 0.45:
            target = rng.choice([i for i, k in enumerate(W.kind) if k == 'Alias'])
        elif r < 0.6 and st['path']:
            target = rng.choice(st['path'])
        elif r < 0.9:
            target = rng.choice(W.linked)
        else:
            target = rng.randrange(nt)
        used = W.moves_used(a)
        res = {p: W.army_path(a, target, cw) for p, cw in CW.items()}
        steps.append({'op': 'army', 'army': a, 'target': target, 'state': st, 'moves_used': used,
                      'results': res})
        totals['army'] += 1


def main():
    rng = random.Random(0x93FDC0)
    rows = []
    totals = {'search': 0, 'army': 0, 'heuristic': 0}
    for ci in range(len(DATA['campaigns'])):
        W = World(ci)
        steps = []
        team = W.w.relations['enemy_140']
        for i in range(8):                               # shipped locations
            phase(W, rng, steps, totals, team, i == 0)
        for variant in ('grid', 'random'):
            steps.append({'op': 'locations', 'variant': variant,
                          'bits': W.set_locations(location_variant(W, rng, variant))})
            for i in range(3):
                phase(W, rng, steps, totals, team, i == 0)
        rows.append({'campaign': ci, 'steps': steps})
        print(json.dumps({'campaign': ci, 'steps': len(steps), 'visited': len(VISITED)}), flush=True)
    report = {
        'schema': 1, 'scope': __doc__,
        'original_sha256': hashlib.sha256(P.b).hexdigest(), 'v14_sha256': hashlib.sha256(V.b).hexdigest(),
        'producer_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        'data_sha256': hashlib.sha256((HERE / 'metagame-data.json').read_bytes()).hexdigest(),
        'control_words': CW,
        'sequences': rows,
        'instructions': [{'va': hex(a), 'bytes': b} for a, b in sorted(VISITED.items())],
    }
    out = HERE / 'native-path.json'
    out.write_text(json.dumps(report, separators=(',', ':')) + '\n')
    # coverage summary
    codes, ok, diff = {}, {}, {'search': 0, 'army': 0, 'heuristic': 0}
    for row in rows:
        for s in row['steps']:
            if s['op'] == 'search':
                c = s['results']['pc53']['code']
                codes[c] = codes.get(c, 0) + 1
            elif s['op'] == 'army':
                o = s['results']['pc53']['ok']
                ok[o] = ok.get(o, 0) + 1
            if s['op'] in diff:
                vals = s['results' if s['op'] != 'heuristic' else 'st0'].values()
                diff[s['op']] += len({json.dumps(v, sort_keys=True) for v in vals}) > 1
    print(json.dumps({'totals': totals, 'search_codes_pc53': codes, 'army_ok_pc53': ok,
                      'precision_dependent_rows': diff, 'visited': len(VISITED),
                      'sha256': hashlib.sha256(out.read_bytes()).hexdigest()}))


if __name__ == '__main__':
    main()
