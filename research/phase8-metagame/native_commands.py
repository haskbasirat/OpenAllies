"""Original metagame command validation KKC_MGCommand 951D70 (with 950D30,
954B30/954C00, the per-command validators 9512B0 move, 951480 attack, 950DB0
startbattle, 950DF0 quickbattle, 951710 endturn, 951770 purchasetech,
951800 purchaseactor, 951890 upgradeactor, 9519D0 upgradeactorall, 950EF0 take,
pending battle 93A320, path search 93FDC0, upgrade list 93F7A0/93F970)
on perturbed InitCreate worlds of every shipped campaign.

Commands are built with the original constructor 9510F0 and setters 950CB0,
950EB0, 950CD0, 950CF0, 950D20, 952570; command data objects carry the
built-in index/flags/hot values of 93BAB0 (+0C, +24, +2C). State perturbations
use original code (940100 relocation, 955620 owner change) or plain field
writes (resources K+38, battles K+3C, current kingdom S+10, S+44, army
fought C+10), all recorded as operations for the Rust replay.
Seams: army vtable+134 (owner == issuer, or a recorded forced false), the cheat gate 40FCA0
(returns 0, cheats off), CRT heap, the InitCreate seams of mg_campaign.
For move/attack the 93FDC0 result (path found or not) is recorded per case.
"""
import json, hashlib, random
from pathlib import Path
from mg_machine import P, V, VISITED, HERE
from mg_campaign import DATA, run, snapshot
from mg_world import VT_ARMY

FLAGS = [0x1000840, 0x1000820, 0, 0, 0, 0, 0x1000001, 0x1010000, 0x1001000, 0x1100800, 0x1000800, 0x20]
HOT = [1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]


