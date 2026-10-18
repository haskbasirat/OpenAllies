"""Native oracle for the CRT pow (_CIpow 0x96D890 -> SSE2 body 0x977A10/0x977A29)
and ___libm_error_support 0x977111 (default _matherr 0x84D110, both native).

Runs UNCHANGED original instructions in Unicorn with MXCSR 0x1F80 and x87 CW
0x27F. The SSE2 body is integer/binary64 arithmetic, which Unicorn computes
exactly, with two documented emulator deviations that are detected per case
instead of being trusted:
  * QEMU 5 (Unicorn 2.1.4) picks between TWO NaN operands of an SSE
    arithmetic instruction with the x87 rule (larger significand), whereas
    SSE hardware returns the first (destination) operand (Intel SDM vol. 1,
    table 4-7). Every such instruction execution is detected from the operand
    values and the case is flagged 'nan2'; Rust checks only that the result is
    a quiet NaN for flagged cases.
  * cvtsd2si out-of-range handling; every execution is checked to be in range.
  * Unicorn 2.1.4 does NOT quiet a signalling NaN in x87 m64 conversions
    (probed: fld m64 / fstp m64 keep 0x7FF0000000000001), whereas hardware
    with IM masked delivers the QNaN (Intel SDM FLD/FST #IA masked response).
    Every executed fld/fst/fstp m64 in the four routines is checked; an SNaN
    operand is corrected to the hardware QNaN (fld: memory operand quieted for
    that one load and restored; fst(p): stored value quieted) and the case is
    flagged 'x87q'. In game play x87 registers never hold an SNaN, so these
    rows only exercise direct SNaN inputs.
Seam: _errno (0x9710F1, CRT per-thread data) returns a scratch cell; its writes
are recorded. Everything else (dispatch, body, error support, _matherr, the
security cookie check) is original code.
Incoming XMM register contents are randomised twice per case and must not
change the result.
"""
from pathlib import Path
import math
import random
import struct
import sys

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import harness as h  # noqa: E402
from capstone import Cs, CS_ARCH_X86, CS_MODE_32  # noqa: E402
from capstone.x86 import X86_OP_MEM, X86_OP_REG  # noqa: E402
from unicorn import UC_HOOK_CODE  # noqa: E402
from unicorn import x86_const as X  # noqa: E402

BODY, BODY_END = 0x977A29, 0x977A29 + 2861
CIPOW, SSE2_ENTRY, X87_ENTRY = 0x96D890, 0x977A10, 0x96D8ED
ERROR_SUPPORT, ERRNO_FN = 0x977111, 0x9710F1
SSE2_FLAG = 0xB4B340
XMM = [getattr(X, f'UC_X86_REG_XMM{i}') for i in range(8)]
GPR = {'eax': X.UC_X86_REG_EAX, 'ecx': X.UC_X86_REG_ECX, 'edx': X.UC_X86_REG_EDX,
       'ebx': X.UC_X86_REG_EBX, 'esp': X.UC_X86_REG_ESP, 'ebp': X.UC_X86_REG_EBP,
       'esi': X.UC_X86_REG_ESI, 'edi': X.UC_X86_REG_EDI}
ARITH = {'addsd', 'subsd', 'mulsd', 'divsd', 'addpd', 'subpd', 'mulpd'}


def is_nan(bits):
    return (bits & 0x7FFFFFFFFFFFFFFF) > 0x7FF0000000000000


def quiet(bits):
    return bits | (1 << 51) if is_nan(bits) else bits


def d(value):
    return struct.unpack('<Q', struct.pack('<d', value))[0]


def ext_bytes(bits):
    """Exact binary64 -> x87 80-bit encoding, all classes (SNaN stays SNaN)."""
    sign, e, frac = bits >> 63, (bits >> 52) & 0x7FF, bits & ((1 << 52) - 1)
    if e == 0x7FF:
        mant, exp = (1 << 63) | (frac << 11), 0x7FFF
    elif e == 0:
        if frac == 0:
            mant, exp = 0, 0
        else:
            n = frac.bit_length()
            mant = frac << (64 - n)
            exp = 16383 + 63 - 1074 - (64 - n)
    else:
        mant, exp = (1 << 63) | (frac << 11), e - 1023 + 16383
    return struct.pack('<QH', mant, (sign << 15) | exp)


