"""Original 2048-entry random-direction table at 0xB39A90 (global 0xB39AF8).

Static initializer 0x9C8680 calls constructor 0x408ED0(count 0x800), which
fills entries through 0x408D50 and virtual 0x4091F0: binary32 angle
i * 2pi / 2048, FCOS/FSIN stored to binary32, then normalization 0x409230.
The unchanged x86 executes under Unicorn (FSIN/FCOS are Unicorn's x87
emulation, the same acceptance basis as research/phase4-trig); only the
allocator 0x6F7810 is a seam. Tables are compared across PC24/53/64.
"""
import hashlib, json, struct, sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
from harness import Machine, VISITED, ORIGINAL_SHA, V14_SHA, HERE, PRECISION  # noqa: E402


def build(precision):
    m = Machine(precision)
    m.seams[0x6f7810] = lambda mm: mm.ret(0, mm.alloc(mm.arg()))
    m.call(0x408ed0, 0xb39a90, [0x800])
    assert m.r32(0xb39af8) == 0xb39a90 and m.r32(0xb39a90 + 4) == 0x800
    assert m.fpu_top() == 0
    table = m.r32(m.r32(0xb39a90 + 8), 0x1000)
    return table, {'vtable': hex(m.r32(0xb39a90)), 'count': m.r32(0xb39a90 + 4),
                   'period_bits': hex(m.r32(0xb39a90 + 0xc)), 'initialized_byte': m.r8(0xb39a90 + 0x10)}


def main():
    tables = {p: build(p) for p in PRECISION}
    reference = tables['Double53'][0]
    identical = {p: t[0] == reference for p, t in tables.items()}
    raw = struct.pack('<4096I', *reference)
    (HERE / 'direction-table.bin').write_bytes(raw)
    doc = {'schema': 1, 'scope': __doc__, 'original_sha256': ORIGINAL_SHA, 'v14_sha256': V14_SHA,
           'object': tables['Double53'][1], 'identical_across_precision': identical,
           'binary_file': 'direction-table.bin', 'binary_sha256': hashlib.sha256(raw).hexdigest(),
           'per_precision_sha256': {p: hashlib.sha256(struct.pack('<4096I', *t[0])).hexdigest()
                                    for p, t in tables.items()},
           'instructions': [{'va': hex(a), 'bytes': b} for a, b in sorted(VISITED.items())]}
    (HERE / 'native-direction-table.json').write_text(json.dumps(doc, indent=1) + '\n')
    print(json.dumps({k: doc[k] for k in ('identical_across_precision', 'binary_sha256', 'object')}))


if __name__ == '__main__':
    main()
