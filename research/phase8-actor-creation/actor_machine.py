"""Unicorn machine for the fresh ACTOR creation oracle (KKC_GWorld::CreateGThing
4F9C10 on an A25B28 actor creation block, class 3).

Base: research/phase8-conversion/native_machine.py (Machine: original image at
0x400000 with every executed instruction checked byte-identical in the v14
executable, page-0 null guard, bump allocator 6F7810 with 0xA5 fill, CString
helpers, standard log/throw seams). Imported read-only; its sha256 is pinned in
the fixture.

Everything reached from 4F9C10 runs NATIVELY except the seams below (each is
another owner's subsystem; every call is recorded in order with its
arguments). Any other function entry stops the case as `Unexpected`.
"""
from pathlib import Path
import struct, sys, hashlib
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase8-conversion'))
import native_machine as nm
from native_machine import Machine, P, V, VISITED, STACK, STOP, Throw, Stop, f32
from unicorn import UC_HOOK_MEM_READ
from unicorn.x86_const import *

H = lambda b: hashlib.sha256(b).hexdigest()
FUNCS = set()
for line in (ROOT.parent / 'reconstruction/research/native-aa/functions.tsv').read_text().splitlines()[1:]:
    FUNCS.add(int(line.split('\t')[0], 16))
DIRECTIONS = (ROOT / 'research/phase5-actors/direction-tables.bin').read_bytes()
PCS = {'Single24': 0, 'Double53': 1, 'Extended64': 2}

# Natively executed functions (an entry to any other function stops the case).
NATIVE = {
    # 4F9C10 and the generic GThing/GObject parts (as in the class 2/9 oracle)
    0x4f9c10, 0x4ea5e0, 0x4ef2a0, 0x4ee9f0, 0x4453a0, 0x49b900, 0x49b440, 0x85d840, 0x4f02b0,
    0x4e9fc0, 0x4ee460, 0x4efdb0, 0x4efb70, 0x4f0190, 0x4ebe50, 0x4ee700, 0x4efa40, 0x4ef6a0,
    0x4f0460, 0x4ee4c0, 0x4cf210, 0x4ea0f0, 0x4ea010, 0x4ea020, 0x4eb6e0, 0x4f1f70, 0x4ea3d0,
    0x4eba40, 0x4d1e10, 0x4f9510, 0x4bedf0, 0x4f4c40, 0x50c220, 0x50c270, 0x42a590, 0x96cc4c,
    0x405b80, 0x4012c0, 0x4047a0, 0x6a71d0, 0x6a71f0, 0x4c9fa0, 0x4f0500, 0x462e50, 0x4d1e40,
    0x401560, 0x4fa030, 0x4fa150, 0x4be740, 0x4c8c70, 0x4c8c20, 0x4f1430,
    # GActor
    0x4d6bb0, 0x4d4a10, 0x4ee840, 0x4d6f10, 0x4d21a0, 0x4cf480, 0x45f240, 0x4d2100, 0x432e90,
    0x96d710, 0x4cfb00, 0x763b90, 0x4b2830, 0x4d1e80, 0x4f4a20, 0x4849f0, 0x4d0720, 0x4cfc80,
    0x4cf1b0, 0x432e40, 0x4d7230, 0x4d1fd0, 0x401060, 0x45e700, 0x45e710, 0x96d5e0, 0x4cf300,
    0x4d62f0, 0x4e10b0, 0x4e1ad0, 0x45d6a0, 0x4bb000, 0x4d1e20, 0x4d69a0,
    0x4fa113, 0x4fa10d, 0x4fa2c4, 0x4fa2bb, 0x514cd0, 0x4ced60,
    # component requirement virtuals (+20, constant returns)
    0x580f70, 0x7c53e0, 0x55ea50, 0x5b6830, 0x589090, 0x581370, 0x6b2450, 0x7c7840,
}

# 4CF480 constructors (slot or Mind kind) -> (size, constructor)
CTORS = {0: (0x44, 0x561960), 1: (0x74, 0x580ed0), 2: (0x30, 0x563670), 3: (0x8c, 0x576920),
         '4org': (0x2c4, 0x526260), '4air': (0x1d8, 0x51bff0), '4elem': (0x1bc, 0x535ba0),
         5: (100, 0x57f4d0), 6: (0xdc, 0x5812f0), 7: (0xdc, 0x579bd0), 8: (0x34, 0x570930),
         9: (0x54, 0x56f370), 10: (0x94, 0x56b720), 11: (0x20, 0x58aa50), 12: (0x24, 0x58cdb0),
         13: (0x38, 0x588fb0), 14: (0x88, 0x5867a0), 15: (0x2c, 0x565fa0), 16: (0x20, 0x581e40),
         17: (0x10, 0x571690), 18: (0x10, 0x562f10), 19: (0x18, 0x575940), 20: (0x34, 0x567cd0),
         21: (0x18, 0x584210), 22: (0x18, 0x56eab0), 23: (0xb4, 0x55e8e0), 24: (0x60, 0x58b9a0),
         25: (0x24, 0x574fa0), 26: (0x14, 0x572180), 27: (0x38, 0x93fec0), 28: (0x10, 0x580120),
         29: (0x24, 0x564160), 30: (0x2c, 0x574490)}
