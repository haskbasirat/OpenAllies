"""KKC_UAIStateMove::ResultCollisionThing 0x536840: the element move state
met a thing on its path and decides whether to Engage or Kill it.

The whole routine executes unchanged with 0x529C30 (state actor), 0x432E40
(GTID registry), 0x45D880 (squared distance), 0x52DE10 (decision record
initialiser), 0x455000 (near-equal test) and the diagnostic logs 0x404F60/
0x404F90 (their bodies return because the log level at 0xB38B7C is 0).

Seams (recorded in order, scripted results): thing virtual +B4, actor
virtual +16C, 0x4D4FF0 relation, 0x5340C0 kill decision (records the record
it was handed, then writes the scripted record), 0x5358A0 engage hand-off,
0x5351C0 kill select and the Mind Updatable (+8) virtual +4.

Usage: python native_result_collision_thing.py [path/to/AA.exe]
(default: the AA.exe two folders above this repository's research folder).
Writes native-result-collision-thing.json next to this script.
"""
import os, sys
from pathlib import Path
if len(sys.argv) > 1:
    os.environ['AA_GAME_DIR'] = str(Path(sys.argv[1]).resolve().parent)
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import copy, random, struct  # noqa: E402
import harness as h  # noqa: E402
from harness import f2b  # noqa: E402

THING, CURRENT = 1, 2
INITIAL = [0, 1, 3, 0xbf800000, 0, 0x7f7fffff, 0]


def stub(m, fn):
    a = m.stub_next
    m.stub_next += 16
    m.u.mem_write(a, b'\xcc' * 16)
    m.seams[a] = fn
    return a


def register(m, reg, gtid, obj):
    idx = gtid & 0xffff
    m.u.mem_write(reg + 4 + 2 * idx, struct.pack('<H', gtid >> 16))
    m.w32(reg + 0x20004 + 4 * idx, obj)


def run(c):
    m = h.Machine(c['precision'])
    events = []
    reg = m.alloc(0x20004 + 0x10000 * 4 + 16)
    m.w32(0xb364c0, reg)
    mind = m.alloc(0x200)
    state = m.alloc(0x40)
    m.w32(state + 4, mind)
    actor = m.alloc(0x100)
    m.w32(mind + 4, actor)
    m.w32(actor + 0x14, 0x00070007)
    m.w32(actor + 0x20, *c['actor_position'])
    ids = {0: 0}

    def ident(p):
        return ids.get(p, 0xdead)

    def v_accepts(mm):
        assert mm.ecx() == actor
        events.append(['accepts', ident(mm.arg(0))])
        mm.ret(4, int(c['accepts']))

    avt = m.alloc(0x200)
    m.w32(actor, avt)
    m.w32(avt + 0x16c, stub(m, v_accepts))

    def v_schedule(mm):
        assert mm.ecx() == mind + 8
        events.append(['schedule', mm.arg(0), mm.arg(1)])
        mm.ret(8, 0)

    uvt = m.alloc(0x10)
    m.w32(mind + 8, uvt)
    m.w32(uvt + 4, stub(m, v_schedule))
    m.w32(mind + 0x104, c['kill_status'])
    m.w32(mind + 0x108, c['kill_target'])
    m.w32(mind + 0x138, c['engage_status'])

    def v_actor(mm):
        events.append(['is_actor', ident(mm.ecx())])
        mm.ret(0, int(c['is_actor']))

    tvt = m.alloc(0x200)
    m.w32(tvt + 0xb4, stub(m, v_actor))
    thing = 0
    if c['thing'] is not None:
        thing = m.alloc(0x100)
        ids[thing] = THING
        m.w32(thing, tvt)
        m.w32(thing + 0x14, c['thing']['gtid'])
        m.w32(thing + 0x20, *c['thing']['position'])
        if c['thing'].get('registered'):
            register(m, reg, c['thing']['gtid'], thing)
    if c['current'] is not None:
        cur = m.alloc(0x100)
        ids[cur] = CURRENT
        m.w32(cur, tvt)
        m.w32(cur + 0x14, c['current']['gtid'])
        m.w32(cur + 0x20, *c['current']['position'])
        register(m, reg, c['current']['gtid'], 0 if c['current'].get('null_entry') else cur)

    def s_relation(mm):
        assert mm.ecx() == actor
        events.append(['relation', ident(mm.arg(0)), mm.arg(1), mm.arg(2)])
        mm.ret(12, c['relation'])

    calls = []

    def s_choose(mm):
        assert mm.ecx() == mind
        target, point, record, by_point = (mm.arg(i) for i in range(4))
        before = m.r32(record, 7)
        events.append(['choose', ident(target), point, by_point, *before])
        words = c['records'][len(calls)]
        calls.append(target)
        m.w32(record, *words)
        mm.ret(16, words[2])

    def s_engage(mm):
        assert mm.ecx() == mind + 0x12c
        events.append(['engage', ident(mm.arg(0))])
        mm.ret(4, 1)

    def s_kill(mm):
        assert mm.ecx() == mind + 0xf8
        events.append(['kill', mm.arg(0), ident(mm.arg(1))])
        mm.ret(8, 1)

    for address, fn in [(0x4d4ff0, s_relation), (0x5340c0, s_choose), (0x5358a0, s_engage),
                        (0x5351c0, s_kill)]:
        m.seams[address] = fn
    result = m.call(0x536840, ecx=state, args=(thing,))
    assert m.final_esp == h.STACK + 8, hex(m.final_esp)
    assert m.fpu_top() == 0
    return {'result': result & 0xff, 'events': events}


