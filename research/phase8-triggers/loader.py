"""Lean Unicorn machine for the ORIGINAL AA.exe data loaders (phase8).

Unlike harness.py (which replaces CString helpers), this machine runs the
original ATL CString code: 856370 returns a fake IAtlStringMgr whose five
methods (Allocate/Free/Reallocate/GetNilString/Clone) are implemented here
with real CStringData headers.  Heap 6F7810/6F7430, logging, timers and
Win32 entry points are explicit seams.  Code hooks are installed only on seam
addresses; visited original code is pinned per basic block.
"""
from pathlib import Path
import hashlib, struct, sys
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase4-body'))
from original_body import IMAGE, ACCEPTED, pe  # noqa: E402
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32, UC_HOOK_CODE, UC_HOOK_BLOCK, UC_HOOK_MEM_READ, UC_HOOK_MEM_WRITE, UcError  # noqa: E402
from unicorn.x86_const import *  # noqa: E402,F403

HEAP, HEAP_SIZE = 0x10000000, 0x08000000
STACK, STACK_SIZE = 0x0f000000, 0x00100000
STUBS = 0x0e000000
STOP = STUBS + 0xff0
IMPORT_STUBS = 0x0d000000
MGR = STUBS + 0x800
LOGGERS = {0x404ea0: 'log_error', 0x404ed0: 'log_warning', 0x404f30: 'log_warning2', 0x404f60: 'log_info',
           0x404f90: 'log_resource', 0x404fc0: 'log_debug', 0x405cb0: 'log_var'}


def imports():
    """IAT slot address -> 'dll!name' from the original PE import directory."""
    data = IMAGE.data
    p = struct.unpack_from('<I', data, 0x3c)[0]
    opt = p + 24
    rva = struct.unpack_from('<I', data, opt + 104)[0]

    def off(r):
        for _, start, length, offset in IMAGE.sections:
            if start <= r < start + max(length, 1):
                return offset + r - start
        raise ValueError(hex(r))

    def cstr(r):
        o = off(r)
        return data[o:data.index(b'\0', o)].decode()

    out = {}
    d = off(rva)
    while True:
        ilt, _, _, name, iat = struct.unpack_from('<5I', data, d)
        if name == 0:
            break
        dll = cstr(name)
        src = ilt or iat
        i = 0
        while True:
            entry = struct.unpack_from('<I', data, off(src) + 4 * i)[0]
            if entry == 0:
                break
            fn = f'#{entry & 0xffff}' if entry & 0x80000000 else cstr(entry + 2)
            out[IMAGE.base + iat + 4 * i] = f'{dll}!{fn}'
            i += 1
        d += 20
    return out


def fs_prefixed(uc, pc):
    """True when the instruction at pc carries the FS segment override (0x64)."""
    for byte in bytes(uc.mem_read(pc, 4)):
        if byte == 0x64:
            return True
        if byte not in (0x26, 0x2e, 0x36, 0x3e, 0x65, 0x66, 0x67, 0xf0, 0xf2, 0xf3):
            return False
    return False


def guard_null_page(uc):
    """FS base is 0 here, so the SEH link fs:[0] is linear 0..3. Only an
    FS-prefixed instruction may touch it; every other page-0 access is a native
    null dereference and fails the run (docs/NATIVE-PORT-RULES.md caveat)."""
    def null_page(u, access, address, size, value, data):
        pc = u.reg_read(UC_X86_REG_EIP)
        if address + size > 4 or not fs_prefixed(u, pc):
            raise RuntimeError(f'native null-page access at {address:#x} pc={pc:#x}')
    uc.hook_add(UC_HOOK_MEM_READ | UC_HOOK_MEM_WRITE, null_page, begin=0, end=0xfff)


IMPORTS = imports()


