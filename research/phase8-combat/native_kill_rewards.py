"""Native oracle for the kill rewards of the kill credit 4D57C0:
4D3E20 (lair reward), its range draw 45D640, and 4D10F0 (organization
reward), all precisions.

45D640 runs with the real 85D840 over random RNG states and, for the
unsigned-conversion boundaries, with 85D840 hooked to return chosen draws.
4D3E20 runs with the real 4F02B0, 45D640, 85D840 and 471A30; recorded seams
are the killer +108 and lair +70/+24 virtuals, 4E2080 (resource grant),
404F60 (debug log), 42D490 (message post) and 77DA70 (sync log, not
recorded). 4D10F0 runs with 7A2370 recorded.

Usage: native_kill_rewards.py  (AA.exe is located by the shared harness)
"""
from pathlib import Path
import random
import struct
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import harness as h  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX  # noqa: E402

RESOURCES_ROOT = 0xB36530
MESSAGE_LOG = 0xB39D8C
BOUNDARY_DRAWS = (0, 1, 0x7FFFFFFF, 0x80000000, 0x80000001, 0xFFFFFFFE, 0xFFFFFFFF, 0x12345678, 0xDEADBEEF)


def f32(v):
    return struct.unpack('<I', struct.pack('<f', v))[0]


def bound(rng):
    return rng.choice((f32(0.0), f32(1.0), f32(rng.uniform(0, 50)), f32(rng.uniform(0, 500)),
                       f32(-rng.uniform(0, 20)), f32(rng.uniform(1e3, 1e6)), f32(0.1), 0x80000000))


def ext(m):
    st = m.st0_ext()
    return {'mantissa': f'{st["mantissa"]:016x}', 'sign_exponent': st['sign_exponent']}


def range_draws(m, rng, precision, out):
    for i in range(160):
        definition = m.alloc(0x600)
        low, high = bound(rng), bound(rng)
        if rng.random() < 0.1:
            high = low
        m.w(definition + 0x528, low)
        m.w(definition + 0x52C, high)
        state = m.alloc(0x10)
        before = rng.getrandbits(32)
        m.w(state, before)
        hooked = i >= 110
        draw = None
        if hooked:
            draw = BOUNDARY_DRAWS[i % len(BOUNDARY_DRAWS)]
            m.hooks[0x85D840] = lambda mm, d=draw, s=state: (
                assert_arg(mm, s), mm.ret(0, d))
        else:
            m.hooks.pop(0x85D840, None)
        m.call(0x45D640, ecx=definition, args=(state,), pop=4)
        case = {'precision': precision, 'low': low, 'high': high, 'state': before,
                'hooked_draw': draw, 'state_after': m.r(state)}
        case.update(ext(m))
        out.append(case)
    m.hooks.pop(0x85D840, None)


def assert_arg(m, expected):
    assert m.arg(0) == expected


