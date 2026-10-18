"""Original army display positions (KKC_TerritoryPositionSolver) on the
InitCreate worlds of every shipped campaign.

Trig tables: the cos/sin tables built by the original static initializers
9C8620/9C8640 under Unicorn (FSIN/FCOS emulated through binary64) are
compared bit for bit with aa-trig's research/phase4-trig/lookup-tables.bin at
every control word, then aa-trig's tables are written over them, so every
row below runs on aa-trig's tables.

Rows (each at x87 PC53 = GAME_PRECISION, PC24 and PC64 unless noted):
  layout     the layout objects of the actor_spacing_type enum table
             (959B20 -> 959A50: SIMPLE 959390, CIRCLE 959560, SQUARE
             959240) called through their vtable +4 (out, center, n,
             distance).
  solve      959420 on a 959220 solver (MG territory, owner) with index -1
             (954A10 first free slot) or an explicit index.
  territory  9552A0 -> 957DD0 (territory reference position, visual z).
  realign    9626D0(army actor, territory): 93EBF0 distances from the army
             actor engine position (actor +0x20, written here) to the
             definition and its alias partner, 954CC0, 959420, 4EE5C0.
  waypoints  962880(army actor, from, to, flag): alias choice by distance,
             959420 with -1, link intermediate points, 9627D0 per point.
  place      955B70(army type) on an owned territory (creates the army with
             the mg_campaign creation seam; PC53 only, it changes the state).

State changes: army relocations with the original 940100. Coverage variants
(fixture ops, applied to MetaGameData by the Rust replay): 'defs' replaces
every definition's actor_spacing (+0x4C), actor_spacing_distance (+0x48) and
actor slots (+0x5C/+0x60); 'links' writes intermediate points (+0x0C/+0x10)
on the link objects; 'visual_z' writes the territory visual object +0x28 or
clears the territory visual handle (+0x20). The shipped data has only CIRCLE
spacing, distance 1.5, one slot per territory and no intermediate points.
mg_campaign does not lay out definition bounds (+0x20 min, +0x2C max) or
link points: they are written from metagame-data.json first.

Seams: those of mg_campaign / mg_world (CRT heap, _wcsicmp, engine lookups,
army creation 4FA030, owner +0x108) plus 4EE5C0 (actor set position, thiscall
1 arg) and 9627D0 (move-order waypoint, thiscall 3 args), both recorded.

Harness speed-up (no semantic change): mg_machine.Machine._hook is replaced
by one that byte-checks each distinct (address, size) once; allocations of a
query are released by resetting the bump allocator.
"""
import json, hashlib, random, struct
from pathlib import Path
import mg_machine
from mg_machine import P, V, VISITED, HERE, ROOT

CW = {'pc53': 0x027f, 'pc24': 0x007f, 'pc64': 0x037f}   # PC53 = GAME_PRECISION
TRIG_BIN = ROOT / 'research/phase4-trig/lookup-tables.bin'
SPACING = {'Simple': 0, 'Circle': 1, 'Square': 2}
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
from mg_campaign import DATA, run  # noqa: E402  (after the hook patch)
from mg_world import install_crt, install_static_tables  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX  # noqa: E402


def f32bits(v):
    return struct.unpack('<I', struct.pack('<f', v))[0]


def bits_f32(b):
    return struct.unpack('<f', struct.pack('<I', b))[0]


