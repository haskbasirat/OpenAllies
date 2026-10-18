"""Execute unchanged native RNG constructors/helpers and the inline float conversion.

Only isolated original x86 is run. No game process, map or user state is changed.
The diagnostic GetFRand store boundary is not a claim of whole-engine FPU state.
"""
from pathlib import Path
import hashlib
import json
import struct
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
GAME = ROOT.parent
sys.path.insert(0, str(ROOT / 'research/engine-baseline'))
sys.path.insert(0, str(GAME / 'reconstruction/tools/python-packages'))
from audit_baseline import PE
from capstone import Cs, CS_ARCH_X86, CS_MODE_32
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_ECX, UC_X86_REG_ESI, UC_X86_REG_ESP, UC_X86_REG_FPCW

sha = lambda value: hashlib.sha256(value).hexdigest()
original = PE(GAME / 'AA.exe')
modified = PE(GAME / 'reconstruction/native-game/runtime-expanded-v14/AA.exe')
assert sha(original.b) == '5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26'
assert sha(modified.b) == '3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1'
RANGES = {'integer': (0x85d840, 0x85d86c), 'default_constructor': (0x85d870, 0x85d879),
          'seed_assignment': (0x85d880, 0x85d89c), 'seed_constructor': (0x85d8a0, 0x85d8c4),
          'bounded_integer': (0x40c8a0, 0x40c8b4), 'inline_float': (0x4f62ac, 0x4f62df),
          'float_constants': (0x9fda98, 0x9fdaa8)}
for start, end in RANGES.values():
    assert original.range(start, end - start) == modified.range(start, end - start)


class Machine:
    STATE = 0x2001000
    STACK = 0x3008000
    STOP = 0x300f000

    def __init__(self, control_word=0x037f):
        self.u = Uc(UC_ARCH_X86, UC_MODE_32)
        self.u.mem_map(original.base, (original.image_size + 4095) & ~4095)
        for _, rva, size, offset in original.sections:
            self.u.mem_write(original.base + rva, original.b[offset:offset + size])
        self.u.mem_map(0x2000000, 0x10000)
        self.u.mem_map(0x3000000, 0x10000)
        self.u.reg_write(UC_X86_REG_FPCW, control_word)
        self.initial = self.u.context_save()

    def w(self, address, value):
        self.u.mem_write(address, struct.pack('<I', value & 0xffffffff))

    def r(self, address):
        return struct.unpack('<I', self.u.mem_read(address, 4))[0]

    def reset(self, state):
        self.u.context_restore(self.initial)
        self.w(self.STATE, state)
        self.w(self.STACK, self.STOP)
        self.u.reg_write(UC_X86_REG_ESP, self.STACK)
        self.u.reg_write(UC_X86_REG_ECX, self.STATE)

    def call(self, address, argument=None):
        if argument is not None:
            self.w(self.STACK + 4, argument)
        self.u.emu_start(address, self.STOP, count=1000)
        assert self.u.reg_read(UC_X86_REG_ESP) == self.STACK + (4 if argument is None else 8)
        return self.u.reg_read(UC_X86_REG_EAX)

    def float_fragment(self, value, draw):
        self.reset(value)
        self.u.reg_write(UC_X86_REG_ESI, self.STATE - 0xd4)
        self.u.reg_write(UC_X86_REG_EAX, value)
        self.u.emu_start(0x4f62ac if draw else 0x4f62bb, 0x4f62df, count=1000)
        assert self.u.reg_read(UC_X86_REG_ESP) == self.STACK - 4
        return self.r(self.STACK + 8), self.r(self.STATE)


def main():
    seeds = [0, 1, 2, 0xffff, 0x10000, 0x12345678, 0x7fffffff, 0x80000000, 0xffffffff, 0x8e4b12a5]
    random = 0xc0decafe
    for _ in range(100):
        seeds.append(random)
        random = (random * 1664525 + 1013904223) & 0xffffffff
    constructors = []
    bounded = []
    machine = Machine()
    for address in [0x85d870, 0x85d880, 0x85d8a0]:
        for seed in seeds:
            machine.reset(0xdeadbeef)
            machine.call(address, None if address == 0x85d870 else seed)
            constructors.append({'address': hex(address), 'seed': seed, 'state': machine.r(machine.STATE)})
    for seed in seeds:
        for bound in [1, 2, 3, 7, 16, 65535, 65536, 0x7fffffff, 0x80000000, 0xffffffff]:
            machine.reset(seed)
            value = machine.call(0x40c8a0, bound)
            bounded.append({'state_before': seed, 'bound': bound, 'value': value, 'state_after': machine.r(machine.STATE)})
    # Include values adjacent to binary32 rounding boundaries and unsigned conversion.
    values = sorted(set(seeds + [max(0, min(0xffffffff, power + delta))
                                for power in [0, 1 << 23, 1 << 24, 1 << 25, 1 << 30, 1 << 31, 0xffffffff]
                                for delta in range(-257, 258)]))
    converted = []
    drawn = []
    for name, word in [('Single24', 0x007f), ('Double53', 0x027f), ('Extended64', 0x037f)]:
        machine = Machine(word)
        for value in values:
            result, state = machine.float_fragment(value, False)
            assert state == value
            converted.append({'precision': name, 'value': value, 'result_bits': result})
        for seed in seeds:
            result, state = machine.float_fragment(seed, True)
            drawn.append({'precision': name, 'state_before': seed, 'state_after': state, 'result_bits': result})
    report = {'schema': 1, 'original_sha256': sha(original.b), 'v14_sha256': sha(modified.b),
              'producer_sha256': sha(Path(__file__).read_bytes()),
              'method': 'Unchanged original x86 without decision hooks; explicit x87 precision/nearest-even modes.',
              'ranges': {name: {'start': hex(start), 'end_exclusive': hex(end),
                                'sha256': sha(original.range(start, end - start)), 'unchanged_v14': True}
                         for name, (start, end) in RANGES.items()},
              'constants': {'reciprocal_u32_max_f64_bits': '0x3df0000000100000', 'unsigned_offset_f64_bits': '0x41f0000000000000'},
              'constructors': constructors, 'bounded': bounded, 'converted': converted, 'drawn': drawn,
              'limits': ['Raw state restoration and native seed mixing are distinct operations.',
                         'Seed source, authoritative call-site ordering and native FPU lifecycle remain unimplemented.',
                         'Float primitive ends at the explicit binary32 diagnostic store, not a whole caller.',
                         'Bound zero is excluded: original divides by zero after drawing; Rust will reject before mutation.']}
    path = HERE / 'native-rng-vectors.json'
    path.write_text(json.dumps(report, indent=2), encoding='utf-8')
    cs = Cs(CS_ARCH_X86, CS_MODE_32)
    lines = []
    for name, (start, end) in RANGES.items():
        if name == 'float_constants':
            continue
        lines.append(name)
        lines.extend(f'{i.address:08x} {i.mnemonic} {i.op_str}' for i in cs.disasm(original.range(start, end - start), start))
    (HERE / 'native-rng-assembly.txt').write_text('\n'.join(lines), encoding='utf-8')
    print(json.dumps({'path': str(path), 'sha256': sha(path.read_bytes()),
                      'counts': {key: len(report[key]) for key in ['constructors', 'bounded', 'converted', 'drawn']}}))


if __name__ == '__main__':
    main()
