"""Cached operational-range actor list of an organization component:
0x5781B0 (use the cache), 0x57A780 (rebuild when empty or older than the
UAICachedORActorListTimeout SVar), 0x558150 (begin on an actor iterator) and
the cached branch of 0x558750 (walk with 0x578470 range test).

Whole routines execute unchanged: 0x5781B0, 0x57A780, 0x558150, 0x577F20,
0x529C30, 0x558750's cached branch and 0x578470. Seams (recorded): the SVar
lookup 0x4B2E60 (returns the scripted timeout cell; asserts the
UAICachedORActorListTimeout name at 0xA3CED8 and kind 2), 0x4CEDE0 (moving,
on the component actor), the circle iterator 0x558720 (construct), 0x5580F0
(begin x, y, radius), the live branch of 0x558750 (scripted actors, one list
per begin) and 0x557FD0 (finish), and the allocator 0x6F7810/0x6F7430.

A case is a sequence of steps on one machine (the list and the SVar static
caches persist between steps): each sets the world time (+E0), the timeout
value and the moving answer, may make some registry entries stale, then
calls 0x57A780 or 0x558150 (walking a cached iterator with 0x558750 until
null). After each step the list (+C4 head to +C8 tail) and +CC are recorded.

Usage: python native_or_actor_list.py [path/to/AA.exe]
Writes native-or-actor-list.json next to this script.
"""
import os, sys
from pathlib import Path
if len(sys.argv) > 1:
    os.environ['AA_GAME_DIR'] = str(Path(sys.argv[1]).resolve().parent)
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / 'phase8-movement'))
import copy, hashlib, json, random, struct  # noqa: E402
import harness as h  # noqa: E402
from harness import f2b, b2f  # noqa: E402

SVAR_NAME, SVAR_KIND, SVARS = 0xa3ced8, 2, 0xb39aa8


def register(m, reg, gtid, obj):
    idx = gtid & 0xffff
    m.u.mem_write(reg + 4 + 2 * idx, struct.pack('<H', gtid >> 16))
    m.w32(reg + 0x20004 + 4 * idx, obj)


def run(c):
    m = h.Machine(c['precision'])
    world = m.alloc(0x100)
    m.w32(0xb36534, world)
    reg = m.alloc(0x20004 + 0x10000 * 4 + 16)
    m.w32(0xb364c0, reg)
    cell = m.alloc(4)
    comp = m.alloc(0x100)
    actor = m.alloc(0x100)
    definition = m.alloc(0x400)
    m.w32(comp + 4, actor)
    m.w32(actor + 4, definition)
    m.w32(actor + 0x20, *c['position'])
    m.w32(definition + 0x300, c['range'])
    m.w32(definition + 0x304, c['range2'])
    actors, ids = [], {0: 0}
    for i, a in enumerate(c['actors']):
        p = m.alloc(0x100)
        m.w32(p + 0x14, a['gtid'])
        m.w32(p + 0x20, *a['position'])
        register(m, reg, a['gtid'], 0 if a.get('null_entry') else p)
        actors.append(p)
        ids[p] = i + 1
    events = []
    live = {}
    state = {'query': 0, 'moving': False}

    def s_svar(mm):
        assert mm.ecx() == SVARS and mm.arg(0) == SVAR_NAME and mm.arg(1) == SVAR_KIND
        mm.ret(8, cell)

    def s_moving(mm):
        assert mm.ecx() == actor
        events.append(['moving'])
        mm.ret(0, int(state['moving']))

    def s_construct(mm):
        m.w32(mm.ecx() + 0xa4, 0)
        events.append(['construct'])
        mm.ret(0, mm.ecx())

    def s_begin(mm):
        x, y, r = mm.arg(0), mm.arg(1), mm.arg(2)
        events.append(['begin', x, y, r])
        script = c['queries'][state['query'] % len(c['queries'])]
        state['query'] += 1
        live[mm.ecx()] = [actors[i] for i in script]
        mm.ret(12, mm.ecx())

    def s_next(mm):
        if m.r32(mm.ecx() + 0xa4) != 0:
            return  # cached branch: runs unchanged
        queue = live.get(mm.ecx(), [])
        mm.ret(0, queue.pop(0) if queue else 0)

    def s_finish(mm):
        events.append(['finish'])
        mm.ret(0, 0)

    for address, fn in [(0x4b2e60, s_svar), (0x4cede0, s_moving), (0x558720, s_construct),
                        (0x5580f0, s_begin), (0x558750, s_next), (0x557fd0, s_finish)]:
        m.seams[address] = fn
    m.seams[0x6f7810] = lambda mm: mm.ret(0, mm.alloc(mm.arg()))
    m.seams[0x6f7430] = lambda mm: mm.ret()

    out = []
    for step in c['steps']:
        events.clear()
        m.w32(world + 0xe0, step['time'])
        m.w32(cell, step['timeout'])
        state['moving'] = step['moving']
        for i in step.get('stale', []):
            g = c['actors'][i]['gtid']
            register(m, reg, (g + 0x10000) & 0xffffffff, actors[i])
        row = {}
        if step['kind'] == 'refresh':
            r = m.call(0x57a780, ecx=comp)
            assert r == comp + 0xc4 and m.final_esp == h.STACK + 4
        else:
            it = m.alloc(0xb0)
            m.call(0x558150, ecx=it, args=(comp,))
            assert m.final_esp == h.STACK + 8
            cached = m.r32(it + 0xa4) != 0
            row['cached'] = cached
            if cached:
                assert m.r32(it + 0xa4) == comp and m.r32(it + 0x9c) == comp + 0xc4
                assert m.r32(it + 0xa0) == m.r32(comp + 0xc4)
                walked = []
                for _ in range(64):
                    got = m.call(0x558750, ecx=it)
                    if got == 0:
                        break
                    walked.append(ids[got])
                row['walked'] = walked
        assert m.fpu_top() == 0
        nodes, p = [], m.r32(comp + 0xc4)
        last = 0
        while p:
            nodes.append(m.r32(p))
            last = p
            p = m.r32(p + 4)
        tail = m.r32(comp + 0xc8)
        assert tail == last, 'tail is the last node'
        row.update(events=copy.deepcopy(events), list=nodes, refreshed=m.r32(comp + 0xcc))
        out.append(row)
    return out


