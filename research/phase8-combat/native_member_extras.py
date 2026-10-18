"""Native oracle for the round-5 member extras (saved-components agent
requests): Element/membership attach 5705A0, property propagation to a new
member 5784C0 and Performer ability gating by rank 57DCF0 (with 4BE4D0 and
4BE4F0 executing). Recorded seams: 4D1350, 4CF0F0, 7A0C20, 4CEF10.
"""
from pathlib import Path
import random
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import harness as h  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX  # noqa: E402


def main():
    rng = random.Random(0x5705A0)
    m = h.Machine('Double53')
    m.u.mem_map(0, 0x1000)
    names = {}
    label = lambda v: names.get(v, v) if v else 0
    attach = []
    for index in range(12):
        events = []
        member = m.alloc(0x100)
        names[member] = 'member'
        membership = m.alloc(0x40)
        m.w(membership + 4, member)
        observer = rng.random() < 0.5
        m.w(0xB36528, m.alloc(0x40) if observer else 0)
        owner, slot = 0x7100 + index, rng.randrange(-1, 6)
        m.hooks[0x4D1350] = lambda mm, ev=events: (ev.append({'notify': label(mm.reg(UC_X86_REG_ECX))}), mm.ret(0))
        m.hooks[0x4CF0F0] = lambda mm, ev=events: (ev.append({'refresh': label(mm.reg(UC_X86_REG_ECX))}), mm.ret(0))
        m.hooks[0x7A0C20] = lambda mm, ev=events: (ev.append({'observer': label(mm.arg(0))}), mm.ret(4))
        m.call(0x5705A0, ecx=membership, args=(owner, slot & 0xFFFFFFFF), pop=8)
        attach.append({'owner': owner, 'slot': slot, 'observer': observer, 'events': events,
                       'fields': [m.r(membership + 0x10), m.r(membership + 0x18), m.rb(membership + 0x1C)]})
    propagation = []
    for index in range(40):
        events = []
        owner = m.alloc(0x100)
        names[owner] = 'owner'
        has_props = rng.random() < 0.9
        specs = [{'kind': rng.choice((1, 2, 3, 7)), 'pass_down': rng.random() < 0.6,
                  'disabled': rng.random() < 0.2} for _ in range(rng.randint(0, 4))]
        if has_props:
            props = m.alloc(0x40)
            lst = m.alloc(4 * max(1, len(specs)))
            for i, s in enumerate(specs):
                inst = m.alloc(0x20)
                d = m.alloc(0x80)
                names[d] = f'p{i}'
                m.wb(d + 0x42, int(s['pass_down']))
                m.w(inst + 8, s['kind'])
                m.w(inst + 0xC, d)
                m.wb(inst + 0x10, int(s['disabled']))
                m.w(lst + 4 * i, inst)
            m.w(props + 0x10, lst)
            m.w(props + 0x14, len(specs))
            m.w(owner + 0x78, props)
        org = m.alloc(0x40)
        m.w(org + 4, owner)
        member_kind = rng.choice(('none', 'no_props', 'plain', 'a4'))
        member = 0
        if member_kind != 'none':
            member = m.alloc(0x100)
            names[member] = 'member'
            if member_kind != 'no_props':
                m.w(member + 0x78, m.alloc(0x20))
            if member_kind == 'a4':
                m.w(member + 0xA4, m.alloc(0x20))
        m.hooks[0x4CEF10] = lambda mm, ev=events: (
            ev.append({'apply': label(mm.reg(UC_X86_REG_ECX)), 'property': label(mm.arg(0)), 'kind': mm.arg(1),
                       'source': label(mm.arg(2))}), mm.ret(12, 1))
        m.call(0x5784C0, ecx=org, args=(member,), pop=4)
        propagation.append({'properties': has_props, 'specs': specs, 'member': member_kind, 'events': events})
    rank = []
    for index in range(40):
        events = []
        perf = m.alloc(0x40)
        owner = m.alloc(0x100)
        names[owner] = 'owner'
        m.w(perf + 4, owner)
        specs = []
        lst = m.alloc(4 * 6)
        for i in range(rng.randint(0, 5)):
            local = rng.random() < 0.85
            lo = rng.choice((-1, 0, 2, 5))
            hi = rng.choice((-1, 1, 3, 6))
            enabled = rng.random() < 0.5
            ab = m.alloc(0x40)
            if local:
                ld = m.alloc(0x80)
                m.w(ld + 0x60, lo & 0xFFFFFFFF)
                m.w(ld + 0x64, hi & 0xFFFFFFFF)
                m.w(ab + 8, ld)
            m.wb(ab + 0x18, int(enabled))
            m.w(lst + 4 * i, ab)
            specs.append({'local': local, 'min': lo, 'max': hi, 'enabled': enabled, 'handle': ab})
        m.w(perf + 0x14, lst)
        m.w(perf + 0x18, len(specs))
        has_rank = rng.random() < 0.8
        value = rng.choice((0, 1, 2, 3, 4, 5, 7))
        robj = 0
        if has_rank:
            robj = m.alloc(0x10)
            m.w(robj + 4, value)
        m.hooks[0x4D1350] = lambda mm, ev=events: (ev.append({'notify': label(mm.reg(UC_X86_REG_ECX))}), mm.ret(0))
        m.call(0x57DCF0, ecx=perf, args=(robj,), pop=4)
        after = [m.rb(s['handle'] + 0x18) for s in specs]
        for s in specs:
            del s['handle']
        rank.append({'specs': specs, 'rank_object': has_rank, 'rank': value, 'events': events, 'after': after})
    h.report('native-member-extras-fixtures.json', '5705A0 attach, 5784C0 propagation, 57DCF0 rank gating.',
             attach, m.visited, extra={'propagation': propagation, 'rank': rank})


if __name__ == '__main__':
    main()
