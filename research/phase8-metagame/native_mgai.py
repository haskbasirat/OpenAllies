"""Original metagame AI (KKC_MGAIPlayer) under Unicorn on perturbed InitCreate
worlds of every shipped campaign.

Per world (campaign ci, seeded variant): MGAI data objects are built from
metagame-data.json 'ai' at their native offsets (optionally perturbed and
with synthetic per-territory records: fixture 'ai'), attached to the
campaign kingdom data (+0x14); world perturbations are applied with
original code (940100 relocation, 955620 owner change) or recorded field
writes; one KKC_MGAIPlayer per MG kingdom is built by the original
constructor 963430(controller, MG kingdom) (Analysis 969CB0, TA 96A780,
goals 9685A0/9686A0/968620) and then PlayTurn 964D30 runs for chosen
kingdoms over several rounds (the players persist, as in the game:
proximity is computed once, goal thresholds carry over).

Observed (fixture), in order: the analysis after DoMapAnalysis 962B70 (per
territory: record, influence binary32 per kingdom, importance, border,
front, proximity; border and front lists), the goals (threshold, items:
territory, priority binary32 bits, assigned armies) after every
SetGoalPriorities 9635A0 return inside PlayTurn and after the other
stages, every command issued through 944F50 (fields read from the
KKC_MGCommand), every yield 944CE0 (argument, scripted result, scripted
battle-resolved flag MGState+0x46, scripted tick [B36534]+0xDC, scripted
arrival: the first issued, not yet applied move/attack order's army is
relocated by the original 940100 to the order's territory), every army
"busy" query (army actor +0x70 vtable +0x64, scripted), every uninitialised
buffer of 96A5D0 (length, scripted nonzero byte count), the app RNG word
([[B36528]+0x2C], 85D840) after the turn, the PlayTurn result and the
player bytes +0x28/+0x29.

Seams: those of mg_campaign / mg_world (CRT heap, _wcsicmp, engine lookups,
army creation, kingdom actor +0x138/+0x140 relation matrices, army actor
+0x108 owner), the debug log 404F60, the timer 59ABA0, yield 944CE0,
command issue 944F50, army busy (body vtable +0x64), and the contents of the
96A5D0 heap buffer (operator new 6F7810 called from 96A684 returns bytes
from a seeded generator; every other allocation is zeroed).
x87: PC53 (0x027F, the metagame thread) for most turns, PC24 and PC64 for
some (field 'cw').
"""
import json, hashlib, random, sys, time, copy
from pathlib import Path
from native_mgai_world import MgaiWorld, BLOCKS, DATA, f32, AI_INTS, AI_FLOATS, DIFFS
from mg_machine import P, V, HERE
from unicorn import UC_HOOK_CODE

CW = {'pc53': 0x027f, 'pc24': 0x007f, 'pc64': 0x037f}
# The data the world was built from (mg_campaign.DATA, loaded at import):
# pinned now and re-checked before writing the fixture.
_DATA_BYTES = (HERE / 'metagame-data.json').read_bytes()
assert json.loads(_DATA_BYTES) == DATA, 'metagame-data.json changed while importing'
DATA_SHA256 = hashlib.sha256(_DATA_BYTES).hexdigest()
STAGES = {0x964da4: 'analysis', 0x964dd0: 'goals1', 0x964e0a: 'purchase', 0x964e4c: 'goals2',
          0x964e8e: 'attack', 0x964ebe: 'goals3', 0x964f00: 'move', 0x964f34: 'end'}
RATIOS = [0x3dcccccd, 0x3e800000, 0x3ecccccd, 0x3f000000, 0x3f19999a, 0x3f400000, 0x3f666666]
MULTS = [0x3e800000, 0x3f000000, 0x3f400000, 0x3f19999a, 0x3f666666, 0x3f800000, 0x3fc00000]


