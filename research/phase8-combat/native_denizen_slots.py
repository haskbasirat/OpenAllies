"""Denizen slot scans 0x568860, 0x568D40, 0x568550, the slot-count helper
0x573AF0 and the record removal 0x573D10.

Whole routines execute unchanged over scripted lists: the +18 list of
0x14-byte slot entries (+4 word, +8 binary32, +D byte, +10 next), the +C
node list {GTID, next} with a real GTID registry at [B364C0], and the +14/+18
word or record array. Seams: 0x57A730 on an OrganizationComponent (recorded,
scripted member count) and, for 0x573D10, the free 0x6F7430 (recorded).

Usage: python native_denizen_slots.py [path/to/AA.exe]
Writes native-denizen-slots.json next to this script.
"""
import os, sys
from pathlib import Path
if len(sys.argv) > 1:
    os.environ['AA_GAME_DIR'] = str(Path(sys.argv[1]).resolve().parent)
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / 'phase8-movement'))
import hashlib, json, random, struct  # noqa: E402
import harness as h  # noqa: E402
from harness import f2b  # noqa: E402


def run(c):
    m = h.Machine('Single24')
    events = []
    d = m.alloc(0x100)
    e = c['entry']
    if e in ('active', 'free'):
        nodes = [m.alloc(0x14) for _ in c['entries']]
        for i, (p, ent) in enumerate(zip(nodes, c['entries'])):
            m.w32(p, 0x77, ent['word4'], ent['timer'])
            m.w8(p + 0xc, 0x5a)
            m.w8(p + 0xd, ent['active'])
            m.w32(p + 0x10, nodes[i + 1] if i + 1 < len(nodes) else 0)
        m.w32(d + 0x18, nodes[0] if nodes else 0)
        if e == 'active':
            r = m.call(0x568860, ecx=d)
            return {'result': r & 0xff}
        obj = m.alloc(0x10)
        m.w32(obj + 4, c['object_word4'])
        r = m.call(0x568d40, ecx=d, args=(obj,))
        assert m.final_esp == h.STACK + 8
        return {'result': nodes.index(r) if r else None}
    if e == 'count':
        arr = m.alloc(4 * max(1, len(c['words'])))
        if c['words']:
            m.w32(arr, *c['words'])
        m.w32(d + 0x14, arr, len(c['words']))
        return {'result': m.call(0x573af0, ecx=d)}
    if e == 'remove':
        ptrs, index = [], {}
        for i, r in enumerate(c['records']):
            if r is None:
                ptrs.append(0)
                continue
            p = m.alloc(0x10)
            m.w32(p, r['kind'], 0x33, r['word8'])
            ptrs.append(p)
            index[p] = i
        arr = m.alloc(4 * max(1, len(ptrs)))
        if ptrs:
            m.w32(arr, *ptrs)
        m.w32(d + 0x14, arr, len(ptrs))

        def s_free(mm):
            events.append(['free', index[mm.arg(0)]])
            mm.ret(0)

        m.seams[0x6f7430] = s_free
        r = m.call(0x573d10, ecx=d, args=(c['value'],))
        assert m.final_esp == h.STACK + 8
        slots = [0 if w == 0 else 1 for w in m.r32(arr, len(ptrs))] if ptrs else []
        return {'result': r - (1 << 32) if r >> 31 else r, 'events': events, 'slots': slots}
    reg = m.alloc(0x20004 + 0x10000 * 4 + 16)
    m.w32(0xb364c0, reg)
    orgs = {}
    for k, a in enumerate(c['actors']):
        p = m.alloc(0x100)
        idx = a['gtid'] & 0xffff
        m.u.mem_write(reg + 4 + 2 * idx, struct.pack('<H', a['registered_generation']))
        m.w32(reg + 0x20004 + 4 * idx, 0 if a['null_entry'] else p)
        if a['org'] is not None:
            o = m.alloc(0x40)
            m.w32(o + 0x2c, a['org']['capacity'])
            m.w32(p + 0x7c, o)
            orgs[o] = k
    nodes = [m.alloc(8) for _ in c['list']]
    for i, (p, g) in enumerate(zip(nodes, c['list'])):
        m.w32(p, g, nodes[i + 1] if i + 1 < len(nodes) else 0)
    m.w32(d + 0xc, nodes[0] if nodes else 0)

    def s_count(mm):
        k = orgs[mm.ecx()]
        events.append(['count', k])
        mm.ret(0, c['actors'][k]['org']['count'])

    m.seams[0x57a730] = s_count
    r = m.call(0x568550, ecx=d)
    return {'result': r & 0xff, 'events': events}


def cases():
    rnd = random.Random(0x568550)
    rows = []
    timers = [0.0, -0.0, 1.0, -2.5, 1e-30, float('nan'), 0.5]
    for i in range(150):
        entries = [{'word4': rnd.choice([3, 3, 5]), 'timer': f2b(rnd.choice(timers)),
                    'active': rnd.choice([0, 0, 0, 1, 0x80])} for _ in range(rnd.randrange(0, 6))]
        rows.append({'name': f'free{i}', 'entry': rnd.choice(['active', 'free']), 'entries': entries,
                     'object_word4': rnd.choice([3, 5, 9])})
    for i in range(80):
        rows.append({'name': f'count{i}', 'entry': 'count',
                     'words': [rnd.choice([0, 0, 0x10, 0x20]) for _ in range(rnd.randrange(0, 8))]})
    for i in range(120):
        records = [None if rnd.random() < 0.3 else {'kind': rnd.choice([2, 2, 1, 0]), 'word8': rnd.choice([4, 4, 6])}
                   for _ in range(rnd.randrange(0, 7))]
        rows.append({'name': f'remove{i}', 'entry': 'remove', 'records': records, 'value': rnd.choice([4, 6, 8])})
    for i in range(150):
        actors = []
        for k in range(rnd.randrange(0, 5)):
            g = ((k + 2) << 16) | (0x40 + k)
            actors.append({'gtid': g, 'registered_generation': (k + 2) if rnd.random() < 0.85 else (k + 3),
                           'null_entry': rnd.random() < 0.1,
                           'org': None if rnd.random() < 0.2 else {'capacity': rnd.choice([0, 3, 5, -1]),
                                                                    'count': rnd.choice([0, 3, 4, 5, 6])}})
        lst = [a['gtid'] for a in actors]
        rnd.shuffle(lst)
        if rnd.random() < 0.2:
            lst.append(0x00990099)
        rows.append({'name': f'open{i}', 'entry': 'open', 'actors': actors, 'list': lst})
    return rows


def main():
    rows = []
    for c in cases():
        try:
            rows.append({'input': c, 'output': run(c)})
        except Exception:
            print(c['name'])
            raise
    doc = {'schema': 1, 'scope': '0x568860 0x568D40 0x568550 0x573AF0', 'original_sha256': h.ORIGINAL_SHA,
           'v14_sha256': h.V14_SHA, 'case_count': len(rows), 'cases': rows,
           'instructions': [{'va': hex(a), 'bytes': b} for a, b in sorted(h.VISITED.items())]}
    out = HERE / 'native-denizen-slots.json'
    out.write_text(json.dumps(doc) + '\n')
    print(json.dumps({'file': out.name, 'cases': len(rows), 'instructions': len(h.VISITED),
                      'sha256': hashlib.sha256(out.read_bytes()).hexdigest()}))


if __name__ == '__main__':
    main()
