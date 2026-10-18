"""Shared Unicorn machine for the Organization (57D130) and Transport (58C810)
saved-component oracles. Standalone (no inherited hidden seams): every hooked
native entry is listed in SEAMS below and every call to it is recorded.

Memory: page 0 mapped for the FS segment (base 0; SEH chain word at 0) with the
page-0 null guard (research/phase8-restoration/null_guard.py); original image
at 0x400000; heap/stubs/stack in 0x2000000..0x3000000. Allocation 6F7810 is a
bump allocator that fills new blocks with 0xA5 (no zero-fill is invented);
free 6F7430 is recorded and ignored (no reuse).
"""
from pathlib import Path
import struct, sys, hashlib
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/engine-baseline'))
sys.path.insert(0, str(ROOT / 'research/phase8-restoration'))
from audit_baseline import PE
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32, UC_HOOK_CODE
from unicorn.x86_const import *
import null_guard

P = PE(ROOT.parent / 'AA.exe')
V = PE(ROOT.parent / 'reconstruction/native-game/runtime-expanded-v14/AA.exe')
assert hashlib.sha256(P.b).hexdigest() == '5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26'
assert hashlib.sha256(V.b).hexdigest() == '3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1'
H = lambda b: hashlib.sha256(b).hexdigest()
VISITED = {}
CW = {'Single24': 0x7f, 'Double53': 0x27f, 'Extended64': 0x37f}

HEAP0, HEAP_END = 0x2000000, 0x2c00000
STUBS = 0x2e00000
STACK, STOP = 0x2f08000, 0x2f10000

# Global objects the code reads directly.
G_WORLD = 0xb36534        # GWorld: +2C world version, +E0/+E4 times, +284 updatable table
G_LAYOUT_REG = 0xb3ab00   # KKC_OrganizationLayoutData registry (463FA0)
G_THING_REG = 0xb39f58    # object definitions registry (4BDBA0, class 0x2B KKC_ActorData)
G_DAMAGE_REG = 0xb3aaf8   # KKC_DamageType registry (48D500)
G_ASSERT_FLAG = 0xb3cc47  # 4FC200 assert-dialog latch


class Throw(Exception):
    pass


class Truncated(Exception):
    pass


def f32(v):
    return struct.unpack('<I', struct.pack('<f', v))[0]


def f64(v):
    return struct.unpack('<Q', struct.pack('<d', v))[0]


