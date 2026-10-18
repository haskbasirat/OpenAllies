"""Shared Unicorn machine for phase-8 definition oracles.

Reuses the restoration agent's whole-parser harness
(research/phase8-restoration/native_tgi_parser.py: original 5BD080 TGI parser
with line-source/heap/mutex seams) and adds:
- a wider fixture heap so whole original data files can be parsed;
- hook-implemented stub functions (fixture leaves) at fixed addresses;
- a C++ exception dispatch seam: when the original code reaches
  _CxxThrowException (96BEF5) and a registered frame is in a try state whose
  handler type is among the thrown object's catchable types (MSVC ThrowInfo),
  the ORIGINAL catch funclet runs with that frame's EBP, and execution resumes
  at the continuation it returns with ESP = [EBP-10] (MSVC try-frame
  convention). Destructors of the unwound inner frames are not run (only
  reference counts would be affected); this is recorded in each fixture.
"""
from pathlib import Path
import sys, struct, hashlib
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase8-restoration'))
sys.path.insert(0, str(HERE))
import native_tgi_parser as ntp
from native_tgi_parser import Machine as ParserMachine, lines_of, P, V, H
from unicorn import UC_HOOK_MEM_READ, UC_HOOK_MEM_WRITE
from unicorn.x86_const import *
import xref

BIG = 0x10000000
BIG_SIZE = 0x08000000  # 128 MiB fixture heap (fixture-only, not a game limit)
THROW = 0x96bef5
STUBS = 0x301f700      # hook-implemented fixture functions live here
CATCH_RETURN = 0x301f6f0
# Original leveled log helpers (404EA0 level 0 ... 404F90 level 20).
LOG_LEAVES = (0x404ea0, 0x404ed0, 0x404f00, 0x404f30, 0x404f60, 0x404f90)


def signed(v):
    return v - (1 << 32) if v & 0x80000000 else v