def ext_to_f64(mant, top):
    """Exact x87 80-bit -> binary64 bits; asserts the value is a binary64."""
    sign, exp = top >> 15, top & 0x7FFF
    if exp == 0x7FFF:
        assert mant >> 63 == 1 and mant & 0x7FF == 0, hex(mant)
        return (sign << 63) | (0x7FF << 52) | ((mant >> 11) & ((1 << 52) - 1))
    if mant == 0:
        assert exp == 0
        return sign << 63
    assert mant >> 63 == 1
    e = exp - 16383
    if e >= -1022:
        assert e <= 1023 and mant & 0x7FF == 0, (e, hex(mant))
        return (sign << 63) | ((e + 1023) << 52) | ((mant >> 11) & ((1 << 52) - 1))
    shift = 11 + (-1022 - e)
    assert shift < 64 and mant & ((1 << shift) - 1) == 0, (e, hex(mant))
    return (sign << 63) | (mant >> shift)


md = Cs(CS_ARCH_X86, CS_MODE_32)
md.detail = True
BODY_INSNS = {i.address: i for i in md.disasm(h.ORIGINAL.at(BODY, BODY_END - BODY), BODY)}
assert sum(i.size for i in BODY_INSNS.values()) == BODY_END - BODY
X87_MEM64 = {}
for start, size in ((CIPOW, 84), (SSE2_ENTRY, 25), (BODY, BODY_END - BODY), (ERROR_SUPPORT, 654)):
    for insn in md.disasm(h.ORIGINAL.at(start, size), start):
        ops = insn.operands
        if insn.mnemonic in ('fld', 'fst', 'fstp') and len(ops) == 1 and ops[0].type == X86_OP_MEM:
            if ops[0].size == 8:
                X87_MEM64[insn.address] = insn


def is_snan(bits):
    return is_nan(bits) and not bits & (1 << 51)


