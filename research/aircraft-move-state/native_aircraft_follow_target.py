"""518A30 aircraft move-state target follow, executed from the original AA.exe.

Seams: the owner's and the object's virtual 0 (radius) and +18 (speed),
returning scripted binary64 values on ST0, and the re-target 516BA0,
recording its arguments. The GTID table [B364C0], 529C30 and every state,
owner and object field are real memory. Usage:
    python native_aircraft_follow_target.py [path-to-AA.exe] [--check]
"""
from pathlib import Path
import hashlib, json, math, os, random, struct, sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
ARGS = [a for a in sys.argv[1:] if not a.startswith('--')]
if ARGS:
    os.environ['AA_GAME_DIR'] = str(Path(ARGS[0]).resolve().parent)
sys.path.insert(0, str(ROOT / 'research/phase8-movement'))
import harness as h  # noqa: E402

INDEX = 0x99
GENERATION = 0x0b
GTID = (GENERATION << 16) | INDEX
VSTUBS = h.STUBS + 0x70000


def d2b(x):
    return struct.unpack('<Q', struct.pack('<d', x))[0]


def cases():
    rng = random.Random(0x518a30)
    out = []
    for i in range(900):
        obj = [rng.uniform(-500, 500), rng.uniform(-500, 500)]
        owner = [obj[0] + rng.uniform(-200, 200), obj[1] + rng.uniform(-200, 200)]
        angle = rng.uniform(-math.pi, math.pi)
        heading = [h.f2b(math.cos(angle)), h.f2b(math.sin(angle))]
        kind = i % 6
        if kind == 0:
            dest = [h.f2b(obj[0]), h.f2b(obj[1]), 0x80000000 if i % 12 == 0 else 0]
        elif kind == 1:
            dest = [h.f2b(obj[0]), h.f2b(obj[1] + 1.0), 0]
        else:
            dest = [h.f2b(rng.uniform(-500, 500)), h.f2b(rng.uniform(-500, 500)), h.f2b(rng.choice([0.0, 25.0]))]
        if i % 5 == 0:
            commanded = [h.f2b(obj[0] + rng.uniform(-0.8, 0.8)), h.f2b(obj[1] + rng.uniform(-0.6, 0.6))]
        else:
            commanded = [h.f2b(rng.uniform(-500, 500)), h.f2b(rng.uniform(-500, 500))]
        out.append(dict(
            force=int(i % 7 == 0),
            object=[h.f2b(obj[0]), h.f2b(obj[1])], owner=[h.f2b(owner[0]), h.f2b(owner[1])],
            heading=heading, destination=dest, commanded=commanded,
            owner_radius=d2b(rng.choice([1.0, 2.5, 0.1 + rng.random(), 150.0])),
            object_radius=d2b(rng.choice([1.0, 3.0, rng.random() * 4, 250.0])),
            object_speed=d2b(0.0 if i % 4 == 0 else rng.uniform(0.5, 60.0)),
            owner_speed=d2b(rng.uniform(5.0, 90.0))))
    return out


def run(m, table, c):
    # Objects are allocated after the GTID table (main resets m.next).
    calls = []
    state = m.alloc(0x60)
    mind = m.alloc(0x200)
    owner = m.alloc(0x100)
    obj = m.alloc(0x100)
    owner_vt = m.alloc(0x40)
    obj_vt = m.alloc(0x40)
    m.w32(state + 4, mind)
    m.w32(mind + 4, owner)
    m.w32(owner, owner_vt)
    m.w32(obj, obj_vt)
    m.w32(owner_vt, VSTUBS)
    m.w32(owner_vt + 0x18, VSTUBS + 0x10)
    m.w32(obj_vt, VSTUBS + 0x20)
    m.w32(obj_vt + 0x18, VSTUBS + 0x30)
    m.w32(owner + 0x20, *c['owner'])
    m.w32(obj + 0x20, *c['object'])
    m.w32(obj + 0x2c, *c['heading'])
    m.w32(state + 0x14, GTID)
    m.w32(state + 0x18, *c['destination'])
    m.w32(state + 0x38, 0x3333, 0x4444)
    m.w32(state + 0x40, *c['commanded'])
    m.u.mem_write(table + 4 + INDEX * 2, GENERATION.to_bytes(2, 'little'))
    m.w32(table + 0x20004 + INDEX * 4, obj)

    def virtual(name, this, key):
        def seam(mm):
            assert mm.ecx() == this
            calls.append([name])
            mm.ret_float(c[key], 'f64')
        return seam

    def retarget(mm):
        assert mm.ecx() == state
        calls.append(['retarget', mm.arg(), mm.arg(1), mm.arg(2) == obj])
        mm.ret(0xc, 1)
    m.seams[VSTUBS] = virtual('owner_radius', owner, 'owner_radius')
    m.seams[VSTUBS + 0x10] = virtual('owner_speed', owner, 'owner_speed')
    m.seams[VSTUBS + 0x20] = virtual('object_radius', obj, 'object_radius')
    m.seams[VSTUBS + 0x30] = virtual('object_speed', obj, 'object_speed')
    m.seams[0x516ba0] = retarget
    eax = m.call(0x518a30, ecx=state, args=(c['force'],))
    assert m.final_esp == h.STACK + 8 and m.fpu_top() == 0 and eax == obj
    return dict(input=c, calls=calls, followed=m.r32(state + 0x38, 2))


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
               original_sha256=h.ORIGINAL_SHA, v14_sha256=h.V14_SHA,
               producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
               seams=['owner/object virtual 0 and +18', hex(0x516ba0)],
               observed_pins=[dict(address=hex(a), bytes=b) for a, b in sorted(h.VISITED.items())])
    path = HERE / 'native-aircraft-follow-target.json'
    payload = (json.dumps(doc, indent=1) + '\n').encode()
    if '--check' in sys.argv:
        assert path.read_bytes() == payload
    else:
        path.write_bytes(payload)
    print(json.dumps(dict(result='PASS', pairs=half, executions=len(rows), pins=len(h.VISITED),
                          sha256=hashlib.sha256(payload).hexdigest())))


if __name__ == '__main__':
    main()
