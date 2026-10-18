"""Probe Unicorn x87 fidelity limits that matter for oracles (no AA.exe code).

1. Precision exception: FNINIT; FLD1; FDIV 3.0; FNSTSW -> real CPUs set PE (0x20).
2. FPATAN(1,1): real x87 returns pi/4 with significand 0xC90FDAA22168C235
   (the same 64-bit pattern as the original constant at 0xB359BA);
   a binary64-based emulation returns ...C000.
3. FSIN/FCOS/FYL2X spot checks for the same binary64 signature (low 11 bits zero).
Output: emulator-limits.json.
"""
from pathlib import Path
import json, struct
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32
from unicorn.x86_const import UC_X86_REG_EAX
import unicorn

HERE = Path(__file__).resolve().parent


def run(code, data=b''):
    u = Uc(UC_ARCH_X86, UC_MODE_32)
    u.mem_map(0x1000, 0x2000)
    u.mem_write(0x1000, code)
    u.mem_write(0x1800, data)
    u.emu_start(0x1000, 0x1000 + len(code))
    return u


def f80(u, a=0x1900):
    b = bytes(u.mem_read(a, 10))
    return {'significand': hex(int.from_bytes(b[:8], 'little')), 'sign_exponent': hex(int.from_bytes(b[8:], 'little'))}


def main():
    out = {'unicorn_version': unicorn.__version__}
    u = run(bytes.fromhex('dbe3d9e8d83500180000dfe0'), struct.pack('<f', 3.0))
    sw = u.reg_read(UC_X86_REG_EAX) & 0xFFFF
    out['fdiv_inexact_sets_PE'] = bool(sw & 0x20)
    out['fdiv_status_word'] = hex(sw)
    store = bytes.fromhex('db3d00190000')
    probes = {'fpatan_1_1': 'd9e8d9e8d9f3', 'fsin_1': 'd9e8d9fe', 'fcos_1': 'd9e8d9ff',
              'fyl2x_1_3': 'd9e8d90500180000d9f1'}
    for name, hexcode in probes.items():
        u = run(bytes.fromhex('dbe3') + bytes.fromhex(hexcode) + store, struct.pack('<f', 3.0))
        out[name] = f80(u)
    out['real_x87_reference'] = {'fpatan_1_1_significand': '0xc90fdaa22168c235',
                                 'note': 'pi/4 in extended precision; low 11 bits zero mean binary64-based emulation'}
    out['binary64_signature'] = {k: int(v['significand'], 16) & 0x7FF == 0 for k, v in out.items()
                                 if isinstance(v, dict) and 'significand' in v}
    target = HERE / 'emulator-limits.json'
    target.write_text(json.dumps(out, indent=1) + '\n')
    print(json.dumps(out, indent=1))


if __name__ == '__main__':
    main()
