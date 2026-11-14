"""Native oracle for the SAI chance roll 4A6540(rng, key, owner), all
precisions, with the real 85D840 draw and the real _CIpow 96D890 (SSE2 path:
[B4B340] = 1, MXCSR 0x1F80). Recorded seams: 7B9050 (lookup on the owner),
7B9570 / 7B96E0 (ST0 values on the looked-up object, binary64 inputs).

Usage: native_chance.py  (AA.exe is located by the shared harness)
"""
from pathlib import Path
import random
import struct
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import harness as h  # noqa: E402
import unicorn.x86_const as X  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX  # noqa: E402


def f32(v):
    return struct.unpack('<I', struct.pack('<f', v))[0]


def main():
    rng = random.Random(0x4A6540)
    cases, visited = [], {}
    for precision in h.PRECISIONS:
        m = h.Machine(precision)
        m.u.mem_map(0, 0x1000)
        m.u.reg_write(X.UC_X86_REG_MXCSR, 0x1F80)
        m.w(0xB4B340, 1)
        for _ in range(150):
            events = []
            this = m.alloc(0x900)
            fields = {'field804': f32(rng.choice((1.0, 0.5, 0.01, 2.0, 0.3))),
                      'field808': f32(rng.choice((1.0, 0.5, 2.0, 1.3))),
                      'field80c': f32(rng.choice((1.0, 0.5, 2.0, 1.7, 0.0))),
                      'field810': f32(rng.choice((0.0, 0.05, 0.2))),
                      'field814': f32(rng.choice((1.0, 0.8, 5.0)))}
            for k, off in (('field804', 0x804), ('field808', 0x808), ('field80c', 0x80C), ('field810', 0x810),
                           ('field814', 0x814)):
                m.w(this + off, fields[k])
            a = rng.choice((0.0, -1.5, 2.0, 0.25, 7.3, 1e-3, rng.uniform(0.01, 10)))
            if a > 0:
                b = rng.choice((0.0, 1.0, 3.7, 50.0, rng.uniform(0, 20)))
            else:
                b = rng.choice((0.0, -2.0, 1.0, rng.uniform(-5, 5)))
            a_bits = struct.unpack('<Q', struct.pack('<d', a))[0]
            b_bits = struct.unpack('<Q', struct.pack('<d', b))[0]
            key, owner = m.alloc(0x20), m.alloc(0x20)
            word8 = rng.getrandbits(32)
            m.w(key + 8, word8)
            obj = m.alloc(0x20)
            state = m.alloc(0x10)
            before = rng.getrandbits(32)
            m.w(state, before)

            def lookup(mm, ev=events, k=key, o=owner, ob=obj):
                assert (mm.reg(UC_X86_REG_ECX), mm.arg(0)) == (o, k)
                ev.append(['lookup'])
                mm.ret(4, ob)

            def v9570(mm, ev=events, ob=obj, bits=a_bits):
                assert mm.reg(UC_X86_REG_ECX) == ob
                ev.append(['7B9570', mm.arg(0)])
                mm.ret_ext(4, bits)

            def v96e0(mm, ev=events, ob=obj, bits=b_bits):
                assert mm.reg(UC_X86_REG_ECX) == ob and mm.arg(0) == 0
                ev.append(['7B96E0'])
                mm.ret_ext(4, bits)
            m.hooks[0x7B9050] = lookup
            m.hooks[0x7B9570] = v9570
            m.hooks[0x7B96E0] = v96e0
            result = m.call(0x4A6540, ecx=this, args=(state, key, owner), pop=12, limit=2000000) & 0xFF
            cases.append({'precision': precision, **fields, 'a': a_bits, 'b': b_bits, 'word8': word8,
                          'state': before, 'state_after': m.r(state), 'result': result, 'events': events})
        visited.update(m.visited)
    h.report('native-chance-fixtures.json', '4A6540 SAI chance roll (85D840, _CIpow SSE2), all precisions.',
             cases, visited)


if __name__ == '__main__':
    main()
