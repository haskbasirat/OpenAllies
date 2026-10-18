"""Shared isolated-x86 harness for phase8 economy oracles.

Loads the ORIGINAL AA.exe image into Unicorn (never launches or writes the game),
checks that every executed function range and constant equals the accepted
runtime-expanded-v14 image, and provides explicit seam hooks. A seam is an
intercepted call whose body is NOT executed; every seam is named by the caller.
"""
from pathlib import Path
import hashlib, struct, sys

ROOT = Path(__file__).resolve().parents[2]
GAME = ROOT.parent
sys.path.insert(0, str(GAME / 'reconstruction/tools/python-packages'))
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32, UC_HOOK_CODE, UC_HOOK_MEM_READ, UC_HOOK_MEM_WRITE  # noqa: E402
from unicorn.x86_const import (  # noqa: E402
    UC_X86_REG_EAX, UC_X86_REG_EBX, UC_X86_REG_ECX, UC_X86_REG_EDX, UC_X86_REG_ESI,
    UC_X86_REG_EDI, UC_X86_REG_EBP, UC_X86_REG_ESP, UC_X86_REG_EIP, UC_X86_REG_FPCW,
    UC_X86_REG_FPSW, UC_X86_REG_FP0)

ORIGINAL_SHA = '5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26'
ACCEPTED_SHA = '3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1'
MODES = [('Single24', 0x007f), ('Double53', 0x027f), ('Extended64', 0x037f)]


def sha(data):
    return hashlib.sha256(data).hexdigest()


def bits(value):
    return struct.unpack('<I', struct.pack('<f', value))[0]


def f32(value):
    return struct.unpack('<f', struct.pack('<I', value & 0xffffffff))[0]


class Pe:
    def __init__(self, data):
        self.data = data
        p = struct.unpack_from('<I', data, 0x3c)[0]
        assert data[p:p + 4] == b'PE\0\0'
        count = struct.unpack_from('<H', data, p + 6)[0]
        optional_size = struct.unpack_from('<H', data, p + 20)[0]
        self.base = struct.unpack_from('<I', data, p + 24 + 28)[0]
        self.size = struct.unpack_from('<I', data, p + 24 + 56)[0]
        self.header_size = struct.unpack_from('<I', data, p + 24 + 60)[0]
        self.sections = []
        for i in range(count):
            s = p + 24 + optional_size + i * 40
            virtual_size, rva, size, offset = struct.unpack_from('<4I', data, s + 8)
            self.sections.append((rva, size, offset, virtual_size))

    def at(self, address, size):
        rva = address - self.base
        for va, length, offset, _ in self.sections:
            if va <= rva and rva + size <= va + length:
                return self.data[offset + rva - va:offset + rva - va + size]
        raise ValueError(hex(address))

    def load(self, uc):
        uc.mem_map(self.base, (self.size + 4095) & ~4095)
        uc.mem_write(self.base, self.data[:self.header_size])
        for rva, size, offset, _ in self.sections:
            if size:
                uc.mem_write(self.base + rva, self.data[offset:offset + size])


ORIGINAL = Pe((GAME / 'AA.exe').read_bytes())
ACCEPTED = Pe((GAME / 'reconstruction/native-game/runtime-expanded-v14/AA.exe').read_bytes())
assert sha(ORIGINAL.data) == ORIGINAL_SHA
assert sha(ACCEPTED.data) == ACCEPTED_SHA


def pin(ranges):
    """Require byte identity of original and accepted v14 for each range."""
    out = {}
    for name, (start, end) in ranges.items():
        a, b = ORIGINAL.at(start, end - start), ACCEPTED.at(start, end - start)
        assert a == b, (name, hex(start))
        out[name] = {'start': f'0x{start:06X}', 'end': f'0x{end:06X}', 'sha256': sha(a)}
    return out


PREFIXES = (0x26, 0x2E, 0x36, 0x3E, 0x65, 0x66, 0x67, 0xF0, 0xF2, 0xF3)


def fs_prefixed(uc, pc):
    """True when the instruction at pc carries the FS segment override."""
    for byte in bytes(uc.mem_read(pc, 4)):
        if byte == 0x64:
            return True
        if byte not in PREFIXES:
            return False
    return False


def guard_null_page(machine):
    """FS base is 0 in Unicorn, so fs:[0] (the SEH link) is linear 0..3 and
    page 0 must stay mapped. Only FS-prefixed accesses to bytes 0..3 are
    legitimate; any other page-0 access is a null-pointer dereference: it is
    recorded and stops the run (docs/NATIVE-PORT-RULES.md, emulator caveats)."""
    machine.null_accesses = []

    def hook(uc, access, address, size, value, _):
        pc = uc.reg_read(UC_X86_REG_EIP)
        if address + size > 4 or not fs_prefixed(uc, pc):
            machine.null_accesses.append((hex(pc), address, size))
            uc.emu_stop()
    machine.uc.hook_add(UC_HOOK_MEM_READ | UC_HOOK_MEM_WRITE, hook, begin=0, end=0xFFF)


