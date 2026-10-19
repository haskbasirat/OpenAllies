"""Actual 451F90 positive Track tail of VActor Init (452061..452136).

After the (seamed) completed 457A40 base, 451F90 runs the ORIGINAL bytes of:
58CB30 Data getter, 45B7A0 with Thing NULL, the VActor pose copies, ActorData
vf38 (4F0500), 45E700 count (twice), the +148 store, 6F7810(count*0x74+4)
(allocator transport seam: a fresh fill-patterned block, or NULL), the leading
count store, _eh_vector_constructor_iterator 96D18D (972820/97285B/96D1D7) with
42D7B0/4B2830, the +144 store, the 45E710/42D830(owner=VActor, info) loop that
re-reads +144/+148 every iteration, and 85D840 into +14C.

Each case runs twice with two allocator/VActor fill patterns (A5, 5A): a byte
that differs between the runs is UNWRITTEN by native code (it keeps whatever
the allocation held). The NULL-allocation row records the native fault: 42D830
then writes through NULL+i*0x74. Seams: 457A40 (al=1) and 6F7810 only.
Every executed non-seam instruction is compared with the v14 image.
No game file is written.
"""
from pathlib import Path
import sys, json, struct, hashlib

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[1] / 'phase8-ai'))
from native_machine import Machine, original, modified, sha, instructions, header

VIEW_SIZE = 0x174
TRACK = 0x74
FILLS = (0xa5, 0x5a)
SNAN = {0x7f800001, 0xff800123, 0x7fbfffff}


def run(precision, c, fill):
    m = Machine(precision)
    events = []
    view = m.alloc(VIEW_SIZE)
    m.u.mem_write(view, bytes([fill]) * VIEW_SIZE)
    m.w(view, 0xa02270)
    data = m.alloc(0x600)
    m.u.mem_write(data, bytes([fill]) * 0x600)
    m.w(data, 0xa05540)
    infos = m.alloc(max(1, len(c['infos'])) * 0x24)
    for i, words in enumerate(c['infos']):
        for j, word in enumerate(words):
            m.w(infos + i * 0x24 + 4 * j, word)
    m.w(data + 0x25c, infos)
    m.w(data + 0x260, c['count'])
    m.w(view + 0x10, data)
    m.w(view + 0x14, 0)
    for i, word in enumerate(c['pose']):
        m.w(view + 0x38 + 4 * i, word)
    app = m.alloc(0x200)
    m.w(0xb3653c, app)
    m.w(app + 0xd4, c['seed'])
    for i, word in enumerate(c['globals']):
        m.w(0xb43cf8 + 4 * i, word)
    allocs = []

    def base(m):
        events.append(['457A40', m.ecx() == view])
        m.ret(0x87654301)

    def alloc(m):
        size = m.arg(1)
        events.append(['6F7810', size])
        if c.get('alloc_null'):
            m.ret(0)
            return
        p = m.alloc(size + 16)
        m.u.mem_write(p, bytes([fill]) * (size + 16))
        allocs.append(p)
        m.ret(p)

    m.seam(0x457a40, base)
    m.seam(0x6f7810, alloc)
    fault = None
    result = None
    try:
        result = m.call(0x451f90, ecx=view) & 0xff
    except AssertionError:
        assert m.null_accesses, 'only the NULL Track allocation may stop the run'
        fault = [hex(v) for v in m.null_accesses[0]]
    m.verify_v14()
    out = {'result': result, 'fault': fault, 'events': events, 'view': view, 'infos': infos}
    out['fields'] = {hex(o): m.r(view + o) for o in (0x144, 0x148, 0x14c)}
    out['seed'] = m.r(app + 0xd4)
    if allocs:
        p = allocs[0]
        assert out['fields']['0x144'] == p + 4
        out['fields']['0x144'] = 'allocation+4'
        out['header'] = m.r(p)
        out['tracks'] = [bytes(m.u.mem_read(p + 4 + i * TRACK, TRACK)) for i in range(c['count'])]
        # The block's 16 trailing transport bytes stay the allocator's.
        out['tail'] = bytes(m.u.mem_read(p + 4 + c['count'] * TRACK, 16))
    else:
        out['header'] = None
        out['tracks'] = []
        out['tail'] = b''
    return out, m.visited


def merge(a, b):
    """Known byte where both fills agree, None where native left it unwritten."""
    return [x if x == y else None for x, y in zip(a, b)]


def info(t, mirrored, pcg, texture, off, size, fade):
    return [t, mirrored, pcg, texture, off[0], off[1], size[0], size[1], fade]