class Machine(ParserMachine):
    def __init__(self, lines=()):
        self._big = None
        self.stubs = {}
        self.eh_frames = []      # [(entry_pc, funcinfo, ebp)]
        self.eh_watch = {}       # post-prologue pc -> funcinfo
        self.eh_events = []
        self.throw_formats = []
        self.logs = []
        self._catch = None
        super().__init__(lines)
        # The base harness allocates the template-registry object at
        # [B39BC8] with 16 bytes, but the parser seam fills its hash map at
        # +24/+28; give it a full-size block so later fixture objects are not
        # overwritten (same field values copied).
        old = self.r(0xb39bc8)
        reg = self.alloc(0x80)
        self.u.mem_write(reg, bytes(self.u.mem_read(old, 16)))
        self.w(0xb39bc8, reg)
        # FS base is 0 in this emulator, so the MSVC SEH link fs:[0] is linear
        # 0..3 (page 0 is mapped by the base harness). Any other page-0
        # access, or one without the FS prefix, is a null dereference and
        # fails the case (NATIVE-PORT-RULES emulator caveat).
        self.null_accesses = []
        self.u.hook_add(UC_HOOK_MEM_READ | UC_HOOK_MEM_WRITE, self._null, begin=0, end=0xfff)

    def _null(self, u, access, address, size, value, _):
        pc = u.reg_read(UC_X86_REG_EIP)
        if address + size > 4 or not self._fs_prefixed(pc):
            self.null_accesses.append((hex(pc), hex(address), size))
            u.emu_stop()

    def _fs_prefixed(self, pc):
        for byte in bytes(self.u.mem_read(pc, 4)):
            if byte == 0x64:
                return True
            if byte not in (0x26, 0x2e, 0x36, 0x3e, 0x65, 0x66, 0x67, 0xf0, 0xf2, 0xf3):
                return False
        return False

    def check_null(self):
        assert not self.null_accesses, self.null_accesses

    # -- heap ---------------------------------------------------------------
    def alloc(self, n):
        if getattr(self, '_big', None) is None:
            try:
                self.u.mem_map(BIG, BIG_SIZE)
            except Exception:
                return super().alloc(n)
            self._big = BIG
        p = self._big
        self._big += (n + 15) & ~15
        assert self._big < BIG + BIG_SIZE, 'fixture heap exhausted'
        return p

    def wstr(self, p):
        if p == 0:
            return None
        n = self.r(p - 12)
        return bytes(self.u.mem_read(p, n * 2)).decode('utf-16-le', 'replace')

    def raw_wstr(self, p):
        out = bytearray()
        while True:
            pair = bytes(self.u.mem_read(p, 2))
            if pair == b'\0\0':
                break
            out += pair
            p += 2
        return out.decode('utf-16-le', 'replace')

    def cstring(self, text):
        return self.string([ord(c) for c in text])

    def reparse(self, data: bytes, name='oracle.tgi'):
        """Parse another whole file in the same process state (template
        registry, string pool), as the original loader does file by file."""
        self.lines = lines_of(data)
        self.at = 0
        self.messages = []
        self.thrown = None
        resource = self.alloc(0x100)
        self.w(resource + 0x38, self.cstring(name))
        stream = self.alloc(0x40)
        vt = self.alloc(0x100)
        self.w(stream, vt)
        self.w(vt + 0x48, ntp.EOF_STUB)
        self.w(self.stack, self.stop)
        self.w(self.stack + 4, stream)
        self.u.reg_write(UC_X86_REG_ESP, self.stack)
        self.u.reg_write(UC_X86_REG_ECX, resource)
        self.u.emu_start(0x5bd080, self.stop, count=50_000_000)
        self.check_null()
        if self.thrown is not None:
            return {'error': self.thrown, 'line': self.at, 'resource': resource}
        assert self.u.reg_read(UC_X86_REG_EIP) == self.stop, hex(self.u.reg_read(UC_X86_REG_EIP))
        return {'error': None, 'resource': resource, 'root': self.r(resource + 0x3c)}

    # -- stubs --------------------------------------------------------------
    def stub(self, fn):
        addr = STUBS + 0x10 * len(self.stubs)
        self.stubs[addr] = fn
        return addr

    # -- C++ EH seam --------------------------------------------------------
    def watch_frames(self, post_prologue_pc, funcinfo_addr):
        self.eh_watch[post_prologue_pc] = xref.funcinfo(funcinfo_addr)

    def _dispatch(self, u):
        esp = u.reg_read(UC_X86_REG_ESP)
        obj, ti = self.r(esp + 4), self.r(esp + 8)
        catchable = []
        if ti:
            cta = self.r(ti + 12)
            for i in range(self.r(cta)):
                catchable.append(self.r(self.r(cta + 4 + 4 * i) + 4))
        for idx in range(len(self.eh_frames) - 1, -1, -1):
            fi, ebp = self.eh_frames[idx]
            if ebp <= esp:
                continue
            state = signed(self.r(ebp - 4))
            for tb in fi['try_blocks']:
                if not (tb['try_low'] <= state <= tb['try_high']):
                    continue
                for h in tb['handlers']:
                    htype = int(h['type'], 16)
                    if htype == 0 or htype in catchable:
                        if h['disp']:
                            self.w((ebp + h['disp']) & 0xffffffff, obj)
                        # __FrameUnwindToState(tryLow) before the funclet.
                        self.w(ebp - 4, tb['try_low'] & 0xffffffff)
                        self.eh_events.append({'frame': idx, 'state': state,
                                               'handler': h['handler'], 'type': h['type'],
                                               'message': self.messages[-1] if self.messages else None})
                        del self.eh_frames[idx + 1:]
                        self._catch = ebp
                        scratch = esp - 0x400
                        self.w(scratch, CATCH_RETURN)
                        u.reg_write(UC_X86_REG_ESP, scratch)
                        u.reg_write(UC_X86_REG_EBP, ebp)
                        u.reg_write(UC_X86_REG_EIP, int(h['handler'], 16))
                        return True
        return False

    def hook(self, u, pc, size, user):
        if pc in self.stubs:
            self.stubs[pc](self, u)
            return
        if pc in LOG_LEAVES:
            # Logging channel (B38B70/B38B98) objects are runtime-initialized;
            # the log call is a recorded leaf: (channel, file, line, function, fmt, ...).
            esp = u.reg_read(UC_X86_REG_ESP)
            try:
                self.logs.append({'level_fn': hex(pc), 'function': self.raw_wstr(self.r(esp + 16)),
                                  'format': self.raw_wstr(self.r(esp + 20))})
            except Exception:
                self.logs.append({'level_fn': hex(pc)})
            self.ret()
            return
        if pc == 0x85c2b0:
            # Fixed-size node pool release (pair of the existing 85C680
            # allocation seam): fixture leaf, memory is not reused.
            self.ret(pop=4)
            return
        if pc == 0x5ab1d0:
            # KKC_Resource full-path formatter (depot directory lookup through
            # the OS working folder); used only for log text. Leaf: the
            # resource's registered path (+38).
            esp = u.reg_read(UC_X86_REG_ESP)
            out = self.r(esp + 4)
            res = u.reg_read(UC_X86_REG_ECX)
            self.w(out, self.cstring(self.wstr(self.r(res + 0x38)) or ''))
            self.ret(out, pop=4)
            return
        if pc in (0x410c40, 0x41dfb0):
            # Record the original throw helper's format text (observation only).
            try:
                self.throw_formats.append(self.raw_wstr(self.r(u.reg_read(UC_X86_REG_ESP) + 4)))
            except Exception:
                self.throw_formats.append(None)
        if pc in self.eh_watch:
            ebp = u.reg_read(UC_X86_REG_EBP)
            self.eh_frames = [f for f in self.eh_frames if f[1] > ebp]
            self.eh_frames.append((self.eh_watch[pc], ebp))
        if pc == CATCH_RETURN and self._catch is not None:
            ebp = self._catch
            self._catch = None
            cont = u.reg_read(UC_X86_REG_EAX)
            u.reg_write(UC_X86_REG_EBP, ebp)
            u.reg_write(UC_X86_REG_ESP, self.r(ebp - 0x10))
            u.reg_write(UC_X86_REG_EIP, cont)
            return
        if pc == THROW and self.eh_frames:
            if self._dispatch(u):
                return
        super().hook(u, pc, size, user)

    # -- calls --------------------------------------------------------------
    def cdecl(self, pc, args, this=0, count=20_000_000):
        self.w(self.stack, self.stop)
        for i, v in enumerate(args):
            self.w(self.stack + 4 + 4 * i, v)
        self.u.reg_write(UC_X86_REG_ESP, self.stack)
        self.u.reg_write(UC_X86_REG_ECX, this)
        self.u.emu_start(pc, self.stop, count=count)
        self.check_null()
        assert self.u.reg_read(UC_X86_REG_EIP) == self.stop, hex(self.u.reg_read(UC_X86_REG_EIP))
        return self.u.reg_read(UC_X86_REG_EAX)

    def thiscall(self, pc, this, args, count=20_000_000):
        """Callee-pops call (thiscall/stdcall); stops on an unhandled throw."""
        self.thrown = None
        self.messages = []
        self.throw_formats = []
        self.w(self.stack, self.stop)
        for i, v in enumerate(args):
            self.w(self.stack + 4 + 4 * i, v)
        self.u.reg_write(UC_X86_REG_ESP, self.stack)
        self.u.reg_write(UC_X86_REG_ECX, this)
        for reg, val in [(UC_X86_REG_EBX, 0x11223344), (UC_X86_REG_ESI, 0x22334455),
                         (UC_X86_REG_EDI, 0x33445566), (UC_X86_REG_EBP, 0x44556677)]:
            self.u.reg_write(reg, val)
        self.u.emu_start(pc, self.stop, count=count)
        self.check_null()
        if self.thrown is not None:
            return {'thrown': self.throw_formats[-1] if self.throw_formats else
                    (self.thrown[-1] if self.thrown else True)}
        assert self.u.reg_read(UC_X86_REG_EIP) == self.stop, hex(self.u.reg_read(UC_X86_REG_EIP))
        assert self.u.reg_read(UC_X86_REG_ESP) == self.stack + 4 + len(args) * 4
        return {'eax': self.u.reg_read(UC_X86_REG_EAX)}


def native_windows(visited):
    spans = []
    for a, n in sorted(visited.items()):
        if not P.base <= a < P.base + P.image_size:
            continue
        if spans and spans[-1][1] >= a:
            spans[-1][1] = max(spans[-1][1], a + n)
        else:
            spans.append([a, a + n])
    out = []
    for a, z in spans:
        b = P.range(a, z - a)
        assert b == V.range(a, z - a), hex(a)
        out.append({'start': hex(a), 'end': hex(z), 'sha256': H(b)})
    return out
