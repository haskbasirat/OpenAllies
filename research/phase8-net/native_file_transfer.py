"""Original lobby file transfer of AA.exe under Unicorn.

Executes the ORIGINAL TGC_FileSender (ctor 0x7611B0, reset 0x761400, start
0x74FE30 -> 0x761540, vtable 0xA91BEC: available 0x761920 -> read-ahead
0x7617E0 -> block read 0x761420, release 0x761850, estimate 0x761310,
serialize 0x7613C0 -> 0x75FFA0, progress 0x7612D0) and TGC_FileReceiver
(ctor 0x761870, reset 0x761500, vtable 0xA91C00: receive 0x761DB0 -> payload
0x7610A0 -> drain 0x761CE0 -> header 0x7619A0 / write 0x761220, skip
0x761150), the AdminConn generation check 0x74C8B0 and, for the channel
scenarios, the original TGC_BlockSender serialize 0x760A80 / packet result
0x7608E0 and TGC_BlockReceiver 0x760F20 / skip 0x75FEA0 carrying file blocks.

Every scenario runs twice: on the original AA.exe and on the accepted v14
AA.exe (.text/.rdata of the v14 image loaded over the original), whose only
executed differences are the terminal-block length sites 0x76146A (sender,
cave 0x9EFF80) and 0x761270 (receiver, cave 0x9EFF90).

Seams (explicit, recorded):
  * TGC_File (0x857FE0, vtable 0xAB4DE8) is the OS file layer. Right after the
    original ctor 0x857FE0 returns inside 0x7611B0 / 0x761870 (pcs 0x76120C /
    0x7618CB) the object's vtable pointer is replaced by a recording model:
    +0x00 read(dst, n) ret 8, +0x04 write(src, n) ret 8, +0x14 is_open(),
    +0x20 open(CString* path, mode) ret 8, +0x28 close(), +0x30 size(). Any
    other slot raises. Files live in a Python dict keyed by the exact UTF-16
    path the original code builds. Open failure (TGC_FileException 0x869200,
    C++ unwinding) is not emulated.
  * allocator 0x6F7810/0x6F7430, CString manager 0x856370 (vtable), CString
    literal ctor 0x401B00, realloc 0x401330 (Phase3/Phase4 machines).
    CString concatenation 0x4108C0 -> 0x40F4B0 executes ORIGINAL code.
  * logging 0x404F60: recorded (line number and decoded arguments).
  * 0x405D60 / 0x405A50 inside 0x760F20: debug CString of the block index
    (no wire or state effect), skipped as in native_channels.py.
  * AVar table [0xB39990] (+0x120 BlocksPerFrame, +0x124 BlockSize, +0x128
    MaxPendingBlocks) and the profile directory CString
    [[0xB36540]+0x90]+4 are synthetic objects set per scenario.
The harness never resets the heap (native statics keep heap pointers).
"""
from pathlib import Path
import json, struct, sys
HERE = Path(__file__).resolve().parent; ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase4-orders'))
from recover_orders import OrdersMachine, PE0, PE1, sha
sys.path.insert(0, str(HERE))
import null_guard
from unicorn.x86_const import *

READER_VT = 0xa39da4; WRITER_VT = 0xa39da8
STUB = 0x7800000                      # own mapping: stub code + model vtables
ADMIN = [0x2030000, 0x2031000]        # synthetic TGC_AdminConn objects (0x424 bytes)
S_OFF, R_OFF, GEN_OFF = 0x39c, 0x3e8, 0x418
AVARS = 0x2032000; APP = 0x2033000
MODE_READ = struct.unpack('<I', PE0.range(0xab4784, 4))[0]
MODE_WRITE = struct.unpack('<I', PE0.range(0xab4788, 4))[0]
assert (MODE_READ, MODE_WRITE) == (0x20011, 0x2010a) and PE1.range(0xab4784, 8) == PE0.range(0xab4784, 8)
PROFILE = 'C:\\AA\\Profiles\\Player\\'
DEFAULT = {'blocks_per_frame': 20, 'block_size': 64, 'max_pending_blocks': 256}
# Code/constant sections replaced by the v14 image for the v14 runs (.text,
# .rdata). .data keeps the Phase3/4 machine initialisation.
V14_SECTIONS = (0x401000, 0x9f0000)

