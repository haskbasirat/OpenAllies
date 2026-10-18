"""Original metagame turn advance on perturbed InitCreate worlds.

Runs the ORIGINAL KKC_MGCommand::ProcessEndTurn 952790 (message block
9527CB..95288D = seam), the advance loop 9391F0, can-play test 938DA0
(95A350; kingdom actor +284 list = seam built from live armies by owner,
93EB30 movement), next kingdom 938C90 (938A80 result clear, battles
counters, territory turn start 9556D0 with the auto-capture of territories
whose defender strength 954FD0 is 0.0f and +1C > 0 (953A20 seam, resolver
966FF0 / 967530 with the whole resolution chain, 938E10), army turn start
93FE10 / 93F9F0, round + 1, income 959FB0; shipped campaigns have no event
states so 95ADD0 / 95BE20 loop zero times) on the InitCreate worlds of every
shipped campaign, interleaved with original battle resolutions 954130.

Perturbations as in native_resolve.py, plus: engine-side army owner changes
(army vtable+160 seam field, followed by the original recount 954CF0) to
strip kingdoms of mobile armies, capitals handed to enemies (955620), armies
moved into empty enemy or unowned territories (auto-capture), RNG / game-over
writes. Seams: see native_resolve.py.
"""
import json, hashlib, random, struct
from pathlib import Path
from mg_machine import P, V, VISITED, HERE
from mg_campaign import DATA
from mg_world import native_wstring
from native_resolve import Harness, PCS, TRACE, end_on_null_order, perturb, setup_battle, world_record

KINDS = ['Unknown', 'AddTech', 'AddActor', 'RemoveTech', 'RemoveActor', 'ChangeTerritory']
# EffectKind field flags (967A40): target 0xf, source 0xf0, tech 0xf00, actor 0xf000, territory 0xf0000
KIND_FLAGS = {'Unknown': 0, 'AddTech': 0x101, 'AddActor': 0x11001, 'RemoveTech': 0x101,
              'RemoveActor': 0x1, 'ChangeTerritory': 0x10011}


def f32bits(x):
    return struct.unpack('<I', struct.pack('<f', x))[0]


def add_events(h, rng, crash=False):
    """Synthetic campaign events laid out as the loader 968000 / 967C10 builds
    them (event: +0C round, +10 round_probability f32, +14 message, +18
    kingdom_turn data, +1C/+20 effects; effect 0x20 bytes: +04 kind, +08
    source, +0C target, +10 technology, +14 territory definition, +18 actor
    data, +1C message), only the fields each kind's flags load; the event
    states are built by the original constructor 95AD60 (campaign, event) and
    appended to MGState +0x54 in event order as InitCreate does (93A215).
    Returns the same events in the MetaGameData JSON model."""
    m, w, st = h.m, h.w, h.st
    kd = h.ctx['kdatas']
    # owned technologies to remove / skip
    names = sorted(h.ctx['techs'])
    for k in range(len(h.kingdoms)):
        if names and rng.random() < 0.5:
            h.set_technologies(k, rng.sample(names, min(len(names), rng.randint(1, 4))))
    defs = h.ctx['defs']
    n = len(kd)
    techs = sorted(h.ctx['techs'])
    things = sorted(h.ctx['things'])
    rnd = m.r32(st + 0x14)
    out = []
    states = []
    for e in range(rng.randint(1, 4)):
        roll = rng.random()
        rnd_e = -1 if roll < 0.55 else rnd + rng.choice([0, 1, 1, 2, 3])
        prob = rng.choice([0.0, 0.25, 0.5, 0.75, 1.0, rng.random(), 0.999999])
        prob_bits = f32bits(prob)
        turn = None if rng.random() < 0.5 else rng.randrange(n)
        message = '#evt_msg' if rng.random() < 0.7 else ''
        effects = []
        objs = []
        for _ in range(rng.randint(1, 4)):
            kind = rng.choices(KINDS, [0.05, 0.2, 0.25, 0.15, 0.15, 0.2])[0]
            flags = KIND_FLAGS[kind]
            eff = {'kind': kind, 'message': '#eff_msg' if rng.random() < 0.7 else '',
                   'kingdom_target': None, 'kingdom_source': None, 'territory': None,
                   'actor_ids': None, 'tech_ids': None}
            if flags & 0xf and rng.random() < 0.5:
                eff['kingdom_target'] = rng.randrange(n)
            if flags & 0xf0 and rng.random() < 0.5:
                eff['kingdom_source'] = rng.randrange(n)
            if flags & 0xf00 and techs and rng.random() < 0.5:
                eff['tech_ids'] = rng.choice(techs)
            if flags & 0xf000 and rng.random() < 0.5:
                eff['actor_ids'] = rng.choice(things)
            # 95AED0 never returns an effect territory (native bug): add_actor
            # with one always fails, so keep it rare there
            if flags & 0xf0000 and rng.random() < (0.2 if kind == 'AddActor' else 0.5):
                # often a territory the target kingdom holds
                k = eff['kingdom_target'] if eff['kingdom_target'] is not None else rng.randrange(n)
                owned = [t for t in range(len(h.territories)) if h.tmeta(t)[0] == k]
                t = rng.choice(owned) if owned and rng.random() < 0.6 else rng.randrange(len(h.territories))
                eff['territory'] = h.ctx['defs'].index(m.r32(h.territories[t] + 0xc))
            if crash and _ == 0:
                # proof of the native crash: change_territory, no source, with a message
                kind = 'ChangeTerritory'
                eff.update(kind=kind, message='#eff_msg', kingdom_source=None, territory=None, kingdom_target=None,
                           actor_ids=None, tech_ids=None)
            elif kind == 'ChangeTerritory' and eff['kingdom_source'] is None and rng.random() < 0.9:
                # its message block calls 95A8D0(NULL source): a native crash
                eff['message'] = ''
            o = m.alloc(0x20)
            m.w32(o, 0xacf024)
            m.w32(o + 4, KINDS.index(kind))
            m.w32(o + 8, 0 if eff['kingdom_source'] is None else kd[eff['kingdom_source']])
            m.w32(o + 0xc, 0 if eff['kingdom_target'] is None else kd[eff['kingdom_target']])
            m.w32(o + 0x10, 0 if eff['tech_ids'] is None else h.ctx['techs'][eff['tech_ids']])
            m.w32(o + 0x14, 0 if eff['territory'] is None else defs[eff['territory']])
            m.w32(o + 0x18, 0 if eff['actor_ids'] is None else h.ctx['things'][eff['actor_ids']])
            m.w32(o + 0x1c, native_wstring(m, eff['message']))
            effects.append(eff)
            objs.append(o)
        ev = m.alloc(0x30)
        m.w32(ev, 0)
        ids = f'synthetic_event_{e}'
        m.w32(ev + 4, native_wstring(m, ids))
        m.w32(ev + 8, native_wstring(m, ''))
        m.w32(ev + 0xc, rnd_e & 0xffffffff)
        m.w32(ev + 0x10, prob_bits)
        m.w32(ev + 0x14, native_wstring(m, message))
        m.w32(ev + 0x18, 0 if turn is None else kd[turn])
        w.vector(ev + 0x1c, objs)
        es = m.alloc(0x14)
        m.call(0x95ad60, args=(h.campaign, ev), ecx=es)
        states.append(es)
        h.event_objs.append(ev)
        h.effect_objs.append(objs)
        out.append({'ids': ids, 'name': '', 'round': rnd_e, 'round_probability_bits': prob_bits,
                    'message': message, 'kingdom_turn': turn, 'effects': effects})
    w.vector(h.campaign + 0x50, h.event_objs)
    w.vector(st + 0x54, states)
    h.events_json = out
    return out


