"""Unicorn oracle: the unstored 80-bit ST0 result of the angle routine 0x445400.

Executes the original 0x445400 -> CRT asin 0x96D780 -> 0x97792B unchanged
(no seams) at PC24/PC53/PC64, with the sticky precision-exception flag clear
or set on entry (the CRT tail takes a binary64 round trip iff PC != 53 and PE
is set). Records the 10-byte FSTP TBYTE image of ST0 after the call, i.e. the
value before any caller store. Inputs: finite binary32 vectors (random,
axis-aligned, opposite, parallel, tiny and large). Output: native-angle-extended.json.
"""
from pathlib import Path
import hashlib, json, random, struct, sys

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import native_tick_driver as D
from unicorn.x86_const import UC_X86_REG_FPSW, UC_X86_REG_FPCW

bits = D.bits


class M(D.Machine):
    def __init__(self, pc):
        super().__init__(pc)
        self.vec_from = self.alloc(8)
        self.vec_to = self.alloc(8)
        # push to; push from; call 445400; add esp,8; fstp tbyte [res80]; ret
        code = (b'\x68' + struct.pack('<I', self.vec_to) + b'\x68' + struct.pack('<I', self.vec_from)
                + b'\xe8\0\0\0\0' + b'\x83\xc4\x08' + b'\xdb\x3d' + struct.pack('<I', self.res80) + b'\xc3')
        self.tramp_angle = self.code(code)
        rel = 0x445400 - (self.tramp_angle + 10 + 5)
        self.u.mem_write(self.tramp_angle + 11, struct.pack('<i', rel))

    def angle(self, a, b, pe):
        self.u.mem_write(self.vec_from, struct.pack('<II', *a))
        self.u.mem_write(self.vec_to, struct.pack('<II', *b))
        self.u.reg_write(UC_X86_REG_FPCW, D.PC[self.pc])
        self.u.reg_write(UC_X86_REG_FPSW, 0x20 if pe else 0)
        self.call(self.tramp_angle, 0)
        return self.f80()


def vectors(seed=0x5EEDE):
    rng = random.Random(seed)
    out = []
    special = [(1.0, 0.0), (0.0, 1.0), (-1.0, 0.0), (0.0, -1.0), (0.6, 0.8), (-0.6, 0.8), (1e-20, 3e-20),
               (3.0, 4.0), (1e6, -2e6), (0.70710677, 0.70710677)]
    for a in special:
        for b in special:
            out.append((a, b))
    for _ in range(300):
        def v():
            k = rng.random()
            if k < 0.6:
                t = rng.uniform(-3.2, 3.2)
                import math
                return (math.cos(t), math.sin(t))
            if k < 0.9:
                return (rng.uniform(-50, 50), rng.uniform(-50, 50))
            return (rng.uniform(-1, 1) * 1e-3, rng.uniform(-1, 1) * 1e-3)
        out.append((v(), v()))
    return [((bits(a[0]), bits(a[1])), (bits(b[0]), bits(b[1]))) for a, b in out]


def main():
    data = D.NATIVE.range(0x445400, 0x445495 - 0x445400)
    assert data == D.ACCEPTED.range(0x445400, 0x445495 - 0x445400)
    rows = []
    vs = vectors()
    for pc in (24, 53, 64):
        m = M(pc)
        for pe in (False, True):
            for a, b in vs:
                rows.append({'precision': pc, 'precision_exception_pending': pe, 'from_bits': list(a),
                             'to_bits': list(b), 'st0_f80': m.angle(a, b, pe)})
    report = {'schema': 1, 'passed': True, 'source_sha256': D.digest(D.NATIVE.b), 'method': __doc__,
              'producer_sha256': D.digest(Path(__file__).read_bytes()),
              'executed_range': {'start': '0x445400', 'end_exclusive': '0x445495',
                                 'sha256': D.digest(data), 'unchanged_in_v14': True},
              'cases': rows}
    target = HERE / 'native-angle-extended.json'
    target.write_text(json.dumps(report, indent=1) + '\n')
    print(json.dumps({'cases': len(rows), 'sha256': D.digest(target.read_bytes())}))


if __name__ == '__main__':
    main()