def main():
    rng = random.Random(0x951d70)
    rows = []
    for ci in range(len(DATA['campaigns'])):
        snap, ctx = run(ci, keep=True)
        m, w, st = ctx['m'], ctx['w'], ctx['st']
        actors, created, defs = ctx['actors'], ctx['created'], ctx['defs']
        kingdoms = w.read_vector(st + 0x1c)
        territories = w.read_vector(st + 0x28)
        c = ctx['c']
        # army vtable+134: assumed "controlled by" = owner == issuer
        from unicorn.x86_const import UC_X86_REG_ECX
        override = {'value': None, 'used': False}
        def vf134(mm):
            this = mm.u.reg_read(UC_X86_REG_ECX)
            override['used'] = True
            v = override['value'] if override['value'] is not None else int(w.army_owner[this] == mm.arg(0))
            mm.ret(4, v)
        w.virtual(VT_ARMY, 0x134, vf134)
        m.seams[0x40fca0] = lambda mm: mm.ret(8, 0)       # cheat gate (thiscall B39690, 2 args): off
        data_objs = []
        for i in range(12):
            d = m.alloc(0x30)
            m.w32(d + 0xc, i)
            m.w32(d + 0x24, FLAGS[i])
            m.u.mem_write(d + 0x2c, bytes([HOT[i]]))
            data_objs.append(d)
        all_divs = ctx['divisions']
        all_techs = list(ctx['techs'].values())
        tech_ids = {v: k for k, v in ctx['techs'].items()}
        thing_ids = {v: k for k, v in ctx['things'].items()}
        rounds = []
        for round_ in range(6):
            ops = []
            cases = []
            # perturb
            for _ in range(rng.randint(2, 12)):
                a = rng.randrange(len(created))
                comp = m.r32(created[a]['actor'] + 0xcc)
                cur = territories.index(m.r32(comp + 0x20))
                tdef = m.r32(territories[cur] + 0xc)
                links = w.read_vector(tdef + 0x68)
                if links and rng.random() < 0.7:
                    target_def = m.r32(rng.choice(links) + 8)
                    t = next(i for i, tp in enumerate(territories) if m.r32(tp + 0xc) == target_def)
                else:
                    t = rng.randrange(len(territories))
                m.call(0x940100, args=(territories[t],), ecx=comp)
                ops.append(['move', a, t])
            caps = [territories.index(m.r32(kp + 0x10)) for kp in kingdoms]
            for _ in range(rng.randint(0, 6)):
                t = rng.choice(caps) if rng.random() < 0.4 else rng.randrange(len(territories))
                k = rng.randrange(len(kingdoms))
                m.call(0x955620, args=(kingdoms[k],), ecx=territories[t])
                ops.append(['owner', t, k])
            for k, kp in enumerate(kingdoms):
                r = rng.choice([0, 10, 25, 40, 75, 100, 150, 1000, rng.randint(0, 300)])
                b = rng.choice([0, 1, 1])
                m.w32(kp + 0x38, r); m.w32(kp + 0x3c, b)
                ops.append(['resources', k, r]); ops.append(['battles', k, b])
            cur_k = rng.randrange(len(kingdoms))
            m.w32(st + 0x10, kingdoms[cur_k]); ops.append(['current', cur_k])
            flag44 = rng.choice([0, 0, 1])
            m.u.mem_write(st + 0x44, bytes([flag44])); ops.append(['battle_started', flag44])
            for _ in range(rng.randint(0, 5)):
                a = rng.randrange(len(created)); f = rng.choice([0, 1, 2])
                m.w32(m.r32(created[a]['actor'] + 0xcc) + 0x10, f); ops.append(['fought', a, f])
            # commands
            for _ in range(60):
                idx = rng.choice(list(range(12)) + [1, 1, 1, 8, 8, 0]) if rng.random() < 0.95 else 12
                issuer_k = cur_k if rng.random() < 0.8 else rng.randrange(len(kingdoms))
                issuer = actors[issuer_k]
                cmd = m.alloc(0x28)
                m.call(0x9510f0, ecx=cmd)
                desc = {'index': idx, 'issuer': issuer_k}
                if idx < 12 or rng.random() < 0.5:
                    if idx < 12:
                        m.call(0x950cb0, args=(data_objs[idx],), ecx=cmd)
                    else:
                        d = m.alloc(0x30); m.w32(d + 0xc, 12); m.call(0x950cb0, args=(d,), ecx=cmd)
                    desc['data'] = True
                # army
                if rng.random() < 0.85:
                    own = [i for i, a in enumerate(created) if w.army_owner[a['actor']] == issuer]
                    a = rng.choice(own) if own and rng.random() < 0.8 else rng.randrange(len(created))
                    m.call(0x950eb0, args=(created[a]['actor'],), ecx=cmd)
                    desc['army'] = a
                    comp = m.r32(created[a]['actor'] + 0xcc)
                    here = m.r32(comp + 0x20)
                else:
                    here = territories[rng.randrange(len(territories))]
                # territory (map def)
                if rng.random() < 0.85:
                    links = w.read_vector(m.r32(here + 0xc) + 0x68)
                    if links and rng.random() < 0.6:
                        d = m.r32(rng.choice(links) + 8)
                    elif rng.random() < 0.5:
                        d = m.r32(m.r32(rng.choice(created)['actor'] + 0xcc) + 0x20)
                        d = m.r32(d + 0xc)
                    else:
                        d = rng.choice(defs)
                    m.call(0x950cf0, args=(d,), ecx=cmd)
                    desc['territory_def'] = defs.index(d)
                # tech
                if rng.random() < 0.5 and all_techs:
                    t = rng.choice(all_techs)
                    m.call(0x950d20, args=(t,), ecx=cmd)
                    desc['tech'] = tech_ids[t]
                # actor definition
                if rng.random() < 0.5:
                    th = rng.choice(list(ctx['things'].values()))
                    m.call(0x950cd0, args=(th,), ecx=cmd)
                    desc['actor_def'] = thing_ids[th]
                # divisions
                if rng.random() < 0.4:
                    divs = rng.sample(range(len(all_divs)), rng.randint(0, 3))
                    for dv in divs:
                        m.call(0x952570, args=(all_divs[dv],), ecx=cmd)
                    desc['divisions'] = divs
                override['value'] = (0 if rng.random() < 0.5 else None) if rng.random() < 0.1 else None
                override['used'] = False
                code, _ = m.call(0x951d70, args=(issuer,), ecx=cmd, count=20_000_000)
                if override['used']:
                    desc['controllable_override'] = override['value']
                desc['flags_word'] = m.r32(cmd + 8)
                desc['code'] = code
                if idx in (0, 1) and 'army' in desc and 'territory_def' in desc:
                    comp = m.r32(created[desc['army']]['actor'] + 0xcc)
                    out = m.alloc(0x10); w.vector(out, [], 4)
                    target = m.r32(cmd + 0x14)
                    tgt = next(i for i, tp in enumerate(territories) if m.r32(tp + 0xc) == target)
                    ok, _ = m.call(0x93fdc0, args=(territories[tgt], out), ecx=comp, count=20_000_000)
                    desc['path_found'] = ok & 0xff
                    desc['path'] = [territories.index(t) for t in w.read_vector(out)]
                cases.append(desc)
            rounds.append({'ops': ops, 'commands': cases})
        rows.append({'campaign': ci, 'rounds': rounds})
    report = {
        'schema': 1, 'scope': __doc__,
        'original_sha256': hashlib.sha256(P.b).hexdigest(), 'v14_sha256': hashlib.sha256(V.b).hexdigest(),
        'producer_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        'data_sha256': hashlib.sha256((HERE / 'metagame-data.json').read_bytes()).hexdigest(),
        'campaigns': rows,
        'instructions': [{'va': hex(a), 'bytes': b} for a, b in sorted(VISITED.items())],
    }
    out = HERE / 'native-commands.json'
    out.write_text(json.dumps(report, indent=1) + '\n')
    from collections import Counter
    codes = Counter((cs['index'], cs['code']) for r in rows for rd in r['rounds'] for cs in rd['commands'])
    print(json.dumps({'cases': sum(len(rd['commands']) for r in rows for rd in r['rounds']), 'visited': len(VISITED),
                      'codes': sorted([f'{k[0]}:{k[1]}={v}' for k, v in codes.items()]),
                      'sha256': hashlib.sha256(out.read_bytes()).hexdigest()}))


if __name__ == '__main__':
    main()
