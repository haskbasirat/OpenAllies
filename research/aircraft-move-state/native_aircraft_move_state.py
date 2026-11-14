"""516D70 / 516DD0 / 518010 aircraft move-state helpers, from the original AA.exe.

Seams: the flight calls 55EB30 (formation join), 55F160 (follow) and 55E1B0
(path clear), each recording its this-pointer and arguments. 529C30, the
state words, Mind +1D4, the owner's +6C Mover +28 and the GTID table
[B364C0] (live, stale and null +14 handles) are real memory. Usage:
    python native_aircraft_move_state.py [path-to-AA.exe] [--check]
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

GROUP = 0x2c00100
LEADER = 0x2c00200
INDEX = 0x321


def cases():
    out = []
    for group in [True, False]:
        for initial in [0, 1, 0x80]:
            for flag in [0, 1, 2, 0xff]:
                out.append(dict(function='516d70', group=group, initial=initial, flag=flag))
            for leader in [False, True]:
                out.append(dict(function='516dd0', group=group, initial=initial, leader=leader))
            for handle in ['live', 'stale', 'null']:
                out.append(dict(function='518010', group=group, initial=initial, handle=handle))
    return out


def run(m, table, c):
    # Objects are allocated after the GTID table (main resets m.next).
    calls = []
    state = m.alloc(0x60)
    mind = m.alloc(0x200)
    actor = m.alloc(0x100)
    flight = m.alloc(0x100)
    mover = m.alloc(0x40)
    target = m.alloc(0x40)
    m.w32(state + 4, mind)
    m.w32(mind + 4, actor)
    m.w32(actor + 0xbc, flight)
    m.w32(actor + 0x6c, mover)
    m.w32(mover + 0x28, h.f2b(37.5))
    m.w32(mind + 0x1d4, GROUP if c['group'] else 0)
    m.w8(state + 0x48, c['initial'])
    m.w32(state + 0xc, 0x77)
    m.w32(state + 0x34, 0x11111111)
    m.w32(state + 0x38, 0x22222222, 0x33333333)
    m.w32(target + 0x20, h.f2b(-12.25), h.f2b(640.5))
    generation = 0x45
    m.u.mem_write(table + 4 + INDEX * 2, generation.to_bytes(2, 'little'))
    m.w32(table + 0x20004 + INDEX * 4, 0 if c.get('handle') == 'null' else target)
    handle_generation = generation ^ (1 if c.get('handle') == 'stale' else 0)
    m.w32(state + 0x14, (handle_generation << 16) | INDEX)

    def join(mm):
        assert mm.ecx() == flight
        calls.append(['join', mm.arg()])
        mm.ret(4)

    def follow(mm):
        assert mm.ecx() == flight
        calls.append(['follow', mm.arg(), mm.arg(1)])
        mm.ret(8)

    def clear(mm):
        assert mm.ecx() == flight + 0x28
        calls.append(['clear_path'])
        mm.ret(0)
    m.seams[0x55eb30] = join
    m.seams[0x55f160] = follow
    m.seams[0x55e1b0] = clear
    f = c['function']
    if f == '516d70':
        m.call(0x516d70, ecx=state, args=(c['flag'],))
        assert m.final_esp == h.STACK + 8
    elif f == '516dd0':
        m.call(0x516dd0, ecx=state, args=(LEADER if c['leader'] else 0,))
        assert m.final_esp == h.STACK + 8
    else:
        m.call(0x518010, ecx=state)
        assert m.final_esp == h.STACK + 4
    return dict(input=c, calls=calls, byte_48=m.r8(state + 0x48), word_c=m.r32(state + 0xc),
                word_34=m.r32(state + 0x34), words_38=m.r32(state + 0x38, 2))


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
                    row = run(m, table, c)
                    rows.append(dict(image=label, precision=precision, **row))
    finally:
        h.P = original
    half = len(rows) // 2
    strip = lambda r: {k: v for k, v in r.items() if k != 'image'}
    assert [strip(r) for r in rows[:half]] == [strip(r) for r in rows[half:]]
    doc = dict(scope=__doc__.splitlines()[0], pairs=half, executions=len(rows), rows=rows,
               group=GROUP, leader=LEADER, mover_28=h.f2b(37.5),
               target_position=[h.f2b(-12.25), h.f2b(640.5)],
               original_sha256=h.ORIGINAL_SHA, v14_sha256=h.V14_SHA,
               producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
               seams=[hex(a) for a in [0x55eb30, 0x55f160, 0x55e1b0]],
               observed_pins=[dict(address=hex(a), bytes=b) for a, b in sorted(h.VISITED.items())])
    path = HERE / 'native-aircraft-move-state.json'
    payload = (json.dumps(doc, indent=1) + '\n').encode()
    if '--check' in sys.argv:
        assert path.read_bytes() == payload
    else:
        path.write_bytes(payload)
    print(json.dumps(dict(result='PASS', pairs=half, executions=len(rows), pins=len(h.VISITED),
                          sha256=hashlib.sha256(payload).hexdigest())))


if __name__ == '__main__':
    main()
