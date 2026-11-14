"""Original 52CED0(other) on a path (the kind-1 node as far from the tail
as the other path's kind-1 count) under Unicorn on the phase8-ai machine
(research/phase8-ai/native_machine.py), called directly. No seams. Paths
are 0x28-byte node chains (+10 kind, +20 prev, +24 next) with +4 head,
+8 tail and +C kind-1 count; counts may disagree with the chain.

Writes native-path-match.json next to this script.
"""
from pathlib import Path
import json, random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / 'phase8-ai'))
from native_machine import Machine, header, instructions, sha  # noqa: E402
from native_uai_harness import body_instructions  # noqa: E402

ENTRY, END = 0x52ced0, 0x52cf04


def cases():
    rnd = random.Random(0x52ced0)
    rows = []
    for i in range(500):
        kinds = [rnd.choice([0, 1, 1, 2]) for _ in range(rnd.randrange(0, 7))]
        real = kinds.count(1)
        rows.append({'name': 'match%d' % i, 'kinds': kinds,
                     'kind1': rnd.choice([real, real, real + 1, real - 1, 0, -1]),
                     'other_kind1': rnd.choice([0, 1, 2, 3, real, real + 1, -0x80000000, 0x7fffffff])})
    return rows


def main():
    m = Machine('Single24')
    out = []
    for c in cases():
        path = m.alloc(0x20); nodes = []
        for k in c['kinds']:
            n = m.alloc(0x28); m.w(n + 0x10, k); nodes.append(n)
        for i, n in enumerate(nodes):
            m.w(n + 0x20, nodes[i - 1] if i else 0); m.w(n + 0x24, nodes[i + 1] if i + 1 < len(nodes) else 0)
        m.w(path + 4, nodes[0] if nodes else 0); m.w(path + 8, nodes[-1] if nodes else 0)
        m.w(path + 0xc, c['kind1'] & 0xffffffff)
        other = m.alloc(0x20); m.w(other + 0xc, c['other_kind1'] & 0xffffffff)
        r = m.call(ENTRY, ecx=path, args=[other])
        out.append({'input': c, 'output': None if r == 0 else nodes.index(r)})
    m.verify_v14()
    d = header(__file__, __doc__)
    cover = {'52CED0': [len([a for a in m.visited if ENTRY <= a < END]), body_instructions(ENTRY, END)]}
    d.update(instructions=instructions(m.visited), coverage=cover, cases=out)
    p = HERE / 'native-path-match.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'coverage': cover, 'found': sum(o['output'] is not None for o in out),
                      'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
