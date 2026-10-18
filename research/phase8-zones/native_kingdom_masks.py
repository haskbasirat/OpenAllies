"""Oracle: kingdom shared-vision and supply masks (0x4E0F90 -> 0x54EBA0 +
0x559600) executed by the ORIGINAL code. Kingdom virtual +0x138 (allied) is a
recorded seam answered from a random alliance matrix. Recorded per call: the
kingdom's +0x228 / +0x22C words and the ordered +0x138 calls.
Writes native-kingdom-masks.json."""
import json, random
from pathlib import Path
from zones_machine import Machine, NativeFault, header, instructions, visited_in
from zones_world import World

HERE = Path(__file__).resolve().parent


def run_case(seed):
    rng = random.Random(seed)
    m = Machine('Double53')
    nk = rng.randint(1, 8)
    ks = [{'index': rng.choice([i, i, rng.randrange(20)]), 'nonmajor': rng.choice([0, 0, 0, 1])} for i in range(nk)]
    for _ in range(rng.choice([0, 0, 1, 2])):
        ks.insert(rng.randrange(len(ks) + 1), None)
    w = World(m, kingdoms=ks)
    ev = w.events
    allied = [[rng.random() < 0.5 for _ in w.kingdoms] for _ in w.kingdoms]
    for i, k in enumerate(w.kingdoms):
        if not k:
            continue

        def f(m, i=i):
            o = w.kingdom_index(m.arg(1))
            ev.append(['allied', i, o])
            m.ret(1 if allied[i][o] else 0, 4)
        m.w(k, m.vtable({0x138: f}))
        m.w(k + 0x228, rng.randrange(1 << 32))
        m.w16(k + 0x22c, rng.randrange(1 << 16))
    calls = []
    for i, k in enumerate(w.kingdoms):
        if not k:
            continue
        del ev[:]
        try:
            m.call(0x4e0f90, ecx=k)
            fault = None
        except NativeFault as e:
            fault = str(e)
        calls.append({'kingdom': i, 'fault': fault, 'events': list(ev), 'shared': m.r(k + 0x228), 'supply': m.r16(k + 0x22c)})
    m.verify_v14()
    kout = [{'addr': p, 'index': m.r(p + 0x18c), 'nonmajor': m.r(p + 0x190)} if p else None for p in w.kingdoms]
    return {'seed': seed, 'kingdoms': kout, 'allied': allied, 'calls': calls}, m.visited


def main():
    cases, visited = [], {}
    for seed in range(500):
        c, v = run_case(seed)
        cases.append(c)
        visited.update(v)
    out = header(__file__, 'Unicorn 2.1.4, original AA.exe kingdom shared-vision/supply masks')
    out.update({'cases': cases, 'case_count': len(cases), 'op_count': sum(len(c['calls']) for c in cases),
                'coverage': {hex(a): len(visited_in(visited, a, a + s)) for a, s in ((0x4e0f90, 0x1d), (0x54eba0, 0xc7), (0x559600, 0xa6))},
                'instructions': instructions(visited)})
    (HERE / 'native-kingdom-masks.json').write_text(json.dumps(out, separators=(',', ':')))
    print('cases', len(cases), 'calls', out['op_count'], out['coverage'])


if __name__ == '__main__':
    main()
