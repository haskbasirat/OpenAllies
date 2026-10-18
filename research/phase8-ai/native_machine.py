"""Shared Unicorn harness for phase8 AI oracles (original AA.exe code only).

The ORIGINAL image is mapped unchanged at 0x400000. Test objects live in a
separate heap; explicit seams are Python callbacks at either original call
targets or synthetic virtual-table stubs. Every executed original instruction
is recorded so producers can prove the bytes are identical in the accepted v14
image. Nothing here writes game files.
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

HEAP, HEAP_SIZE = 0x20000000, 0x01000000
STUBS, STUB_SIZE = 0x1f000000, 0x00010000
STACK_BASE, STACK_SIZE = 0x1e000000, 0x00010000
STACK = STACK_BASE + 0x8000
STOP = STACK_BASE + 0xf000
OUT = STACK_BASE + 0xe000
CALLEE_SAVED = [UC_X86_REG_EBX, UC_X86_REG_ESI, UC_X86_REG_EDI, UC_X86_REG_EBP]


class Machine:
    def __init__(self, precision):
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
        u.hook_add(UC_HOOK_CODE, self._hook)
        # FS base is 0 in this emulator, so the MSVC SEH frame link fs:[0] is
        # linear 0..3. Page 0 is mapped only for that word, and only accesses
        # through an FS-prefixed instruction may touch it; any other access
        # there is a null-pointer dereference and fails the run.
        u.mem_map(0, 0x1000)
        self.null_accesses = []
        u.hook_add(UC_HOOK_MEM_READ | UC_HOOK_MEM_WRITE, self._null, begin=0, end=0xfff)

    # memory -------------------------------------------------------------
    def alloc(self, size, align=16):
        self.heap = (self.heap + align - 1) & ~(align - 1)
        p = self.heap
        self.heap += max(size, 4)
        assert self.heap < HEAP + HEAP_SIZE
        return p

    def w(self, a, v):
        self.u.mem_write(a, struct.pack('<I', v & 0xffffffff))

    def r(self, a):
        return struct.unpack('<I', self.u.mem_read(a, 4))[0]

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
        """Call fn before the original instruction at address executes."""
        self.observers[address] = fn

    def stub(self, fn):
        a = self.next_stub
        self.next_stub += 16
        self.u.mem_write(a, b'\xcc' * 16)
        self.seams[a] = fn
        return a

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
        """Return through ST0 with an f32 value loaded by a tiny scratch stub."""
        scratch = self.alloc(16)
        self.w(scratch, bits)
        code = b'\xd9\x05' + struct.pack('<I', scratch) + b'\xc2' + struct.pack('<H', pop)
        stub = self.alloc(16)
        self.u.mem_write(stub, code)
        self.u.reg_write(UC_X86_REG_EIP, stub)

    def ret_st0_f64(self, bits, pop=0):
        """Return through ST0 with a binary64 value (fld qword) from a scratch stub."""
        scratch = self.alloc(16)
        self.u.mem_write(scratch, struct.pack('<Q', bits))
        code = b'\xdd\x05' + struct.pack('<I', scratch) + b'\xc2' + struct.pack('<H', pop)
        stub = self.alloc(16)
        self.u.mem_write(stub, code)
        self.u.reg_write(UC_X86_REG_EIP, stub)

    def _null(self, u, access, address, size, value, _):
        pc = u.reg_read(UC_X86_REG_EIP)
        if address + size > 4 or not self._fs_prefixed(pc):
            self.null_accesses.append((pc, address, size))
            u.emu_stop()

    def _fs_prefixed(self, pc):
        """True when the instruction at pc carries the FS segment override."""
        for byte in bytes(self.u.mem_read(pc, 4)):
            if byte == 0x64:
                return True
            if byte not in (0x26, 0x2e, 0x36, 0x3e, 0x65, 0x66, 0x67, 0xf0, 0xf2, 0xf3):
                return False
        return False

    def _hook(self, u, pc, size, _):
        fn = self.seams.get(pc)
        if fn is not None:
            # Seam entries are replaced, not executed: never recorded as visited.
            fn(self)
            return
        if original.base <= pc < original.base + original.image_size:
            self.visited[pc] = bytes(u.mem_read(pc, size))
        obs = self.observers.get(pc)
        if obs is not None:
            obs(self)

    # calls --------------------------------------------------------------
    def call(self, entry, ecx=0, args=(), pop=None, count=2_000_000, fpsw=0):
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
        u.emu_start(entry, STOP, count=count)
        assert not self.null_accesses, [tuple(map(hex, a)) for a in self.null_accesses]
        assert u.reg_read(UC_X86_REG_EIP) == STOP, hex(u.reg_read(UC_X86_REG_EIP))
        expected = sp + 4 + (4 * len(args) if pop is None else pop)
        assert u.reg_read(UC_X86_REG_ESP) == expected, (hex(u.reg_read(UC_X86_REG_ESP)), hex(expected))
        assert [u.reg_read(r) for r in CALLEE_SAVED] == [0x11111111 * (i + 1) for i in range(4)]
        return u.reg_read(UC_X86_REG_EAX)

    def st0_live(self):
        """Read the live ST0 (sig, sign/exponent) inside a hook."""
        from unicorn import x86_const as X
        top = (self.u.reg_read(UC_X86_REG_FPSW) >> 11) & 7
        sig, se = self.u.reg_read(getattr(X, 'UC_X86_REG_FP%d' % top))
        return {'extended_significand': sig, 'extended_sign_exponent': se}

    def st0(self):
        """Store ST0 as f32 and as raw 80-bit (fstp) without changing PC."""
        code = b'\xd9\x15' + struct.pack('<I', OUT) + b'\xdb\x3d' + struct.pack('<I', OUT + 8)
        self.u.mem_write(STOP, code)
        self.u.emu_start(STOP, STOP + len(code))
        sig, exp = struct.unpack('<QH', self.u.mem_read(OUT + 8, 10))
        return {'f32_bits': self.r(OUT), 'extended_significand': sig, 'extended_sign_exponent': exp}

    def verify_v14(self):
        for a, b in self.visited.items():
            assert b == original.range(a, len(b)) == modified.range(a, len(b)), hex(a)


def instructions(visited):
    return [{'va': hex(a), 'bytes': b.hex()} for a, b in sorted(visited.items())]


def header(producer, method):
    return {'schema': 1, 'method': method, 'original_sha256': sha(original.b), 'v14_sha256': sha(modified.b),
            'producer_sha256': sha(Path(producer).read_bytes())}