def fnv(b):
    h = 0xcbf29ce484222325
    for x in b: h = ((h ^ x) * 0x100000001b3) & 0xffffffffffffffff
    return '%016x' % h

def s32(v): return struct.unpack('<i', struct.pack('<I', v & 0xffffffff))[0]
def units(text):
    b = text.encode('utf-16-le', 'surrogatepass'); return list(struct.unpack('<%dH' % (len(b) // 2), b))

class TGCFile:
    def __init__(self): self.open = False; self.path = None; self.pos = 0

class FileMachine(OrdersMachine):
    def __init__(self, image='original'):
        super().__init__()
        self.mode = 'file'; self.image = image
        null_guard.install(self)  # page 0: only FS-prefixed [0..3] (docs/NATIVE-PORT-RULES.md)
        if image == 'v14':
            for vs, r, size, off in PE1.sections:
                if PE1.base + r in V14_SECTIONS: self.u.mem_write(PE1.base + r, PE1.b[off:off + size])
        self.u.mem_map(STUB, 0x2000)
        # disks[0]: the sending machine's files, disks[1]: the receiving machine's.
        self.files = {}; self.roles = {}; self.disks = [{}, {}]; self.reported = {}
        self.calls = []; self.log = []
        self.w(0xb39990, AVARS)
        for i, off in enumerate((0x120, 0x124, 0x128)): self.w(AVARS + off, AVARS + 0x800 + 4 * i)
        self.w(0xb36540, APP); self.w(APP + 0x90, APP + 0x100)
        self.w(APP + 0x104, self.cstring(units(PROFILE)))
        for k in range(4):
            for slot in range(16): self.w(self.vtable_of(k) + 4 * slot, STUB + 0x100 * k + 4 * slot)
    # --- helpers -------------------------------------------------------
    def vtable_of(self, k): return STUB + 0x1000 + 0x40 * k
    def file_addr(self, k): return ADMIN[k // 2] + (S_OFF + 0x3c if k % 2 == 0 else R_OFF + 0x18)
    def cstring(self, us):
        p = self.alloc(16 + 2 * len(us) + 2)
        self.w(p, self.manager); self.w(p + 4, len(us)); self.w(p + 8, len(us)); self.w(p + 12, 1)
        if us: self.u.mem_write(p + 16, struct.pack('<%dH' % len(us), *us))
        return p + 16
    def cunits(self, p):
        n = s32(self.r(p - 12)); assert 0 <= n < 0x20000
        return list(struct.unpack('<%dH' % n, self.u.mem_read(p, 2 * n))) if n else []
    def set_config(self, c):
        for i, k in enumerate(('blocks_per_frame', 'block_size', 'max_pending_blocks')): self.w(AVARS + 0x800 + 4 * i, c[k])
    def call(self, pc, args=(), this=None, callee_pop=0):
        self.w(self.stack, self.stop)
        for i, x in enumerate(args): self.w(self.stack + 4 + 4 * i, x)
        self.u.reg_write(UC_X86_REG_ESP, self.stack); self.u.reg_write(UC_X86_REG_ECX, this or 0)
        regs = {UC_X86_REG_EBX: 0xabcdef01, UC_X86_REG_ESI: 0xabcdef02, UC_X86_REG_EDI: 0xabcdef03, UC_X86_REG_EBP: 0xabcdef04}
        for r, v in regs.items(): self.u.reg_write(r, v)
        try: self.u.emu_start(pc, self.stop, count=50_000_000)
        except Exception as e: raise RuntimeError(f'native call {pc:08x} at {self.u.reg_read(UC_X86_REG_EIP):08x}') from e
        assert self.u.reg_read(UC_X86_REG_EIP) == self.stop, hex(self.u.reg_read(UC_X86_REG_EIP))
        assert self.u.reg_read(UC_X86_REG_ESP) == self.stack + 4 + callee_pop, (hex(pc), self.u.reg_read(UC_X86_REG_ESP))
        for r, v in regs.items(): assert self.u.reg_read(r) == v, (hex(pc), r)
        return self.u.reg_read(UC_X86_REG_EAX)
    def stream(self, vt, data=None, size=0x2000):
        obj = self.alloc(16)
        buf = self.packet(list(data) + [0] * 8 if data is not None else [0] * size, refs=1)
        self.w(obj, vt); self.w(obj + 4, buf); self.w(obj + 8, 0); self.w(obj + 12, 3)
        return obj
    # --- hooks -----------------------------------------------------------
    def hook(self, u, pc, size, user):
        self.visited[pc] = size
        if STUB <= pc < STUB + 0x400:
            return self.file_call(u, (pc - STUB) // 0x100, ((pc - STUB) % 0x100) // 4)
        if pc in (0x76120c, 0x7618cb):
            # Inside 0x7611B0 / 0x761870, right after the original TGC_File ctor
            # 0x857FE0 returned (it stored vtable 0xAB4DE8): install the model.
            obj = u.reg_read(UC_X86_REG_ESI) + 0x3c if pc == 0x76120c else u.reg_read(UC_X86_REG_EDI)
            k = [self.file_addr(i) for i in range(4)].index(obj)
            assert self.r(obj) == 0xab4de8
            self.w(obj, self.vtable_of(k)); self.files[k] = TGCFile(); return
        if pc == 0x404f60:
            self.log_line(); self.ret(); return
        if pc == 0x405d60:  # CString::Format of a debug label in 0x760F20
            self.ret(); return
        if pc == 0x405a50:  # CString assign of that label
            self.ret(8); return
        if pc == 0x4108c0:  # run the ORIGINAL length-based concatenation (0x40F4B0)
            return
        assert pc not in self.nonorder, hex(pc)
        super().hook(u, pc, size, user)
    def log_line(self):
        line = self.arg(2)
        if line == 0xbf: e = {'line': line, 'name': self.cunits(self.arg(5))}
        elif line == 0xac: e = {'line': line, 'component': self.cunits(self.arg(5)), 'name': self.cunits(self.arg(6)), 'size': s32(self.arg(7))}
        elif line in (0x168c, 0x1694): e = {'line': line, 'packet': s32(self.arg(5)), 'mine': s32(self.arg(6))}
        else: raise AssertionError(f'unexpected log line {line:#x}')
        self.log.append(e)
    def file_call(self, u, k, slot):
        f = self.files[k]; assert u.reg_read(UC_X86_REG_ECX) == self.file_addr(k)
        role = self.roles.get(k)
        def rec(d):
            if role: self.calls.append({'file': role, **d})
        if slot == 0x20 // 4:
            path = self.cunits(self.r(self.arg(0))); mode = self.arg(1); key = tuple(path)
            disk = self.disks[k % 2]
            if mode == MODE_READ: ok = key in disk
            elif mode == MODE_WRITE: ok = True; disk[key] = bytearray()
            else: raise AssertionError(hex(mode))
            assert ok, 'open failure throws TGC_FileException (0x869200): not emulated'
            f.open = True; f.path = key; f.pos = 0
            rec({'op': 'open', 'path': path, 'mode': mode}); self.ret(8, 1)
        elif slot == 0x14 // 4:
            rec({'op': 'is_open', 'result': f.open}); self.ret(0, int(f.open))
        elif slot == 0:
            dst, n = self.arg(0), s32(self.arg(1)); assert f.open and 0 <= n <= s32(self.r(dst - 4))
            data = self.disks[k % 2][f.path]; chunk = bytes(data[f.pos:f.pos + n]); f.pos += len(chunk)
            if chunk: self.u.mem_write(dst, chunk)
            rec({'op': 'read', 'len': n, 'returned': len(chunk)}); self.ret(8, len(chunk))
        elif slot == 1:
            src, n = self.arg(0), s32(self.arg(1))
            assert f.open and 0 <= n <= s32(self.r(src - 4)), 'native would read past the block buffer'
            chunk = bytes(self.u.mem_read(src, n)) if n else b''; self.disks[k % 2][f.path].extend(chunk)
            rec({'op': 'write', 'bytes': chunk.hex()}); self.ret(8, n)
        elif slot == 0x28 // 4:
            rec({'op': 'close'}); f.open = False; self.ret(0)
        elif slot == 0x30 // 4:
            assert f.open, 'size of an unopened TGC_File dereferences a null impl (0x856DA0)'
            v = self.reported.get(f.path, len(self.disks[k % 2][f.path]))
            rec({'op': 'size', 'result': v}); self.ret(0, v & 0xffffffff)
        else:
            raise AssertionError(f'unmodelled TGC_File slot {slot * 4:#x}')
    # --- native state ------------------------------------------------------
    def buffers(self, base):
        begin, end, cap, data = s32(self.r(base)), s32(self.r(base + 4)), self.r(base + 8), self.r(base + 12)
        assert 0 <= end - begin <= 0x10000, (begin, end)
        slots = []
        for i in range(begin, end):
            p = self.r(data + 4 * (((i + 0x80000000) & 0xffffffff) % cap))
            if p:
                b = bytes(self.u.mem_read(p, self.r(p - 4))); slots.append([i, len(b), fnv(b)])
        return {'begin': begin, 'end': end, 'capacity': cap, 'slots': slots}
    def records(self, base, size, fields):
        begin, end, cap, data = s32(self.r(base)), s32(self.r(base + 4)), self.r(base + 8), self.r(base + 12)
        out = []
        for i in range(begin, end):
            r = data + (((i + 0x80000000) & 0xffffffff) % cap) * size
            out.append([s32(self.r(r + 4 * j)) if j < 3 else self.r(r + 4 * j) for j in range(fields)])
        return {'begin': begin, 'end': end, 'records': out}
    def sender_state(self, a):
        S = ADMIN[a] + S_OFF
        return {'packets': self.records(S + 4, 16, 4), 'blocks': self.records(S + 0x14, 12, 2),
                'span': s32(self.r(S + 0x24)), 'copies': s32(self.r(S + 0x28)),
                'queue': self.buffers(S + 0x2c), 'size': s32(self.r(S + 0x48)), 'generation': s32(self.r(ADMIN[a] + GEN_OFF))}
    def receiver_state(self, a):
        R = ADMIN[a] + R_OFF
        return {'span': s32(self.r(R + 4)), 'queue': self.buffers(R + 8), 'size': s32(self.r(R + 0x24)),
                'generation': s32(self.r(ADMIN[a] + GEN_OFF))}

class Scenario:
    """One recorded run. The sender lives in ADMIN[sa], the receiver in ADMIN[ra]."""
    def __init__(self, m, name, kind='unit', sender_config=DEFAULT, receiver_config=None, sources=(), reported=()):
        self.m = m; self.name = name; self.kind = kind; self.sa = 0; self.ra = 0 if kind == 'unit' else 1
        self.sc = dict(sender_config); self.rc = dict(receiver_config or sender_config)
        m.disks = [{tuple(units(PROFILE + n)): bytearray(d) for n, d in sources}, {}]
        m.reported = {tuple(units(PROFILE + n)): v for n, v in reported}
        self.sources = [[units(n), bytes(d).hex()] for n, d in sources]
        self.reported_sizes = [[units(n), v] for n, v in reported]
        m.roles = {2 * self.sa: 'sender', 2 * self.ra + 1: 'receiver'}
        self.steps = []; self.payloads = {}
    def step(self, op, side, fn, **inputs):
        m = self.m; m.calls = []; m.log = []
        m.set_config(self.sc if side == 'sender' else self.rc)
        out = fn(); out = out if isinstance(out, dict) else {}
        self.steps.append({'op': op, **inputs, **out, 'calls': m.calls, 'log': m.log,
                           'sender': m.sender_state(self.sa), 'receiver': m.receiver_state(self.ra)})
        return out
    # --- operations ------------------------------------------------------
    def construct(self):
        m = self.m
        def run():
            ctor_size = {}
            for a in sorted({self.sa, self.ra}):
                S, R = ADMIN[a] + S_OFF, ADMIN[a] + R_OFF
                m.u.mem_write(S, b'\xcd' * 0x4c); m.u.mem_write(R, b'\xcd' * 0x30)
                m.set_config(self.sc if a == self.sa else self.rc); m.call(0x7611b0, this=S)
                ctor_size[a] = s32(m.r(S + 0x48))
                m.set_config(self.rc if a == self.ra else self.sc); m.call(0x761870, this=R)
                # AdminConn ctor 0x756BC0 -> reset 0x752110: 0x761400, 0x761500, +0x418 = 0.
                m.call(0x761400, this=S); m.call(0x761500, this=R)
                m.w(ADMIN[a] + GEN_OFF, 0)
            return {'sender_size_after_ctor': ctor_size[self.sa]}
        self.step('construct', 'sender', run)
    def start(self, name):
        m = self.m
        self.step('start', 'sender', lambda: m.call(0x74fe30, [m.cstring(name)], this=ADMIN[self.sa], callee_pop=4), name=name)
    def sender_reset(self): self.step('sender_reset', 'sender', lambda: self.m.call(0x761400, this=ADMIN[self.sa] + S_OFF))
    def receiver_reset(self): self.step('receiver_reset', 'receiver', lambda: self.m.call(0x761500, this=ADMIN[self.ra] + R_OFF))
    def available(self):
        return self.step('available', 'sender', lambda: {'result': s32(self.m.call(0x761920, this=ADMIN[self.sa] + S_OFF))})['result']
    def progress(self):
        return self.step('progress', 'sender', lambda: {'result': s32(self.m.call(0x7612d0, this=ADMIN[self.sa] + S_OFF))})['result']
    def estimate(self, i):
        return self.step('estimate', 'sender', lambda: {'result': s32(self.m.call(0x761310, [i & 0xffffffff], this=ADMIN[self.sa] + S_OFF, callee_pop=4))}, index=i)['result']
    def release(self, i):
        self.step('release', 'sender', lambda: self.m.call(0x761850, [i & 0xffffffff], this=ADMIN[self.sa] + S_OFF, callee_pop=4), index=i)
    def serialize(self, i):
        m = self.m
        def run():
            W = m.stream(WRITER_VT)
            m.call(0x7613c0, [W, i & 0xffffffff], this=ADMIN[self.sa] + S_OFF, callee_pop=8)
            end = m.position(W); bits = end['byte'] * 8 + end['bit']
            data = bytes(m.u.mem_read(m.r(W + 4), (bits + 7) // 8))
            self.payloads[i] = data
            return {'bytes': data.hex(), 'end': end}
        return bytes.fromhex(self.step('serialize', 'sender', run, index=i)['bytes'])
    def _read(self, op, pc, i, data):
        m = self.m
        def run():
            Rd = m.stream(READER_VT, data)
            m.call(pc, [Rd, i & 0xffffffff], this=ADMIN[self.ra] + R_OFF, callee_pop=8)
            return {'end': m.position(Rd)}
        self.step(op, 'receiver', run, index=i, bytes=data.hex())
    def receive(self, i, data=None): self._read('receive', 0x761db0, i, self.payloads[i] if data is None else data)
    def skip(self, i, data=None): self._read('skip', 0x761150, i, self.payloads[i] if data is None else data)
    # --- channel-level operations (original BlockSender / BlockReceiver) -------
    def section(self, sequence, limit):
        m = self.m
        def run():
            W = m.stream(WRITER_VT)
            gen = m.alloc(4); m.w(gen, m.r(ADMIN[self.sa] + GEN_OFF))
            m.call(0x40d420, [W, gen, 32])  # 32-bit file generation word (as 0x74E280 writes it)
            m.call(0x760a80, [W, sequence & 0xffffffff, limit & 0xffffffff], this=ADMIN[self.sa] + S_OFF, callee_pop=12)
            end = m.position(W); bits = end['byte'] * 8 + end['bit']
            return {'bytes': bytes(m.u.mem_read(m.r(W + 4), (bits + 7) // 8)).hex(), 'end': end}
        return bytes.fromhex(self.step('section', 'sender', run, sequence=sequence, limit=limit)['bytes'])
    def deliver(self, sequence, data):
        m = self.m
        def run():
            Rd = m.stream(READER_VT, data)
            m.call(0x74c8b0, [Rd, sequence & 0xffffffff], this=ADMIN[self.ra], callee_pop=8)
            return {'end': m.position(Rd)}
        self.step('deliver', 'receiver', run, sequence=sequence, bytes=data.hex())
    def packet_result(self, sequence, acked):
        self.step('packet_result', 'sender', lambda: self.m.call(0x7608e0, [sequence & 0xffffffff, int(acked)], this=ADMIN[self.sa] + S_OFF, callee_pop=8), sequence=sequence, acked=acked)
    def finish(self):
        rf = self.m.files[2 * self.ra + 1]
        disk = [[list(k), bytes(v).hex(), rf.open and rf.path == k] for k, v in sorted(self.m.disks[1].items())]
        return {'name': self.name, 'kind': self.kind, 'sender_config': self.sc, 'receiver_config': self.rc,
                'profile_dir': units(PROFILE), 'sources': self.sources, 'reported_sizes': self.reported_sizes,
                'steps': self.steps, 'disk': disk}

# ----------------------------------------------------------------------------
def data_of(n, seed):
    return bytes(((i * 131 + seed * 17 + (i >> 3) * 7) ^ (i >> 5)) & 0xff for i in range(n))

def unit_transfer(m, name, size, order='in', duplicates=(), config=DEFAULT, receiver_config=None,
                  file='maps\\europe.tgm', header_last=False, reported=None, release_each=True):
    """Start, read ahead, estimate and serialize every available block, deliver
    them to the receiver in the requested order, release in order."""
    src = data_of(size, len(name))
    sc = Scenario(m, name, sender_config=config, receiver_config=receiver_config, sources=[(file, src)],
                  reported=[(file, reported)] if reported is not None else [])
    sc.construct(); sc.available(); sc.progress()
    sc.start(units(file))
    released = 0; seen = set()
    for _round in range(64):
        n = sc.available()
        new = [i for i in range(released, n) if i not in seen]
        if not new: break
        for i in new: sc.estimate(i); sc.serialize(i); seen.add(i)
        idx = list(new)
        if order == 'reverse': idx.reverse()
        elif order == 'shuffle': idx = idx[1::2] + idx[0::2]
        if header_last and 0 in idx: idx.remove(0); idx.append(0)
        for i in idx:
            sc.receive(i)
            if i in duplicates: sc.receive(i)
        if release_each:
            for i in new: sc.release(i + 1); sc.progress()
        else:
            sc.release(n); sc.progress()
        released = max(released, n)
    sc.available(); sc.progress()
    return sc.finish()

def edge_cases(m):
    """Direct calls outside the BlockSender's normal order: estimate/serialize of
    indices beyond the queue, release past the end, stale/duplicate/far-ahead
    receiver indices, skip, receiver/sender reset mid-transfer."""
    file = 'saves\\game.sav'
    sc = Scenario(m, 'edge_direct_calls', sources=[(file, data_of(400, 3))])
    sc.construct()
    sc.progress()
    sc.start(units(file))
    sc.estimate(0); sc.serialize(0)
    n = sc.available()
    for i in range(n): sc.serialize(i)
    sc.estimate(n + 2)              # beyond the queue: extends it with empty slots, 32
    sc.available()
    sc.receive(3); sc.receive(3)    # buffered ahead of the header, duplicate replaces
    sc.receive(40, sc.payloads[2])  # far ahead: window grows to 41
    sc.skip(1)                      # decode only
    sc.receive(0)                   # header drains 0, then stops at missing 1
    sc.receive(0)                   # stale (below begin): decoded and dropped
    sc.receive(-5, sc.payloads[1])  # negative index: dropped
    sc.receive(1); sc.receive(2)    # drains 1..3
    sc.release(2); sc.progress()
    sc.release(n + 10); sc.progress(); sc.available()  # release past the end
    sc.receiver_reset()             # closes the open download
    sc.receive(4)                   # index 4 after reset: buffered, no header
    sc.sender_reset(); sc.available(); sc.progress()
    return sc.finish()

def restart_case(m):
    """Start a second transfer before the first finished (0x761540 does not
    close the previous file)."""
    a, b = 'maps\\first.tgm', 'maps\\second.tgm'
    sc = Scenario(m, 'restart_mid_transfer', sources=[(a, data_of(500, 1)), (b, data_of(130, 2))])
    sc.construct(); sc.start(units(a))
    n = sc.available()
    for i in range(3): sc.estimate(i); sc.serialize(i); sc.receive(i)
    sc.release(2)
    sc.start(units(b))
    n = sc.available()
    sc.receiver_reset()
    for i in range(n): sc.estimate(i); sc.serialize(i); sc.receive(i)
    sc.release(n); sc.progress()
    return sc.finish()

def channel_run(m, name, size, limits, drop=(), ack_delay=0, late=(), restart_at=None, config=DEFAULT, file='maps\\net.tgm'):
    """Original BlockSender/BlockReceiver sections carrying file blocks between
    two AdminConns (generation word + channel section, as in 0x74E280/0x75A350).
    `late` sequences are delivered three sections later and reported lost."""
    second = 'maps\\net2.tgm'
    sc = Scenario(m, name, kind='channel', sender_config=config,
                  sources=[(file, data_of(size, 5)), (second, data_of(size // 2 + 3, 6))])
    sc.construct(); sc.start(units(file))
    pending = []; held = []
    for seq, limit in enumerate(limits):
        if seq == restart_at: sc.start(units(second))
        data = sc.section(seq, limit)
        if seq in late: held.append((seq, data))
        elif seq not in drop: sc.deliver(seq, data)
        for h in [h for h in held if h[0] == seq - 3]: sc.deliver(*h); held.remove(h)
        pending.append((seq, seq not in drop and seq not in late))
        while len(pending) > ack_delay:
            s0, ok = pending.pop(0); sc.packet_result(s0, ok)
    for h in held: sc.deliver(*h)
    sc.progress()
    return sc.finish()

def channel_cases(m):
    return [
        channel_run(m, 'channel_lossless_700', 700, [0x400] * 16),
        channel_run(m, 'channel_two_blocks_per_section_700', 700, [0x500] * 9),
        channel_run(m, 'channel_tight_budget_300', 300, [0x80, 0x100, 0x252, 0x253, 0x300, 0x60] * 4, ack_delay=1),
        channel_run(m, 'channel_losses_1200', 1200, [0x300] * 30, drop={1, 2, 6, 9, 14}, ack_delay=2),
        channel_run(m, 'channel_exact_multiple_256', 256, [0x400] * 6),
        channel_run(m, 'channel_restart_generation_600', 600, [0x300] * 16, late={2, 3}, restart_at=4, ack_delay=1),
    ]

def scenarios(m):
    rows = []
    BS = 64
    for size in [0, 1, BS - 1, BS, BS + 1, 2 * BS, 3 * BS, 3 * BS + 5]:
        rows.append(unit_transfer(m, f'size_{size}', size))
    rows.append(unit_transfer(m, 'size_3000_bulk_release', 3000, release_each=False))
    rows.append(unit_transfer(m, 'size_1500_reverse_order', 1500, order='reverse'))
    rows.append(unit_transfer(m, 'size_700_shuffled_duplicates', 700, order='shuffle', duplicates={0, 2, 5, 9}))
    rows.append(unit_transfer(m, 'size_300_header_last', 300, header_last=True))
    small = {'blocks_per_frame': 20, 'block_size': 64, 'max_pending_blocks': 3}
    rows.append(unit_transfer(m, 'max_pending_3_size_1000', 1000, config=small))
    rows.append(unit_transfer(m, 'max_pending_1_size_200', 200, config={**small, 'max_pending_blocks': 1}))
    rows.append(unit_transfer(m, 'max_pending_0_size_200', 200, config={**small, 'max_pending_blocks': 0}))
    odd = {'blocks_per_frame': 7, 'block_size': 7, 'max_pending_blocks': 5}
    rows.append(unit_transfer(m, 'block_size_7_size_50', 50, config=odd))
    rows.append(unit_transfer(m, 'block_size_7_size_49_exact', 49, config=odd))
    rows.append(unit_transfer(m, 'receiver_block_size_32_vs_sender_64', 232, receiver_config={**DEFAULT, 'block_size': 32}))
    rows.append(unit_transfer(m, 'short_read_reported_size_larger', 150, reported=170))
    for label, file in [('dotdot_parent', '..\\evil.dll'), ('dotdot_inside_name', 'maps\\a..b.tgm'),
                        ('dotdot_after_embedded_nul', 'maps\\x\0..\\y.tgm'), ('long_name_300', 'maps\\' + 'n' * 300 + '.tgm'),
                        ('non_ascii_name', 'maps\\\u00e9t\u00e9\U0001f600.tgm'), ('single_dot_names', '.\\maps\\.x.tgm')]:
        rows.append(unit_transfer(m, f'name_{label}', 100, file=file))
    rows.append(unit_transfer(m, 'empty_name', 70, file=''))
    if m.image == 'original':  # 0x761540 / 0x7619A0 are identical in v14; recorded once (large)
        rows.append(unit_transfer(m, 'name_length_wraps_16_bits', 10, file='maps\\' + 'w' * 65531 + '.tgm'))
    rows.append(edge_cases(m))
    rows.append(restart_case(m))
    rows += channel_cases(m)
    return rows

def windows_of(visited, pe):
    ranges = []
    for a, size in sorted(visited.items()):
        if not pe.base <= a < pe.base + pe.image_size: continue
        if ranges and ranges[-1][1] >= a: ranges[-1][1] = max(ranges[-1][1], a + size)
        else: ranges.append([a, a + size])
    return ranges

def main():
    images = {}; windows = {}; differing = []
    for image, pe, other in (('original', PE0, PE1), ('v14', PE1, PE0)):
        m = FileMachine(image)
        images[image] = scenarios(m)
        windows[image] = []
        for a, b in windows_of(m.visited, pe):
            raw = pe.range(a, b - a); same = raw == other.range(a, b - a)
            windows[image].append({'start': hex(a), 'end': hex(b), 'sha256': sha(raw), 'identical_in_other_image': same})
            if not same: differing.append([image, hex(a), hex(b)])
        print(image, len(images[image]), 'scenarios', sum(len(r['steps']) for r in images[image]), 'steps', 'heap', hex(m.heap))
    # The only executed bytes that differ: the two terminal-length sites and the v14 caves.
    patches = [(0x76146a, 0x76146f), (0x761270, 0x761275), (0x9eff80, 0x9effaf)]
    for image, a, b in differing:
        assert any(int(a, 16) < hi and lo < int(b, 16) for lo, hi in patches), (image, a, b)
    # Interop: original image with only one side's v14 site + cave installed
    # (original host -> v14 client and v14 host -> original client).
    images['interop'] = []
    for sender_image, receiver_image in (('original', 'v14'), ('v14', 'original')):
        m = FileMachine('original')
        side = [(0x76146a, 0x76146f), (0x9eff80, 0x9eff90)] if sender_image == 'v14' else [(0x761270, 0x761275), (0x9eff90, 0x9effaf)]
        for lo, hi in side: m.u.mem_write(lo, PE1.range(lo, hi - lo))
        for size in (128, 197):
            row = unit_transfer(m, f'interop_{sender_image}_sender_{receiver_image}_receiver_size_{size}', size)
            row['sender_image'] = sender_image; row['receiver_image'] = receiver_image
            images['interop'].append(row)
        for a, b in windows_of(m.visited, PE0):
            for lo, hi in side:
                if a < hi and lo < b: assert PE1.range(lo, hi - lo) == bytes(m.u.mem_read(lo, hi - lo))
    print('interop', len(images['interop']), 'scenarios')
    report = {'schema': 1, 'scope': __doc__, 'original_sha256': sha(PE0.b), 'v14_sha256': sha(PE1.b),
              'producer_sha256': sha(Path(__file__).read_bytes()),
              'constants': {'mode_read_0xAB4784': MODE_READ, 'mode_write_0xAB4788': MODE_WRITE, 'defaults': DEFAULT,
                            'v14_patches': {'0x76146A': 'sender terminal length -> cave 0x9EFF80',
                                            '0x761270': 'receiver terminal length -> cave 0x9EFF90'}},
              'images': images, 'executed_windows': windows, 'differing_windows': differing}
    (HERE / 'native-file-transfer.json').write_text(json.dumps(report, separators=(',', ':')) + '\n', encoding='utf-8')

if __name__ == '__main__':
    main()
