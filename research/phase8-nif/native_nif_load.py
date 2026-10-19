"""Native NiStream load oracle: run the ORIGINAL AA.exe NIF stream reader.

The original image is mapped unchanged. The loader registry is populated by
the native static initializer 881600 (map + post-process list) and the native
RegisterLoader 880490 for each (name, create) pair pushed by the original
registration code (read from the instructions, see registry()). A native
NiStream is built by its constructor 8816B0 and the file bytes are loaded by
NiStream::Load(char*,int) 87FEF0, which wraps them in the native NiMemStream
(vtable ABF284) and calls LoadStream 881BD0.

Explicit seams (never counted as executed original code):
  6F7810 game allocator  -> bump allocation in a private heap
  6F7430 game free       -> recorded, memory not reused
Any execution of the CRT heap (_malloc/_free/_realloc/_calloc/_aligned_malloc)
or of an unmapped import aborts the run as an unsupported native path.

Observations: every NiMemStream::Read 8D0BD0 call (absolute stream offset,
requested length, delivered length, and the loader's current object index
NiStream+0x1A0 / link index +0x1A4 / post-link index +0x1A8), the created
object classes (vtable -> registered class), the top-level object list
(NiStream+0x148/+0x150), LoadStream's return value and the native error code
(B44168) and message (B44068).
"""
from pathlib import Path
import struct, sys, json, hashlib

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
GAME = ROOT.parent
sys.path.insert(0, str(GAME / 'reconstruction/tools/python-packages'))
import capstone
from unicorn import (Uc, UC_ARCH_X86, UC_MODE_32, UC_HOOK_CODE, UC_HOOK_BLOCK, UC_HOOK_MEM_READ,
                     UC_HOOK_MEM_WRITE, UC_HOOK_MEM_UNMAPPED, UcError)
from unicorn.x86_const import (UC_X86_REG_EAX, UC_X86_REG_EBX, UC_X86_REG_ECX, UC_X86_REG_EDX,
                               UC_X86_REG_ESI, UC_X86_REG_EDI, UC_X86_REG_EBP, UC_X86_REG_ESP,
                               UC_X86_REG_EIP, UC_X86_REG_FPCW)

sha = lambda b: hashlib.sha256(b).hexdigest()


class PE:
    def __init__(self, path):
        self.b = Path(path).read_bytes()
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
            if r <= rva < r + max(raw, vs):
                if rva - r + size > raw:
                    return None
                return self.b[off + rva - r:off + rva - r + size]
        return None


ORIGINAL = PE(GAME / 'AA.exe')
V14 = PE(GAME / 'reconstruction/native-game/runtime-expanded-v14/AA.exe')
assert sha(ORIGINAL.b) == '5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26'
assert sha(V14.b) == '3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1'

HEAP, HEAP_SIZE = 0x20000000, 0x10000000
STACK_BASE, STACK_SIZE = 0x1e000000, 0x00100000
STACK = STACK_BASE + 0xf0000
STOP = STACK_BASE + 0xff000
CALLEE_SAVED = [UC_X86_REG_EBX, UC_X86_REG_ESI, UC_X86_REG_EDI, UC_X86_REG_EBP]

ALLOC, FREE = 0x6f7810, 0x6f7430
FORBIDDEN = {0x96d331: '_free', 0x96e19f: '_malloc', 0x97117c: '_realloc', 0x97184d: '_aligned_malloc',
             0x97259a: '_calloc', 0x6f7650: 'game heap body', 0x6f7700: 'game pool body'}
REGISTER, STATIC_INIT = 0x880490, 0x881600
GETPTD, MBCINFO = 0x9738a9, 0xb4b370
RTTI_CTOR = 0x872e40
NISTREAM_CTOR, NISTREAM_LOAD_BUFFER = 0x8816b0, 0x87fef0
MEMSTREAM_READ = 0x8d0bd0
LOAD_CLEANUP = 0x880e60
POP_ID, POP_COUNT = 0x8803a0, 0x8803d0
LOAD_TOP = 0x8812f0
REFOBJECT_VTABLE = 0xab6928  # written by the NiRefObject destructor 8815C0
ERROR_CODE, ERROR_TEXT = 0xb44168, 0xb44068
# C++ static initializers that fill the NiAnimationKey function tables
# (B45898..B45E00); each is guarded by its own done flag.
KEY_TABLE_INITS = [0x860aa0, 0x860f10, 0x861580, 0x862040, 0x862680, 0x862b30, 0x862ed0, 0x8635a0,
                   0x863920, 0x863c30, 0x864020, 0x864350, 0x864f70, 0x8656c0, 0x865c10]
