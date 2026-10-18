"""Native metagame world for one shipped campaign, built by the original
KKC_MGState::InitCreate (939B90) on objects laid out from metagame-data.json.
Shared by the phase8 oracles (see native_init.py for the seam list).
"""
import json
from mg_machine import Machine, HERE
from mg_world import NativeWorld, install_crt, install_static_tables, install_wcsicmp, native_wstring, read_wstring, VT_ARMY
from unicorn.x86_const import UC_X86_REG_ECX

DATA = json.loads((HERE / 'metagame-data.json').read_text())


def build(m, w, campaign_index):
    c = DATA['campaigns'][campaign_index]
    mp = DATA['maps'][c['map']]
    # map territory definitions
    defs = []
    for t in mp['territories']:
        d = m.alloc(0x80)
        m.w32(d + 4, native_wstring(m, t['ids']))
        m.w32(d + 0xc, {'Unknown': 0, 'Normal': 1, 'Neutral': 2, 'Background': 3, 'Alias': 4}[t['kind']])
        m.w32(d + 0x10, t['production'])
        for i, v in enumerate(t['location']):
            m.w32(d + 0x14 + 4 * i, v)
        m.w32(d + 0x3c, m.alloc(0x200))           # territory object (engine data, opaque here)
        m.w32(d + 0x48, t['actor_spacing_distance_bits'])
        m.w32(d + 0x4c, {'Simple': 0, 'Circle': 1, 'Square': 2}[t['actor_spacing']])
        slots = m.alloc(12 * len(t['actor_slots']))
        for i, sl in enumerate(t['actor_slots']):
            for j in range(3):
                m.w32(slots + 12 * i + 4 * j, sl[j])
        m.w32(d + 0x5c, slots)
        m.w32(d + 0x60, len(t['actor_slots']))
        m.w32(d + 0x64, len(t['actor_slots']))
        defs.append(d)
    # links (962530 layout: +0 vtable ACDE44, +4 owner def, +8 target def,
    # +0C intermediate points, +18 rmc attacker string) and alias partners (+44)
    for t, d in zip(mp['territories'], defs):
        links = []
        for l in t['links']:
            lo = m.alloc(0x20)
            m.w32(lo, 0xacde44)
            m.w32(lo + 4, d)
            m.w32(lo + 8, defs[l['target']])
            w.vector(lo + 0xc, [], 1)
            m.w32(lo + 0x18, native_wstring(m, l['rmc_attacker_ids']))
            links.append(lo)
        w.vector(d + 0x68, links, 5)
        m.w32(d + 0x44, 0 if t['alias_partner'] is None else defs[t['alias_partner']])
    mapo = m.alloc(0x80)
    w.vector(mapo + 0x28, defs)
    table = m.alloc(0x40)
    w.vector(table + 0x18, [])
    # actor (thing) data objects, one per army IDS
    # division data (941240 layout: +10 name string, +34 damage, +38 prereqs)
    divisions = [m.alloc(0x60) for _ in DATA['divisions']]
    for dv, dd in zip(divisions, DATA['divisions']):
        m.w32(dv + 0x10, native_wstring(m, dd['ids']))
        m.w32(dv + 0x34, dd['damage_bits'])
        w.vector(dv + 0x38, [divisions[i] for i in dd['prerequisites']])
    techs = {}
    def tech(ids):
        if ids not in techs:
            techs[ids] = m.alloc(0x40)
            m.w32(techs[ids] + 0x10, native_wstring(m, ids))
        return techs[ids]
    # army Things: +0x5B8 MetaGameComponent data (93B250 layout)
    things = {}
    def thing(ids):
        if ids not in things:
            th = m.alloc(0x600)
            a = next(x for x in DATA['armies'] if x['ids'] == ids)
            g = m.alloc(0x60)
            m.w32(g + 4, a['movement'])
            w.vector(g + 8, [divisions[i] for i in a['divisions']], 5)
            m.w32(g + 0x14, a['damage_bits'])
            m.w32(g + 0x18, a['damage_with_divisions_bits'])
            m.u.mem_write(g + 0x1c, bytes([int(a['capturable']), int(a['upgradable'])]))
            m.w32(g + 0x4c, a['division_matching_preference'])
            m.w32(th + 0x5b8, g)
            things[ids] = th
        return things[ids]
    def shop_entry(data, cost):
        e = m.alloc(0x10)
        m.w32(e, 0xacdde4)
        m.w32(e + 4, cost)
        m.w32(e + 8, data)
        return e
    kdatas = []
    for k in c['kingdoms']:
        kd = m.alloc(0x80)
        m.w32(kd + 4, native_wstring(m, k['ids']))
        m.w32(kd + 0x10, defs[k['capital']])
        m.w32(kd + 0x24, k['starting_resources'])
        shop = m.alloc(0x40)                      # 962290 layout, lists already in qsort order
        sh = k['shop']
        rows = lambda key, f: [shop_entry(0 if r['ids'] is None else f(r['ids']), r['cost']) for r in sh[key]]
        w.vector(shop + 0x24, rows('technologies', tech), 20)
        w.vector(shop + 0xc, rows('actors', thing), 10)
        w.vector(shop + 0x18, rows('divisions', lambda i: divisions[next(j for j, x in enumerate(DATA['divisions']) if x['ids'] == i)]), 10)
        m.w32(kd + 0x60, shop)
        objective = m.alloc(0x10)                 # 968250: objective object, +4 capital_captures
        m.w32(objective + 4, k['capital_captures'])
        m.w32(kd + 0x64, objective)
        kdatas.append(kd)
    for k, kd in zip(c['kingdoms'], kdatas):
        recs = []
        for r in k['territories']:
            rec = m.alloc(0x20)
            m.w32(rec, 0 if r['liberates_to'] is None else kdatas[r['liberates_to']])
            m.w32(rec + 4, defs[r['territory']])
            w.vector(rec + 8, [thing(a) for a in r['actors']])
            recs.append(rec)
        w.vector(kd + 0x54, recs)
    campaign = m.alloc(0x100)
    m.w32(campaign + 8, native_wstring(m, c['ids']))
    m.w32(campaign + 0x18, mapo)
    m.w32(campaign + 0x1c, table)
    w.vector(campaign + 0x44, kdatas)
    w.vector(campaign + 0x50, [])
    # MGState built by the original new-campaign constructor 9397E0 (factory
    # 9399A0 path): round 1, RNG 0x8E4B12A5, bucket count = map territories.
    # arg2 (+0x0C, an object whose +8 string is saved) is a synthetic input.
    st = m.alloc(0x80)
    arg2 = m.alloc(0x20)
    m.w32(arg2 + 8, native_wstring(m, ''))
    m.call(0x9397e0, args=(campaign, arg2), ecx=st)
    m.w32(0xb49fa0, st)
    return c, defs, kdatas, things, st, divisions, techs


