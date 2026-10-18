"""Native oracle for SummonEffect Apply 4CCA70 and its creators (round 5):
SummonObject 4CC2C0, SummonActor 4CC470 and SummonTransportedActor 4CB110,
with 409230 (normalize), 4C89A0 (summoned GTID list, 4D0900) and 4CA7B0
(FX event 4 over the actor or its organization members, 4364C0) executing.

Recorded seams (in call order, with their arguments):
4C8330 target, 4F02B0/444A90 random direction (world RNG + [B39AF8] table),
554B30 open-position search, 404F60 log, 4CB5B0 failure message,
4F9C10 / 4FA030 / 4FA150 creation (descriptor words), 7A0C20 SAI notify,
4CEF10 property, 4C8500 ability FX, 4CA330/4D9B60/4CA2D0 commands,
552640 map clamp, 4C2290 organization data, 58BA90 transport payload,
4D9E20 command constructor. GTIDs resolve through the real registry layout.
"""
from pathlib import Path
import random
import struct
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import harness as h  # noqa: E402
import native_special_ops_effects as nse  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX  # noqa: E402

f32 = h.f32
ff = lambda b: struct.unpack('<f', struct.pack('<I', b))[0]


def words(m, address, count):
    return list(struct.unpack(f'<{count}I', bytes(m.u.mem_read(address, 4 * count))))


class Oracle(nse.Oracle):
    def __init__(self):
        super().__init__()
        self.names = {}

    def label(self, value):
        return self.names.get(value, value) if value else 0


