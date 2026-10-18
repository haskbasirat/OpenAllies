"""Original metagame command processing KKC_MGCommand::Process 954570 for
commands that pass validation 951D70: move 9543C0 / attack 954400 (9537B0:
path 93FDC0, waypoint queue 93F4B0), startbattle 954440, playbattle 950E40,
cancelbattle 950E70, purchasetech 9528B0 (95A770), purchaseactor 952A00
(955B70 at the capital, 938A30), upgradeactor 952D10 (952B90 / 93F750,
951AA0 ChangeGActor), upgradeactorall 952D90 (93F7A0), on perturbed
InitCreate worlds of every shipped campaign. endturn (6), quickbattle (5) and
take (11) are not executed here (turn/resolve modules; cheats off).

Seams in addition to mg_campaign's: message post 40BE60 (cdecl, category
recorded), localisation 4B9F40 (returns an empty string, ret 8), kingdom
display name 95A8D0 (needs the KingdomGlyphInfo UI var; empty string),
world kingdom lookup by name 4F8F00 and actor hand-over 4D5EA0 (respawn
after an army type change), army vtable
+0x24 name (empty string), actor destroy 4F6760 (recorded; marks actor+8 bit 0), army vtable+134
(owner == issuer). An upgrade that changes the army type respawns the actor
through 955DD0/955820/4FA030 (creation seam): the replacement is recorded.
"""
import json, hashlib, random
from pathlib import Path
from mg_machine import P, V, VISITED, HERE
from mg_campaign import DATA, run
from mg_world import VT_ARMY, native_wstring
from native_commands import FLAGS, HOT
from unicorn.x86_const import UC_X86_REG_ECX