def ulp(bits, k):
    out = bits + k
    if out < 0 or out > 0xffffffff or (out ^ bits) >> 31 or out & 0x7f800000 == 0x7f800000:
        return bits
    return out


TIMES = [0.0, 0.1, 1.0, 2.0, 3.5, 100.1, 1000.3, 4096.75, 1e5 + 0.03]


def cases():
    rnd = random.Random(0x57a780)
    rows = []
    for i in range(160):
        position = [f2b(rnd.uniform(10, 300)), f2b(rnd.uniform(10, 300))]
        rng = rnd.choice([0.0, 5.0, 12.5, 30.0, 64.0, 1.0 / 3.0])
        range2 = f2b(rng * rng) if rnd.random() < 0.8 else f2b(rnd.choice([0.0, 100.0, 900.0]))
        count = rnd.randrange(0, 7)
        actors = []
        for k in range(count):
            px, py = b2f(position[0]), b2f(position[1])
            if rnd.random() < 0.3:
                # On the circle of the squared range (boundary of 0x578470).
                r = b2f(range2) ** 0.5
                pos = [f2b(px + r), position[1]] if rnd.random() < 0.5 else [position[0], f2b(py - r)]
            else:
                pos = [f2b(px + rnd.uniform(-40, 40)), f2b(py + rnd.uniform(-40, 40))]
            actors.append({'gtid': ((k + 1) << 16) | (0x20 + k), 'position': pos,
                           'null_entry': rnd.random() < 0.08})
        queries = []
        for _ in range(rnd.randrange(1, 4)):
            q = list(range(count))
            rnd.shuffle(q)
            queries.append(q[:rnd.randrange(0, count + 1)])
        steps, t, last = [], f2b(rnd.choice(TIMES)), None
        for s in range(rnd.randrange(2, 8)):
            if s and rnd.random() < 0.8:
                t = f2b(b2f(t) + rnd.choice([0.0, 0.5, 1.0, 2.0, 2.5, 0.1, 3.0, 1000.0]))
            if last is not None and rnd.random() < 0.5:
                age = f2b(b2f(t) - b2f(last))
                timeout = ulp(age, rnd.choice([-1, 0, 0, 1]))
            else:
                timeout = f2b(rnd.choice([2.0, 2.0, 0.5, 5.0, 0.0, -1.0, 1e-4, 1000.0]))
            kind = rnd.choice(['refresh', 'begin', 'begin'])
            stale = [k for k in range(count) if rnd.random() < 0.1]
            steps.append({'kind': kind, 'time': t, 'timeout': timeout,
                          'moving': rnd.random() < 0.25, 'stale': stale})
            last = t if last is None or rnd.random() < 0.6 else last
        rows.append({'name': f'list{i}', 'position': position, 'range': f2b(rng), 'range2': range2,
                     'actors': actors, 'queries': queries, 'steps': steps})
    out = []
    for p in h.PRECISION:
        for c in rows:
            out.append({**copy.deepcopy(c), 'name': c['name'] + '_' + p, 'precision': p})
    return out


def main():
    rows = []
    for c in cases():
        try:
            rows.append({'input': c, 'output': run(c)})
        except Exception:
            print(c['name'])
            raise
    steps = [s for r in rows for s in r['output']]
    print({'steps': len(steps), 'cached_walks': sum(1 for s in steps if s.get('cached')),
           'live_begins': sum(1 for s in steps if s.get('cached') is False),
           'rebuilds': sum(1 for s in steps if ['construct'] in s['events'])})
    doc = {'schema': 1, 'scope': '0x5781B0 0x57A780 0x558150 0x558750 cached OR actor list',
           'original_sha256': h.ORIGINAL_SHA, 'v14_sha256': h.V14_SHA, 'case_count': len(rows),
           'cases': rows,
           'instructions': [{'va': hex(a), 'bytes': b} for a, b in sorted(h.VISITED.items())]}
    out = HERE / 'native-or-actor-list.json'
    out.write_text(json.dumps(doc) + '\n')
    print(json.dumps({'file': out.name, 'cases': len(rows), 'instructions': len(h.VISITED),
                      'sha256': hashlib.sha256(out.read_bytes()).hexdigest()}))


if __name__ == '__main__':
    main()