def cases():
    special = [0x3eaaa9fc, 0x80000000, 0x00000001, 0x00000003, 0x00800000, 0x00800001, 0x7f7fffff,
               0x7f800000, 0xff800000, 0x7fc00001, 0xffc12345, 0x3e800000, 0x00000000, 0x80000003]
    out = []
    out.append(dict(name='one-continuous', count=1, globals=[0x41200000, 0xc2c80000, 0x3f800000],
                    infos=[info(1, 0xdeadbe01, 0x3f266666, 0x7e001000, (0x3ecccccd, 0xbecccccd),
                                (0x3eb33333, 0x3eb33333), 0x40800000)]))
    out.append(dict(name='biped-pair', count=2, globals=[0, 0x80000000, 0x7fc00000],
                    infos=[info(0, 0x00000001, 0x3f000000, 0x7e002000, (0x3dcccccd, 0xbdcccccd),
                                (0x3e800000, 0x3e800000), 0x40800000),
                           info(0, 0xffffff00, 0x3f800000, 0x7e002000, (0xbdcccccd, 0xbdcccccd),
                                (0x3e800000, 0x3e800000), 0x40800000)]))
    specials = []
    for i in range(0, len(special), 2):
        specials.append(info(0x12345678 + i, 0x5a5a5aff if i % 4 else 0x000000a5, special[(i + 3) % len(special)],
                             0 if i % 3 == 0 else 0x7e003000 + i, (special[(i + 5) % len(special)], 0xffffffff),
                             (special[i], special[i + 1]), special[(i + 7) % len(special)]))
    out.append(dict(name='special-sizes', count=len(specials), globals=[0x7f800000, 0x00000001, 0xbf800000],
                    infos=specials))
    out.append(dict(name='three', count=3, globals=[0x3f800000, 0x40000000, 0x40400000],
                    infos=[info(1, 0, 0x3f59999a, 0x7e004000 + 16 * i, (0x3ec28f5c, 0x3f800000),
                                (0x3eb33333 + i, 0x3eb33333 - i), 0x40a00000) for i in range(3)]))
    out.append(dict(name='signalling-nan-sizes', count=2, globals=[1, 2, 3],
                    infos=[info(1, 1, 0x7f800001, 0x7e005000, (0x7f800001, 0xff800123), (0x7f800001, 0xff800123), 0x7fbfffff),
                           info(1, 1, 0, 0x7e005010, (0, 0), (0x7fbfffff, 0x3f800000), 0)]))
    out.append(dict(name='allocation-null', count=1, alloc_null=True, globals=[0x41200000, 0xc2c80000, 0x3f800000],
                    infos=[info(1, 1, 0x3f266666, 0x7e001000, (0, 0), (0x3eb33333, 0x3eb33333), 0x40800000)]))
    for c in out:
        c.setdefault('alloc_null', False)
        c['seed'] = 0x8e4b12a5
        c['pose'] = [0x3f800001, 0x80000000, 0x47ffffff, 0xbf19999a, 0x3f4ccccd, 0]
    return out


def main():
    rows = []
    visited = {}
    for precision in ('Single24', 'Double53', 'Extended64'):
        for c in cases():
            a, va = run(precision, c, FILLS[0])
            b, vb = run(precision, c, FILLS[1])
            visited.update(va)
            visited.update(vb)
            for key in ('result', 'fault', 'events', 'seed', 'header', 'view', 'infos'):
                assert a[key] == b[key], (c['name'], key)
            # A field the run never reached keeps its fill byte: unwritten.
            fields = {k: (v if v == b['fields'][k] else None) for k, v in a['fields'].items()}
            tracks = [merge(x, y) for x, y in zip(a['tracks'], b['tracks'])]
            assert all(v is None for v in merge(a['tail'], b['tail']))
            snan = any(w in SNAN for words in c['infos'] for w in words[6:8])
            rows.append(dict(precision=precision, input=c, emulator_snan=snan,
                             output=dict(result=a['result'], fault=a['fault'], events=a['events'], fields=fields,
                                         seed=a['seed'], header=a['header'], view=a['view'], tracks=tracks)))
    # The positive Track tail is precision independent (exact power-of-two
    # product, m32 store rounding only).
    by = {}
    for r in rows:
        by.setdefault(r['input']['name'], []).append(r['output'])
    for name, outs in by.items():
        assert all(o == outs[0] for o in outs), name
    data_pins = []
    for va, size in [(0x9fbdb8, 4), (0x9fda98, 8), (0x9fdaa0, 8), (0xa05540 + 0x38, 4)]:
        raw = original.range(va, size)
        assert raw == modified.range(va, size)
        data_pins.append(dict(va=hex(va), bytes=raw.hex()))
    doc = header(__file__, __doc__)
    doc.update(rows=rows, instructions=instructions(visited), data_pins=data_pins)
    out = HERE / 'native-actor-tracks.json'
    text = json.dumps(doc, indent=1) + '\n'
    if '--write' in sys.argv:
        out.write_text(text)
    else:
        assert json.loads(out.read_text()) == json.loads(text), 'fixture differs from a fresh oracle run'
    print(json.dumps(dict(rows=len(rows), instructions=len(doc['instructions']),
                          sha256=hashlib.sha256(out.read_bytes()).hexdigest())))


if __name__ == '__main__':
    main()
