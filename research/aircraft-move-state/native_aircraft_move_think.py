"""51A720 aircraft move-state Think, executed from the original AA.exe.

Seams: the Think base 529E60, the group leader 5685B0, the target follow
518A30 (which rewrites the destination +18/+1C so the later re-reads are
observed), the slot offset 5685E0 and the flight calls 560F10 / 560ED0
(560ED0 also rewrites +18), each recording its this-pointer and arguments.
529C30, 432E40, 4CF1B0, the GTID table [B364C0] and every state, Mind,
actor and flight field are real memory. Usage:
    python native_aircraft_move_think.py [path-to-AA.exe] [--check]
"""
from pathlib import Path
import hashlib, itertools, json, os, sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
ARGS = [a for a in sys.argv[1:] if not a.startswith('--')]
if ARGS:
    os.environ['AA_GAME_DIR'] = str(Path(ARGS[0]).resolve().parent)
sys.path.insert(0, str(ROOT / 'research/phase8-movement'))
import harness as h  # noqa: E402

INDEX = 0x77
GENERATION = 0x31
GTID = (GENERATION << 16) | INDEX
ACTOR_GTID = 0x00ab0042
DEST = [h.f2b(100.5), h.f2b(-20.25), h.f2b(30.0)]
DIRECTION = [h.f2b(0.6), h.f2b(0.8), 0]
FOLLOWED = [h.f2b(7.5), h.f2b(-8.25)]
FLOWN = h.f2b(55.5)
OFFSET = [h.f2b(-3.0), h.f2b(4.5), h.f2b(0.25)]
CRUISE = h.f2b(42.0)


def cases():
    out = []
    for formation, group, leader, gtid, f90, f34, dest, direction in itertools.product(
            [0, 1], [True, False], ['self', 'other', 'null'], ['zero', 'live', 'stale', 'null'],
            [0, 1], [0, 5], [0, 1], [0, 1]):
        out.append(dict(formation=formation, group=group, leader=leader, gtid=gtid,
                        flight_90=f90, flight_34=f34, has_destination=dest,
                        has_direction=direction))
    return out


def run(m, table, c):
    # Objects are allocated after the GTID table (main resets m.next).
    calls = []
    state = m.alloc(0x60)
    mind = m.alloc(0x200)
    actor = m.alloc(0x100)
    flight = m.alloc(0x100)
    group = m.alloc(0x40)
    other = m.alloc(0x100)
    m.w32(state + 4, mind)
    m.w32(mind + 4, actor)
    m.w32(mind + 0x1d4, group if c['group'] else 0)
    m.w32(actor + 0xbc, flight)
    m.w32(actor + 0x14, ACTOR_GTID)
    m.w8(flight + 0x90, c['flight_90'])
    m.w32(flight + 0x34, c['flight_34'])
    m.w32(flight + 0x14, 0x1111, 0)
    m.w32(flight + 0x84, 0x2222)
    m.w8(state + 0x10, c['has_destination'])
    m.w32(state + 0x14, {'zero': 0, 'live': GTID, 'stale': GTID ^ 0x10000, 'null': GTID}[c['gtid']])
    m.w32(state + 0x18, *DEST, *DIRECTION)
    m.w8(state + 0x30, c['has_direction'])
    m.w32(state + 0x34, CRUISE)
    m.w32(state + 0x40, 0x4444, 0x5555)
    m.w8(state + 0x48, c['formation'])
    m.u.mem_write(table + 4 + INDEX * 2, GENERATION.to_bytes(2, 'little'))
    m.w32(table + 0x20004 + INDEX * 4, 0 if c['gtid'] == 'null' else 0x2c00500)

    def base(mm):
        assert mm.ecx() == state
        calls.append(['think_base'])
        mm.ret(0)

    def leader(mm):
        assert mm.ecx() == group
        calls.append(['leader'])
        mm.ret(0, {'self': actor, 'other': other, 'null': 0}[c['leader']])

    def follow(mm):
        assert mm.ecx() == state and mm.arg() == 0
        calls.append(['follow_target'])
        mm.w32(state + 0x18, *FOLLOWED)
        mm.ret(4)

    def offset(mm):
        assert mm.ecx() == group and mm.arg(2) == 1
        calls.append(['offset', mm.arg(1)])
        mm.w32(mm.arg(), *OFFSET)
        mm.ret(0xc)

    def direction(pointer):
        assert pointer in (0, state + 0x24)
        return pointer == state + 0x24

    def formation(mm):
        assert mm.ecx() == flight
        a = [mm.arg(i) for i in range(7)]
        calls.append(['fly_formation', a[0:3], a[3:6], direction(a[6])])
        mm.ret(0x1c)

    def fly(mm):
        assert mm.ecx() == flight
        a = [mm.arg(i) for i in range(4)]
        calls.append(['fly_to', a[0:3], direction(a[3]), mm.r32(flight + 0x14), mm.r32(flight + 0x84)])
        mm.w32(state + 0x18, FLOWN)
        mm.ret(0x10)
    m.seams[0x529e60] = base
    m.seams[0x5685b0] = leader
    m.seams[0x518a30] = follow
    m.seams[0x5685e0] = offset
    m.seams[0x560f10] = formation
    m.seams[0x560ed0] = fly
    m.call(0x51a720, ecx=state)
    assert m.final_esp == h.STACK + 4 and m.fpu_top() == 0
    m.w32(table + 0x20004 + INDEX * 4, 0)
    return dict(input=c, calls=calls, commanded=m.r32(state + 0x40, 2),
                flight_14=m.r32(flight + 0x14), flight_84=m.r32(flight + 0x84))


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
               gtid=GTID, actor_gtid=ACTOR_GTID, destination=DEST, followed=FOLLOWED,
               flown=FLOWN, offset=OFFSET, cruise=CRUISE,
               original_sha256=h.ORIGINAL_SHA, v14_sha256=h.V14_SHA,
               producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
               seams=[hex(a) for a in [0x529e60, 0x5685b0, 0x518a30, 0x5685e0, 0x560f10, 0x560ed0]],
               observed_pins=[dict(address=hex(a), bytes=b) for a, b in sorted(h.VISITED.items())])
    path = HERE / 'native-aircraft-move-think.json'
    payload = (json.dumps(doc, indent=1) + '\n').encode()
    if '--check' in sys.argv:
        assert path.read_bytes() == payload
    else:
        path.write_bytes(payload)
    print(json.dumps(dict(result='PASS', pairs=half, executions=len(rows), pins=len(h.VISITED),
                          sha256=hashlib.sha256(payload).hexdigest())))


if __name__ == '__main__':
    main()