def case(m, rng, index):
    events = []
    kind = rng.choice(('object', 'actor', 'organization', 'transported'))
    flags54 = rng.choice((0, 0x20))          # target_location
    has_source = rng.random() < 0.6
    zero_vector = rng.random() < 0.2
    campaign = rng.random() < 0.25
    ethereal = rng.random() < 0.2
    open_ok = rng.random() < 0.85
    create_ok = rng.random() < 0.85
    controllable = rng.random() < 0.5
    has_property = rng.random() < 0.4
    has_target = rng.random() < 0.6
    target_attacks = rng.random() < 0.5
    body_attacker = rng.choice(('none', 'live', 'dead'))
    has_allegiance = rng.random() < 0.8
    sai = rng.choice((None, 0, 1))
    members = rng.choice(([], ['m0'], ['m0', None, 'm2']))
    W, H = rng.choice(((512.0, 512.0), (1024.0, 768.0), (300.0, 900.0)))

    m.u.mem_write(m.storage + 0x128, bytes([int(campaign)]))
    world = m.world_obj
    mapobj = m.alloc(0x40)
    m.w(world + 0x30, mapobj)
    m.w(mapobj + 0xC, f32(W))
    m.w(mapobj + 0x10, f32(H))
    m.w(0xB36528, 0)
    if sai is not None:
        s = m.alloc(0x40)
        m.wb(s + 0x20, sai)
        m.w(0xB36528, s)

    def thing(name, actor, organization=False, transport=False):
        t = m.alloc(0x400)
        m.names[t] = name
        m.w(t + 0x10, m.r(m.cstring(name)))
        m.w(t, m.vtable({0x38: lambda mm, a=actor: mm.ret(0, int(a))}))
        m.w(t + 0x1C, 0x40000 if ethereal else 0)
        m.w(t + 0x174, (1 << 7) if organization else 0)
        if transport:
            tr = m.alloc(0x500)
            m.names[tr] = 'transport_def'
            m.w(tr + 0x414, f32(40.0))
            m.w(t + 0x184, tr)
        m.w(t + 0x2F0, 0x2F0F0)
        return t
    obj = thing('object', kind != 'object', kind == 'organization', kind == 'transported')
    camp = thing('campaign', kind != 'object', kind == 'organization', kind == 'transported')
    prop = m.alloc(0x20) if has_property else 0
    m.names[prop] = 'property'
    effect = m.alloc(0x40)
    m.w(effect, 0xA25DA8)
    m.w(effect + 0x14, obj)
    m.w(effect + 0x18, camp)
    m.w(effect + 0x24, prop)
    m.wb(effect + 0x28, int(controllable))
    ability = m.alloc(0x40)
    m.names[ability] = 'ability'
    adef = m.alloc(0x80)
    m.w(adef + 0x54, flags54)
    m.w(ability + 4, adef)
    ctx = m.alloc(0x30)
    m.names[ctx] = 'context'
    src = 0
    sp = [f32(rng.uniform(0, W)), f32(rng.uniform(0, H))]
    tp = list(sp) if zero_vector else [f32(rng.uniform(0, W)), f32(rng.uniform(0, H))]
    facing = [f32(rng.uniform(-1, 1)), f32(rng.uniform(-1, 1))]
    extent = f32(rng.choice((0.0, 2.5, 7.25)))
    src_pos = [f32(rng.uniform(0, W)), f32(rng.uniform(0, H))]
    if has_source:
        src = m.handle()
        m.names[src] = 'source'
        sdef = m.alloc(0x80)
        m.w(sdef + 0x54, extent)
        m.w(src + 4, sdef)
        m.w(src + 0x20, src_pos[0])
        m.w(src + 0x24, src_pos[1])
        m.w(src + 0x2C, facing[0])
        m.w(src + 0x30, facing[1])
    m.w(ctx + 8, src)
    allegiance = m.handle() if has_allegiance else 0
    m.names[allegiance] = 'allegiance'
    m.w(ctx + 0xC, allegiance)
    m.w(ctx + 0x10, sp[0])
    m.w(ctx + 0x14, sp[1])
    m.w(ctx + 0x20, tp[0])
    m.w(ctx + 0x24, tp[1])
    saved_target = 0xCAFE
    m.w(ctx + 0x1C, saved_target)
    tgt = 0
    attacker = 0
    if has_target:
        tgt = m.handle()
        m.names[tgt] = 'target'
        m.w(tgt, m.vtable({0x16C: lambda mm, r=target_attacks, ev=events: (
            ev.append({'attacks_query': mm.label(mm.arg(0))}), mm.ret(4, int(r)))}))
        if body_attacker != 'none':
            body = m.alloc(0x80)
            m.w(tgt + 0x60, body)
            if body_attacker == 'live':
                attacker = m.handle()
                m.names[attacker] = 'attacker'
                m.w(body + 0x28, m.r(attacker + 0x14))
            else:
                m.w(body + 0x28, m.gtid(0))
    random_dir = [f32(0.6), f32(-0.8)]
    open_shift = [f32(1.25), f32(-0.5)]
    created = m.handle(0x200) if create_ok else 0
    m.names[created] = 'created'
    if created:
        m.w(created + 8, 0x100)
        if kind == 'organization':
            m.w(created, m.vtable({0xB4: lambda mm: mm.ret(0, 1)}))
            org = m.alloc(0x40)
            arr = m.alloc(4 * max(1, len(members)))
            for i, name in enumerate(members):
                if name:
                    mem = m.alloc(0x100)
                    m.names[mem] = name
                    comp = m.alloc(0x40)
                    m.w(mem + 0x80, comp)
                    m.w(arr + 4 * i, mem)
            m.w(org + 0x28, arr)
            m.w(org + 0x2C, len(members))
            m.w(created + 0x7C, org)
        else:
            m.w(created, m.vtable({0xB4: lambda mm: mm.ret(0, 0)}))
    hooks = {
        0x4C8330: lambda mm: mm.ret(0, tgt),
        0x4F02B0: lambda mm: (events.append({'random_rng': True}), mm.ret(0, 0x1234)),
        0x444A90: lambda mm: (mm.w(mm.arg(0), random_dir[0]), mm.w(mm.arg(0) + 4, random_dir[1]),
                              events.append({'random_direction': True}), mm.ret(4)),
        0x554B30: lambda mm: (events.append({'open': words(mm, mm.arg(0), 2), 'dir': words(mm, mm.arg(1), 2),
                                             'object': mm.label(mm.arg(2)), 'radius': mm.arg(3),
                                             'a4': mm.arg(4), 'a5': mm.arg(5)}),
                              open_ok and (mm.w(mm.arg(0), open_shift[0]), mm.w(mm.arg(0) + 4, open_shift[1])),
                              mm.ret(24, int(open_ok))),
        0x404F60: lambda mm: (events.append({'log': True}), mm.ret(0)),
        0x4CB5B0: lambda mm: (events.append({'failed': mm.label(mm.arg(0))}), mm.ret(4)),
        0x7A0C20: lambda mm: (events.append({'sai': mm.label(mm.arg(0))}), mm.ret(4)),
        0x4CEF10: lambda mm: (events.append({'property': mm.label(mm.reg(UC_X86_REG_ECX)),
                                             'value': mm.label(mm.arg(0)), 'kind': mm.arg(1), 'source': mm.arg(2)}),
                              mm.ret(12, 1)),
        0x4C8500: lambda mm: (events.append({'fx': mm.label(mm.reg(UC_X86_REG_ECX)), 'event': mm.arg(0),
                                             'target': mm.label(mm.r(mm.arg(1) + 0x1C))}), mm.ret(8)),
        0x4CA330: lambda mm: (events.append({'command': mm.label(mm.arg(0))}), mm.ret(4, mm.reg(UC_X86_REG_ECX))),
        0x4D9B60: lambda mm: (events.append({'issue': mm.label(mm.arg(0)),
                                             'command_words': words(mm, mm.reg(UC_X86_REG_ECX) + 0xC, 1)
                                             + words(mm, mm.reg(UC_X86_REG_ECX) + 0x18, 5)}), mm.ret(4)),
        0x4CA2D0: lambda mm: (events.append({'command_done': True}), mm.ret(0)),
        0x552640: lambda mm: (events.append({'clamp': words(mm, mm.arg(0), 2)}), mm.ret(4)),
        0x4C2290: lambda mm: (events.append({'organization_data': mm.arg(0)}), mm.ret(4, 0x0D0D0)),
        0x58BA90: lambda mm: (events.append({'payload': mm.label(mm.arg(0)), 'organization': mm.arg(2),
                                             'start': [mm.arg(3), mm.arg(4)], 'drop': [mm.arg(5), mm.arg(6)],
                                             'component': mm.reg(UC_X86_REG_ECX) == mm.r(created + 0xC0)}),
                              mm.ret(28)),
        0x4D9E20: lambda mm: (mm.u.mem_write(mm.reg(UC_X86_REG_ECX), bytes(0x40)),
                              events.append({'command_kind': mm.arg(0)}), mm.ret(4, mm.reg(UC_X86_REG_ECX))),
    }

    def creator(name):
        def run(mm):
            d = mm.arg(0)
            w = words(mm, d, 21)
            events.append({name: [mm.label(w[1])] + w[2:7] + [w[8]] + w[9:21],
                           'world': mm.reg(UC_X86_REG_ECX) == world})
            mm.ret(4, created)
        return run
    hooks[0x4F9C10] = creator('create_object')
    hooks[0x4FA030] = creator('create_actor')
    hooks[0x4FA150] = creator('create_organization')
    saved = dict(m.hooks)
    m.hooks.update(hooks)
    al = m.call(0x4CCA70, ecx=effect, args=(ability, ctx, f32(1.0)), pop=12) & 0xFF
    m.hooks = saved
    summoned = []
    cell = m.r(ability + 0x24)
    while cell:
        summoned.append(m.label(m.r(m.r(cell)) if False else m.r(cell)))
        cell = m.r(cell + 4)
    return {'kind': kind, 'target_location': bool(flags54), 'source': has_source, 'zero_vector': zero_vector,
            'campaign': campaign, 'ethereal': ethereal, 'open_ok': open_ok, 'create_ok': create_ok,
            'controllable': controllable, 'property': has_property, 'target': has_target,
            'target_attacks': target_attacks, 'body_attacker': body_attacker, 'allegiance': has_allegiance,
            'sai': sai, 'members': members, 'map': [f32(W), f32(H)], 'source_position': sp,
            'target_position': tp, 'facing': facing, 'extent': extent, 'source_actor_position': src_pos,
            'random_direction': random_dir, 'open_shift': open_shift,
            'created_gtid': m.r(created + 0x14) if created else 0,
            'attacker_gtid': m.r(attacker + 0x14) if attacker else 0,
            'returned': al, 'events': events, 'created_flags8': m.r(created + 8) if created else None,
            'summoned': summoned, 'context_target_restored': m.r(ctx + 0x1C) == saved_target}


def main():
    rng = random.Random(0x4CCA70)
    m = Oracle()
    cases = [case(m, rng, i) for i in range(160)]
    h.report('native-summon-fixtures.json', 'SummonEffect Apply 4CCA70 and creators (round 5); seams in docstring.',
             cases, m.visited)
    kinds = {}
    for c in cases:
        kinds[(c['kind'], c['returned'])] = kinds.get((c['kind'], c['returned']), 0) + 1
    print(kinds)


if __name__ == '__main__':
    main()
