"""Original 4DDC50(&out, object) (an actor's 16-byte group record for the
object's GTID, vector +188 / count +18C) under Unicorn on the phase8-ai
machine (research/phase8-ai/native_machine.py), called directly. No seams.
The output record is pre-filled with 0x5A bytes so untouched bytes show.

Writes native-group-records.json next to this script.
"""
from pathlib import Path
import json, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / 'phase8-ai'))
from native_machine import Machine, header, instructions, sha  # noqa: E402
from native_uai_harness import body_instructions  # noqa: E402

ENTRY, END = 0x4ddc50, 0x4ddcce


def cases():
    rnd = random.Random(0x4ddc50)
    rows = []
    gt = [0x00910910, 0x00920920, 0x00930930]
    for i in range(300):
        recs = [[rnd.choice(gt + [0]), rnd.getrandbits(32), rnd.getrandbits(32), rnd.getrandbits(8), rnd.getrandbits(8)]
                for _ in range(rnd.randrange(0, 5))]
        rows.append({'name': 'group%d' % i, 'records': recs, 'count': rnd.choice([len(recs)] * 4 + [0, -1]),
                     'gtid': rnd.choice(gt + [0])})
    return rows


def main():
    m = Machine('Single24')
    out = []
    for c in cases():
        actor = m.alloc(0x200); vec = m.alloc(16 * max(1, len(c['records'])))
        for k, r in enumerate(c['records']):
            m.w(vec + 16 * k, r[0]); m.w(vec + 16 * k + 4, r[1]); m.w(vec + 16 * k + 8, r[2])
            m.wb(vec + 16 * k + 12, r[3]); m.wb(vec + 16 * k + 13, r[4]); m.wb(vec + 16 * k + 14, 0xee); m.wb(vec + 16 * k + 15, 0xee)
        m.w(actor + 0x188, vec); m.w(actor + 0x18c, c['count'] & 0xffffffff)
        obj = m.alloc(0x40); m.w(obj + 0x14, c['gtid'])
        res = m.alloc(16); m.u.mem_write(res, b'\x5a' * 16)
        r = m.call(ENTRY, ecx=actor, args=[res, obj])
        assert r == res
        out.append({'input': c, 'output': list(bytes(m.u.mem_read(res, 16)))})
    m.verify_v14()
    d = header(__file__, __doc__)
    cover = {'4DDC50': [len([a for a in m.visited if ENTRY <= a < END]), body_instructions(ENTRY, END)]}
    d.update(instructions=instructions(m.visited), coverage=cover, cases=out)
    p = HERE / 'native-group-records.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'coverage': cover, 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
