"""51D560 city-block binding test, executed from the original AA.exe.

No seams: the component's +34 handle and the GTID table [B364C0] (+4
generation words, +20004 slots) are real memory, with live, stale and null
slots. Usage:
    python native_city_block_bound.py [path-to-AA.exe] [--check]
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


def cases():
    out = []
    for index in [0, 1, 0x1234, 0x7fff, 0x8000, 0xffff]:
        for table_generation in [0, 1, 0x7fff, 0x8000, 0xffff]:
            for handle_generation in sorted({table_generation, table_generation ^ 1, table_generation ^ 0x8000}):
                for live in [True, False]:
                    out.append(dict(handle=(handle_generation << 16) | index,
                                    table_generation=table_generation, live=live))
    return out


def run(m, table, c):
    index = c['handle'] & 0xffff
    m.u.mem_write(table + 4 + index * 2, c['table_generation'].to_bytes(2, 'little'))
    m.w32(table + 0x20004 + index * 4, 0x2c00010 if c['live'] else 0)
    component = m.alloc(0x40)
    m.w32(component + 0x34, c['handle'])
    eax = m.call(0x51d560, ecx=component)
    assert m.final_esp == h.STACK + 4
    m.u.mem_write(table + 4 + index * 2, b'\0\0')
    m.w32(table + 0x20004 + index * 4, 0)
    return dict(input=c, al=eax & 0xff)


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
               seams=[],
               observed_pins=[dict(address=hex(a), bytes=b) for a, b in sorted(h.VISITED.items())])
    path = HERE / 'native-city-block-bound.json'
    payload = (json.dumps(doc, indent=1) + '\n').encode()
    if '--check' in sys.argv:
        assert path.read_bytes() == payload
    else:
        path.write_bytes(payload)
    print(json.dumps(dict(result='PASS', pairs=half, executions=len(rows), pins=len(h.VISITED),
                          sha256=hashlib.sha256(payload).hexdigest())))


if __name__ == '__main__':
    main()
