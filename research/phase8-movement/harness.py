"""Shared Unicorn harness for phase8 movement oracles.

Executes UNCHANGED original AA.exe instructions. Every executed in-image
instruction is compared with the accepted v14 executable and recorded. Only
explicitly registered seams (allocation, world callbacks, virtual services)
are intercepted; a seam entry address appears in the visited list but its body
does not execute. No game process is started; no original file is modified.
"""
from pathlib import Path
import hashlib, json, struct, sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
GAME = ROOT.parent
sys.path.insert(0, str(GAME / 'reconstruction/tools/python-packages'))
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32, UC_HOOK_CODE, UC_HOOK_MEM_READ, UC_HOOK_MEM_WRITE  # noqa: E402
from unicorn.x86_const import *  # noqa: E402,F401,F403

ORIGINAL_SHA = '5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26'
V14_SHA = '3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1'
PRECISION = {'Single24': 0x007f, 'Double53': 0x027f, 'Extended64': 0x037f}


class Image:
    def __init__(self, path, expected):
        self.b = Path(path).read_bytes()
        assert hashlib.sha256(self.b).hexdigest() == expected, path
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
            if r <= rva < r + max(vs, raw):
                o = off + rva - r
                return self.b[o:o + size]
        raise ValueError(hex(va))

    def u32(self, va):
        return struct.unpack('<I', self.range(va, 4))[0]


P = Image(GAME / 'AA.exe', ORIGINAL_SHA)
V = Image(GAME / 'reconstruction/native-game/runtime-expanded-v14/AA.exe', V14_SHA)
VISITED = {}

HEAP, HEAP_END = 0x2000000, 0x2d00000
STUBS = 0x2e00000
STACK, STOP = 0x2f08000, 0x2f0fff0
SCRATCH = 0x2e80000


def f2b(x):
    return struct.unpack('<I', struct.pack('<f', x))[0]


def b2f(x):
    return struct.unpack('<f', struct.pack('<I', x & 0xffffffff))[0]


def d2b(x):
    return struct.unpack('<Q', struct.pack('<d', x))[0]


class NullDereference(Exception):
    pass


class UnseamedAllocator(Exception):
    """Native code reached the engine/CRT heap without a seam (docs/NATIVE-PORT-RULES.md)."""


# Engine operator new/delete 0x6F7810/0x6F7430, their pooled and plain bodies
# 0x6F7700/0x6F7650, CRT _malloc/__nh_malloc/__heap_alloc 0x96E19F/0x96E173/
# 0x96E0F8 and _free 0x96D331. The emulated heap globals are not initialized, so
# reaching any of these without a seam would fault inside the allocator and
# masquerade as a native fault of the function under test.
ALLOCATORS = (0x6f7810, 0x6f7430, 0x6f7700, 0x6f7650, 0x96e19f, 0x96e173, 0x96e0f8, 0x96d331)


def fs_prefixed(u, pc):
    """True when the instruction at pc carries the FS segment override."""
    for byte in bytes(u.mem_read(pc, 4)):
        if byte == 0x64:
            return True
        if byte not in (0x26, 0x2e, 0x36, 0x3e, 0x65, 0x66, 0x67, 0xf0, 0xf2, 0xf3):
            return False
    return False


def page_zero_access(u, address, size):
    """Unicorn's FS base is 0, so the MSVC SEH link fs:[0] is linear 0..3. Only an
    FS-prefixed access to those bytes is legitimate; anything else on page 0 is
    a native null-pointer dereference."""
    if address + size <= 4 and fs_prefixed(u, u.reg_read(UC_X86_REG_EIP)):
        return
    raise NullDereference(hex(address))


