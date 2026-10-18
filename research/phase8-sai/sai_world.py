"""Shared synthetic SAI world for the phase-8 SAI oracles (original AA.exe
code only, research/phase8-ai/native_machine.py underneath: page-0 null
guard, allocator seams, v14 byte check).

A case describes symbolic objects (integer ids; 0 is null) and their fields.
The harness lays them out at native offsets, fills the GTID registry
[B364C0], and answers every virtual call and seamed callee with a value
drawn from the case's seeded RNG at the moment of the call. Each answer is
recorded as an event {op, args..., result}; the Rust replay answers the same
calls from those events in order (lockstep), so any divergence in call
order, arguments or count fails the replay.
"""
from pathlib import Path
import math, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / 'phase8-ai'))
from native_machine import Machine, PRECISIONS, f32, from_f32, header, instructions, sha
from unicorn import x86_const as X

SAI, WORLD, MAP, REGISTRY = 0xb36528, 0xb36534, 0xb3652c, 0xb364c0


def x87_of(v):
    """80-bit image of a finite Python float (exact for binary64 inputs)."""
    if v == 0:
        return {'significand': 0, 'sign_exponent': 0}
    m, e = math.frexp(abs(v))
    return {'significand': int(m * (1 << 64)) & ((1 << 64) - 1), 'sign_exponent': ((v < 0) << 15) | (e - 1 + 16383)}


def x87_ext(rnd, lo, hi):
    """An 80-bit value with a full 64-bit significand in [lo, hi)."""
    v = rnd.uniform(lo, hi)
    img = x87_of(v)
    if img['significand']:
        img['significand'] |= rnd.getrandbits(11)
    return img


class World(Machine):
    """Objects are dicts: {'kind': ..., fields...}; ids are case-wide."""

    def __init__(self, c):
        super().__init__(c['precision'])
        self.c = c
        self.rnd = random.Random(c['seed'])
        u = self.u
        u.reg_write(X.UC_X86_REG_MXCSR, 0x1f80)
        self.w(0xb4b340, 1)  # CRT SSE2 path of _CIpow (96D890)
        self.seam(0x6f7810, lambda m: self.ret(self.alloc(max(self.arg(1), 4))))
        self.seam(0x6f7430, lambda m: self.ret(0))
        self.table = self.alloc(0x40000)
        self.w(REGISTRY, self.table)
        self.addr, self.ident = {}, {}
        self.vtables = {}

    # identity ------------------------------------------------------------
    def a(self, ident):
        return 0 if not ident else self.addr[ident]

    def id_of(self, address):
        if address == 0:
            return 0
        return self.ident.get(address, -1)

    def place(self, ident, size):
        p = self.alloc(size)
        self.addr[ident] = p
        self.ident[p] = ident
        return p

    def registry(self, entries):
        """entries: {index: [serial, object id or 0]}."""
        for index, (serial, obj) in entries.items():
            index = int(index)
            self.u.mem_write(self.table + 4 + index * 2, serial.to_bytes(2, 'little'))
            self.w(self.table + 0x20004 + index * 4, self.a(obj))

    # answers ---------------------------------------------------------------
    def ev(self, op, **kw):
        self.events.append({'op': op, **kw})

    def x87_return(self, img, pop=0):
        scratch = self.alloc(16)
        self.u.mem_write(scratch, struct.pack('<QH', img['significand'], img['sign_exponent']))
        stub = self.alloc(16)
        code = b'\xdb\x2d' + struct.pack('<I', scratch) + (b'\xc2' + struct.pack('<H', pop) if pop else b'\xc3')
        self.u.mem_write(stub, code)
        self.u.reg_write(X.UC_X86_REG_EIP, stub)

    def vtable(self, slots):
        """slots: {offset: handler(self_address)}; returns the table."""
        vt = self.alloc(0x200)
        for off, fn in slots.items():
            self.w(vt + off, self.stub(fn))
        return vt

    def pick(self, choices):
        return self.rnd.choice(choices)
