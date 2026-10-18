"""Shared oracle support for the original SAIData loaders: an emulated ATL
CStringW string manager and seamed native data-node getters that answer from
a recorded answer tree (see crates/aa-mind/examples/dump_sai_inputs.rs).

Answer tree: nodes[id][getter][lower name] = value, getters fixed (80-bit
image {significand, sign_exponent} or {f32}), int, bool, string, localized,
count, item ("name#index" -> child id), section (child id or null); a value
{"error": ...} makes the native getter throw (a fault). Managers:
managers[kind][lower ids] = [handle, index] or null.

Every getter call is recorded as an event {op, node, name, ...} in native
order; the Rust replay must make the same calls.
"""
from pathlib import Path
import struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import Machine
from unicorn import x86_const as X

ROOT_SIZE = 0x1000
# Data managers by their place in the [B36530] root: embedded (root+offset)
# or pointer slots (*(root+offset)).
MANAGERS = {('embedded', 0x5c): 'Nations', ('embedded', 0xa8): 'Factions', ('pointer', 0x56c): 'Egos',
            ('pointer', 0x468): 'Abilities', ('pointer', 0x474): 'Formations', ('embedded', 0x10): 'Resources'}
# Found-object handles at or above this value are emulator addresses: the
# object is mapped AT the handle (the ego loaders store and hash the native
# pointers, so the handle must be the pointer value itself).
ADDRESS_HANDLES = 0x10000


def hash405030(name):
    """405030: h = h*33 + c over the name with a-z folded to upper case."""
    h = 0
    for ch in name:
        c = ord(ch)
        if 0x61 <= c <= 0x7a:
            c -= 0x20
        h = (h * 33 + c) & 0xffffffff
    return h