def event_perturb(h, rng):
    """Field writes on the event states (+0C active, +10 kingdom data) and the
    round counter, so that fires happen often."""
    n = len(h.kingdoms)
    states = h.w.read_vector(h.st + 0x54)
    if not states:
        return
    if rng.random() < 0.35:
        i = rng.randrange(len(states))
        cur = h.m.r32(h.st + 0x10)
        nxt = (h.kindex[cur] + 1) % n if cur else 0
        k = nxt if rng.random() < 0.7 else (None if rng.random() < 0.2 else rng.randrange(n))
        active = 1 if rng.random() < 0.85 else 0
        kdp = 0 if k is None else h.m.r32(h.kingdoms[k] + 8)
        h.m.u.mem_write(states[i] + 0xc, bytes([active]))
        h.m.w32(states[i] + 0x10, kdp)
        h.op('event_state', i, active, None if kdp == 0 else h.ctx['kdatas'].index(kdp))
    if rng.random() < 0.1:
        r = h.m.r32(h.st + 0x14) + rng.choice([-1, 0, 1])
        h.m.w32(h.st + 0x14, r)
        h.op('round', r)


def zero_strength_targets(h):
    """Territories without defenders of their owner (or unowned)."""
    out = []
    for t in range(len(h.territories)):
        owner, refs, count = h.tmeta(t)
        if owner is None or all(h.enemy[owner][h.owner(a)] for a in refs):
            out.append(t)
    return out


def turn_perturb(h, rng):
    n = len(h.kingdoms)
    live = h.live()
    # armies walking into empty / unowned territories -> auto-capture at the next turn start
    targets = zero_strength_targets(h)
    for _ in range(rng.choice([0, 1, 1, 2, 3])):
        if not targets:
            break
        a = rng.choice(live)
        h.move(a, rng.choice(targets))
    # a kingdom loses its capital (and maybe every mobile army)
    if rng.random() < 0.2:
        k = rng.randrange(n)
        cap = h.m.r32(h.kingdoms[k] + 0x10)
        enemies = [x for x in range(n) if h.enemy[k][x]]
        if cap and enemies:
            h.set_territory_owner(h.tindex[cap], rng.choice(enemies))
        if rng.random() < 0.6:
            others = [x for x in range(n) if x != k]
            for a in h.live():
                if h.owner(a) == k and h.movement(a) != 0 and rng.random() < 0.95:
                    h.set_army_owner(a, rng.choice(others))
    if rng.random() < 0.2:
        h.set_current(rng.randrange(n))
    if rng.random() < 0.15:
        k = rng.randrange(n)
        h.set_resources(k, rng.choice([0, 7, 100, 0x7ffffff0, -50]))


