"""Performer task targeting context 0x57E360(&context).

The whole routine executes unchanged over a real GTID registry at
[B364C0]; there are no seams. The context starts filled with marker words so
that untouched words show. The points include a quiet NaN and -0.0 (copied
as they are); a signalling NaN is left out because the emulator's fld does
not quiet it as the x87 does.

Usage: python native_performer_context.py [path/to/AA.exe]
Writes native-performer-context.json next to this script.
"""
import os, sys
from pathlib import Path
if len(sys.argv) > 1:
    os.environ['AA_GAME_DIR'] = str(Path(sys.argv[1]).resolve().parent)
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / 'phase8-movement'))
import hashlib, json, random, struct  # noqa: E402
import harness as h  # noqa: E402
from harness import f2b  # noqa: E402

TARGET = 1


def run(c):
    m = h.Machine('Single24')
    reg = m.alloc(0x20004 + 0x10000 * 4 + 16)
    m.w32(0xb364c0, reg)
    task = m.alloc(0x40)
    m.w32(task + 4, 0x00aa00aa)
    m.w32(task + 0x28, c['gtid'])
    m.w32(task + 0x2c, *c['task_point'])
    target = m.alloc(0x40)
    m.w32(target + 0x20, *c['target_point'])
    idx = 0x51
    m.u.mem_write(reg + 4 + 2 * idx, struct.pack('<H', c['generation']))
    m.w32(reg + 0x20004 + 4 * idx, target if c['entry'] else 0)
    ctx = m.alloc(0x30)
    m.w32(ctx, *[0xc0000000 + k for k in range(12)])
    m.call(0x57e360, ecx=task, args=(ctx,))
    assert m.final_esp == h.STACK + 8 and m.fpu_top() == 0
    words = m.r32(ctx, 12)
    words[7] = TARGET if words[7] == target else words[7]
    return {'context': words}


def cases():
    rnd = random.Random(0x57e360)
    pts = [[f2b(1.5), f2b(-2.0)], [0x7fc00001, f2b(3.0)], [f2b(10.0), 0x7fe00000], [0xffc00000, 0x80000000]]
    rows = []
    for i in range(120):
        rows.append({'name': f'context{i}', 'gtid': rnd.choice([0x00030051, 0x00040051, 0]),
                     'generation': rnd.choice([3, 3, 4]), 'entry': rnd.random() < 0.8,
                     'task_point': rnd.choice(pts), 'target_point': rnd.choice(pts)})
    return rows


def main():
    rows = [{'input': c, 'output': run(c)} for c in cases()]
    doc = {'schema': 1, 'scope': '0x57E360 Performer targeting context', 'original_sha256': h.ORIGINAL_SHA,
           'v14_sha256': h.V14_SHA, 'case_count': len(rows), 'cases': rows,
           'instructions': [{'va': hex(a), 'bytes': b} for a, b in sorted(h.VISITED.items())]}
    out = HERE / 'native-performer-context.json'
    out.write_text(json.dumps(doc) + '\n')
    print(json.dumps({'file': out.name, 'cases': len(rows), 'instructions': len(h.VISITED),
                      'sha256': hashlib.sha256(out.read_bytes()).hexdigest()}))


if __name__ == '__main__':
    main()
