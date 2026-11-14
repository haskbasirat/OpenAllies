"""51A260 aircraft GetValidAbility, executed from the original AA.exe.

Seams: the diagnostic scope 77DC50; the admission 4C8F20 (scripted status
per ability, recording the descriptor words it receives); the owner modifier
4CF110; recharge 4C81C0 and range 4C8270 (scripted binary64 results on
ST0); the GWorld RNG 4F02B0 / 85D840 (scripted draws). The descriptor
constructor 4C8300, the ability iteration 517F60 and every field (Performer
array, ability +18 bytes, definition +54 flags, owner +6C and definition
+1E8) are real. Usage:
    python native_ability_pick.py [path-to-AA.exe] [--check]
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

MODIFIER = 0x2c0abcd
RNG = 0x2c0bb00
VALUES = [1.0, 2.0, 0.5, 1.0000000001, 0.99999999999, 3.0, 1.0]
STATUSES = [0, 0, 1, 2, 3, 4, 5, 5, 6, 7, 7, 8, 9, 12]
REGEN = [0, 0x80000000, 0x3f800000, 0x7fc00000]


def d2b(x):
    return struct.unpack('<Q', struct.pack('<d', x))[0]


def cases():
    rng = random.Random(0x51a260)
    out = []
    for i in range(600):
        n = rng.randrange(0, 7)
        abilities = []
        for _ in range(n):
            abilities.append(dict(enabled=rng.random() < 0.85, bit3=rng.random() < 0.6,
                                  status=rng.choice(STATUSES),
                                  recharge=d2b(rng.choice(VALUES[:3] if i % 3 else VALUES)),
                                  range=d2b(rng.choice(VALUES[:3] if i % 2 else VALUES))))
        out.append(dict(abilities=abilities, mover=rng.random() < 0.6, regen=rng.choice(REGEN),
                        last_word=rng.choice([0, 1, 0x100, 0x101, 0xffffff00, 0xffffff01, 2]),
                        words=[rng.getrandbits(32) for _ in range(4)],
                        draws=[rng.getrandbits(32) for _ in range(8)],
                        out_pointer=i % 7 != 0))
    return out


def run(m, c):
    m.next = h.HEAP + 0x1000
    calls = []
    mind = m.alloc(0x200)
    owner = m.alloc(0x100)
    owner_def = m.alloc(0x200)
    performer = m.alloc(0x40)
    m.w32(mind + 4, owner)
    m.w32(owner + 4, owner_def)
    m.w32(owner_def + 0x1e8, c['regen'])
    m.w32(owner + 0x6c, m.alloc(0x40) if c['mover'] else 0)
    m.w32(owner + 0x74, performer)
    n = len(c['abilities'])
    items = m.alloc(4 * max(n, 1))
    pointers = []
    for i, a in enumerate(c['abilities']):
        ability = m.alloc(0x40)
        definition = m.alloc(0x60)
        m.w32(items + 4 * i, ability)
        m.w32(ability + 4, definition)
        m.w8(ability + 0x18, 1 if a['enabled'] else 0)
        m.w32(definition + 0x54, 0x8 if a['bit3'] else 0x7)
        pointers.append(ability)
    m.w32(performer + 0x14, items, n)
    out = m.alloc(16)
    m.w32(out, 0xdeadbeef)
    draws = list(c['draws'])

    def diag(mm):
        mm.ret(8)

    def admission(mm):
        i = pointers.index(mm.ecx())
        d = mm.arg()
        words = mm.r32(d, 12)
        assert mm.arg(1) == 1
        calls.append(['admission', i])
        calls.append(['descriptor', words[0], words[2] == owner, words[7:11]])
        mm.ret(8, c['abilities'][i]['status'])

    def modifier(mm):
        assert mm.ecx() == owner
        calls.append(['modifier'])
        mm.ret(0, MODIFIER)

    def recharge(mm):
        i = pointers.index(mm.ecx())
        calls.append(['recharge', i])
        mm.ret_float(c['abilities'][i]['recharge'], 'f64')

    def range_(mm):
        i = pointers.index(mm.ecx())
        assert mm.arg() == MODIFIER
        calls.append(['range', i])
        mm.ret_float(c['abilities'][i]['range'], 'f64', 4)

    def world_rng(mm):
        calls.append(['rng'])
        mm.ret(0, RNG)

    def draw(mm):
        assert mm.arg() == RNG
        value = draws.pop(0)
        calls.append(['draw', value])
        mm.ret(0, value)
    m.seams[0x77dc50] = diag
    m.seams[0x4c8f20] = admission
    m.seams[0x4cf110] = modifier
    m.seams[0x4c81c0] = recharge
    m.seams[0x4c8270] = range_
    m.seams[0x4f02b0] = world_rng
    m.seams[0x85d840] = draw
    w = c['words']
    eax = m.call(0x51a260, ecx=mind,
                 args=(w[0], w[1], w[2], w[3], out if c['out_pointer'] else 0, c['last_word']))
    assert m.final_esp == h.STACK + 0x1c and m.fpu_top() == 0
    best = m.r32(out)
    best = None if best == 0 else 'untouched' if best == 0xdeadbeef else pointers.index(best)
    return dict(input=c, calls=calls, eax=eax, out=best)


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
               seams=[hex(a) for a in [0x77dc50, 0x4c8f20, 0x4cf110, 0x4c81c0, 0x4c8270,
                                       0x4f02b0, 0x85d840]],
               observed_pins=[dict(address=hex(a), bytes=b) for a, b in sorted(h.VISITED.items())])
    path = HERE / 'native-ability-pick.json'
    payload = (json.dumps(doc, indent=1) + '\n').encode()
    if '--check' in sys.argv:
        assert path.read_bytes() == payload
    else:
        path.write_bytes(payload)
    print(json.dumps(dict(result='PASS', pairs=half, executions=len(rows), pins=len(h.VISITED),
                          sha256=hashlib.sha256(payload).hexdigest())))


if __name__ == '__main__':
    main()