class Loader:
    def __init__(self, precision_cw=0x027f):
        u = self.u = Uc(UC_ARCH_X86, UC_MODE_32)
        IMAGE.load(u)
        u.mem_map(0, 0x1000)                       # fs:[0] SEH chain lives at 0
        u.mem_map(HEAP, HEAP_SIZE)
        u.mem_map(STACK, STACK_SIZE)
        u.mem_map(STUBS, 0x1000)
        u.mem_write(STUBS, b'\xc3' * 0x1000)
        u.reg_write(UC_X86_REG_FPCW, precision_cw)
        self.bump = HEAP + 0x1000
        self.sizes = {}
        self.visited = {}
        self.diags = []
        self.seams = {}
        self.calls = []
        u.hook_add(UC_HOOK_BLOCK, self._block)

        guard_null_page(u)
        # IAtlStringMgr: vtable at MGR, object MGR+0x40 -> vtable
        self.mgr = MGR + 0x40
        self.w(self.mgr, MGR)
        for i in range(5):
            self.w(MGR + i * 4, STUBS + 0x100 + i * 0x10)
            self.seam(STUBS + 0x100 + i * 0x10, getattr(self, ['_alloc', '_free', '_realloc', '_nil', '_clone'][i]))
        self.nil = self.alloc(0x20)
        self.w(self.nil, self.mgr)
        self.w(self.nil + 4, 0)
        self.w(self.nil + 8, 0)
        self.w(self.nil + 0xc, 2)
        self.seam(0x856370, lambda: self.ret(0, self.mgr))
        self.seam(0x6f7810, lambda: self.ret(0, self.alloc(self.arg(0))))
        self.seam(0x6f7430, lambda: self.ret(0))
        # Fixed-size object pools (KKC pool 85C680 alloc): the pool objects are
        # built by static initializers; blocks are served from the fixture heap.
        self.seam(0x85c680, lambda: self.ret(0, self.alloc(0x200)))
        self.seam(0x85c2b0, lambda: self.ret(4))         # pool free
        for a, name in LOGGERS.items():
            self.seam(a, (lambda n: lambda: (self.diags.append(n), self.ret(0)))(name))
        # CRT per-thread data. The game never changes LC_CTYPE: its only
        # setlocale (97042E) callers (7F19D9/7F1A08/7F1AA1) query LC_NUMERIC and
        # restore it to "C"; __init_ctype (97E948) is reached only through
        # setlocale, and the only writer of __ptlocinfo [B35444] is setlocale.
        # So the thread uses the image's static initial locale info B353F0
        # (C locale, _pctype [B358A0] = static table AD0680), as in the game.
        self.ptd = self.alloc(0x100)
        self.locinfo = self.r(0xb35444)
        assert self.locinfo == 0xb353f0
        self.w(self.ptd + 0x64, self.locinfo)
        self.seam(0x9738a9, lambda: self.ret(0, self.ptd))
        # Bound game-setting tables [B39940] and [B39990]: every slot points at
        # a zero cell unless a fixture overrides it (no fall-through to page 0).
        self.zero = self.alloc(16)
        for table in (0xb39940, 0xb39990):
            block = self.alloc(0x1000)
            for off in range(0, 0x1000, 4):
                self.w(block + off, self.zero)
            self.w(table, block)
        # 405370 game-variable lookup (thiscall, &name): node -> entry; string
        # variables keep their CString at entry+10, bytes at the entry itself.
        self.vars = {'languageids': ('string', 'English')}
        self.var_reads = []
        self.seam(0x405370, self._var)
        # Every IAT slot points at its own stub; unknown imports fail loudly.
        u.mem_map(IMPORT_STUBS, 0x10000)
        u.mem_write(IMPORT_STUBS, b'\xcc' * 0x10000)
        self.import_handlers = {
            'KERNEL32.dll!InitializeCriticalSection': (4, 0),
            'KERNEL32.dll!DeleteCriticalSection': (4, 0),
            'KERNEL32.dll!EnterCriticalSection': (4, 0),
            'KERNEL32.dll!LeaveCriticalSection': (4, 0),
            'KERNEL32.dll!GetCurrentThreadId': (0, 0x1000),
            'KERNEL32.dll!InterlockedIncrement': None,
            'KERNEL32.dll!InterlockedDecrement': None,
        }
        self.import_log = []
        for i, (slot, name) in enumerate(sorted(IMPORTS.items())):
            stub = IMPORT_STUBS + i * 4
            self.w(slot, stub)
            self.seam(stub, (lambda n: lambda: self._import(n))(name))

    def _var(self):
        name = self.text(self.r(self.arg(0)))
        self.var_reads.append(name)
        spec = self.vars.get(name.lower())
        if spec is None:
            self.ret(4, 0)
            return
        node = self.alloc(8)
        entry = self.alloc(0x40)
        self.w(node, entry)
        if spec[0] == 'string':
            self.w(entry + 0x10, self.cstring(spec[1]))
        else:
            self.u.mem_write(entry, bytes([spec[1] & 0xff]))
        self.ret(4, node)

    def _import(self, name):
        self.import_log.append(name)
        if name == 'KERNEL32.dll!HeapAlloc':
            self.ret(0xc, self.alloc(self.arg(2)))
            return
        if name == 'KERNEL32.dll!HeapFree':
            self.ret(0xc, 1)
            return
        if name == 'KERNEL32.dll!HeapReAlloc':
            old, size = self.arg(2), self.arg(3)
            new = self.alloc(size)
            self.u.mem_write(new, bytes(self.u.mem_read(old, min(size, self.sizes.get(old, size)))))
            self.ret(0x10, new)
            return
        if name == 'KERNEL32.dll!HeapSize':
            self.ret(0xc, self.sizes.get(self.arg(2), 0))
            return
        if name in ('KERNEL32.dll!InterlockedIncrement', 'KERNEL32.dll!InterlockedDecrement'):
            p = self.arg(0)
            v = (self.r(p) + (1 if name.endswith('Increment') else -1)) & 0xffffffff
            self.w(p, v)
            self.ret(4, v)
            return
        spec = self.import_handlers.get(name)
        if spec is None:
            raise RuntimeError(f'unhandled import {name}')
        self.ret(*spec)

    # ---- memory helpers ----
    def w(self, a, v):
        self.u.mem_write(a, struct.pack('<I', v & 0xffffffff))

    def r(self, a):
        return struct.unpack('<I', self.u.mem_read(a, 4))[0]

    def alloc(self, n):
        p = self.bump
        self.sizes[p] = n
        self.bump += (max(n, 1) + 15) & ~15
        assert self.bump < HEAP + HEAP_SIZE
        self.u.mem_write(p, b'\0' * max(n, 1))
        return p

    def arg(self, i):
        return self.r(self.u.reg_read(UC_X86_REG_ESP) + 4 + 4 * i)

    def ret(self, pop, value=0):
        sp = self.u.reg_read(UC_X86_REG_ESP)
        self.u.reg_write(UC_X86_REG_EIP, self.r(sp))
        self.u.reg_write(UC_X86_REG_ESP, sp + 4 + pop)
        self.u.reg_write(UC_X86_REG_EAX, value & 0xffffffff)

    def seam(self, address, fn):
        self.seams[address] = fn
        self.u.hook_add(UC_HOOK_CODE, lambda uc, a, n, d: fn(), begin=address, end=address)

    def _block(self, uc, a, n, d):
        if 0x400000 <= a < 0xc00000:
            self.visited[a] = n

    # ---- CString manager ----
    def _data(self, chars, char_size=2):
        d = self.alloc(16 + (chars + 1) * char_size)
        self.w(d, self.mgr)
        self.w(d + 4, 0)
        self.w(d + 8, chars)
        self.w(d + 0xc, 1)
        return d

    def _alloc(self):
        self.ret(8, self._data(self.arg(0), self.arg(1)))

    def _free(self):
        self.ret(4)

    def _realloc(self):
        old, n, size = self.arg(0), self.arg(1), self.arg(2)
        d = self._data(n, size)
        length = self.r(old + 4)
        self.u.mem_write(d + 16, bytes(self.u.mem_read(old + 16, (length + 1) * size)))
        self.w(d + 4, length)
        self.ret(0xc, d)

    def _nil(self):
        self.ret(0, self.nil)

    def _clone(self):
        self.ret(0, self.mgr)

    def cstring(self, text):
        raw = text.encode('utf-16le')
        d = self._data(len(text))
        self.u.mem_write(d + 16, raw + b'\0\0')
        self.w(d + 4, len(text))
        self.w(d + 0xc, 0x10000)          # held by the fixture
        return d + 16

    def text(self, p, limit=100000):
        out = bytearray()
        while len(out) < limit:
            pair = bytes(self.u.mem_read(p, 2))
            p += 2
            if pair == b'\0\0':
                break
            out += pair
        return out.decode('utf-16le', errors='surrogatepass')

    # ---- calls ----
    def call(self, address, ecx=0, args=(), count=50_000_000):
        sp = STACK + STACK_SIZE - 0x1000
        sp -= 4 * len(args)
        for i, v in enumerate(args):
            self.w(sp + 4 * i, v)
        sp -= 4
        self.w(sp, STOP)
        u = self.u
        for reg in (UC_X86_REG_EBX, UC_X86_REG_ESI, UC_X86_REG_EDI, UC_X86_REG_EBP):
            u.reg_write(reg, 0x5a5a0000)
        u.reg_write(UC_X86_REG_ESP, sp)
        u.reg_write(UC_X86_REG_ECX, ecx)
        try:
            u.emu_start(address, STOP, count=count)
        except UcError as e:
            raise RuntimeError(f'{e} at {u.reg_read(UC_X86_REG_EIP):#x}') from e
        assert u.reg_read(UC_X86_REG_EIP) == STOP, hex(u.reg_read(UC_X86_REG_EIP))
        return u.reg_read(UC_X86_REG_EAX)

    def windows(self):
        out = []
        for a, n in sorted(self.visited.items()):
            raw = IMAGE.at(a, n)
            assert raw == ACCEPTED.at(a, n), hex(a)
            out.append({'address': hex(a), 'sha256': hashlib.sha256(raw).hexdigest()})
        return out
