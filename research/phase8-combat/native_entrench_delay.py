"""Native oracle for the entrenchment delay getter 0x45D520 (definition +2EC;
with a modifier context, tail call 0x5014D0: extended division by the
modifier's +40 scalar). ST0 is recorded as the 80-bit register image.
"""
from pathlib import Path
import random
import struct
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import harness as h  # noqa: E402


def f32(v):
    return struct.unpack('<I', struct.pack('<f', v))[0]


def main():
    rng = random.Random(0x45D520)
    cases, visited = [], {}
    for precision in h.PRECISIONS:
        m = h.Machine(precision)
        for i in range(120):
            base = rng.choice((f32(0), f32(rng.uniform(0, 30)), f32(rng.uniform(0.1, 5)), f32(7.0)))
            scalar = rng.choice((None, f32(1.0), f32(rng.uniform(0.25, 3)), f32(3.0), f32(0.7)))
            definition = m.alloc(0x300)
            m.w(definition + 0x2EC, base)
            modifier = 0
            if scalar is not None:
                modifier = m.alloc(0x80)
                m.w(modifier + 0x40, scalar)
            m.call(0x45D520, ecx=definition, args=(modifier,), pop=4)
            st = m.st0_ext()
            cases.append({'precision': precision, 'base': base, 'scalar_40': scalar,
                          'mantissa': f'{st["mantissa"]:016x}', 'sign_exponent': st['sign_exponent']})
        visited.update(m.visited)
    h.report('native-entrench-delay-fixtures.json',
             '0x45D520 entrenchment delay (definition +2EC, 0x5014D0 modifier division); all precisions.',
             cases, visited)


if __name__ == '__main__':
    main()
