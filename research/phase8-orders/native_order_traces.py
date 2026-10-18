"""Execute every original order handler's wire decoding (and request->decree
re-encoding) under Unicorn and record each primitive bitstream operation.

No native process and no modified executable. The input stream starts empty;
whenever the ORIGINAL reader 0x40D5D0 is entered on it, this harness chooses a
value and places its bits at the reader's current position using the packing
already proven against 0x40D420 (Phase3/5). The original reader then decodes
those bits; the value it actually produced is recorded and must equal the
chosen value. Requests run through the original generic validation and the
original writer into the admin decree stream (admin+0x1F8); every 0x40D420 call
on that stream is recorded. Emulation stops at the handler's first own call to
0x4012C0 (decree-type logging), i.e. before the gameplay/lobby process call.

Seams (declared): allocator/free/CString/logging from Phase3/4 machines;
definition managers are synthetic with unique counts so each registry read is
identified by its count; thing registry 0xB364C0 maps every slot to one fake
thing whose type-check virtuals return true; session player list is empty;
mirror flag 0x766C50 returns false (no recording stream).
"""
from pathlib import Path
import bisect, json, random, re, struct, sys
HERE = Path(__file__).resolve().parent; ROOT = HERE.parents[1]; GAME = ROOT.parent
sys.path.insert(0, str(ROOT / 'research/phase4-orders'))
from recover_orders import OrdersMachine, PE0, PE1, sha, registry, FUNCTIONS
from unicorn import UcError
sys.path.insert(0, str(ROOT / 'research/phase8-net'))
import null_guard
from unicorn.x86_const import *

SRC = (ROOT / 'crates/aa-orders/src/native_registry.rs').read_text()
DESCRIPTORS = [(int(i), n, int(h, 16)) for i, n, h in re.findall(r'id: (\d+), name: "([^"]+)", handler_va: 0x([0-9a-f]+)', SRC)]
assert len(DESCRIPTORS) == 358

PSEUDO = GAME / 'reconstruction/research/native-aa/pseudocode'
def manager_globals():
    found = set()
    for p in PSEUDO.glob('*.c'):
        t = p.read_text(errors='replace')
        if '+ 0x28) + 1' not in t: continue
        for g in re.findall(r'\(DAT_00([0-9a-f]{6}) \+ 0x28\) \+ 1', t): found.add(int(g, 16))
    return sorted(found)

STARTS = sorted(set(FUNCTIONS) | {h for _, _, h in DESCRIPTORS})
def containing(va):
    i = bisect.bisect_right(STARTS, va) - 1
    return STARTS[i] if i >= 0 else None
def extent(va):
    i = bisect.bisect_right(STARTS, va)
    return (va, STARTS[i] if i < len(STARTS) else va + 0x1000)

IN_BASE = 0x6000000; OUT_BASE = 0x6100000; BUF = 0x80000
STUB_BASE = 0x7000000
THING_BASE = 0x7100000  # fake virtual stubs: STUB_BASE + offset
THING_VIRTUALS = {0xa8: (0, 1), 0xac: (0, 1), 0xb0: (0, 1), 0xb4: (0, 1), 0xb8: (0, 1), 0xbc: (0, 1), 0xc0: (0, 1)}
DEF_VIRTUALS = {0x38: (0, 1)}
STRING_GLOBALS = [0xb3888c, 0xb38894, 0xb388a0, 0xb43b40, 0xb43b48]
READER_VTABLE = 0xa39da4  # [0]=0x4F0500 returns 1 (reading)
WRITER_VTABLE = 0xa39da8  # [0]=0x470F20 returns 2 (writing)
BIG_STACK = 0x5000000
TEAM_FLAGS = [0x0, 0x1, 0x2, 0x8, 0xb]
SESSION_PLAYER_IDS = [1, 2]
DEF_KIND_MODULUS = 4
STRING_POOL = [0x41, 0x42, 0x5a, 0x61, 0x7a, 0x30, 0x20, 0x00, 0xd800, 0xdfff, 0xe9, 0x4e2d]
FLOAT_POOL = [0x3f800000, 0xc0200000, 0x80000000, 0x7fc12345, 0x43480000, 0xffffffff, 0x00000001]

