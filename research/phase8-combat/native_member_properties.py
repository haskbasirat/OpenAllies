"""Native oracle for the property copy 4D0660(source, no_source, need40,
need41): random property lists on the source (+78: +10 instance slots,
+14 count, instance +C definition with +40/+41 bytes), with 5035E0
(applicability) and 5817A0 (add) recorded.

Usage: native_member_properties.py  (AA.exe is located by the shared harness)
"""
from pathlib import Path
import random
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import harness as h  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX  # noqa: E402


def main():
    rng = random.Random(0x4D0660)
    m = h.Machine('Double53')
    cases = []
    for _ in range(300):
        events = []
        actor, source = m.alloc(0x100), m.alloc(0x100)
        actor_props = m.alloc(0x40) if rng.random() < 0.9 else 0
        m.w(actor + 0x78, actor_props)
        source_props = 0
        need40, need41 = rng.random() < 0.4, rng.random() < 0.4
        no_source = rng.choice((0, 1, 0x100))
        defs, names = [], {}
        if rng.random() < 0.9:
            source_props = m.alloc(0x40)
            n = rng.randint(0, 5)
            slots = m.alloc(4 * max(1, n))
            for k in range(n):
                inst = m.alloc(0x20)
                d = None
                if (need40 or need41) or rng.random() < 0.85:
                    d = {'b40': rng.choice((0, 1)), 'b41': rng.choice((0, 1)), 'applicable': rng.choice((1, 0, 0x100))}
                    dp = m.alloc(0x80)
                    m.wb(dp + 0x40, d['b40'])
                    m.wb(dp + 0x41, d['b41'])
                    m.w(inst + 0xC, dp)
                    names[dp] = (k, d)
                m.w(slots + 4 * k, inst)
                defs.append(d)
            m.w(source_props + 0x10, slots)
            count = n if rng.random() < 0.85 else rng.choice((0, -1, max(0, n - 1)))
            m.w(source_props + 0x14, count & 0xFFFFFFFF)
        else:
            count = None
        m.w(source + 0x78, source_props)

        def applicable(mm, ev=events):
            assert mm.arg(0) == actor
            k, d = names[mm.arg(1)]
            ev.append(['applicable', k])
            mm.ret(0, d['applicable'])

        def add(mm, ev=events):
            assert mm.reg(UC_X86_REG_ECX) == actor_props
            k, _ = names[mm.arg(0)]
            src = mm.arg(2)
            ev.append(['add', k, mm.arg(1), 'source' if src == source else src])
            mm.ret(12)
        m.hooks[0x5035E0] = applicable
        m.hooks[0x5817A0] = add
        m.call(0x4D0660, ecx=actor, args=(source, no_source, int(need40), int(need41)), pop=16)
        cases.append({'actor_properties': bool(actor_props), 'source_properties': bool(source_props),
                      'count': count, 'definitions': defs, 'no_source': no_source, 'need40': need40,
                      'need41': need41, 'events': events})
    h.report('native-member-properties-fixtures.json', '4D0660 property copy with recorded 5035E0/5817A0.',
             cases, m.visited)


if __name__ == '__main__':
    main()
