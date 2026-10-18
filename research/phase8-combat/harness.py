"""Shared Unicorn harness for phase8 combat oracles.

Executes UNCHANGED original AA.exe instructions in an isolated 32-bit Unicorn
address space. No game process, file, save or profile is touched. Every executed
instruction inside the image is pinned and compared against the accepted v14
executable. Seams (hooks) are explicit: a hooked address never executes; its
behaviour is the fixture's recorded leaf, not a claim about the original body.
"""
from pathlib import Path
import hashlib
import struct
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
GAME = ROOT.parent
sys.path.insert(0, str(GAME / 'reconstruction/tools/python-packages'))
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32, UC_HOOK_CODE  # noqa: E402
from unicorn.x86_const import (  # noqa: E402
    UC_X86_REG_EAX, UC_X86_REG_EBP, UC_X86_REG_EBX, UC_X86_REG_ECX, UC_X86_REG_EDI,
    UC_X86_REG_EDX, UC_X86_REG_EIP, UC_X86_REG_ESI, UC_X86_REG_ESP, UC_X86_REG_FPCW)

ORIGINAL_SHA = '5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26'
ACCEPTED_SHA = '3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1'
CONTROL = {'Single24': 0x7f, 'Double53': 0x27f, 'Extended64': 0x37f}
PRECISIONS = list(CONTROL)


class Image:
    def __init__(self, path, digest):
        self.data = path.read_bytes()
        assert hashlib.sha256(self.data).hexdigest() == digest, path
        pe = struct.unpack_from('<I', self.data, 0x3c)[0]
        count = struct.unpack_from('<H', self.data, pe + 6)[0]
        opt = pe + 24
        self.base = struct.unpack_from('<I', self.data, opt + 28)[0]
        self.size = struct.unpack_from('<I', self.data, opt + 56)[0]
        self.headers = struct.unpack_from('<I', self.data, opt + 60)[0]
        table = opt + struct.unpack_from('<H', self.data, pe + 20)[0]
        self.sections = [struct.unpack_from('<IIII', self.data, table + i * 40 + 8) for i in range(count)]

    def at(self, va, size):
        rva = va - self.base
        for _, start, length, offset in self.sections:
            if start <= rva and rva + size <= start + length:
                return self.data[offset + rva - start:offset + rva - start + size]
        raise ValueError(hex(va))

    def load(self, uc):
        uc.mem_map(self.base, (self.size + 0xfff) & ~0xfff)
        uc.mem_write(self.base, self.data[:self.headers])
        for _, rva, length, offset in self.sections:
            if length:
                uc.mem_write(self.base + rva, self.data[offset:offset + length])


ORIGINAL = Image(GAME / 'AA.exe', ORIGINAL_SHA)
ACCEPTED = Image(GAME / 'reconstruction/native-game/runtime-expanded-v14/AA.exe', ACCEPTED_SHA)


def f32(value):
    return struct.unpack('<I', struct.pack('<f', value))[0]


def f64(value):
    return struct.unpack('<Q', struct.pack('<d', value))[0]


def from_f32(bits):
    return struct.unpack('<f', struct.pack('<I', bits & 0xffffffff))[0]


def ext_from_f64_bits(bits):
    """Exact binary64 -> x87 80-bit encoding (finite normal or zero inputs)."""
    sign = bits >> 63
    exponent = (bits >> 52) & 0x7ff
    fraction = bits & ((1 << 52) - 1)
    if exponent == 0 and fraction == 0:
        return bytes(8) + struct.pack('<H', sign << 15)
    assert 0 < exponent < 0x7ff, 'only finite normal binary64 values are supported'
    mantissa = (1 << 63) | (fraction << 11)
    return struct.pack('<QH', mantissa, (sign << 15) | (exponent - 1023 + 16383))