class Oracle(h.Machine):
    """Machine with a per-address pin cache (same check, executed once per pc)."""

    def __init__(self):
        super().__init__('Double53')
        self.pinned = set()
        self.u.reg_write(X.UC_X86_REG_MXCSR, 0x1F80)
        self.w(SSE2_FLAG, 1)
        self.errno_cell = self.alloc(4)
        self.hooks[ERRNO_FN] = lambda m: m.ret(0, m.errno_cell)
        self.hooks[ERROR_SUPPORT] = self._error_support
        self.u.hook_add(UC_HOOK_CODE, self._body, begin=BODY, end=BODY_END - 1)
        self.stub = self.STUBS + 0x800
        self.ext_x = self.SCRATCH + 0x40
        self.ext_y = self.SCRATCH + 0x50
        code = b'\xdb\x2d' + struct.pack('<I', self.ext_x) + b'\xdb\x2d' + struct.pack('<I', self.ext_y)
        code += b'\xe8' + struct.pack('<i', CIPOW - (self.stub + len(code) + 5)) + b'\xc3'
        self.u.mem_write(self.stub, code)
        self.rng = random.Random(0x977A29)
        self.pending = []
        self.x87q = False

    def _address(self, insn, mem):
        a = mem.disp
        if mem.base:
            a += self.u.reg_read(GPR[insn.reg_name(mem.base)])
        if mem.index:
            a += self.u.reg_read(GPR[insn.reg_name(mem.index)]) * mem.scale
        return a & 0xFFFFFFFF

    def _x87(self, pc):
        for kind, a, value in self.pending:
            if kind == 'restore':
                self.u.mem_write(a, struct.pack('<Q', value))
            else:
                stored = struct.unpack('<Q', self.u.mem_read(a, 8))[0]
                if is_snan(stored):
                    self.u.mem_write(a, struct.pack('<Q', quiet(stored)))
                    self.x87q = True
        self.pending = []
        insn = X87_MEM64.get(pc)
        if insn is None:
            return
        a = self._address(insn, insn.operands[0].mem)
        if insn.mnemonic == 'fld':
            value = struct.unpack('<Q', self.u.mem_read(a, 8))[0]
            if is_snan(value):
                self.u.mem_write(a, struct.pack('<Q', quiet(value)))
                self.pending.append(('restore', a, value))
                self.x87q = True
        else:
            self.pending.append(('store', a, None))

    def _code(self, uc, pc, size, _):
        self._x87(pc)
        if pc not in self.pinned and h.ORIGINAL.base <= pc < h.ORIGINAL.base + h.ORIGINAL.size:
            raw = h.ORIGINAL.at(pc, size)
            assert raw == h.ACCEPTED.at(pc, size), hex(pc)
            self.visited[pc] = raw.hex()
            self.pinned.add(pc)
        handler = self.hooks.get(pc)
        if handler is not None:
            handler(self)

    def _error_support(self, m):
        # Observation only: the native body executes.
        self.codes.append(self.arg(3))

    def _operand(self, insn, op, lane_count):
        if op.type == X86_OP_REG:
            value = self.u.reg_read(XMM[int(insn.reg_name(op.reg)[3:])])
            return [(value >> (64 * k)) & ((1 << 64) - 1) for k in range(lane_count)]
        raw = self.u.mem_read(self._address(insn, op.mem), 8 * lane_count)
        return list(struct.unpack('<%dQ' % lane_count, raw))

    def _body(self, uc, pc, size, _):
        insn = BODY_INSNS[pc]
        self.body_visited.add(pc)
        if insn.mnemonic in ARITH:
            lanes = 2 if insn.mnemonic.endswith('pd') else 1
            dst = self._operand(insn, insn.operands[0], lanes)
            src = self._operand(insn, insn.operands[1], lanes)
            # Ambiguous only when the two quieted NaNs differ (equal ones give the
            # same result under both pick rules).
            if any(is_nan(a) and is_nan(b) and quiet(a) != quiet(b) for a, b in zip(dst, src)):
                self.nan2 = True
        elif insn.mnemonic == 'cvtsd2si':
            value = struct.unpack('<d', struct.pack('<Q', self._operand(insn, insn.operands[1], 1)[0]))[0]
            assert not math.isnan(value) and -2147483648.5 < value < 2147483647.5, value

    def _garbage(self):
        for reg in XMM:
            self.u.reg_write(reg, self.rng.getrandbits(128))

    def _prepare(self):
        self.codes = []
        self.nan2 = False
        self.x87q = False
        assert not self.pending
        self.w(self.errno_cell, 0xDEADBEEF)

    def _finish(self):
        st = self.st0_ext()
        errno = self.r(self.errno_cell)
        return (ext_to_f64(st['mantissa'], st['sign_exponent']), self.codes[:],
                None if errno == 0xDEADBEEF else errno, self.nan2, self.x87q)

    def body(self, x, y):
        """Direct entry at 0x977A29 with 0x977A10's stack layout ([esp+4] 16-aligned)."""
        self._prepare()
        self._garbage()
        sp = self.STACK - 0x100 - 4
        self.w(sp, self.STOP)
        self.u.mem_write(sp + 4, struct.pack('<QQ', x, y))
        self.u.reg_write(X.UC_X86_REG_ESP, sp)
        saved = {X.UC_X86_REG_EBX: 0x5A5A0001, X.UC_X86_REG_ESI: 0x5A5A0002,
                 X.UC_X86_REG_EDI: 0x5A5A0003, X.UC_X86_REG_EBP: 0x5A5A0004}
        for reg, value in saved.items():
            self.u.reg_write(reg, value)
        self.u.emu_start(BODY, self.STOP, count=20000)
        assert self.reg(X.UC_X86_REG_EIP) == self.STOP
        assert self.reg(X.UC_X86_REG_ESP) == sp + 4
        for reg, value in saved.items():
            assert self.reg(reg) == value
        return self._finish()

    def cipow(self, x, y):
        """Full _CIpow: x87 operands ST1 = x, ST0 = y loaded exactly (fld tbyte)."""
        self._prepare()
        self._garbage()
        self.u.mem_write(self.ext_x, ext_bytes(x))
        self.u.mem_write(self.ext_y, ext_bytes(y))
        self.call(self.stub, args=(), pop=0)
        return self._finish()


