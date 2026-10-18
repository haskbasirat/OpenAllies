"""Unicorn machine for the class-changed record conversion oracle
(4FAE80 -> 4F26F0/405B80/4EA050/4F9840, 4FC340, 4FC600 tail 4FD471..4FD7B1,
4F9C10 dispatch). Standalone: every hooked native entry is a seam listed in
SEAM_DOC and every call to it is recorded in order.

Memory: page 0 mapped for the FS segment (base 0; SEH chain word at 0) with the
page-0 null guard (research/phase8-restoration/null_guard.py); original image
at 0x400000; heap/stubs/stack in 0x2000000..0x3000000. Allocation 6F7810 is a
bump allocator filling new blocks with 0xA5 (no zero-fill invented); free
6F7430 is recorded and ignored. Every executed instruction of the image is
checked byte-identical in the v14 executable (runtime-expanded-v14/AA.exe).
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
VISITED = {}
CW = {'Single24': 0x7f, 'Double53': 0x27f, 'Extended64': 0x37f}

HEAP0, HEAP_END = 0x2000000, 0x2d00000
STUBS = 0x2e00000
STACK, STOP = 0x2f08000, 0x2f10000

G_IDMGR = 0xb364c0       # ID manager: +4 + slot*2 generation words (4FAF03)
G_THING_REG = 0xb39f58   # Thing DataManager (427BD0 this)
G_TEXT = 0xb36544        # text/localization manager (4B9F40 this)
G_EMPTY = 0xb43b48       # 4B9F40 second argument (static CString)

# Original definition vtables (constructors 4BBE70 ThingData, 49C760
# ObjectData, 4624B0 ActorData, 491EF0 DynamicDoodadData). +0x30 is the
# executed IsObjectData leaf: 4C9FA0 (xor eax,eax) or 4F0500 (mov eax,1).
DEF_VTABLES = {'ThingData': 0xa20770, 'ObjectData': 0xa13318, 'ActorData': 0xa05540,
               'DynamicDoodadData': 0xa0f6e0}

SEAM_DOC = {
    '6F7810': 'operator new (bump allocator, 0xA5 fill)',
    '6F7430': 'operator delete (recorded, ignored)',
    '856370': 'CString allocator object (vtable: +4 release, +C new empty, +10 identity)',
    '401B00': 'CString(const wchar_t*) literal constructor',
    '41D290': 'stream string read: u16 count then UTF-16 units through the stream seam',
    'stream+0': 'stream read (dst, n) from the chunk payload; keeps chunk descriptor +68 (position) current',
    '427BD0': 'Thing DataManager find (fixture IDS -> definition object or null)',
    '4B9F40': 'localized text lookup of a key CString (returns a CString holding the key text)',
    '5A1040': 'KKC_MapResource map warning (ctx, CString text, IDS)',
    '41DFB0': 'TGC_LoadWarningException throw (format literal) -- ends the case',
    '858AD0': 'TGC file exception throw -- ends the case',
    '4F0110': 'assertion/Loading Exception throw -- ends the case',
    '404ED0': 'debug log (B38B70, file, line, function, format, ...)',
    '859030': 'chunk open by name (fixture chunk map; positions the stream at 0)',
    '858F50': 'chunk seek (absolute chunk offset)',
    '4FB0A9': 'class-equal construction+Load path of 4FAE80 (restoration agent): consumes a fixture byte count, *out = marker, resumes at 4FB539 (common exit)',
    '4F9C10': 'KKC_GWorld::CreateGThing (tail section only): records the parameter block, returns a fixture result',
    'ctor': '4F9C10 class constructors 4EA5E0/4DC450/4DCB30/4DB610/4DBEE0/4E4DC0/4E66D0/4DE350/4D6BB0 (dispatch section): emulation stops on entry',
}


class Throw(Exception):
    pass


class Stop(Exception):
    pass


def f32(v):
    return struct.unpack('<I', struct.pack('<f', v))[0]


class Machine:
    def __init__(self, precision='Double53'):
        self.precision = precision
        self.chunks = {}       # name -> bytes
        self.payload = b''
        self.at = 0
        self.reads = []
        self.calls = []
        self.outcome = None
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
        self.u.reg_write(UC_X86_REG_FPCW, CW[precision])
        self.u.hook_add(UC_HOOK_CODE, self._hook)
        null_guard.install(self)
        self._strings()
        self._stream()
        self._standard_seams()
        self.generations = self.alloc(4 + 0x10000 * 2 + 0x10000 * 4, 0)
        self.w(G_IDMGR, self.generations)

    # ---------------------------------------------------------------- memory
    def alloc(self, n, fill=0):
        p = self.next
        self.next += (max(n, 1) + 15) & ~15
        assert self.next < HEAP_END, 'fixture heap exhausted'
        self.u.mem_write(p, bytes([fill]) * max(n, 1))
        return p

    def w(self, a, v):
        self.u.mem_write(a, struct.pack('<I', v & 0xffffffff))

    def r(self, a):
        return struct.unpack('<I', self.u.mem_read(a, 4))[0]

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
        size = max(entries) + 4
        t = self.alloc(size)
        for off, h in entries.items():
            self.w(t + off, h if isinstance(h, int) else self.stub(h))
        return t

    def _hook(self, u, pc, size, _):
        if P.base <= pc < P.base + P.image_size:
            if pc not in VISITED:
                raw = P.range(pc, size)
                assert raw == V.range(pc, size), hex(pc)
                VISITED[pc] = raw.hex()
        h = self.seams.get(pc)
        if h is not None:
            h(self)
        elif STUBS <= pc < STUBS + 0x10000:
            raise RuntimeError(('unhooked stub', hex(pc)))

    def run(self, entry, until):
        """Run from entry until `until`; seams raise Throw/Stop to end a case."""
        try:
            self.u.emu_start(entry, until, count=50_000_000)
        except (Throw, Stop):
            pass
        if self.null_accesses and self.outcome is None:
            self.outcome = {'kind': 'NullDereference', 'pc': self.null_accesses[0][0]}

    def call(self, entry, this, args, pop):
        """thiscall/cdecl call returning to STOP; checks the callee stack pop."""
        self.w(STACK, STOP)
        for i, a in enumerate(args):
            self.w(STACK + 4 + 4 * i, a)
        self.u.reg_write(UC_X86_REG_ESP, STACK)
        self.u.reg_write(UC_X86_REG_ECX, this)
        self.run(entry, STOP)
        if self.outcome is None:
            assert self.u.reg_read(UC_X86_REG_EIP) == STOP, hex(self.u.reg_read(UC_X86_REG_EIP))
            assert self.u.reg_read(UC_X86_REG_ESP) == STACK + 4 + pop, 'callee stack pop'
        return self.u.reg_read(UC_X86_REG_EAX)

    # --------------------------------------------------------------- strings
    def _strings(self):
        self.allocator = self.alloc(16)
        av = self.vtable({0x0: self._alloc_unexpected, 0x4: lambda m: m.ret(4),
                          0xc: lambda m: m.ret(0, m.cstring('') - 16),
                          0x10: lambda m: m.ret(0, m.allocator)})
        self.w(self.allocator, av)

    def _alloc_unexpected(self, m):
        raise RuntimeError('CString allocator +0 reached (buffer growth not seamed)')

    def cstring(self, text):
        raw = text.encode('utf-16-le', 'surrogatepass')
        n = len(raw) // 2
        h = self.alloc(16 + len(raw) + 2)
        self.w(h, self.allocator)
        self.w(h + 4, n)
        self.w(h + 8, n)
        self.w(h + 12, 1000)
        self.u.mem_write(h + 16, raw + b'\0\0')
        return h + 16

    def cstring_units(self, p):
        n = self.r(p - 12)
        raw = bytes(self.u.mem_read(p, 2 * n))
        return list(struct.unpack('<%dH' % n, raw))

    def read_wide(self, p, limit=512):
        out = bytearray()
        while len(out) < limit * 2:
            c = bytes(self.u.mem_read(p + len(out), 2))
            if c == b'\0\0':
                break
            out += c
        return out.decode('utf-16-le', 'surrogatepass')

    # ---------------------------------------------------------------- stream
    def _stream(self):
        """ctx (4FAE80 param_1 / 4FC340 param_1): stream object at +3C whose
        +3C (= ctx+78) is the current chunk descriptor: +58 size, +68 position
        (858F50 compares/sets these; 4FC340 reads +68 before each record)."""
        self.ctx = self.alloc(0x100)
        vt = self.vtable({0x0: self._read, 0x4: self._write_unexpected,
                          0xc: lambda m: m.ret(0, 0)})      # IsWriting: loading
        self.stream = self.ctx + 0x3c
        self.w(self.stream, vt)
        self.desc = self.alloc(0x80)
        self.w(self.ctx + 0x78, self.desc)
        self._sync()

    def set_payload(self, data, at=0):
        self.payload = bytes(data)
        self.at = at
        self._sync()

    def _sync(self):
        self.w(self.desc + 0x58, len(self.payload))
        self.w(self.desc + 0x68, self.at)

    def take(self, n):
        if self.at + n > len(self.payload):
            self.outcome = {'kind': 'ReadPastChunk', 'at': self.at, 'wanted': n}
            raise Throw()
        b = self.payload[self.at:self.at + n]
        self.reads.append([self.at, n])
        self.at += n
        self._sync()
        return b

    def _read(self, m):
        assert self.u.reg_read(UC_X86_REG_ECX) == self.stream
        dst, n = self.arg(0), self.arg(1)
        self.u.mem_write(dst, self.take(n))
        self.ret(8, self.stream)

    def _write_unexpected(self, m):
        raise RuntimeError('stream write during Load')

    # ----------------------------------------------------------------- seams
    def record(self, name, **args):
        entry = {'seam': name, **args}
        self.calls.append(entry)
        return entry

    def throw(self, kind, **info):
        self.outcome = {'kind': kind, **info}
        raise Throw()

    def _standard_seams(self):
        m = self
        m.seam(0x6f7810, lambda m: m.ret(0, m.alloc(m.arg(0), 0xa5)))
        m.seam(0x6f7430, lambda m: m.ret(0))
        m.seam(0x856370, lambda m: m.ret(0, m.allocator))

        def literal_ctor(m):  # 401B00 thiscall
            m.w(m.u.reg_read(UC_X86_REG_ECX), m.cstring(m.read_wide(m.arg(0))))
            m.ret(4, m.u.reg_read(UC_X86_REG_ECX))
        m.seam(0x401b00, literal_ctor)

        def read_string(m):  # 41D290(stream, &CString) cdecl
            stream, dest = m.arg(0), m.arg(1)
            assert stream == m.stream
            n = struct.unpack('<H', m.take(2))[0]
            raw = m.take(2 * n)
            m.w(dest, m.cstring(raw.decode('utf-16-le', 'surrogatepass')))
            m.ret(0, stream)
        m.seam(0x41d290, read_string)

        m.seam(0x41dfb0, lambda m: m.throw('LoadWarningException', format=m.read_wide(m.arg(0))))
        m.seam(0x858ad0, lambda m: m.throw('FileException', format=m.read_wide(m.arg(0))))
        m.seam(0x4f0110, lambda m: m.throw('Assertion', line=m.arg(2), expression=m.read_wide(m.arg(3))))

        def debug_log(m):  # 404ED0(channel, file, line, function, format, ...) cdecl
            m.record('404ED0', line=m.arg(2), format=m.read_wide(m.arg(4)))
            m.ret(0)
        m.seam(0x404ed0, debug_log)

        def localize(m):  # 4B9F40 thiscall(&key CString, fallback) ret 8
            key = m.read_wide(m.r(m.arg(0)))
            assert m.u.reg_read(UC_X86_REG_ECX) == m.r(G_TEXT) and m.arg(1) == G_EMPTY
            m.record('4B9F40', key=key)
            m.w(m.arg(0), m.cstring(key))
            m.ret(8, m.arg(0))
        m.seam(0x4b9f40, localize)
        m.w(G_TEXT, m.alloc(16))

        def map_warning(m):  # 5A1040(ctx, CString text by value, IDS) cdecl
            m.record('5A1040', ctx=m.arg(0), text=m.read_wide(m.arg(1)),
                     ids=m.read_wide(m.arg(2)))
            m.ret(0)
        m.seam(0x5a1040, map_warning)

    # ------------------------------------------------------- definitions etc
    def definition(self, name, klass, vtable='DynamicDoodadData', variations=0, count=0):
        """Definition object: vtable, +8 name, +18 category descriptor (+1C
        runtime class), +108 aspects_as_variations byte, +110 aspect count."""
        d = self.alloc(0x200, 0)
        self.w(d, DEF_VTABLES[vtable])
        self.w(d + 8, self.cstring(name))
        cat = self.alloc(0x40, 0)
        self.w(cat + 0x1c, klass)
        self.w(d + 0x18, cat)
        self.wb(d + 0x108, variations)
        self.w(d + 0x110, count)
        return d

    def set_generation(self, slot, generation):
        self.u.mem_write(self.generations + 4 + 2 * slot, struct.pack('<H', generation))

    def new_vector(self):
        """4FC600 4FCE64..4FCEA8: new[1] of the 0x30 record (count prefix 1,
        4F26F0 run on the element), vector {data, count 0, capacity 1}."""
        block = self.alloc(0x34, 0xa5)
        self.w(block, 1)
        self.call(0x4f26f0, block + 4, [], 0)
        vec = self.alloc(12)
        self.w(vec, block + 4)
        self.w(vec + 4, 0)
        self.w(vec + 8, 1)
        return vec

    def vector_records(self, vec):
        data, count = self.r(vec), self.r(vec + 4)
        return [self.element(data + 0x30 * i) for i in range(count)]

    def element(self, e):
        w = [self.r(e + 4 * i) for i in range(12)]
        return {'ids': self.cstring_units(w[0]), 'definition': w[1], 'saved_id': w[2],
                'name': self.cstring_units(w[3]), 'word_10': w[4], 'position': w[5:8],
                'direction': w[8:11], 'variant': w[11]}