SDM_PREINIT_REGISTER, SDM_REGISTER, NIINIT = 0x8601a0, 0x8601c0, 0x860120
SDM_PREINIT = (0x8c3ed0, 0x8c4380)  # NiMain: loader map + NiMain classes
# (init, shutdown) pairs registered by the original static initializers. The
# DX9 renderer entry (8D7C50/8D7CB0) is not part of stream loading and is not run.
SDM_ENTRIES = [(0x865ca0, 0x865ee0), (0x89ab50, 0x89ac00), (0x89aca0, 0x89aee0), (0x89b090, 0x89b0b0)]
# Registration call sites of RegisterLoader in original order (three init blocks).
REGISTRATION_BLOCKS = [(0x865c10, 0x865e70), (0x89a950, 0x89aed0), (0x8c3cc0, 0x8c4360)]


IMPORT_STUBS = 0x1d000000


def import_slots(pe):
    b = pe.b
    o = struct.unpack_from('<I', b, 0x3c)[0] + 24
    imp_rva = struct.unpack_from('<I', b, o + 104)[0]

    def off(r):
        for vs, va, raw, fo in pe.sections:
            if va <= r < va + max(vs, raw):
                return fo + r - va
    p, names = off(imp_rva), {}
    while True:
        oft, ts, fc, name, ft = struct.unpack_from('<IIIII', b, p)
        if name == 0:
            return names
        dll = b[off(name):].split(b'\0')[0].decode()
        q, slot = off(oft or ft), pe.base + ft
        while True:
            e = struct.unpack_from('<I', b, q)[0]
            if e == 0:
                break
            nm = ('#%d' % (e & 0xffff)) if e & 0x80000000 else b[off(e) + 2:].split(b'\0')[0].decode()
            names[slot] = dll + '!' + nm
            q, slot = q + 4, slot + 4
        p += 20


IMPORT_MODELS = {
    'KERNEL32.dll!GetCurrentThreadId': lambda m: (1, 0),
    'KERNEL32.dll!EnterCriticalSection': lambda m: (0, 4),
    'KERNEL32.dll!LeaveCriticalSection': lambda m: (0, 4),
    'KERNEL32.dll!InitializeCriticalSection': lambda m: (0, 4),
    'KERNEL32.dll!DeleteCriticalSection': lambda m: (0, 4),
}


class Abort(Exception):
    pass


