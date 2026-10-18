"""Exact check of the 4096-entry binary32 cos/sin tables (B39A48/B39A60).

The tables are filled by the static initializers 9C8620 (cos) / 9C8640 (sin)
-> 409040 / 4090E0 -> 408DE0, which for i = 0..4095 computes
    fild i; fmul dword [0x40C90FDB]; fidiv dword [4096]; fstp dword [esp]
and calls the table vtable slot +8: 4090C0 (fld dword arg; fcos) or 409160
(fld dword arg; fsin); the result is stored with fstp dword (round to
nearest, CW 0x027F at static init).

Unicorn computes FSIN/FCOS through host binary64, so the emulated tables are
not ground truth by themselves. This script proves the table bits without
relying on any FSIN/FCOS implementation:

1. The argument x_i is reconstructed from the instructions: the product
   i * f32(2*pi) has at most 36 significant bits (exact at PC53/PC64; at PC24
   rounded to 24 bits), the division by 4096 is exact, the binary32 store
   rounds once, so x_i = round_binary32(i * 0x40C90FDB / 4096) at every PC.
   The reconstruction is compared with the argument captured from the
   emulator at the FCOS/FSIN instruction (all 3 control words).
2. cos/sin(x_i) is evaluated with mpmath at 256 bits and rounded to binary32
   (nearest-even); the table entry (aa-trig's lookup-tables.bin and the
   emulated table) must equal it bit for bit.
3. Margin: the distance d of the exact value to the nearest binary32 rounding
   boundary (midpoint toward the neighbour on the exact value's side) must
   exceed the error bound
       E = 2 * ulp53(v) + 2^-62
   ulp53(v) = 2^(floor(log2|v|) - 52). This covers, with slack:
   - the x87 FSIN/FCOS accuracy claim (Intel SDM: < 1 ulp of the 64-bit
     significand, round to nearest) and, should precision control apply to
     the result, a rounding to 53 bits (< 0.5 ulp53);
   - the 66-bit pi argument reduction for |x| <= 2*pi: the reduction uses at
     most k <= 4 multiples of pi/2 with |pi/2 - pi66/2| <= 2^-66, so the
     reduced argument is off by <= 2^-64 and (|f'| <= 1) the result by
     <= 2^-64 absolute (bound used: 2^-62);
   - Unicorn's binary64 path (correctly rounded or 1-ulp host libm, then
     conversion) at <= 1 ulp53.
   Any hardware result within E of the exact value rounds to the same
   binary32. Entries with exact results (sin 0 = 0, cos 0 = 1; FSIN/FCOS of
   +0 are exact) are reported separately.
Output: trig-tables-exact.json.
"""
import json, hashlib, struct
from fractions import Fraction
from pathlib import Path
import mg_machine
from mg_machine import HERE, ROOT
from mg_world import install_crt, install_static_tables
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ESP
import mpmath

mpmath.mp.prec = 256
TRIG_BIN = ROOT / 'research/phase4-trig/lookup-tables.bin'
TWO_PI_BITS = 0x40C90FDB
COUNT = 4096
FCOS_AT, FSIN_AT = 0x4090C4, 0x409164


def f32_value(bits):
    """Exact Fraction of a finite binary32."""
    e = (bits >> 23) & 0xff
    m = bits & 0x7fffff
    assert e != 0xff
    v = Fraction(m, 1 << 23) * Fraction(2) ** -126 if e == 0 else (1 + Fraction(m, 1 << 23)) * Fraction(2) ** (e - 127)
    return -v if bits >> 31 else v


def round_f32(v):
    """Nearest-even binary32 bits of an exact Fraction (normal range or 0)."""
    if v == 0:
        return 0
    sign = 0x80000000 if v < 0 else 0
    a = abs(v)
    e = a.numerator.bit_length() - a.denominator.bit_length()
    if Fraction(2) ** e > a:
        e -= 1
    assert -126 <= e <= 127, e
    scaled = a / Fraction(2) ** (e - 23)              # in [2^23, 2^24)
    q, r = divmod(scaled.numerator, scaled.denominator)
    rem = Fraction(r, scaled.denominator)
    if rem > Fraction(1, 2) or (rem == Fraction(1, 2) and q & 1):
        q += 1
    if q == 1 << 24:
        q >>= 1
        e += 1
    return sign | ((e + 127) << 23) | (q - (1 << 23))


def next_toward(bits, up):
    """Adjacent binary32 (by magnitude) for a positive or negative normal."""
    if bits & 0x7fffffff == 0:
        return 0x00000001 if up else 0x80000001
    mag_up = (bits >> 31 == 0) == up
    return bits + 1 if mag_up else bits - 1


def mp_fraction(x):
    """Exact Fraction of a finite mpf (mpmath keeps the sign separately)."""
    sign, man, exp, _ = x._mpf_
    v = Fraction(int(man)) * (Fraction(2) ** int(exp))
    return -v if sign else v


