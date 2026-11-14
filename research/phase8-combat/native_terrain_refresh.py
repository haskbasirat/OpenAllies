"""Native oracle for the actor terrain refresh 4D4130. Recorded seams:
4D10A0, 570560 (membership terrain), 54ECE0 (tile terrain; the tile is
recovered from the grid record address), 579EB0, 581810, 5035E0, 5817A0.
Terrain +B0 lists are [property, next] nodes.

Usage: native_terrain_refresh.py  (AA.exe is located by the shared harness)
"""
from pathlib import Path
import random
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import harness as h  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX  # noqa: E402

WIDTH = 7


def main():
    rng = random.Random(0x4D4130)
    m = h.Machine('Double53')
    world = m.alloc(0x100)
    m.w(h.WORLD_ROOT, world)
    grid = m.alloc(0x40)
    records = m.alloc(0x30 * WIDTH * WIDTH)
    m.w(grid + 0x1C, WIDTH)
    m.w(grid + 0x28, records)
    m.w(world + 0x30, grid)
    props = {}
    for k in range(5):
        props[m.alloc(0x20)] = k
    prop_ptr = {v: k for k, v in props.items()}
    cases = []
    for _ in range(300):
        events = []
        terrains, tptr = [], {}
        for t in range(3):
            values = [rng.choice((None, 0, 1, 2, 3, 4)) for _ in range(rng.randint(0, 3))]
            obj = m.alloc(0x100)
            head = 0
            for v in reversed(values):
                node = m.alloc(8)
                m.w(node, prop_ptr[v] if v is not None else 0)
                m.w(node + 4, head)
                head = node
            m.w(obj + 0xB0, head)
            terrains.append(values)
            tptr[obj] = t
        tobj = {t: p for p, t in tptr.items()}
        a = m.alloc(0x200)
        flags = rng.choice((0, 0, 0, 0, 0, 0, 0x5, 1 << 29, 1 << 12))
        word100 = rng.choice((0, 0, 0, 0, 0x2000000, 0x1000000))
        check = rng.choice((1, 1, 1, 1, 1, 0x101, 0, 0x100))
        old = rng.choice((None, 0, 1, 2))
        has_membership = rng.random() < 0.4
        has_org = rng.random() < 0.5
        has_props = rng.random() < 0.8
        new = rng.choice((0, 1, 2, None)) if rng.random() < 0.9 else old
        if new is None and new != old and has_props:
            new = 0
        tile = [rng.randrange(WIDTH), rng.randrange(WIDTH)]
        m.w(a + 8, flags)
        m.w(a + 0x100, word100)
        m.w(a + 0xF8, tobj[old] if old is not None else 0)
        m.w(a + 0x44, tile[0])
        m.w(a + 0x48, tile[1])
        ms = org = pr = 0
        if has_membership:
            ms = m.alloc(0x20)
        if has_org:
            org = m.alloc(0x40)
        if has_props:
            pr = m.alloc(0x40)
        m.w(a + 0x80, ms)
        m.w(a + 0x7C, org)
        m.w(a + 0x78, pr)
        new_ptr = tobj[new] if new is not None else 0
        applicable = {k: rng.choice((1, 0, 0x100)) for k in range(5)}

        def rec(tag, pop, value=None, ev=events):
            def run(mm):
                ev.append([tag])
                mm.ret(pop, value if value is not None else 0)
            return run
        m.hooks[0x4D10A0] = rec('check', 0, check)
        m.hooks[0x570560] = rec('membership_terrain', 0, new_ptr)

        def tile_hook(mm, ev=events, value=new_ptr):
            idx = (mm.reg(UC_X86_REG_ECX) - records) // 0x30
            ev.append(['tile_terrain', idx % WIDTH, idx // WIDTH])
            mm.ret(0, value)
        m.hooks[0x54ECE0] = tile_hook
        m.hooks[0x579EB0] = rec('organization_refresh', 0)

        def remove(mm, ev=events):
            p = mm.arg(0)
            assert mm.reg(UC_X86_REG_ECX) == pr and mm.arg(1) == a
            ev.append(['remove', props[p] if p else None])
            mm.ret(8)

        def appl(mm, ev=events, ap=applicable):
            assert mm.arg(0) == a
            k = props[mm.arg(1)]
            ev.append(['applicable', k])
            mm.ret(0, ap[k])

        def add(mm, ev=events):
            assert mm.reg(UC_X86_REG_ECX) == pr and mm.arg(2) == a
            ev.append(['add', props[mm.arg(0)], mm.arg(1)])
            mm.ret(12)
        m.hooks[0x581810] = remove
        m.hooks[0x5035E0] = appl
        m.hooks[0x5817A0] = add
        m.hooks[0x404ED0] = lambda mm: mm.ret(0)
        m.call(0x4D4130, ecx=a, pop=0)
        final = m.r(a + 0xF8)
        cases.append({'terrains': terrains, 'flags': flags, 'word100': word100, 'check': check, 'old': old,
                      'new': new, 'membership': has_membership, 'organization': has_org, 'properties': has_props,
                      'tile': tile, 'applicable': [applicable[k] for k in range(5)],
                      'final': tptr.get(final) if final else None, 'events': events})
    h.report('native-terrain-refresh-fixtures.json', '4D4130 terrain refresh with recorded seams.', cases, m.visited)


if __name__ == '__main__':
    main()