def mask(n): return (1 << n) - 1
def place_bits(buf, byte, bit, source, requested):
    """Python twin of the Phase5-proven 0x40D420 packing (Rust WireWriter)."""
    if requested == 0: return byte, bit
    available = 8 - bit
    if requested <= available:
        buf[byte] |= ((source[0] & mask(requested)) << bit) & 0xff
        return byte, bit + requested
    remaining = requested - available; whole = remaining // 8
    lead = source[whole] if whole < len(source) else 0
    buf[byte] |= ((lead & mask(available)) << bit) & 0xff
    buf[byte + 1:byte + 1 + whole] = bytes(source[:whole])
    byte += whole; tail = remaining % 8
    if tail == 0: return byte, 8
    byte += 1
    if available + tail <= 8:
        buf[byte] |= (source[whole] >> available) & mask(tail)
    else:
        low = 8 - available
        buf[byte] |= (source[whole] >> available) & mask(low)
        buf[byte] |= (source[whole + 1] & mask(tail - low)) << low
    return byte, tail

class Limit(Exception): pass
DEBUG = False

GD_4A0_WORD = 0x5ead04a0  # synthetic value of [[0xB36530]+0x4A0]

class TraceMachine(OrdersMachine):
    def __init__(self):
        self.steps = 0; self.step_limit = 1 << 62; self.pending = None; self.stub_table = {}; self.null_accesses = []
        super().__init__()
        registry(self)
        self.mode = 'trace'
        self.u.mem_map(IN_BASE, 2 * BUF + 0x100000)
        self.u.mem_map(STUB_BASE, 0x1000)
        self.u.mem_map(BIG_STACK, 0x400000)
        # Page 0 is mapped by the base machine (fs:[0] SEH head lives at 0).
        # Any other access to 0x4..0xFFF is a native null-pointer dereference
        # that would fault in the real process: record it.
        from unicorn import UC_HOOK_MEM_READ, UC_HOOK_MEM_WRITE
        # [0..3] counts too unless the instruction is FS-prefixed (the SEH link;
        # FS base is 0 in this emulator, docs/NATIVE-PORT-RULES.md).
        self.u.hook_add(UC_HOOK_MEM_READ | UC_HOOK_MEM_WRITE, self.null_hook, begin=0x0, end=0xfff)
        # Synthetic definition managers: +0x24 array, +0x28 count, unique counts.
        self.managers = {}
        self.def_vtable = self.vtable_for(DEF_VIRTUALS)
        for k, g in enumerate(manager_globals()):
            count = 1000 + 3 * k  # never equal to a literal 411FA0 bound
            manager = self.alloc(0x40); array = self.alloc(4 * count)
            self.w(g, manager); self.w(manager + 0x24, array); self.w(manager + 0x28, count)
            for i in range(count):
                d = self.alloc(0x80); self.w(d, self.def_vtable); self.w(d + 0xc, i)
                self.w(d + 0x20, i % DEF_KIND_MODULUS)  # e.g. WorldRule kind switch in 0x511E50
                self.w(d + 8, self.string(f'def{g:x}_{i}')); self.w(array + 4 * i, d)
            self.managers[g] = count
        self.count_to_manager = {c: g for g, c in self.managers.items()}
        assert len(self.count_to_manager) == len(self.managers)
        # Thing registry: every slot resolves to one fake thing.
        self.thing_vtable = self.vtable_for(THING_VIRTUALS)
        # One small fake thing per slot; +0x14 low16 is its own slot (writer 0x76A500 uses it).
        self.u.mem_map(THING_BASE, 65536 * 0x40)
        blob = bytearray(65536 * 0x40)
        for slot in range(65536):
            struct.pack_into('<I', blob, slot * 0x40, self.thing_vtable)
            struct.pack_into('<I', blob, slot * 0x40 + 0x14, 0x5a5a0000 | slot)
        self.u.mem_write(THING_BASE, bytes(blob))
        self.thing_registry = self.alloc(0x60004)
        self.u.mem_write(self.thing_registry + 0x20004, b''.join(struct.pack('<I', THING_BASE + s * 0x40) for s in range(65536)))
        self.w(0xb364c0, self.thing_registry)
        # Admin/ordering player/client for the generic request validation.
        self.w(0xb36558, self.admin)
        self.client = self.alloc(0x100); self.u.mem_write(self.client + 0x6c, b'\x01')
        self.player = self.alloc(0x80); self.w(self.player + 4, self.client); self.w(self.player + 0x20, 7)
        self.w(self.admin + 0x284, self.player)
        self.w(0xb36550, self.session); self.w(self.session + 0xe4, 0)
        # Session players with IDs 1 and 2 (live, film byte +0xE = 0): player
        # references 0x765F70 resolve only these IDs; others decode to null.
        nxt = 0
        for pid in reversed(SESSION_PLAYER_IDS):
            pl = self.alloc(0x80); self.w(pl + 0x20, pid); self.w(pl + 4, self.client)
            node = self.alloc(8); self.w(node, pl); self.w(node + 4, nxt); nxt = node
        self.w(self.session + 0xbc, nxt)
        # Static CString globals normally built by skipped CRT initializers.
        for g in STRING_GLOBALS: self.w(g, self.string(''))
        # Cheat-permission variables (inline validity of KKC_Cheat*Request):
        # [B39AB0]+0x10C -> byte 1 (enabled), +0x124 -> byte 0.
        table = self.alloc(0x400); on = self.alloc(4); off = self.alloc(4)
        self.u.mem_write(on, b''); self.w(table + 0x10c, on); self.w(table + 0x124, off); self.w(0xb39ab0, table)
        # +0x1BC / +0x1C0 -> byte vars read and toggled by the cheat decrees 30 / 35
        # (0x5FAB6B / 0x5FB10B; byte 0). Missing before: the null reads were masked by
        # the zero page (docs/NATIVE-PORT-RULES.md emulator caveats).
        self.w(table + 0x1bc, self.alloc(4)); self.w(table + 0x1c0, self.alloc(4))
        # Game data [B36530]+0x270 -> vector, +0x24 array of TeamCommand definitions with
        # flag words at +0x24 covering every conditional branch of decoder 0x509090.
        gd = self.alloc(0x1000); vec = self.alloc(0x40); arr = self.alloc(4 * len(TEAM_FLAGS))
        for i, flags in enumerate(TEAM_FLAGS):
            d = self.alloc(0x40); self.w(d + 0x24, flags); self.w(d + 0xc, i); self.w(arr + 4 * i, d)
        self.w(vec + 0x24, arr); self.w(vec + 0x28, len(TEAM_FLAGS)); self.w(gd + 0x270, vec); self.w(0xb36530, gd)
        mg = self.alloc(0x40); self.w(mg + 8, self.alloc(0x40)); self.w(gd + 0x820, mg)
        # [gd+0x4A0]+0: first word copied to +8 by the settings-object ctor 0x511D90 (RMC /
        # session-load orders). Without it the ctor dereferences null (previously masked by
        # the zero page / SEH word, docs/NATIVE-PORT-RULES.md emulator caveats).
        w4a0 = self.alloc(0x40); self.w(w4a0, GD_4A0_WORD); self.w(gd + 0x4a0, w4a0)
        # Trigger condition/action definitions [gd+0x3C8]+0x24 (raw index, read by 0x6B76A0).
        tv = self.alloc(0x40); ta = self.alloc(4 * 4096)
        for i in range(4096): self.w(ta + 4 * i, self.def_vtable)
        self.w(tv + 0x24, ta); self.w(tv + 0x28, 4096); self.w(gd + 0x3c8, tv)
        self.team_defs = arr
        self.force = {}
        self.mark = self.heap
    def null_hook(self, u, access, address, size, value, user):
        if address + size <= 4 and null_guard.fs_prefixed(u, u.reg_read(UC_X86_REG_EIP)):
            return
        if self.mode == 'trace':
            self.null_accesses.append({'address': address, 'pc': f'{u.reg_read(UC_X86_REG_EIP):#x}'})
    def text(self, p):
        out = b''
        while True:
            v = bytes(self.u.mem_read(p, 2)); p += 2
            if v == bytes(2): return out.decode('utf-16-le', errors='surrogatepass')
            out += v; assert len(out) < 100000
    def vtable_for(self, table):
        vt = self.alloc(0x400)
        for off in range(0, 0x400, 4):
            self.w(vt + off, STUB_BASE + off)
        self.stub_table[vt] = table
        return vt
    # ------------------------------------------------------------------
    def choose(self, fn, width):
        r = self.rng
        if fn == 0x411fa0:
            return r.randrange(self.last_count) if self.last_count > 0 else 0
        if fn == 0x410b50 and width == 16:
            return r.choice([0, 1, 2, 3, 5]) if self.policy == 'mixed' else r.choice([0, 1, 2])
        if width == 1: return r.randrange(2)
        if width == 16 and self.policy == 'mixed':
            # Inline CString counts are also 16-bit: bias toward short strings.
            return r.randrange(9) if r.random() < 0.6 else r.randrange(1 << 16)
        if width < 32:
            return r.randrange(1 << width) if self.policy == 'mixed' else r.randrange(min(3, 1 << width))
        if self.policy == 'small': return r.randrange(3)
        k = r.randrange(4)
        if k < 2: return r.randrange(4)
        if k == 2: return r.choice(FLOAT_POOL)
        return r.getrandbits(32)
    def hook(self, u, pc, size, user):
        if self.mode != 'trace':
            return super().hook(u, pc, size, user)
        self.visited[pc] = size
        if pc == 0x751fc0 and self.extent[0] == 0x766cf0:
            # The recovered non-template acknowledgement thunk reads no wire
            # payload. Its Admin acknowledgement effect is outside the generic
            # request/decree schema oracle, and is never fabricated as a Shape.
            self.stopped = True; u.emu_stop(); return
        self.steps += 1
        self.recent.append(pc)
        if len(self.recent) > 4000: del self.recent[:2000]
        if self.steps > self.step_limit:
            raise Limit()
        p = self.pending
        if p is not None and pc == p['ret'] and u.reg_read(UC_X86_REG_ESP) == p['esp']:
            self.finish_pending(); return
        if STUB_BASE <= pc < STUB_BASE + 0x1000:
            off = pc - STUB_BASE; this = u.reg_read(UC_X86_REG_ECX)
            vt = self.r(this) if this else 0
            table = self.stub_table.get(vt, {})
            if off not in table:
                raise RuntimeError(f'unknown virtual +{off:#x} on {vt:#x} called from {self.r(u.reg_read(UC_X86_REG_ESP)):#x}')
            pop, value = table[off]
            self.events.append({'kind': 'virtual', 'offset': off, 'object': 'thing' if vt == self.thing_vtable else 'definition', 'this': this})
            self.ret(pop, value); return
        if pc == 0x40d5d0:
            stream, dst, width = self.arg(0), self.arg(1), self.arg(2)
            ret = self.r(u.reg_read(UC_X86_REG_ESP))
            if stream == self.reader:
                fn = containing(ret)
                byte, bit = self.r(stream + 8), self.r(stream + 12)
                if width > 32:
                    units = [self.rng.choice(STRING_POOL) for _ in range(width // 16)] if width % 16 == 0 else []
                    if units: source = b''.join(struct.pack('<H', x) for x in units)
                    else: source = bytes(self.rng.getrandbits(8) for _ in range((width + 7) // 8))
                    chosen = source.hex()
                else:
                    value = self.choose(fn, width); source = struct.pack('<I', value) + b'\0'
                    chosen = value
                buf = bytearray(self.u.mem_read(IN_BASE + byte, (width + 7) // 8 + 2))
                nb, nbit = place_bits(buf, 0, bit, source, width)
                self.u.mem_write(IN_BASE + byte, bytes(buf))
                self.pending = {'ret': ret, 'esp': u.reg_read(UC_X86_REG_ESP) + 4, 'dst': dst, 'width': width,
                                'fn': fn, 'start': [byte, bit], 'chosen': chosen, 'stream': 'in',
                                'registry_count': self.last_count if fn == 0x411fa0 else None}
            return
        if pc == 0x40d420:
            stream, src, width = self.arg(0), self.arg(1), self.arg(2)
            if stream == self.out_stream:
                ret = self.r(u.reg_read(UC_X86_REG_ESP))
                n = (width + 7) // 8
                data = bytes(self.u.mem_read(src, n))
                self.pending = {'ret': ret, 'esp': u.reg_read(UC_X86_REG_ESP) + 4, 'width': width, 'fn': containing(ret),
                                'start': [self.r(stream + 8), self.r(stream + 12)], 'stream': 'out',
                                'value': struct.unpack('<I', (data + b'\0' * 4)[:4])[0] & mask(width) if width <= 32 else data.hex()}
            return
        if pc == 0x411fa0:
            self.last_count = self.arg(2); return
        if pc in self.force:
            self.events.append({'kind': 'forced_validity', 'function': f'{pc:#x}'})
            self.ret(self.force[pc], 1); return
        if pc == 0x6fb1f0:
            # Clock seam: local FILETIME (EDX:EAX) = 0.
            self.events.append({'kind': 'clock'}); u.reg_write(UC_X86_REG_EDX, 0); self.ret(0, 0); return
        if pc == 0x783870:
            # Address-interning seam: sockaddr_in-like object, +6 port (u16), +8 IPv4.
            ip, port = self.arg(0), self.arg(1); obj = self.alloc(0x20)
            self.u.mem_write(obj + 6, struct.pack('<H', port & 0xffff)); self.w(obj + 8, ip)
            self.events.append({'kind': 'net_address', 'ip': ip, 'port': port}); self.ret(8, obj); return
        if pc == 0x781f20:
            self.ret(0, self.r(u.reg_read(UC_X86_REG_ECX) + 8)); return
        if pc == 0x781f30:
            self.ret(0, struct.unpack('<H', self.u.mem_read(u.reg_read(UC_X86_REG_ECX) + 6, 2))[0]); return
        if pc == 0x405370:
            # Configuration-variable lookup seam: node -> zero-valued variable object.
            key = self.r(self.arg(0)); var = self.alloc(0x40); node = self.alloc(8)
            self.w(var + 0x1c, self.string(self.text(key))); self.w(node, var)
            self.events.append({'kind': 'variable', 'name': self.text(key)}); self.ret(4, node); return
        if pc == 0x93e170:
            # Metagame container-by-name seam (writer 0x957ED0 reads container+8).
            name = self.text(self.arg(0)); c = self.alloc(0x40); self.w(c + 8, self.string(name))
            self.events.append({'kind': 'mg_container', 'name': name}); self.ret(4, c); return
        if pc == 0x95d900:
            # Metagame element-by-name seam: element+0x40 container, +4 element name.
            name = self.text(self.r(self.arg(0))); o = self.alloc(0x60)
            self.w(o + 0x40, u.reg_read(UC_X86_REG_ECX)); self.w(o + 4, self.string(name))
            self.events.append({'kind': 'mg_element', 'name': name}); self.ret(4, o); return
        if pc == 0x41eec0:
            # Named-definition lookup seam. The native first statement returns null
            # for an empty name (*param_2 == 0); otherwise the wide-string map
            # lookup (0x4154D0) is replaced by a synthetic object whose +8 is the
            # NUL-terminated name (C-string key semantics).
            if self.u.mem_read(self.arg(0), 2) == bytes(2):
                self.events.append({'kind': 'named_lookup', 'name': ''}); self.ret(4, 0); return
            name = self.arg(0); obj = self.alloc(0x40)
            self.w(obj + 8, self.string(self.text(name))); self.events.append({'kind': 'named_lookup', 'name': self.text(name)})
            self.ret(4, obj); return
        if pc == 0x4012c0:
            ret = self.r(u.reg_read(UC_X86_REG_ESP))
            if self.extent[0] <= ret < self.extent[1]:
                self.stopped = True; u.emu_stop(); return
        if pc == 0x405cb0:
            raise RuntimeError('native exception format ' + repr(self.text(self.arg())) + f' from {self.r(u.reg_read(UC_X86_REG_ESP)):#x}')
        if pc == 0x855d30:
            raise RuntimeError('native assertion ' + self.text(self.arg()))
        super().hook(u, pc, size, user)
    def finish_pending(self):
        p = self.pending; self.pending = None
        if p['stream'] == 'in':
            stream = self.reader
            if p['width'] <= 32:
                got = self.r(p['dst']) & mask(p['width']) if p['width'] < 32 else self.r(p['dst'])
                assert got == p['chosen'], (p, got)
                value = got
            else:
                value = bytes(self.u.mem_read(p['dst'], (p['width'] + 7) // 8)).hex()
                assert value == p['chosen'], (p, value)
            self.reads.append({'fn': f"{p['fn']:08x}", 'width': p['width'], 'value': value, 'start': p['start'],
                               'end': [self.r(stream + 8), self.r(stream + 12)], 'registry_count': p['registry_count']})
        else:
            stream = self.out_stream
            self.writes.append({'fn': f"{p['fn']:08x}", 'width': p['width'], 'value': p['value'], 'start': p['start'],
                                'end': [self.r(stream + 8), self.r(stream + 12)]})
    # ------------------------------------------------------------------
    def run(self, descriptor, seed, policy, force=()):
        ident, name, handler = descriptor
        self.force = {f: ret_pop(f) for f in force}
        self.heap = self.mark
        self.u.mem_write(0, bytes(0x1000))  # zero page (fs:[0] SEH head) may be dirtied by a faulting run
        if not hasattr(self, "cpu_context"): self.cpu_context = self.u.context_save()
        self.u.context_restore(self.cpu_context)  # a faulting run may leave EFLAGS/segments altered
        self.u.mem_write(IN_BASE, b'\0' * (2 * BUF))
        self.w(self.reader, READER_VTABLE); self.w(self.reader + 4, IN_BASE); self.w(self.reader + 8, 0); self.w(self.reader + 12, 3)
        self.out_stream = self.admin + 0x1f8
        self.u.mem_write(OUT_BASE, bytes(BUF))  # the native writer ORs into its buffer
        self.w(OUT_BASE, 1); self.w(OUT_BASE + 4, BUF - 16)
        self.w(self.out_stream, WRITER_VTABLE); self.w(self.out_stream + 4, OUT_BASE + 8); self.w(self.out_stream + 8, 0); self.w(self.out_stream + 12, 3)
        self.rng = random.Random(seed * 1000 + ident); self.policy = policy
        self.reads, self.writes, self.events = [], [], []; self.null_accesses = []
        self.pending = None; self.last_count = 0; self.stopped = False
        self.steps = 0; self.step_limit = 3_000_000; self.extent = extent(handler); self.recent = []
        top = BIG_STACK + 0x3f0000
        self.w(top, self.stop); self.w(top + 4, self.reader)
        self.u.reg_write(UC_X86_REG_ESP, top); self.u.reg_write(UC_X86_REG_ECX, 0)
        status, error = 'returned', None
        try:
            self.u.emu_start(handler, self.stop, count=0)
            if self.stopped: status = 'stopped_before_process'
            elif self.u.reg_read(UC_X86_REG_EIP) != self.stop: status = 'error'; error = f'eip {self.u.reg_read(UC_X86_REG_EIP):#x}'
        except Limit:
            status, error = 'error', 'step limit'
        except Exception as e:
            status, error = 'error', f'{type(e).__name__}: {e} at {self.u.reg_read(UC_X86_REG_EIP):#x}'

        end = [self.r(self.reader + 8), self.r(self.reader + 12)]
        nbytes = (end[0] * 8 + end[1] + 7) // 8
        oend = [self.r(self.out_stream + 8), self.r(self.out_stream + 12)]
        obytes = (oend[0] * 8 + oend[1] + 7) // 8
        if self.null_accesses:
            # The original process faults at the first such access; whatever the
            # emulation did afterwards (zero page) is not native behaviour.
            status = 'native_null_dereference'
        return {'seed': seed, 'policy': policy, 'status': status, 'error': error, 'null_accesses': self.null_accesses[:8],
                'input': bytes(self.u.mem_read(IN_BASE, nbytes)).hex(), 'end': end,
                'reads': self.reads, 'writes': self.writes,
                'output': bytes(self.u.mem_read(self.r(self.out_stream + 4), obytes)).hex() if self.writes else None,
                'output_end': oend if self.writes else None,
                'virtuals': [e for e in self.events if e['kind'] == 'virtual']}

def ret_pop(f, depth=0):
    """Callee stack pop of function f: its `ret n`, following tail-call thunks."""
    from natdis import insns
    lo, hi = extent(f)
    for i in insns(lo, size=max(hi - lo, 16)):
        if i.mnemonic == 'ret': return int(i.op_str, 16) if i.op_str else 0
        if i.mnemonic == 'jmp' and depth < 4:
            try: target = int(i.op_str, 16)
            except ValueError: continue
            if not (lo <= target < hi): return ret_pop(target, depth + 1)
    raise ValueError(hex(f))

STATIC = {r['id']: r for r in json.loads((HERE / 'static-handler-calls.json').read_text())}

def main(seeds=8, only=None):
    m = TraceMachine(); out = []
    for d in DESCRIPTORS:
        if only is not None and d[0] not in only: continue
        runs = []
        for seed in range(seeds):
            validity = [int(v, 16) for v in STATIC[d[0]].get('validity') or []]
            r = m.run(d, seed, 'mixed', force=validity)
            if r['status'] == 'error':
                r2 = m.run(d, seed, 'small', force=validity); r2['retry_of_error'] = r['error']; r = r2
            r['forced_validity'] = [f'{v:#x}' for v in validity]
            runs.append(r)
        out.append({'id': d[0], 'name': d[1], 'handler': f'{d[2]:#x}', 'runs': runs})
        st = {x['status'] for x in runs}
        print(d[0], d[1], st, [x['error'] for x in runs if x['error']][:1], flush=True)
        if DEBUG and 'error' in st:
            m.run(d, 0, 'small'); fns = []
            for x in m.recent[-3000:]:
                f = containing(x)
                if not fns or fns[-1] != f: fns.append(f)
            print('  path', ' '.join(f'{f:x}' for f in fns[-40:]))
    return m, out

if __name__ == '__main__':
    args = sys.argv[1:]
    if args and args[0] == 'debug': DEBUG = True; args = args[1:]
    only = set(int(x) for x in args) or None
    m, out = main(only=only)
    report = {'schema': 1, 'scope': __doc__, 'original_sha256': sha(PE0.b), 'v14_sha256': sha(PE1.b),
              'producer_sha256': sha(Path(__file__).read_bytes()),
              'managers': {f'{g:#x}': c for g, c in m.managers.items()},
              'environment': {'definition_plus_0x20': f'index % {DEF_KIND_MODULUS}', 'definition_plus_0xc': 'index',
                              'team_command_flags': TEAM_FLAGS, 'session_player_ids': SESSION_PLAYER_IDS,
                              'thing_slots': 'every slot resolves; thing+0x14 low16 = slot; all type checks true',
                              'definitions_valid': 'virtual +0x38 returns 1 for every index < count',
                              'game_data_plus_0x4a0_word': GD_4A0_WORD},
              'descriptors': out}
    name = 'order-traces.json' if only is None else 'order-traces-partial.json'
    (HERE / name).write_text(json.dumps(report) + '\n')
