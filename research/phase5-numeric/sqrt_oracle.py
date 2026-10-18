"""Isolated synthetic x87 FSQRT values, including the unstored extended result.

This validates arithmetic, not game control flow. Actual callers have separate
unchanged-original-instruction fixtures owned by the actor/navigation modules.
"""
from pathlib import Path
import hashlib
import json
import struct
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT.parent / 'reconstruction/tools/python-packages'))
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32
from unicorn.x86_const import UC_X86_REG_FPCW


def f32(value):
    return struct.unpack('<I', struct.pack('<f', value))[0]


def f64(value):
    return struct.unpack('<Q', struct.pack('<d', value))[0]


def main():
    direct32 = {0, 0x80000000, 1, 2, 3, 0x7fffff, 0x800000, 0x7f7fffff}
    direct64 = {0, 0x8000000000000000, 1, 2, 3, 0xfffffffffffff, 0x10000000000000, 0x7fefffffffffffff}
    for n in [0.5, 1, 2, 3, 4, 0.1, 0.0001, 100, 16777215, 16777216, 16777217]:
        direct32.add(f32(n))
        direct64.add(f64(n))
    for e in range(1, 255):
        direct32.add(e << 23)
        direct32.add((e << 23) | 1)
    for e in range(1, 2047, 13):
        direct64.add(e << 52)
        direct64.add((e << 52) | 1)
    # Exactly representable input squares on both sides of a PC24 midpoint,
    # including even/odd ties and a carry into the next binade.
    for q in [2**23 + i for i in range(12)] + [2**24 - 1]:
        exact = f64(((2*q+1)**2) / 4)
        direct64.update([exact-1, exact, exact+1])
    random = 0x95ae317b
    pairs = [(f32(3), f32(4)), (0x7f7fffff, 0x7f7fffff), (1, 1), (0, 0)]
    for _ in range(192):
        random = (random*1664525+1013904223) & 0xffffffff
        x = random % 0x7f800000
        random = (random*1664525+1013904223) & 0xffffffff
        y = random % 0x7f800000
        pairs.append((x,y))
    specs = [('f32', [n]) for n in sorted(direct32)]
    specs += [('f64', [n]) for n in sorted(direct64)]
    specs += [('sum_squares_f32', list(pair)) for pair in pairs]
    rows = []
    code_by_kind = {}
    for name, control in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
        u = Uc(UC_ARCH_X86, UC_MODE_32)
        u.mem_map(0x100000, 0x4000)
        u.reg_write(UC_X86_REG_FPCW, control)
        context = u.context_save()
        for kind_index, kind in enumerate(['f32','f64','sum_squares_f32']):
            address = 0x100000 + kind_index*0x100
            if kind == 'sum_squares_f32':
                code = b'\xd9\x05'+struct.pack('<I',0x101000)+b'\xd8\x0d'+struct.pack('<I',0x101000)
                code += b'\xd9\x05'+struct.pack('<I',0x101008)+b'\xd8\x0d'+struct.pack('<I',0x101008)+b'\xde\xc1'
            else:
                code = (b'\xd9\x05' if kind=='f32' else b'\xdd\x05')+struct.pack('<I',0x101000)
            code += b'\xd9\xfa\xd9\xc0\xdb\x3d'+struct.pack('<I',0x102000)
            code += b'\xd9\x1d'+struct.pack('<I',0x102010)
            u.mem_write(address, code)
            code_by_kind[kind] = code.hex()
            for case_kind, values in specs:
                if case_kind != kind:
                    continue
                u.context_restore(context)
                for i,value in enumerate(values):
                    u.mem_write(0x101000+8*i,struct.pack('<Q',value))
                u.emu_start(address,address+len(code),count=100)
                raw = bytes(u.mem_read(0x102000,10))
                rows.append({'precision':name,'kind':kind,'inputs':values,
                             'result80':raw.hex(), 'result32':struct.unpack('<I',u.mem_read(0x102010,4))[0]})
    report = {'schema':1,'scope':'Synthetic x87 FSQRT in isolated Unicorn; extended results retained before final binary32 store. Not original game control-flow evidence.',
              'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              'instructions':code_by_kind,'cases':rows}
    output = HERE/'sqrt-vectors.json'
    output.write_text(json.dumps(report,indent=2),encoding='utf-8')
    print(json.dumps({'cases':len(rows),'sha256':hashlib.sha256(output.read_bytes()).hexdigest()}))


if __name__ == '__main__':
    main()
