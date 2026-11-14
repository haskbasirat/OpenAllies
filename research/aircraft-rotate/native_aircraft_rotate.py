"""516B30 aircraft attack vector turn, executed from the original AA.exe.

The 4096-entry cosine/sine tables are built by the original startup
constructors 409040 (0xB39A48) and 4090E0 (0xB39A60); the only seam is the
engine allocator 6F7810 they call. 516B30 then runs with its table lookups
85DAA0/85DA80 unhooked. Usage:
    python native_aircraft_rotate.py [path-to-AA.exe] [--check]
"""
from pathlib import Path
import hashlib, json, os, random, sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
ARGS = [a for a in sys.argv[1:] if not a.startswith('--')]
if ARGS:
    os.environ['AA_GAME_DIR'] = str(Path(ARGS[0]).resolve().parent)
sys.path.insert(0, str(ROOT / 'research/phase8-movement'))
import harness as h  # noqa: E402

QUARTER = 0x3f490fdb
MINUS_QUARTER = 0xbf490fdb


def cases():
    rng = random.Random(0x516b30)
    angles = [QUARTER, MINUS_QUARTER, 0, 0x80000000, h.f2b(1.0), h.f2b(-2.5), h.f2b(3.0),
              h.f2b(1.5707964), h.f2b(-7.0), h.f2b(0.001)]
    vectors = [[h.f2b(1.0), 0], [0, h.f2b(1.0)], [h.f2b(0.70710677), h.f2b(-0.70710677)],
               [h.f2b(-3.25), h.f2b(12.5)], [0, 0], [0x80000000, h.f2b(-1.0)]]
    for _ in range(18):
        vectors.append([h.f2b(rng.uniform(-200.0, 200.0)), h.f2b(rng.uniform(-200.0, 200.0))])
    for _ in range(6):
        vectors.append([h.f2b(rng.uniform(-1.0, 1.0)), h.f2b(rng.uniform(-1.0, 1.0))])
    out = []
    for i, v in enumerate(vectors):
        for a in angles:
            out.append(dict(vector=v, angle=a, alias=(i % 5 == 0)))
    return out


def tables(m):
    def allocate(mm):
        p = mm.alloc(mm.arg())
        mm.ret(0, p)
    m.seams[0x6f7810] = allocate
    m.call(0x409040, ecx=0xb39a48, args=(4096,))
    m.call(0x4090e0, ecx=0xb39a60, args=(4096,))
    del m.seams[0x6f7810]
    assert m.r32(0xb39b08) == 0xb39a48 and m.r32(0xb39b10) == 0xb39a60
    return m.next


def run(m, c, heap):
    m.next = heap
    vin = m.alloc(16)
    out = vin if c['alias'] else m.alloc(16)
    m.w32(vin, *c['vector'])
    if not c['alias']:
        m.w32(out, 0xdeadbeef, 0xdeadbeef)
    c_top = m.fpu_top()
    m.call(0x516b30, args=(out, vin, c["angle"]))
    assert m.final_esp == h.STACK + 4, hex(m.final_esp)
    assert m.fpu_top() == c_top, (m.fpu_top(), c_top)
    return dict(input=c, out=m.r32(out, 2))


def main():
    original = h.P
    rows = []
    try:
        for label, image in [('original', original), ('v14', h.V)]:
            h.P = image
            for precision in h.PRECISION:
                m = h.Machine(precision)
                heap = tables(m)
                for c in cases():
                    rows.append(dict(image=label, precision=precision, **run(m, c, heap)))
    finally:
        h.P = original
    half = len(rows) // 2
    strip = lambda r: {k: v for k, v in r.items() if k != 'image'}
    assert [strip(r) for r in rows[:half]] == [strip(r) for r in rows[half:]]
    doc = dict(scope=__doc__.splitlines()[0], pairs=half, executions=len(rows), rows=rows,
               original_sha256=h.ORIGINAL_SHA, v14_sha256=h.V14_SHA,
               producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
               seams=[hex(0x6f7810)],
               observed_pins=[dict(address=hex(a), bytes=b) for a, b in sorted(h.VISITED.items())])
    path = HERE / 'native-aircraft-rotate.json'
    payload = (json.dumps(doc, indent=1) + '\n').encode()
    if '--check' in sys.argv:
        assert path.read_bytes() == payload
    else:
        path.write_bytes(payload)
    print(json.dumps(dict(result='PASS', pairs=half, executions=len(rows), pins=len(h.VISITED),
                          sha256=hashlib.sha256(payload).hexdigest())))


if __name__ == '__main__':
    main()
