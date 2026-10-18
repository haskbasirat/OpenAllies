"""Unicorn harness for the phase-8 zone/detection/fog oracles (original AA.exe only).

Pattern from research/phase8-ai/native_machine.py (copied, not imported, so a
change there cannot silently alter these fixtures). The ORIGINAL image is
mapped unchanged at 0x400000; test objects live in a separate heap; seams are
Python callbacks at original call targets or at synthetic stub addresses.
Every executed original instruction is recorded and checked byte-identical in
the accepted v14 image. Page 0 is mapped only for the MSVC SEH word fs:[0]
(FS base 0): any page-0 access without the FS prefix is a null dereference and
stops the run (recorded as a native fault). Operator new (0x6F7810) and delete
(0x6F7430) are always seamed; new allocations are filled with 0xCD so a native
read of an uninitialised field cannot pass unnoticed.
"""
from pathlib import Path
import hashlib, struct, sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
GAME = ROOT.parent
sys.path.insert(0, str(GAME / 'reconstruction/tools/python-packages'))
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32, UC_HOOK_CODE, UC_HOOK_MEM_READ, UC_HOOK_MEM_WRITE, UcError
from unicorn.x86_const import (UC_X86_REG_EAX, UC_X86_REG_EBX, UC_X86_REG_ECX, UC_X86_REG_EDX,
                               UC_X86_REG_ESI, UC_X86_REG_EDI, UC_X86_REG_EBP, UC_X86_REG_ESP,
                               UC_X86_REG_EIP, UC_X86_REG_FPCW, UC_X86_REG_FPSW)

sha = lambda b: hashlib.sha256(b).hexdigest()
PRECISIONS = {'Single24': 0x007f, 'Double53': 0x027f, 'Extended64': 0x037f}
f32 = lambda f: struct.unpack('<I', struct.pack('<f', f))[0]
from_f32 = lambda bits: struct.unpack('<f', struct.pack('<I', bits & 0xffffffff))[0]


class PE:
    def __init__(self, path):
        self.path, self.b = path, path.read_bytes()
        pe = struct.unpack_from('<I', self.b, 0x3c)[0]
        n = struct.unpack_from('<H', self.b, pe + 6)[0]
        op = pe + 24
        self.base = struct.unpack_from('<I', self.b, op + 28)[0]
        self.image_size = struct.unpack_from('<I', self.b, op + 56)[0]
        sec = op + struct.unpack_from('<H', self.b, pe + 20)[0]
        self.sections = [struct.unpack_from('<IIII', self.b, sec + i * 40 + 8) for i in range(n)]

    def range(self, va, size):
        rva = va - self.base
        for vs, r, raw, off in self.sections:
            if r <= rva < r + raw:
                return self.b[off + rva - r:off + rva - r + size]
        raise ValueError(hex(va))


original = PE(GAME / 'AA.exe')
modified = PE(GAME / 'reconstruction/native-game/runtime-expanded-v14/AA.exe')
assert sha(original.b) == '5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26'
assert sha(modified.b) == '3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1'

HEAP, HEAP_SIZE = 0x20000000, 0x04000000
STUBS, STUB_SIZE = 0x1f000000, 0x00100000
STACK_BASE, STACK_SIZE = 0x1e000000, 0x00040000
STACK = STACK_BASE + 0x30000
STOP = STACK_BASE + 0x3f000
OUT = STACK_BASE + 0x3e000
CALLEE_SAVED = [UC_X86_REG_EBX, UC_X86_REG_ESI, UC_X86_REG_EDI, UC_X86_REG_EBP]
GWORLD_GLOBAL = 0xB36534


class NativeFault(Exception):
    """The original code dereferenced null / ran off a seam (a native crash)."""


