"""51B0C0 aircraft attack-state aim point, executed from the original AA.exe.

Seams: the owner's virtual +18 (speed) and +104 (reach), the target's
virtual +18 (speed), all scripted binary64 results on ST0, and the plan
51A920 (reached by a tail jump), recorded. The GTID table [B364C0], 529C30
and every state, owner, flight and target field are real memory. Usage:
    python native_attack_aim.py [path-to-AA.exe] [--check]
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

INDEX = 0x1d
GENERATION = 0x03
GTID = (GENERATION << 16) | INDEX
VSTUBS = h.STUBS + 0x50000


def d2b(x):
    return struct.unpack('<Q', struct.pack('<d', x))[0]


def cases():
    rng = random.Random(0x51b0c0)
    out = []
    for i in range(800):
        target = [rng.uniform(-600, 600), rng.uniform(-600, 600)]
        owner = [target[0] + rng.uniform(-300, 300), target[1] + rng.uniform(-300, 300)]
        angle = rng.uniform(-math.pi, math.pi)
        owner_speed = rng.uniform(5.0, 120.0)
        target_speed = 0.0 if i % 5 == 0 else rng.uniform(0.0, 40.0)
        t = math.hypot(target[0] - owner[0], target[1] - owner[1]) / owner_speed
        aim_guess = [target[0] + math.cos(angle) * target_speed * t,
                     target[1] + math.sin(angle) * target_speed * t]
        if i % 3 == 0:
            previous = [h.f2b(aim_guess[0] + rng.uniform(-30, 30)), h.f2b(aim_guess[1] + rng.uniform(-30, 30))]
        else:
            previous = [h.f2b(rng.uniform(-600, 600)), h.f2b(rng.uniform(-600, 600))]
        out.append(dict(
            target=[h.f2b(target[0]), h.f2b(target[1])], owner=[h.f2b(owner[0]), h.f2b(owner[1])],
            heading=[h.f2b(math.cos(angle)), h.f2b(math.sin(angle))], previous=previous,
            owner_speed=d2b(owner_speed), target_speed=d2b(target_speed),
            reach=d2b(8000.0 if i % 3 == 0 and i % 2 == 0 else
                      rng.choice([rng.uniform(0, 50), rng.uniform(0, 5000), 400.0])),
            flight_90=rng.choice([0, 1]), flight_34=rng.choice([0, 3]), pending=rng.choice([0, 1])))
    return out


def run(m, table, c):
    # Objects are allocated after the GTID table (main resets m.next).
    calls = []
    state = m.alloc(0x60)
    mind = m.alloc(0x200)
    owner = m.alloc(0x100)
    flight = m.alloc(0x100)
    target = m.alloc(0x100)
    owner_vt = m.alloc(0x200)
    target_vt = m.alloc(0x40)
    m.w32(state + 4, mind)
    m.w32(mind + 4, owner)
    m.w32(owner, owner_vt)
    m.w32(owner_vt + 0x18, VSTUBS)
    m.w32(owner_vt + 0x104, VSTUBS + 0x10)
    m.w32(target, target_vt)
    m.w32(target_vt + 0x18, VSTUBS + 0x20)
    m.w32(owner + 0x20, *c['owner'])
    m.w32(owner + 0xbc, flight)
    m.w8(flight + 0x90, c['flight_90'])
    m.w32(flight + 0x34, c['flight_34'])
    m.w32(target + 0x20, *c['target'])
    m.w32(target + 0x2c, *c['heading'])
    m.w8(state + 0x14, c['pending'])
    m.w32(state + 0x18, GTID)
    m.w32(state + 0x24, 0x1111, 0x2222, 0x3333)
    m.w32(state + 0x3c, *c['previous'])
    m.u.mem_write(table + 4 + INDEX * 2, GENERATION.to_bytes(2, 'little'))
    m.w32(table + 0x20004 + INDEX * 4, target)

    def virtual(name, this, key):
        def seam(mm):
            assert mm.ecx() == this
            calls.append([name])
            mm.ret_float(c[key], 'f64')
        return seam

    def plan(mm):
        assert mm.ecx() == state
        calls.append(['plan'])
        mm.ret(0)
    m.seams[VSTUBS] = virtual('owner_speed', owner, 'owner_speed')
    m.seams[VSTUBS + 0x10] = virtual('reach', owner, 'reach')
    m.seams[VSTUBS + 0x20] = virtual('target_speed', target, 'target_speed')
    m.seams[0x51a920] = plan
    m.call(0x51b0c0, ecx=state)
    assert m.final_esp == h.STACK + 4 and m.fpu_top() == 0
    return dict(input=c, calls=calls, aim=m.r32(state + 0x24, 3))


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
               seams=['owner virtual +18/+104', 'target virtual +18', hex(0x51a920)],
               observed_pins=[dict(address=hex(a), bytes=b) for a, b in sorted(h.VISITED.items())])
    path = HERE / 'native-attack-aim.json'
    payload = (json.dumps(doc, indent=1) + '\n').encode()
    if '--check' in sys.argv:
        assert path.read_bytes() == payload
    else:
        path.write_bytes(payload)
    print(json.dumps(dict(result='PASS', pairs=half, executions=len(rows), pins=len(h.VISITED),
                          sha256=hashlib.sha256(payload).hexdigest())))


if __name__ == '__main__':
    main()
