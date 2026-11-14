"""51B290 aircraft attack-state release test, executed from the original AA.exe.

Seam: GetValidAbility 51A260 (scripted class and ability, recording the
slot and aim point it receives). The normalizers 4453A0 / 409230, 529C30,
the GTID table [B364C0] and every state, Mind, owner, flight and target
field are real memory. Usage:
    python native_attack_release.py [path-to-AA.exe] [--check]
"""
from pathlib import Path
import hashlib, json, math, os, random, sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
ARGS = [a for a in sys.argv[1:] if not a.startswith('--')]
if ARGS:
    os.environ['AA_GAME_DIR'] = str(Path(ARGS[0]).resolve().parent)
sys.path.insert(0, str(ROOT / 'research/phase8-movement'))
import harness as h  # noqa: E402

INDEX = 0x2b
GENERATION = 0x09
GTID = (GENERATION << 16) | INDEX
ABILITY = 0x2c00abc


def cases():
    rng = random.Random(0x51b290)
    out = []
    for i in range(900):
        owner = [rng.uniform(-500, 500), rng.uniform(-500, 500), rng.uniform(0, 120)]
        yaw = rng.uniform(-math.pi, math.pi)
        pitch = rng.uniform(-0.6, 0.6)
        heading = [math.cos(yaw) * math.cos(pitch), math.sin(yaw) * math.cos(pitch), math.sin(pitch)]
        reach = rng.uniform(1, 300)
        ahead = rng.random() < 0.6
        angle = yaw + (rng.uniform(-0.3, 0.3) if ahead else rng.uniform(-math.pi, math.pi))
        target = [owner[0] + reach * math.cos(angle), owner[1] + reach * math.sin(angle),
                  rng.uniform(0, 120)]
        if i % 17 == 0:
            target = list(owner)
        aim = [target[0] + rng.uniform(-5, 5), target[1] + rng.uniform(-5, 5), rng.uniform(0, 30)]
        out.append(dict(
            dogfighter=rng.choice([0, 1, 0x80]), live=rng.random() < 0.9,
            owner=[h.f2b(x) for x in owner], heading=[h.f2b(x) for x in heading],
            target=[h.f2b(x) for x in target], aim=[h.f2b(x) for x in aim],
            klass=rng.choice([0, 0, 0, 1, 2, 3]), ability=rng.random() < 0.7,
            object_byte=rng.choice([0, 1])))
    return out


def run(m, table, c):
    calls = []
    state = m.alloc(0x60)
    mind = m.alloc(0x200)
    owner = m.alloc(0x100)
    flight = m.alloc(0x100)
    target = m.alloc(0x100)
    m.w32(state + 4, mind)
    m.w8(mind + 0x1d0, c['dogfighter'])
    m.w32(mind + 4, owner)
    m.w32(owner + 0x20, *c['owner'], *c['heading'])
    m.w32(owner + 0xbc, flight)
    m.w32(flight + 0x94, 0x9494, 0x9898, 0x9c9c, 0xa0a0, 0xa4a4)
    m.w32(target + 0x20, *c['target'])
    m.w8(state + 0x20, c['object_byte'])
    m.w32(state + 0x18, GTID if c['live'] else GTID ^ 0x10000)
    m.w32(state + 0x24, *c['aim'])
    m.u.mem_write(table + 4 + INDEX * 2, GENERATION.to_bytes(2, 'little'))
    m.w32(table + 0x20004 + INDEX * 4, target)
    out = m.alloc(16)
    m.w32(out, 0xdeadbeef)

    def pick(mm):
        assert mm.ecx() == mind and mm.arg(5) == 0
        slot = mm.arg()
        calls.append(['pick', slot == target, [mm.arg(1), mm.arg(2), mm.arg(3)], mm.r32(mm.arg(4))])
        if c['ability']:
            mm.w32(mm.arg(4), ABILITY)
        mm.ret(0x18, c['klass'])
    m.seams[0x51a260] = pick
    if c['dogfighter'] and not c['live']:
        # The dog-fight branch dereferences the null slot: not run.
        return None
    eax = m.call(0x51b290, ecx=state, args=(out,))
    assert m.final_esp == h.STACK + 8 and m.fpu_top() == 0
    m.w32(table + 0x20004 + INDEX * 4, 0)
    return dict(input=c, calls=calls, al=eax & 0xff, out=m.r32(out),
                flight=m.r32(flight + 0x94, 5))


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
                    if row is not None:
                        rows.append(dict(image=label, precision=precision, **row))
    finally:
        h.P = original
    half = len(rows) // 2
    strip = lambda r: {k: v for k, v in r.items() if k != 'image'}
    assert [strip(r) for r in rows[:half]] == [strip(r) for r in rows[half:]]
    doc = dict(scope=__doc__.splitlines()[0], pairs=half, executions=len(rows), rows=rows,
               gtid=GTID, ability=ABILITY, original_sha256=h.ORIGINAL_SHA, v14_sha256=h.V14_SHA,
               producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
               seams=[hex(0x51a260)],
               observed_pins=[dict(address=hex(a), bytes=b) for a, b in sorted(h.VISITED.items())])
    path = HERE / 'native-attack-release.json'
    payload = (json.dumps(doc, indent=1) + '\n').encode()
    if '--check' in sys.argv:
        assert path.read_bytes() == payload
    else:
        path.write_bytes(payload)
    print(json.dumps(dict(result='PASS', pairs=half, executions=len(rows), pins=len(h.VISITED),
                          sha256=hashlib.sha256(payload).hexdigest())))


if __name__ == '__main__':
    main()
