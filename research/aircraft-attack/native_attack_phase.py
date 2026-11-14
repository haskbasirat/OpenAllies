"""51B4C0 aircraft attack-state phase step, executed from the original AA.exe.

Seams: the plan 51A920 (which may rewrite the phase +30, so the later
re-read is observed), the path clear 55E1B0, the Mind's +34 move state
virtual +C and the Mind +8 scheduler virtual +4, each recording its
this-pointer and arguments. 529C30, the state +14/+30 words and the owner's
flight +10 byte are real memory. Usage:
    python native_attack_phase.py [path-to-AA.exe] [--check]
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

VSTUBS = h.STUBS + 0x40000


def cases():
    return [dict(phase=p, retarget=r, plan_sets=s)
            for p, r, s in itertools.product([0, 1, 2, 3, 4, 5, 0xffffffff], [0, 1, 0x80],
                                              [None, 0, 2, 4])]


def run(m, c):
    m.next = h.HEAP + 0x1000
    calls = []
    state = m.alloc(0x60)
    mind = m.alloc(0x200)
    owner = m.alloc(0x100)
    flight = m.alloc(0x100)
    move_vt = m.alloc(0x40)
    scheduler_vt = m.alloc(0x40)
    m.w32(state + 4, mind)
    m.w32(mind + 4, owner)
    m.w32(owner + 0xbc, flight)
    m.w8(flight + 0x10, 0x77)
    m.w32(mind + 0x34, move_vt)
    m.w32(move_vt + 0xc, VSTUBS)
    m.w32(mind + 8, scheduler_vt)
    m.w32(scheduler_vt + 4, VSTUBS + 0x10)
    m.w32(state + 0x30, c['phase'])
    m.w8(state + 0x14, c['retarget'])

    def plan(mm):
        assert mm.ecx() == state
        calls.append(['plan', mm.r32(state + 0x30)])
        if c['plan_sets'] is not None:
            mm.w32(state + 0x30, c['plan_sets'])
        mm.ret(0)

    def clear(mm):
        assert mm.ecx() == flight + 0x28
        calls.append(['clear_path', mm.r8(flight + 0x10)])
        mm.ret(0)

    def think(mm):
        assert mm.ecx() == mind + 0x34
        calls.append(['move_think'])
        mm.ret(0)

    def schedule(mm):
        assert mm.ecx() == mind + 8
        calls.append(['schedule', mm.arg(), mm.arg(1)])
        mm.ret(8)
    m.seams[0x51a920] = plan
    m.seams[0x55e1b0] = clear
    m.seams[VSTUBS] = think
    m.seams[VSTUBS + 0x10] = schedule
    m.call(0x51b4c0, ecx=state)
    assert m.final_esp == h.STACK + 4
    return dict(input=c, calls=calls, phase=m.r32(state + 0x30), retarget=m.r8(state + 0x14),
                flight_10=m.r8(flight + 0x10))


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
               seams=[hex(0x51a920), hex(0x55e1b0), 'move state +C', 'scheduler +4'],
               observed_pins=[dict(address=hex(a), bytes=b) for a, b in sorted(h.VISITED.items())])
    path = HERE / 'native-attack-phase.json'
    payload = (json.dumps(doc, indent=1) + '\n').encode()
    if '--check' in sys.argv:
        assert path.read_bytes() == payload
    else:
        path.write_bytes(payload)
    print(json.dumps(dict(result='PASS', pairs=half, executions=len(rows), pins=len(h.VISITED),
                          sha256=hashlib.sha256(payload).hexdigest())))


if __name__ == '__main__':
    main()
