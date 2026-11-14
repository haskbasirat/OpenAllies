"""4ECB90 kingdom memory record update, executed from the original AA.exe.

Seams: the kingdom's virtual +8C and +108 (scripted results), 4EC4A0 (record
refresh) and the [B36528] notifications 79DEB0 / 79E7B0, each recording its
this-pointer and arguments. The actor +18C/+190 words, the kingdom +5C
record array, the record +14 word and the [B36528] +20 gate are real
memory. Usage:
    python native_memory_reveal.py [path-to-AA.exe] [--check]
"""
from pathlib import Path
import hashlib, itertools, json, os, sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
ARGS = [a for a in sys.argv[1:] if not a.startswith('--')]
if ARGS:
    os.environ['AA_GAME_DIR'] = str(Path(ARGS[0]).resolve().parent)
sys.path.insert(0, str(ROOT / 'research/phase8-movement'))
import harness as h  # noqa: E402

VSTUBS = h.STUBS + 0x60000
OWNER = 0x2c00777
INDEX = 5


def cases():
    out = []
    for actor, w190, v8c, record, keep, word14, notify in itertools.product(
            [True, False], [0, 1], [1, 0], [True, False], [0, 1], ['owner', 'other'],
            ['none', 'off', 'on']):
        out.append(dict(actor=actor, word_190=w190, kingdom_8c=v8c, record=record,
                        keep_owner=keep, word_14=word14, notifications=notify))
    return out


def run(m, c):
    m.next = h.HEAP + 0x1000
    calls = []
    kingdom = m.alloc(0x100)
    vt = m.alloc(0x200)
    m.w32(kingdom, vt)
    m.w32(vt + 0x8c, VSTUBS)
    m.w32(vt + 0x108, VSTUBS + 0x10)
    records = m.alloc(4 * 8)
    m.w32(kingdom + 0x5c, records)
    record = m.alloc(0x40)
    m.w32(record + 0x14, OWNER if c['word_14'] == 'owner' else 0x1234)
    if c['record']:
        m.w32(records + 4 * INDEX, record)
    actor = 0
    if c['actor']:
        actor = m.alloc(0x200)
        m.w32(actor + 0x18c, INDEX)
        m.w32(actor + 0x190, c['word_190'])
    app = 0
    if c['notifications'] != 'none':
        app = m.alloc(0x40)
        m.w8(app + 0x20, 1 if c['notifications'] == 'on' else 0)
    m.w32(0xb36528, app)

    def v8c(mm):
        assert mm.ecx() == kingdom
        calls.append(['8c'])
        mm.ret(0, c['kingdom_8c'])

    def v108(mm):
        assert mm.ecx() == kingdom
        calls.append(['108'])
        mm.ret(0, OWNER)

    def refresh(mm):
        assert mm.ecx() == kingdom and mm.arg() == record
        calls.append(['refresh'])
        mm.ret(4)

    def notify(name):
        def seam(mm):
            assert mm.ecx() == app and mm.arg(1) == actor
            calls.append([name, mm.arg() == record])
            mm.ret(8)
        return seam
    m.seams[VSTUBS] = v8c
    m.seams[VSTUBS + 0x10] = v108
    m.seams[0x4ec4a0] = refresh
    m.seams[0x79deb0] = notify('before')
    m.seams[0x79e7b0] = notify('after')
    eax = m.call(0x4ecb90, ecx=kingdom, args=(actor, c['keep_owner']))
    assert m.final_esp == h.STACK + 0xc
    return dict(input=c, calls=calls, al=eax & 0xff, word_14=m.r32(record + 0x14))


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
               owner=OWNER, original_sha256=h.ORIGINAL_SHA, v14_sha256=h.V14_SHA,
               producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
               seams=['kingdom virtual +8C/+108', hex(0x4ec4a0), hex(0x79deb0), hex(0x79e7b0)],
               observed_pins=[dict(address=hex(a), bytes=b) for a, b in sorted(h.VISITED.items())])
    path = HERE / 'native-memory-reveal.json'
    payload = (json.dumps(doc, indent=1) + '\n').encode()
    if '--check' in sys.argv:
        assert path.read_bytes() == payload
    else:
        path.write_bytes(payload)
    print(json.dumps(dict(result='PASS', pairs=half, executions=len(rows), pins=len(h.VISITED),
                          sha256=hashlib.sha256(payload).hexdigest())))


if __name__ == '__main__':
    main()
