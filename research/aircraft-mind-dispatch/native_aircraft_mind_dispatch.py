"""51C6F0 aircraft Mind dispatcher and the 51C8C0 / 517190 state exits, from
the original AA.exe.

Seams: 52A340 (pop the head state), the Mind +8 scheduler's virtual +4 and
the attack state's virtual +1C (reached through 529C20), each recording its
this-pointer and arguments. 52A000 and 529C20 run unhooked over a real Mind
stack (+10 head node), the states' +4 owners and the +F4/+118 words. Usage:
    python native_aircraft_mind_dispatch.py [path-to-AA.exe] [--check]
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

STATES = {'9c': 0x9c, 'e8': 0xe8, '12c': 0x12c}


def cases():
    out = []
    for function in ['51c6f0', '51c8c0', '517190']:
        currents = ['empty', '9c', 'e8', '12c', 'other'] if function == '51c6f0' else ['other']
        for current in currents:
            for separate_owner in [False, True]:
                for status in [2, 1, 0]:
                    for owner_status in ([2, 0] if separate_owner else [None]):
                        for phase in [0, 4, 1, 3]:
                            out.append(dict(function=function, current=current,
                                            separate_owner=separate_owner, status=status,
                                            owner_status=owner_status, phase=phase))
    return out


def run(m, c):
    m.next = h.HEAP + 0x1000
    calls = []
    mind = m.alloc(0x200)
    owner = m.alloc(0x200) if c['separate_owner'] else mind
    other_state = m.alloc(0x40)
    stub_vt = m.alloc(0x40)
    m.w32(stub_vt + 0x1c, h.STUBS + 0x300)
    for node_mind in {mind, owner}:
        for off in STATES.values():
            m.w32(node_mind + off, stub_vt)
            m.w32(node_mind + off + 4, node_mind)
    # The +9C state's owner ([Mind+A0]) is the separate owner when asked.
    m.w32(mind + 0x9c + 4, owner)
    m.w32(mind + 0xf4, c['status'])
    if c['separate_owner']:
        m.w32(owner + 0xf4, c['owner_status'])
    m.w32(owner + 0x118, c['phase'])
    m.w32(mind + 0x118, c['phase'])
    m.w32(mind + 0xb0, 0x5555)
    if c['current'] != 'empty':
        node = m.alloc(16)
        m.w32(node, other_state if c['current'] == 'other' else mind + STATES[c['current']])
        m.w32(mind + 0x10, node)
        # 52A000 reads the stack of the state's +4 owner: the separate owner
        # shares the Mind's head node.
        m.w32(owner + 0x10, node)
    scheduler_vt = m.alloc(0x20)
    m.w32(mind + 8, scheduler_vt)
    m.w32(scheduler_vt + 4, h.STUBS + 0x340)

    def leave(mm):
        who = 'mind' if mm.ecx() == mind + 0xe8 else 'owner' if mm.ecx() == owner + 0xe8 else hex(mm.ecx())
        calls.append(['leave_e8', who])
        mm.ret(0)

    def schedule(mm):
        assert mm.ecx() == mind + 8
        calls.append(['schedule', mm.arg(), mm.arg(1)])
        mm.ret(8)

    def pop(mm):
        assert mm.ecx() == mind
        calls.append(['pop'])
        mm.ret(0)
    m.seams[h.STUBS + 0x300] = leave
    m.seams[h.STUBS + 0x340] = schedule
    m.seams[0x52a340] = pop
    f = c['function']
    result = dict(input=c, calls=calls)
    if f == '51c6f0':
        eax = m.call(0x51c6f0, ecx=mind, args=(0x7777,))
        assert m.final_esp == h.STACK + 8
        result.update(al=eax & 0xff, word_b0=m.r32(mind + 0xb0))
    else:
        # The exit runs on the +9C state, whose +4 owner is the Mind (or the
        # separate owner).
        state = mind + 0x9c
        m.w32(state + 4, owner)
        m.w32(state + 0xc, 0x66)
        m.w32(state + 0x14, 0x5555)
        m.call(int(f, 16), ecx=state)
        assert m.final_esp == h.STACK + 4
        result.update(word_14=m.r32(state + 0x14), word_c=m.r32(state + 0xc))
    return result


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
               seams=['52a340', 'scheduler +4', 'attack state +1c'],
               observed_pins=[dict(address=hex(a), bytes=b) for a, b in sorted(h.VISITED.items())])
    path = HERE / 'native-aircraft-mind-dispatch.json'
    payload = (json.dumps(doc, indent=1) + '\n').encode()
    if '--check' in sys.argv:
        assert path.read_bytes() == payload
    else:
        path.write_bytes(payload)
    print(json.dumps(dict(result='PASS', pairs=half, executions=len(rows), pins=len(h.VISITED),
                          sha256=hashlib.sha256(payload).hexdigest())))


if __name__ == '__main__':
    main()
