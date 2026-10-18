"""Shared Unicorn machine for phase8 metagame oracles.

Maps the ORIGINAL AA.exe image (sha256 pinned) and checks every executed
instruction byte against the restored v14 AA.exe (they must be identical for
the claimed range). Heap objects are synthetic; every non-original behaviour is
an explicit seam listed in SEAMS of the calling script.
"""
from pathlib import Path
import sys, struct, hashlib
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]                      # rust-rebuild
GAME = ROOT.parent                          # E:/Games/Axis and Alies
sys.path.insert(0, str(GAME / 'reconstruction/tools/python-packages'))
sys.path.insert(0, str(ROOT / 'research/engine-baseline'))
from audit_baseline import PE
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32, UC_HOOK_CODE, UC_HOOK_MEM_UNMAPPED, UC_HOOK_MEM_READ, UC_HOOK_MEM_WRITE
from unicorn.x86_const import *

P = PE(GAME / 'AA.exe')
V = PE(GAME / 'reconstruction/native-game/runtime-expanded-v14/AA.exe')
ORIGINAL_SHA256 = '5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26'
V14_SHA256 = '3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1'
assert hashlib.sha256(P.b).hexdigest() == ORIGINAL_SHA256
assert hashlib.sha256(V.b).hexdigest() == V14_SHA256
VISITED = {}

HEAP = 0x2000000
HEAP_SIZE = 0x1000000
STACK_TOP = 0x2f00000
STOP = 0x2fff000          # return sentinel (mapped, never executed)
STUB = 0x2ffe000          # trampolines written per call
OUT = 0x2ffd000           # trampoline output area

# Emulator guards (docs/NATIVE-PORT-RULES.md "Emulator caveats"):
# - FS base is 0, so the MSVC SEH link fs:[0] is linear 0..3. Page 0 is mapped
#   only for that word: any other access below 0x1000 (or a non-FS access to
#   0..3) is a NULL dereference, stops the run and is reported.
# - Allocators: the CRT heap internals and the engine pool must never run in
#   the emulator (uninitialised heap state would crash and look like a native
#   fault). Every allocator entry not replaced by a seam traps.
# - Imports: the image's IAT holds unbound RVAs (e.g. HeapAlloc -> 0x720368,
#   which is engine code). Every IAT slot is redirected to an unmapped trap
#   address so an unseamed import call faults with its name.
ALLOCATOR_TRAPS = {
    0x96e0f8: '__heap_alloc', 0x96e173: '__nh_malloc', 0x96e19f: '_malloc',
    0x96d331: '_free', 0x97117c: '_realloc', 0x97259a: '_calloc',
    0x975640: '__msize', 0x97184d: '__aligned_malloc',
    0x9717bd: '__aligned_offset_malloc', 0x6f7810: 'engine operator new',
    0x6f7430: 'engine operator delete', 0x6f7700: 'engine pool allocator',
}
IMPORT_TRAP = 0x0e000000


def _imports():
    pe = struct.unpack_from('<I', P.b, 0x3c)[0]
    op = pe + 24
    imp_rva = struct.unpack_from('<I', P.b, op + 96 + 8)[0]
    out = []
    d = P.base + imp_rva
    while True:
        oft, _, _, name_rva, ft = struct.unpack('<5I', P.range(d, 20))
        if ft == 0:
            break
        dll = P.range(P.base + name_rva, 64).split(b'\0')[0].decode()
        i = 0
        while True:
            thunk = struct.unpack('<I', P.range(P.base + (oft or ft) + 4 * i, 4))[0]
            if thunk == 0:
                break
            fn = f'#{thunk & 0xffff}' if thunk >> 31 else P.range(P.base + thunk + 2, 64).split(b'\0')[0].decode()
            out.append((P.base + ft + 4 * i, f'{dll}::{fn}'))
            i += 1
        d += 20
    return out


IMPORTS = _imports()


def install_guards(m):
    """Page-0, allocator and import guards on a Machine-like object (u, seams)."""
    m.null_accesses = []
    m.u.hook_add(UC_HOOK_MEM_READ | UC_HOOK_MEM_WRITE, m._null, begin=0, end=0xfff)
    for a in ALLOCATOR_TRAPS:
        m.u.hook_add(UC_HOOK_CODE, m._allocator_trap, begin=a, end=a)
    for i, (slot, _) in enumerate(IMPORTS):
        m.u.mem_write(slot, struct.pack('<I', IMPORT_TRAP + 16 * i))


class NullAccess(RuntimeError):
    pass


