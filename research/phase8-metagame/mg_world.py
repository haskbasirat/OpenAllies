"""Synthetic native metagame objects laid out at their native offsets in a
Unicorn Machine (mg_machine.py), shared by the phase8 oracles.

Only layouts read from the original code are used (offsets cited in the
README / inventory). Engine-side objects (kingdom actors, army actors, their
virtual calls) are explicit seams with recorded calls. Nothing here claims
native behaviour; it only builds inputs and reads outputs.
"""
import struct
from unicorn.x86_const import UC_X86_REG_ECX, UC_X86_REG_EIP, UC_X86_REG_ESP

VT_ARMY = 0x2f80000
VT_KINGDOM = 0x2f81000
VT_REF = 0x2f83000
SEAM_BASE = 0x2f82000
H_GEN0 = 0x4100


class NativeWorld:
    """Handle table, actor vtables with seamed virtuals, string helpers."""

    def __init__(self, m, relations=None):
        self.m = m
        self.relations = relations or {}
        self.calls = []                      # (slot, this, args) of seamed virtuals
        self.kind = {}                       # pointer -> ('army'|'kingdom_actor', index)
        self.army_owner = {}                 # army actor -> kingdom actor pointer
        m.u.mem_write(SEAM_BASE, b'\xc3' * 0x400)
        for vt in (VT_ARMY, VT_KINGDOM, VT_REF):
            for off in range(0, 0x400, 4):
                m.w32(vt + off, SEAM_BASE + 0x200 + off)   # unmapped slots trap below
        self.table = m.alloc(0x20004 + 4 * 0x1000)
        m.w32(0xb364c0, self.table)
        self.next_handle = 1
        for off in range(0, 0x400, 4):
            m.seams[SEAM_BASE + 0x200 + off] = self._unexpected(off)
        self.virtual(VT_ARMY, 0x108, self._owner)
        self.virtual(VT_KINGDOM, 0x140, lambda mm: self._relation(mm, 'enemy_140'))
        self.virtual(VT_KINGDOM, 0x138, lambda mm: self._relation(mm, 'relation_138'))

    def _unexpected(self, off):
        def seam(mm):
            raise RuntimeError(f'unseamed virtual slot +{off:#x} on {mm.u.reg_read(UC_X86_REG_ECX):#x}')
        return seam

    def virtual(self, vt, slot, fn):
        target = SEAM_BASE + slot
        self.m.w32(vt + slot, target)
        self.m.seams[target] = fn

    # ---- handles
    def handle(self, ptr):
        idx = self.next_handle
        self.next_handle += 1
        gen = H_GEN0 + idx
        self.m.w16(self.table + 4 + idx * 2, gen)
        self.m.w32(self.table + 0x20004 + idx * 4, ptr)
        return (gen << 16) | idx

    def resolve(self, h):
        idx = h & 0xffff
        if struct.unpack('<H', self.m.u.mem_read(self.table + 4 + idx * 2, 2))[0] != h >> 16:
            return 0
        return self.m.r32(self.table + 0x20004 + idx * 4)

    # ---- vectors {ptr, count, capacity}
    def vector(self, at, values, capacity=None):
        cap = max(capacity or len(values), len(values), 1)
        p = self.m.alloc(4 * cap)
        for i, v in enumerate(values):
            self.m.w32(p + 4 * i, v)
        self.m.w32(at, p)
        self.m.w32(at + 4, len(values))
        self.m.w32(at + 8, cap)
        return p

    def read_vector(self, at):
        p, n = self.m.r32(at), self.m.r32(at + 4)
        return [self.m.r32(p + 4 * i) for i in range(n)]

    # ---- actors
    def kingdom_actor(self, index):
        a = self.m.alloc(0x400)          # engine object: fields up to +0x284 (army list) are read
        self.m.w32(a, VT_KINGDOM)
        self.kind[a] = ('kingdom_actor', index)
        return a

    def _owner(self, mm):
        this = mm.u.reg_read(UC_X86_REG_ECX)
        self.calls.append((0x108, this, ()))
        mm.ret(0, self.army_owner[this])

    def _relation(self, mm, name):
        this = mm.u.reg_read(UC_X86_REG_ECX)
        other = mm.arg(0)
        self.calls.append((name, this, (other,)))
        a, b = self.kind[this], self.kind.get(other)
        assert a[0] == 'kingdom_actor' and b is not None and b[0] == 'kingdom_actor', (a, b)
        mm.ret(4, int(self.relations[name][a[1]][b[1]]))

    # ---- native ref-counted UTF-16 strings (CString-like: header 16 bytes before data)
    def wstring(self, text):
        raw = text.encode('utf-16le')
        h = self.m.alloc(16 + len(raw) + 2)
        self.m.w32(h, 0)                 # allocator object (unused by readers here)
        self.m.w32(h + 4, len(text))
        self.m.w32(h + 8, len(text))
        self.m.w32(h + 12, 1)
        self.m.u.mem_write(h + 16, raw + b'\0\0')
        return h + 16


def install_crt(m):
    """Seam the CRT heap only: malloc 96E19F / free 96D331 (cdecl) and the
    engine operator new 6F7810 / delete 6F7430. The wide-string allocator at
    B31E50 (vtable AB5834, byte +1C = 0 -> CRT heap) then runs natively."""
    def malloc(mm):
        mm.ret(0, mm.alloc(max(mm.arg(0), 4)))
    def free(mm):
        mm.ret(0)
    for va in (0x96e19f, 0x6f7810):
        m.seams[va] = malloc
    for va in (0x96d331, 0x6f7430):
        m.seams[va] = free
    m.seams[0x404f60] = lambda mm: mm.ret(0)   # debug log (variadic, cdecl)


def native_wstring(m, text):
    """A string header exactly as allocator B31E50 slot 0 builds it."""
    raw = text.encode('utf-16le')
    h = m.alloc(16 + len(raw) + 2)
    m.w32(h, 0xb31e50)
    m.w32(h + 4, len(text))
    m.w32(h + 8, len(text))
    m.w32(h + 12, 1)
    m.u.mem_write(h + 16, raw + b'\0\0')
    return h + 16


def read_wstring(m, p):
    if p == 0:
        return None
    n = m.r32(p - 12)
    return bytes(m.u.mem_read(p, 2 * n)).decode('utf-16le')


def install_static_tables(m):
    """Run the original static initializers the metagame code depends on:
    cos/sin lookup tables 9C8620/9C8640 (objects B39A48/B39A60, 4096 entries,
    published in B39B08/B39B10) and the actor_spacing_type enum table 959B20
    (object B4A524). atexit 96CAB0 is a seam."""
    m.seams[0x96cab0] = lambda mm: mm.ret(0, 0)
    m.call(0x9c8620)
    m.call(0x9c8640)
    m.call(0x959b20, ecx=0xb4a524)


def install_wcsicmp(m):
    """CRT __wcsicmp 96CAD6 (cdecl) as a seam: the original depends on CRT
    locale state not initialised in the emulator. C locale semantics: fold
    ASCII A-Z only, compare UTF-16 code units, return the sign difference."""
    def read(p):
        out = []
        while True:
            ch = int.from_bytes(m.u.mem_read(p, 2), 'little')
            if ch == 0:
                return out
            out.append(ch + 0x20 if 0x41 <= ch <= 0x5a else ch)
            p += 2
    def seam(mm):
        a, b = read(mm.arg(0)), read(mm.arg(1))
        for x, y in zip(a + [0], b + [0]):
            if x != y:
                mm.ret(0, (x - y) & 0xffffffff)
                return
        mm.ret(0, 0)
    m.seams[0x96cad6] = seam