def ai_values(ai_index, rng, perturb, n_territories):
    v = copy.deepcopy(DATA['ai'][ai_index])
    if not perturb:
        return v
    for key, _ in AI_INTS:
        if rng.random() < 0.3:
            v[key] = {'no_attack_distance': rng.randint(2, 12), 'max_influence_depth': rng.randint(0, 4)}.get(key, rng.randint(0, 20))
    for key, _ in AI_FLOATS:
        if rng.random() < 0.3:
            v[key] = rng.choice(MULTS if 'multiplier' in key else RATIOS)
    for mr in v['maps']:
        for key in ('default_territory', 'capital', 'friendly_capital', 'enemy_capital'):
            if rng.random() < 0.3:
                mr[key] = rand_record(rng, None)
        for _ in range(rng.randint(0, 8)):
            mr['territories'].append(rand_record(rng, rng.randrange(n_territories)))
    return v


def rand_record(rng, territory):
    return {'territory': territory, 'additional_importance': rng.randint(0, 30),
            'additional_neighbor_proximity_importance_depth': rng.randint(0, 3),
            'additional_neighbor_proximity_importance_max': rng.randint(0, 20),
            'required_attack_strength_ratio_bits': rng.choice(RATIOS),
            'desired_defense_threat_ratio_bits': rng.choice(RATIOS)}


class Case:
    def __init__(self, W):
        self.W = W
        self.m = W.m
        self.players = {}
        self.records = {}      # record pointer -> label
        self.active = None
        for addr, name in STAGES.items():
            self.m.u.hook_add(UC_HOOK_CODE, self._observe(name), begin=addr, end=addr)

    def _observe(self, name):
        def hook(u, pc, size, _):
            if self.active is not None:
                self.active['stages'].append({'stage': name, 'issued': len(self.W.issued),
                                              'yields': len(self.W.yield_log),
                                              'analysis': self.analysis(self.active_player) if name == 'analysis' else None,
                                              'goals': self.goals(self.active_player),
                                              'flags': [self.m.u.mem_read(self.active_player + o, 1)[0] for o in (0x28, 0x29)]})
        return hook

    def label_records(self, ai_index, mgai):
        m, w = self.m, self.W.w
        for mi, mp in enumerate(w.read_vector(mgai + 0x18)):
            for key, off in (('default', 0x14), ('capital', 0x18), ('friendly_capital', 0x1c), ('enemy_capital', 0x20)):
                self.records[m.r32(mp + off)] = [key, None]
            for i, r in enumerate(w.read_vector(mp + 8)):
                self.records[r] = ['territory', i]

    def tas(self, player):
        return self.W.w.read_vector(self.m.r32(player + 0xc) + 0xc)

    def analysis(self, player):
        m, w = self.m, self.W.w
        an = m.r32(player + 0xc)
        tas = w.read_vector(an + 0xc)
        out = []
        for ta in tas:
            assert m.r32(ta + 0xc) == self.W.T[len(out)]
            out.append({'record': self.records[m.r32(ta + 4)],
                        'influence': w.read_vector(ta + 0x10),
                        'importance': m.r32(ta + 0x1c) - ((m.r32(ta + 0x1c) >> 31) << 32),
                        'border': m.u.mem_read(ta + 0x20, 1)[0], 'front': m.u.mem_read(ta + 0x21, 1)[0],
                        'proximity': m.r32(ta + 0x24) - ((m.r32(ta + 0x24) >> 31) << 32)})
        return {'territories': out,
                'borders': [tas.index(t) for t in w.read_vector(an + 0x18)],
                'fronts': [tas.index(t) for t in w.read_vector(an + 0x24)],
                'proximity_done': m.u.mem_read(an + 0x30, 1)[0]}

    def goals(self, player):
        m, w = self.m, self.W.w
        tas = self.tas(player)
        out = []
        for off in (0x1c, 0x20, 0x24):
            g = m.r32(player + off)
            items = []
            for it in w.read_vector(g + 8):
                armies = []
                for h in w.read_vector(it + 8):
                    a = w.resolve(h)
                    armies.append(w.kind[a][1] if a else None)
                items.append({'territory': tas.index(m.r32(it + 4)), 'priority': m.r32(it + 0x14), 'armies': armies})
            out.append({'threshold': m.r32(g + 0x14), 'items': items})
        return out