class DataMachine(Machine):
    def __init__(self, precision, answers):
        super().__init__(precision)
        self.answers = answers
        self.fault = None
        # String manager (IAtlStringMgr) and nil string.
        smvt = self.alloc(0x20)
        for slot, fn in enumerate([self.sm_allocate, self.sm_free, self.sm_reallocate, self.sm_nil, self.sm_clone]):
            self.w(smvt + 4 * slot, self.stub(fn))
        self.smgr = self.alloc(0x10); self.w(self.smgr, smvt)
        self.nil = self.alloc(0x20); self.w(self.nil, self.smgr); self.w(self.nil + 0xc, 0x40000000)
        self.seam(0x856370, lambda m: self.ret(self.smgr))
        self.seam(0x6f7810, self.s_new)
        self.seam(0x6f7430, lambda m: self.ret(0))
        self.seam(0x404f60, lambda m: self.ret(0))
        self.seam(0x404ed0, lambda m: self.ret(0))
        self.seam(0x59aba0, lambda m: self.ret(0))
        self.seam(0x410c40, self.s_data_error)
        self.seam(0x96cad6, self.s_wcsicmp)
        # Data manager root.
        root = self.root = self.alloc(ROOT_SIZE)
        self.w(0xb36530, root)
        self.fake_managers = {}
        for off in range(0, ROOT_SIZE, 4):
            if off == 0x824:
                continue
            fm = self.alloc(0x10); self.w(root + off, fm); self.fake_managers[fm] = off
        self.w(root + 0x824, 0)
        self.objects = {}
        self.mapped_pages = set()
        # Data nodes.
        self.node_ptr = {}
        self.node_id = {}
        for key in answers['nodes']:
            self.node(int(key))
        self.cstr_slots = {}
        for address, fn in {0x5b5130: self.g_fixed, 0x5b4ed0: self.g_int, 0x5b5380: self.g_bool,
                            0x5b54d0: self.g_string, 0x5b5620: self.g_localized, 0x5b4220: self.g_count,
                            0x42be10: self.g_item, 0x5b4410: self.g_section, 0x48d500: self.s_find}.items():
            self.seam(address, fn)

    # Fill byte of operator new (6F7810) allocations; None keeps them zeroed.
    poison = None

    def s_new(self, m):
        n = max(self.arg(1), 4)
        p = self.alloc(n)
        if self.poison is not None:
            self.u.mem_write(p, bytes([self.poison]) * n)
        self.ret(p)

    def at_handle(self, handle, size=0x40):
        """Map (once) zeroed memory for an object whose address is `handle`."""
        page = handle & ~0xfff
        end = (handle + size + 0xfff) & ~0xfff
        for q in range(page, end, 0x1000):
            if q not in self.mapped_pages:
                self.u.mem_map(q, 0x1000)
                self.mapped_pages.add(q)
        return handle

    def enum_map(self, address, entries, buckets=17):
        """Populate a native name->value map (497220 layout: {buckets, count},
        node {entry, next}, entry {value, name}) hashed exactly like 405030
        (ASCII upper-case fold, h = h*33 + c), newest entry first per bucket."""
        table = self.alloc(4 * buckets)
        for value, name in entries:
            entry = self.alloc(0x10); self.w(entry, value & 0xffffffff)
            text = self.alloc(2 * (len(name) + 1))
            self.u.mem_write(text, name.encode('utf-16-le') + bytes(2))
            self.w(entry + 4, text)
            b = hash405030(name) % buckets
            node = self.alloc(8); self.w(node, entry); self.w(node + 4, self.r(table + 4 * b))
            self.w(table + 4 * b, node)
        self.w(address, table); self.w(address + 4, buckets)

    def s_wcsicmp(self, m):
        """CRT _wcsicmp (96CAD6) in the C locale: ASCII letters fold to lower case."""
        a, b = self.wstr(self.arg(1)), self.wstr(self.arg(2))
        fold = lambda t: [ord(ch) + 32 if 'A' <= ch <= 'Z' else ord(ch) for ch in t] + [0]
        x, y = fold(a), fold(b)
        r = 0
        for i in range(min(len(x), len(y))):
            if x[i] != y[i]:
                r = x[i] - y[i]
                break
        self.ret(r & 0xffffffff)

    # strings ------------------------------------------------------------
    def cstring(self, text):
        data = self.alloc(0x10 + 2 * (len(text) + 1))
        self.w(data, self.smgr); self.w(data + 4, len(text)); self.w(data + 8, len(text)); self.w(data + 0xc, 1)
        self.u.mem_write(data + 0x10, text.encode('utf-16-le') + b'\0\0')
        return data + 0x10

    def wstr(self, p):
        out = bytearray()
        while True:
            c = bytes(self.u.mem_read(p, 2))
            if c == b'\0\0':
                return out.decode('utf-16-le')
            out += c; p += 2

    def cstr_text(self, p):
        return self.wstr(p) if p else ''

    def sm_allocate(self, m):
        n = self.arg(1); assert self.arg(2) == 2
        data = self.alloc(0x10 + 2 * (n + 1))
        self.w(data, self.smgr); self.w(data + 4, 0); self.w(data + 8, n); self.w(data + 0xc, 1)
        self.ret(data, 8)

    def sm_free(self, m):
        self.ret(0, 4)

    def sm_reallocate(self, m):
        old, n = self.arg(1), self.arg(2); assert self.arg(3) == 2
        data = self.alloc(0x10 + 2 * (n + 1))
        length = self.r(old + 4)
        self.u.mem_write(data, bytes(self.u.mem_read(old, 0x10 + 2 * (length + 1))))
        self.w(data + 8, n)
        self.ret(data, 12)

    def sm_nil(self, m):
        self.ret(self.nil)

    def sm_clone(self, m):
        self.ret(self.smgr)

    # nodes --------------------------------------------------------------
    def node(self, nid):
        if nid not in self.node_ptr:
            p = self.alloc(0x40); self.w(p + 0x20, 0)
            self.node_ptr[nid] = p; self.node_id[p] = nid
        return self.node_ptr[nid]

    def answer(self, nid, getter, name):
        table = self.answers['nodes'].get(str(nid), {}).get(getter, {})
        key = name.lower()
        if key not in table:
            raise AssertionError('unanswered %s %s on node %s' % (getter, name, nid))
        return table[key]

    def ev(self, op, **kw):
        self.events.append({'op': op, **kw})

    def throw(self, why):
        self.fault = why
        self.u.emu_stop()

    def getter(self, op):
        nid = self.node_id[self.ecx()]
        name = self.wstr(self.arg(1))
        value = self.answer(nid, op, name)
        self.ev(op, node=nid, name=name, result=value)
        if isinstance(value, dict) and 'error' in value:
            self.throw('%s %s: %s' % (op, name, value['error']))
            return nid, name, None
        return nid, name, value

    def g_fixed(self, m):
        nid, name, v = self.getter('fixed')
        if v is None:
            return
        scratch = self.alloc(16)
        self.u.mem_write(scratch, struct.pack('<QH', v['significand'], v['sign_exponent']))
        code = b'\xdb\x2d' + struct.pack('<I', scratch) + b'\xc2' + struct.pack('<H', 4)
        stub = self.alloc(16); self.u.mem_write(stub, code)
        self.u.reg_write(X.UC_X86_REG_EIP, stub)

    def g_int(self, m):
        nid, name, v = self.getter('int')
        if v is not None:
            self.ret(v & 0xffffffff, 4)

    def g_bool(self, m):
        nid, name, v = self.getter('bool')
        if v is not None:
            self.ret(int(v), 4)

    def g_text(self, op):
        nid, name, v = self.getter(op)
        if v is None:
            return
        key = (nid, op, name.lower())
        if key not in self.cstr_slots:
            slot = self.alloc(4); self.w(slot, self.cstring(v)); self.cstr_slots[key] = slot
        self.ret(self.cstr_slots[key], 4)

    def g_string(self, m):
        self.g_text('string')

    def g_localized(self, m):
        self.g_text('localized')

    def g_count(self, m):
        nid, name, v = self.getter('count')
        if v is not None:
            self.ret(v & 0xffffffff, 4)

    def g_item(self, m):
        it = self.ecx()
        nid = self.node_id[self.r(it)]
        index, name, count = self.r(it + 4), self.wstr(self.r(it + 8)), self.r(it + 0xc)
        index -= (index >> 31) << 32
        key = '%s#%d' % (name.lower(), index)
        value = self.answer(nid, 'item', key)
        self.ev('item', node=nid, name=name, index=index, count=count - ((count >> 31) << 32), result=value)
        if isinstance(value, dict):
            self.throw('item %s: %s' % (key, value['error'])); return
        self.ret(self.node(value))

    def g_section(self, m):
        nid = self.node_id[self.ecx()]
        name, required = self.wstr(self.arg(1)), self.arg(2) & 0xff
        value = self.answer(nid, 'section', name)
        self.ev('section', node=nid, name=name, required=required, result=value)
        if isinstance(value, dict):
            self.throw('section %s: %s' % (name, value['error'])); return
        if value is None:
            if required:
                self.throw('SubNode not found. (%s)' % name); return
            self.ret(0, 8); return
        self.ret(self.node(value), 8)

    def manager_kind(self, ecx):
        if ecx in self.fake_managers:
            key = ('pointer', self.fake_managers[ecx])
        else:
            key = ('embedded', ecx - self.root)
        return MANAGERS.get(key, 'Other(%#x)' % key[1])

    def s_find(self, m):
        kind = self.manager_kind(self.ecx())
        ids = self.wstr(self.arg(1))
        table = self.answers.get('managers', {}).get(kind, {})
        if ids.lower() not in table:
            raise AssertionError('unanswered find %s %s' % (kind, ids))
        r = table[ids.lower()]
        self.ev('find', manager=kind, ids=ids, result=r)
        if r is None:
            self.ret(0, 4); return
        handle, index = r
        key = (kind, handle)
        if key not in self.objects:
            p = self.at_handle(handle) if handle >= ADDRESS_HANDLES else self.alloc(0x40)
            self.w(p + 0xc, index); self.objects[key] = p
            self.found_object(kind, handle, p)
        self.ret(self.objects[key], 4)

    def found_object(self, kind, handle, p):
        """Hook for subclasses to fill more fields of a found object."""

    def object_handle(self, p):
        for (kind, handle), q in self.objects.items():
            if q == p:
                return handle
        raise AssertionError('unknown object %#x' % p)

    def s_data_error(self, m):
        """410C40(format, ...): records the format and its %s arguments (other
        conversions are skipped: one stack slot, two for doubles), then throws."""
        fmt = self.wstr(self.arg(1))
        args = []
        k = 2
        i = 0
        while i < len(fmt):
            if fmt[i] == '%' and i + 1 < len(fmt):
                spec = fmt[i + 1]
                if spec == 's':
                    args.append(self.wstr(self.arg(k)))
                    k += 1
                elif spec in 'gfe':
                    k += 2
                elif spec != '%':
                    k += 1
                i += 2
                continue
            i += 1
        self.ev('data_error', format=fmt, args=args)
        self.throw('data error: ' + fmt)

    def pending_refs(self):
        """Pending DataRefs ([B36530]+824 list, newest first) as dicts."""
        out, p = [], self.r(self.root + 0x824)
        while p:
            out.append({'address': p, 'vtable': self.r(p), 'ids': self.cstr_text(self.r(p + 4))})
            p = self.r(p + 8)
        return out

    def run_call(self, entry, ecx, args, pop, count=2_000_000):
        from unicorn import UcError
        try:
            self.call(entry, ecx=ecx, args=args, pop=pop, count=count)
        except (AssertionError, UcError) as e:
            if self.fault:
                return self.fault
            if self.null_accesses or isinstance(e, UcError):
                return 'null dereference'
            raise
        return None