def run_world(ci, pc, rng, relation_mode, n_ops, events=False):
    crash = events == 'crash'
    h = Harness(ci, PCS[pc], rng, relation_mode)
    n = len(h.kingdoms)
    stats = {}
    if events:
        add_events(h, rng, crash)
    for step in range(n_ops):
        if rng.random() < 0.5:
            perturb(h, rng)
        turn_perturb(h, rng)
        if events:
            event_perturb(h, rng)
        kind = rng.choices(['end_turn', 'advance', 'advance_playable', 'resolve'], [0.4, 0.25, 0.2, 0.15])[0]
        if kind == 'end_turn':
            cur = h.m.r32(h.st + 0x10)
            k = h.kindex[cur] if (cur and rng.random() < 0.85) else rng.randrange(n)
            case = h.end_turn(k)
        elif kind == 'advance':
            case = h.advance()
        elif kind == 'advance_playable':
            case = h.advance_playable()
        else:
            t, ak = setup_battle(h, rng, ci)
            h.set_behaviours(rng.randrange(8), rng.randrange(8))
            owner = h.tmeta(t)[0]
            atts = h.attackers(t)
            att = h.kingdom_of_army(atts[0]) if atts else ak
            won = 1
            if rng.random() < 0.4 and all(len(h.path(a)) > 1 for a in atts):
                won = 0
            case = h.resolve(att, owner, t, won)
        stats[case['op'][0]] = stats.get(case['op'][0], 0) + 1
        if h.crashed:
            # the original would crash inside a skipped message block: the
            # Rust port must report Unsupported; the world ends here
            case['unsupported'] = h.crashed
            stats['unsupported'] = stats.get('unsupported', 0) + 1
            break
        if end_on_null_order(h, case):
            stats['null_order'] = stats.get('null_order', 0) + 1
            break
        for entry in case['log']:
            if entry[0] in ('messages', 'kill', 'set_owner', 'game_over', 'victory', 'create_army',
                            'event_message', 'effect_message'):
                key = 'auto_' + entry[0] if kind != 'resolve' else entry[0]
                stats[key] = stats.get(key, 0) + 1
    return h, stats


def main(small=False):
    rng = random.Random(0x938c90)
    plan = []
    for ci in range(len(DATA['campaigns'])):
        for v in range(4):
            plan.append((ci, 'double53', 'random' if v % 2 else 'teams', 40))
    for ci in range(len(DATA['campaigns'])):
        plan.append((ci, 'single24' if ci % 2 else 'extended64', 'random', 20))
    plan = [p + (False,) for p in plan]
    # synthetic campaign events (no shipped campaign has any)
    for ci in range(len(DATA['campaigns'])):
        for v in range(4):
            plan.append((ci, 'double53', 'random' if v % 2 else 'teams', 40, True))
        plan.append((ci, 'single24', 'teams', 30, True))
        plan.append((ci, 'extended64', 'random', 30, True))
    # worlds that end in the native crash of 95B9B0's message block
    for ci in (0, 3):
        plan.append((ci, 'double53', 'teams', 40, 'crash'))
    if small:
        plan = [(0, 'double53', 'teams', 15, True), (2, 'single24', 'random', 15, True)]
    worlds = []
    totals = {}
    for ci, pc, mode, n_ops, events in plan:
        h, stats = run_world(ci, pc, rng, mode, n_ops, events)
        for k, v in stats.items():
            totals[k] = totals.get(k, 0) + v
        worlds.append(world_record(h, pc, mode))
    report = {
        'schema': 1, 'scope': __doc__,
        'original_sha256': hashlib.sha256(P.b).hexdigest(), 'v14_sha256': hashlib.sha256(V.b).hexdigest(),
        'producer_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        'harness_sha256': hashlib.sha256((HERE / 'native_resolve.py').read_bytes()).hexdigest(),
        'data_sha256': hashlib.sha256((HERE / 'metagame-data.json').read_bytes()).hexdigest(),
        'worlds': worlds,
        'branch_coverage': dict(sorted(TRACE.items())),
        'instructions': [{'va': hex(a), 'bytes': b} for a, b in sorted(VISITED.items())],
    }
    out = HERE / 'native-turn.json'
    out.write_text(json.dumps(report, separators=(',', ':')) + '\n')
    print(json.dumps({'worlds': len(worlds), 'cases': totals, 'visited': len(VISITED), 'trace': dict(sorted(TRACE.items())),
                      'sha256': hashlib.sha256(out.read_bytes()).hexdigest()}))


if __name__ == '__main__':
    import sys
    main(small='small' in sys.argv[1:])
