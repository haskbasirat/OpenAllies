"""4E16F0 Kingdom +2C4 list removal, executed from the original AA.exe.

No seams. The array is mapped with a guard word below it (index -1) and
after it, so the not-found shift into index -1 is observed. Usage:
    python native_kingdom_structure_list.py [path-to-AA.exe] [--check]
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

GUARD = 0x6badf00d


def cases():
    lists = [[], [11], [11, 22], [11, 22, 33], [11, 22, 11, 44], [5, 6, 7, 8, 9, 10]]
    out = []
    for words in lists:
        for value in sorted(set(words) | {99}):
            out.append(dict(words=words, value=value))
    return out


def run(m, c):
    m.next = h.HEAP + 0x1000
    kingdom = m.alloc(0x300)
    n = len(c['words'])
    storage = m.alloc(4 * (n + 2))
    m.w32(storage, GUARD, *c['words'], GUARD)
    m.w32(kingdom + 0x2c4, storage + 4, n, n)
    m.call(0x4e16f0, ecx=kingdom, args=(c['value'],))
    assert m.final_esp == h.STACK + 8
    count = m.r32(kingdom + 0x2c8)
    count = count - (1 << 32) if count & 0x80000000 else count
    words = m.r32(storage, n + 2)
    return dict(input=c, count=count, below=words[0], words=words[1:n + 1], above=words[n + 1])


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
               guard=GUARD, original_sha256=h.ORIGINAL_SHA, v14_sha256=h.V14_SHA,
               producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
               seams=[],
               observed_pins=[dict(address=hex(a), bytes=b) for a, b in sorted(h.VISITED.items())])
    path = HERE / 'native-kingdom-structure-list.json'
    payload = (json.dumps(doc, indent=1) + '\n').encode()
    if '--check' in sys.argv:
        assert path.read_bytes() == payload
    else:
        path.write_bytes(payload)
    print(json.dumps(dict(result='PASS', pairs=half, executions=len(rows), pins=len(h.VISITED),
                          sha256=hashlib.sha256(payload).hexdigest())))


if __name__ == '__main__':
    main()
