"""51D410 CAI state behaviour query, executed from the original AA.exe.

4D49B0 (the state-group-4 descriptor lookup on the state's +4 owner) is the
only seam; it returns null or a descriptor whose +24 word is scripted. The
owner +7C organization +8D byte is real memory. Usage:
    python native_cai_behavior.py [path-to-AA.exe] [--check]
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

WORDS = [None, 0, 1, 2, 3, 0xffffffff, 0x100, 0x101, 0x80000000]
BYTES = [0, 1, 2, 0x7f, 0x80, 0xff]


def cases():
    return [dict(word_24=w, byte_8d=b) for w in WORDS for b in BYTES]


def run(m, c):
    m.next = h.HEAP + 0x1000
    calls = []
    state = m.alloc(0x40)
    owner = m.alloc(0x200)
    org = m.alloc(0x100)
    record = m.alloc(0x40)
    m.w32(state + 4, owner)
    m.w32(owner + 0x7c, org)
    m.w8(org + 0x8d, c['byte_8d'])
    if c['word_24'] is not None:
        m.w32(record + 0x24, c['word_24'])

    def lookup(mm):
        calls.append(['group4', mm.ecx() == owner, mm.arg()])
        mm.ret(4, 0 if c['word_24'] is None else record)
    m.seams[0x4d49b0] = lookup
    eax = m.call(0x51d410, ecx=state)
    assert m.final_esp == h.STACK + 4
    return dict(input=c, calls=calls, al=eax & 0xff)


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
               seams=[hex(0x4d49b0)],
               observed_pins=[dict(address=hex(a), bytes=b) for a, b in sorted(h.VISITED.items())])
    path = HERE / 'native-cai-behavior.json'
    payload = (json.dumps(doc, indent=1) + '\n').encode()
    if '--check' in sys.argv:
        assert path.read_bytes() == payload
    else:
        path.write_bytes(payload)
    print(json.dumps(dict(result='PASS', pairs=half, executions=len(rows), pins=len(h.VISITED),
                          sha256=hashlib.sha256(payload).hexdigest())))


if __name__ == '__main__':
    main()