class Machine:
    def __init__(self, control_word):
        self.u = Uc(UC_ARCH_X86, UC_MODE_32)
        self.u.mem_map(0, 0x1000)
        self.u.mem_map(P.base, (P.image_size + 4095) & ~4095)
        for vs, r, size, off in P.sections:
            self.u.mem_write(P.base + r, P.b[off:off + size])
        self.u.mem_map(HEAP, HEAP_SIZE)
        self.next = HEAP + 0x1000
        self.cw = control_word
        self.seams = {}
        self.stubs = {}
        self.seam_log = []
        self.u.hook_add(UC_HOOK_CODE, self._hook)
        self.u.hook_add(UC_HOOK_MEM_UNMAPPED, self._unmapped)
        self.fault = None
        self.last = []
        install_guards(self)

    # ---- memory helpers
    def alloc(self, n, align=16):
        p = (self.next + align - 1) & ~(align - 1)
        self.next = p + n
        assert self.next < STACK_TOP - 0x100000, 'heap exhausted'
        self.u.mem_write(p, b'\0' * n)
        return p
    def w32(self, a, v): self.u.mem_write(a, struct.pack('<I', v & 0xffffffff))
    def r32(self, a): return struct.unpack('<I', self.u.mem_read(a, 4))[0]
    def w16(self, a, v): self.u.mem_write(a, struct.pack('<H', v & 0xffff))
    def wf32bits(self, a, bits): self.w32(a, bits)
    def array(self, values):
        p = self.alloc(max(4, 4 * len(values)))
        for i, v in enumerate(values):
            self.w32(p + 4 * i, v)
        return p
    def esp(self): return self.u.reg_read(UC_X86_REG_ESP)
    def arg(self, n):  # stack argument n at a function entry/seam
        return self.r32(self.esp() + 4 + 4 * n)
    def ret(self, pop=0, value=None):
        sp = self.esp()
        self.u.reg_write(UC_X86_REG_EIP, self.r32(sp))
        self.u.reg_write(UC_X86_REG_ESP, sp + 4 + pop)
        if value is not None:
            self.u.reg_write(UC_X86_REG_EAX, value & 0xffffffff)

    # ---- tracing / seams
    def _unmapped(self, u, access, address, size, value, _):
        self.fault = (access, address, u.reg_read(UC_X86_REG_EIP))
        if IMPORT_TRAP <= address < IMPORT_TRAP + 16 * len(IMPORTS):
            self.fault += ('unseamed import ' + IMPORTS[(address - IMPORT_TRAP) // 16][1],)
        return False

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

    def _allocator_trap(self, u, pc, size, _):
        if pc not in self.seams:
            raise AssertionError(f'unseamed allocator {ALLOCATOR_TRAPS[pc]} at {pc:#x}')

    def check_null(self):
        """Raise NullAccess when the last run stopped on a page-0 access."""
        if self.null_accesses:
            pc, address, size = self.null_accesses[0]
            self.null_accesses = []
            e = NullAccess(f'null access {address:#x} at {pc:#x}')
            e.pc, e.address, e.size = pc, address, size
            raise e

    def _hook(self, u, pc, size, _):
        self.last.append(pc)
        if len(self.last) > 4096:
            del self.last[:2048]
        seam = self.seams.get(pc)
        if seam is not None:
            self.seam_log.append(pc)
            seam(self)
            return
        if P.base <= pc < P.base + P.image_size:
            raw = P.range(pc, size)
            assert raw == V.range(pc, size), f'original/v14 differ at {pc:#x}'
            VISITED[pc] = raw.hex()

    # ---- calls
    def call(self, target, args=(), ecx=0, store_st0=False, count=2_000_000):
        """Run target with the given 32-bit stack args. Returns (eax, st0 bytes|None).
        A trampoline `call target; [fstp tbyte [OUT]]` is used so callee-cleanup and
        caller-cleanup conventions both work and st0 can be captured exactly."""
        # Unicorn caches translated blocks; code at a stub address is never
        # rewritten, so each (target, store_st0) pair gets its own stub.
        key = (target, store_st0)
        if key not in self.stubs:
            at = STUB + 16 * len(self.stubs)
            assert at + 16 <= STUB + 0x1000
            code = bytearray(bytes([0xe8]) + struct.pack('<i', target - (at + 5)))
            if store_st0:
                code += bytes([0xdb, 0x3d]) + struct.pack('<I', OUT)  # fstp tbyte ptr [OUT]
            self.u.mem_write(at, bytes(code))
            self.stubs[key] = (at, at + len(code))
        start, end = self.stubs[key]
        sp = STACK_TOP - 0x1000
        for i, v in enumerate(args):
            self.w32(sp + 4 * i, v)
        self.u.reg_write(UC_X86_REG_ESP, sp)
        self.u.reg_write(UC_X86_REG_EBP, 0)
        self.u.reg_write(UC_X86_REG_ECX, ecx)
        self.u.reg_write(UC_X86_REG_FPCW, self.cw)
        # Each original call starts with an empty x87 stack (MSVC ABI).
        self.u.reg_write(UC_X86_REG_FPSW, 0)
        self.u.reg_write(UC_X86_REG_FPTAG, 0xffff)
        self.fault = None
        self.null_accesses = []
        try:
            self.u.emu_start(start, end, count=count)
        except Exception as e:
            raise RuntimeError(f'{e} fault={self.fault} null={self.null_accesses} last={[hex(x) for x in [p for p in self.last if p > 0x400000][-24:]]}') from e
        self.check_null()
        assert self.u.reg_read(UC_X86_REG_EIP) == end, hex(self.u.reg_read(UC_X86_REG_EIP))
        eax = self.u.reg_read(UC_X86_REG_EAX)
        st0 = bytes(self.u.mem_read(OUT, 10)) if store_st0 else None
        return eax, st0

def ext_to_json(raw):
    """x87 80-bit value as {sign, exponent, significand} hex, exact."""
    mant = int.from_bytes(raw[:8], 'little')
    se = int.from_bytes(raw[8:10], 'little')
    return {'sign': se >> 15, 'exponent': se & 0x7fff, 'significand': f'{mant:016x}'}