class Machine:
    def __init__(self, payload, precision='Double53'):
        self.payload = payload
        self.at = 0
        self.reads = []
        self.calls = []       # ordered seam log
        self.logs = []        # 404EA0/404F30/5A1040 diagnostics (also in calls)
        self.outcome = None   # None = returned; dict = native throw
        self.u = Uc(UC_ARCH_X86, UC_MODE_32)
        self.u.mem_map(0, 0x1000)
        self.u.mem_map(P.base, (P.image_size + 0xfff) & ~0xfff)
        for vs, r, size, off in P.sections:
            self.u.mem_write(P.base + r, P.b[off:off + size])
        self.u.mem_map(0x2000000, 0x1000000)
        self.u.mem_write(STUBS, b'\xcc' * 0x10000)
        self.next = HEAP0
        self.next_stub = STUBS + 0x1000
        self.seams = {}
        self.probes = {}
        self.labels = {}      # pointer -> symbolic label (for logs)
        self.u.reg_write(UC_X86_REG_FPCW, CW[precision])
        self.u.hook_add(UC_HOOK_CODE, self._hook)
        null_guard.install(self)
        self._strings()
        self._stream()
        self._standard_seams()

    # ---------------------------------------------------------------- memory
    def alloc(self, n, fill=0):
        p = self.next
        self.next += (max(n, 1) + 15) & ~15
        assert self.next < HEAP_END
        self.u.mem_write(p, bytes([fill]) * max(n, 1))
        return p

    def w(self, a, v):
        self.u.mem_write(a, struct.pack('<I', v & 0xffffffff))

    def r(self, a):
        return struct.unpack('<I', self.u.mem_read(a, 4))[0]

    def b(self, a):
        return self.u.mem_read(a, 1)[0]

    def wb(self, a, v):
        self.u.mem_write(a, bytes([v & 0xff]))

    def esp(self):
        return self.u.reg_read(UC_X86_REG_ESP)

    def arg(self, n=0):
        return self.r(self.esp() + 4 + 4 * n)

    def ret(self, pop=0, eax=None):
        sp = self.esp()
        self.u.reg_write(UC_X86_REG_EIP, self.r(sp))
        self.u.reg_write(UC_X86_REG_ESP, sp + 4 + pop)
        if eax is not None:
            self.u.reg_write(UC_X86_REG_EAX, eax & 0xffffffff)

    def stub(self, handler):
        a = self.next_stub
        self.next_stub += 16
        self.seams[a] = handler
        return a

    def seam(self, address, handler):
        self.seams[address] = handler

    def vtable(self, entries):
        """entries: {offset: handler or address}"""
        size = max(entries) + 4
        t = self.alloc(size)
        for off in range(0, size, 4):
            self.w(t + off, 0)
        for off, h in entries.items():
            self.w(t + off, h if isinstance(h, int) else self.stub(h))
        return t

    def call(self, entry, this, args, count=2000000):
        self.w(STACK, STOP)
        for i, a in enumerate(args):
            self.w(STACK + 4 + 4 * i, a)
        self.u.reg_write(UC_X86_REG_ESP, STACK)
        self.u.reg_write(UC_X86_REG_ECX, this)
        try:
            self.u.emu_start(entry, STOP, count=count)
        except Throw:
            pass
        if self.null_accesses and self.outcome is None:
            # Genuine native access violation (page 0, no FS prefix).
            self.outcome = {'kind': 'NullDereference', 'pc': self.null_accesses[0][0]}
        if self.outcome is None:
            assert self.u.reg_read(UC_X86_REG_EIP) == STOP, hex(self.u.reg_read(UC_X86_REG_EIP))
            assert self.u.reg_read(UC_X86_REG_ESP) == STACK + 4 + 4 * len(args), 'callee stack pop'
        return self.u.reg_read(UC_X86_REG_EAX)

    def _hook(self, u, pc, size, _):
        if P.base <= pc < P.base + P.image_size:
            raw = P.range(pc, size)
            if pc not in VISITED:
                assert raw == V.range(pc, size), hex(pc)
                VISITED[pc] = raw.hex()
        probe = self.probes.get(pc)
        if probe is not None:  # observation only; execution continues natively
            probe(self)
        h = self.seams.get(pc)
        if h is not None:
            h(self)
        elif STUBS <= pc < STUBS + 0x10000 and pc not in self.fld_stubs:
            raise RuntimeError(('unhooked stub', hex(pc)))

    # --------------------------------------------------------------- strings
    def _strings(self):
        """CString: data pointer; header at -16 {allocator, length, capacity,
        refcount}. The allocator object's vtable: +0 Allocate(n, 2), +4
        release (ignored), +8 Reallocate(header, n, 2), +C new empty string,
        +10 identity (4012C0 compares it with the header)."""
        self.allocator = self.alloc(16)
        av = self.vtable({0x0: self._string_allocate, 0x4: lambda m: m.ret(4),
                          0x8: self._string_reallocate,
                          0xc: lambda m: m.ret(0, m.cstring('') - 16),
                          0x10: lambda m: m.ret(0, m.allocator)})
        self.w(self.allocator, av)
        self.fld_stubs = set()

    def _new_header(self, chars, length=0, data=b''):
        h = self.alloc(16 + 2 * (chars + 1))
        self.w(h, self.allocator)
        self.w(h + 4, length)
        self.w(h + 8, chars)
        self.w(h + 12, 1)
        self.u.mem_write(h + 16, data + b'\0\0')
        return h

    def _string_allocate(self, m):  # +0 Allocate(nChars, nCharSize) -> header
        assert m.arg(1) == 2
        m.ret(8, m._new_header(m.arg(0)))

    def _string_reallocate(self, m):  # +8 Reallocate(header, nChars, nCharSize)
        old, chars = m.arg(0), m.arg(1)
        assert m.arg(2) == 2
        length = m.r(old + 4)
        data = bytes(m.u.mem_read(old + 16, 2 * length))
        m.ret(12, m._new_header(chars, length, data))

    def cstring(self, text):
        raw = text.encode('utf-16-le')
        h = self.alloc(16 + len(raw) + 2)
        self.w(h, self.allocator)
        self.w(h + 4, len(text))
        self.w(h + 8, len(text))
        self.w(h + 12, 1000)  # never reaches zero inside one case
        self.u.mem_write(h + 16, raw + b'\0\0')
        return h + 16

    def read_wide(self, p, limit=512):
        out = bytearray()
        while len(out) < limit * 2:
            c = bytes(self.u.mem_read(p + len(out), 2))
            if c == b'\0\0':
                break
            out += c
        return out.decode('utf-16-le')

    def read_narrow(self, p):
        out = bytearray()
        while True:
            c = self.u.mem_read(p + len(out), 1)[0]
            if c == 0:
                return out.decode('latin1')
            out.append(c)

    def cstring_text(self, p):
        return bytes(self.u.mem_read(p, self.r(p - 12) * 2)).decode('utf-16-le')

    # ---------------------------------------------------------------- stream
    def _stream(self):
        self.ctx = self.alloc(0x100)
        vt = self.vtable({0x0: self._read, 0x4: self._write_unexpected,
                          0xc: lambda m: m.ret(0, 0),     # IsWriting: loading
                          0x14: lambda m: m.ret(0, 0),    # 4C2D90 mode query: not writing
                          0x18: lambda m: (_ for _ in ()).throw(RuntimeError('stream +18 reached'))})
        self.w(self.ctx + 0x3c, vt)
        self.stream = self.ctx + 0x3c

    def take(self, n):
        if self.at + n > len(self.payload):
            self.outcome = {'kind': 'Truncated', 'at': self.at, 'wanted': n}
            raise Throw()
        b = self.payload[self.at:self.at + n]
        self.reads.append([self.at, n])
        self.at += n
        return b

    def _read(self, m):
        dst, n = self.arg(0), self.arg(1)
        data = self.take(n)
        self.u.mem_write(dst, data)
        self.ret(8, self.stream)

    def _write_unexpected(self, m):
        raise RuntimeError('stream write during Load')

    # ----------------------------------------------------------- seams
    def record(self, name, **args):
        entry = {'seam': name, **args}
        self.calls.append(entry)
        return entry

    def throw(self, kind, **info):
        self.outcome = {'kind': kind, **info}
        raise Throw()

    def fmt_args(self, fmt, first):
        """Decode printf-style args of the original formats (%s wide, %hs narrow, %d)."""
        out = []
        i = 0
        k = first
        while i < len(fmt):
            if fmt[i] == '%':
                j = i + 1
                spec = ''
                while j < len(fmt) and fmt[j] in 'hl0123456789':
                    spec += fmt[j]
                    j += 1
                c = fmt[j] if j < len(fmt) else ''
                if c == 's':
                    p = self.arg(k)
                    out.append(self.read_narrow(p) if spec == 'h' else self.read_wide(p))
                    k += 1
                elif c == 'd':
                    v = self.arg(k)
                    out.append(v - (1 << 32) if v >> 31 else v)
                    k += 1
                i = j + 1
            else:
                i += 1
        return out

    def _standard_seams(self):
        m = self
        m.seam(0x6f7810, lambda m: m.ret(0, m.alloc(m.arg(0), 0xa5)))
        m.seam(0x6f7430, lambda m: m.ret(0))
        m.seam(0x856370, lambda m: m.ret(0, m.allocator))

        def literal_ctor(m):  # 401B00 CString(const wchar_t*) thiscall
            m.w(m.u.reg_read(UC_X86_REG_ECX), m.cstring(m.read_wide(m.arg(0))))
            m.ret(4, m.u.reg_read(UC_X86_REG_ECX))
        m.seam(0x401b00, literal_ctor)

        def placement_literal(m):  # 856560(dest, literal) cdecl
            m.w(m.arg(0), m.cstring(m.read_wide(m.arg(1))))
            m.ret(0, m.arg(0))
        m.seam(0x856560, placement_literal)

        def read_string(m):  # 41D290(stream, &CString) cdecl: u16 count then chars
            stream, dest = m.arg(0), m.arg(1)
            assert stream == m.stream
            n = struct.unpack('<H', m.take(2))[0]
            raw = m.take(2 * n)
            m.w(dest, m.cstring(raw.decode('utf-16-le')))
            m.ret(0, stream)
        m.seam(0x41d290, read_string)

        def throw_exception(m):  # 41DFB0(fmt literal, ...) cdecl
            fmt = m.read_wide(m.arg(0))
            m.throw('ThrowException', format=fmt, args=m.fmt_args(fmt, 1))
        m.seam(0x41dfb0, throw_exception)

        def file_throw(kind):
            def h(m):  # 858A00/858AD0(CString fmt by value, ...) cdecl
                fmt = m.read_wide(m.arg(0))
                m.throw(kind, format=fmt, args=m.fmt_args(fmt, 1))
            return h
        m.seam(0x858a00, file_throw('Warning'))
        m.seam(0x858ad0, file_throw('Fatal'))

        def assertion(m):  # 4F0110(fmt, file, line, expression) cdecl
            m.throw('Assertion', line=m.arg(2), expression=m.read_wide(m.arg(3)))
        m.seam(0x4f0110, assertion)

        def logger(name):
            def h(m):  # 404EA0/404F30(B38B70, file, line, function, fmt, ...) cdecl
                fmt = m.read_wide(m.arg(4))
                e = m.record(name, line=m.arg(2), format=fmt, args=m.fmt_args(fmt, 5))
                m.logs.append(e)
                m.ret(0)
            return h
        m.seam(0x404ea0, logger('404EA0'))
        m.seam(0x404f30, logger('404F30'))

        def map_warning(m):  # 5A1040(x, CString fmt, ...) cdecl
            fmt = m.cstring_text(m.arg(1))
            e = m.record('5A1040', format=fmt, args=m.fmt_args(fmt, 2))
            m.logs.append(e)
            m.ret(0)
        m.seam(0x5a1040, map_warning)

    def fld_stub(self, value_bits64):
        """Return through a fresh 'fld qword [data]; ret 4' stub (ST0 result)."""
        code = self.next_stub
        self.next_stub += 32
        data = code + 16
        self.u.mem_write(data, struct.pack('<Q', value_bits64))
        self.u.mem_write(code, b'\xdd\x05' + struct.pack('<I', data) + b'\xc2\x04\x00')
        self.fld_stubs.add(code)
        self.fld_stubs.update(range(code, code + 9))
        self.u.reg_write(UC_X86_REG_EIP, code)