def rtti_initializers(pe):
    """C++ initializer-table (B21000..B27964) entries that only construct a
    static NiRTTI (push base; push name; mov ecx, rtti; call 872E40; ret)."""
    md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)
    table = struct.unpack_from('<%dI' % ((0xb27964 - 0xb21000) // 4), pe.range(0xb21000, 0xb27964 - 0xb21000))
    out = []
    for f in table:
        code = pe.range(f, 32) if 0x401000 <= f < 0x9f0000 else None
        if not code:
            continue
        seq = []
        for ins in md.disasm(code, f):
            seq.append(ins)
            if ins.mnemonic == 'ret':
                break
        shape = [i.mnemonic for i in seq]
        if (shape == ['push', 'push', 'mov', 'call', 'ret'] and seq[2].op_str.startswith('ecx, 0x')
                and seq[3].op_str == hex(RTTI_CTOR)):
            out.append(f)
    return out


def registry():
    """(name, create) pairs in the original call order, read from the push
    operands that immediately precede every `call 880490`."""
    md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)
    pairs = []
    for start, end in REGISTRATION_BLOCKS:
        code = ORIGINAL.range(start, end - start)
        sites = []
        for i in range(len(code) - 4):
            if code[i] == 0xe8 and start + i + 5 + struct.unpack_from('<i', code, i + 1)[0] == REGISTER:
                sites.append(start + i)
        for s in sites:
            found = None
            for back in range(5, 80):
                seq = list(md.disasm(ORIGINAL.range(s - back, back), s - back))
                if not seq or seq[-1].address + seq[-1].size != s or sum(x.size for x in seq) != back:
                    continue
                pushes = [int(x.op_str, 16) for x in seq if x.mnemonic == 'push' and x.op_str.startswith('0x')]
                if len(pushes) >= 2:
                    name = ORIGINAL.range(pushes[-1], 64)
                    if name and name.startswith(b'Ni') and 0x401000 <= pushes[-2] < 0x9e0000:
                        found = (name.split(b'\0')[0].decode(), pushes[-2])
                        break
            assert found, hex(s)
            pairs.append(found)
    assert len(pairs) == 143 and len({n for n, _ in pairs}) == 143
    return pairs


class NifMachine:
    def __init__(self, image=ORIGINAL):
        self.image = image
        u = self.u = Uc(UC_ARCH_X86, UC_MODE_32)
        u.mem_map(image.base, (image.image_size + 0xfff) & ~0xfff)
        for vs, rva, raw, off in image.sections:
            u.mem_write(image.base + rva, image.b[off:off + raw])
        u.mem_map(HEAP, HEAP_SIZE)
        u.mem_map(STACK_BASE, STACK_SIZE)
        u.mem_map(0, 0x1000)
        self.heap = HEAP
        # x87 control word for every call: PC53/RC nearest/all masked unless
        # a caller selects another precision (0x007F PC24, 0x037F PC64).
        self.fpcw = 0x027f
        self.blocks = {}
        self.allocations = []
        self.frees = []
        self.reads = []
        self.null_accesses = []
        self.stream = None
        self.unsupported = None
        seams = {ALLOC: self._alloc, FREE: self._free, GETPTD: self._getptd}
        self.ptd = None
        for a, name in FORBIDDEN.items():
            seams[a] = (lambda n: (lambda: self._abort('native ' + n)))(name)
        self.seams = seams
        for a in seams:
            u.hook_add(UC_HOOK_CODE, self._seam, begin=a, end=a)
        u.hook_add(UC_HOOK_CODE, self._read, begin=MEMSTREAM_READ, end=MEMSTREAM_READ)
        # LoadStream 881BD0 calls 880E60 on success and failure, before which
        # the object array (+0x130, count +0x134) still holds every object.
        u.hook_add(UC_HOOK_CODE, self._snapshot, begin=LOAD_CLEANUP, end=LOAD_CLEANUP)
        self.snapshot = None
        self.on_snapshot = None
        # LoadTopLevelObjects 8812F0 runs once every object's LoadBinary is
        # done and before any LinkObject or post-processing.
        u.hook_add(UC_HOOK_CODE, self._loaded, begin=LOAD_TOP, end=LOAD_TOP)
        self.loaded = None
        self.link_ids = self.link_counts = self.link_consumed = None
        # LinkObject consumers: GetObjectFromLinkID 8803A0, GetNumberOfLinkIDs 8803D0.
        self.pops = []
        u.hook_add(UC_HOOK_CODE, self._pop_id, begin=POP_ID, end=POP_ID)
        u.hook_add(UC_HOOK_CODE, self._pop_count, begin=POP_COUNT, end=POP_COUNT)
        u.hook_add(UC_HOOK_BLOCK, self._block)
        u.hook_add(UC_HOOK_MEM_READ | UC_HOOK_MEM_WRITE, self._null, begin=0, end=0xfff)
        u.hook_add(UC_HOOK_MEM_UNMAPPED, self._unmapped)
        # Every import slot points at its own stub. Only the listed imports
        # are modelled (thread identity and uncontended critical sections);
        # any other import aborts as an unsupported native path.
        u.mem_map(IMPORT_STUBS, 0x10000)
        u.mem_write(IMPORT_STUBS, bytes([0xcc]) * 0x10000)
        self.imports = {}
        for k, (slot, name) in enumerate(sorted(import_slots(image).items())):
            stub = IMPORT_STUBS + 16 * k
            self.imports[stub] = name
            self.w(slot, stub)
        u.hook_add(UC_HOOK_CODE, self._import, begin=IMPORT_STUBS, end=IMPORT_STUBS + 0xffff)
        self.import_calls = {}

    def _import(self, u, pc, size, _):
        name = self.imports.get(pc)
        model = IMPORT_MODELS.get(name)
        self.import_calls[name] = self.import_calls.get(name, 0) + 1
        if model is None:
            self._abort('unsupported import %s from %#x' % (name, self.r(u.reg_read(UC_X86_REG_ESP))))
            return
        value, pop = model(self)
        self.ret(value, pop)

    # memory ---------------------------------------------------------------
    def w(self, a, v):
        self.u.mem_write(a, struct.pack('<I', v & 0xffffffff))

    def r(self, a):
        return struct.unpack('<I', self.u.mem_read(a, 4))[0]

    def r16(self, a):
        return struct.unpack('<H', self.u.mem_read(a, 2))[0]

    def cstr(self, a, n=256):
        return bytes(self.u.mem_read(a, n)).split(b'\0')[0]

    def bump(self, size, align=16):
        self.heap = (self.heap + align - 1) & ~(align - 1)
        p = self.heap
        self.heap += max(size, 4)
        if self.heap >= HEAP + HEAP_SIZE:
            raise Abort('oracle heap exhausted')
        return p

    # hooks -----------------------------------------------------------------
    def _abort(self, why):
        self.unsupported = why
        self.u.emu_stop()

    def arg(self, i):
        return self.r(self.u.reg_read(UC_X86_REG_ESP) + 4 * i)

    def ret(self, value=0, pop=0):
        s = self.u.reg_read(UC_X86_REG_ESP)
        self.u.reg_write(UC_X86_REG_EAX, value & 0xffffffff)
        self.u.reg_write(UC_X86_REG_EIP, self.r(s))
        self.u.reg_write(UC_X86_REG_ESP, s + 4 + pop)

    def _seam(self, u, pc, size, _):
        self.seams[pc]()

    def _alloc(self):
        size = self.arg(1)
        p = self.bump(size)
        self.u.mem_write(p, b'\xcd' * max(size, 4))  # uninitialised fill, never zero by accident
        self.allocations.append((p, size, self.phase()))
        self.ret(p)

    def _getptd(self):
        # CRT per-thread data on its first use, exactly as 9738A9 builds it
        # (calloc 0x8C, +0x54 initaddr B35D90, +0x14 holdrand 1, +4 handle -1,
        # +0 thread id). The CRT locale state it reaches is the process ANSI
        # single-byte code page (ismbcodepage 0) that CRT startup selects on
        # the reference machine; every string it is used for is asserted ASCII.
        if self.ptd is None:
            ptd = self.bump(0x8c)
            self.u.mem_write(ptd, bytes(0x8c))
            self.w(ptd + 0x54, 0xb35d90)
            self.w(ptd + 0x14, 1)
            self.w(ptd + 4, 0xffffffff)
            self.w(ptd, 1)
            mbc = self.bump(0x220)
            self.u.mem_write(mbc, bytes(0x220))
            self.w(mbc, 1)
            self.w(mbc + 4, 1252)
            self.w(ptd + 0x60, mbc)
            self.w(MBCINFO, mbc)
            self.ptd = ptd
        self.ret(self.ptd)

    def _free(self):
        self.frees.append(self.arg(1))
        self.ret(0)

    def phase(self):
        s = self.stream
        if s is None:
            return None
        return (self.r(s + 0x1a0), self.r(s + 0x1a4), self.r(s + 0x1a8))

    def lists(self):
        """Current lengths of the stream's link-ID list (+0x16C) and its
        link-count list (+0x17C); pushes happen right after the read of the
        value, so a read entry sees every push of the reads before it."""
        s = self.stream
        return (self.r(s + 0x16c), self.r(s + 0x17c))

    def _pop_id(self, u, pc, size, _):
        s = u.reg_read(UC_X86_REG_ECX)
        if s == self.stream:
            i = self.r(s + 0x170)
            self.pops.append((self.r(s + 0x1a4), 'id', i, self.r(self.r(s + 0x164) + 4 * i)))

    def _pop_count(self, u, pc, size, _):
        s = u.reg_read(UC_X86_REG_ECX)
        if s == self.stream:
            i = self.r(s + 0x180)
            self.pops.append((self.r(s + 0x1a4), 'count', i, self.r(self.r(s + 0x174) + 4 * i)))

    def _loaded(self, u, pc, size, _):
        s = self.stream
        if s is not None and self.loaded is None and u.reg_read(UC_X86_REG_ECX) == s:
            array, count = self.r(s + 0x130), self.r(s + 0x134)
            self.loaded = [(p, self.r(p) if p else 0) for p in (self.r(array + 4 * i) for i in range(count))]

    def _snapshot(self, u, pc, size, _):
        s = self.stream
        if s is not None and self.snapshot is None and u.reg_read(UC_X86_REG_ECX) == s:
            array, count = self.r(s + 0x130), self.r(s + 0x134)
            self.snapshot = [self.r(array + 4 * i) for i in range(count)]
            ids, counts = self.r(s + 0x164), self.r(s + 0x174)
            self.link_ids = [self.r(ids + 4 * i) for i in range(self.r(s + 0x16c))]
            self.link_counts = [self.r(counts + 4 * i) for i in range(self.r(s + 0x17c))]
            self.link_consumed = (self.r(s + 0x170), self.r(s + 0x180))
            if self.on_snapshot is not None:
                # Every object is alive and linked here (after LinkObject,
                # PostLinkObject and post-processing, before 880E60 releases
                # the stream's references).
                self.on_snapshot(self)

    def _read(self, u, pc, size, _):
        mem = u.reg_read(UC_X86_REG_ECX)
        dest, want = self.arg(1), self.arg(2)
        pos, end = self.r(mem + 8), self.r(mem + 0xc)
        self.reads.append((pos, want, min(want, end - pos), self.r(self.stream + 0x1a0)) + self.lists())

    def _block(self, u, address, size, _):
        if self.image.base <= address < self.image.base + self.image.image_size:
            old = self.blocks.get(address)
            if old is None or old < size:
                self.blocks[address] = size

    def _null(self, u, access, address, size, value, _):
        pc = u.reg_read(UC_X86_REG_EIP)
        code = bytes(u.mem_read(pc, 4))
        fs = False
        for byte in code:
            if byte == 0x64:
                fs = True
                break
            if byte not in (0x26, 0x2e, 0x36, 0x3e, 0x65, 0x66, 0x67, 0xf0, 0xf2, 0xf3):
                break
        if address + size > 4 or not fs:
            self.null_accesses.append((pc, address, size))
            u.emu_stop()

    def _unmapped(self, u, access, address, size, value, _):
        self.unsupported = 'unmapped access %#x at eip %#x' % (address, u.reg_read(UC_X86_REG_EIP))
        return False

    # calls -----------------------------------------------------------------
    def call(self, entry, ecx=0, args=(), count=500_000_000):
        u = self.u
        u.reg_write(UC_X86_REG_FPCW, self.fpcw)
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
            if self.unsupported is None:
                self.unsupported = 'emulation error %s at eip %#x' % (e, u.reg_read(UC_X86_REG_EIP))
        if self.null_accesses:
            raise Abort('null access %s' % [tuple(map(hex, a)) for a in self.null_accesses])
        if self.unsupported:
            raise Abort(self.unsupported)
        if u.reg_read(UC_X86_REG_EIP) != STOP:
            raise Abort('did not return (eip %#x)' % u.reg_read(UC_X86_REG_EIP))
        return u.reg_read(UC_X86_REG_EAX)

    # loader ----------------------------------------------------------------
    def native_init(self):
        """The Gamebryo start-up the loader depends on, all native code:
        the C++ static initializers that fill the animation-key function
        tables, then NiInit 860120 over the static-data-manager table built
        by the native registration helpers 8601A0/8601C0 (pre-init 8C3ED0
        creates the loader map 881600 and registers NiMain classes)."""
        for f in rtti_initializers(self.image):
            self.call(f)
        for f in KEY_TABLE_INITS:
            self.call(f)
        self.call(SDM_PREINIT_REGISTER, args=SDM_PREINIT)
        for init, shutdown in SDM_ENTRIES:
            self.call(SDM_REGISTER, args=(init, shutdown))
        self.call(NIINIT)

    def rtti_name(self, obj):
        """NiObject::GetRTTI (vtable slot 1) -> NiRTTI, whose first word is its name."""
        vt = self.r(obj)
        rtti = self.call(self.r(vt + 4), ecx=obj)
        name = self.cstr(self.r(rtti)) if self.r(rtti) else b''
        if not name or not name.isascii():
            return '?vtable=%#x rtti=%#x' % (vt, rtti)
        return name.decode()

    def class_vtables(self, pairs):
        """Create one object per registered create function to learn its vtable."""
        out = {}
        for name, create in pairs:
            obj = self.call(create)
            out.setdefault(self.r(obj), []).append(name)
        return out


def class_vtables(pairs, image=ORIGINAL):
    m = NifMachine(image)
    m.native_init()
    return m.class_vtables(pairs)


def load(data, pairs, vtables=None, image=ORIGINAL, on_snapshot=None):
    if vtables is None:
        vtables = class_vtables(pairs, image)
    m = NifMachine(image)
    m.native_init()
    m.on_snapshot = on_snapshot
    stream = m.bump(0x200)
    m.call(NISTREAM_CTOR, ecx=stream)
    m.stream = stream
    buf = m.bump(len(data) + 16)
    m.u.mem_write(buf, data)
    m.reads.clear()
    m.pops.clear()
    m.allocations.clear()
    ok = m.call(NISTREAM_LOAD_BUFFER, ecx=stream, args=(buf, len(data))) & 0xff
    m.stream = None
    objects = []
    destroyed = []
    vt_names = {}
    for i, (p, vt) in enumerate(m.loaded or []):
        if p == 0:
            objects.append(None)
            continue
        if vt not in vt_names:
            now = m.r(p)
            m.w(p, vt)  # name the class by its vtable as created (RTTI via slot 1)
            vt_names[vt] = m.rtti_name(p)
            m.w(p, now)
        objects.append((p, [vt_names[vt]]))
        if m.r(p) == REFOBJECT_VTABLE:
            destroyed.append(i)
    if m.snapshot is not None and [o[0] if o else 0 for o in objects] != list(m.snapshot):
        raise Abort('object array changed between load and cleanup')
    index = {p: i for i, o in enumerate(objects) if o for p in [o[0]]}
    roots = []
    for i in range(m.r(stream + 0x150)):
        p = m.r(m.r(stream + 0x148) + 4 * i)
        roots.append(None if p == 0 else index.get(p, 'foreign %#x' % p))
    return dict(ok=ok, error_code=m.r(ERROR_CODE), error=m.cstr(ERROR_TEXT).decode('latin-1'),
                version=m.r(stream + 0x1c), user_version=m.r(stream + 0x20),
                classes=[None if o is None else o[1][0] for o in objects], roots=roots, destroyed=destroyed,
                reads=m.reads, pops=m.pops, link_ids=m.link_ids, link_counts=m.link_counts,
                link_consumed=m.link_consumed,
                allocations=len(m.allocations), blocks=m.blocks, machine=m, objects=objects)


def trace_digest(reads):
    """SHA-256 over little-endian u32 sextets (offset, requested, delivered,
    load object index, link-ID list length, link-count list length)."""
    h = hashlib.sha256()
    for row in reads:
        h.update(struct.pack('<6I', *row))
    return h.hexdigest()


def words_digest(words):
    h = hashlib.sha256()
    for w in words:
        h.update(struct.pack('<I', w & 0xffffffff))
    return h.hexdigest()


if __name__ == '__main__':
    pairs = registry()
    path = Path(sys.argv[1])
    data = path.read_bytes()
    out = load(data, pairs)
    print(json.dumps({k: v for k, v in out.items() if k not in ('reads', 'pops', 'link_ids', 'link_counts', 'blocks', 'machine', 'objects')}))
    print('reads', len(out['reads']), 'bytes consumed', sum(r[2] for r in out['reads']), 'of', len(data))
    print('link ids', out['link_ids'], 'counts', out['link_counts'], 'consumed', out['link_consumed'], 'pops', len(out['pops']))
    for r in out['reads'][:60]:
        print(r)