def ulp(bits, k):
    """`bits` moved k binary32 units; a step that leaves the finite numbers of
    the same sign keeps `bits`."""
    out = bits + k
    if out < 0 or out > 0xffffffff or (out ^ bits) >> 31 or out & 0x7f800000 == 0x7f800000:
        return bits
    return out


def near(rnd, bits):
    """A binary32 near `bits`: equal, a few units away or about 1e-4 away."""
    v = h.b2f(bits)
    choice = rnd.randrange(8)
    if choice == 0:
        return bits
    if choice < 3:
        return ulp(bits, rnd.choice([-2, -1, 1, 2]))
    delta = rnd.choice([1e-4, -1e-4, 0.99e-4, -0.99e-4, 1.01e-4, -1.01e-4, 3e-4, -3e-4, 5e-5])
    out = f2b(v + delta)
    if rnd.random() < 0.3:
        out = ulp(out, rnd.choice([-1, 1]))
    return out


SCORES = [-1.0, 0.0, 1.0, 10.0, 12.5, 1.0 / 3.0, 1e4, 0.75, 2e-4, 1e-4]
DISTS = [0.0, 1.0, 25.0, 100.0, 3.4028234663852886e38, 1e-4, 2e-4, 1e4, 0.5]


def record(rnd):
    return [rnd.choice([0, 0x1234, 0x1234, 0x1234, 0x5678]), rnd.choice([1, 2, 5]),
            rnd.choice([0, 1, 2, 3]), f2b(rnd.choice(SCORES)), 0, f2b(rnd.choice(DISTS)),
            rnd.choice([0, 1, 2, 3, -1, 0x7fffffff])]


def tie(rnd, a):
    b = record(rnd)
    if rnd.random() < 0.6:
        b[6] = a[6]
    if rnd.random() < 0.6:
        b[3] = near(rnd, a[3])
    if rnd.random() < 0.6:
        b[5] = near(rnd, a[5]) if a[5] != 0x7f7fffff else ulp(a[5], -rnd.randrange(3))
    return b


def position(rnd):
    return [f2b(rnd.choice([rnd.uniform(0, 400), float(rnd.randrange(400)), rnd.uniform(0, 1)])),
            f2b(rnd.choice([rnd.uniform(0, 400), float(rnd.randrange(400)), rnd.uniform(0, 1)]))]


def mirrored(rnd, actor, other):
    a = [h.b2f(x) for x in actor]
    o = [h.b2f(x) for x in other]
    pick = rnd.randrange(4)
    if pick == 0:
        return [f2b(2 * a[0] - o[0]), other[1]]
    if pick == 1:
        return [other[0], f2b(2 * a[1] - o[1])]
    if pick == 2:
        return [ulp(other[0], rnd.choice([-1, 1])), other[1]]
    return [f2b(a[0] + (o[1] - a[1])), f2b(a[1] + (o[0] - a[0]))]


def cases():
    rnd = random.Random(0x536840)
    rows = []
    for i in range(1100):
        actor = position(rnd)
        thing = None
        if rnd.random() < 0.97:
            thing = {'gtid': 0x00030011, 'position': position(rnd), 'registered': False}
        current = None
        kill_target = rnd.choice([0, 0x00050021, 0x00050021, 0x00050021, 0x00030011, 0x00060021])
        mode = rnd.random()
        if kill_target in (0x00050021, 0x00060021):
            current = {'gtid': 0x00050021, 'null_entry': rnd.random() < 0.1,
                       'position': position(rnd)}
            if thing is not None and rnd.random() < 0.5:
                thing['position'] = mirrored(rnd, actor, current['position'])
        elif kill_target == 0x00030011 and thing is not None:
            thing['registered'] = True
        r1 = record(rnd)
        r2 = tie(rnd, r1) if mode < 0.7 else record(rnd)
        rows.append({'name': f'collision{i}', 'actor_position': actor, 'thing': thing,
                     'current': current, 'kill_target': kill_target,
                     'is_actor': rnd.random() < 0.93, 'accepts': rnd.random() < 0.93,
                     'relation': rnd.choice([0, 0, 0, 0, 0, 1, 2, 6, 0xffffffff]),
                     'engage_status': rnd.choice([0, 1, 2, 2]),
                     'kill_status': rnd.choice([0, 1, 2, 2, 2]),
                     'records': [r1, r2]})
        if i >= 400:
            # Biased toward the record comparison: every gate open.
            rows[-1].update(is_actor=True, accepts=True, relation=0, kill_status=2)
            if thing is None:
                rows[-1]['thing'] = {'gtid': 0x00030011, 'position': position(rnd), 'registered': False}
    out = []
    for p in h.PRECISION:
        for c in rows:
            out.append({**copy.deepcopy(c), 'name': c['name'] + '_' + p, 'precision': p})
    return out


def main():
    rows = []
    for c in cases():
        try:
            output = run(c)
        except Exception:
            print(c['name'])
            raise
        rows.append({'input': c, 'output': output})
    counts = {}
    for r in rows:
        key = 'none' if not r['output']['result'] else r['output']['events'][-2][0]
        counts[key] = counts.get(key, 0) + 1
    print(counts)
    h.write_document('native-result-collision-thing.json', '0x536840 ResultCollisionThing', rows,
                     {'initial_record': INITIAL})


if __name__ == '__main__':
    main()