def check_null_accesses(machine):
    if machine.null_accesses:
        raise AssertionError(f'null-pointer access in native code: {machine.null_accesses}')


class Machine:
    STACK = 0x3008000
    STOP = 0x300f000
    HEAP = 0x3100000

    def __init__(self, control_word):
        self.uc = uc = Uc(UC_ARCH_X86, UC_MODE_32)
        ORIGINAL.load(uc)
        uc.mem_map(0, 0x1000)  # isolated SEH chain head (fs base 0)
        uc.mem_map(0x2000000, 0x400000)
        uc.mem_map(0x3000000, 0x10000)
        uc.mem_map(self.HEAP, 0x100000)
        self.heap = self.HEAP
        self.cw = control_word
        self.seams = {}
        self.events = []
        self.visited = set()
        self.identical = set()
        self.ranges = []
        uc.hook_add(UC_HOOK_CODE, self._code)
        guard_null_page(self)

    # memory helpers
    def put(self, address, value):
        self.uc.mem_write(address, struct.pack('<I', value & 0xffffffff))

    def put8(self, address, value):
        self.uc.mem_write(address, bytes([value & 0xff]))

    def put16(self, address, value):
        self.uc.mem_write(address, struct.pack('<H', value & 0xffff))

    def read(self, address):
        return struct.unpack('<I', self.uc.mem_read(address, 4))[0]

    def read8(self, address):
        return self.uc.mem_read(address, 1)[0]

    def words(self, address, count):
        return list(struct.unpack('<' + 'I' * count, self.uc.mem_read(address, 4 * count))) if count else []

    def put_words(self, address, values):
        if values:
            self.uc.mem_write(address, struct.pack('<' + 'I' * len(values), *[v & 0xffffffff for v in values]))

    def alloc(self, size):
        result = self.heap
        self.heap += max(16, (size + 15) & ~15)
        assert self.heap < self.HEAP + 0x100000
        self.uc.mem_write(result, b'\0' * size)
        return result

    # seams
    def seam(self, address, handler):
        """handler(machine, esp) -> (eax or None, bytes_to_pop_after_return)."""
        self.seams[address] = handler

    def arg(self, index):
        esp = self.uc.reg_read(UC_X86_REG_ESP)
        return self.read(esp + 4 + 4 * index)

    def _code(self, uc, address, size, _data):
        if ORIGINAL.base <= address < ORIGINAL.base + ORIGINAL.size:
            if address not in self.identical:
                assert ORIGINAL.at(address, size) == ACCEPTED.at(address, size), hex(address)
                self.identical.add(address)
        if self.ranges:
            self.visited.add(address)
        handler = self.seams.get(address)
        if handler is None:
            return
        esp = uc.reg_read(UC_X86_REG_ESP)
        eax, pop = handler(self, esp)
        if eax is not None:
            uc.reg_write(UC_X86_REG_EAX, eax & 0xffffffff)
        uc.reg_write(UC_X86_REG_ESP, esp + 4 + pop)
        uc.reg_write(UC_X86_REG_EIP, self.read(esp))

    def call(self, address, args, this, count=2_000_000):
        uc = self.uc
        self.put(self.STACK, self.STOP)
        for i, value in enumerate(args):
            self.put(self.STACK + 4 + i * 4, value)
        uc.reg_write(UC_X86_REG_ESP, self.STACK)
        uc.reg_write(UC_X86_REG_ECX, this)
        uc.reg_write(UC_X86_REG_FPCW, self.cw)
        preserved = {reg: 0x77770000 + i for i, reg in enumerate(
            [UC_X86_REG_EBX, UC_X86_REG_ESI, UC_X86_REG_EDI, UC_X86_REG_EBP])}
        for reg, value in preserved.items():
            uc.reg_write(reg, value)
        uc.emu_start(address, self.STOP, count=count)
        check_null_accesses(self)
        assert uc.reg_read(UC_X86_REG_EIP) == self.STOP
        for reg, value in preserved.items():
            assert uc.reg_read(reg) == value, 'callee-saved register changed'
        return uc.reg_read(UC_X86_REG_EAX), uc.reg_read(UC_X86_REG_ESP)


def ret_pop(count):
    """Seam returning nothing and popping `count` stdcall/thiscall args."""
    def handler(machine, esp):
        return None, 4 * count
    return handler