def run(campaign_index, cw=0x027f, keep=False):
    m = Machine(cw)                      # PC53 by default (net/simulation thread, lead policy)
    install_crt(m)
    install_wcsicmp(m)
    c = DATA['campaigns'][campaign_index]
    team = {}
    for ti, t in enumerate(c['teams']):
        for k in t['kingdoms']:
            team[k] = ti
    n = len(c['kingdoms'])
    rel = {'enemy_140': [[team.get(a) != team.get(b) for b in range(n)] for a in range(n)],
           'relation_138': [[team.get(a) == team.get(b) for b in range(n)] for a in range(n)]}
    w = NativeWorld(m, rel)
    install_static_tables(m)             # trig tables + actor_spacing_type table (original static init)
    c, defs, kdatas, things, st, divisions, techs = build(m, w, campaign_index)
    world = m.alloc(0x100)
    m.w32(0xb36534, world)
    actors = [w.kingdom_actor(i) for i in range(n)]
    for i, a in enumerate(actors):
        m.w32(a + 0x14, w.handle(a))
        # engine kingdom object name string (+0x18, read by 4BEDF0): its IDS (assumed)
        m.w32(a + 0x18, native_wstring(m, c['kingdoms'][i]['ids']))
    ids_to_kingdom = {k['ids']: i for i, k in enumerate(c['kingdoms'])}
    created = []
    def lookup(mm):
        key = read_wstring(mm, mm.r32(mm.arg(0)))
        mm.ret(4, actors[ids_to_kingdom[key]])
    def visual(mm):
        o = mm.alloc(0x40)
        mm.w32(o + 0x14, w.handle(o))
        mm.ret(4, o)
    def create_army(mm):
        desc = mm.arg(0)
        data, owner = mm.r32(desc + 4), mm.r32(desc + 0x28)
        a = mm.alloc(0x200)
        mm.w32(a, VT_ARMY)
        mm.w32(a + 4, data)
        mm.w32(a + 0x14, w.handle(a))
        comp = mm.alloc(0x40)
        mm.w32(comp + 4, a)
        # ASSUMPTION (engine component creation): live divisions = the type's list
        w.vector(comp + 0x14, w.read_vector(mm.r32(data + 0x5b8) + 8), 5)
        w.vector(comp + 0x24, [], 1)
        # +0x30/+0x34: waypoint list head/tail (nodes {territory, next}), empty
        mm.w32(a + 0xcc, comp)
        w.army_owner[a] = owner
        w.kind[a] = ('army', len(created))
        created.append({'data': data, 'owner': owner, 'actor': a,
                        'position_bits': [mm.r32(desc + 8 + 4 * i) for i in range(3)]})
        mm.ret(4, a)
    m.seams[0x4f48e0] = lookup
    m.seams[0x4f9c10] = visual
    m.seams[0x4fa030] = create_army
    m.seams[0x939230] = lambda mm: mm.ret(4)
    eax, _ = m.call(0x939b90, ecx=st, count=50_000_000)
    assert eax & 0xff == 1, hex(eax)
    ctx = {'m': m, 'w': w, 'st': st, 'defs': defs, 'kdatas': kdatas, 'things': things,
           'actors': actors, 'created': created, 'c': c, 'divisions': divisions, 'techs': techs}
    snap = snapshot(ctx)
    if keep:
        return snap, ctx
    return snap


