"""517360 aircraft attack-state point activation, executed from the original AA.exe.

Seams: the map test 4282A0 and clamp 552640 on [B36534]+30, the ground
height 4F0E50 (scripted binary64 on ST0, recording the point it reads), the
move-state calls 516D70 / 516BA0 and the activation 52A810 (scripted AL),
each recording its this-pointer and arguments. 529C30 and the state, Mind
and owner fields are real memory. Usage:
    python native_attack_point.py [path-to-AA.exe] [--check]
"""
from pathlib import Path
import hashlib, json, os, random, struct, sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
ARGS = [a for a in sys.argv[1:] if not a.startswith('--')]
if ARGS:
    os.environ['AA_GAME_DIR'] = str(Path(ARGS[0]).resolve().parent)
sys.path.insert(0, str(ROOT / 'research/phase8-movement'))
import harness as h  # noqa: E402


def d2b(x):
    return struct.unpack('<Q', struct.pack('<d', x))[0]


def cases():
    rng = random.Random(0x517360)
    out = []
    for i in range(240):
        out.append(dict(performer=i % 9 != 0, inside=rng.random() < 0.6,
                        point=[h.f2b(rng.uniform(-900, 900)), h.f2b(rng.uniform(-900, 900))],
                        clamped=[h.f2b(rng.uniform(0, 500)), h.f2b(rng.uniform(0, 500))],
                        height=d2b(rng.choice([0.0, rng.uniform(-20, 200), 1.0000000001])),
                        mode=rng.choice([1, 2, 7]), formation=rng.choice([0, 1, 0xff]),
                        al=rng.choice([0, 1])))
    return out


def run(m, c):
    m.next = h.HEAP + 0x1000
    calls = []
    state = m.alloc(0x60)
    mind = m.alloc(0x200)
    owner = m.alloc(0x100)
    world = m.alloc(0x100)
    gmap = m.alloc(0x100)
    m.w32(state + 4, mind)
    m.w32(mind + 4, owner)
    m.w32(owner + 0x74, m.alloc(0x40) if c['performer'] else 0)
    m.w32(0xb36534, world)
    m.w32(world + 0x30, gmap)
    m.w32(state + 0x14, 0x01010101)
    m.w32(state + 0x20, 0x01010101)
    m.w32(state + 0x24, 0x1111, 0x2222, 0x3333)
    m.w32(state + 0x30, 0x7777)
    m.w32(state + 0x3c, 0x4444, 0x5555)

    def contains(mm):
        assert mm.ecx() == gmap
        calls.append(['contains', mm.arg(), mm.arg(1)])
        mm.ret(8, int(c['inside']))

    def clamp(mm):
        assert mm.ecx() == gmap
        p = mm.arg()
        calls.append(['clamp', mm.r32(p, 2)])
        mm.w32(p, *c['clamped'])
        mm.ret(4)

    def height(mm):
        assert mm.ecx() == world and mm.arg(1) == 0x123
        calls.append(['height', mm.r32(mm.arg(), 2), mm.r32(state + 0x24, 3)])
        mm.ret_float(c['height'], 'f64', 8)

    def formation(mm):
        assert mm.ecx() == mind + 0x34
        calls.append(['set_formation', mm.arg() & 0xff])
        mm.ret(4)

    def retarget(mm):
        assert mm.ecx() == mind + 0x34
        calls.append(['retarget', mm.arg(), mm.arg(1), mm.arg(2)])
        mm.ret(0xc, 1)

    def activate(mm):
        assert mm.ecx() == state
        calls.append(['activate', mm.arg(), m.r8(state + 0x14), m.r8(state + 0x20), m.r32(state + 0x30)])
        mm.ret(4, c['al'])
    m.seams[0x4282a0] = contains
    m.seams[0x552640] = clamp
    m.seams[0x4f0e50] = height
    m.seams[0x516d70] = formation
    m.seams[0x516ba0] = retarget
    m.seams[0x52a810] = activate
    eax = m.call(0x517360, ecx=state, args=(c['mode'], *c['point'], c['formation']))
    assert m.final_esp == h.STACK + 0x14 and m.fpu_top() == 0
    return dict(input=c, calls=calls, al=eax & 0xff, point=m.r32(state + 0x24, 3),
                previous=m.r32(state + 0x3c, 2))


def main():
    original = h.P
    rows = []
    try:
        for label, image in [('original', original), ('v14', h.V)]:
            h.P = image
            for precision in h.PRECISION:
                m = h.Machine(precision)
                for c in cases():
                    rows.append(dict(image=label, precision=precision, **run(m, c)))
    finally:
        h.P = original
    half = len(rows) // 2
    strip = lambda r: {k: v for k, v in r.items() if k != 'image'}
    assert [strip(r) for r in rows[:half]] == [strip(r) for r in rows[half:]]
    doc = dict(scope=__doc__.splitlines()[0], pairs=half, executions=len(rows), rows=rows,
               original_sha256=h.ORIGINAL_SHA, v14_sha256=h.V14_SHA,
               producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
               seams=[hex(a) for a in [0x4282a0, 0x552640, 0x4f0e50, 0x516d70, 0x516ba0, 0x52a810]],
               observed_pins=[dict(address=hex(a), bytes=b) for a, b in sorted(h.VISITED.items())])
    path = HERE / 'native-attack-point.json'
    payload = (json.dumps(doc, indent=1) + '\n').encode()
    if '--check' in sys.argv:
        assert path.read_bytes() == payload
    else:
        path.write_bytes(payload)
    print(json.dumps(dict(result='PASS', pairs=half, executions=len(rows), pins=len(h.VISITED),
                          sha256=hashlib.sha256(payload).hexdigest())))


if __name__ == '__main__':
    main()
