"""518850 aircraft Mind guard/escort command, executed from the original AA.exe.

Seams: the home lookup 4D09A0, the command object 530F10, the Denizen
leader 568ED0 and the activations 516FD0 / 518170 (scripted AL), each
recording its this-pointer and arguments. 52A000 and 4CF1B0 run unhooked
over a real Mind stack; the GTID table [B364C0], the command +C/+10 words,
the Mind +28/+AC/+E0 words, the owner +74 and the objects' +14/+88 are real
memory. Cases where the original dereferences a null object are excluded.
Usage:
    python native_aircraft_guard_command.py [path-to-AA.exe] [--check]
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

COMMAND_GTID = (0x21 << 16) | 0x40
HOME_GTID = (0x22 << 16) | 0x41
OBJECT_GTID = 0x00550123
OTHER_GTID = 0x00550124


def cases():
    rng = random.Random(0x518850)
    out = []
    for i in range(700):
        home_live = rng.random() < 0.6
        c = dict(performer=rng.random() < 0.9, flag=rng.random() < 0.9,
                 command_live=rng.random() < 0.9, home_live=home_live,
                 home_object=home_live or rng.random() < 0.8,
                 command_object=home_live or rng.random() < 0.85,
                 home_matches=rng.random() < 0.5, current=rng.choice(['none', '9c', 'd0']),
                 escorted_matches=rng.random() < 0.5, denizen=rng.random() < 0.85,
                 leader=rng.choice(['a', 'b', None]), guarded=rng.choice(['a', 'b', None]),
                 al=rng.choice([0, 1]))
        out.append(c)
    return out


def run(m, table, c):
    calls = []
    mind = m.alloc(0x200)
    owner = m.alloc(0x100)
    command = m.alloc(0x40)
    home = m.alloc(0x100)
    target = m.alloc(0x100)
    denizen = m.alloc(0x40)
    leaders = {'a': m.alloc(0x20), 'b': m.alloc(0x20), None: 0}
    names = {v: k for k, v in leaders.items()}
    m.w32(mind + 4, owner)
    m.w32(owner + 0x74, m.alloc(0x40) if c['performer'] else 0)
    m.w32(command + 0xc, 0x11 if c['flag'] else 0x10)
    m.w32(command + 0x10, COMMAND_GTID if c['command_live'] else COMMAND_GTID ^ 0x10000)
    m.w32(mind + 0x28, HOME_GTID if c['home_live'] else HOME_GTID ^ 0x10000)
    m.w32(target + 0x14, OBJECT_GTID)
    m.w32(home + 0x14, OBJECT_GTID if c['home_matches'] else OTHER_GTID)
    m.w32(home + 0x88, denizen if c['denizen'] else 0)
    m.w32(mind + 0xac, OBJECT_GTID if c['escorted_matches'] else OTHER_GTID)
    m.w32(mind + 0xe0, leaders[c['guarded']])
    for off in (0x9c, 0xd0):
        m.w32(mind + off + 4, mind)
    if c['current'] != 'none':
        node = m.alloc(16)
        m.w32(node, mind + int(c['current'], 16))
        m.w32(mind + 0x10, node)
    for gtid in (COMMAND_GTID, HOME_GTID):
        index = gtid & 0xffff
        m.u.mem_write(table + 4 + index * 2, (gtid >> 16).to_bytes(2, 'little'))
        m.w32(table + 0x20004 + index * 4, 0x2c00600)

    def home_lookup(mm):
        assert mm.ecx() == mind
        calls.append(['home'])
        mm.ret(0, home if c['home_object'] else 0)

    def command_object(mm):
        assert mm.ecx() == command
        calls.append(['command_object'])
        mm.ret(0, target if c['command_object'] else 0)

    def leader(mm):
        assert mm.ecx() == denizen
        calls.append(['leader', mm.arg()])
        mm.ret(4, leaders[c['leader']])

    def escort(mm):
        assert mm.ecx() == mind + 0x9c
        calls.append(['escort', mm.arg(), mm.arg(1) == target, mm.arg(2)])
        mm.ret(0xc, c['al'])

    def guard(mm):
        assert mm.ecx() == mind + 0xd0
        calls.append(['guard', mm.arg(), names[mm.arg(1)]])
        mm.ret(8, c['al'])
    m.seams[0x4d09a0] = home_lookup
    m.seams[0x530f10] = command_object
    m.seams[0x568ed0] = leader
    m.seams[0x516fd0] = escort
    m.seams[0x518170] = guard
    eax = m.call(0x518850, ecx=mind, args=(command,))
    assert m.final_esp == h.STACK + 8
    for gtid in (COMMAND_GTID, HOME_GTID):
        index = gtid & 0xffff
        m.u.mem_write(table + 4 + index * 2, b'\0\0')
        m.w32(table + 0x20004 + index * 4, 0)
    return dict(input=c, calls=calls, al=eax & 0xff)


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
               command_gtid=COMMAND_GTID, home_gtid=HOME_GTID, object_gtid=OBJECT_GTID,
               other_gtid=OTHER_GTID, original_sha256=h.ORIGINAL_SHA, v14_sha256=h.V14_SHA,
               producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
               seams=[hex(a) for a in [0x4d09a0, 0x530f10, 0x568ed0, 0x516fd0, 0x518170]],
               observed_pins=[dict(address=hex(a), bytes=b) for a, b in sorted(h.VISITED.items())])
    path = HERE / 'native-aircraft-guard-command.json'
    payload = (json.dumps(doc, indent=1) + '\n').encode()
    if '--check' in sys.argv:
        assert path.read_bytes() == payload
    else:
        path.write_bytes(payload)
    print(json.dumps(dict(result='PASS', pairs=half, executions=len(rows), pins=len(h.VISITED),
                          sha256=hashlib.sha256(payload).hexdigest())))


if __name__ == '__main__':
    main()