class World:
    def __init__(self, ci):
        self.snap, ctx = run(ci, keep=True)
        self.ctx = ctx
        m, w, st = ctx['m'], ctx['w'], ctx['st']
        self.m, self.w, self.st = m, w, st
        self.T = w.read_vector(st + 0x28)
        self.tindex = {p: i for i, p in enumerate(self.T)}
        self.created = ctx['created']
        self.mp = DATA['maps'][ctx['c']['map']]
        self.defs = ctx['defs']
        self.tdef = [t['definition'] for t in self.snap['territories']]
        # trig tables: original static init under Unicorn vs aa-trig
        trig = TRIG_BIN.read_bytes()
        cos_p, sin_p = m.r32(0xb39a48 + 8), m.r32(0xb39a60 + 8)
        emulated = bytes(m.u.mem_read(cos_p, 0x4000)) + bytes(m.u.mem_read(sin_p, 0x4000))
        self.tables = {'emulated_sha256': hashlib.sha256(emulated).hexdigest(),
                       'aa_trig_sha256': hashlib.sha256(trig).hexdigest(),
                       'equal': emulated == trig,
                       'objects': [[m.r32(o + 4), m.r32(o + 0xc), m.u.mem_read(o + 0x10, 1)[0]]
                                   for o in (0xb39a48, 0xb39a60)]}
        m.u.mem_write(cos_p, trig[:0x4000])
        m.u.mem_write(sin_p, trig[0x4000:])
        # definition bounds and link points (not laid out by mg_campaign)
        self.link_objs = []
        for t, d in zip(self.mp['territories'], self.defs):
            for i in range(3):
                m.w32(d + 0x20 + 4 * i, t['bounds_min'][i])
                m.w32(d + 0x2c + 4 * i, t['bounds_max'][i])
            links = w.read_vector(d + 0x68)
            assert len(links) == len(t['links'])
            self.link_objs.append(links)
            for lo, l in zip(links, t['links']):
                self.set_points(lo, l['intermediate_points'])
        # spacing layouts (enum table B4A524, data B4A528)
        table = m.r32(0xb4a528)
        self.layouts = [m.r32(m.r32(table + 4 * i) + 8) for i in range(3)]
        self.layout_fn = [m.r32(m.r32(o) + 4) for o in self.layouts]
        assert self.layout_fn == [0x959390, 0x959560, 0x959240], [hex(x) for x in self.layout_fn]
        self.visual_handles = [m.r32(t + 0x20) for t in self.T]
        # engine seams recorded by the queries
        self.log = []
        def set_position(mm):
            p = mm.arg(0)
            self.log.append(('pos', mm.u.reg_read(UC_X86_REG_ECX), [mm.r32(p + 4 * i) for i in range(3)]))
            mm.ret(4)
        def waypoint(mm):
            p = mm.arg(1)
            self.log.append(('wp', mm.arg(0), [mm.r32(p + 4 * i) for i in range(3)], mm.arg(2) & 0xff))
            mm.ret(12)
        m.seams[0x4ee5c0] = set_position
        m.seams[0x9627d0] = waypoint

    def set_points(self, link, points):
        n = len(points)
        arr = self.m.alloc(max(12, 12 * n))
        for j, p in enumerate(points):
            for k in range(3):
                self.m.w32(arr + 12 * j + 4 * k, p[k])
        self.m.w32(link + 0xc, arr)
        self.m.w32(link + 0x10, n)
        self.m.w32(link + 0x14, max(1, n))

    def comp(self, a):
        return self.m.r32(self.created[a]['actor'] + 0xcc)

    def army_territory(self, a):
        return self.tindex[self.m.r32(self.comp(a) + 0x20)]

    def move(self, a, t):
        self.m.cw = CW['pc53']
        self.m.call(0x940100, args=(self.T[t],), ecx=self.comp(a))

    def visual_z(self):
        out = []
        for t in self.T:
            obj = self.w.resolve(self.m.r32(t + 0x20))
            out.append(None if obj == 0 else self.m.r32(obj + 0x28))
        return out

    def owner_actor(self, t):
        k = self.m.r32(self.T[t] + 0x24)
        return 0 if k == 0 else self.w.resolve(self.m.r32(k + 0xc))

    # ---- queries (allocations released afterwards)
    def _scoped(self, cw, fn):
        m = self.m
        mark = m.next
        m.cw = cw
        try:
            return fn()
        finally:
            m.next = mark

    def layout(self, spacing, center, n, distance, cw):
        def go():
            out, c = self.m.alloc(12), self.m.alloc(12)
            for i in range(3):
                self.m.w32(c + 4 * i, center[i])
            self.m.call(self.layout_fn[spacing], args=(out, c, n & 0xffffffff, distance), ecx=self.layouts[spacing])
            return [self.m.r32(out + 4 * i) for i in range(3)]
        return self._scoped(cw, go)

    def solve(self, t, index, cw):
        def go():
            solver, out = self.m.alloc(12), self.m.alloc(12)
            self.m.call(0x959220, args=(self.T[t], self.owner_actor(t)), ecx=solver)
            self.m.call(0x959420, args=(out, index & 0xffffffff), ecx=solver)
            return [self.m.r32(out + 4 * i) for i in range(3)]
        return self._scoped(cw, go)

    def territory_position(self, t, cw):
        def go():
            out = self.m.alloc(12)
            self.m.call(0x9552a0, args=(out,), ecx=self.T[t])
            return [self.m.r32(out + 4 * i) for i in range(3)]
        return self._scoped(cw, go)

    def realign(self, a, t, cw):
        def go():
            self.log = []
            eax, _ = self.m.call(0x9626d0, args=(self.created[a]['actor'], self.T[t]))
            pos = [e[2] for e in self.log if e[0] == 'pos']
            assert len(pos) == (eax & 0xff), (eax, self.log)
            return {'ok': eax & 0xff, 'position': pos[0] if pos else None}
        return self._scoped(cw, go)

    def waypoints(self, a, src, dst, flag, cw):
        def go():
            self.log = []
            this = self.m.alloc(0x40)
            eax, _ = self.m.call(0x962880, args=(self.created[a]['actor'], self.T[src], self.T[dst], flag),
                                 ecx=this)
            assert eax & 0xff == 1
            wps = [e for e in self.log if e[0] == 'wp']
            assert all(e[1] == self.created[a]['actor'] for e in wps)
            return {'points': [e[2] for e in wps], 'flags': [e[3] for e in wps]}
        return self._scoped(cw, go)


