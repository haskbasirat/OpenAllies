"""516FD0 escort and 518170 guard activations, from the original AA.exe.

Seams: the move-state calls 516D70 / 516DD0 (this-pointer must be the Mind's
+34 state), the state activation 52A810 (scripted AL) and the leader's
deleting destructor (virtual 0), each recording its arguments. The target's
+14 GTID, the GTID table [B364C0] (live, stale, null slots), the state +10
word and the leaders' +4 reference counts are real memory. Usage:
    python native_aircraft_guard_activate.py [path-to-AA.exe] [--check]
"""
from pathlib import Path
import hashlib, json, os, sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
ARGS = [a for a in sys.argv[1:] if not a.startswith('--')]
if ARGS:
    os.environ['AA_GAME_DIR'] = str(Path(ARGS[0]).resolve().parent)
sys.path.insert(0, str(ROOT / 'research/phase8-movement'))
import harness as h  # noqa: E402

INDEX = 0x456
GENERATION = 0x12


def cases():
    out = []
    for target in ['none', 'live', 'stale', 'null', 'zero']:
        for formation in [0, 1, 0xff]:
            for al in [1, 0]:
                out.append(dict(function='516fd0', target=target, formation=formation,
                                mode=[2, 7][formation & 1], al=al))
    for old in ['none', 'a', 'b']:
        for new in ['none', 'a', 'b']:
            for count in [1, 2, 5]:
                out.append(dict(function='518170', old=old, new=new, count=count, mode=2))
    return out


def run(m, table, c):
    # Objects are allocated after the GTID table (main resets m.next).
    calls = []
    state = m.alloc(0x40)
    mind = m.alloc(0x200)
    m.w32(state + 4, mind)
    m.w32(state + 0x10, 0x9999)

    def formation(mm):
        assert mm.ecx() == mind + 0x34
        calls.append(['set_formation', mm.arg() & 0xff])
        mm.ret(4)

    def follow(mm):
        assert mm.ecx() == mind + 0x34
        calls.append(['follow', names.get(mm.arg(), mm.arg())])
        mm.ret(4)

    def activate(mm):
        assert mm.ecx() == state
        calls.append(['activate', mm.arg()])
        mm.ret(4, c.get('al', 1))

    def destroy(mm):
        calls.append(['destroy', names[mm.ecx()], mm.arg()])
        mm.ret(4)
    names = {0: None}
    m.seams[0x516d70] = formation
    m.seams[0x516dd0] = follow
    m.seams[0x52a810] = activate
    m.seams[h.STUBS + 0x380] = destroy
    if c['function'] == '516fd0':
        target = 0
        gtid = (GENERATION << 16) | INDEX
        m.u.mem_write(table + 4 + INDEX * 2, GENERATION.to_bytes(2, 'little'))
        m.w32(table + 0x20004 + INDEX * 4, 0 if c['target'] == 'null' else 0x2c00400)
        if c['target'] != 'none':
            target = m.alloc(0x40)
            handle = {'stale': gtid ^ 0x10000, 'zero': 0}.get(c['target'], gtid)
            m.w32(target + 0x14, handle)
        eax = m.call(0x516fd0, ecx=state, args=(c['mode'], target, c['formation']))
        assert m.final_esp == h.STACK + 0x10
        m.w32(table + 0x20004 + INDEX * 4, 0)
        return dict(input=c, calls=calls, al=eax & 0xff, word_10=m.r32(state + 0x10))
    vt = m.alloc(0x10)
    m.w32(vt, h.STUBS + 0x380)
    leaders = {}
    for name in ['a', 'b']:
        p = m.alloc(0x20)
        m.w32(p, vt)
        m.w32(p + 4, c['count'])
        leaders[name] = p
        names[p] = name
    m.w32(state + 0x10, leaders.get(c['old'], 0))
    m.call(0x518170, ecx=state, args=(c['mode'], leaders.get(c['new'], 0)))
    assert m.final_esp == h.STACK + 0xc
    return dict(input=c, calls=calls, word_10=names[m.r32(state + 0x10)],
                counts={k: m.r32(p + 4) for k, p in leaders.items()})


def main():
    original = h.P
    rows = []
    try:
        for label, image in [('original', original), ('v14', h.V)]:
            h.P = image
            for precision in h.PRECISION:
                m = h.Machine(precision)
                table = m.alloc(0x20004 + 0x10000 * 4)
                m.w32(0xb364c0, table)
                start = m.next
                for c in cases():
                    m.next = start
                    rows.append(dict(image=label, precision=precision, **run(m, table, c)))
    finally:
        h.P = original
    half = len(rows) // 2
    strip = lambda r: {k: v for k, v in r.items() if k != 'image'}
    assert [strip(r) for r in rows[:half]] == [strip(r) for r in rows[half:]]
    doc = dict(scope=__doc__.splitlines()[0], pairs=half, executions=len(rows), rows=rows,
               gtid=(GENERATION << 16) | INDEX,
               original_sha256=h.ORIGINAL_SHA, v14_sha256=h.V14_SHA,
               producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
               seams=['516d70', '516dd0', '52a810', 'leader virtual 0'],
               observed_pins=[dict(address=hex(a), bytes=b) for a, b in sorted(h.VISITED.items())])
    path = HERE / 'native-aircraft-guard-activate.json'
    payload = (json.dumps(doc, indent=1) + '\n').encode()
    if '--check' in sys.argv:
        assert path.read_bytes() == payload
    else:
        path.write_bytes(payload)
    print(json.dumps(dict(result='PASS', pairs=half, executions=len(rows), pins=len(h.VISITED),
                          sha256=hashlib.sha256(payload).hexdigest())))


if __name__ == '__main__':
    main()