def lair_rewards(m, rng, precision, out):
    for i in range(60):
        events = []
        world = m.alloc(0x200)
        m.w(h.WORLD_ROOT, world)
        state = rng.getrandbits(32)
        m.w(world + 0xD4, state)
        m.w(0xB3655C, m.alloc(0x20))
        m.w(0xB36560, m.alloc(0x20))
        m.hooks[0x77DA70] = lambda mm: mm.ret(12)
        log = m.alloc(0x20)
        m.w(MESSAGE_LOG, log)
        # resources
        count = rng.choice((0, 1, 2, 3, 4, 5, 5, -1))
        resources = []
        table = m.alloc(0x40)
        arr = m.alloc(4 * 6)
        for k in range(max(count, 0)):
            res = m.alloc(0x40)
            rec = m.alloc(0x40)
            word18 = 0 if rng.random() < 0.6 else rng.choice((1, 2, 0x100))
            label = rng.getrandbits(16)
            m.w(res + 0x18, word18)
            m.w(res + 0x28, rec)
            m.u.mem_write(rec + 0x18, struct.pack('<H', label))
            m.w(arr + 4 * k, res)
            resources.append({'word18': word18, 'label': label})
        m.w(table + 0x34, arr)
        m.w(table + 0x38, count & 0xFFFFFFFF)
        m.w(RESOURCES_ROOT, table)
        # killer
        has_object = rng.random() < 0.9
        kingdom = rng.choice((1, 2, 5, 0x80000003))
        killer = m.alloc(0x200)
        kobj = m.alloc(0x40) if has_object else 0
        m.w(killer + 0xE8, kobj)

        def kingdom_hook(mm, ev=events, k=kingdom):
            ev.append(['kingdom'])
            mm.ret(0, k)

        def grant_hook(mm, ev=events, o=kobj):
            assert mm.reg(UC_X86_REG_ECX) == o
            ev.append(['grant', mm.arg(0), mm.arg(1)])
            mm.ret(8)
        m.w(killer, m.vtable({0x108: kingdom_hook}))
        m.hooks[0x4E2080] = grant_hook
        # lair
        lair = m.alloc(0x100)
        definition = m.alloc(0x600)
        low, high = bound(rng), bound(rng)
        m.w(definition + 0x528, low)
        m.w(definition + 0x52C, high)
        m.w(lair + 4, definition)
        lair_id = rng.getrandbits(20)
        m.w(lair + 0x14, lair_id)
        see = rng.choice((1, 0x101, 0, 0x100)) if rng.random() < 0.3 else 1
        string = m.alloc(0x40) + 0x10
        m.w(string - 0x10, m.alloc(0x10))
        m.w(string - 4, 1000)

        def see_hook(mm, ev=events, s=see):
            ev.append(['can_see', mm.arg(0), mm.arg(1)])
            mm.ret(8, s)

        def name_hook(mm, ev=events, s=string):
            ev.append(['name'])
            mm.w(mm.arg(0), s)
            mm.ret(4, mm.arg(0))

        def post_hook(mm, ev=events, lg=log, la=lair, s=string):
            assert (mm.arg(0), mm.arg(3), mm.arg(7)) == (lg, la, s)
            ev.append(['post', mm.arg(1), mm.arg(2), mm.arg(4), mm.arg(5) | (mm.arg(6) << 32)])
            mm.ret(0)

        def ignored_hook(mm, ev=events):
            ev.append(['ignored', mm.arg(5)])
            mm.ret(0)
        m.w(lair, m.vtable({0x70: see_hook, 0x24: name_hook}))
        m.hooks[0x42D490] = post_hook
        m.hooks[0x404F60] = ignored_hook
        m.call(0x4D3E20, ecx=killer, args=(lair,), pop=4)
        out.append({'precision': precision, 'kingdom_object': has_object, 'kingdom': kingdom, 'can_see': see,
                    'lair_id': lair_id, 'low': low, 'high': high, 'state': state,
                    'state_after': m.r(world + 0xD4), 'resource_count': count, 'resources': resources,
                    'events': events})


def organization_rewards(m, rng, out):
    for i in range(40):
        events = []
        killer = m.alloc(0x200)
        names = {}
        own = rng.random() < 0.4
        if own:
            o = m.alloc(0x20)
            names[o] = 'own'
            m.w(killer + 0x154, o)
        membership = rng.choice(('none', 'no_org', 'org', 'org'))
        org154 = False
        if membership != 'none':
            ms = m.alloc(0x20)
            m.w(killer + 0x80, ms)
            if membership == 'org':
                org = m.alloc(0x200)
                m.w(ms + 0x10, org)
                org154 = rng.random() < 0.7
                if org154:
                    o = m.alloc(0x20)
                    names[o] = 'organization'
                    m.w(org + 0x154, o)
        victim = m.alloc(0x100)
        m.hooks[0x7A2370] = lambda mm, ev=events, v=victim, nm=names: (
            ev.append(['reward', nm[mm.reg(UC_X86_REG_ECX)], mm.arg(0) == v]), mm.ret(4))
        m.call(0x4D10F0, ecx=killer, args=(victim,), pop=4)
        out.append({'own': own, 'membership': membership, 'organization154': org154, 'events': events})


def main():
    rng = random.Random(0x4D3E20)
    draws, lairs, orgs, visited = [], [], [], {}
    for precision in h.PRECISIONS:
        m = h.Machine(precision)
        m.u.mem_map(0, 0x1000)  # fs:[0] SEH chain (FS base 0)
        range_draws(m, rng, precision, draws)
        lair_rewards(m, rng, precision, lairs)
        if precision == 'Double53':
            organization_rewards(m, rng, orgs)
        visited.update(m.visited)
    h.report('native-kill-rewards-fixtures.json',
             '45D640 range draw, 4D3E20 lair reward and 4D10F0 organization reward; all precisions.',
             draws, visited, extra={'lairs': lairs, 'organizations': orgs})


if __name__ == '__main__':
    main()