def random_coordinate(rng):
    return f32bits(rng.choice((-1.0, 1.0)) * 10.0 ** rng.uniform(-2, 3))


def random_distance(rng):
    r = rng.random()
    if r < 0.1:
        return 0
    if r < 0.2:
        return f32bits(-rng.uniform(0.1, 4))
    return f32bits(10.0 ** rng.uniform(-2, 2))


def layout_rows(W, rng, steps, totals):
    centers = [[t['location'][0], t['location'][1], t['location'][2]] for t in W.mp['territories'][:8]]
    for spacing in range(3):
        ns = list(range(0, 80)) + [rng.randrange(80, 1200) for _ in range(40)]
        for n in ns:
            center = rng.choice(centers) if rng.random() < 0.5 else [random_coordinate(rng), random_coordinate(rng),
                                                                       random_coordinate(rng)]
            distance = 0x3fc00000 if rng.random() < 0.3 else random_distance(rng)
            res = {p: W.layout(spacing, center, n, distance, cw) for p, cw in CW.items()}
            steps.append({'op': 'layout', 'spacing': spacing, 'center': center, 'n': n, 'distance': distance,
                          'results': res})
            totals['layout'] += 1


def defs_variant(W, rng):
    out = []
    for d in W.defs:
        spacing = rng.randrange(3)
        distance = random_distance(rng)
        slots = [[f32bits(rng.uniform(-6, 6)), f32bits(rng.uniform(-6, 6)), f32bits(rng.uniform(-1, 1))]
                 for _ in range(rng.choice((1, 1, 2, 3, 5)))]
        W.m.w32(d + 0x4c, spacing)
        W.m.w32(d + 0x48, distance)
        arr = W.m.alloc(12 * len(slots))
        for i, s in enumerate(slots):
            for k in range(3):
                W.m.w32(arr + 12 * i + 4 * k, s[k])
        W.m.w32(d + 0x5c, arr)
        W.m.w32(d + 0x60, len(slots))
        W.m.w32(d + 0x64, len(slots))
        out.append({'spacing': spacing, 'distance': distance, 'slots': slots})
    return out