def argument(i):
    return round_f32(Fraction(i) * f32_value(TWO_PI_BITS) / COUNT)


def captured_arguments(cw):
    m = mg_machine.Machine(cw)
    install_crt(m)
    seen = {'cos': [], 'sin': []}
    def grab(name):
        def hook(u, pc, size, _):
            seen[name].append(m.r32(u.reg_read(UC_X86_REG_ESP) + 4))
        return hook
    m.u.hook_add(UC_HOOK_CODE, grab('cos'), begin=FCOS_AT, end=FCOS_AT)
    m.u.hook_add(UC_HOOK_CODE, grab('sin'), begin=FSIN_AT, end=FSIN_AT)
    install_static_tables(m)
    tables = {name: list(struct.unpack('<4096I', bytes(m.u.mem_read(m.r32(obj + 8), 0x4000))))
              for name, obj in (('cos', 0xB39A48), ('sin', 0xB39A60))}
    return seen, tables


def main():
    trig = TRIG_BIN.read_bytes()
    aa = {'cos': list(struct.unpack('<4096I', trig[:0x4000])), 'sin': list(struct.unpack('<4096I', trig[0x4000:]))}
    args = [argument(i) for i in range(COUNT)]
    captures = []
    for cw in (0x027F, 0x007F, 0x037F):
        seen, emulated = captured_arguments(cw)
        captures.append({'control_word': cw,
                         'arguments_equal_reconstruction': seen['cos'] == args and seen['sin'] == args,
                         'emulated_tables_equal_aa_trig': emulated == aa})
        assert captures[-1]['arguments_equal_reconstruction'] and captures[-1]['emulated_tables_equal_aa_trig'], captures[-1]
    report_tables = {}
    exact_bytes = b''
    for name, fn in (('cos', mpmath.cos), ('sin', mpmath.sin)):
        rows, exact_entries, mismatches, failures = [], [], [], []
        exact_bits = []
        for i, xb in enumerate(args):
            x = f32_value(xb)
            v = mp_fraction(fn(mpmath.mpf(x.numerator) / x.denominator))
            r = round_f32(v)
            exact_bits.append(r)
            if r != aa[name][i]:
                mismatches.append(i)
            if x == 0:                                  # sin(+0) = +0, cos(+0) = 1: exact on the FPU
                exact_entries.append(i)
                continue
            neighbour = next_toward(r, v > f32_value(r))
            mid = (f32_value(r) + f32_value(neighbour)) / 2
            d = abs(v - mid)
            e = abs(v).numerator.bit_length() - abs(v).denominator.bit_length()
            if Fraction(2) ** e > abs(v):
                e -= 1
            bound = 2 * Fraction(2) ** (e - 52) + Fraction(2) ** -62
            ratio = d / bound
            rows.append((ratio, i, xb, float(v), float(d)))
            if d <= bound:
                failures.append({'index': i, 'argument_bits': xb, 'exact': mpmath.nstr(fn(mpmath.mpf(x.numerator) / x.denominator), 30),
                                 'distance_to_boundary': float(d), 'bound': float(bound)})
        rows.sort()
        exact_bytes += struct.pack('<4096I', *exact_bits)
        report_tables[name] = {
            'entries': COUNT,
            'table_equals_exact_rounding': not mismatches,
            'mismatched_indices': mismatches,
            'exact_results': exact_entries,
            'margin_failures': failures,
            'smallest_margins': [{'index': i, 'argument_bits': xb, 'value': v, 'distance_to_boundary': d,
                                  'distance_over_bound': float(ratio)} for ratio, i, xb, v, d in rows[:10]],
            'min_distance_over_bound': float(rows[0][0]),
        }
    report = {
        'schema': 1, 'scope': __doc__,
        'producer_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        'original_sha256': hashlib.sha256(mg_machine.P.b).hexdigest(),
        'aa_trig_tables_sha256': hashlib.sha256(trig).hexdigest(),
        'exact_rounding_tables_sha256': hashlib.sha256(exact_bytes).hexdigest(),
        'mpmath': {'version': mpmath.__version__, 'precision_bits': mpmath.mp.prec},
        'error_bound': '2 * ulp53(v) + 2^-62 (see scope)',
        'captures': captures,
        'tables': report_tables,
    }
    out = HERE / 'trig-tables-exact.json'
    out.write_text(json.dumps(report, indent=1) + '\n')
    print(json.dumps({name: {k: t[k] for k in ('table_equals_exact_rounding', 'min_distance_over_bound')}
                            | {'mismatches': len(t['mismatched_indices']), 'margin_failures': len(t['margin_failures'])}
                      for name, t in report_tables.items()}))
    print(json.dumps({'exact_rounding_tables_sha256': report['exact_rounding_tables_sha256'],
                      'aa_trig_tables_sha256': report['aa_trig_tables_sha256'],
                      'sha256': hashlib.sha256(out.read_bytes()).hexdigest()}))


if __name__ == '__main__':
    main()