def dispatch_cases():
    out = []
    for flag in (0, 1):
        for mxcsr in (0x1F80, 0x1F00, 0x1E80, 0x0F80, 0x9FC0, 0x3F80, 0x1FBF):
            for cw in (0x27F, 0x37F, 0x07F, 0x27E, 0x23F, 0x1272):
                m = h.Machine('Double53')
                m.w(SSE2_FLAG, flag)
                m.u.reg_write(X.UC_X86_REG_MXCSR, mxcsr)
                m.u.reg_write(X.UC_X86_REG_FPCW, cw)
                reached = []

                def stop(mm, path):
                    reached.append(path)
                    mm.u.emu_stop()
                m.hooks[SSE2_ENTRY] = lambda mm: stop(mm, 'Sse2')
                m.hooks[X87_ENTRY] = lambda mm: stop(mm, 'X87')
                sp = m.STACK - 0x40
                m.w(sp, m.STOP)
                m.u.reg_write(X.UC_X86_REG_ESP, sp)
                m.u.emu_start(CIPOW, m.STOP, count=100)
                assert len(reached) == 1, reached
                out.append({'sse2_flag': flag, 'mxcsr': mxcsr, 'x87_control': cw, 'path': reached[0]})
    return out


SPECIAL = sorted(set([
    0, 1 << 63, 1, 0x8000000000000001, 0x000FFFFFFFFFFFFF, 0x800FFFFFFFFFFFFF, 0x0008000000000000,
    0x0010000000000000, 0x8010000000000000, 0x7FEFFFFFFFFFFFFF, 0xFFEFFFFFFFFFFFFF,
    0x7FF0000000000000, 0xFFF0000000000000, 0x7FF8000000000000, 0xFFF8000000000000,
    0x7FF8000000000123, 0x7FF0000000000001, 0xFFF4000000000000, 0x7FF7FFFFFFFFFFFF,
    0x3FEFFFFFFFFFFFFF, 0x3FF0000000000001, 0xBFEFFFFFFFFFFFFF, 0xBFF0000000000001,
] + [d(v) for v in (
    0.5, -0.5, 1.0, -1.0, 1.5, -1.5, 2.0, -2.0, 3.0, -3.0, 10.0, -10.0, 0.1, 1e300, -1e300, 1e-300,
    2.0 ** 53, -2.0 ** 53, 2.0 ** 53 + 2, 2.0 ** 52 + 1, -(2.0 ** 52 + 1), 2.0 ** 52 + 0.5, 2.0 ** 63,
    2.0 ** 64, -2.0 ** 64, 2.0 ** 31 - 1, 2.0 ** 31 + 1, -(2.0 ** 31 + 1), 0.25, 4.0, 1024.0, -1074.0,
    -1075.0, 1023.9, 0.75, 2.5, -2.5, 7.0, -7.0, 0.3333333333333333, 1.0000001, 0.9999999)]))


def random_pairs(rng):
    pairs = []
    for _ in range(600):
        pairs.append((rng.getrandbits(64), rng.getrandbits(64)))
    for _ in range(600):
        pairs.append((d(rng.uniform(0, 8)), d(rng.uniform(-16, 16))))
    for _ in range(400):
        k = rng.randint(1, 52)
        x = 1 + rng.choice((-1, 1)) * rng.random() * 2.0 ** -k
        y = rng.choice((-1, 1)) * rng.random() * 2.0 ** rng.randint(0, 60)
        pairs.append((d(x), d(y)))
    for _ in range(400):
        x = -rng.uniform(0.01, 10)
        y = float(rng.choice((rng.randint(-64, 64), rng.randint(-2 ** 60, 2 ** 60), rng.randint(-2 ** 20, 2 ** 20))))
        pairs.append((d(x), d(y)))
    for _ in range(400):
        x = rng.uniform(1.0001, 100) if rng.random() < 0.5 else rng.uniform(0.01, 0.9999)
        t = rng.choice((1024, 1023.5, -1022, -1074, -1075, -1060, -1080))
        y = t / math.log2(x) * (1 + rng.uniform(-1e-9, 1e-9))
        pairs.append((d(x), d(y)))
    for _ in range(300):
        x = struct.unpack('<f', struct.pack('<f', rng.uniform(0, 2)))[0]
        y = struct.unpack('<f', struct.pack('<f', rng.uniform(0, 4)))[0]
        pairs.append((d(x), d(y)))
    for _ in range(300):
        x = rng.getrandbits(52) | (rng.getrandbits(1) << 63)
        y = d(rng.uniform(-3, 3))
        pairs.append((x, y))
    return pairs