class Machine:
    HEAP = 0x2000000
    HEAP_SIZE = 0x400000
    STACK = 0x3008000
    STOP = 0x300f000
    STUBS = 0x3100000
    VIRTUALS = 0x3200000
    SCRATCH = 0x3100f00

    def __init__(self, precision):
        self.precision = precision
        self.u = Uc(UC_ARCH_X86, UC_MODE_32)
        ORIGINAL.load(self.u)
        self.u.mem_map(self.HEAP, self.HEAP_SIZE)
        self.u.mem_map(0x3000000, 0x10000)
        self.u.mem_map(self.STUBS, 0x1000)
        self.u.mem_map(self.VIRTUALS, 0x10000)
        self.u.mem_write(self.VIRTUALS, b'\xcc' * 0x10000)
        self.u.reg_write(UC_X86_REG_FPCW, CONTROL[precision])
        self.cursor = self.HEAP
        self.hooks = {}
        self.visited = {}
        self.events = []
        self.next_virtual = self.VIRTUALS
        self.stub_cursor = self.STUBS
        self.stubs = {}
        self.u.hook_add(UC_HOOK_CODE, self._code)

    # memory -----------------------------------------------------------------
    def alloc(self, size, align=16):
        self.cursor = (self.cursor + align - 1) & ~(align - 1)
        ptr = self.cursor
        self.cursor += size
        assert self.cursor < self.HEAP + self.HEAP_SIZE
        return ptr

    def w(self, address, value):
        self.u.mem_write(address, struct.pack('<I', value & 0xffffffff))

    def r(self, address):
        return struct.unpack('<I', self.u.mem_read(address, 4))[0]

    def wb(self, address, value):
        self.u.mem_write(address, bytes([value & 0xff]))

    def rb(self, address):
        return self.u.mem_read(address, 1)[0]

    def arg(self, index=0):
        return self.r(self.u.reg_read(UC_X86_REG_ESP) + 4 + 4 * index)

    def reg(self, register):
        return self.u.reg_read(register)

    # calls and returns ------------------------------------------------------
    def virtual(self, handler):
        """Return an address that, when called, runs handler(machine)."""
        address = self.next_virtual
        self.next_virtual += 0x10
        self.hooks[address] = handler
        return address

    def vtable(self, slots):
        """slots: {byte offset: handler}. Unlisted slots trap."""
        size = max(slots) + 4 if slots else 4
        table = self.alloc(size)
        for offset, handler in slots.items():
            self.w(table + offset, self.virtual(handler))
        return table

    def ret(self, pop=0, value=0):
        sp = self.reg(UC_X86_REG_ESP)
        self.u.reg_write(UC_X86_REG_EIP, self.r(sp))
        self.u.reg_write(UC_X86_REG_ESP, sp + 4 + pop)
        self.u.reg_write(UC_X86_REG_EAX, value & 0xffffffff)

    def _stub(self, kind, pop):
        key = (kind, pop)
        if key not in self.stubs:
            code = (b'\xd9\x05' if kind == 'f32' else b'\xdb\x2d') + struct.pack('<I', self.SCRATCH)
            code += b'\xc2' + struct.pack('<H', pop) if pop else b'\xc3'
            self.u.mem_write(self.stub_cursor, code)
            self.stubs[key] = self.stub_cursor
            self.stub_cursor += 16
        return self.stubs[key]

    def ret_f32(self, pop, bits):
        """Return a binary32 value in ST0 through an fld dword stub."""
        self.w(self.SCRATCH, bits)
        self.u.reg_write(UC_X86_REG_EIP, self._stub('f32', pop))

    def ret_ext(self, pop, f64_bits):
        """Return an exact binary64 value in ST0 through an fld tbyte stub."""
        self.u.mem_write(self.SCRATCH, ext_from_f64_bits(f64_bits))
        self.u.reg_write(UC_X86_REG_EIP, self._stub('ext', pop))

    def _code(self, uc, pc, size, _):
        if ORIGINAL.base <= pc < ORIGINAL.base + ORIGINAL.size:
            raw = ORIGINAL.at(pc, size)
            assert raw == ACCEPTED.at(pc, size), hex(pc)
            self.visited[pc] = raw.hex()
        handler = self.hooks.get(pc)
        if handler is not None:
            handler(self)
        elif self.VIRTUALS <= pc < self.VIRTUALS + 0x10000:
            raise RuntimeError(f'unhooked virtual slot {pc:#x}')

    def call(self, address, ecx=0, args=(), pop=None, limit=200000):
        sp = self.STACK - 0x40
        self.w(sp, self.STOP)
        for i, value in enumerate(args):
            self.w(sp + 4 + 4 * i, value)
        self.u.reg_write(UC_X86_REG_ESP, sp)
        self.u.reg_write(UC_X86_REG_ECX, ecx)
        saved = {UC_X86_REG_EBX: 0x5a5a0001, UC_X86_REG_ESI: 0x5a5a0002,
                 UC_X86_REG_EDI: 0x5a5a0003, UC_X86_REG_EBP: 0x5a5a0004}
        for register, value in saved.items():
            self.u.reg_write(register, value)
        self.u.reg_write(UC_X86_REG_EDX, 0x5a5a0005)
        try:
            self.u.emu_start(address, self.STOP, count=limit)
        except Exception as exc:
            raise RuntimeError(f'fault at {self.reg(UC_X86_REG_EIP):#x}') from exc
        assert self.reg(UC_X86_REG_EIP) == self.STOP, hex(self.reg(UC_X86_REG_EIP))
        expected = sp + 4 + (4 * len(args) if pop is None else pop)
        assert self.reg(UC_X86_REG_ESP) == expected, (hex(self.reg(UC_X86_REG_ESP)), hex(expected))
        for register, value in saved.items():
            assert self.reg(register) == value, (hex(address), register)
        return self.reg(UC_X86_REG_EAX)

    def st0_f32(self):
        """Store ST0 (the native float10 return) to binary32 via fstp and read it."""
        code = b'\xd9\x1d' + struct.pack('<I', self.SCRATCH) + b'\xc3'
        self.u.mem_write(self.STUBS + 0xe00, code)
        sp = self.STACK - 0x80
        self.w(sp, self.STOP)
        self.u.reg_write(UC_X86_REG_ESP, sp)
        self.u.emu_start(self.STUBS + 0xe00, self.STOP, count=10)
        return self.r(self.SCRATCH)

    def st0_ext(self):
        """Store ST0 as 80-bit and return (mantissa, sign_exponent)."""
        code = b'\xdb\x3d' + struct.pack('<I', self.SCRATCH) + b'\xc3'
        self.u.mem_write(self.STUBS + 0xe40, code)
        sp = self.STACK - 0x80
        self.w(sp, self.STOP)
        self.u.reg_write(UC_X86_REG_ESP, sp)
        self.u.emu_start(self.STUBS + 0xe40, self.STOP, count=10)
        mantissa, top = struct.unpack('<QH', self.u.mem_read(self.SCRATCH, 10))
        return {'mantissa': mantissa, 'sign_exponent': top}