def links_variant(W, rng):
    out = []
    for t, links in zip(W.mp['territories'], W.link_objs):
        per = []
        for lo in links:
            pts = [[random_coordinate(rng), random_coordinate(rng), random_coordinate(rng)]
                   for _ in range(rng.choice((0, 0, 1, 2, 3)))]
            W.set_points(lo, pts)
            per.append(pts)
        out.append(per)
    return out


def visual_variant(W, rng):
    for i, t in enumerate(W.T):
        r = rng.random()
        if r < 0.2:
            W.m.w32(t + 0x20, 0)                       # handle no longer resolves
        else:
            W.m.w32(t + 0x20, W.visual_handles[i])
            obj = W.w.resolve(W.visual_handles[i])
            W.m.w32(obj + 0x28, 0 if r < 0.4 else random_coordinate(rng))
    return W.visual_z()


def phase(W, rng, steps, totals, variant):
    nt, na = len(W.T), len(W.created)
    if variant:
        steps.append({'op': 'defs', 'defs': defs_variant(W, rng)})
        steps.append({'op': 'links', 'links': links_variant(W, rng)})
    steps.append({'op': 'visual_z', 'z': visual_variant(W, rng) if variant else W.visual_z()})
    adj = [[W.tdef.index(l['target']) for l in W.mp['territories'][d]['links']] for d in W.tdef]
    # relocations (crowd some territories so that layouts are used)
    crowd = rng.sample(range(nt), 6)
    for _ in range(40):
        a = rng.randrange(na)
        t = rng.choice(crowd) if rng.random() < 0.6 else rng.randrange(nt)
        W.move(a, t)
        steps.append({'op': 'move', 'army': a, 'territory': t})
    # solver on every territory: first free slot and explicit indices
    for t in range(nt):
        for index in [-1] + [rng.randrange(0, 12) for _ in range(2)] + [rng.randrange(12, 200)]:
            steps.append({'op': 'solve', 'territory': t, 'index': index,
                          'results': {p: W.solve(t, index, cw) for p, cw in CW.items()}})
            totals['solve'] += 1
        steps.append({'op': 'territory', 'territory': t,
                      'results': {p: W.territory_position(t, cw) for p, cw in CW.items()}})
        totals['territory'] += 1
    # alias-related territories (definition with a partner)
    paired = [i for i, d in enumerate(W.tdef) if W.mp['territories'][d]['alias_partner'] is not None]
    for _ in range(40):
        a = rng.randrange(na)
        t = rng.choice(paired) if rng.random() < 0.8 else rng.randrange(nt)
        if rng.random() < 0.5:
            W.move(a, t)                                 # listed in t: its own slot
            steps.append({'op': 'move', 'army': a, 'territory': t})
        d = W.mp['territories'][W.tdef[t]]
        base = d['location'] if d['alias_partner'] is None or rng.random() < 0.5 else \
            W.mp['territories'][d['alias_partner']]['location']
        pos = [f32bits(bits_f32(base[0]) + rng.uniform(-8, 8)), f32bits(bits_f32(base[1]) + rng.uniform(-8, 8)),
               f32bits(rng.uniform(-3, 3))]
        actor = W.created[a]['actor']
        for i in range(3):
            W.m.w32(actor + 0x20 + 4 * i, pos[i])
        steps.append({'op': 'realign', 'army': a, 'territory': t, 'actor_position': pos,
                      'results': {p: W.realign(a, t, cw) for p, cw in CW.items()}})
        totals['realign'] += 1
    for _ in range(40):
        a = rng.randrange(na)
        src = rng.randrange(nt)
        r = rng.random()
        if r < 0.5 and adj[src]:
            dst = rng.choice(adj[src])
        elif r < 0.8:
            dst = rng.choice(paired)
        else:
            dst = rng.randrange(nt)
        flag = rng.randrange(2)
        steps.append({'op': 'waypoints', 'army': a, 'from': src, 'to': dst, 'flag': flag,
                      'results': {p: W.waypoints(a, src, dst, flag, cw) for p, cw in CW.items()}})
        totals['waypoints'] += 1