CTOR_KIND = {addr: kind for kind, (size, addr) in CTORS.items()}
# Native requirement virtual (+20) of each constructed class.
REQUIREMENT = {0: 0x580f70, 1: 0x580f70, 2: 0x580f70, 3: 0x580f70, '4org': 0x7c53e0, '4air': 0x55ea50,
               '4elem': 0x5b6830, 5: 0x580f70, 6: 0x580f70, 7: 0x580f70, 8: 0x580f70, 9: 0x580f70,
               10: 0x580f70, 11: 0x580f70, 12: 0x580f70, 13: 0x589090, 14: 0x580f70, 15: 0x4c9fa0,
               16: 0x580f70, 17: 0x581370, 18: 0x7c53e0, 19: 0x580f70, 20: 0x6b2450, 21: 0x580f70,
               22: 0x580f70, 23: 0x6b2450, 24: 0x7c7840, 25: 0x580f70, 26: 0x580f70, 27: 0x580f70,
               28: 0x580f70, 29: 0x580f70, 30: 0x580f70}
SLOT = lambda kind: 4 if isinstance(kind, str) else kind
SHAPE_A, SHAPE_B = 0x38, 0x50
ACTORDATA_VT = 0xa05540


class ActorMachine(Machine):
    def __init__(self, precision='Double53'):
        super().__init__(precision)
        self.guard = False
        self.trace = []
        self.unexpected = []
        self.mem_reads = {}
        self.labels = {}
        self.actors = []           # allocated 0x180 blocks in order
        self.components = []       # (actor index, kind, address)
        self.objects = {}          # fixture GTID objects
        self.idmgr = self.alloc(0x60020, 0)
        self.bitmap = self.alloc(0x2000, 0)
        self.w(self.idmgr + 0x60010, self.bitmap)
        self.w(self.idmgr + 0x60008, 1)
        self.generations = self.idmgr
        self.w(0xb364c0, self.idmgr)
        self.gworld = self.alloc(0x800, 0)
        self.w(0xb36534, self.gworld)
        self.gamedata = self.alloc(0x800, 0)
        self.w(0xb36530, self.gamedata)
        for addr, name, pop, nargs in ((0x77dc50, '77DC50', 8, 2), (0x77da70, '77DA70', 0xc, 3)):
            self.leaf(addr, name, pop, nargs, record=False)

    # ------------------------------------------------------------ helpers
    def _hook(self, u, pc, size, user):
        if self.guard and pc in FUNCS:
            self.trace.append(pc)
            if pc not in self.seams and pc not in NATIVE:
                self.unexpected.append(hex(pc))
                self.outcome = {'kind': 'Unexpected', 'pc': hex(pc)}
                raise Stop()
        super()._hook(u, pc, size, user)

    def ecx(self):
        return self.u.reg_read(UC_X86_REG_ECX)

    def probe(self, name, base, size):
        seen = self.mem_reads.setdefault(name, set())

        def h(u, access, address, sz, value, _):
            seen.add(address - base)
        self.u.hook_add(UC_HOOK_MEM_READ, h, begin=base, end=base + size - 1)

    def leaf(self, address, name, pop, nargs=0, ret=0, record=True, extra=None):
        def h(m):
            args = [m.arg(i) for i in range(nargs)]
            if record:
                e = m.record(name, ecx=m.label(m.ecx()), args=[m.label(a) for a in args])
                if extra:
                    extra(m, e, args)
            m.ret(pop, ret(m, args) if callable(ret) else ret)
        self.seam(address, h)

    def label(self, v):
        if v in self.labels:
            return self.labels[v]
        if 0x2f00000 <= v < 0x2f10000:
            return 'stack'
        return v

    def ret_float(self, bits, pop):
        code = self.alloc(32)
        data = code + 16
        self.w(data, bits)
        self.u.mem_write(code, bytes([0xd9, 0x05]) + struct.pack('<I', data) + bytes([0xc2]) + struct.pack('<H', pop))
        self.u.reg_write(UC_X86_REG_EIP, code)

    def set_generation(self, slot, generation):
        self.u.mem_write(self.idmgr + 4 + 2 * slot, struct.pack('<H', generation))

    def register(self, gtid, obj):
        """IDMGR slot of a fixture object (generation, object pointer)."""
        slot = gtid & 0xffff
        self.set_generation(slot, gtid >> 16)
        self.w(self.idmgr + 0x20004 + 4 * slot, obj)
        if obj:
            self.w(obj + 0x14, gtid)

    def rb(self, a):
        return self.u.mem_read(a, 1)[0]

    def words(self, a, n):
        return [self.r(a + 4 * i) for i in range(n)]
