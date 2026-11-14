"""Native oracle for the active-component switch 4CFC10 (actor +E4).

Random actors (+8 bit 0, current +E4) and components whose virtuals +44
(accepts), +40 (deactivated; some rewrite actor +E4 to test the reread) and
+3C (activated) are recording stubs. Records the return byte, the final +E4,
the call order and the low byte of the word passed to +40/+3C (its upper
bytes are left over from the component pointer).

Usage: native_activation.py  (AA.exe is located by the shared harness)
"""
from pathlib import Path
import random
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import harness as h  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX  # noqa: E402


def main():
    rng = random.Random(0x4CFC10)
    m = h.Machine('Double53')
    cases = []
    for i in range(300):
        events = []
        n = rng.randint(1, 3)
        actor = m.alloc(0x100)
        flags = rng.choice((0, 1, 0x1001, 0x100, rng.getrandbits(32)))
        m.w(actor + 8, flags)
        comps = [m.alloc(0x20) for _ in range(n)]
        index = {c: k for k, c in enumerate(comps)}
        spec = []
        for k, c in enumerate(comps):
            s = {'accepts': rng.choice((1, 0, 0x100, 0x201)) if rng.random() < 0.4 else 1,
                 'rewrites': None}
            roll = rng.random()
            if roll < 0.15:
                s['rewrites'] = 'null'
            elif roll < 0.3:
                s['rewrites'] = rng.randrange(n)
            spec.append(s)

            def accepts(mm, k=k, s=s):
                events.append(['accepts', k])
                mm.ret(0, s['accepts'])

            def deactivated(mm, k=k, s=s):
                word = mm.arg(0)
                events.append(['deactivated', k, word & 0xFF])
                if s['rewrites'] == 'null':
                    mm.w(actor + 0xE4, 0)
                elif s['rewrites'] is not None:
                    mm.w(actor + 0xE4, comps[s['rewrites']])
                mm.ret(4)

            def activated(mm, k=k):
                events.append(['activated', k, mm.arg(0) & 0xFF])
                mm.ret(4)
            m.w(c, m.vtable({0x44: accepts, 0x40: deactivated, 0x3C: activated}))
        current = rng.randrange(n) if rng.random() < 0.75 else None
        m.w(actor + 0xE4, comps[current] if current is not None else 0)
        target = rng.randrange(n) if rng.random() < 0.85 else None
        result = m.call(0x4CFC10, ecx=actor, args=(comps[target] if target is not None else 0,), pop=4) & 0xFF
        final = m.r(actor + 0xE4)
        cases.append({'flags': flags, 'components': spec, 'current': current, 'component': target,
                      'result': result, 'final': index[final] if final else None, 'events': events})
    h.report('native-activation-fixtures.json', '4CFC10 active-component switch with recording component virtuals.',
             cases, m.visited)


if __name__ == '__main__':
    main()