def placements(W, rng, steps, totals):
    """955B70 at PC53 only: each call creates an army (state change)."""
    owned = [i for i in range(len(W.T)) if W.m.r32(W.T[i] + 0x24)]
    things = sorted(W.ctx['things'].items())
    for _ in range(15):
        t = rng.choice(owned)
        ids, thing = rng.choice(things)
        W.m.cw = CW['pc53']
        before = len(W.created)
        actor, _ = W.m.call(0x955b70, args=(thing,), ecx=W.T[t])
        assert len(W.created) == before + 1 and W.created[-1]['actor'] == actor
        c = W.created[-1]
        assert W.army_territory(before) == t
        steps.append({'op': 'place', 'territory': t, 'army_type': ids,
                      'owner': W.ctx['actors'].index(c['owner']), 'position_pc53': c['position_bits']})
        totals['place'] += 1


def main():
    rng = random.Random(0x959420)
    rows, tables = [], []
    totals = {'layout': 0, 'solve': 0, 'territory': 0, 'realign': 0, 'waypoints': 0, 'place': 0}
    layouts = []
    for ci in range(len(DATA['campaigns'])):
        W = World(ci)
        tables.append({'campaign': ci, **W.tables})
        if ci == 0:
            for cw in CW.values():                         # static init at the other precisions too
                m2 = mg_machine.Machine(cw)
                install_crt(m2)
                install_static_tables(m2)
                e = bytes(m2.u.mem_read(m2.r32(0xb39a48 + 8), 0x4000)) + bytes(m2.u.mem_read(m2.r32(0xb39a60 + 8), 0x4000))
                tables.append({'static_init_cw': cw, 'equal': e == TRIG_BIN.read_bytes(),
                               'emulated_sha256': hashlib.sha256(e).hexdigest()})
            layout_rows(W, rng, layouts, totals)
        steps = []
        for variant in (False, True, True):
            phase(W, rng, steps, totals, variant)
        placements(W, rng, steps, totals)
        rows.append({'campaign': ci, 'steps': steps})
        print(json.dumps({'campaign': ci, 'steps': len(steps), 'visited': len(VISITED)}), flush=True)
    assert all(t['equal'] for t in tables), tables
    report = {
        'schema': 1, 'scope': __doc__,
        'original_sha256': hashlib.sha256(P.b).hexdigest(), 'v14_sha256': hashlib.sha256(V.b).hexdigest(),
        'producer_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        'data_sha256': hashlib.sha256((HERE / 'metagame-data.json').read_bytes()).hexdigest(),
        'trig_tables': tables,
        'control_words': CW,
        'layouts': layouts,
        'sequences': rows,
        'instructions': [{'va': hex(a), 'bytes': b} for a, b in sorted(VISITED.items())],
    }
    out = HERE / 'native-positions.json'
    out.write_text(json.dumps(report, separators=(',', ':')) + '\n')
    diff = {}
    for s in layouts + [s for r in rows for s in r['steps']]:
        if 'results' in s:
            vals = {json.dumps(v, sort_keys=True) for v in s['results'].values()}
            diff[s['op']] = diff.get(s['op'], 0) + (len(vals) > 1)
    print(json.dumps({'totals': totals, 'precision_dependent_rows': diff, 'visited': len(VISITED),
                      'sha256': hashlib.sha256(out.read_bytes()).hexdigest()}))


if __name__ == '__main__':
    main()
