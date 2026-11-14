"""Native oracle for the ability ready/cooldown predicates 45D430, 4C85D0,
4C8100, 4C8120 (x87 compares against the binary32 zero at [9F32DC], with
signed zeros, denormals, infinities and NaNs) and the ActorData +308 getter
45D4F0 (with the real 501CF0 through a modifier context; settings +1A4),
all precisions. ST0 of 45D4F0 is recorded as the 80-bit register image.

Usage: native_ability_state.py  (AA.exe is located by the shared harness)
"""
from pathlib import Path
import random
import struct
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import harness as h  # noqa: E402

SPECIAL = (0x00000000, 0x80000000, 0x00000001, 0x80000001, 0x3F800000, 0xBF800000, 0x7F7FFFFF,
           0xFF7FFFFF, 0x7F800000, 0xFF800000, 0x7FC00000, 0xFFC00000, 0x7F800001, 0x3A83126F)


def f32(v):
    return struct.unpack('<I', struct.pack('<f', v))[0]


def value(rng):
    if rng.random() < 0.6:
        return rng.choice(SPECIAL)
    return f32(rng.uniform(-5, 5))


def finite(rng):
    return rng.choice((0, 0x80000000, f32(1.0), f32(0.5), f32(rng.uniform(-3, 10)), f32(rng.uniform(0, 100)),
                       f32(rng.uniform(0, 1e-3)), 0x00000001))


def main():
    rng = random.Random(0x4C85D0)
    predicates, getters, visited = [], [], {}
    for precision in h.PRECISIONS:
        m = h.Machine(precision)
        for _ in range(250):
            obj = m.alloc(0x80)
            fields = {'f10': value(rng), 'f14': value(rng), 'f38': value(rng), 'f4c': value(rng),
                      'flags54': rng.choice((0, 0x20, 0xFFFFFFDF, 0xFFFFFFFF, rng.getrandbits(32)))}
            if rng.random() < 0.2:
                fields['f14'] = fields['f10']
            m.w(obj + 0x10, fields['f10'])
            m.w(obj + 0x14, fields['f14'])
            m.w(obj + 0x38, fields['f38'])
            m.w(obj + 0x4C, fields['f4c'])
            m.w(obj + 0x54, fields['flags54'])
            results = {}
            for name, address in (('45D430', 0x45D430), ('4C85D0', 0x4C85D0), ('4C8100', 0x4C8100),
                                  ('4C8120', 0x4C8120)):
                results[name] = m.call(address, ecx=obj, pop=0) & 0xFF
            predicates.append({'precision': precision, **fields, 'results': results})
        for _ in range(150):
            definition = m.alloc(0x400)
            base = finite(rng)
            m.w(definition + 0x308, base)
            setting = finite(rng)
            h.install_settings(m, {0x1A4: setting})
            context, delta = 0, None
            if rng.random() < 0.8:
                delta = finite(rng)
                context = m.alloc(0x80)
                m.w(context + 0x24, delta)
            m.call(0x45D4F0, ecx=definition, args=(context,), pop=4)
            st = m.st0_ext()
            getters.append({'precision': precision, 'base': base, 'delta24': delta, 'setting': setting,
                            'mantissa': f'{st["mantissa"]:016x}', 'sign_exponent': st['sign_exponent']})
        visited.update(m.visited)
    h.report('native-ability-state-fixtures.json',
             '45D430, 4C85D0, 4C8100, 4C8120 predicates and 45D4F0 (+501CF0) getter; all precisions.',
             predicates, visited, extra={'getters': getters})


if __name__ == '__main__':
    main()
