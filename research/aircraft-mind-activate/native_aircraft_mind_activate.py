"""517C60 aircraft Mind activation and dog-fighter byte, from the original AA.exe.

Seams: 529EC0 (base activation, scripted AL), 55EB30 (formation join) and
52A810 (state activation), each recording its this-pointer and argument.
529C30 and every field (Mind +10/+188/+18C/+1D0, the Performer ability array
and the definitions' +3C/+2B4) are real memory. Usage:
    python native_aircraft_mind_activate.py [path-to-AA.exe] [--check]
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

# Each ability: None = definition +3C null, else the +3C object's +2B4 word.
ABILITIES = [[], [None], [0], [7], [7, 1], [7, None, 3], [7, 0, None], [1, 2, 3, 0],
             [5, 5, 5, 5, 5]]
COUNTS = [None, 0, -1, 2]  # None: the array length


def cases():
    out = []
    for empty in [True, False]:
        for base in [1, 0, 0x101]:
            for abilities in ABILITIES:
                for count in COUNTS:
                    n = len(abilities) if count is None else min(count, len(abilities))
                    if count is not None and count > len(abilities):
                        continue
                    out.append(dict(stack_empty=empty, base_al=base, abilities=abilities,
                                    count=n if count is None else count))
    return out


def run(m, c):
    m.next = h.HEAP + 0x1000
    calls = []
    mind = m.alloc(0x200)
    other = m.alloc(0x100)
    holder = m.alloc(0x10)
    actor = m.alloc(0x100)
    flight = m.alloc(0x40)
    owner = m.alloc(0x100)
    performer = m.alloc(0x40)
    m.w32(mind + 0x10, 0 if c['stack_empty'] else m.alloc(16))
    m.w32(mind + 0x18c, other)
    m.w8(other + 0x7c, 0xcc)
    m.w32(other + 0x34 + 4, holder)
    m.w32(holder + 4, actor)
    m.w32(actor + 0xbc, flight)
    m.w8(mind + 0x1d0, 0xaa)
    m.w32(mind + 4, owner)
    m.w32(owner + 0x74, performer)
    items = m.alloc(4 * max(1, len(c['abilities'])))
    for i, word in enumerate(c['abilities']):
        ability = m.alloc(0x20)
        definition = m.alloc(0x40)
        m.w32(items + 4 * i, ability)
        m.w32(ability + 4, definition)
        if word is not None:
            obj = m.alloc(0x2c0)
            m.w32(definition + 0x3c, obj)
            m.w32(obj + 0x2b4, word)
    m.w32(performer + 0x14, items)
    m.w32(performer + 0x18, c['count'])

    def join(mm):
        calls.append(['join', mm.ecx() == flight, mm.arg()])
        mm.ret(4)

    def state(mm):
        calls.append(['activate_188', mm.ecx() == mind + 0x188, mm.arg()])
        mm.ret(4)

    def base(mm):
        calls.append(['base', mm.ecx() == mind])
        mm.ret(0, c['base_al'])
    m.seams[0x55eb30] = join
    m.seams[0x52a810] = state
    m.seams[0x529ec0] = base
    eax = m.call(0x517c60, ecx=mind)
    assert m.final_esp == h.STACK + 4
    return dict(input=c, calls=calls, al=eax & 0xff, byte_1d0=m.r8(mind + 0x1d0),
                byte_7c=m.r8(other + 0x7c))


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
               seams=[hex(a) for a in [0x529ec0, 0x55eb30, 0x52a810]],
               observed_pins=[dict(address=hex(a), bytes=b) for a, b in sorted(h.VISITED.items())])
    path = HERE / 'native-aircraft-mind-activate.json'
    payload = (json.dumps(doc, indent=1) + '\n').encode()
    if '--check' in sys.argv:
        assert path.read_bytes() == payload
    else:
        path.write_bytes(payload)
    print(json.dumps(dict(result='PASS', pairs=half, executions=len(rows), pins=len(h.VISITED),
                          sha256=hashlib.sha256(payload).hexdigest())))


if __name__ == '__main__':
    main()