def snapshot(ctx):
    m, w, st, defs, kdatas, things = ctx['m'], ctx['w'], ctx['st'], ctx['defs'], ctx['kdatas'], ctx['things']
    actors, created, c = ctx['actors'], ctx['created'], ctx['c']
    thing_ids = {v: k for k, v in things.items()}
    kingdoms = w.read_vector(st + 0x1c)
    territories = w.read_vector(st + 0x28)
    def tindex(p):
        return None if p == 0 else territories.index(p)
    def kindex(p):
        return None if p == 0 else kingdoms.index(p)
    out_k = []
    for kp in kingdoms:
        out_k.append({
            'back_pointer_is_state': m.r32(kp + 4) == st,
            'data': kdatas.index(m.r32(kp + 8)),
            'actor': actors.index(w.resolve(m.r32(kp + 0xc))),
            'capital': tindex(m.r32(kp + 0x10)),
            'territories': [tindex(t) for t in w.read_vector(kp + 0x14)],
            'technologies': len(w.read_vector(kp + 0x20)),
            'list_2c': len(w.read_vector(kp + 0x2c)),
            'resources': m.r32(kp + 0x38),
            'battles': m.r32(kp + 0x3c),
        })
    out_t = []
    for i, tp in enumerate(territories):
        rec = m.r32(tp + 8)
        record = None
        if rec:
            for ki, kd in enumerate(kdatas):
                recs = w.read_vector(kd + 0x54)
                if rec in recs:
                    record = [ki, recs.index(rec)]
        refs = []
        for r in w.read_vector(tp + 0x10):
            actor = w.resolve(m.r32(r + 4))
            refs.append({'army': w.kind[actor][1], 'slot': m.r32(r + 8)})
        out_t.append({
            'index': m.r32(tp + 4),
            'record': record,
            'definition': defs.index(m.r32(tp + 0xc)),
            'armies': refs,
            'enemy_army_count': m.r32(tp + 0x1c),
            'visual_handle_set': m.r32(tp + 0x20) != 0,
            'owner': kindex(m.r32(tp + 0x24)),
        })
    out_a = []
    for a in created:
        comp = m.r32(a['actor'] + 0xcc)
        out_a.append({
            'definition': thing_ids[a['data']],
            'owner': actors.index(a['owner']),
            'territory': tindex(m.r32(comp + 0x20)),
            'path': [tindex(t) for t in w.read_vector(comp + 0x24)],
            'position_bits': a['position_bits'],
        })
    return {
        'campaign': c['ids'],
        'current': kindex(m.r32(st + 0x10)),
        'state_byte_50': m.u.mem_read(st + 0x50, 1)[0],
        'round': m.r32(st + 0x14),
        'flags_44_45_46_50_51': [m.u.mem_read(st + o, 1)[0] for o in (0x44, 0x45, 0x46, 0x50, 0x51)],
        'last_purchased': (lambda h: None if h == 0 else w.kind.get(w.resolve(h), (None, None))[1])(m.r32(st + 0x4c)),
        'results': [{'territory': tindex(m.r32(e + 4)), 'attacker': kindex(m.r32(e + 8)),
                     'defender': kindex(m.r32(e + 0xc)), 'attacker_won': m.u.mem_read(e + 0x10, 1)[0]}
                    for e in w.read_vector(st + 0x60)],
        'rng_state': m.r32(st + 0x18),
        'kingdoms': out_k, 'territories': out_t, 'armies': out_a,
        'events': len(w.read_vector(st + 0x54)),
    }