class Machine:
    def __init__(self, precision):
        self.precision = precision
        u = self.u = Uc(UC_ARCH_X86, UC_MODE_32)
        u.mem_map(0, 0x1000)
        u.mem_map(P.base, (P.image_size + 0xfff) & ~0xfff)
        u.mem_write(P.base, P.b[:0x1000])
        for vs, r, raw, off in P.sections:
            if raw:
                u.mem_write(P.base + r, P.b[off:off + min(raw, max(vs, raw))])
        u.mem_map(HEAP, 0x1000000)
        self.next = HEAP + 0x1000
        u.reg_write(UC_X86_REG_FPCW, PRECISION[precision])
        self.seams = {}
        self.events = []
        self.stub_next = STUBS
        u.hook_add(UC_HOOK_CODE, self._hook)
        # Page 0 is mapped only for the SEH prologue's fs:[0] (FS base is 0 in
        # Unicorn); any other access there, including a plain read of [0..3],
        # is a native null dereference.
        u.hook_add(UC_HOOK_MEM_READ | UC_HOOK_MEM_WRITE, self._null, begin=0, end=0xfff)

    # ---- memory helpers -------------------------------------------------
    def alloc(self, n, align=16):
        p = (self.next + align - 1) & ~(align - 1)
        self.next = p + max(n, 4)
        assert self.next < HEAP_END
        self.u.mem_write(p, b'\0' * max(n, 4))
        return p

    def w32(self, a, *v):
        self.u.mem_write(a, struct.pack('<' + 'I' * len(v), *[x & 0xffffffff for x in v]))

    def r32(self, a, n=None):
        if n is None:
            return struct.unpack('<I', self.u.mem_read(a, 4))[0]
        return list(struct.unpack('<' + 'I' * n, self.u.mem_read(a, 4 * n)))

    def w8(self, a, v):
        self.u.mem_write(a, bytes([v & 255]))

    def r8(self, a):
        return self.u.mem_read(a, 1)[0]

    def esp(self):
        return self.u.reg_read(UC_X86_REG_ESP)

    def arg(self, i=0):
        return self.r32(self.esp() + 4 + 4 * i)

    def ecx(self):
        return self.u.reg_read(UC_X86_REG_ECX)

    def ret(self, pop=0, value=None):
        sp = self.esp()
        self.u.reg_write(UC_X86_REG_EIP, self.r32(sp))
        self.u.reg_write(UC_X86_REG_ESP, sp + 4 + pop)
        if value is not None:
            self.u.reg_write(UC_X86_REG_EAX, value & 0xffffffff)

    def ret_float(self, bits, kind, pop=0):
        """Return through st(0): kind f32/f64 loads the exact pattern."""
        if kind == 'f32':
            self.w32(SCRATCH, bits)
            code = b'\xd9\x05' + struct.pack('<I', SCRATCH)
        else:
            self.u.mem_write(SCRATCH, struct.pack('<Q', bits))
            code = b'\xdd\x05' + struct.pack('<I', SCRATCH)
        code += (b'\xc2' + struct.pack('<H', pop)) if pop else b'\xc3'
        stub = self.stub_next
        self.stub_next += 16
        self.u.mem_write(stub, code)
        self.u.reg_write(UC_X86_REG_EIP, stub)

    def event(self, name, *bits):
        self.events.append({'name': name, 'bits': [b & 0xffffffff for b in bits]})

    # ---- execution ------------------------------------------------------
    def _null(self, u, access, address, size, value, _):
        page_zero_access(u, address, size)

    def _hook(self, u, pc, size, _):
        if pc == STOP:
            u.emu_stop()
            return
        if P.base <= pc < P.base + P.image_size:
            raw = P.range(pc, size)
            assert raw == V.range(pc, size), hex(pc)
            VISITED[pc] = raw.hex()
        handler = self.seams.get(pc)
        if handler is not None:
            handler(self)
        elif pc in ALLOCATORS:
            raise UnseamedAllocator(hex(pc))

    def call(self, entry, ecx=0, args=(), count=2_000_000):
        self.w32(STACK, STOP, *args)
        self.u.reg_write(UC_X86_REG_ESP, STACK)
        self.u.reg_write(UC_X86_REG_ECX, ecx)
        keep = [(UC_X86_REG_EBX, 0x11223344), (UC_X86_REG_ESI, 0x33445566),
                (UC_X86_REG_EDI, 0x55667788), (UC_X86_REG_EBP, 0x778899aa)]
        for r, v in keep:
            self.u.reg_write(r, v)
        self.u.emu_start(entry, STOP, count=count)
        assert self.u.reg_read(UC_X86_REG_EIP) == STOP, hex(self.u.reg_read(UC_X86_REG_EIP))
        self.final_esp = self.esp()
        assert all(self.u.reg_read(r) == v for r, v in keep), 'nonvolatile register changed'
        return self.u.reg_read(UC_X86_REG_EAX)

    def fpu_top(self):
        """TOP field of the x87 status word (0 means an empty balanced stack)."""
        return (self.u.reg_read(UC_X86_REG_FPSW) >> 11) & 7


def write_document(name, scope, cases, extra=None):
    doc = {'schema': 1, 'scope': scope, 'original_sha256': ORIGINAL_SHA, 'v14_sha256': V14_SHA,
           'case_count': len(cases), 'cases': cases}
    if extra:
        doc.update(extra)
    doc['instructions'] = [{'va': hex(a), 'bytes': b} for a, b in sorted(VISITED.items())]
    out = HERE / name
    out.write_text(json.dumps(doc, indent=1) + '\n')
    digest = hashlib.sha256(out.read_bytes()).hexdigest()
    print(json.dumps({'file': name, 'cases': len(cases), 'instructions': len(VISITED), 'sha256': digest}))
    return digest