# Native global roots used by the combat routines.
SETTINGS_ROOT = 0xb39ab0
WORLD_ROOT = 0xb36534
REGISTRY_ROOT = 0xb364c0


def install_settings(machine, values):
    """values: {byte offset in the settings pointer table: f32 bits}."""
    table = machine.alloc(0x400)
    machine.w(SETTINGS_ROOT, table)
    storage = machine.alloc(0x400)
    for offset, bits in values.items():
        machine.w(storage + offset, bits)
        machine.w(table + offset, storage + offset)
    return table, storage


def install_world(machine, mode=0, time=0):
    world = machine.alloc(0x200)
    machine.w(WORLD_ROOT, world)
    machine.w(world + 0xd8, mode)
    machine.w(world + 0xe0, time)
    return world


def visited_windows(rows):
    spans = []
    for address, hexbytes in sorted(rows.items()):
        size = len(hexbytes) // 2
        if spans and spans[-1][1] >= address:
            spans[-1][1] = max(spans[-1][1], address + size)
        else:
            spans.append([address, address + size])
    out = []
    for start, end in spans:
        raw = ORIGINAL.at(start, end - start)
        assert raw == ACCEPTED.at(start, end - start)
        out.append({'start': hex(start), 'end': hex(end), 'sha256': hashlib.sha256(raw).hexdigest()})
    return out


def report(name, doc, cases, visited, extra=None):
    body = {
        'schema': 1,
        'passed': True,
        'scope': doc,
        'original_sha256': ORIGINAL_SHA,
        'accepted_sha256': ACCEPTED_SHA,
        'producer_sha256': hashlib.sha256(Path(sys.argv[0]).resolve().read_bytes()).hexdigest(),
        'harness_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        'cases': cases,
        'native_windows': visited_windows(visited),
        'window_scope': 'Visited original instruction bytes (equal in v14). Hooked seam entries are '
                        'listed but their bodies did not execute.',
    }
    if extra:
        body.update(extra)
    import json
    path = HERE / name
    path.write_text(json.dumps(body, indent=1) + '\n')
    print(json.dumps({'file': name, 'cases': len(cases),
                      'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}))