def libm_cases(m):
    values = [(d(2.5), d(-3.25)), (0x7FF0000000000001, d(1.0)), (d(1.0), 0xFFF0000000000003),
              (1, 0x7FF8000000000005), (1 << 63, 0x000FFFFFFFFFFFFF)]
    codes = (list(range(-2, 0x40)) + list(range(0x9E, 0xB0)) + list(range(0x3E5, 0x3F5))
             + [0x7FFFFFFF, 0x80000000])
    out = []
    for code in codes:
        for x, r in values:
            y = d(-0.75)
            px, py, pr = m.alloc(8), m.alloc(8), m.alloc(8)
            m.u.mem_write(px, struct.pack('<Q', x))
            m.u.mem_write(py, struct.pack('<Q', y))
            m.u.mem_write(pr, struct.pack('<Q', r))
            m.codes = []
            m.x87q = False
            m.w(m.errno_cell, 0xDEADBEEF)
            m.call(ERROR_SUPPORT, args=(px, py, pr, code), pop=0)
            result = struct.unpack('<Q', m.u.mem_read(pr, 8))[0]
            errno = m.r(m.errno_cell)
            out.append([code, f'{x:016x}', f'{y:016x}', f'{r:016x}', f'{result:016x}',
                        None if errno == 0xDEADBEEF else errno, m.x87q])
    return out


def main():
    m = Oracle()
    m.body_visited = set()
    cases = []
    nan2 = 0
    pairs = [(x, y) for x in SPECIAL for y in SPECIAL]
    for x, y in pairs:
        a = m.cipow(x, y)
        b = m.cipow(x, y)
        assert a == b, (hex(x), hex(y))
        nan2 += a[3]
        cases.append(['cipow', f'{x:016x}', f'{y:016x}', f'{a[0]:016x}',
                      a[1][0] if a[1] else None, a[2], a[3], a[4]])
        assert len(a[1]) <= 1
    for x, y in pairs + random_pairs(m.rng):
        a = m.body(x, y)
        b = m.body(x, y)
        assert a == b, (hex(x), hex(y))
        nan2 += a[3]
        cases.append(['body', f'{x:016x}', f'{y:016x}', f'{a[0]:016x}',
                      a[1][0] if a[1] else None, a[2], a[3], a[4]])
        assert len(a[1]) <= 1
    leaders = set()
    insns = sorted(BODY_INSNS)
    for i in insns:
        ins = BODY_INSNS[i]
        if ins.mnemonic.startswith('j'):
            leaders.add(ins.operands[0].imm)
            leaders.add(i + ins.size)
        if ins.mnemonic == 'ret':
            leaders.add(i + ins.size)
    leaders = sorted(t for t in leaders | {BODY} if t in BODY_INSNS)
    uncovered = [hex(t) for t in leaders if t not in m.body_visited]
    libm = libm_cases(m)
    dispatch = dispatch_cases()
    codes = sorted({c[4] for c in cases if c[4] is not None})
    print(f'pow cases {len(cases)}, nan2 {nan2}, libm codes seen {codes}, '
          f'blocks {len(leaders) - len(uncovered)}/{len(leaders)} uncovered {uncovered}')
    h.report('native-crt-pow-fixtures.json',
             '_CIpow 0x96D890 dispatch, SSE2 body 0x977A10/0x977A29 and ___libm_error_support 0x977111 '
             '(default _matherr 0x84D110 native; _errno 0x9710F1 seam). Case row: [entry, x, y, result '
             'bits (ST0 as binary64), libm code, errno, nan2, x87q]. nan2 = an SSE arithmetic instruction '
             'saw two distinct NaN operands (Unicorn uses the x87 pick rule there; only NaN-ness is ground '
             'truth). x87q = an x87 m64 conversion of an SNaN was corrected to the hardware QNaN. '
             'XMM inputs randomised twice per case with identical results.',
             cases, m.visited,
             extra={'mxcsr': 0x1F80, 'x87_control': 0x27F, 'block_leaders': len(leaders),
                    'uncovered_blocks': uncovered,
                    'uncovered_reason': '0x97811A/0x978139 test x == -1.0 inside 0x9780E8, which is '
                                        'entered only after the first range check accepted x with '
                                        'high word 0x0010..0x7FEF (positive normal); unreachable.',
                    'libm_cases': libm, 'dispatch_cases': dispatch,
                    'libm_row': '[code, x, y, result in, result out, errno, x87q]'})


if __name__ == '__main__':
    main()