def main():
    rng = random.Random(0x954570)
    rows = []
    for ci in range(len(DATA['campaigns'])):
        snap, ctx = run(ci, keep=True)
        m, w, st = ctx['m'], ctx['w'], ctx['st']
        actors, created, defs = ctx['actors'], ctx['created'], ctx['defs']
        kingdoms = w.read_vector(st + 0x1c)
        territories = w.read_vector(st + 0x28)
        def vf134(mm):
            mm.ret(4, int(w.army_owner[mm.u.reg_read(UC_X86_REG_ECX)] == mm.arg(0)))
        w.virtual(VT_ARMY, 0x134, vf134)
        def name(mm):
            out = mm.arg(0); mm.w32(out, native_wstring(mm, '')); mm.ret(4, out)
        w.virtual(VT_ARMY, 0x24, name)
        messages = []
        m.seams[0x40be60] = lambda mm: (messages.append(mm.arg(1)), mm.ret(0))
        def localize(mm):
            out = mm.arg(0); mm.w32(out, native_wstring(mm, '')); mm.ret(8, out)
        m.seams[0x4b9f40] = localize
        m.seams[0x95a8d0] = localize.__class__(lambda mm: (mm.w32(mm.arg(0), native_wstring(mm, '')), mm.ret(4, mm.arg(0)))) if False else (lambda mm: (mm.w32(mm.arg(0), native_wstring(mm, '')), mm.ret(4, mm.arg(0))))
        from mg_world import read_wstring
        ids_actor = {k['ids']: actors[i] for i, k in enumerate(ctx['c']['kingdoms'])}
        # world kingdom lookup by name 4F8F00 (thiscall world, &CString) ret 4: engine seam
        m.seams[0x4f8f00] = lambda mm: mm.ret(4, ids_actor.get(read_wstring(mm, mm.r32(mm.arg(0))), 0))
        m.seams[0x4d5ea0] = lambda mm: mm.ret(4)          # engine actor hand-over after respawn
        destroyed = []
        dead = set()
        def destroy(mm):
            a = mm.arg(0)
            destroyed.append(a); dead.add(a)
            mm.w32(a + 8, mm.r32(a + 8) | 1)          # engine marks the actor deleted
            mm.ret(8)
        m.seams[0x4f6760] = destroy
        data_objs = []
        for i in range(12):
            d = m.alloc(0x30)
            m.w32(d + 0xc, i); m.w32(d + 0x24, FLAGS[i]); m.u.mem_write(d + 0x2c, bytes([HOT[i]]))
            data_objs.append(d)
        divisions = ctx['divisions']
        tech_ids = {v: k for k, v in ctx['techs'].items()}
        thing_ids = {v: k for k, v in ctx['things'].items()}
        def army_index(actor):
            return w.kind[actor][1]
        def state_dump():
            ks = [{'resources': m.r32(kp + 0x38) - (1 << 32) * (m.r32(kp + 0x38) >> 31),
                   'technologies': [tech_ids[t] for t in w.read_vector(kp + 0x20)]} for kp in kingdoms]
            ar = []
            for a in created:
                comp = m.r32(a['actor'] + 0xcc)
                wp, node = [], m.r32(comp + 0x30)
                while node:
                    wp.append(territories.index(m.r32(node))); node = m.r32(node + 4)
                t = m.r32(comp + 0x20)
                ar.append({'definition': thing_ids[m.r32(a['actor'] + 4)],
                           'territory': None if t == 0 else territories.index(t),
                           'divisions': [divisions.index(d) for d in w.read_vector(comp + 0x14)],
                           'waypoints': wp})
            terr = [[[army_index(w.resolve(m.r32(r + 4))), m.r32(r + 8)] for r in w.read_vector(tp + 0x10)] for tp in territories]
            h = m.r32(st + 0x4c)
            return {'kingdoms': ks, 'armies': ar, 'territories': terr,
                    'flags_44_45_46': [m.u.mem_read(st + o, 1)[0] for o in (0x44, 0x45, 0x46)],
                    'last_purchased': None if h == 0 else army_index(w.resolve(h))}
        rounds = []
        for round_ in range(5):
            ops = []
            for _ in range(rng.randint(2, 10)):
                a = rng.randrange(len(created))
                comp = m.r32(created[a]['actor'] + 0xcc)
                if m.r32(comp + 0x20) == 0 or created[a]['actor'] in dead:
                    continue
                cur = territories.index(m.r32(comp + 0x20))
                links = w.read_vector(m.r32(territories[cur] + 0xc) + 0x68)
                if links:
                    target_def = m.r32(rng.choice(links) + 8)
                    t = next(i for i, tp in enumerate(territories) if m.r32(tp + 0xc) == target_def)
                    m.call(0x940100, args=(territories[t],), ecx=comp)
                    ops.append(['move', a, t])
            for k, kp in enumerate(kingdoms):
                r = rng.choice([0, 25, 40, 75, 150, 400, 1000, 1000, 1000])
                m.w32(kp + 0x38, r); m.w32(kp + 0x3c, 0)
                ops.append(['resources', k, r]); ops.append(['battles', k, 0])
            cur_k = rng.randrange(len(kingdoms))
            m.w32(st + 0x10, kingdoms[cur_k]); ops.append(['current', cur_k])
            for a in range(len(created)):
                m.w32(m.r32(created[a]['actor'] + 0xcc) + 0x10, 0)
            ops.append(['fought_all', 0])
            cmds = []
            for _ in range(40):
                idx = rng.choice([0, 0, 1, 1, 2, 3, 4, 7, 7, 8, 8, 9, 9, 9, 9, 10, 10, 10])
                issuer = actors[cur_k]
                cmd = m.alloc(0x28); m.call(0x9510f0, ecx=cmd)
                m.call(0x950cb0, args=(data_objs[idx],), ecx=cmd)
                desc = {'index': idx}
                own = [i for i, a in enumerate(created) if w.army_owner[a['actor']] == issuer
                       and a['actor'] not in dead and m.r32(m.r32(a['actor'] + 0xcc) + 0x20) != 0]
                if own:
                    a = rng.choice(own)
                    m.call(0x950eb0, args=(created[a]['actor'],), ecx=cmd); desc['army'] = a
                    here = m.r32(m.r32(created[a]['actor'] + 0xcc) + 0x20)
                    links = w.read_vector(m.r32(here + 0xc) + 0x68)
                    if links:
                        d = m.r32(rng.choice(links) + 8)
                        if rng.random() < 0.3:
                            l2 = w.read_vector(d + 0x68)
                            if l2:
                                d = m.r32(rng.choice(l2) + 8)
                        m.call(0x950cf0, args=(d,), ecx=cmd); desc['territory_def'] = defs.index(d)
                shop = m.r32(ctx['kdatas'][cur_k] + 0x60)
                if idx == 7:
                    techs = w.read_vector(shop + 0x24)
                    if techs:
                        t = m.r32(rng.choice(techs) + 8)
                        m.call(0x950d20, args=(t,), ecx=cmd); desc['tech'] = tech_ids[t]
                if idx == 8:
                    acts = w.read_vector(shop + 0xc)
                    th = m.r32(rng.choice(acts) + 8)
                    m.call(0x950cd0, args=(th,), ecx=cmd); desc['actor_def'] = thing_ids[th]
                if idx == 9:
                    divs = w.read_vector(shop + 0x18)
                    if 'army' in desc and rng.random() < 0.7:
                        have = w.read_vector(m.r32(created[desc['army']]['actor'] + 0xcc) + 0x14)
                        ok = [e for e in divs if m.r32(e + 8) not in have
                              and all(p in have for p in w.read_vector(m.r32(e + 8) + 0x38))]
                        divs = ok or divs
                    pick = rng.sample(divs, min(len(divs), rng.randint(1, 2)))
                    desc['divisions'] = []
                    for e in pick:
                        dv = m.r32(e + 8)
                        m.call(0x952570, args=(dv,), ecx=cmd); desc['divisions'].append(divisions.index(dv))
                code, _ = m.call(0x951d70, args=(issuer,), ecx=cmd, count=20_000_000)
                desc['code'] = code
                if code == 0:
                    if idx in (0, 1):
                        comp = m.r32(created[desc['army']]['actor'] + 0xcc)
                        out = m.alloc(0x10); w.vector(out, [], 4)
                        tgt = next(i for i, tp in enumerate(territories) if m.r32(tp + 0xc) == m.r32(cmd + 0x14))
                        m.call(0x93fdc0, args=(territories[tgt], out), ecx=comp, count=20_000_000)
                        desc['path'] = [territories.index(t) for t in w.read_vector(out)]
                    messages.clear(); destroyed.clear(); n_before = len(created)
                    r, _ = m.call(0x954570, args=(issuer,), ecx=cmd, count=50_000_000)
                    desc['result'] = r & 0xff
                    desc['messages'] = list(messages)
                    desc['respawned'] = [[army_index(x), army_index(created[i]['actor'])]
                                         for x in destroyed for i in range(n_before, len(created))]
                    desc['created'] = len(created) - n_before
                    desc['after'] = state_dump()
                cmds.append(desc)
            rounds.append({'ops': ops, 'commands': cmds})
        rows.append({'campaign': ci, 'rounds': rounds})
    report = {
        'schema': 1, 'scope': __doc__,
        'original_sha256': hashlib.sha256(P.b).hexdigest(), 'v14_sha256': hashlib.sha256(V.b).hexdigest(),
        'producer_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        'data_sha256': hashlib.sha256((HERE / 'metagame-data.json').read_bytes()).hexdigest(),
        'campaigns': rows,
        'instructions': [{'va': hex(a), 'bytes': b} for a, b in sorted(VISITED.items())],
    }
    out = HERE / 'native-process.json'
    out.write_text(json.dumps(report) + '\n')
    from collections import Counter
    done = Counter(c['index'] for r in rows for rd in r['rounds'] for c in rd['commands'] if c['code'] == 0)
    print(json.dumps({'processed': dict(sorted(done.items())), 'visited': len(VISITED),
                      'sha256': hashlib.sha256(out.read_bytes()).hexdigest()}))


if __name__ == '__main__':
    main()