class Machine:
    def __init__(self, precision='Double53'):
        self.precision = precision
        u = self.u = Uc(UC_ARCH_X86, UC_MODE_32)
        u.mem_map(original.base, (original.image_size + 0xfff) & ~0xfff)
        for vs, rva, raw, off in original.sections:
            u.mem_write(original.base + rva, original.b[off:off + raw])
        u.mem_map(HEAP, HEAP_SIZE)
        u.mem_map(STUBS, STUB_SIZE)
        u.mem_map(STACK_BASE, STACK_SIZE)
        self.heap = HEAP
        self.next_stub = STUBS
        self.seams = {}
        self.observers = {}
        self.visited = {}
        self.events = []
        self.freed = []
        u.hook_add(UC_HOOK_CODE, self._hook)
        u.mem_map(0, 0x1000)
        self.null_accesses = []
        u.hook_add(UC_HOOK_MEM_READ | UC_HOOK_MEM_WRITE, self._null, begin=0, end=0xfff)
        self.seam(0x6f7810, lambda m: m.ret(m.alloc(max(m.arg(1), 4), fill=0xcd)))
        self.seam(0x6f7430, self._delete)

    def _delete(self, m):
        self.freed.append(m.arg(1))
        m.ret(0)

    # memory -------------------------------------------------------------
    def alloc(self, size, align=16, fill=0):
        self.heap = (self.heap + align - 1) & ~(align - 1)
        p = self.heap
        self.heap += max(size, 4)
        assert self.heap < HEAP + HEAP_SIZE
        self.u.mem_write(p, bytes([fill]) * max(size, 4))
        return p

    def w(self, a, v):
        self.u.mem_write(a, struct.pack('<I', v & 0xffffffff))

    def r(self, a):
        return struct.unpack('<I', self.u.mem_read(a, 4))[0]

    def w16(self, a, v):
        self.u.mem_write(a, struct.pack('<H', v & 0xffff))

    def r16(self, a):
        return struct.unpack('<H', self.u.mem_read(a, 2))[0]

    def wb(self, a, v):
        self.u.mem_write(a, bytes([v & 0xff]))

    def rb(self, a):
        return self.u.mem_read(a, 1)[0]

    def array(self, words):
        p = self.alloc(4 * max(len(words), 1))
        for i, v in enumerate(words):
            self.w(p + 4 * i, v)
        return p

    # seams --------------------------------------------------------------
    def seam(self, address, fn):
        self.seams[address] = fn

    def observe(self, address, fn):
        self.observers[address] = fn

    def stub(self, fn):
        a = self.next_stub
        self.next_stub += 16
        self.u.mem_write(a, b'\xcc' * 16)
        self.seams[a] = fn
        return a

    def vtable(self, slots, size=0x200):
        """A synthetic vtable; slots maps byte offset -> python seam."""
        vt = self.alloc(size)
        trap = self.stub(self._unexpected_slot)
        for off in range(0, size, 4):
            self.w(vt + off, trap)
        for off, fn in slots.items():
            self.w(vt + off, self.stub(fn))
        return vt

    def _unexpected_slot(self, m):
        raise AssertionError('unexpected synthetic virtual call from %08x' % self.r(self.u.reg_read(UC_X86_REG_ESP)))

    def arg(self, i):
        """i-th stack argument (1-based) at a seam entry (return address at [esp])."""
        return self.r(self.u.reg_read(UC_X86_REG_ESP) + 4 * i)

    def ecx(self):
        return self.u.reg_read(UC_X86_REG_ECX)

    def ret(self, value=0, pop=0):
        s = self.u.reg_read(UC_X86_REG_ESP)
        self.u.reg_write(UC_X86_REG_EAX, value & 0xffffffff)
        self.u.reg_write(UC_X86_REG_EIP, self.r(s))
        self.u.reg_write(UC_X86_REG_ESP, s + 4 + pop)

    def ret_st0_f32(self, bits, pop=0):
        scratch = self.alloc(16)
        self.w(scratch, bits)
        code = b'\xd9\x05' + struct.pack('<I', scratch) + b'\xc2' + struct.pack('<H', pop)
        stub = self.alloc(16)
        self.u.mem_write(stub, code)
        self.u.reg_write(UC_X86_REG_EIP, stub)

    def _null(self, u, access, address, size, value, _):
        pc = u.reg_read(UC_X86_REG_EIP)
        if address + size > 4 or not self._fs_prefixed(pc):
            self.null_accesses.append((pc, address, size))
            u.emu_stop()

    def _fs_prefixed(self, pc):
        for byte in bytes(self.u.mem_read(pc, 4)):
            if byte == 0x64:
                return True
            if byte not in (0x26, 0x2e, 0x36, 0x3e, 0x65, 0x66, 0x67, 0xf0, 0xf2, 0xf3):
                return False
        return False

    def _hook(self, u, pc, size, _):
        fn = self.seams.get(pc)
        if fn is not None:
            fn(self)
            return
        if original.base <= pc < original.base + original.image_size:
            self.visited[pc] = bytes(u.mem_read(pc, size))
        obs = self.observers.get(pc)
        if obs is not None:
            obs(self)

    # calls --------------------------------------------------------------
    def call(self, entry, ecx=0, args=(), pop=None, count=5_000_000, fpsw=0):
        """Run entry; raise NativeFault on a null dereference or a stray stop."""
        u = self.u
        u.reg_write(UC_X86_REG_FPCW, PRECISIONS[self.precision])
        u.reg_write(UC_X86_REG_FPSW, fpsw)
        sp = STACK - 4 * len(args)
        for i, v in enumerate(args):
            self.w(sp + 4 * i, v)
        sp -= 4
        self.w(sp, STOP)
        u.reg_write(UC_X86_REG_ESP, sp)
        u.reg_write(UC_X86_REG_ECX, ecx)
        for i, r in enumerate(CALLEE_SAVED):
            u.reg_write(r, 0x11111111 * (i + 1))
        try:
            u.emu_start(entry, STOP, count=count)
        except UcError as e:
            raise NativeFault('unicorn %s at %08x' % (e, u.reg_read(UC_X86_REG_EIP)))
        if self.null_accesses:
            pc, addr, size = self.null_accesses[0]
            self.null_accesses.clear()
            raise NativeFault('null access %08x at pc %08x' % (addr, pc))
        if u.reg_read(UC_X86_REG_EIP) != STOP:
            raise NativeFault('stopped at %08x' % u.reg_read(UC_X86_REG_EIP))
        expected = sp + 4 + (4 * len(args) if pop is None else pop)
        assert u.reg_read(UC_X86_REG_ESP) == expected, (hex(u.reg_read(UC_X86_REG_ESP)), hex(expected))
        assert [u.reg_read(r) for r in CALLEE_SAVED] == [0x11111111 * (i + 1) for i in range(4)]
        return u.reg_read(UC_X86_REG_EAX)

    def verify_v14(self):
        for a, b in self.visited.items():
            assert b == original.range(a, len(b)) == modified.range(a, len(b)), hex(a)


def instructions(visited):
    return [{'va': hex(a), 'bytes': b.hex()} for a, b in sorted(visited.items())]


def visited_in(visited, start, end):
    return sorted(a for a in visited if start <= a < end)


def header(producer, method):
    return {'schema': 1, 'method': method, 'original_sha256': sha(original.b), 'v14_sha256': sha(modified.b),
            'producer_sha256': sha(Path(producer).read_bytes())}