def main():
    ci = int(sys.argv[1])
    variants = int(sys.argv[2]) if len(sys.argv) > 2 else 2
    rounds_per = int(sys.argv[3]) if len(sys.argv) > 3 else 6
    seed = 0x964d30 + ci
    rng = random.Random(seed)
    worlds = []
    t0 = time.time()
    n_turns = 0
    for ci in [ci]:
        for variant in range(variants):
            W = MgaiWorld(ci, cw=0x027f)
            m, w, st = W.m, W.w, W.st
            case = Case(W)
            camp = W.camp
            nk = len(W.K)
            n_t = len(W.T)
            n_map = len(DATA['maps'][camp['map']]['territories'])
            perturb_ai = variant > 0
            ai = {}
            for k in camp['kingdoms']:
                i = k['mgai']
                if i not in ai:
                    ai[i] = ai_values(i, rng, perturb_ai, n_map)
                    case.label_records(i, W.build_mgai(i, ai[i]))
            W.attach_mgai()
            ops = []
            # relations: team based, or a random enemy matrix (diagonal false)
            if variant > 0 and rng.random() < 0.5:
                rel = {'enemy_140': [[a != b and rng.random() < 0.6 for b in range(nk)] for a in range(nk)],
                       'relation_138': [[a == b or rng.random() < 0.4 for b in range(nk)] for a in range(nk)]}
                w.relations.update(rel)
            ops.append(['relations', w.relations['enemy_140'], w.relations['relation_138']])
            perturb(W, rng, ops, initial=True)
            # players (963430) after the initial perturbation (records bind to capitals/relations now)
            for k in range(nk):
                p = m.alloc(0x2c)
                m.call(0x963430, args=(W.controller, W.K[k]), ecx=p, count=50_000_000)
                case.players[k] = p
            construct = {k: case.analysis(p) for k, p in case.players.items()}
            ops.append(['construct'])
            turns = []
            for rd in range(rounds_per):
                rops = []
                if rd > 0:
                    if rng.random() < 0.3:
                        rel = {'enemy_140': [[a != b and rng.random() < 0.6 for b in range(nk)] for a in range(nk)],
                               'relation_138': [[a == b or rng.random() < 0.4 for b in range(nk)] for a in range(nk)]}
                        w.relations.update(rel)
                        rops.append(['relations', rel['enemy_140'], rel['relation_138']])
                    perturb(W, rng, rops, initial=False)
                for k in rng.sample(range(nk), rng.randint(1, 2)):
                    pc = rng.choice(['pc53', 'pc53', 'pc53', 'pc24', 'pc64'])
                    m.cw = CW[pc]
                    rng_word = rng.getrandbits(32)
                    m.w32(W.rng_word, rng_word)
                    trng = random.Random(rng.getrandbits(32))
                    abort_at = rng.randint(1, 30) if rng.random() < 0.2 else None
                    ycount = [0]
                    set46_p = rng.choice([0.2, 0.5, 1.0])
                    dt_max = rng.choice([30, 60, 200, 500])
                    def yscript(arg, r=trng, a=abort_at, s=set46_p, d=dt_max, c=ycount):
                        c[0] += 1
                        return (0 if c[0] == a else 1, int(arg == 1 and r.random() < s), r.randint(0, d))
                    W.yield_script = yscript
                    brng = random.Random(rng.getrandbits(32))
                    busy_p = rng.choice([0, 0.3])
                    W.busy_script = lambda idx, r=brng, b=busy_p: int(r.random() < b)
                    W.uninit_rng = random.Random(rng.getrandbits(32)) if rng.random() < 0.7 else None
                    arng = random.Random(rng.getrandbits(32))
                    arrive_p = rng.choice([0, 0.3, 0.7])
                    W.arrive_script = lambda r=arng, a=arrive_p: r.random() < a
                    W.applied = set()
                    W.issued.clear(); W.yield_log.clear(); W.busy_log.clear(); W.uninit_log.clear()
                    rec = {'kingdom': k, 'cw': pc, 'rng_in': rng_word, 'tick_in': m.r32(W.world + 0xdc),
                           'flag46_in': m.u.mem_read(st + 0x46, 1)[0], 'stages': []}
                    case.active = rec
                    case.active_player = case.players[k]
                    r = W.run_resumable(0x964d30, ecx=case.players[k])
                    case.active = None
                    W.m.stop_coverage()
                    rec.update({'result': r & 0xff, 'rng_out': m.r32(W.rng_word),
                                'issued': list(W.issued), 'yields': list(W.yield_log),
                                'busy': list(W.busy_log), 'uninit': list(W.uninit_log),
                                'flag46_out': m.u.mem_read(st + 0x46, 1)[0]})
                    rops.append(['turn', len(turns)])
                    turns.append(rec)
                    n_turns += 1
                    print(f'ci {ci} v {variant} round {rd} k {k} {pc} result {r & 0xff} issued {len(W.issued)} '
                          f'yields {len(W.yield_log)} t={time.time() - t0:.0f}s', flush=True)
                ops.extend(rops)
            worlds.append({'campaign': ci, 'variant': variant, 'ai': {str(k): v for k, v in ai.items()},
                           'ops': ops, 'construct': {str(k): v for k, v in construct.items()}, 'turns': turns})
    report = {
        'schema': 1, 'scope': __doc__, 'seed': seed, 'campaign': ci, 'variants': variants, 'rounds': rounds_per,
        'original_sha256': hashlib.sha256(P.b).hexdigest(), 'v14_sha256': hashlib.sha256(V.b).hexdigest(),
        'producer_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        'world_sha256': hashlib.sha256((HERE / 'native_mgai_world.py').read_bytes()).hexdigest(),
        'data_sha256': DATA_SHA256,
        'original_v14_difference_ranges': [[hex(a), n] for a, n in DIFFS],
        'worlds': worlds,
        'blocks': [{'va': hex(a), 'bytes': b} for a, b in sorted(BLOCKS.items())],
    }
    assert hashlib.sha256((HERE / 'metagame-data.json').read_bytes()).hexdigest() == DATA_SHA256, 'metagame-data.json changed during the run'
    out = HERE / f'native-mgai-c{ci}.json'
    out.write_text(json.dumps(report) + '\n')
    print(json.dumps({'turns': n_turns, 'blocks': len(BLOCKS), 'sha256': hashlib.sha256(out.read_bytes()).hexdigest()}))


