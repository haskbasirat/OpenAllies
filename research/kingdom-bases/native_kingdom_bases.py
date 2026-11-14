"""4E1D90 Kingdom nearest home base, executed from the original AA.exe.

Seams: the Denizen queries 568120 and 568EA0, each recording which candidate
asked (this-pointer = the candidate's +88) and its argument (the actor's +4
word) and returning a scripted AL. The Kingdom +284 list, the actors' +8
flags, +20/+24 positions, +88 and +C0 words are real memory. Usage:
    python native_kingdom_bases.py [path-to-AA.exe] [--check]
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


def base(rng, x, y):
    return dict(null=False, removed=rng.random() < 0.15, denizen=rng.random() > 0.1,
                c0=rng.random() < 0.5, accepts=rng.random() > 0.2, pos=[h.f2b(x), h.f2b(y)])


def cases():
    rng = random.Random(0x4e1d90)
    out = []
    origin = [h.f2b(10.0), h.f2b(-4.0)]
    # Null and removed actors.
    out.append(dict(actor=None, bases=[]))
    out.append(dict(actor=dict(removed=True, pos=origin), bases=[base(rng, 1.0, 2.0)]))
    out.append(dict(actor=dict(removed=False, pos=origin), bases=[]))
    for i in range(120):
        ax, ay = rng.uniform(-300, 300), rng.uniform(-300, 300)
        bases = []
        for j in range(rng.randrange(1, 7)):
            if rng.random() < 0.1:
                bases.append(dict(null=True))
                continue
            if bases and rng.random() < 0.3:
                prev = next((b for b in reversed(bases) if not b['null']), None)
                if prev is not None:
                    # Equal distance: same point, or mirrored about the actor.
                    px, py = h.b2f(prev['pos'][0]), h.b2f(prev['pos'][1])
                    x, y = (px, py) if rng.random() < 0.5 else (2 * ax - px, 2 * ay - py)
                    bases.append(base(rng, x, y))
                    continue
            bases.append(base(rng, ax + rng.uniform(-80, 80), ay + rng.uniform(-80, 80)))
        if i % 4 == 0:
            for b in bases:
                if not b['null']:
                    b.update(removed=False, denizen=True, accepts=True)
        out.append(dict(actor=dict(removed=False, pos=[h.f2b(ax), h.f2b(ay)]), bases=bases))
    return out


def run(m, c):
    m.next = h.HEAP + 0x1000
    calls = []
    kingdom = m.alloc(0x300)
    actor = 0
    if c['actor'] is not None:
        actor = m.alloc(0x100)
        m.w32(actor + 4, 0x1234abcd)
        m.w32(actor + 8, 1 if c['actor']['removed'] else 0x10)
        m.w32(actor + 0x20, *c['actor']['pos'])
    actors = []
    denizens = {}
    prev = 0
    for i, b in enumerate(c['bases']):
        node = m.alloc(8)
        if prev:
            m.w32(prev + 4, node)
        else:
            m.w32(kingdom + 0x284, node)
        prev = node
        if b['null']:
            actors.append(0)
            continue
        a = m.alloc(0x100)
        actors.append(a)
        m.w32(node, a)
        m.w32(a + 8, 0x21 if b['removed'] else 0x20)
        m.w32(a + 0x20, *b['pos'])
        if b['denizen']:
            d = m.alloc(0x40)
            m.w32(a + 0x88, d)
            denizens[d] = i
        m.w32(a + 0xc0, 0x55 if b['c0'] else 0)

    def query(name):
        def seam(mm):
            i = denizens[mm.ecx()]
            calls.append([name, i, mm.arg()])
            mm.ret(4, int(c['bases'][i]['accepts']))
        return seam
    m.seams[0x568120] = query('568120')
    m.seams[0x568ea0] = query('568ea0')
    eax = m.call(0x4e1d90, ecx=kingdom, args=(actor,))
    assert m.final_esp == h.STACK + 8 and m.fpu_top() == 0
    result = None if eax == 0 else actors.index(eax)
    return dict(input=c, calls=calls, result=result)


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
               seams=[hex(a) for a in [0x568120, 0x568ea0]],
               observed_pins=[dict(address=hex(a), bytes=b) for a, b in sorted(h.VISITED.items())])
    path = HERE / 'native-kingdom-bases.json'
    payload = (json.dumps(doc, indent=1) + '\n').encode()
    if '--check' in sys.argv:
        assert path.read_bytes() == payload
    else:
        path.write_bytes(payload)
    print(json.dumps(dict(result='PASS', pairs=half, executions=len(rows), pins=len(h.VISITED),
                          sha256=hashlib.sha256(payload).hexdigest())))


if __name__ == '__main__':
    main()