def perturb(W, rng, ops, initial):
    m, w, st = W.m, W.w, W.st
    nk, n_t = len(W.K), len(W.T)
    created = W.created
    messy = rng.random() < 0.4
    # owners
    for _ in range(rng.randint(0, 6 if initial else 3)):
        t = rng.randrange(n_t); k = rng.randrange(nk)
        m.call(0x955620, args=(W.K[k],), ecx=W.T[t])
        ops.append(['owner', t, k])
    # lost capitals: a capital handed to another kingdom (955620)
    if rng.random() < 0.35:
        for k, kp in enumerate(W.K):
            cap = m.r32(kp + 0x10)
            if cap and rng.random() < 0.4:
                o = rng.choice([x for x in range(nk) if x != k])
                t = W.T.index(cap)
                m.call(0x955620, args=(W.K[o],), ecx=cap)
                ops.append(['owner', t, o])
    # army moves along links (several steps); battle-free unless messy
    for _ in range(rng.randint(2, 14)):
        a = rng.randrange(len(created))
        actor = created[a]['actor']
        comp = m.r32(actor + 0xcc)
        cur = W.T.index(m.r32(comp + 0x20))
        links = w.read_vector(m.r32(W.T[cur] + 0xc) + 0x68)
        if not links:
            continue
        tdef = m.r32(rng.choice(links) + 8)
        t = next(i for i, tp in enumerate(W.T) if m.r32(tp + 0xc) == tdef)
        owner = m.r32(W.T[t] + 0x24)
        if not messy and owner:
            ok = W.actors.index(w.army_owner[actor])
            if w.relations['enemy_140'][W.K.index(owner)][ok]:
                continue
        m.call(0x940100, args=(W.T[t],), ecx=comp)
        ops.append(['move', a, t])
    # turn start (93FE10 effect: fought 0, path = [current]) for most armies
    for a in range(len(created)):
        comp = m.r32(created[a]['actor'] + 0xcc)
        r = rng.random()
        if r < 0.7:
            m.w32(comp + 0x10, 0)
            cur = m.r32(comp + 0x20)
            w.vector(comp + 0x24, [cur], 4)
            ops.append(['turn_start', a])
        elif r < 0.75:
            f = rng.choice([0, 1])
            m.w32(comp + 0x10, f)
            ops.append(['fought', a, f])
    # divisions: random shop divisions of the owner added for some armies
    for _ in range(rng.randint(0, 4)):
        a = rng.randrange(len(created))
        comp = m.r32(created[a]['actor'] + 0xcc)
        have = w.read_vector(comp + 0x14)
        new = list(have)
        for d in rng.sample(W.divisions, rng.randint(0, 3)):
            if d not in new:
                new.append(d)
        w.vector(comp + 0x14, new, max(5, len(new)))
        ops.append(['divisions', a, [W.divisions.index(d) for d in new]])
    # fully upgraded kingdoms (962DB0 false): every army gets every shop division
    for k, ka in enumerate(W.actors):
        if rng.random() < 0.12:
            shop = m.r32(W.kdatas[k] + 0x60)
            divs = [m.r32(e + 8) for e in w.read_vector(shop + 0x18)]
            for a, c in enumerate(created):
                if w.army_owner[c['actor']] != ka:
                    continue
                comp = m.r32(c['actor'] + 0xcc)
                new = list(w.read_vector(comp + 0x14))
                for d in divs:
                    if d and d not in new:
                        new.append(d)
                w.vector(comp + 0x14, new, max(5, len(new)))
                ops.append(['divisions', a, [W.divisions.index(d) for d in new]])
    for k, kp in enumerate(W.K):
        r = rng.choice([0, 5, 10, 25, 40, 75, 150, 300, 1000, rng.randint(0, 600)])
        b = rng.choice([0, 0, 0, 1])
        m.w32(kp + 0x38, r); m.w32(kp + 0x3c, b)
        ops.append(['resources', k, r]); ops.append(['battles', k, b])
    f46 = rng.choice([0, 0, 1])
    m.u.mem_write(st + 0x46, bytes([f46])); ops.append(['flag46', f46])
    f51 = rng.choice([0, 1])
    m.u.mem_write(st + 0x51, bytes([f51])); ops.append(['flag51', f51])
    if rng.random() < 0.1:
        a = rng.randrange(len(created))
        actor = created[a]['actor']
        m.w32(actor + 8, m.r32(actor + 8) | 1)
        ops.append(['deleted', a])
    # kingdom actor army lists: armies owned by each kingdom actor, shuffled
    orders = {}
    for k, ka in enumerate(W.actors):
        own = [i for i, c in enumerate(created) if w.army_owner[c['actor']] == ka]
        if rng.random() < 0.5:
            rng.shuffle(own)
        orders[k] = own
    W.build_lists(orders)
    ops.append(['lists', [orders[k] for k in range(len(W.actors))]])


if __name__ == '__main__':
    main()
